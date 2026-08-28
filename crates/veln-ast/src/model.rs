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
    pub aliases: Vec<PublicAlias>,
    pub effects: Vec<EffectDecl>,
    pub handlers: Vec<HandlerDecl>,
    pub types: Vec<TypeDecl>,
    pub schemas: Vec<SchemaDecl>,
    pub codecs: Vec<CodecDecl>,
    pub functions: Vec<Function>,
    pub invalid_names: Vec<InvalidName>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameClass {
    Type,
    Constructor,
    Module,
    Function,
    ValueBinding,
}

impl NameClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Module => "module",
            Self::Function => "function",
            Self::ValueBinding => "value_binding",
        }
    }

    pub fn required_initial(self) -> &'static str {
        match self {
            Self::Type | Self::Constructor => "ascii_uppercase",
            Self::Module | Self::Function | Self::ValueBinding => "ascii_lowercase",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameOccurrence {
    Declaration,
    Binding,
    PatternHead,
    AliasTarget,
    PathSegment,
}

impl NameOccurrence {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Declaration => "declaration",
            Self::Binding => "binding",
            Self::PatternHead => "pattern_head",
            Self::AliasTarget => "alias_target",
            Self::PathSegment => "path_segment",
        }
    }
}

#[derive(Clone, Debug)]
pub struct InvalidName {
    pub name: String,
    pub class: NameClass,
    pub occurrence: NameOccurrence,
    pub span: SourceSpan,
    pub enclosing_function_span: Option<SourceSpan>,
    pub segment_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct HandlerDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub effect: Vec<String>,
    pub effect_span: SourceSpan,
    pub effects: Option<Vec<String>>,
    pub effect_spans: Option<Vec<SourceSpan>>,
    pub operation_clauses: Vec<HandlerOperationClauseDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct HandlerOperationClauseDecl {
    pub node_id: NodeId,
    pub operation: Option<String>,
    pub operation_span: SourceSpan,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: SourceSpan,
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
    pub module_name: Option<String>,
    pub name: String,
    pub alias: String,
    pub name_spans: Vec<SourceSpan>,
    pub package: Option<String>,
    pub package_span: Option<SourceSpan>,
    pub span: SourceSpan,
    pub origin: UseOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseOrigin {
    Source,
    ImplicitStandardPrelude,
}

impl UseDecl {
    pub fn implicit_standard_prelude(module_name: String, span: SourceSpan) -> Self {
        Self {
            node_id: NodeId::new(u32::MAX),
            module_name: Some(module_name),
            name: "std::prelude".to_string(),
            alias: "prelude".to_string(),
            name_spans: Vec::new(),
            package: Some("std".to_string()),
            package_span: None,
            span,
            origin: UseOrigin::ImplicitStandardPrelude,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PublicAlias {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub kind: PublicAliasKind,
    pub name: Option<String>,
    pub target: Vec<String>,
    pub target_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicAliasKind {
    Function,
    Type,
    Schema,
}

#[derive(Clone, Debug)]
pub struct EffectDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub operations: Vec<EffectOperationDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct EffectOperationDecl {
    pub node_id: NodeId,
    pub name: Option<String>,
    pub name_span: SourceSpan,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct TypeDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<String>,
    pub variants: Vec<TypeVariantDecl>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct TypeVariantDecl {
    pub node_id: NodeId,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub fields: Vec<TypeVariantField>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct TypeVariantField {
    pub node_id: NodeId,
    pub name: String,
    pub ty: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub format: Option<SchemaFormatClause>,
    pub fields: Vec<SchemaField>,
    pub validations: Vec<SchemaValidationClause>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaFormatClause {
    pub node_id: NodeId,
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaField {
    pub node_id: NodeId,
    pub name: String,
    pub ty: String,
    pub where_clause: Option<SchemaFieldWhereClause>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaFieldWhereClause {
    pub node_id: NodeId,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaValidationClause {
    pub node_id: NodeId,
    pub predicate: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct CodecDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub schema: Option<String>,
    pub directions: Vec<CodecDirection>,
    pub implementations: Vec<CodecImplementationClause>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecDirection {
    Decode,
    Encode,
}

#[derive(Clone, Debug)]
pub struct CodecImplementationClause {
    pub node_id: NodeId,
    pub direction: CodecDirection,
    pub kind: CodecImplementationKind,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum CodecImplementationKind {
    Derive,
    With { function: Option<String> },
}

#[derive(Clone, Debug)]
pub struct Function {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub effect_binder: Option<EffectBinder>,
    pub params: Vec<Param>,
    pub return_binding: Option<ResultBinding>,
    pub return_type: Option<String>,
    pub return_type_span: Option<SourceSpan>,
    pub effects: Option<Vec<String>>,
    pub effect_spans: Option<Vec<SourceSpan>>,
    pub contracts: Vec<Contract>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct EffectBinder {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    pub ty_span: Option<SourceSpan>,
    pub is_variadic: bool,
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
    Invariant,
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
    BoolLiteral(bool),
    Unit,
    TypeApply {
        callee: Box<Expr>,
        type_args: Vec<String>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Perform {
        effect: Vec<String>,
        effect_span: SourceSpan,
        operation: String,
        operation_span: SourceSpan,
        args: Vec<Expr>,
    },
    Handle {
        body: Box<Expr>,
        handler: Vec<String>,
        handler_span: SourceSpan,
        args: Vec<Expr>,
    },
    SchemaDecode {
        schema: Vec<String>,
        input: Box<Expr>,
        base: Box<Expr>,
    },
    SchemaEncode {
        schema: Vec<String>,
        value: Box<Expr>,
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
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_if_branches: Vec<IfBranch>,
        else_branch: Box<Expr>,
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
pub struct IfBranch {
    pub node_id: NodeId,
    pub condition: Expr,
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
    BitwiseNot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    PipeGreater,
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    ShiftLeft,
    ShiftRight,
    ShiftRightLogical,
    Add,
    Subtract,
    Multiply,
    Divide,
}
