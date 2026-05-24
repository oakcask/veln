//! Runtime-neutral typed IR.

use veln_ast::{BinaryOp, NodeId, PrefixOp, Visibility};
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreExpr, CoreExprKind, CoreReadiness, CoreStmt,
    CoreStmtKind, CoreType,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TypedProgram {
    pub functions: Vec<IrFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrFunction {
    pub node_id: NodeId,
    pub name: String,
    pub visibility: Visibility,
    pub params: Vec<IrParam>,
    pub return_type: CoreType,
    pub effects: Vec<String>,
    pub body: Vec<IrStmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrParam {
    pub node_id: NodeId,
    pub name: String,
    pub ty: CoreType,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrStmt {
    pub node_id: NodeId,
    pub kind: IrStmtKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrStmtKind {
    Let {
        name: String,
        ty: CoreType,
        value: IrExpr,
    },
    Expr {
        value: IrExpr,
    },
    Return {
        value: IrExpr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrExpr {
    pub node_id: NodeId,
    pub ty: CoreType,
    pub kind: IrExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrExprKind {
    Local(String),
    BoolLiteral(bool),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    ResultOk(Box<IrExpr>),
    ResultErr(Box<IrExpr>),
    OptionSome(Box<IrExpr>),
    Call {
        target: IrCallTarget,
        args: Vec<IrExpr>,
    },
    Try(Box<IrExpr>),
    Record(Vec<IrRecordField>),
    List(Vec<IrExpr>),
    Prefix {
        op: PrefixOp,
        expr: Box<IrExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<IrExpr>,
        right: Box<IrExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrCallTarget {
    Function(String),
    StdioBuiltin(String),
    Value(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrRecordField {
    pub node_id: NodeId,
    pub name: String,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrLowerError {
    Blocked(CoreBlocker),
    UnresolvedCallTarget { node_id: NodeId, symbol: String },
    MissingExpression { node_id: NodeId },
    Hole { node_id: NodeId },
}

pub fn lower_checked_core(program: &CheckedProgram) -> Result<TypedProgram, IrLowerError> {
    if let CoreReadiness::Blocked(blockers) = &program.readiness {
        if let Some(blocker) = blockers.first() {
            return Err(IrLowerError::Blocked(blocker.clone()));
        }
    }

    let functions = program
        .functions
        .iter()
        .map(lower_function)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypedProgram { functions })
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
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        body: function
            .body
            .iter()
            .map(lower_stmt)
            .collect::<Result<Vec<_>, _>>()?,
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
        },
    })
}

fn lower_expr(expr: &CoreExpr) -> Result<IrExpr, IrLowerError> {
    Ok(IrExpr {
        node_id: expr.node_id,
        ty: expr.ty.clone(),
        kind: match &expr.kind {
            CoreExprKind::Missing => {
                return Err(IrLowerError::MissingExpression {
                    node_id: expr.node_id,
                });
            }
            CoreExprKind::Hole { .. } => {
                return Err(IrLowerError::Hole {
                    node_id: expr.node_id,
                });
            }
            CoreExprKind::Local(name) => IrExprKind::Local(name.clone()),
            CoreExprKind::BoolLiteral(value) => IrExprKind::BoolLiteral(*value),
            CoreExprKind::StringLiteral(value) => IrExprKind::StringLiteral(value.clone()),
            CoreExprKind::IntLiteral(value) => IrExprKind::IntLiteral(value.clone()),
            CoreExprKind::FloatLiteral(value) => IrExprKind::FloatLiteral(value.clone()),
            CoreExprKind::Unit => IrExprKind::Unit,
            CoreExprKind::ResultOk(value) => IrExprKind::ResultOk(Box::new(lower_expr(value)?)),
            CoreExprKind::ResultErr(value) => IrExprKind::ResultErr(Box::new(lower_expr(value)?)),
            CoreExprKind::OptionSome(value) => IrExprKind::OptionSome(Box::new(lower_expr(value)?)),
            CoreExprKind::Call { target, args } => IrExprKind::Call {
                target: lower_call_target(expr.node_id, target)?,
                args: args.iter().map(lower_expr).collect::<Result<Vec<_>, _>>()?,
            },
            CoreExprKind::Try(value) => IrExprKind::Try(Box::new(lower_expr(value)?)),
            CoreExprKind::Record(fields) => IrExprKind::Record(
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
            ),
            CoreExprKind::List(items) => IrExprKind::List(
                items
                    .iter()
                    .map(lower_expr)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            CoreExprKind::Prefix { op, expr } => IrExprKind::Prefix {
                op: *op,
                expr: Box::new(lower_expr(expr)?),
            },
            CoreExprKind::Binary { op, left, right } => IrExprKind::Binary {
                op: *op,
                left: Box::new(lower_expr(left)?),
                right: Box::new(lower_expr(right)?),
            },
        },
    })
}

fn lower_call_target(
    node_id: NodeId,
    target: &CoreCallTarget,
) -> Result<IrCallTarget, IrLowerError> {
    match target {
        CoreCallTarget::Function(name) => Ok(IrCallTarget::Function(name.clone())),
        CoreCallTarget::StdioBuiltin(name) => Ok(IrCallTarget::StdioBuiltin(name.clone())),
        CoreCallTarget::Value(name) => Ok(IrCallTarget::Value(name.clone())),
        CoreCallTarget::Unresolved(symbol) => Err(IrLowerError::UnresolvedCallTarget {
            node_id,
            symbol: symbol.clone(),
        }),
    }
}
