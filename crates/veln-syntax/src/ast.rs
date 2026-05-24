use veln_source::SourceSpan;

#[derive(Clone, Debug)]
pub struct ModuleDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdrLiteRecord {
    pub id: String,
    pub status: String,
    pub scope: String,
    pub context: String,
    pub decision: String,
    pub consequences: String,
    pub anchor: Option<AdrLiteAnchor>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdrLiteAnchor {
    Module { name: String },
    Function { name: String },
}

#[derive(Clone, Debug)]
pub struct UseDecl {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum SyntaxItem {
    Function(FunctionDecl),
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub return_binding: Option<ResultBinding>,
    pub return_type: Option<String>,
    pub effects: Option<Vec<String>>,
    pub contracts: Vec<ContractClause>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunctionKind {
    Function,
    Test,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub name: String,
    pub ty: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ResultBinding {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ContractClause {
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
pub enum BodyLine {
    Let {
        pattern: Pattern,
        annotation: Option<String>,
        expr: Expr,
        span: SourceSpan,
    },
    Expr {
        expr: Expr,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
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
    BoolLiteral(bool),
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
    pub name: String,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct DictEntry {
    pub key: Expr,
    pub value: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub expr: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct Pattern {
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
