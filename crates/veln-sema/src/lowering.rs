use veln_ast::{
    BinaryOp, BodyLine, BodyLineKind, DictEntry, EffectDecl, EffectOperationDecl, Expr, ExprKind,
    Function, FunctionKind, HandlerDecl, IfBranch, MatchArm, Pattern, PatternKind, RecordField,
    SurfaceModule, Visibility,
};
use veln_core::{
    CheckedProgram, ContractObligationStatus, CoreBlocker, CoreCallTarget, CoreCallbackTarget,
    CoreCleanupRegion, CoreContract, CoreContractCall, CoreDeferredBlock, CoreDeferredCapture,
    CoreDictEntry, CoreEffectDecl, CoreEffectOperationDecl, CoreExpr, CoreExprKind, CoreFunction,
    CoreHandlerProvider, CoreMatchArm, CoreParam, CorePattern, CorePatternField, CorePatternKind,
    CoreReadiness, CoreRecordField, CoreStmt, CoreStmtKind, CoreType,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_literals::parse_integer_literal;

use crate::adt::descriptors::{AdtConstructor, AdtVariantKind};
use crate::adt::registry::ConstructorLookup;
use crate::adt::{type_operations as adt, unification};
use crate::call_resolution::CoreCallSignature;
use crate::contracts::contract_predicate_is_statically_true;
use crate::effects::{core_concurrency_signature, is_concurrency_call};
use crate::prelude::{
    float_arithmetic_prelude_name, float_comparison_prelude_name, float_prefix_prelude_name,
};
use crate::semantic_model::Type;
use crate::type_lowering::core_type;
use crate::type_syntax::{parse_type_annotation, parse_type_or_unknown};
use crate::types::environment::TypeEnvironment;
use crate::types::signatures::{
    FunctionLookup, HandlerPathResolution, SCHEMA_DECODE_STEP_TARGET_PREFIX,
    SCHEMA_ENCODE_TARGET_PREFIX, SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX, UserEffectPathResolution,
    synthetic_handler_clause_function_name,
};

mod call_lowering;
mod data_and_control_flow;
mod expression_lowering;
mod function_lowering;

#[derive(Clone)]
struct CoreBinding {
    name: String,
    ty: CoreType,
}

struct IfLoweringTarget<'a> {
    node_id: veln_ast::NodeId,
    span: &'a veln_source::SourceSpan,
}

struct CoreLowerer<'a> {
    function: &'a Function,
    environment: &'a TypeEnvironment,
    block_unsupported_callsite_runtime: bool,
    bindings: Vec<CoreBinding>,
    blockers: Vec<CoreBlocker>,
    diagnostics: Vec<Diagnostic>,
    generated_local_count: usize,
    defer_capture_boundaries: Vec<usize>,
    defer_captures: Vec<Vec<CoreDeferredCapture>>,
}

impl CoreLowerer<'_> {
    fn core_type_annotation(&self, annotation: &str) -> CoreType {
        let ty = parse_type_or_unknown(Some(annotation));
        let ty = self
            .environment
            .canonicalize_type_annotation(ty, self.function.module_name.as_deref());
        core_type(&ty)
    }

    fn parsed_core_type(&self, ty: Type) -> CoreType {
        let ty = self
            .environment
            .canonicalize_type_annotation(ty, self.function.module_name.as_deref());
        core_type(&ty)
    }
}

pub(crate) struct CoreLoweringOutput {
    pub(crate) program: CheckedProgram,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
struct LoweredFunctions {
    functions: Vec<CoreFunction>,
    blockers: Vec<CoreBlocker>,
    diagnostics: Vec<Diagnostic>,
}

impl LoweredFunctions {
    fn push(
        &mut self,
        function: &Function,
        environment: &TypeEnvironment,
        block_unsupported_callsite_runtime: bool,
    ) {
        let mut lowerer =
            CoreLowerer::new(function, environment, block_unsupported_callsite_runtime);
        self.functions.push(lowerer.lower_function());
        self.blockers.extend(lowerer.blockers);
        self.diagnostics.extend(lowerer.diagnostics);
    }
}

pub(crate) fn lower_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CoreLoweringOutput {
    lower_surface_module_to_core_if(module, environment, false, |_| true)
}

pub(crate) fn lower_executable_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CoreLoweringOutput {
    lower_surface_module_to_core_if(module, environment, true, |_| true)
}

pub(crate) fn lower_project_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CoreLoweringOutput {
    let has_application_functions = module.functions.iter().any(|function| {
        !function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    });
    lower_surface_module_to_core_if(module, environment, false, |function| {
        !has_application_functions
            || !function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
    })
}

fn lower_surface_module_to_core_if(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    block_unsupported_callsite_runtime: bool,
    include: impl Fn(&Function) -> bool,
) -> CoreLoweringOutput {
    let LoweredFunctions {
        functions,
        blockers,
        diagnostics,
    } = lower_module_functions(
        module,
        environment,
        block_unsupported_callsite_runtime,
        include,
    );
    let readiness = if blockers.is_empty() {
        CoreReadiness::Complete
    } else {
        CoreReadiness::Blocked(blockers)
    };
    CoreLoweringOutput {
        program: CheckedProgram {
            functions,
            effects: module
                .effects
                .iter()
                .filter_map(|effect| lower_effect_decl(effect, environment))
                .collect(),
            readiness,
        },
        diagnostics,
    }
}

fn lower_module_functions(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    block_unsupported_callsite_runtime: bool,
    include: impl Fn(&Function) -> bool,
) -> LoweredFunctions {
    let mut lowered = LoweredFunctions::default();
    for function in module.functions.iter().filter(|function| include(function)) {
        lowered.push(function, environment, block_unsupported_callsite_runtime);
    }
    for handler in &module.handlers {
        for function in lower_handler_clause_functions(handler, environment) {
            lowered.push(&function, environment, block_unsupported_callsite_runtime);
        }
    }
    lowered
}

fn lower_effect_decl(effect: &EffectDecl, environment: &TypeEnvironment) -> Option<CoreEffectDecl> {
    Some(CoreEffectDecl {
        node_id: effect.node_id,
        name: effect.name.clone()?,
        visibility: effect.visibility,
        operations: effect
            .operations
            .iter()
            .filter_map(|operation| lower_effect_operation(operation, effect, environment))
            .collect(),
        span: effect.span.clone(),
    })
}

fn lower_effect_operation(
    operation: &EffectOperationDecl,
    effect: &EffectDecl,
    environment: &TypeEnvironment,
) -> Option<CoreEffectOperationDecl> {
    let lower_type = |annotation| {
        core_type(&environment.canonicalize_type_annotation(
            parse_type_or_unknown(annotation),
            effect.module_name.as_deref(),
        ))
    };
    Some(CoreEffectOperationDecl {
        node_id: operation.node_id,
        name: operation.name.clone()?,
        params: operation
            .params
            .iter()
            .map(|param| lower_type(param.ty.as_deref()))
            .collect(),
        return_type: lower_type(operation.return_type.as_deref()),
        span: operation.span.clone(),
    })
}

fn lower_handler_clause_functions(
    handler: &HandlerDecl,
    environment: &TypeEnvironment,
) -> Vec<Function> {
    let effect = match environment
        .resolve_user_effect_path(&handler.effect, handler.module_name.as_deref())
    {
        UserEffectPathResolution::Found(effect) => effect,
        UserEffectPathResolution::PrivateCompanionTargetMismatch { .. }
        | UserEffectPathResolution::QuarantinedImportTarget
        | UserEffectPathResolution::Missing => return Vec::new(),
    };
    handler
        .operation_clauses
        .iter()
        .filter_map(|clause| {
            let operation_name = clause.operation.as_deref()?;
            let operation = effect
                .operations
                .iter()
                .find(|operation| operation.name == operation_name)?;
            Some(lower_handler_clause_function(handler, clause, operation))
        })
        .collect()
}

fn lower_handler_clause_function(
    handler: &HandlerDecl,
    clause: &veln_ast::HandlerOperationClauseDecl,
    operation: &crate::types::signatures::EffectOperationSignature,
) -> Function {
    Function {
        node_id: clause.node_id,
        module_name: handler.module_name.clone(),
        package_name: handler.package_name.clone(),
        kind: FunctionKind::Function,
        visibility: Visibility::Private,
        name: Some(synthetic_handler_clause_function_name(
            handler.name.as_deref().unwrap_or("missing"),
            &operation.name,
        )),
        effect_binder: None,
        params: lower_handler_clause_params(handler, clause, operation),
        return_binding: None,
        return_type: Some(operation.return_type.render()),
        return_type_span: Some(operation.name_span.clone()),
        return_type_paths: Vec::new(),
        return_type_refinements: Vec::new(),
        effects: None,
        effect_spans: None,
        callsite: None,
        contracts: Vec::new(),
        body: vec![BodyLine {
            node_id: clause.body.node_id,
            kind: BodyLineKind::Expr {
                expr: clause.body.clone(),
            },
            span: clause.body.span.clone(),
        }],
        span: clause.span.clone(),
    }
}

fn lower_handler_clause_params(
    handler: &HandlerDecl,
    clause: &veln_ast::HandlerOperationClauseDecl,
    operation: &crate::types::signatures::EffectOperationSignature,
) -> Vec<veln_ast::Param> {
    let mut params = handler.params.clone();
    params.extend(
        clause
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| veln_ast::Param {
                node_id: param.node_id,
                name: param.name.clone(),
                name_span: param.name_span.clone(),
                ty: operation.params.get(index).map(Type::render),
                ty_span: None,
                ty_paths: Vec::new(),
                ty_refinements: Vec::new(),
                is_variadic: false,
                span: param.span.clone(),
            }),
    );
    params
}

fn callee_symbol(callee: &Expr) -> Option<String> {
    callee
        .callee_name_path()
        .map(|segments| segments.join("::"))
}

fn expected_concurrency_type_arg_count(segments: &[String]) -> Option<usize> {
    match segments {
        [module, name] if module == "task" && name == "spawn_with" => Some(2),
        [module, _] if module == "channel" || module == "task" => Some(1),
        _ => None,
    }
}

fn render_core_type(ty: &CoreType) -> String {
    match ty {
        CoreType::Unknown => "unknown".to_string(),
        CoreType::Named { name, args } if name == "Unit" && args.is_empty() => "()".to_string(),
        CoreType::Named { name, args } if args.is_empty() => name.clone(),
        CoreType::Named { name, args } => {
            let args = args
                .iter()
                .map(render_core_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}<{args}>")
        }
        CoreType::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(name, ty)| format!("{name}: {}", render_core_type(ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        CoreType::Function {
            params,
            variadic,
            return_type,
            effects,
        } => {
            let mut rendered_params = params.iter().map(render_core_type).collect::<Vec<_>>();
            if let Some(variadic) = variadic {
                rendered_params.push(format!("...{}", render_core_type(variadic)));
            }
            let params = rendered_params.join(", ");
            let effects = if effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", effects.join(", "))
            };
            format!("fn({params}) -> {}{effects}", render_core_type(return_type))
        }
    }
}

fn core_type_contains_unknown(ty: &CoreType) -> bool {
    match ty {
        CoreType::Unknown => true,
        CoreType::Named { args, .. } => args.iter().any(core_type_contains_unknown),
        CoreType::Record(fields) => fields
            .iter()
            .any(|(_, field_ty)| core_type_contains_unknown(field_ty)),
        CoreType::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(core_type_contains_unknown)
                || variadic.as_deref().is_some_and(core_type_contains_unknown)
                || core_type_contains_unknown(return_type)
        }
    }
}

fn constructor_arity_reason(constructor: AdtConstructor) -> &'static str {
    match constructor.descriptor.type_name.as_str() {
        "Option" => "option_constructor_arity_mismatch",
        "Result" => "result_constructor_arity_mismatch",
        _ => "constructor_arity_mismatch",
    }
}

fn core_nullary_constructor_kind(constructor: AdtConstructor) -> CoreExprKind {
    match constructor.variant.kind {
        AdtVariantKind::OptionNone => CoreExprKind::OptionNone,
        AdtVariantKind::ListNil => CoreExprKind::ListNil,
        AdtVariantKind::Source => CoreExprKind::AdtVariant {
            name: vec![
                constructor.descriptor.type_name.clone(),
                constructor.variant.name.clone(),
            ],
            payloads: Vec::new(),
        },
        _ => CoreExprKind::Missing,
    }
}

fn core_payload_constructor_kind(
    constructor: AdtConstructor,
    mut payloads: Vec<CoreExpr>,
) -> CoreExprKind {
    match constructor.variant.kind {
        AdtVariantKind::OptionSome => CoreExprKind::OptionSome(Box::new(payloads.remove(0))),
        AdtVariantKind::ResultOk => CoreExprKind::ResultOk(Box::new(payloads.remove(0))),
        AdtVariantKind::ResultErr => CoreExprKind::ResultErr(Box::new(payloads.remove(0))),
        AdtVariantKind::OptionNone => CoreExprKind::OptionNone,
        AdtVariantKind::ListNil => CoreExprKind::ListNil,
        AdtVariantKind::ListCons => {
            let head = payloads.remove(0);
            let tail = payloads.remove(0);
            CoreExprKind::ListCons {
                head: Box::new(head),
                tail: Box::new(tail),
            }
        }
        AdtVariantKind::Source => CoreExprKind::AdtVariant {
            name: vec![
                constructor.descriptor.type_name.clone(),
                constructor.variant.name.clone(),
            ],
            payloads,
        },
    }
}

fn is_ordering_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}

fn binary_operand_and_result(op: BinaryOp, numeric_type: CoreType) -> (CoreType, CoreType) {
    match op {
        BinaryOp::Or | BinaryOp::And => (CoreType::bool(), CoreType::bool()),
        BinaryOp::BitwiseOr
        | BinaryOp::BitwiseXor
        | BinaryOp::BitwiseAnd
        | BinaryOp::ShiftLeft
        | BinaryOp::ShiftRight
        | BinaryOp::ShiftRightLogical => (CoreType::int(), CoreType::int()),
        BinaryOp::Equal | BinaryOp::NotEqual => (CoreType::Unknown, CoreType::bool()),
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
            (numeric_type, CoreType::bool())
        }
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
            (numeric_type.clone(), numeric_type)
        }
        BinaryOp::PipeGreater => unreachable!("pipeline handled before binary lowering"),
    }
}
