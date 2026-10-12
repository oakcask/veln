use std::ops::Range;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedPathSegment {
    pub name: String,
    pub role: NameClass,
    pub occurrence: NameOccurrence,
    pub span: SourceSpan,
    pub segment_index: usize,
    pub evidence: QualifiedPathSegmentEvidence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QualifiedPathSegmentEvidence {
    Syntax,
    Resolved,
    UniqueRecovery,
}

#[derive(Clone, Debug)]
pub struct HandlerDecl {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub package_name: Option<String>,
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
    pub return_type_paths: Vec<TypePathSegments>,
    pub return_type_refinements: Vec<VariantRefinementType>,
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
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
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
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
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
pub struct Function {
    pub node_id: NodeId,
    pub module_name: Option<String>,
    pub package_name: Option<String>,
    pub kind: FunctionKind,
    pub visibility: Visibility,
    pub name: Option<String>,
    pub effect_binder: Option<EffectBinder>,
    pub params: Vec<Param>,
    pub return_binding: Option<ResultBinding>,
    pub return_type: Option<String>,
    pub return_type_span: Option<SourceSpan>,
    pub return_type_paths: Vec<TypePathSegments>,
    pub return_type_refinements: Vec<VariantRefinementType>,
    pub effects: Option<Vec<String>>,
    pub effect_spans: Option<Vec<SourceSpan>>,
    pub callsite: Option<SourceSpan>,
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
    pub name_span: SourceSpan,
    pub ty: Option<String>,
    pub ty_span: Option<SourceSpan>,
    pub ty_paths: Vec<TypePathSegments>,
    pub ty_refinements: Vec<VariantRefinementType>,
    pub is_variadic: bool,
    pub span: SourceSpan,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeAnnotationStructure {
    pub paths: Vec<TypePathSegments>,
    pub variant_refinements: Vec<VariantRefinementType>,
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
    pub remainder_spans: Vec<PredicateRemainder>,
    pub callsite_reference_spans: Vec<SourceSpan>,
    pub call_callee_spans: Vec<(String, SourceSpan)>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateRemainder {
    pub left_text: String,
    pub left_is_prior_remainder: bool,
    pub right_text: String,
    pub operator_span: SourceSpan,
    pub left_span: SourceSpan,
    pub right_span: SourceSpan,
    pub operation_span: SourceSpan,
}

pub fn predicate_remainder_chain_ranges(remainders: &[PredicateRemainder]) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, remainder) in remainders.iter().enumerate() {
        if !remainder.left_is_prior_remainder {
            if let Some(start) = start.replace(index) {
                ranges.push(start..index);
            }
        }
    }
    if let Some(start) = start {
        ranges.push(start..remainders.len());
    }
    ranges
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PredicateRemainderChain {
    pub remainder_range: Range<usize>,
    pub predicate_range: Range<usize>,
}

pub fn index_predicate_remainder_chains(
    predicate: &str,
    remainders: &[PredicateRemainder],
) -> Option<Vec<PredicateRemainderChain>> {
    let ranges = predicate_remainder_chain_ranges(remainders);
    let mut chains = Vec::with_capacity(ranges.len());
    let mut cursor = 0;
    for remainder_range in ranges {
        let expected = compact_remainder_chain_text(&remainders[remainder_range.clone()]);
        let predicate_range = find_compact_expression(predicate, &expected, cursor)
            .or_else(|| find_compact_expression(predicate, &expected, 0))?;
        cursor = cursor.max(predicate_range.end);
        chains.push(PredicateRemainderChain {
            remainder_range,
            predicate_range,
        });
    }
    Some(chains)
}

fn compact_remainder_chain_text(remainders: &[PredicateRemainder]) -> String {
    let mut text = remainders[0]
        .left_text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    for remainder in remainders {
        text.push('%');
        text.extend(
            remainder
                .right_text
                .chars()
                .filter(|character| !character.is_whitespace()),
        );
    }
    text
}

fn find_compact_expression(
    predicate: &str,
    expected: &str,
    search_start: usize,
) -> Option<Range<usize>> {
    let mut in_string = false;
    let mut escaped = false;
    for (relative_start, character) in predicate.get(search_start..)?.char_indices() {
        let start = search_start + relative_start;
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if character == '"' {
            in_string = true;
            continue;
        }
        if character.is_whitespace() {
            continue;
        }
        if let Some(end) = compact_prefix_end(&predicate[start..], expected) {
            return Some(start..start + end);
        }
    }
    None
}

fn compact_prefix_end(candidate: &str, expected: &str) -> Option<usize> {
    let mut expected = expected.chars();
    let mut wanted = expected.next()?;
    for (offset, character) in candidate.char_indices() {
        if character.is_whitespace() {
            continue;
        }
        if character != wanted {
            return None;
        }
        match expected.next() {
            Some(next) => wanted = next,
            None => return Some(offset + character.len_utf8()),
        }
    }
    None
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
        annotation_structure: Box<TypeAnnotationStructure>,
        expr: Expr,
    },
    Expr {
        expr: Expr,
    },
    Defer {
        body: Vec<BodyLine>,
        keyword_span: SourceSpan,
        block_span: SourceSpan,
    },
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub node_id: NodeId,
    pub kind: ExprKind,
    pub span: SourceSpan,
}

impl Expr {
    pub fn callee_name_path(&self) -> Option<&[String]> {
        match &self.kind {
            ExprKind::NamePath { segments, .. } => Some(segments),
            ExprKind::TypeApply { callee, .. } => callee.callee_name_path(),
            _ => None,
        }
    }

    pub fn callee_name_path_and_type_args(&self) -> Option<(&[String], Option<&[String]>)> {
        match &self.kind {
            ExprKind::NamePath { segments, .. } => Some((segments, None)),
            ExprKind::TypeApply {
                callee, type_args, ..
            } => {
                let ExprKind::NamePath { segments, .. } = &callee.kind else {
                    return None;
                };
                Some((segments, Some(type_args)))
            }
            _ => None,
        }
    }
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

#[derive(Clone, Debug)]
pub struct SatisfyClause {
    pub candidate: Option<String>,
    pub candidate_span: Option<SourceSpan>,
    pub predicate: String,
    pub remainder_spans: Vec<PredicateRemainder>,
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

impl Pattern {
    pub fn for_each_binding(&self, visitor: &mut impl FnMut(&str)) {
        match &self.kind {
            PatternKind::Binding(name) => visitor(name),
            PatternKind::Record(fields) => {
                for field in fields {
                    field.pattern.for_each_binding(visitor);
                }
            }
            PatternKind::Constructor { args, .. } => {
                for argument in args {
                    argument.for_each_binding(visitor);
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {}
        }
    }
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
    Remainder,
}
