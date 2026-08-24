use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::Diagnostic;
use veln_ir::TypedProgram;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, ReusableStandardEnvironment,
    check_project_surface_module_with_standard_modules_environment,
    lower_project_reachable_surface_modules_with_standard_environment_filtering_diagnostics,
    prepare_current_reusable_standard_surface_module_environment,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{
    CapturedDependencyProject, ReachabilityCache, load_embedded_standard_surface_module_for_names,
    load_surface_modules, load_surface_modules_with_captured_dependencies,
    reachable_entry_diagnostic_module_with_standard_cache,
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

    fn input_for_standard_modules(&self, module_names: &BTreeSet<String>) -> ReusableStandardInput {
        let mut inputs = self
            .inputs
            .lock()
            .expect("standard environment cache should not be poisoned");
        inputs
            .entry(module_names.clone())
            .or_insert_with(|| {
                let standard_module = load_embedded_standard_surface_module_for_names(module_names);
                let environment =
                    prepare_current_reusable_standard_surface_module_environment(&standard_module);
                ReusableStandardInput {
                    module: Arc::new(standard_module),
                    environment,
                }
            })
            .clone()
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
    mut project: Project,
    doctest_mode: DoctestMode,
    mut timings: Option<&mut Vec<AnalysisTiming>>,
    standard_for_module: impl FnOnce(&BTreeSet<String>) -> ReusableStandardInput,
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

    let (loaded, parse_diagnostics) = load_surface_modules(&project);
    source_diagnostics.extend(parse_diagnostics);
    record_timing(&mut timings, "surface_parse_lower", surface_start.elapsed());

    let standard = standard_for_module(&loaded.selected_standard_module_names);
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

fn analyze_project_with_captured_dependencies(
    mut project: Project,
    doctest_mode: DoctestMode,
    dependencies: &[CapturedDependencyProject],
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

    let (loaded, parse_diagnostics) =
        load_surface_modules_with_captured_dependencies(&project, dependencies);
    source_diagnostics.extend(parse_diagnostics);

    let standard = standard_environment_for_modules(&loaded.selected_standard_module_names);
    let (semantic_diagnostics, checked) =
        check_project_surface_module_with_standard_modules_environment(
            &loaded.application,
            &loaded.selected_standard_module_names,
            &standard.environment,
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

impl ProjectAnalysis {
    #[cfg(test)]
    pub(crate) fn selected_standard_module_names_for_test(&self) -> &BTreeSet<String> {
        &self.selected_standard_module_names
    }

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
        self.lower_reachable_entry_with_timing_and_diagnostic_filter(entry, entry_kind, |_, _| true)
    }

    pub fn lower_reachable_entry_with_timing_and_diagnostic_filter(
        &self,
        entry: &str,
        entry_kind: FunctionKind,
        retain_diagnostic: impl Fn(&SurfaceModule, &Diagnostic) -> bool,
    ) -> (ReachableEntryAnalysis, AnalysisTiming) {
        let start = std::time::Instant::now();
        let module = reachable_entry_module_with_standard_cache(
            &self.selected_standard,
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let diagnostic_module = reachable_entry_diagnostic_module_with_standard_cache(
            &self.selected_standard,
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let standard = standard_environment_for_modules(&self.selected_standard_module_names);
        let lowered =
            lower_project_reachable_surface_modules_with_standard_environment_filtering_diagnostics(
                &diagnostic_module,
                &module,
                &self.selected_standard,
                &standard.environment,
                |diagnostic| retain_diagnostic(&module, diagnostic),
            );
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

fn standard_environment_for_modules(module_names: &BTreeSet<String>) -> ReusableStandardInput {
    STANDARD_ENVIRONMENTS
        .get_or_init(StandardEnvironmentCache::new)
        .input_for_standard_modules(module_names)
}

#[cfg(test)]
fn standard_environment_with_test_cache(
    cache: &TestStandardEnvironmentCache,
    module_names: &BTreeSet<String>,
) -> ReusableStandardInput {
    let mut inputs = cache
        .inputs
        .lock()
        .expect("test standard environment cache should not be poisoned");
    inputs
        .entry(module_names.clone())
        .or_insert_with(|| {
            cache.standard_prepares.fetch_add(1, Ordering::SeqCst);
            let module = load_embedded_standard_surface_module_for_names(module_names);
            let environment = prepare_current_reusable_standard_surface_module_environment(&module);
            ReusableStandardInput {
                module: Arc::new(module),
                environment,
            }
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use veln_diagnostics::diagnostic_to_json;
    use veln_project::Project;
    use veln_source::{SourceFile, SourceSpan};

    use super::*;
    use crate::surface::reachable_entry_module_with_cache;

    #[test]
    fn separated_reachable_lowering_matches_combined_lowering_outputs() {
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![SourceFile::new(
                    "src/main.veln",
                    concat!(
                        "effect Ask\n",
                        "  value() -> Int\n",
                        "end\n",
                        "\n",
                        "fn compute() -> Int effects [Ask]\n",
                        "  perform Ask::value()\n",
                        "end\n",
                        "\n",
                        "handler ask(seed: Int) handles Ask\n",
                        "  value() => seed\n",
                        "end\n",
                        "\n",
                        "pub fn main() -> Int\n",
                        "  let observed = handle compute() with ask(1)\n",
                        "  observed + vec_len([1, 2, 3])\n",
                        "end\n",
                    ),
                )],
                manifest: None,
            },
            DoctestMode::Exclude,
        );

        let separated = analysis.lower_reachable_entry("main", FunctionKind::Function);
        let combined_module =
            merge_surface_modules_for_test(&analysis.selected_standard, &analysis.module);
        let combined_reachable = reachable_entry_module_with_cache(
            &combined_module,
            "main",
            FunctionKind::Function,
            &ReachabilityCache::default(),
        );
        let standard = standard_environment_for_modules(&analysis.selected_standard_module_names);
        let combined_lowered =
            veln_sema::lower_project_reachable_surface_module_with_standard_environment(
                &combined_reachable,
                &standard.environment,
            );

        assert_eq!(
            diagnostic_json(&separated.lowered.diagnostics),
            diagnostic_json(&combined_lowered.diagnostics)
        );
        assert_eq!(separated.lowered.core, combined_lowered.core);
        assert_eq!(separated.lowered.ir, combined_lowered.ir);
        assert_eq!(
            reachable_function_names(&separated.module),
            reachable_function_names(&combined_reachable)
        );
    }

    #[test]
    fn run_selected_unreachable_invalid_handler_bindings_do_not_enter_artifacts() {
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![SourceFile::new(
                    "main.veln",
                    concat!(
                        "effect Ask\n",
                        "  value(item: Int) -> Int\n",
                        "end\n",
                        "\n",
                        "handler unused(Value: Int) handles Ask\n",
                        "  value(Item) => Value + Item\n",
                        "end\n",
                        "\n",
                        "pub fn main() -> Int\n",
                        "  1\n",
                        "end\n",
                    ),
                )],
                manifest: None,
            },
            DoctestMode::Exclude,
        );

        let reachable = analysis.lower_reachable_entry_with_timing_and_diagnostic_filter(
            "main",
            FunctionKind::Function,
            retain_run_reachable_casing_diagnostic_for_test,
        );
        let lowered = reachable.0.lowered;

        assert!(
            lowered
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.invalid_case"),
            "{:#?}",
            lowered.diagnostics
        );
        let core = lowered
            .core
            .expect("unreachable invalid handler should not block core");
        assert_no_invalid_handler_artifacts_in_core(&core);
        let ir = lowered
            .ir
            .expect("unreachable invalid handler should not block IR");
        assert_no_invalid_handler_artifacts_in_ir(&ir);
    }

    #[test]
    fn run_selected_reachable_invalid_handler_bindings_block_artifacts() {
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![SourceFile::new(
                    "main.veln",
                    concat!(
                        "effect Ask\n",
                        "  value(item: Int) -> Int\n",
                        "end\n",
                        "\n",
                        "handler used(Value: Int) handles Ask\n",
                        "  value(Item) => Value + Item\n",
                        "end\n",
                        "\n",
                        "pub fn main() -> Int\n",
                        "  handle perform Ask::value(1) with used(2)\n",
                        "end\n",
                    ),
                )],
                manifest: None,
            },
            DoctestMode::Exclude,
        );

        let reachable = analysis.lower_reachable_entry_with_timing_and_diagnostic_filter(
            "main",
            FunctionKind::Function,
            retain_run_reachable_casing_diagnostic_for_test,
        );
        let lowered = reachable.0.lowered;

        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.invalid_case"),
            "{:#?}",
            lowered.diagnostics
        );
        assert!(lowered.core.is_none(), "{:#?}", lowered.core);
        assert!(lowered.ir.is_none(), "{:#?}", lowered.ir);
    }

    #[test]
    fn run_selected_unreachable_handler_missing_clause_still_blocks() {
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files: vec![SourceFile::new(
                    "main.veln",
                    concat!(
                        "effect Ask\n",
                        "  first() -> Int\n",
                        "  second() -> Int\n",
                        "end\n",
                        "\n",
                        "handler incomplete() handles Ask\n",
                        "  first() => 1\n",
                        "end\n",
                        "\n",
                        "pub fn main() -> Int\n",
                        "  1\n",
                        "end\n",
                    ),
                )],
                manifest: None,
            },
            DoctestMode::Exclude,
        );

        let reachable = analysis.lower_reachable_entry_with_timing_and_diagnostic_filter(
            "main",
            FunctionKind::Function,
            retain_run_reachable_casing_diagnostic_for_test,
        );
        let lowered = reachable.0.lowered;

        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "handler.missing_operation_clause"),
            "{:#?}",
            lowered.diagnostics
        );
        assert!(lowered.core.is_none(), "{:#?}", lowered.core);
        assert!(lowered.ir.is_none(), "{:#?}", lowered.ir);
    }

    fn diagnostic_json(diagnostics: &[veln_diagnostics::Diagnostic]) -> Vec<String> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
            .collect()
    }

    fn retain_run_reachable_casing_diagnostic_for_test(
        module: &SurfaceModule,
        diagnostic: &veln_diagnostics::Diagnostic,
    ) -> bool {
        if diagnostic.id != "name.invalid_case" {
            return true;
        }
        let Some(span) = diagnostic.span.as_ref() else {
            return false;
        };
        reachable_casing_regions_for_test(module)
            .iter()
            .any(|region| span_inside_for_test(span, region))
    }

    fn reachable_casing_regions_for_test(module: &SurfaceModule) -> Vec<SourceSpan> {
        let mut regions = Vec::new();
        regions.extend(
            module
                .functions
                .iter()
                .map(|function| function.span.clone()),
        );
        regions.extend(module.types.iter().map(|type_decl| type_decl.span.clone()));
        regions.extend(module.handlers.iter().map(|handler| handler.span.clone()));
        regions
    }

    fn span_inside_for_test(span: &SourceSpan, region: &SourceSpan) -> bool {
        span.file == region.file
            && span.start.offset >= region.start.offset
            && span.end.offset <= region.end.offset
    }

    fn assert_no_invalid_handler_artifacts_in_core(program: &veln_core::CheckedProgram) {
        for function in &program.functions {
            assert!(
                !function.name.starts_with("__handler_"),
                "unreachable handler lowered to core function: {function:#?}"
            );
            assert!(
                function
                    .params
                    .iter()
                    .all(|param| param.name != "Value" && param.name != "Item"),
                "invalid handler parameter entered core: {function:#?}"
            );
            assert!(
                function.body.iter().all(|stmt| match &stmt.kind {
                    veln_core::CoreStmtKind::Let { name, .. } => name != "Value" && name != "Item",
                    veln_core::CoreStmtKind::Expr { .. }
                    | veln_core::CoreStmtKind::Return { .. } => true,
                }),
                "invalid handler binding entered core body: {function:#?}"
            );
        }
    }

    fn assert_no_invalid_handler_artifacts_in_ir(program: &veln_ir::TypedProgram) {
        for function in &program.functions {
            assert!(
                !function.name.starts_with("__handler_"),
                "unreachable handler lowered to IR function: {function:#?}"
            );
            assert!(
                function
                    .params
                    .iter()
                    .all(|param| param.name != "Value" && param.name != "Item"),
                "invalid handler parameter entered IR: {function:#?}"
            );
            assert!(
                function.body.iter().all(|stmt| match &stmt.kind {
                    veln_ir::IrStmtKind::Let { name, .. } => name != "Value" && name != "Item",
                    veln_ir::IrStmtKind::Expr { .. } | veln_ir::IrStmtKind::Return { .. } => true,
                }),
                "invalid handler binding entered IR body: {function:#?}"
            );
        }
    }

    fn reachable_function_names(module: &SurfaceModule) -> Vec<(&str, &str)> {
        let mut functions = module
            .functions
            .iter()
            .filter_map(|function| {
                Some((function.module_name.as_deref()?, function.name.as_deref()?))
            })
            .collect::<Vec<_>>();
        functions.sort_unstable();
        functions
    }

    fn merge_surface_modules_for_test(
        standard_module: &SurfaceModule,
        application_module: &SurfaceModule,
    ) -> SurfaceModule {
        let mut merged = standard_module.clone();
        merged.uses.extend(application_module.uses.clone());
        merged.aliases.extend(application_module.aliases.clone());
        merged.effects.extend(application_module.effects.clone());
        merged.handlers.extend(application_module.handlers.clone());
        merged.types.extend(application_module.types.clone());
        merged.schemas.extend(application_module.schemas.clone());
        merged.codecs.extend(application_module.codecs.clone());
        merged
            .functions
            .extend(application_module.functions.clone());
        if merged.module.is_none() {
            merged.module = application_module.module.clone();
        }
        merged
    }
}
