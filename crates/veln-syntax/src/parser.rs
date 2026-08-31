use std::collections::BTreeMap;

use veln_literals::{IntegerLiteralError, parse_integer_literal};
use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::tree::build_lossless_root;
use crate::{
    AdrLiteAnchor, AdrLiteRecord, BinaryOp, BodyLine, ContractClause, ContractKind, DictEntry,
    EffectBinder, EffectDecl, EffectOperationDecl, Expr, ExprKind, FunctionDecl, FunctionKind,
    HandlerDecl, HandlerOperationClauseDecl, IfBranch, MatchArm, ModuleDecl, Param, Pattern,
    PatternField, PatternKind, PrefixOp, PublicAliasDecl, PublicAliasKind, RecordField,
    SatisfyClause, SchemaDecl, SchemaField, SchemaFieldWhereClause, SchemaFormatClause,
    SchemaValidationClause, SyntaxItem, SyntaxTree, Token, TokenKind, TypeDecl, TypePathSegments,
    TypeVariantDecl, TypeVariantField, TypeVariantFieldDelimiter, UseDecl, UsePackage, Visibility,
    lex,
};

mod body_and_types;
mod contract_predicates;
mod declarations;
mod diagnostics_and_tokens;
mod expression_aggregates;
mod expression_control;
mod expression_core;
mod expression_primaries;
mod functions_and_imports;
mod schemas;

fn is_contextual_identifier(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Ident | TokenKind::Handle | TokenKind::Handler | TokenKind::Handles
    )
}

fn binary_operator(kind: TokenKind, allow_pipeline: bool) -> Option<(BinaryOp, u8, u8)> {
    match kind {
        TokenKind::PipeGreater if allow_pipeline => Some((BinaryOp::PipeGreater, 1, 2)),
        TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
        TokenKind::And => Some((BinaryOp::And, 5, 6)),
        TokenKind::Pipe => Some((BinaryOp::BitwiseOr, 7, 8)),
        TokenKind::Caret => Some((BinaryOp::BitwiseXor, 9, 10)),
        TokenKind::Ampersand => Some((BinaryOp::BitwiseAnd, 11, 12)),
        TokenKind::EqualEqual => Some((BinaryOp::Equal, 13, 14)),
        TokenKind::BangEqual => Some((BinaryOp::NotEqual, 13, 14)),
        TokenKind::Less => Some((BinaryOp::Less, 15, 16)),
        TokenKind::LessEqual => Some((BinaryOp::LessEqual, 15, 16)),
        TokenKind::Greater => Some((BinaryOp::Greater, 15, 16)),
        TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 15, 16)),
        TokenKind::ShiftLeft => Some((BinaryOp::ShiftLeft, 17, 18)),
        TokenKind::ShiftRight => Some((BinaryOp::ShiftRight, 17, 18)),
        TokenKind::ShiftRightLogical => Some((BinaryOp::ShiftRightLogical, 17, 18)),
        TokenKind::Plus => Some((BinaryOp::Add, 19, 20)),
        TokenKind::Minus => Some((BinaryOp::Subtract, 19, 20)),
        TokenKind::Star => Some((BinaryOp::Multiply, 21, 22)),
        TokenKind::Slash => Some((BinaryOp::Divide, 21, 22)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct ParseOutput {
    pub tree: SyntaxTree,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Clone, Debug)]
pub struct ParseDiagnostic {
    pub id: &'static str,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub parser_context: &'static str,
    pub unexpected: UnexpectedToken,
    pub expected: Vec<&'static str>,
    pub recovery: Recovery,
    pub repair_candidates: Vec<ParseRepairCandidate>,
}

#[derive(Clone, Debug)]
pub struct ParseRepairCandidate {
    pub candidate_id: String,
    pub name: String,
    pub application_policy: String,
    pub application_status: String,
    pub edit_summary: String,
    pub edits: Vec<ParseRepairEdit>,
}

#[derive(Clone, Debug)]
pub struct ParseRepairEdit {
    pub span: SourceSpan,
    pub replacement: String,
}

#[derive(Clone, Debug)]
pub struct UnexpectedToken {
    pub kind: String,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Recovery {
    pub strategy: RecoveryStrategy,
    pub anchor: Option<String>,
    pub dropped_token_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryStrategy {
    None,
    SkipToken,
    InsertToken,
    CloseBlock,
    SynchronizeToAnchor,
}

struct DiagnosticRequest {
    id: &'static str,
    message: String,
    parser_context: &'static str,
    expected: Vec<&'static str>,
    strategy: RecoveryStrategy,
    anchor: Option<&'static str>,
    repair_candidates: Vec<ParseRepairCandidate>,
}

fn diagnostic_at_token(
    source: &SourceFile,
    token: &Token,
    request: DiagnosticRequest,
) -> ParseDiagnostic {
    ParseDiagnostic {
        id: request.id,
        message: request.message,
        span: Some(source.span(token.range)),
        parser_context: request.parser_context,
        unexpected: UnexpectedToken {
            kind: token.kind.label().to_string(),
            text: token.text.clone(),
        },
        expected: request.expected,
        recovery: Recovery {
            strategy: request.strategy,
            anchor: request.anchor.map(str::to_string),
            dropped_token_count: 0,
        },
        repair_candidates: request.repair_candidates,
    }
}

fn invalid_expression_token_diagnostic(
    source: &SourceFile,
    token: &Token,
    parser_context: &'static str,
    recovery_anchor: &'static str,
) -> ParseDiagnostic {
    ParseDiagnostic {
        id: "parse.invalid_token",
        message: "invalid token in expression".to_string(),
        span: Some(source.span(token.range)),
        parser_context,
        unexpected: UnexpectedToken {
            kind: token.kind.label().to_string(),
            text: token.text.clone(),
        },
        expected: vec!["expression"],
        recovery: Recovery {
            strategy: RecoveryStrategy::SkipToken,
            anchor: Some(recovery_anchor.to_string()),
            dropped_token_count: 1,
        },
        repair_candidates: Vec::new(),
    }
}

impl RecoveryStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SkipToken => "skip_token",
            Self::InsertToken => "insert_token",
            Self::CloseBlock => "close_block",
            Self::SynchronizeToAnchor => "synchronize_to_anchor",
        }
    }
}
pub fn parse(source: &SourceFile) -> ParseOutput {
    let lexed = lex(source);
    Parser::new(source, lexed.tokens).parse()
}

pub fn bare_expression_bool_literal(segments: &[String]) -> Option<bool> {
    match segments {
        [segment] if segment == "true" => Some(true),
        [segment] if segment == "false" => Some(false),
        _ => None,
    }
}

struct Parser<'a> {
    source: &'a SourceFile,
    tokens: Vec<Token>,
    lossless_tokens: Vec<Token>,
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

struct FunctionHeader {
    visibility: Visibility,
    name: Option<String>,
    name_span: Option<SourceSpan>,
    effect_binder: Option<EffectBinder>,
    params: Vec<Param>,
}

struct FunctionReturn {
    binding: Option<crate::ResultBinding>,
    ty: Option<String>,
    ty_span: Option<SourceSpan>,
    ty_paths: Vec<TypePathSegments>,
    effects: Option<Vec<String>>,
    effect_spans: Option<Vec<SourceSpan>>,
}

struct HandlerHeader {
    visibility: Visibility,
    start: TextRange,
    name: Option<String>,
    params: Vec<Param>,
    effect: HandlerEffect,
}

struct HandlerEffect {
    path: Vec<String>,
    span: SourceSpan,
    effects: Option<Vec<String>>,
    effect_spans: Option<Vec<SourceSpan>>,
}

struct HandlerBody {
    operation_clauses: Vec<HandlerOperationClauseDecl>,
    end: TextRange,
    end_present: bool,
}

#[derive(Default)]
struct SchemaBody {
    format: Option<SchemaFormatClause>,
    fields: Vec<SchemaField>,
    validations: Vec<SchemaValidationClause>,
    end_present: bool,
}

#[derive(Default)]
struct TypeArgumentNesting {
    parentheses: usize,
    braces: usize,
    brackets: usize,
    angles: usize,
}

struct AngleClosers {
    total: usize,
    nested: usize,
}

enum TypeArgumentTokenAction {
    Finish { nested_angle_closers: usize },
    Separate,
    Append,
}

#[derive(Default)]
struct TypeArgumentListState {
    args: Vec<String>,
    current: String,
    nesting: TypeArgumentNesting,
}

impl TypeArgumentNesting {
    fn is_outer_level(&self) -> bool {
        self.parentheses == 0 && self.braces == 0 && self.brackets == 0 && self.angles == 0
    }

    fn consume_delimiter(&mut self, kind: TokenKind) {
        match kind {
            TokenKind::LParen => self.parentheses += 1,
            TokenKind::RParen => self.parentheses = self.parentheses.saturating_sub(1),
            TokenKind::LBrace => self.braces += 1,
            TokenKind::RBrace => self.braces = self.braces.saturating_sub(1),
            TokenKind::LBracket => self.brackets += 1,
            TokenKind::RBracket => self.brackets = self.brackets.saturating_sub(1),
            TokenKind::Less => self.angles += 1,
            _ => {}
        }
    }

    fn consume_angle_closers(&mut self, kind: TokenKind) -> Option<AngleClosers> {
        let total = closing_angle_count(kind);
        if total == 0 {
            return None;
        }
        let nested = total.min(self.angles);
        self.angles -= nested;
        Some(AngleClosers { total, nested })
    }

    fn classify(&mut self, kind: TokenKind, close: TokenKind) -> TypeArgumentTokenAction {
        if kind == close && self.is_outer_level() {
            return TypeArgumentTokenAction::Finish {
                nested_angle_closers: 0,
            };
        }
        if kind == TokenKind::Comma && self.is_outer_level() {
            return TypeArgumentTokenAction::Separate;
        }
        if let Some(closers) = self.consume_angle_closers(kind) {
            return if closers.total > closers.nested {
                TypeArgumentTokenAction::Finish {
                    nested_angle_closers: closers.nested,
                }
            } else {
                TypeArgumentTokenAction::Append
            };
        }
        self.consume_delimiter(kind);
        TypeArgumentTokenAction::Append
    }
}

impl TypeArgumentListState {
    fn consume(&mut self, token: &Token, close: TokenKind) -> bool {
        match self.nesting.classify(token.kind, close) {
            TypeArgumentTokenAction::Finish {
                nested_angle_closers,
            } => {
                self.current.push_str(&">".repeat(nested_angle_closers));
                self.flush_current(false);
                true
            }
            TypeArgumentTokenAction::Separate => {
                self.flush_current(true);
                false
            }
            TypeArgumentTokenAction::Append => {
                self.current.push_str(&token.text);
                false
            }
        }
    }

    fn flush_current(&mut self, include_empty: bool) {
        if include_empty || !self.current.is_empty() {
            let current = std::mem::take(&mut self.current);
            self.args.push(normalize_type_text(vec![current]));
        }
    }

    fn finish(mut self) -> Vec<String> {
        self.flush_current(false);
        self.args
    }
}

fn integer_literal_diagnostics(source: &SourceFile, tokens: &[Token]) -> Vec<ParseDiagnostic> {
    tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Int | TokenKind::MalformedInt))
        .filter_map(|token| {
            let error = parse_integer_literal(&token.text).err()?;
            let (message, range, expected) = integer_literal_error_details(token, error);
            let (strategy, dropped_token_count) =
                if matches!(error, IntegerLiteralError::OutOfRange { .. }) {
                    (RecoveryStrategy::None, 0)
                } else {
                    (RecoveryStrategy::SkipToken, 1)
                };
            Some(ParseDiagnostic {
                id: "parse.integer_literal",
                message,
                span: Some(source.span(range)),
                parser_context: "integer_literal",
                unexpected: UnexpectedToken {
                    kind: token.kind.label().to_string(),
                    text: token.text.clone(),
                },
                expected,
                recovery: Recovery {
                    strategy,
                    anchor: None,
                    dropped_token_count,
                },
                repair_candidates: Vec::new(),
            })
        })
        .collect()
}

fn integer_literal_error_details(
    token: &Token,
    error: IntegerLiteralError,
) -> (String, TextRange, Vec<&'static str>) {
    match error {
        IntegerLiteralError::MissingDigits { radix } => (
            format!(
                "{} integer literal requires at least one digit",
                radix.name()
            ),
            token.range,
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::UnsupportedUppercasePrefix { radix } => (
            format!(
                "uppercase {} integer literal prefix is unsupported",
                radix.name()
            ),
            literal_error_character_range(token, 1),
            vec![match radix {
                veln_literals::IntegerRadix::Binary => "lowercase `0b` prefix",
                veln_literals::IntegerRadix::Hexadecimal => "lowercase `0x` prefix",
                veln_literals::IntegerRadix::Decimal => "decimal integer",
            }],
        ),
        IntegerLiteralError::InvalidDigit {
            radix,
            byte_offset,
            character,
        } => (
            format!(
                "`{character}` is not a valid {} integer digit",
                radix.name()
            ),
            literal_error_character_range(token, byte_offset),
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::Separator { radix, byte_offset } => (
            format!(
                "digit separators are not supported in {} integer literals",
                radix.name()
            ),
            literal_error_character_range(token, byte_offset),
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::PrefixedFloat { radix, .. } => (
            format!("{} floating-point literals are unsupported", radix.name()),
            token.range,
            vec!["integer literal"],
        ),
        IntegerLiteralError::OutOfRange { radix } => (
            format!(
                "{} integer literal exceeds the maximum Int value {}",
                radix.name(),
                i64::MAX
            ),
            token.range,
            vec!["Int value at or below 9223372036854775807"],
        ),
    }
}

fn literal_error_character_range(token: &Token, byte_offset: usize) -> TextRange {
    let start = token.range.start + byte_offset;
    let length = token.text[byte_offset..]
        .chars()
        .next()
        .map_or(0, char::len_utf8);
    TextRange::new(start, start + length)
}

fn collect_adr_lite_records(
    source: &SourceFile,
    tokens: &[Token],
    module: Option<&ModuleDecl>,
    items: &[SyntaxItem],
) -> Vec<AdrLiteRecord> {
    let anchors = adr_lite_anchors(module, items);
    let mut records = Vec::new();
    let mut cursor = 0;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind != TokenKind::Comment || !is_adr_lite_marker(&doc_comment_text(token)) {
            cursor += 1;
            continue;
        }

        let start = token.range;
        let mut end = token.range;
        let mut fields = BTreeMap::<String, String>::new();
        cursor += 1;

        while cursor < tokens.len() {
            match tokens[cursor].kind {
                TokenKind::Whitespace | TokenKind::Newline => {
                    cursor += 1;
                }
                TokenKind::Comment => {
                    let content = doc_comment_text(&tokens[cursor]);
                    if content.is_empty() {
                        end = end.cover(tokens[cursor].range);
                        cursor += 1;
                        continue;
                    }
                    if content.starts_with('@') {
                        break;
                    }
                    if let Some((key, value)) = content.split_once(':') {
                        fields.insert(key.trim().to_string(), value.trim().to_string());
                    }
                    end = end.cover(tokens[cursor].range);
                    cursor += 1;
                }
                _ => break,
            }
        }

        let Some(id) = fields.remove("id") else {
            continue;
        };
        let Some(status) = fields.remove("status") else {
            continue;
        };
        let Some(scope) = fields.remove("scope") else {
            continue;
        };
        let Some(context) = fields.remove("context") else {
            continue;
        };
        let Some(decision) = fields.remove("decision") else {
            continue;
        };
        let Some(consequences) = fields.remove("consequences") else {
            continue;
        };
        let span = source.span(start.cover(end));
        let anchor = anchors
            .iter()
            .find_map(|(offset, anchor)| (*offset >= span.end.offset).then(|| anchor.clone()));
        records.push(AdrLiteRecord {
            id,
            status,
            scope,
            context,
            decision,
            consequences,
            anchor,
            span,
        });
    }

    records
}

fn adr_lite_anchors(
    module: Option<&ModuleDecl>,
    items: &[SyntaxItem],
) -> Vec<(usize, AdrLiteAnchor)> {
    let mut anchors = Vec::new();
    if let Some(module) = module {
        anchors.push((
            module.span.start.offset,
            AdrLiteAnchor::Module {
                name: module.name.clone(),
            },
        ));
    }
    for item in items {
        let SyntaxItem::Function(function) = item else {
            continue;
        };
        if function.visibility == Visibility::Public
            && let Some(name) = &function.name
        {
            anchors.push((
                function.span.start.offset,
                AdrLiteAnchor::Function { name: name.clone() },
            ));
        }
    }
    anchors.sort_by_key(|(offset, _)| *offset);
    anchors
}

fn doc_comment_text(token: &Token) -> String {
    token
        .text
        .strip_prefix("##")
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn is_adr_lite_marker(content: &str) -> bool {
    matches!(content, "@adr" | "@adr-lite")
}

struct ExprParser<'a> {
    source: &'a SourceFile,
    context: &'static str,
    tokens: &'a [Token],
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

struct ContractPredicateParser<'a> {
    source: &'a SourceFile,
    context: &'static str,
    diagnostic_id: &'static str,
    tokens: &'a [Token],
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

trait TokenCursor {
    fn token_slice(&self) -> &[Token];
    fn cursor_index(&self) -> usize;
    fn cursor_index_mut(&mut self) -> &mut usize;
    fn source_file(&self) -> &SourceFile;
    fn diagnostics_mut(&mut self) -> &mut Vec<ParseDiagnostic>;

    fn holds_at_eof(&self) -> bool {
        false
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        self.at(kind).then(|| self.bump())
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.token_slice()
            .get(self.cursor_index())
            .is_some_and(|token| token.kind == kind)
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.token_slice()
            .get(self.cursor_index() + offset)
            .map(|token| token.kind)
    }

    fn at_ident_text(&self, text: &str) -> bool {
        self.token_slice()
            .get(self.cursor_index())
            .is_some_and(|token| token.kind == TokenKind::Ident && token.text == text)
    }

    fn current(&self) -> &Token {
        &self.token_slice()[self.cursor_index()]
    }

    fn previous(&self) -> Option<&Token> {
        self.cursor_index()
            .checked_sub(1)
            .and_then(|index| self.token_slice().get(index))
    }

    fn is_at_end(&self) -> bool {
        self.cursor_index() >= self.token_slice().len()
    }

    fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if !self.holds_at_eof() || token.kind != TokenKind::Eof {
            *self.cursor_index_mut() += 1;
        }
        token
    }

    fn error_at_token(&mut self, token: &Token, request: DiagnosticRequest) {
        let diagnostic = diagnostic_at_token(self.source_file(), token, request);
        self.diagnostics_mut().push(diagnostic);
    }
}

macro_rules! impl_token_cursor {
    ($parser:ident, $holds_at_eof:literal) => {
        impl TokenCursor for $parser<'_> {
            fn token_slice(&self) -> &[Token] {
                self.tokens.as_ref()
            }

            fn cursor_index(&self) -> usize {
                self.cursor
            }

            fn cursor_index_mut(&mut self) -> &mut usize {
                &mut self.cursor
            }

            fn source_file(&self) -> &SourceFile {
                self.source
            }

            fn diagnostics_mut(&mut self) -> &mut Vec<ParseDiagnostic> {
                &mut self.diagnostics
            }

            fn holds_at_eof(&self) -> bool {
                $holds_at_eof
            }
        }
    };
}

impl_token_cursor!(Parser, true);
impl_token_cursor!(ExprParser, false);
impl_token_cursor!(ContractPredicateParser, false);

fn lhs_range(expr: &Expr) -> TextRange {
    TextRange::new(expr.span.start.offset, expr.span.end.offset)
}

fn pattern_range(pattern: &Pattern) -> TextRange {
    TextRange::new(pattern.span.start.offset, pattern.span.end.offset)
}

fn return_type_can_take_effects(ty: &str) -> bool {
    ty.trim_start().starts_with("fn")
}

fn normalize_collected_text(parts: Vec<String>) -> String {
    parts
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace("->(", "-> (")
        .replace(" )", ")")
        .replace(" . ", ".")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
        .replace(" ; ", "; ")
        .replace(" ;", ";")
        .replace(";  ", "; ")
}

fn is_byte_view_multiple_predicate_text(text: &str) -> bool {
    let Some(divisor) = text
        .trim()
        .strip_prefix("payload_count multiple of ")
        .map(str::trim)
    else {
        return false;
    };
    if divisor.is_empty() || divisor.contains(char::is_whitespace) {
        return false;
    }
    if parse_integer_literal(divisor).is_ok() {
        return true;
    }
    is_schema_where_identifier(divisor)
}

fn is_schema_where_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn closing_angle_count(kind: TokenKind) -> usize {
    match kind {
        TokenKind::Greater => 1,
        TokenKind::ShiftRight => 2,
        TokenKind::ShiftRightLogical => 3,
        _ => 0,
    }
}

fn normalize_type_text(parts: Vec<String>) -> String {
    normalize_collected_text(parts)
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
}

fn schema_repeated_field_type_missing_semicolon(text: &str) -> bool {
    let text = text.trim();
    let Some(inner) = text
        .strip_prefix('[')
        .and_then(|text| text.strip_suffix(']'))
    else {
        return false;
    };
    !contains_top_level_semicolon(inner)
}

fn contains_top_level_semicolon(text: &str) -> bool {
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => return true,
            _ => {}
        }
    }
    false
}

fn unquote_string_token(text: &str) -> String {
    text.strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text)
        .to_string()
}
