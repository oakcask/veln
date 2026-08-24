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
    lower_project_reachable_surface_modules_with_standard_environment,
    prepare_current_reusable_standard_surface_module_environment,
};
use veln_source::SourceSpan;
use veln_test::{DoctestExpectation, doctest_sources, reconcile_expected_doctest_failures};

use crate::surface::{
    CapturedDependencyProject, CasingNameClass, CasingRecoveryRecord, ReachabilityCache,
    load_embedded_standard_surface_module_for_names, load_surface_modules,
    load_surface_modules_with_captured_dependencies, reachable_entry_module_with_standard_cache,
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
    casing_records: Vec<CasingRecoveryRecord>,
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
        casing_records: loaded.casing_records,
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
        casing_records: loaded.casing_records,
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

    pub fn invalid_entry_casing_diagnostics(&self, entry: &str) -> Vec<Diagnostic> {
        self.casing_records
            .iter()
            .filter(|record| {
                record.name_class == CasingNameClass::ValueBinding
                    && record.enclosing_function.as_deref() == Some(entry)
            })
            .map(|record| record.diagnostic.clone())
            .collect()
    }

    pub fn semantic_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(
            self.casing_records
                .iter()
                .map(|record| record.diagnostic.clone()),
        );
        diagnostics.extend(self.semantic_diagnostics.clone());
        self.reconcile_doctest_failures(diagnostics)
    }

    pub fn checked_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.source_diagnostics.clone();
        diagnostics.extend(
            self.casing_records
                .iter()
                .map(|record| record.diagnostic.clone()),
        );
        diagnostics.extend(filter_recovery_derivative_diagnostics(
            self.checked.diagnostics.clone(),
            &self.casing_records,
        ));
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
        let module = reachable_entry_module_with_standard_cache(
            &self.selected_standard,
            &self.module,
            entry,
            entry_kind,
            &self.reachability_cache,
        );
        let standard = standard_environment_for_modules(&self.selected_standard_module_names);
        let mut lowered = lower_project_reachable_surface_modules_with_standard_environment(
            &module,
            &self.selected_standard,
            &standard.environment,
        );
        let reachable_casing = reachable_casing_diagnostics(&module, &self.casing_records);
        lowered.diagnostics.extend(reachable_casing);
        lowered.diagnostics =
            filter_recovery_derivative_diagnostics(lowered.diagnostics, &self.casing_records);
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

fn reachable_casing_diagnostics(
    module: &SurfaceModule,
    records: &[CasingRecoveryRecord],
) -> Vec<Diagnostic> {
    records
        .iter()
        .filter(|record| match record.name_class {
            CasingNameClass::ValueBinding => {
                record.enclosing_function.as_ref().is_some_and(|name| {
                    module.functions.iter().any(|function| {
                        function.name.as_ref() == Some(name)
                            && function.module_name == record.module_name
                    })
                })
            }
            _ => module.functions.iter().any(|function| {
                function.module_name == record.module_name
                    && function_references_record(function, record)
            }),
        })
        .map(|record| record.diagnostic.clone())
        .collect()
}

fn function_references_record(
    function: &veln_ast::Function,
    record: &CasingRecoveryRecord,
) -> bool {
    function_signature_references_name(function, &record.name)
        || function.body.iter().any(|line| match &line.kind {
            veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
                pattern_references_name(pattern, &record.name)
                    || expr_references_name(expr, &record.name)
            }
            veln_ast::BodyLineKind::Expr { expr } => expr_references_name(expr, &record.name),
        })
}

fn function_signature_references_name(function: &veln_ast::Function, name: &str) -> bool {
    function.params.iter().any(|param| {
        param
            .ty
            .as_deref()
            .is_some_and(|ty| type_text_references_name(ty, name))
    }) || function
        .return_type
        .as_deref()
        .is_some_and(|ty| type_text_references_name(ty, name))
}

fn type_text_references_name(ty: &str, name: &str) -> bool {
    ty.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
        .any(|part| part == name || part.rsplit_once("::").is_some_and(|(_, leaf)| leaf == name))
}

fn pattern_references_name(pattern: &veln_ast::Pattern, name: &str) -> bool {
    match &pattern.kind {
        veln_ast::PatternKind::Binding(binding) => binding == name,
        veln_ast::PatternKind::Constructor {
            name: segments,
            args,
        } => {
            segments.last().is_some_and(|segment| segment == name)
                || args.iter().any(|arg| pattern_references_name(arg, name))
        }
        veln_ast::PatternKind::Record(fields) => fields
            .iter()
            .any(|field| pattern_references_name(&field.pattern, name)),
        _ => false,
    }
}

fn expr_references_name(expr: &veln_ast::Expr, name: &str) -> bool {
    match &expr.kind {
        veln_ast::ExprKind::NamePath(segments) => {
            segments.last().is_some_and(|segment| segment == name)
        }
        veln_ast::ExprKind::TypeApply { callee, .. } => expr_references_name(callee, name),
        veln_ast::ExprKind::Call { callee, args } => {
            expr_references_name(callee, name)
                || args.iter().any(|arg| expr_references_name(arg, name))
        }
        veln_ast::ExprKind::Perform { args, .. } => {
            args.iter().any(|arg| expr_references_name(arg, name))
        }
        veln_ast::ExprKind::Handle { body, args, .. } => {
            expr_references_name(body, name)
                || args.iter().any(|arg| expr_references_name(arg, name))
        }
        veln_ast::ExprKind::SchemaDecode { input, base, .. } => {
            expr_references_name(input, name) || expr_references_name(base, name)
        }
        veln_ast::ExprKind::SchemaEncode { value, .. }
        | veln_ast::ExprKind::FieldAccess { base: value, .. }
        | veln_ast::ExprKind::Try(value)
        | veln_ast::ExprKind::Prefix { expr: value, .. } => expr_references_name(value, name),
        veln_ast::ExprKind::Record(fields) => fields
            .iter()
            .any(|field| expr_references_name(&field.expr, name)),
        veln_ast::ExprKind::Dict(entries) => entries.iter().any(|entry| {
            expr_references_name(&entry.key, name) || expr_references_name(&entry.value, name)
        }),
        veln_ast::ExprKind::List(items) => {
            items.iter().any(|item| expr_references_name(item, name))
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            expr_references_name(scrutinee, name)
                || arms.iter().any(|arm| {
                    pattern_references_name(&arm.pattern, name)
                        || expr_references_name(&arm.expr, name)
                })
        }
        veln_ast::ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            expr_references_name(condition, name)
                || expr_references_name(then_branch, name)
                || else_if_branches.iter().any(|branch| {
                    expr_references_name(&branch.condition, name)
                        || expr_references_name(&branch.expr, name)
                })
                || expr_references_name(else_branch, name)
        }
        veln_ast::ExprKind::Binary { left, right, .. } => {
            expr_references_name(left, name) || expr_references_name(right, name)
        }
        _ => false,
    }
}

fn filter_recovery_derivative_diagnostics(
    diagnostics: Vec<Diagnostic>,
    records: &[CasingRecoveryRecord],
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| !is_unique_recovery_derivative(diagnostic, records))
        .collect()
}

fn is_unique_recovery_derivative(
    diagnostic: &Diagnostic,
    records: &[CasingRecoveryRecord],
) -> bool {
    let Some((name, compatible_name_class)) = recovery_derivative_name_and_class(diagnostic) else {
        return false;
    };
    let diagnostic_source = diagnostic.span.as_ref().map(|span| &span.file);
    records
        .iter()
        .filter(|record| {
            record.name == name.as_str()
                && compatible_name_class == record.name_class
                && diagnostic_source == Some(&record.source_path)
                && unresolved_span_is_in_recovery_scope(diagnostic, record)
        })
        .take(2)
        .count()
        == 1
}

fn recovery_derivative_name_and_class(
    diagnostic: &Diagnostic,
) -> Option<(String, CasingNameClass)> {
    match diagnostic.id.as_str() {
        "name.unresolved" => {
            let name = unresolved_name_from_message(&diagnostic.message)?.to_string();
            let namespace = diagnostic_string_detail(diagnostic, "namespace")?;
            Some((name, recovery_name_class_for_namespace(namespace)?))
        }
        "hole.unfilled" => {
            let label = diagnostic_string_detail(diagnostic, "label")?;
            Some((label.to_string(), CasingNameClass::ValueBinding))
        }
        _ => None,
    }
}

fn diagnostic_string_detail<'a>(diagnostic: &'a Diagnostic, key: &str) -> Option<&'a str> {
    let veln_diagnostics::JsonValue::Object(entries) = &diagnostic.details else {
        return None;
    };
    entries.iter().find_map(|(entry_key, value)| {
        if entry_key == key
            && let veln_diagnostics::JsonValue::String(value) = value
        {
            Some(value.as_str())
        } else {
            None
        }
    })
}

fn recovery_name_class_for_namespace(namespace: &str) -> Option<CasingNameClass> {
    match namespace {
        "call_target" => Some(CasingNameClass::Function),
        "value" | "contract_predicate" => Some(CasingNameClass::ValueBinding),
        _ => None,
    }
}

fn unresolved_span_is_in_recovery_scope(
    diagnostic: &Diagnostic,
    record: &CasingRecoveryRecord,
) -> bool {
    if record.name_class != CasingNameClass::ValueBinding {
        return true;
    }
    let Some(function_name) = record.enclosing_function.as_deref() else {
        return false;
    };
    let Some(span) = diagnostic.span.as_ref() else {
        return false;
    };
    record.lexical_scope.as_ref().is_some_and(|scope| {
        scope.file == span.file
            && scope.start.offset <= span.start.offset
            && span.end.offset <= scope.end.offset
    }) && !function_name.is_empty()
}

fn unresolved_name_from_message(message: &str) -> Option<&str> {
    message
        .rsplit_once('`')?
        .0
        .rsplit_once('`')
        .map(|(_, name)| name)
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
