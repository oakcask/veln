use std::collections::BTreeSet;

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
use veln_core::{
    CheckedProgram, CoreExpr, CoreExprKind, CoreFunction, CorePattern, CorePatternKind,
    CoreReadiness, CoreStmt, CoreStmtKind,
};
use veln_diagnostics::{Diagnostic, Severity};
use veln_ir::{TypedProgram, lower_checked_core};

use crate::analysis::{
    check_declared_effect_labels, check_duplicate_constructor_names, check_duplicate_effect_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_function_body, check_handler_declarations,
    check_module_boundary, check_public_aliases, check_public_function_boundary,
    check_reserved_prelude_aliases, check_schema_field_primitives, check_schema_type_references,
    check_test_declaration_boundary,
};
use crate::lowering::{lower_project_surface_module_to_core, lower_surface_module_to_core};
use crate::schema;
use crate::types::{
    ReusableStandardEnvironment, TypeEnvironment, prepare_current_reusable_standard_environment,
    prepare_reusable_standard_environment,
};

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

#[cfg(any(test, debug_assertions))]
pub mod reachable_lowering_counters {
    use std::cell::Cell;

    use veln_ast::Function;

    thread_local! {
        static APPLICATION_BODY_CHECKS: Cell<usize> = const { Cell::new(0) };
        static APPLICATION_CORE_LOWERS: Cell<usize> = const { Cell::new(0) };
    }

    pub fn reset() {
        APPLICATION_BODY_CHECKS.set(0);
        APPLICATION_CORE_LOWERS.set(0);
    }

    pub fn application_body_checks() -> usize {
        APPLICATION_BODY_CHECKS.get()
    }

    pub fn application_core_lowers() -> usize {
        APPLICATION_CORE_LOWERS.get()
    }

    pub(crate) fn record_application_body_check(function: &Function) {
        if is_application_function(function) {
            APPLICATION_BODY_CHECKS.set(APPLICATION_BODY_CHECKS.get() + 1);
        }
    }

    pub(crate) fn record_application_core_lower(function: &Function) {
        if is_application_function(function) {
            APPLICATION_CORE_LOWERS.set(APPLICATION_CORE_LOWERS.get() + 1);
        }
    }

    fn is_application_function(function: &Function) -> bool {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    }
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    let environment = TypeEnvironment::from_module(module);
    analyze_surface_module_with_environment(module, &environment, true)
}

pub fn check_project_surface_module(
    module: &SurfaceModule,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_modules_with_standard_environment(
    application_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_application_module_with_standard(
        application_module,
        selected_standard_module,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn check_project_surface_module_with_standard_modules_environment(
    application_module: &SurfaceModule,
    selected_standard_module_names: &BTreeSet<String>,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_application_module_with_standard_module_names(
        application_module,
        selected_standard_module_names,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn prepare_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    prepare_reusable_standard_environment(module)
}

pub fn prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    prepare_current_reusable_standard_environment(module)
}

pub fn lower_reusable_standard_surface_module_core(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    let lowered_core = lower_surface_module_to_core(module, &environment);
    LoweredSurfaceModule {
        diagnostics: lowered_core.diagnostics,
        core: Some(lowered_core.program),
        ir: None,
    }
}

fn check_project_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let validate_standard_bodies = should_validate_standard_bodies(module);
    let semantic_diagnostics =
        analyze_surface_module_with_environment(module, &environment, validate_standard_bodies);
    let checked = lower_analyzed_surface_module_with_environment(
        module,
        semantic_diagnostics.clone(),
        &environment,
        true,
    );
    (semantic_diagnostics, checked)
}

fn analyze_surface_module_with_environment(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    validate_standard_bodies: bool,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_effect_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module));
    diagnostics.extend(check_schema_field_primitives(module));
    diagnostics.extend(check_schema_type_references(module));
    diagnostics.extend(check_handler_declarations(module, environment));

    for function in &module.functions {
        if !validate_standard_bodies
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            continue;
        }
        diagnostics.extend(check_declared_effect_labels(function, environment));
        if function.visibility == Visibility::Public {
            diagnostics.extend(check_public_function_boundary(function));
        }
        if function.kind == FunctionKind::Test {
            diagnostics.extend(check_test_declaration_boundary(function));
        }
        #[cfg(any(test, debug_assertions))]
        reachable_lowering_counters::record_application_body_check(function);
        diagnostics.extend(check_function_body(function, environment));
    }

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    lower_analyzed_surface_module(module, analyze_surface_module(module))
}

pub fn lower_project_reachable_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module(module);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_modules_with_standard_environment(
    reachable_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_application_module_with_standard(
        reachable_module,
        selected_standard_module,
        standard,
    );
    lower_project_reachable_surface_module_with_environment(reachable_module, environment)
}

pub fn lower_reachable_checked_application_with_checked_standard(
    reachable_module: &SurfaceModule,
    checked_standard: LoweredSurfaceModule,
    checked_application: LoweredSurfaceModule,
) -> LoweredSurfaceModule {
    let (Some(standard_core), Some(application_core)) =
        (checked_standard.core, checked_application.core)
    else {
        return LoweredSurfaceModule {
            diagnostics: checked_application.diagnostics,
            core: None,
            ir: None,
        };
    };

    let mut diagnostics = checked_application.diagnostics;
    diagnostics.extend(checked_standard.diagnostics);
    let standard_functions = standard_core.functions;
    let standard_readiness = selected_readiness(&standard_core.readiness, &standard_functions);
    let application_functions = application_core.functions;
    let application_readiness =
        selected_readiness(&application_core.readiness, &application_functions);
    let mut functions = standard_functions;
    functions.extend(application_functions);
    let mut effects = standard_core.effects;
    effects.extend(application_core.effects);
    let program = CheckedProgram {
        functions,
        effects,
        readiness: merged_readiness(standard_readiness, application_readiness),
    };
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&program).ok().map(|mut ir| {
            ir.schema_decoders = schema::ir::schema_decode_specs(reachable_module);
            ir
        })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(program),
        ir,
    }
}

fn merged_readiness(standard: CoreReadiness, application: CoreReadiness) -> CoreReadiness {
    match (standard, application) {
        (CoreReadiness::Complete, CoreReadiness::Complete) => CoreReadiness::Complete,
        (CoreReadiness::Blocked(mut left), CoreReadiness::Blocked(right)) => {
            left.extend(right);
            CoreReadiness::Blocked(left)
        }
        (CoreReadiness::Blocked(blockers), CoreReadiness::Complete)
        | (CoreReadiness::Complete, CoreReadiness::Blocked(blockers)) => {
            CoreReadiness::Blocked(blockers)
        }
    }
}

fn selected_readiness(readiness: &CoreReadiness, functions: &[CoreFunction]) -> CoreReadiness {
    let CoreReadiness::Blocked(blockers) = readiness else {
        return CoreReadiness::Complete;
    };
    let node_ids = functions
        .iter()
        .flat_map(core_function_node_ids)
        .collect::<BTreeSet<_>>();
    let blockers = blockers
        .iter()
        .filter(|blocker| node_ids.contains(&blocker.node_id()))
        .cloned()
        .collect::<Vec<_>>();
    if blockers.is_empty() {
        CoreReadiness::Complete
    } else {
        CoreReadiness::Blocked(blockers)
    }
}

trait CoreBlockerNode {
    fn node_id(&self) -> veln_ast::NodeId;
}

impl CoreBlockerNode for veln_core::CoreBlocker {
    fn node_id(&self) -> veln_ast::NodeId {
        match self {
            Self::Hole { node_id }
            | Self::MissingExpression { node_id }
            | Self::UnsupportedExpression { node_id, .. } => *node_id,
        }
    }
}

fn core_function_node_ids(function: &CoreFunction) -> BTreeSet<veln_ast::NodeId> {
    let mut node_ids = BTreeSet::new();
    node_ids.insert(function.node_id);
    for param in &function.params {
        node_ids.insert(param.node_id);
    }
    for contract in &function.contracts {
        node_ids.insert(contract.node_id);
    }
    for stmt in &function.body {
        collect_stmt_node_ids(stmt, &mut node_ids);
    }
    node_ids
}

fn collect_stmt_node_ids(stmt: &CoreStmt, node_ids: &mut BTreeSet<veln_ast::NodeId>) {
    node_ids.insert(stmt.node_id);
    match &stmt.kind {
        CoreStmtKind::Let { expr, .. }
        | CoreStmtKind::Expr { expr }
        | CoreStmtKind::Return { expr } => collect_expr_node_ids(expr, node_ids),
    }
}

fn collect_expr_node_ids(expr: &CoreExpr, node_ids: &mut BTreeSet<veln_ast::NodeId>) {
    node_ids.insert(expr.node_id);
    match &expr.kind {
        CoreExprKind::ResultOk(inner)
        | CoreExprKind::ResultErr(inner)
        | CoreExprKind::OptionSome(inner)
        | CoreExprKind::Try(inner)
        | CoreExprKind::Prefix { expr: inner, .. } => collect_expr_node_ids(inner, node_ids),
        CoreExprKind::ListCons { head, tail } => {
            collect_expr_node_ids(head, node_ids);
            collect_expr_node_ids(tail, node_ids);
        }
        CoreExprKind::AdtVariant { payloads, .. }
        | CoreExprKind::Call { args: payloads, .. }
        | CoreExprKind::List(payloads) => {
            for payload in payloads {
                collect_expr_node_ids(payload, node_ids);
            }
        }
        CoreExprKind::Perform { args, .. } => {
            for arg in args {
                collect_expr_node_ids(arg, node_ids);
            }
        }
        CoreExprKind::Handle {
            context_args, body, ..
        } => {
            for arg in context_args {
                collect_expr_node_ids(arg, node_ids);
            }
            collect_expr_node_ids(body, node_ids);
        }
        CoreExprKind::FieldAccess { base, .. } => collect_expr_node_ids(base, node_ids),
        CoreExprKind::Record(fields) => {
            for field in fields {
                node_ids.insert(field.node_id);
                collect_expr_node_ids(&field.expr, node_ids);
            }
        }
        CoreExprKind::Dict(entries) => {
            for entry in entries {
                node_ids.insert(entry.node_id);
                collect_expr_node_ids(&entry.key, node_ids);
                collect_expr_node_ids(&entry.value, node_ids);
            }
        }
        CoreExprKind::Match { scrutinee, arms } => {
            collect_expr_node_ids(scrutinee, node_ids);
            for arm in arms {
                node_ids.insert(arm.node_id);
                collect_pattern_node_ids(&arm.pattern, node_ids);
                collect_expr_node_ids(&arm.expr, node_ids);
            }
        }
        CoreExprKind::Binary { left, right, .. } => {
            collect_expr_node_ids(left, node_ids);
            collect_expr_node_ids(right, node_ids);
        }
        CoreExprKind::Missing
        | CoreExprKind::Hole { .. }
        | CoreExprKind::Local(_)
        | CoreExprKind::BoolLiteral(_)
        | CoreExprKind::StringLiteral(_)
        | CoreExprKind::IntLiteral(_)
        | CoreExprKind::FloatLiteral(_)
        | CoreExprKind::Unit
        | CoreExprKind::FunctionValue(_)
        | CoreExprKind::OptionNone
        | CoreExprKind::ListNil => {}
    }
}

fn collect_pattern_node_ids(pattern: &CorePattern, node_ids: &mut BTreeSet<veln_ast::NodeId>) {
    node_ids.insert(pattern.node_id);
    match &pattern.kind {
        CorePatternKind::Record(fields) => {
            for field in fields {
                node_ids.insert(field.node_id);
                collect_pattern_node_ids(&field.pattern, node_ids);
            }
        }
        CorePatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_pattern_node_ids(arg, node_ids);
            }
        }
        CorePatternKind::Wildcard
        | CorePatternKind::Binding(_)
        | CorePatternKind::StringLiteral(_)
        | CorePatternKind::IntLiteral(_)
        | CorePatternKind::FloatLiteral(_)
        | CorePatternKind::BoolLiteral(_)
        | CorePatternKind::Unit => {}
    }
}

fn lower_project_reachable_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> LoweredSurfaceModule {
    let diagnostics = analyze_surface_module_with_environment(
        module,
        &environment,
        should_validate_standard_bodies(module),
    );
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn should_validate_standard_bodies(module: &SurfaceModule) -> bool {
    !module.functions.iter().any(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    })
}

pub fn lower_analyzed_surface_module(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> LoweredSurfaceModule {
    let environment = TypeEnvironment::from_module(module);
    lower_analyzed_surface_module_with_environment(module, diagnostics, &environment, false)
}

fn lower_analyzed_surface_module_with_environment(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    project_check: bool,
) -> LoweredSurfaceModule {
    let had_diagnostics_errors = diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if had_diagnostics_errors && !project_check {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
        };
    }

    let lowered_core = if project_check {
        lower_project_surface_module_to_core(module, environment)
    } else {
        lower_surface_module_to_core(module, environment)
    };
    if !had_diagnostics_errors {
        diagnostics.extend(lowered_core.diagnostics);
    }
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&lowered_core.program)
            .ok()
            .map(|mut ir| {
                ir.schema_decoders = schema::ir::schema_decode_specs(module);
                ir
            })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(lowered_core.program),
        ir,
    }
}
