use veln_ast::{BinaryOp, NodeId, PrefixOp, Visibility};
use veln_core::CoreType;
use veln_source::SourceSpan;

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
    pub span: SourceSpan,
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
    OptionNone,
    Call {
        target: IrCallTarget,
        args: Vec<IrExpr>,
    },
    FieldAccess {
        base: Box<IrExpr>,
        field: String,
    },
    Try(Box<IrExpr>),
    Record(Vec<IrRecordField>),
    Dict(Vec<IrDictEntry>),
    List(Vec<IrExpr>),
    Match {
        scrutinee: Box<IrExpr>,
        arms: Vec<IrMatchArm>,
    },
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
    PreludeBuiltin(String),
    Value(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrRecordField {
    pub node_id: NodeId,
    pub name: String,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrDictEntry {
    pub node_id: NodeId,
    pub key: IrExpr,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrMatchArm {
    pub node_id: NodeId,
    pub pattern: IrPattern,
    pub value: IrExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrPattern {
    pub node_id: NodeId,
    pub kind: IrPatternKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IrPatternKind {
    Wildcard,
    Binding(String),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    BoolLiteral(bool),
    Unit,
    Record(Vec<IrPatternField>),
    Constructor {
        name: Vec<String>,
        args: Vec<IrPattern>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct IrPatternField {
    pub node_id: NodeId,
    pub name: String,
    pub pattern: IrPattern,
}
