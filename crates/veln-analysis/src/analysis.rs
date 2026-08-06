use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_core::CheckedProgram;
use veln_diagnostics::Diagnostic;
use veln_ir::TypedProgram;
use veln_project::Project;
use veln_sema::{
    LoweredSurfaceModule, ReusableStandardEnvironment,
    check_project_surface_module_with_standard_modules_environment,
    lower_reachable_checked_application_with_checked_standard,
    lower_reusable_standard_surface_module_core,
    prepare_current_reusable_standard_surface_module_environment,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{
    ReachabilityCache, load_embedded_standard_surface_module_for_names, load_surface_modules,
    reachable_entry_selection_with_standard_cache,
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
                let checked =
                    lower_reusable_standard_surface_module_core(&standard_module, &environment);
                let checked_core_functions_by_name = checked_core_functions_by_name(&checked);
                ReusableStandardInput {
                    module: Arc::new(standard_module),
                    environment,
                    checked,
                    checked_core_functions_by_name,
                }
            })
            .clone()
    }
}

#[derive(Clone)]
struct ReusableStandardInput {
    module: Arc<SurfaceModule>,
    environment: ReusableStandardEnvironment,
    checked: LoweredSurfaceModule,
    checked_core_functions_by_name: BTreeMap<String, usize>,
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
    selected_standard_checked: LoweredSurfaceModule,
    selected_standard_core_functions_by_name: BTreeMap<String, usize>,
    #[cfg(test)]
    selected_standard_module_names: BTreeSet<String>,
    pub doctest_expectations: BTreeMap<String, DoctestExpectation>,
    source_diagnostics: Vec<Diagnostic>,
    semantic_diagnostics: Vec<Diagnostic>,
    checked: LoweredSurfaceModule,
    checked_core_functions_by_name: BTreeMap<String, usize>,
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
        selected_standard_checked: standard.checked,
        selected_standard_core_functions_by_name: standard.checked_core_functions_by_name,
        #[cfg(test)]
        selected_standard_module_names: loaded.selected_standard_module_names,
        doctest_expectations,
        source_diagnostics,
        semantic_diagnostics,
        checked_core_functions_by_name: checked_core_functions_by_name(&checked),
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
        let start = std::time::Instant::now();
        let selection = reachable_entry_selection_with_standard_cache(
            &self.selected_standard,
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let checked_application = self.checked_reachable_application(&selection.application);
        let checked_standard = self.checked_reachable_standard(&selection.standard);
        let lowered = lower_reachable_checked_application_with_checked_standard(
            &selection.module,
            checked_standard,
            checked_application,
        );
        (
            ReachableEntryAnalysis {
                module: selection.module,
                lowered,
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

    fn checked_reachable_application(&self, module: &SurfaceModule) -> LoweredSurfaceModule {
        self.checked_reachable_module(module, &self.checked, &self.checked_core_functions_by_name)
    }

    fn checked_reachable_standard(&self, module: &SurfaceModule) -> LoweredSurfaceModule {
        self.checked_reachable_module(
            module,
            &self.selected_standard_checked,
            &self.selected_standard_core_functions_by_name,
        )
    }

    fn checked_reachable_module(
        &self,
        module: &SurfaceModule,
        checked: &LoweredSurfaceModule,
        core_functions_by_name: &BTreeMap<String, usize>,
    ) -> LoweredSurfaceModule {
        let Some(core) = &checked.core else {
            return checked.clone();
        };
        let mut function_indexes = reachable_core_function_names(module)
            .into_iter()
            .filter_map(|name| core_functions_by_name.get(&name))
            .copied()
            .collect::<Vec<_>>();
        function_indexes.sort_unstable();
        let functions = function_indexes
            .into_iter()
            .map(|index| core.functions[index].clone())
            .collect();
        LoweredSurfaceModule {
            diagnostics: reachable_diagnostics(module, &checked.diagnostics),
            core: Some(CheckedProgram {
                functions,
                effects: core.effects.clone(),
                readiness: core.readiness.clone(),
            }),
            ir: None,
        }
    }
}

fn reachable_diagnostics(module: &SurfaceModule, diagnostics: &[Diagnostic]) -> Vec<Diagnostic> {
    let spans = reachable_declaration_spans(module);
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_none_or(|span| spans.iter().any(|decl| span_within(span, decl)))
        })
        .cloned()
        .collect()
}

fn reachable_declaration_spans(module: &SurfaceModule) -> Vec<&SourceSpan> {
    let mut spans = Vec::new();
    spans.extend(module.uses.iter().map(|decl| &decl.span));
    spans.extend(module.aliases.iter().map(|decl| &decl.span));
    spans.extend(module.effects.iter().map(|decl| &decl.span));
    spans.extend(module.handlers.iter().map(|decl| &decl.span));
    spans.extend(module.types.iter().map(|decl| &decl.span));
    spans.extend(module.schemas.iter().map(|decl| &decl.span));
    spans.extend(module.codecs.iter().map(|decl| &decl.span));
    spans.extend(module.functions.iter().map(|decl| &decl.span));
    spans
}

fn span_within(span: &SourceSpan, container: &SourceSpan) -> bool {
    span.file == container.file
        && span.start.offset >= container.start.offset
        && span.end.offset <= container.end.offset
}

fn checked_core_functions_by_name(checked: &LoweredSurfaceModule) -> BTreeMap<String, usize> {
    checked
        .core
        .as_ref()
        .map(|core| {
            core.functions
                .iter()
                .enumerate()
                .map(|(index, function)| (function.name.clone(), index))
                .collect()
        })
        .unwrap_or_default()
}

fn reachable_core_function_names(module: &SurfaceModule) -> BTreeSet<String> {
    let mut names = module
        .functions
        .iter()
        .filter_map(surface_function_core_name)
        .collect::<BTreeSet<_>>();
    names.extend(module.handlers.iter().flat_map(|handler| {
        handler.operation_clauses.iter().map(|clause| {
            core_function_name_for_module(
                handler.module_name.as_deref(),
                &synthetic_handler_clause_function_name(
                    handler.name.as_deref().unwrap_or("missing"),
                    clause.operation.as_deref().unwrap_or("missing"),
                ),
            )
        })
    }));
    names
}

fn surface_function_core_name(function: &veln_ast::Function) -> Option<String> {
    function
        .name
        .as_ref()
        .map(|name| core_function_name_for_module(function.module_name.as_deref(), name))
}

fn core_function_name_for_module(module_name: Option<&str>, name: &str) -> String {
    let Some(module_name) = module_name else {
        return name.to_string();
    };
    let Some(standard_module) = module_name.strip_prefix("std::") else {
        return name.to_string();
    };
    format!("__veln_std${}${name}", standard_module.replace("::", "$"))
}

fn synthetic_handler_clause_function_name(handler: &str, operation: &str) -> String {
    format!(
        "__handler_{}${handler}_{}${operation}",
        handler.len(),
        operation.len()
    )
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
            let checked = lower_reusable_standard_surface_module_core(&module, &environment);
            let checked_core_functions_by_name = checked_core_functions_by_name(&checked);
            ReusableStandardInput {
                module: Arc::new(module),
                environment,
                checked,
                checked_core_functions_by_name,
            }
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use veln_diagnostics::diagnostic_to_json;
    use veln_project::Project;
    use veln_sema::reachable_lowering_counters;
    use veln_source::SourceFile;

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
    fn reachable_lowering_reuses_checked_application_core_for_unrelated_annotated_modules() {
        let mut files = vec![SourceFile::new(
            "src/main.veln",
            concat!(
                "fn helper() -> Int\n",
                "  1\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  helper()\n",
                "end\n",
            ),
        )];
        for index in 0..24 {
            files.push(SourceFile::new(
                format!("src/unrelated_{index}.veln"),
                format!(
                    "fn unrelated_{index}(value: Int) -> Int\n\
                     \tvalue + {index}\n\
                     end\n"
                ),
            ));
        }
        let analysis = analyze_project(
            Project {
                root: ".".into(),
                files,
                manifest: None,
            },
            DoctestMode::Exclude,
        );
        let checked_diagnostics = analysis.checked_diagnostics();
        assert!(checked_diagnostics.is_empty(), "{checked_diagnostics:#?}");

        reachable_lowering_counters::reset();
        let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);

        assert!(reachable.lowered.diagnostics.is_empty());
        assert_eq!(reachable_lowering_counters::application_body_checks(), 0);
        assert_eq!(reachable_lowering_counters::application_core_lowers(), 0);
        assert_eq!(
            lowered_core_function_names(&reachable),
            ["helper".to_string(), "main".to_string()]
        );
    }

    fn diagnostic_json(diagnostics: &[veln_diagnostics::Diagnostic]) -> Vec<String> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic_to_json(diagnostic).to_json())
            .collect()
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

    fn lowered_core_function_names(analysis: &ReachableEntryAnalysis) -> Vec<String> {
        let mut names = analysis
            .lowered
            .core
            .as_ref()
            .expect("reachable core should lower")
            .functions
            .iter()
            .map(|function| function.name.clone())
            .collect::<Vec<_>>();
        names.sort();
        names
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
