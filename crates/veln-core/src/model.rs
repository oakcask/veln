use veln_ast::{BinaryOp, ContractKind, NodeId, PrefixOp, Visibility};
use veln_source::SourceSpan;

use crate::{CoreReadiness, CoreType};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedProgram {
    pub functions: Vec<CoreFunction>,
    pub readiness: CoreReadiness,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreFunction {
    pub node_id: NodeId,
    pub name: String,
    pub visibility: Visibility,
    pub params: Vec<CoreParam>,
    pub return_type: CoreType,
    pub effects: Vec<String>,
    pub contracts: Vec<CoreContract>,
    pub body: Vec<CoreStmt>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreParam {
    pub node_id: NodeId,
    pub name: String,
    pub ty: CoreType,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreContract {
    pub node_id: NodeId,
    pub kind: ContractKind,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreStmt {
    pub node_id: NodeId,
    pub kind: CoreStmtKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoreStmtKind {
    Let {
        name: String,
        ty: CoreType,
        expr: CoreExpr,
    },
    Expr {
        expr: CoreExpr,
    },
    Return {
        expr: CoreExpr,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreExpr {
    pub node_id: NodeId,
    pub ty: CoreType,
    pub kind: CoreExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoreExprKind {
    Missing,
    Hole {
        label: Option<String>,
    },
    Local(String),
    BoolLiteral(bool),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    ResultOk(Box<CoreExpr>),
    ResultErr(Box<CoreExpr>),
    OptionSome(Box<CoreExpr>),
    OptionNone,
    Call {
        target: CoreCallTarget,
        args: Vec<CoreExpr>,
    },
    FieldAccess {
        base: Box<CoreExpr>,
        field: String,
    },
    Try(Box<CoreExpr>),
    Record(Vec<CoreRecordField>),
    Dict(Vec<CoreDictEntry>),
    List(Vec<CoreExpr>),
    Prefix {
        op: PrefixOp,
        expr: Box<CoreExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<CoreExpr>,
        right: Box<CoreExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreCallTarget {
    Function(String),
    StdioBuiltin(String),
    PreludeBuiltin(String),
    Value(String),
    Unresolved(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreRecordField {
    pub node_id: NodeId,
    pub name: String,
    pub expr: CoreExpr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreDictEntry {
    pub node_id: NodeId,
    pub key: CoreExpr,
    pub value: CoreExpr,
    pub span: SourceSpan,
}
