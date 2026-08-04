use std::collections::BTreeMap;
use std::sync::OnceLock;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_ir::TypedProgram;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, ReusableStandardEnvironment,
    check_project_surface_module_with_standard_environment,
    lower_project_reachable_surface_module_with_standard_environment,
    prepare_reusable_standard_surface_module_environment,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{
    ReachabilityCache, load_embedded_standard_surface_module, load_surface_module,
    reachable_entry_module_with_cache,
};

static STANDARD_ENVIRONMENT: OnceLock<ReusableStandardEnvironment> = OnceLock::new();

#[cfg(test)]
pub(crate) struct TestStandardEnvironmentCache {
    environment: OnceLock<ReusableStandardEnvironment>,
    standard_prepares: AtomicUsize,
    application_analyses: AtomicUsize,
}

#[cfg(test)]
impl TestStandardEnvironmentCache {
    pub(crate) fn new() -> Self {
        Self {
            environment: OnceLock::new(),
            standard_prepares: AtomicUsize::new(0),
            application_analyses: AtomicUsize::new(0),
        }
    }

    pub(crate) fn standard_prepares(&self) -> usize {
        self.standard_prepares.load(Ordering::SeqCst)
    }

    pub(crate) fn application_analyses(&self) -> usize {
        self.application_analyses.load(Ordering::SeqCst)
    }
}

pub enum DoctestMode {
    Include,
    Exclude,
}

pub struct ProjectAnalysis {
    pub project: Project,
    pub module: SurfaceModule,
    pub doctest_expectations: BTreeMap<String, DoctestExpectation>,
    source_diagnostics: Vec<Diagnostic>,
    semantic_diagnostics: Vec<Diagnostic>,
    checked: LoweredSurfaceModule,
    expected_doctest_failures: BTreeMap<String, SourceSpan>,
    reachability_cache: ReachabilityCache,
}

pub struct ReachableEntryAnalysis {
    pub module: SurfaceModule,
    pub lowered: LoweredSurfaceModule,
}

pub fn analyze_project(project: Project, doctest_mode: DoctestMode) -> ProjectAnalysis {
    analyze_project_with_standard_environment(project, doctest_mode, standard_environment())
}

fn analyze_project_with_standard_environment(
    mut project: Project,
    doctest_mode: DoctestMode,
    standard: &ReusableStandardEnvironment,
) -> ProjectAnalysis {
    let doctests = match doctest_mode {
        DoctestMode::Include => Some(doctest_sources(&project.files)),
        DoctestMode::Exclude => None,
    };
    let mut source_diagnostics = Vec::new();
    let mut doctest_expectations = BTreeMap::new();
    let mut expected_doctest_failures = BTreeMap::new();

    if let Some(doctests) = doctests {
        source_diagnostics.extend(doctests.diagnostics);
        project.files.extend(doctests.sources);
        doctest_expectations = doctests.expectations;
        expected_doctest_failures = doctests.expected_failures;
    }

    let (module, parse_diagnostics) = load_surface_module(&project);
    source_diagnostics.extend(parse_diagnostics);
    let (semantic_diagnostics, checked) =
        check_project_surface_module_with_standard_environment(&module, standard);

    ProjectAnalysis {
        project,
        module,
        doctest_expectations,
        source_diagnostics,
        semantic_diagnostics,
        checked,
        expected_doctest_failures,
        reachability_cache: ReachabilityCache::default(),
    }
}

#[cfg(test)]
pub(crate) fn analyze_project_with_test_standard_cache(
    project: Project,
    doctest_mode: DoctestMode,
    cache: &TestStandardEnvironmentCache,
) -> ProjectAnalysis {
    cache.application_analyses.fetch_add(1, Ordering::SeqCst);
    analyze_project_with_standard_environment(
        project,
        doctest_mode,
        standard_environment_with_test_cache(cache),
    )
}

pub fn checked_project_diagnostics(project: Project, doctest_mode: DoctestMode) -> Vec<Diagnostic> {
    analyze_project(project, doctest_mode).checked_diagnostics()
}

impl ProjectAnalysis {
    pub fn reusable_standard_ir(&self) -> Option<&TypedProgram> {
        self.module
            .functions
            .iter()
            .all(|function| {
                function
                    .module_name
                    .as_deref()
                    .is_some_and(|module| module.starts_with("std::"))
            })
            .then_some(self.checked.ir.as_ref())
            .flatten()
    }

    pub fn source_diagnostics(&self) -> Vec<Diagnostic> {
        self.reconcile_doctest_failures(self.source_diagnostics.clone())
    }

    pub fn semantic_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(self.semantic_diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(self.checked.diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn lower_reachable_entry(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
    ) -> ReachableEntryAnalysis {
        let module = reachable_entry_module_with_cache(
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let lowered = lower_project_reachable_surface_module_with_standard_environment(
            &module,
            standard_environment(),
        );
        ReachableEntryAnalysis { module, lowered }
    }

    fn reconcile_doctest_failures(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        reconcile_expected_doctest_failures(diagnostics, &self.expected_doctest_failures)
    }
}

fn standard_environment() -> &'static ReusableStandardEnvironment {
    standard_environment_with(&STANDARD_ENVIRONMENT)
}

fn standard_environment_with(
    cache: &OnceLock<ReusableStandardEnvironment>,
) -> &ReusableStandardEnvironment {
    cache.get_or_init(|| {
        let module = load_embedded_standard_surface_module();
        prepare_reusable_standard_surface_module_environment(&module)
    })
}

#[cfg(test)]
fn standard_environment_with_test_cache(
    cache: &TestStandardEnvironmentCache,
) -> &ReusableStandardEnvironment {
    cache.environment.get_or_init(|| {
        cache.standard_prepares.fetch_add(1, Ordering::SeqCst);
        let module = load_embedded_standard_surface_module();
        prepare_reusable_standard_surface_module_environment(&module)
    })
}
