use veln_ast::{BinaryOp, ContractKind, NodeId, PrefixOp, Visibility};
use veln_source::SourceSpan;

use crate::{CoreReadiness, CoreType};

#[derive(Clone, Debug, PartialEq)]
pub struct CheckedProgram {
    pub functions: Vec<CoreFunction>,
    pub effects: Vec<CoreEffectDecl>,
    pub readiness: CoreReadiness,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreEffectDecl {
    pub node_id: NodeId,
    pub name: String,
    pub visibility: Visibility,
    pub operations: Vec<CoreEffectOperationDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreEffectOperationDecl {
    pub node_id: NodeId,
    pub name: String,
    pub params: Vec<CoreType>,
    pub return_type: CoreType,
    pub span: SourceSpan,
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
    pub body: CoreCleanupRegion,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreCleanupRegion {
    pub statements: Vec<CoreStmt>,
}

impl CoreCleanupRegion {
    pub fn new(statements: Vec<CoreStmt>) -> Self {
        Self { statements }
    }
}

impl std::ops::Deref for CoreCleanupRegion {
    type Target = [CoreStmt];

    fn deref(&self) -> &Self::Target {
        &self.statements
    }
}

impl std::ops::DerefMut for CoreCleanupRegion {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.statements
    }
}

impl<'a> IntoIterator for &'a CoreCleanupRegion {
    type Item = &'a CoreStmt;
    type IntoIter = std::slice::Iter<'a, CoreStmt>;

    fn into_iter(self) -> Self::IntoIter {
        self.statements.iter()
    }
}

impl<'a> IntoIterator for &'a mut CoreCleanupRegion {
    type Item = &'a mut CoreStmt;
    type IntoIter = std::slice::IterMut<'a, CoreStmt>;

    fn into_iter(self) -> Self::IntoIter {
        self.statements.iter_mut()
    }
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
    Defer(CoreDeferredBlock),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreDeferredBlock {
    pub captures: Vec<CoreDeferredCapture>,
    pub body: Vec<CoreStmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoreDeferredCapture {
    pub name: String,
    pub ty: CoreType,
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
    Perform {
        effect: String,
        operation: String,
        args: Vec<CoreExpr>,
    },
    Handle {
        effect: String,
        providers: Vec<CoreHandlerProvider>,
        context_args: Vec<CoreExpr>,
        body: Box<CoreExpr>,
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
    CleanupRegion {
        region: CoreCleanupRegion,
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

#[derive(Clone, Debug, PartialEq)]
pub struct CoreHandlerProvider {
    pub operation: String,
    pub function: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoreCallTarget {
    Function(String),
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
