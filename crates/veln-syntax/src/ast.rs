use veln_source::SourceSpan;

#[derive(Clone, Debug)]
pub struct ModuleDecl {
    pub name: String,
    pub name_spans: Vec<SourceSpan>,
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
    pub name_spans: Vec<SourceSpan>,
    pub package: Option<UsePackage>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct UsePackage {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub enum SyntaxItem {
    Function(Box<FunctionDecl>),
    Effect(EffectDecl),
    Handler(HandlerDecl),
    Type(TypeDecl),
    Schema(SchemaDecl),
    PublicAlias(PublicAliasDecl),
}

#[derive(Clone, Debug)]
pub struct HandlerDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub params: Vec<Param>,
    pub effect: Vec<String>,
    pub effect_span: SourceSpan,
    pub effect_recovered: bool,
    pub effects: Option<Vec<String>>,
    pub effect_spans: Option<Vec<SourceSpan>>,
    pub effects_recovered: bool,
    pub operation_clauses: Vec<HandlerOperationClauseDecl>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Debug)]
pub struct HandlerOperationClauseDecl {
    pub operation: Option<String>,
    pub operation_span: SourceSpan,
    pub params: Vec<Param>,
    pub body: Expr,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct EffectDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub operations: Vec<EffectOperationDecl>,
    pub span: SourceSpan,
    pub end_present: bool,
    pub recovered: bool,
}

#[derive(Clone, Debug)]
pub struct EffectOperationDecl {
    pub name: Option<String>,
    pub name_span: SourceSpan,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub return_type_paths: Vec<TypePathSegments>,
    pub return_type_refinements: Vec<VariantRefinementType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct PublicAliasDecl {
    pub kind: PublicAliasKind,
    pub name: Option<String>,
    pub name_span: Option<SourceSpan>,
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
pub struct TypeDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub name_span: Option<SourceSpan>,
    pub params: Vec<String>,
    pub variants: Vec<TypeVariantDecl>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Debug)]
pub struct TypeVariantDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub name_span: Option<SourceSpan>,
    pub field_delimiter: Option<TypeVariantFieldDelimiter>,
    pub fields: Vec<TypeVariantField>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeVariantFieldDelimiter {
    Tuple,
    Record,
}

#[derive(Clone, Debug)]
pub struct TypeVariantField {
    pub name: String,
    pub ty: String,
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaDecl {
    pub visibility: Visibility,
    pub name: Option<String>,
    pub format: Option<SchemaFormatClause>,
    pub fields: Vec<SchemaField>,
    pub validations: Vec<SchemaValidationClause>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Debug)]
pub struct SchemaFormatClause {
    pub name: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaField {
    pub name: String,
    pub ty: String,
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
    pub where_clause: Option<SchemaFieldWhereClause>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaFieldWhereClause {
    pub predicate: String,
    pub perform_effect_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct SchemaValidationClause {
    pub predicate: String,
    pub perform_effect_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct FunctionDecl {
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub name_span: Option<SourceSpan>,
    pub effect_binder: Option<EffectBinder>,
    pub params: Vec<Param>,
    pub return_binding: Option<ResultBinding>,
    pub return_type: Option<String>,
    pub return_type_span: Option<SourceSpan>,
    pub return_type_paths: Vec<TypePathSegments>,
    pub return_type_refinements: Vec<VariantRefinementType>,
    pub effects: Option<Vec<String>>,
    pub effect_spans: Option<Vec<SourceSpan>>,
    pub effects_recovered: bool,
    pub callsite: Option<SourceSpan>,
    pub contracts: Vec<ContractClause>,
    pub body: Vec<BodyLine>,
    pub span: SourceSpan,
    pub end_present: bool,
}

#[derive(Clone, Debug)]
pub struct EffectBinder {
    pub name: String,
    pub span: SourceSpan,
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
    pub name_span: SourceSpan,
    pub ty: Option<String>,
    pub ty_span: Option<SourceSpan>,
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
    pub is_variadic: bool,
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
    pub perform_effect_spans: Vec<SourceSpan>,
    pub callsite_reference_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractKind {
    Require,
    Ensure,
    Invariant,
}

#[derive(Clone, Debug)]
pub enum BodyLine {
    Let {
        pattern: Pattern,
        annotation: Option<String>,
        annotation_paths: Box<[TypePathSegments]>,
        annotation_refinements: Box<[VariantRefinementType]>,
        expr: Expr,
        span: SourceSpan,
    },
    Expr {
        expr: Expr,
        span: SourceSpan,
    },
    Defer {
        body: Vec<BodyLine>,
        keyword_span: SourceSpan,
        block_span: SourceSpan,
        span: SourceSpan,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypePathSegments {
    pub segments: Vec<String>,
    pub segment_spans: Vec<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantRefinementType {
    pub alternatives: Vec<VariantRefinementAlternative>,
    pub pipe_spans: Vec<SourceSpan>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantRefinementAlternative {
    pub base: TypePathSegments,
    pub type_arguments: Vec<VariantRefinementTypeArgument>,
    pub variant: String,
    pub variant_span: SourceSpan,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantRefinementTypeArgument {
    pub ty_fragments: Vec<String>,
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
    pub span: SourceSpan,
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
    NamePath {
        segments: Vec<String>,
        segment_spans: Vec<SourceSpan>,
    },
    StringLiteral(String),
    IntLiteral(String),
    FloatLiteral(String),
    BoolLiteral(bool),
    Unit,
    TypeApply {
        callee: Box<Expr>,
        type_args: Vec<String>,
        type_arg_refinements: Vec<Vec<VariantRefinementType>>,
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
        recovered: bool,
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
        schema_spans: Vec<SourceSpan>,
        recovered: bool,
        input: Box<Expr>,
        base: Box<Expr>,
    },
    SchemaEncode {
        schema: Vec<String>,
        schema_spans: Vec<SourceSpan>,
        recovered: bool,
        value: Box<Expr>,
    },
    FieldAccess {
        base: Box<Expr>,
        field: String,
        field_span: SourceSpan,
    },
    Try {
        expr: Box<Expr>,
        question_span: SourceSpan,
    },
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
    Begin {
        body: Vec<BodyLine>,
        block_span: SourceSpan,
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

pub(crate) enum ExprChildren<'a> {
    None,
    One {
        child: &'a Expr,
        followed_by_parent_syntax: bool,
    },
    Pair {
        first: &'a Expr,
        first_followed_by_parent_syntax: bool,
        second: &'a Expr,
        second_followed_by_parent_syntax: bool,
    },
    Slice(&'a [Expr]),
    HeadAndSlice(&'a Expr, &'a [Expr]),
    Record(&'a [RecordField]),
    Dict(&'a [DictEntry]),
    Match(&'a Expr, &'a [MatchArm]),
    If {
        condition: &'a Expr,
        then_branch: &'a Expr,
        else_if_branches: &'a [IfBranch],
        else_branch: &'a Expr,
    },
    BeginBody(&'a [BodyLine]),
}

impl Expr {
    pub(crate) fn children(&self) -> ExprChildren<'_> {
        match &self.kind {
            ExprKind::TypeApply { callee: child, .. }
            | ExprKind::FieldAccess { base: child, .. }
            | ExprKind::Try { expr: child, .. } => ExprChildren::One {
                child,
                followed_by_parent_syntax: true,
            },
            ExprKind::SchemaEncode { value: child, .. } | ExprKind::Prefix { expr: child, .. } => {
                ExprChildren::One {
                    child,
                    followed_by_parent_syntax: false,
                }
            }
            ExprKind::SchemaDecode {
                input: first,
                base: second,
                ..
            }
            | ExprKind::Binary {
                left: first,
                right: second,
                ..
            } => ExprChildren::Pair {
                first,
                first_followed_by_parent_syntax: true,
                second,
                second_followed_by_parent_syntax: false,
            },
            ExprKind::Perform { args, .. } | ExprKind::List(args) => ExprChildren::Slice(args),
            ExprKind::Call { callee: head, args }
            | ExprKind::Handle {
                body: head, args, ..
            } => ExprChildren::HeadAndSlice(head, args),
            ExprKind::Record(fields) => ExprChildren::Record(fields),
            ExprKind::Dict(entries) => ExprChildren::Dict(entries),
            ExprKind::Match { scrutinee, arms } => ExprChildren::Match(scrutinee, arms),
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => ExprChildren::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            },
            ExprKind::Begin { body, .. } => ExprChildren::BeginBody(body),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => ExprChildren::None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SatisfyClause {
    pub candidate: Option<String>,
    pub candidate_span: Option<SourceSpan>,
    pub predicate: String,
    pub perform_effect_spans: Vec<SourceSpan>,
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
pub struct IfBranch {
    pub condition: Expr,
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
        name_spans: Vec<SourceSpan>,
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
