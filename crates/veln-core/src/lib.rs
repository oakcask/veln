//! Source-linked checked core representation.

use veln_ast::{BinaryOp, ContractKind, NodeId, PrefixOp, Visibility};
use veln_source::SourceSpan;

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedProgram {
    pub functions: Vec<CoreFunction>,
    pub readiness: CoreReadiness,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreReadiness {
    Complete,
    Blocked(Vec<CoreBlocker>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreBlocker {
    Hole { node_id: NodeId },
    MissingExpression { node_id: NodeId },
    UnsupportedExpression { node_id: NodeId, reason: String },
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
    Call {
        target: CoreCallTarget,
        args: Vec<CoreExpr>,
    },
    Try(Box<CoreExpr>),
    Record(Vec<CoreRecordField>),
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreType {
    Unknown,
    Named {
        name: String,
        args: Vec<CoreType>,
    },
    Record(Vec<(String, CoreType)>),
    Function {
        params: Vec<CoreType>,
        return_type: Box<CoreType>,
        effects: Vec<String>,
    },
}

impl CoreType {
    pub fn named(name: impl Into<String>, args: Vec<CoreType>) -> Self {
        Self::Named {
            name: name.into(),
            args,
        }
    }

    pub fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    pub fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    pub fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    pub fn string() -> Self {
        Self::named("String", Vec::new())
    }

    pub fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    pub fn result(value: CoreType, error: CoreType) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub fn option(value: CoreType) -> Self {
        Self::named("Option", vec![value])
    }

    pub fn list(value: CoreType) -> Self {
        Self::named("List", vec![value])
    }

    pub fn result_parts(&self) -> Option<(&CoreType, &CoreType)> {
        match self {
            Self::Named { name, args } if name == "Result" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub fn option_part(&self) -> Option<&CoreType> {
        match self {
            Self::Named { name, args } if name == "Option" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub fn list_part(&self) -> Option<&CoreType> {
        match self {
            Self::Named { name, args } if name == "List" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub fn record_field(&self, field_name: &str) -> Option<&CoreType> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }
}
