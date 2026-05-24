use veln_source::SourceSpan;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub(crate) fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }

    pub fn display(self, prefix: &str) -> String {
        format!("{prefix}-{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceModule {
    pub module: Option<ModuleHeader>,
    pub uses: Vec<UseDecl>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug)]
pub struct ModuleHeader {
    pub node_id: NodeId,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct UseDecl {
    pub node_id: NodeId,
    pub name: String,
    pub alias: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub return_binding: Option<ResultBinding>,
    pub return_type: Option<String>,
    pub effects: Option<Vec<String>>,
    pub contracts: Vec<Contract>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionKind {
    Function,
    Test,
}

impl FunctionKind {
    pub fn node_prefix(self) -> &'static str {
        match self {
            Self::Function => "fn",
            Self::Test => "test",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub node_id: NodeId,
    pub name: String,
    pub ty: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ResultBinding {
    pub node_id: NodeId,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Contract {
    pub node_id: NodeId,
    pub kind: ContractKind,
    pub text: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Require,
    Ensure,
}

#[derive(Clone, Debug)]
pub struct BodyLine {
    pub node_id: NodeId,
    pub kind: BodyLineKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum BodyLineKind {
    Let {
        pattern: Pattern,
        annotation: Option<String>,
        expr: Expr,
    },
    Expr {
        expr: Expr,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub node_id: NodeId,
    pub kind: ExprKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Missing,
    Hole {
        name: Option<String>,
        satisfy: Option<SatisfyClause>,
    },
    NamePath(Vec<String>),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    Unit,
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        field_span: SourceSpan,
    },
    Try(Box<Expr>),
    Record(Vec<RecordField>),
    Dict(Vec<DictEntry>),
    List(Vec<Expr>),
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Clone, Debug)]
pub struct SatisfyClause {
    pub candidate: Option<String>,
    pub candidate_span: Option<SourceSpan>,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct RecordField {
    pub node_id: NodeId,
    pub name: String,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct DictEntry {
    pub node_id: NodeId,
    pub key: Expr,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub node_id: NodeId,
    pub pattern: Pattern,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Pattern {
    pub node_id: NodeId,
    pub kind: PatternKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum PatternKind {
    Wildcard,
    Binding(String),
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    BoolLiteral(bool),
    Unit,
    Record(Vec<PatternField>),
    Constructor {
        name: Vec<String>,
        args: Vec<Pattern>,
    },
}

#[derive(Clone, Debug)]
pub struct PatternField {
    pub node_id: NodeId,
    pub name: String,
    pub pattern: Pattern,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixOp {
    Not,
    Negate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    PipeGreater,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
}
