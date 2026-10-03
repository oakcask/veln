use veln_ast::NodeId;
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreCallbackTarget, CoreExpr, CoreExprKind,
    CoreReadiness, CoreStmt, CoreStmtKind,
};

use crate::{
    IrCallTarget, IrCallbackTarget, IrCleanupRegion, IrContract, IrDeferredBlock,
    IrDeferredCapture, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrHandlerProvider, IrMatchArm,
    IrParam, IrPattern, IrPatternField, IrPatternKind, IrRecordField, IrStmt, IrStmtKind,
    TypedProgram,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrLowerError {
    Blocked(CoreBlocker),
    UnresolvedCallTarget { node_id: NodeId, symbol: String },
    MissingExpression { node_id: NodeId },
    Hole { node_id: NodeId },
}

pub fn lower_checked_core(program: &CheckedProgram) -> Result<TypedProgram, IrLowerError> {
    if let CoreReadiness::Blocked(blockers) = &program.readiness
        && let Some(blocker) = blockers.first()
    {
        return Err(IrLowerError::Blocked(blocker.clone()));
    }

    let functions = program
        .functions
        .iter()
        .map(lower_function)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypedProgram {
        functions,
        schema_decoders: Vec::new(),
    })
}

fn lower_function(function: &veln_core::CoreFunction) -> Result<IrFunction, IrLowerError> {
    Ok(IrFunction {
        node_id: function.node_id,
        name: function.name.clone(),
        visibility: function.visibility,
        params: function
            .params
            .iter()
            .map(|param| IrParam {
                node_id: param.node_id,
                name: param.name.clone(),
                ty: param.ty.clone(),
            })
            .collect(),
        return_binding: function.return_binding.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        callsite: function.callsite,
        contracts: function
            .contracts
            .iter()
            .map(|contract| IrContract {
                node_id: contract.node_id,
                kind: contract.kind,
                predicate: contract.predicate.clone(),
                callsite_calls: contract.callsite_calls.clone(),
                obligation_status: contract.obligation_status,
                span: contract.span.clone(),
            })
            .collect(),
        body: IrCleanupRegion::new(
            function
                .body
                .iter()
                .map(lower_stmt)
                .collect::<Result<Vec<_>, _>>()?,
        ),
    })
}

fn lower_stmt(stmt: &CoreStmt) -> Result<IrStmt, IrLowerError> {
    Ok(IrStmt {
        node_id: stmt.node_id,
        kind: match &stmt.kind {
            CoreStmtKind::Let { name, ty, expr } => IrStmtKind::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: lower_expr(expr)?,
            },
            CoreStmtKind::Expr { expr } => IrStmtKind::Expr {
                value: lower_expr(expr)?,
            },
            CoreStmtKind::Return { expr } => IrStmtKind::Return {
                value: lower_expr(expr)?,
            },
            CoreStmtKind::Defer(block) => IrStmtKind::Defer(IrDeferredBlock {
                captures: block
                    .captures
                    .iter()
                    .map(|capture| IrDeferredCapture {
                        name: capture.name.clone(),
                        ty: capture.ty.clone(),
                    })
                    .collect(),
                body: block
                    .body
                    .iter()
                    .map(lower_stmt)
                    .collect::<Result<Vec<_>, _>>()?,
            }),
        },
    })
}

fn lower_expr(expr: &CoreExpr) -> Result<IrExpr, IrLowerError> {
    Ok(IrExpr {
        node_id: expr.node_id,
        ty: expr.ty.clone(),
        span: expr.span.clone(),
        kind: lower_expr_kind(expr)?,
    })
}

fn lower_expr_kind(expr: &CoreExpr) -> Result<IrExprKind, IrLowerError> {
    if let Some(kind) = lower_blocked_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_scalar_expr(expr) {
        return Ok(kind);
    }
    if let Some(kind) = lower_wrapper_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_constructor_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_invocation_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_effect_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_access_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_collection_expr(expr)? {
        return Ok(kind);
    }
    lower_operator_expr(expr)
}

fn lower_blocked_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Missing => Err(IrLowerError::MissingExpression {
            node_id: expr.node_id,
        }),
        CoreExprKind::Hole { .. } => Err(IrLowerError::Hole {
            node_id: expr.node_id,
        }),
        _ => Ok(None),
    }
}

fn lower_scalar_expr(expr: &CoreExpr) -> Option<IrExprKind> {
    match &expr.kind {
        CoreExprKind::Local(name) => Some(IrExprKind::Local(name.clone())),
        CoreExprKind::BoolLiteral(value) => Some(IrExprKind::BoolLiteral(*value)),
        CoreExprKind::StringLiteral(value) => Some(IrExprKind::StringLiteral(value.clone())),
        CoreExprKind::IntLiteral(value) => Some(IrExprKind::IntLiteral(value.clone())),
        CoreExprKind::FloatLiteral(value) => Some(IrExprKind::FloatLiteral(value.clone())),
        CoreExprKind::Unit => Some(IrExprKind::Unit),
        CoreExprKind::FunctionValue { name, callsite } => Some(IrExprKind::FunctionValue {
            name: name.clone(),
            callsite: *callsite,
        }),
        CoreExprKind::OptionNone => Some(IrExprKind::OptionNone),
        CoreExprKind::ListNil => Some(IrExprKind::ListNil),
        _ => None,
    }
}

fn lower_wrapper_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::ResultOk(value) => lower_unary_expr(value, IrExprKind::ResultOk).map(Some),
        CoreExprKind::ResultErr(value) => lower_unary_expr(value, IrExprKind::ResultErr).map(Some),
        CoreExprKind::OptionSome(value) => {
            lower_unary_expr(value, IrExprKind::OptionSome).map(Some)
        }
        CoreExprKind::Try(value) => lower_unary_expr(value, IrExprKind::Try).map(Some),
        _ => Ok(None),
    }
}

fn lower_constructor_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::ListCons { head, tail } => Ok(Some(IrExprKind::ListCons {
            head: Box::new(lower_expr(head)?),
            tail: Box::new(lower_expr(tail)?),
        })),
        CoreExprKind::AdtVariant { name, payloads } => Ok(Some(IrExprKind::AdtVariant {
            name: name.clone(),
            payloads: lower_exprs(payloads)?,
        })),
        _ => Ok(None),
    }
}

fn lower_invocation_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Call { target, args } => {
            lower_call_expr(expr.node_id, target, args).map(Some)
        }
        _ => Ok(None),
    }
}

fn lower_effect_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Perform {
            effect,
            operation,
            args,
        } => Ok(Some(IrExprKind::Perform {
            effect: effect.clone(),
            operation: operation.clone(),
            args: lower_exprs(args)?,
        })),
        CoreExprKind::Handle {
            effect,
            providers,
            context_args,
            body,
        } => Ok(Some(IrExprKind::Handle {
            effect: effect.clone(),
            providers: providers
                .iter()
                .map(|provider| IrHandlerProvider {
                    operation: provider.operation.clone(),
                    function: provider.function.clone(),
                })
                .collect(),
            context_args: lower_exprs(context_args)?,
            body: Box::new(lower_expr(body)?),
        })),
        _ => Ok(None),
    }
}

fn lower_access_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::FieldAccess { base, field } => Ok(Some(IrExprKind::FieldAccess {
            base: Box::new(lower_expr(base)?),
            field: field.clone(),
        })),
        _ => Ok(None),
    }
}

fn lower_collection_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Record(fields) => lower_record_expr(fields).map(Some),
        CoreExprKind::Dict(entries) => lower_dict_expr(entries).map(Some),
        CoreExprKind::List(items) => Ok(Some(IrExprKind::List(lower_exprs(items)?))),
        CoreExprKind::Match { scrutinee, arms } => lower_match_expr(scrutinee, arms).map(Some),
        CoreExprKind::CleanupRegion { region } => Ok(Some(IrExprKind::CleanupRegion {
            region: IrCleanupRegion::new(
                region
                    .iter()
                    .map(lower_stmt)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        })),
        _ => Ok(None),
    }
}

fn lower_operator_expr(expr: &CoreExpr) -> Result<IrExprKind, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Prefix { op, expr } => Ok(IrExprKind::Prefix {
            op: *op,
            expr: Box::new(lower_expr(expr)?),
        }),
        CoreExprKind::Binary { op, left, right } => Ok(IrExprKind::Binary {
            op: *op,
            left: Box::new(lower_expr(left)?),
            right: Box::new(lower_expr(right)?),
        }),
        _ => unreachable!("core expression variant should be handled before operator lowering"),
    }
}

fn lower_unary_expr(
    expr: &CoreExpr,
    build: impl FnOnce(Box<IrExpr>) -> IrExprKind,
) -> Result<IrExprKind, IrLowerError> {
    Ok(build(Box::new(lower_expr(expr)?)))
}

fn lower_call_expr(
    node_id: NodeId,
    target: &veln_core::CoreCallTarget,
    args: &[CoreExpr],
) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Call {
        target: lower_call_target(node_id, target)?,
        args: lower_exprs(args)?,
    })
}

fn lower_exprs(exprs: &[CoreExpr]) -> Result<Vec<IrExpr>, IrLowerError> {
    exprs.iter().map(lower_expr).collect()
}

fn lower_record_expr(fields: &[veln_core::CoreRecordField]) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Record(
        fields
            .iter()
            .map(|field| {
                Ok(IrRecordField {
                    node_id: field.node_id,
                    name: field.name.clone(),
                    value: lower_expr(&field.expr)?,
                })
            })
            .collect::<Result<Vec<_>, IrLowerError>>()?,
    ))
}

fn lower_dict_expr(entries: &[veln_core::CoreDictEntry]) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Dict(
        entries
            .iter()
            .map(|entry| {
                Ok(IrDictEntry {
                    node_id: entry.node_id,
                    key: lower_expr(&entry.key)?,
                    value: lower_expr(&entry.value)?,
                })
            })
            .collect::<Result<Vec<_>, IrLowerError>>()?,
    ))
}

fn lower_match_expr(
    scrutinee: &CoreExpr,
    arms: &[veln_core::CoreMatchArm],
) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Match {
        scrutinee: Box::new(lower_expr(scrutinee)?),
        arms: arms
            .iter()
            .map(|arm| {
                Ok(IrMatchArm {
                    node_id: arm.node_id,
                    pattern: lower_pattern(&arm.pattern),
                    value: lower_expr(&arm.expr)?,
                })
            })
            .collect::<Result<Vec<_>, IrLowerError>>()?,
    })
}

fn lower_pattern(pattern: &veln_core::CorePattern) -> IrPattern {
    IrPattern {
        node_id: pattern.node_id,
        kind: match &pattern.kind {
            veln_core::CorePatternKind::Wildcard => IrPatternKind::Wildcard,
            veln_core::CorePatternKind::Binding(name) => IrPatternKind::Binding(name.clone()),
            veln_core::CorePatternKind::StringLiteral(value) => {
                IrPatternKind::StringLiteral(value.clone())
            }
            veln_core::CorePatternKind::IntLiteral(value) => {
                IrPatternKind::IntLiteral(value.clone())
            }
            veln_core::CorePatternKind::FloatLiteral(value) => {
                IrPatternKind::FloatLiteral(value.clone())
            }
            veln_core::CorePatternKind::BoolLiteral(value) => IrPatternKind::BoolLiteral(*value),
            veln_core::CorePatternKind::Unit => IrPatternKind::Unit,
            veln_core::CorePatternKind::Record(fields) => IrPatternKind::Record(
                fields
                    .iter()
                    .map(|field| IrPatternField {
                        node_id: field.node_id,
                        name: field.name.clone(),
                        pattern: lower_pattern(&field.pattern),
                    })
                    .collect(),
            ),
            veln_core::CorePatternKind::Constructor { name, args } => IrPatternKind::Constructor {
                name: name.clone(),
                args: args.iter().map(lower_pattern).collect(),
            },
        },
    }
}

fn lower_call_target(
    node_id: NodeId,
    target: &CoreCallTarget,
) -> Result<IrCallTarget, IrLowerError> {
    if let Some(target) = lower_schema_call_target(target) {
        return Ok(target);
    }
    lower_non_schema_call_target(node_id, target)
}

fn lower_schema_call_target(target: &CoreCallTarget) -> Option<IrCallTarget> {
    match target {
        CoreCallTarget::SchemaDecode(name) => Some(IrCallTarget::SchemaDecode(name.clone())),
        CoreCallTarget::SchemaDecodeStep(name) => {
            Some(IrCallTarget::SchemaDecodeStep(name.clone()))
        }
        CoreCallTarget::SchemaNeutralDecode(name) => {
            Some(IrCallTarget::SchemaNeutralDecode(name.clone()))
        }
        CoreCallTarget::SchemaNeutralEncode(name) => {
            Some(IrCallTarget::SchemaNeutralEncode(name.clone()))
        }
        CoreCallTarget::SchemaEncode(name) => Some(IrCallTarget::SchemaEncode(name.clone())),
        CoreCallTarget::SchemaEncodeStep(name) => {
            Some(IrCallTarget::SchemaEncodeStep(name.clone()))
        }
        CoreCallTarget::SchemaValidate(name) => Some(IrCallTarget::SchemaValidate(name.clone())),
        _ => None,
    }
}

fn lower_non_schema_call_target(
    node_id: NodeId,
    target: &CoreCallTarget,
) -> Result<IrCallTarget, IrLowerError> {
    match target {
        CoreCallTarget::Function(name) => Ok(IrCallTarget::Function(name.clone())),
        CoreCallTarget::StdioBuiltin(name) => Ok(IrCallTarget::StdioBuiltin(name.clone())),
        CoreCallTarget::ConcurrencyBuiltin(name) => {
            Ok(IrCallTarget::ConcurrencyBuiltin(name.clone()))
        }
        CoreCallTarget::StandardLibraryBuiltin(name) => {
            Ok(IrCallTarget::StandardLibraryBuiltin(name.clone()))
        }
        CoreCallTarget::PreludeBuiltin(name) => Ok(IrCallTarget::PreludeBuiltin(name.clone())),
        CoreCallTarget::CallbackBoundary { target, callsite } => {
            Ok(IrCallTarget::CallbackBoundary {
                target: match target {
                    CoreCallbackTarget::Function(name) => IrCallbackTarget::Function(name.clone()),
                    CoreCallbackTarget::ConcurrencyBuiltin(name) => {
                        IrCallbackTarget::ConcurrencyBuiltin(name.clone())
                    }
                    CoreCallbackTarget::StandardLibraryBuiltin(name) => {
                        IrCallbackTarget::StandardLibraryBuiltin(name.clone())
                    }
                    CoreCallbackTarget::PreludeBuiltin(name) => {
                        IrCallbackTarget::PreludeBuiltin(name.clone())
                    }
                },
                callsite: Box::new(lower_expr(callsite)?),
            })
        }
        CoreCallTarget::Value(name) => Ok(IrCallTarget::Value(name.clone())),
        CoreCallTarget::CallsiteValue { name, callsite } => Ok(IrCallTarget::CallsiteValue {
            name: name.clone(),
            callsite: Box::new(lower_expr(callsite)?),
        }),
        CoreCallTarget::Unresolved(symbol) => Err(IrLowerError::UnresolvedCallTarget {
            node_id,
            symbol: symbol.clone(),
        }),
        CoreCallTarget::SchemaDecode(_)
        | CoreCallTarget::SchemaDecodeStep(_)
        | CoreCallTarget::SchemaNeutralDecode(_)
        | CoreCallTarget::SchemaNeutralEncode(_)
        | CoreCallTarget::SchemaEncode(_)
        | CoreCallTarget::SchemaEncodeStep(_)
        | CoreCallTarget::SchemaValidate(_) => {
            unreachable!("schema call targets are lowered before this fallback")
        }
    }
}

#[cfg(test)]
#[path = "lowering/tests.rs"]
mod tests;
