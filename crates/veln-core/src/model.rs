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
    pub return_binding: Option<String>,
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
    pub obligation_status: ContractObligationStatus,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractObligationStatus {
    RuntimeRequired,
    StaticallyProven,
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
    FunctionValue(String),
    ResultOk(Box<CoreExpr>),
    ResultErr(Box<CoreExpr>),
    OptionSome(Box<CoreExpr>),
    OptionNone,
    ListNil,
    ListCons {
        head: Box<CoreExpr>,
        tail: Box<CoreExpr>,
    },
    AdtVariant {
        name: Vec<String>,
        payloads: Vec<CoreExpr>,
    },
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
    Match {
        scrutinee: Box<CoreExpr>,
        arms: Vec<CoreMatchArm>,
    },
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
    CodecDecode { function: String, codec: String },
    SchemaDecode(String),
    SchemaDecodeStep(String),
    SchemaNeutralDecode(String),
    SchemaNeutralEncode(String),
    SchemaEncode(String),
    SchemaEncodeStep(String),
    SchemaValidate(String),
    StdioBuiltin(String),
    ConcurrencyBuiltin(String),
    StandardLibraryBuiltin(String),
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

#[derive(Clone, Debug, PartialEq)]
pub struct CoreMatchArm {
    pub node_id: NodeId,
    pub pattern: CorePattern,
    pub expr: CoreExpr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorePattern {
    pub node_id: NodeId,
    pub kind: CorePatternKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorePatternKind {
    Wildcard,
    Binding(String),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    BoolLiteral(bool),
    Unit,
    Record(Vec<CorePatternField>),
    Constructor {
        name: Vec<String>,
        args: Vec<CorePattern>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorePatternField {
    pub node_id: NodeId,
    pub name: String,
    pub pattern: CorePattern,
    pub span: SourceSpan,
}
