use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_ir::TypedProgram;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, ReusableStandardEnvironment,
    check_project_surface_module_with_standard_environment,
    lower_project_reachable_surface_module_with_standard_environment,
    prepare_current_reusable_standard_surface_module_environment,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{ReachabilityCache, load_surface_module, reachable_entry_module_with_cache};

static STANDARD_ENVIRONMENTS: OnceLock<StandardEnvironmentCache> = OnceLock::new();
static EMBEDDED_STANDARD_MODULE_NAMES: OnceLock<BTreeSet<String>> = OnceLock::new();

struct StandardEnvironmentCache {
    environments: Mutex<BTreeMap<BTreeSet<String>, ReusableStandardEnvironment>>,
}

impl StandardEnvironmentCache {
    fn new() -> Self {
        Self {
            environments: Mutex::new(BTreeMap::new()),
        }
    }

    fn environment_for_module(&self, module: &SurfaceModule) -> ReusableStandardEnvironment {
        let module_names = standard_module_names(module);
        let mut environments = self
            .environments
            .lock()
            .expect("standard environment cache should not be poisoned");
        environments
            .entry(module_names)
            .or_insert_with(|| prepare_current_reusable_standard_surface_module_environment(module))
            .clone()
    }
}

#[cfg(test)]
pub(crate) struct TestStandardEnvironmentCache {
    environments: Mutex<BTreeMap<BTreeSet<String>, ReusableStandardEnvironment>>,
    standard_prepares: AtomicUsize,
    application_analyses: AtomicUsize,
}

#[cfg(test)]
impl TestStandardEnvironmentCache {
    pub(crate) fn new() -> Self {
        Self {
            environments: Mutex::new(BTreeMap::new()),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisTiming {
    pub stage: &'static str,
    pub duration: Duration,
}

pub fn analyze_project(project: Project, doctest_mode: DoctestMode) -> ProjectAnalysis {
    analyze_project_with_standard_environment_and_timings(project, doctest_mode, None)
}

pub fn analyze_project_with_timings(
    project: Project,
    doctest_mode: DoctestMode,
) -> (ProjectAnalysis, Vec<AnalysisTiming>) {
    let mut timings = Vec::new();
    let analysis = analyze_project_with_standard_environment_and_timings(
        project,
        doctest_mode,
        Some(&mut timings),
    );
    (analysis, timings)
}

fn analyze_project_with_standard_environment_and_timings(
    project: Project,
    doctest_mode: DoctestMode,
    timings: Option<&mut Vec<AnalysisTiming>>,
) -> ProjectAnalysis {
    analyze_project_with_standard_provider(
        project,
        doctest_mode,
        timings,
        standard_environment_for_module,
    )
}

fn analyze_project_with_standard_provider(
    mut project: Project,
    doctest_mode: DoctestMode,
    mut timings: Option<&mut Vec<AnalysisTiming>>,
    standard_for_module: impl FnOnce(&SurfaceModule) -> ReusableStandardEnvironment,
) -> ProjectAnalysis {
    let surface_start = std::time::Instant::now();
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
    record_timing(&mut timings, "surface_parse_lower", surface_start.elapsed());

    let semantic_start = std::time::Instant::now();
    let standard = standard_for_module(&module);
    let (semantic_diagnostics, checked) =
        check_project_surface_module_with_standard_environment(&module, &standard);
    record_timing(
        &mut timings,
        "semantic_environment_check",
        semantic_start.elapsed(),
    );

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

fn record_timing(
    timings: &mut Option<&mut Vec<AnalysisTiming>>,
    stage: &'static str,
    duration: Duration,
) {
    if let Some(timings) = timings.as_deref_mut() {
        timings.push(AnalysisTiming { stage, duration });
    }
}

#[cfg(test)]
pub(crate) fn analyze_project_with_test_standard_cache(
    project: Project,
    doctest_mode: DoctestMode,
    cache: &TestStandardEnvironmentCache,
) -> ProjectAnalysis {
    cache.application_analyses.fetch_add(1, Ordering::SeqCst);
    analyze_project_with_standard_provider(project, doctest_mode, None, |module| {
        standard_environment_with_test_cache(cache, module)
    })
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
        self.lower_reachable_entry_with_timing(entry, entry_kind).0
    }

    pub fn lower_reachable_entry_with_timing(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
    ) -> (ReachableEntryAnalysis, AnalysisTiming) {
        let start = std::time::Instant::now();
        let module = reachable_entry_module_with_cache(
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let standard = standard_environment_for_module(&module);
        let lowered =
            lower_project_reachable_surface_module_with_standard_environment(&module, &standard);
        (
            ReachableEntryAnalysis { module, lowered },
            AnalysisTiming {
                stage: "reachable_entry_lowering",
                duration: start.elapsed(),
            },
        )
    }

    fn reconcile_doctest_failures(&self, diagnostics: Vec<Diagnostic>) -> Vec<Diagnostic> {
        reconcile_expected_doctest_failures(diagnostics, &self.expected_doctest_failures)
    }
}

fn standard_environment_for_module(module: &SurfaceModule) -> ReusableStandardEnvironment {
    STANDARD_ENVIRONMENTS
        .get_or_init(StandardEnvironmentCache::new)
        .environment_for_module(module)
}

#[cfg(test)]
fn standard_environment_with_test_cache(
    cache: &TestStandardEnvironmentCache,
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    let module_names = standard_module_names(module);
    let mut environments = cache
        .environments
        .lock()
        .expect("test standard environment cache should not be poisoned");
    environments
        .entry(module_names)
        .or_insert_with(|| {
            cache.standard_prepares.fetch_add(1, Ordering::SeqCst);
            prepare_current_reusable_standard_surface_module_environment(&module)
        })
        .clone()
}

fn standard_module_names(module: &SurfaceModule) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    names.extend(
        module
            .uses
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .aliases
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .effects
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .handlers
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .types
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .schemas
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .codecs
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names.extend(
        module
            .functions
            .iter()
            .filter_map(|decl| standard_name(&decl.module_name)),
    );
    names
}

fn standard_name(module_name: &Option<String>) -> Option<String> {
    module_name
        .as_deref()
        .filter(|module_name| embedded_standard_module_names().contains(*module_name))
        .map(str::to_string)
}

fn embedded_standard_module_names() -> &'static BTreeSet<String> {
    EMBEDDED_STANDARD_MODULE_NAMES.get_or_init(|| {
        veln_stdlib::package_bundle()
            .files
            .iter()
            .filter_map(|file| {
                file.path.strip_suffix(".veln").and_then(|module| {
                    (!module.ends_with("_test"))
                        .then(|| format!("std::{}", module.replace('/', "::")))
                })
            })
            .collect()
    })
}
