use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use veln_ast::{Function, FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_ir::TypedProgram;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, ModuleValuePublicationWork, ReusableStandardEnvironment,
    check_project_surface_module_with_standard_modules_environment,
    try_prepare_current_reusable_standard_surface_module_environment,
    validate_standard_symbol_registry_diagnostic,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{
    CapturedDependencyIndex, CapturedDependencyProject, LoadedSurfaceModules, ReachabilityCache,
    load_embedded_standard_surface_module_for_names, load_surface_modules,
    load_surface_modules_with_captured_dependency_index,
    reachable_entry_module_with_standard_cache,
};

static STANDARD_ENVIRONMENTS: OnceLock<StandardEnvironmentCache> = OnceLock::new();

struct StandardEnvironmentCache {
    inputs: Mutex<BTreeMap<BTreeSet<String>, ReusableStandardInput>>,
}

impl StandardEnvironmentCache {
    fn new() -> Self {
        Self {
            inputs: Mutex::new(BTreeMap::new()),
        }
    }

    fn input_for_standard_modules(
        &self,
        module_names: &BTreeSet<String>,
    ) -> Result<ReusableStandardInput, Box<Diagnostic>> {
        validate_standard_symbol_registry_diagnostic()?;
        let mut inputs = self
            .inputs
            .lock()
            .expect("standard environment cache should not be poisoned");
        let input = inputs
            .entry(module_names.clone())
            .or_insert_with(|| {
                let standard_module = load_embedded_standard_surface_module_for_names(module_names);
                let environment = try_prepare_current_reusable_standard_surface_module_environment(
                    &standard_module,
                )
                .expect("validated standard environment");
                ReusableStandardInput {
                    module: Arc::new(standard_module),
                    environment,
                }
            })
            .clone();
        Ok(input)
    }
}

#[derive(Clone)]
struct ReusableStandardInput {
    module: Arc<SurfaceModule>,
    environment: ReusableStandardEnvironment,
}

#[cfg(test)]
pub(crate) struct TestStandardEnvironmentCache {
    inputs: Mutex<BTreeMap<BTreeSet<String>, ReusableStandardInput>>,
    standard_prepares: AtomicUsize,
    application_analyses: AtomicUsize,
}

#[cfg(test)]
impl TestStandardEnvironmentCache {
    pub(crate) fn new() -> Self {
        Self {
            inputs: Mutex::new(BTreeMap::new()),
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
    selected_standard: Arc<SurfaceModule>,
    selected_standard_module_names: BTreeSet<String>,
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
    pub entry_target: Option<String>,
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
        standard_environment_for_modules,
    )
}

fn analyze_project_with_standard_provider(
    project: Project,
    doctest_mode: DoctestMode,
    timings: Option<&mut Vec<AnalysisTiming>>,
    standard_for_module: impl FnOnce(
        &BTreeSet<String>,
    ) -> Result<ReusableStandardInput, Box<Diagnostic>>,
) -> ProjectAnalysis {
    analyze_project_with_surface_and_standard_provider(
        project,
        doctest_mode,
        timings,
        load_surface_modules,
        standard_for_module,
    )
}

fn analyze_project_with_surface_and_standard_provider(
    mut project: Project,
    doctest_mode: DoctestMode,
    mut timings: Option<&mut Vec<AnalysisTiming>>,
    load_surface: impl FnOnce(&Project) -> (LoadedSurfaceModules, Vec<Diagnostic>),
    standard_for_module: impl FnOnce(
        &BTreeSet<String>,
    ) -> Result<ReusableStandardInput, Box<Diagnostic>>,
) -> ProjectAnalysis {
    let surface_start = std::time::Instant::now();
    let PreparedDoctestAnalysis {
        mut source_diagnostics,
        doctest_expectations,
        expected_doctest_failures,
        invalid_origins,
    } = prepare_doctest_analysis(&mut project, doctest_mode);

    let (loaded, parse_diagnostics) = load_surface(&project);
    source_diagnostics.extend(unique_parse_diagnostics(
        parse_diagnostics,
        &invalid_origins,
    ));
    record_timing(&mut timings, "surface_parse_lower", surface_start.elapsed());

    let standard = match standard_for_module(&loaded.selected_standard_module_names) {
        Ok(standard) => standard,
        Err(diagnostic) => {
            let selected_standard = Arc::new(load_embedded_standard_surface_module_for_names(
                &loaded.selected_standard_module_names,
            ));
            return ProjectAnalysis {
                project,
                module: loaded.application,
                selected_standard,
                selected_standard_module_names: loaded.selected_standard_module_names,
                doctest_expectations,
                source_diagnostics,
                semantic_diagnostics: Vec::new(),
                checked: lowered_internal_failure(*diagnostic),
                expected_doctest_failures,
                reachability_cache: ReachabilityCache::default(),
            };
        }
    };
    let semantic_start = std::time::Instant::now();
    let (semantic_diagnostics, checked) =
        check_project_surface_module_with_standard_modules_environment(
            &loaded.application,
            &loaded.selected_standard_module_names,
            &standard.environment,
        );
    record_timing(
        &mut timings,
        "semantic_environment_check",
        semantic_start.elapsed(),
    );

    ProjectAnalysis {
        project,
        module: loaded.application,
        selected_standard: standard.module,
        selected_standard_module_names: loaded.selected_standard_module_names,
        doctest_expectations,
        source_diagnostics,
        semantic_diagnostics,
        checked,
        expected_doctest_failures,
        reachability_cache: ReachabilityCache::default(),
    }
}

struct PreparedDoctestAnalysis {
    source_diagnostics: Vec<Diagnostic>,
    doctest_expectations: BTreeMap<String, DoctestExpectation>,
    expected_doctest_failures: BTreeMap<String, SourceSpan>,
    invalid_origins: BTreeSet<String>,
}

fn prepare_doctest_analysis(
    project: &mut Project,
    doctest_mode: DoctestMode,
) -> PreparedDoctestAnalysis {
    let Some(doctests) = (match doctest_mode {
        DoctestMode::Include => Some(doctest_sources(&project.files)),
        DoctestMode::Exclude => None,
    }) else {
        return PreparedDoctestAnalysis {
            source_diagnostics: Vec::new(),
            doctest_expectations: BTreeMap::new(),
            expected_doctest_failures: BTreeMap::new(),
            invalid_origins: BTreeSet::new(),
        };
    };
    let invalid_origins = doctests
        .diagnostics
        .iter()
        .filter_map(|diagnostic| {
            if diagnostic.id == "module.invalid_source_path" {
                diagnostic
                    .span
                    .as_ref()
                    .map(|span| span.file.as_str().to_string())
            } else {
                None
            }
        })
        .collect();
    project.files.extend(doctests.sources);
    PreparedDoctestAnalysis {
        source_diagnostics: doctests.diagnostics,
        doctest_expectations: doctests.expectations,
        expected_doctest_failures: doctests.expected_failures,
        invalid_origins,
    }
}

fn unique_parse_diagnostics(
    diagnostics: Vec<Diagnostic>,
    invalid_doctest_origins: &BTreeSet<String>,
) -> impl Iterator<Item = Diagnostic> + '_ {
    diagnostics.into_iter().filter(|diagnostic| {
        diagnostic.id != "module.invalid_source_path"
            || diagnostic
                .span
                .as_ref()
                .is_none_or(|span| !invalid_doctest_origins.contains(span.file.as_str()))
    })
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

pub fn checked_project_diagnostics_with_captured_dependencies(
    project: Project,
    doctest_mode: DoctestMode,
    dependencies: Vec<CapturedDependencyProject>,
) -> Vec<Diagnostic> {
    analyze_project_with_captured_dependencies(project, doctest_mode, &dependencies)
        .checked_diagnostics()
}

pub fn analyze_project_with_captured_dependencies(
    project: Project,
    doctest_mode: DoctestMode,
    dependencies: &[CapturedDependencyProject],
) -> ProjectAnalysis {
    let dependencies = CapturedDependencyIndex::new(dependencies);
    analyze_project_with_captured_dependency_index(project, doctest_mode, &dependencies)
}

pub fn analyze_project_with_captured_dependency_index(
    project: Project,
    doctest_mode: DoctestMode,
    dependencies: &CapturedDependencyIndex<'_>,
) -> ProjectAnalysis {
    analyze_project_with_surface_and_standard_provider(
        project,
        doctest_mode,
        None,
        |project| load_surface_modules_with_captured_dependency_index(project, dependencies),
        standard_environment_for_modules,
    )
}

impl ProjectAnalysis {
    #[cfg(test)]
    pub(crate) fn selected_standard_module_names_for_test(&self) -> &BTreeSet<String> {
        &self.selected_standard_module_names
    }

    pub fn reusable_standard_ir(&self) -> Option<&TypedProgram> {
        self.module
            .functions
            .iter()
            .all(|function| function.package_name.as_deref() == Some(veln_stdlib::PACKAGE_NAME))
            .then_some(self.checked.ir.as_ref())
            .flatten()
    }

    pub fn source_diagnostics(&self) -> Vec<Diagnostic> {
        self.reconcile_doctest_failures(self.source_diagnostics.clone())
    }

    pub fn semantic_diagnostics(&self) -> Vec<Diagnostic> {
        let diagnostics = self.aggregate_diagnostics(self.semantic_diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        let diagnostics = self.aggregate_diagnostics(self.checked.diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    fn aggregate_diagnostics(&self, additional: Vec<Diagnostic>) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(additional);
        order_combined_parse_diagnostics(&mut diagnostics, &self.project);
        diagnostics
    }

    pub fn unpublished_declarations(&self) -> &[veln_sema::UnpublishedDeclaration] {
        &self.checked.unpublished_declarations
    }

    pub fn module_value_publication_diagnostics_for_identities(
        &self,
        identities: &[veln_sema::ModuleValuePublicationIdentity],
    ) -> (
        Vec<veln_sema::ModuleValuePublicationDiagnostic>,
        ModuleValuePublicationWork,
    ) {
        self.checked
            .module_value_publication_diagnostics_for_identities(identities)
    }

    pub fn lower_reachable_entry(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
    ) -> ReachableEntryAnalysis {
        self.lower_reachable_entry_with_timing(entry, entry_kind).0
    }

    pub fn lower_reachable_function(&self, entry: &Function) -> ReachableEntryAnalysis {
        self.lower_reachable_function_with_timing(entry).0
    }

    pub fn lower_reachable_function_with_timing(
        &self,
        entry: &Function,
    ) -> (ReachableEntryAnalysis, AnalysisTiming) {
        let name = entry.name.as_deref().unwrap_or_default();
        self.lower_reachable_entry_selected_with_timing(name, entry.kind, Some(entry))
    }

    pub fn lower_reachable_entry_with_timing(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
    ) -> (ReachableEntryAnalysis, AnalysisTiming) {
        let selected_function = self.module.functions.iter().find(|function| {
            function.kind == entry_kind && function.name.as_deref() == Some(entry)
        });
        self.lower_reachable_entry_selected_with_timing(entry, entry_kind, selected_function)
    }

    fn lower_reachable_entry_selected_with_timing(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
        selected_function: Option<&Function>,
    ) -> (ReachableEntryAnalysis, AnalysisTiming) {
        let start = std::time::Instant::now();
        let module = reachable_entry_module_with_standard_cache(
            &self.selected_standard,
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let (lowered, entry_target) = match standard_environment_for_modules(
            &self.selected_standard_module_names,
        ) {
            Ok(standard) => {
                veln_sema::lower_project_reachable_surface_modules_with_standard_environment_and_target(
                    &module,
                    &self.selected_standard,
                    &standard.environment,
                    selected_function,
                )
            }
            Err(diagnostic) => (lowered_internal_failure(*diagnostic), None),
        };
        (
            ReachableEntryAnalysis {
                module,
                lowered,
                entry_target,
            },
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

fn order_combined_parse_diagnostics(diagnostics: &mut [Diagnostic], project: &Project) {
    let slots = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            (diagnostic.kind == veln_diagnostics::DiagnosticKind::Parse
                && diagnostic.span.is_some())
            .then_some(index)
        })
        .collect::<Vec<_>>();
    if slots.len() < 2 {
        return;
    }
    let source_order = project
        .files
        .iter()
        .enumerate()
        .map(|(index, source)| (source.path().clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut ordered = slots
        .iter()
        .enumerate()
        .map(|(original_order, &index)| (original_order, diagnostics[index].clone()))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(original_order, diagnostic)| {
        let span = diagnostic.span.as_ref().expect("parse diagnostic span");
        (
            source_order.get(&span.file).copied().unwrap_or(usize::MAX),
            span.start.offset,
            span.end.offset,
            *original_order,
        )
    });
    for (&slot, (_, diagnostic)) in slots.iter().zip(ordered) {
        diagnostics[slot] = diagnostic;
    }
}

fn standard_environment_for_modules(
    module_names: &BTreeSet<String>,
) -> Result<ReusableStandardInput, Box<Diagnostic>> {
    STANDARD_ENVIRONMENTS
        .get_or_init(StandardEnvironmentCache::new)
        .input_for_standard_modules(module_names)
}

#[cfg(test)]
fn standard_environment_with_test_cache(
    cache: &TestStandardEnvironmentCache,
    module_names: &BTreeSet<String>,
) -> Result<ReusableStandardInput, Box<Diagnostic>> {
    validate_standard_symbol_registry_diagnostic()?;
    let mut inputs = cache
        .inputs
        .lock()
        .expect("test standard environment cache should not be poisoned");
    let input = inputs
        .entry(module_names.clone())
        .or_insert_with(|| {
            cache.standard_prepares.fetch_add(1, Ordering::SeqCst);
            let module = load_embedded_standard_surface_module_for_names(module_names);
            let environment =
                try_prepare_current_reusable_standard_surface_module_environment(&module)
                    .expect("validated test standard environment");
            ReusableStandardInput {
                module: Arc::new(module),
                environment,
            }
        })
        .clone();
    Ok(input)
}

fn lowered_internal_failure(diagnostic: Diagnostic) -> LoweredSurfaceModule {
    LoweredSurfaceModule::internal_failure(vec![diagnostic])
}

#[cfg(test)]
mod tests;
