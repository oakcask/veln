use std::collections::{BTreeMap, BTreeSet};

use veln_source::{SourceFile, SourceSpan, TextRange};

use crate::tree::build_lossless_root;
use crate::{
    AdrLiteAnchor, AdrLiteRecord, BinaryOp, BodyLine, CodecDecl, CodecDirection,
    CodecImplementationClause, CodecImplementationKind, ContractClause, ContractKind, DictEntry,
    Expr, ExprKind, FunctionDecl, FunctionKind, MatchArm, ModuleDecl, Param, Pattern, PatternField,
    PatternKind, PrefixOp, PublicAliasDecl, PublicAliasKind, RecordField, SatisfyClause,
    SchemaDecl, SchemaField, SchemaFieldWhereClause, SchemaFormatClause, SchemaMappingAssignment,
    SchemaMappingClause, SchemaMappingInverseConverter, SchemaMappingSelector,
    SchemaValidationClause, SyntaxItem, SyntaxTree, Token, TokenKind, TypeDecl, TypeVariantDecl,
    TypeVariantField, TypeVariantFieldDelimiter, UseDecl, UsePackage, Visibility, lex,
};

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
    params: Vec<Param>,
}

struct FunctionReturn {
    binding: Option<crate::ResultBinding>,
    ty: Option<String>,
    effects: Option<Vec<String>>,
}

impl<'a> Parser<'a> {
    fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        let parse_tokens = tokens
            .iter()
            .filter(|token| !token.kind.is_trivia())
            .cloned()
            .collect();
        Self {
            source,
            tokens: parse_tokens,
            lossless_tokens: tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> ParseOutput {
        self.eat_newlines();
        let module = if self.at(TokenKind::Mod) {
            Some(
                self.parse_named_header(TokenKind::Mod, "module_declaration")
                    .0,
            )
        } else {
            None
        };

        let uses = self.parse_use_declarations();

        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::Eof) {
                break;
            }
            if self.at_public_alias_header() {
                items.push(SyntaxItem::PublicAlias(self.parse_public_alias()));
            } else if self.at(TokenKind::Pub) && self.peek_at(TokenKind::Type) {
                items.push(SyntaxItem::Type(self.parse_type_decl()));
            } else if self.at(TokenKind::Pub) && self.peek_at(TokenKind::Schema) {
                items.push(SyntaxItem::Schema(self.parse_schema_decl()));
            } else if self.at(TokenKind::Pub) && self.peek_at(TokenKind::Codec) {
                items.push(SyntaxItem::Codec(self.parse_codec_decl()));
            } else if self.at(TokenKind::Pub) || self.at(TokenKind::Fn) {
                items.push(SyntaxItem::Function(
                    self.parse_function_like(FunctionKind::Function),
                ));
            } else if self.at(TokenKind::Type) {
                items.push(SyntaxItem::Type(self.parse_type_decl()));
            } else if self.at(TokenKind::Schema) {
                items.push(SyntaxItem::Schema(self.parse_schema_decl()));
            } else if self.at(TokenKind::Codec) {
                items.push(SyntaxItem::Codec(self.parse_codec_decl()));
            } else if self.at(TokenKind::Test) {
                items.push(SyntaxItem::Function(
                    self.parse_function_like(FunctionKind::Test),
                ));
            } else {
                self.error_current(
                    "parse.expected_item",
                    "expected a function, test, type, schema, or codec declaration",
                    "module",
                    vec!["pub", "fn", "test", "type", "schema", "codec"],
                    RecoveryStrategy::SynchronizeToAnchor,
                    Some("fn"),
                );
                self.synchronize_to_item();
            }
        }

        let adr_lite_records =
            collect_adr_lite_records(self.source, &self.lossless_tokens, module.as_ref(), &items);

        let root = build_lossless_root(
            self.lossless_tokens,
            self.source.len(),
            module.as_ref(),
            &uses,
            &items,
        );

        ParseOutput {
            tree: SyntaxTree {
                root,
                module,
                adr_lite_records,
                uses,
                items,
            },
            diagnostics: self.diagnostics,
        }
    }

    fn at_public_alias_header(&self) -> bool {
        self.at(TokenKind::Pub)
            && matches!(
                (self.peek_kind(1), self.peek_kind(2), self.peek_kind(3)),
                (
                    Some(TokenKind::Fn | TokenKind::Type | TokenKind::Schema),
                    Some(TokenKind::Ident),
                    Some(TokenKind::Equal)
                )
            )
    }

    fn parse_public_alias(&mut self) -> PublicAliasDecl {
        let start = self
            .expect(TokenKind::Pub, "public_alias", vec!["pub"])
            .range;
        let kind = if self.eat(TokenKind::Fn).is_some() {
            PublicAliasKind::Function
        } else if self.eat(TokenKind::Type).is_some() {
            PublicAliasKind::Type
        } else {
            self.expect(
                TokenKind::Schema,
                "public_alias",
                vec!["fn", "type", "schema"],
            );
            PublicAliasKind::Schema
        };
        let name = self.expect_ident("public_alias", "public member name");
        self.expect(TokenKind::Equal, "public_alias", vec!["="]);
        let target = self.parse_member_alias_target();
        let end = self.expect_newline("public_alias").range;
        PublicAliasDecl {
            kind,
            name,
            target,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_member_alias_target(&mut self) -> Vec<String> {
        let mut segments = Vec::new();
        if let Some(segment) = self.expect_ident("public_alias", "member path") {
            segments.push(segment);
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if let Some(segment) = self.expect_ident("public_alias", "member path segment") {
                segments.push(segment);
            }
        }
        segments
    }

    fn parse_type_decl(&mut self) -> TypeDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Type, "type_declaration", vec!["type"])
            .range;
        let name = self.expect_ident("type_declaration", "type name");
        let params = if self.eat(TokenKind::Less).is_some() {
            let params = self.parse_type_params_until(TokenKind::Greater);
            self.expect(TokenKind::Greater, "type_declaration", vec![">"]);
            params
        } else {
            Vec::new()
        };
        let header_cursor = self.cursor;
        let header_end = self.expect_newline("type_declaration").range;
        if self.cursor == header_cursor {
            self.skip_to_next_line();
        }

        let mut variants = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            variants.push(self.parse_type_variant());
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close type declaration",
                "type_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(header_end, |token| token.range);
        TypeDecl {
            visibility,
            name,
            params,
            variants,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_type_params_until(&mut self, close: TokenKind) -> Vec<String> {
        let mut params = Vec::new();
        while !self.at(close) && !self.at(TokenKind::Eof) {
            if let Some(param) = self.expect_ident("type_declaration", "type parameter") {
                params.push(param);
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn parse_schema_decl(&mut self) -> SchemaDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Schema, "schema_declaration", vec!["schema"])
            .range;
        let name = self.expect_ident("schema_declaration", "schema name");
        let header_cursor = self.cursor;
        let header_end = self.expect_newline("schema_declaration").range;
        if self.cursor == header_cursor {
            self.skip_to_next_line();
        }

        let mut format = None;
        let mut fields = Vec::new();
        let mut validations = Vec::new();
        let mut mappings = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            if self.at(TokenKind::Format) {
                let clause = self.parse_schema_format_clause(format.is_some());
                if format.is_none() {
                    format = Some(clause);
                }
            } else if self.at_ident_text("validate") && !self.peek_at(TokenKind::Colon) {
                validations.push(self.parse_schema_validation_clause(format.is_some()));
            } else if self.at_ident_text("map") && !self.peek_at(TokenKind::Colon) {
                mappings.push(self.parse_schema_mapping_clause(format.is_some()));
            } else {
                fields.push(self.parse_schema_field(format.is_some()));
            }
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close schema declaration",
                "schema_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(header_end, |token| token.range);
        SchemaDecl {
            visibility,
            name,
            format,
            fields,
            validations,
            mappings,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_schema_format_clause(&mut self, duplicate: bool) -> SchemaFormatClause {
        let start = self
            .expect(TokenKind::Format, "schema_format", vec!["format"])
            .range;
        if duplicate {
            self.error_current(
                "parse.schema_multiple_format",
                "schema declaration has multiple format clauses",
                "schema_format",
                vec!["field", "end"],
                RecoveryStrategy::SkipToken,
                Some("end"),
            );
        }
        let name_token = self.expect(TokenKind::Ident, "schema_format", vec!["binary"]);
        if name_token.text != "binary" {
            self.error_at_token(
                &name_token,
                DiagnosticRequest {
                    id: "parse.schema_format_name",
                    message: "schema format must be `binary`".to_string(),
                    parser_context: "schema_format",
                    expected: vec!["binary"],
                    strategy: RecoveryStrategy::SkipToken,
                    anchor: Some("newline"),
                    repair_candidates: Vec::new(),
                },
            );
        }
        let end = self.expect_newline("schema_format").range;
        SchemaFormatClause {
            name: name_token.text,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_field(&mut self, has_format: bool) -> SchemaField {
        let start = self.current().range;
        if !has_format {
            self.error_current(
                "parse.schema_field_before_format",
                "schema field appears before a format clause",
                "schema_field",
                vec!["format"],
                RecoveryStrategy::InsertToken,
                Some("format"),
            );
        }
        let name = if self.at(TokenKind::Hole) {
            let token = self.bump();
            self.error_at_token(
                &token,
                DiagnosticRequest {
                    id: "parse.schema_field_name",
                    message: "schema field name cannot start with `_`".to_string(),
                    parser_context: "schema_field",
                    expected: vec!["field name"],
                    strategy: RecoveryStrategy::SkipToken,
                    anchor: Some(":"),
                    repair_candidates: Vec::new(),
                },
            );
            token.text
        } else {
            self.expect_ident("schema_field", "schema field name")
                .unwrap_or_else(|| "<missing>".to_string())
        };
        self.expect(TokenKind::Colon, "schema_field", vec![":"]);
        let ty = self.collect_type_until(
            "schema_field",
            &[TokenKind::Where, TokenKind::Newline, TokenKind::Eof],
        );
        let where_clause = self
            .eat(TokenKind::Where)
            .map(|where_token| self.parse_schema_field_where_clause(where_token));
        let end = self.expect_newline("schema_field").range;
        SchemaField {
            name,
            ty,
            where_clause,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_field_where_clause(&mut self, where_token: Token) -> SchemaFieldWhereClause {
        let start = where_token.range;
        let mut end = start;
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        if predicate_tokens.is_empty() {
            self.error_current(
                "parse.schema_field_where",
                "expected schema field where predicate",
                "schema_field",
                vec!["predicate"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
        } else {
            self.diagnostics.extend(
                ContractPredicateParser::new(
                    self.source,
                    "schema_field_where",
                    "parse.schema_field_where",
                    &predicate_tokens,
                )
                .parse(),
            );
        }
        SchemaFieldWhereClause {
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_validation_clause(&mut self, has_format: bool) -> SchemaValidationClause {
        let start = self
            .expect_ident_text("validate", "schema_validation", "validate")
            .range;
        if !has_format {
            self.error_current(
                "parse.schema_validation_before_format",
                "schema validation appears before a format clause",
                "schema_validation",
                vec!["format"],
                RecoveryStrategy::InsertToken,
                Some("format"),
            );
        }
        let mut end = start;
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        if predicate_tokens.is_empty() {
            self.error_current(
                "parse.schema_validation",
                "expected schema validation predicate",
                "schema_validation",
                vec!["predicate"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
        } else {
            self.diagnostics.extend(
                ContractPredicateParser::new(
                    self.source,
                    "schema_validation",
                    "parse.schema_validation",
                    &predicate_tokens,
                )
                .parse(),
            );
        }
        let end = self.expect_newline("schema_validation").range.cover(end);
        SchemaValidationClause {
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_mapping_clause(&mut self, has_format: bool) -> SchemaMappingClause {
        let start = self.expect_ident_text("map", "schema_mapping", "map").range;
        if !has_format {
            self.error_current(
                "parse.schema_mapping_before_format",
                "schema mapping appears before a format clause",
                "schema_mapping",
                vec!["format"],
                RecoveryStrategy::InsertToken,
                Some("format"),
            );
        }
        if self.eat_ident_text("to", "schema_mapping", "to").is_none() {
            self.error_current(
                "parse.schema_mapping",
                "schema mapping must start with `map to Target`",
                "schema_mapping",
                vec!["to"],
                RecoveryStrategy::InsertToken,
                Some("target"),
            );
        }
        let target = self.expect_name_path("schema_mapping", "mapping target");
        let selector = self.parse_schema_mapping_selector();
        let mut end = if selector.is_some() {
            self.previous().map_or(start, |token| token.range)
        } else {
            self.expect_newline("schema_mapping").range
        };

        let mut assignments = Vec::new();
        let mut assigned_targets = BTreeSet::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if matches!(self.current().kind, TokenKind::End | TokenKind::Format)
                || (self.at_ident_text("map") && !self.peek_at(TokenKind::Colon))
                || (self.at_ident_text("validate") && !self.peek_at(TokenKind::Colon))
            {
                break;
            }
            let assignment = self.parse_schema_mapping_assignment(&mut assigned_targets);
            end = self.previous().map_or(start, |token| token.range);
            assignments.push(assignment);
        }
        if assignments.is_empty() {
            self.error_current(
                "parse.schema_mapping_assignment",
                "schema mapping requires at least one assignment",
                "schema_mapping",
                vec!["assignment"],
                RecoveryStrategy::InsertToken,
                Some("assignment"),
            );
        }

        SchemaMappingClause {
            target,
            selector,
            assignments,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_mapping_selector(&mut self) -> Option<SchemaMappingSelector> {
        if !self.at_ident_text("when") {
            return None;
        }
        let start = self.bump().range;
        if self.at(TokenKind::Newline) || self.at(TokenKind::Eof) {
            self.error_current(
                "parse.schema_mapping_selector",
                "expected schema mapping selector expression",
                "schema_mapping",
                vec!["selector expression"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
        }
        let (expr, _) = self.parse_expr_until_newline("schema_mapping_selector");
        let text = schema_mapping_expr_source_text(self.source, &expr);
        let end = TextRange::new(expr.span.start.offset, expr.span.end.offset);
        Some(SchemaMappingSelector {
            text,
            expr,
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_schema_mapping_assignment(
        &mut self,
        assigned_targets: &mut BTreeSet<String>,
    ) -> SchemaMappingAssignment {
        let start = self.current().range;
        let target = if self.at(TokenKind::Ident) && self.peek_at(TokenKind::Equal) {
            self.bump().text
        } else {
            if self.at(TokenKind::Ident) {
                self.error_current(
                    "parse.schema_mapping_implicit_assignment",
                    "schema mapping assignments must name a target with `target = field`",
                    "schema_mapping",
                    vec!["="],
                    RecoveryStrategy::InsertToken,
                    Some("newline"),
                );
            } else {
                self.error_current(
                    "parse.schema_mapping_assignment_target",
                    "expected schema mapping assignment target",
                    "schema_mapping",
                    vec!["assignment target"],
                    RecoveryStrategy::InsertToken,
                    Some("="),
                );
            }
            "<missing>".to_string()
        };
        if !assigned_targets.insert(target.clone()) && target != "<missing>" {
            self.error_current(
                "parse.schema_mapping_duplicate_assignment",
                format!("schema mapping assigns target `{target}` more than once"),
                "schema_mapping",
                vec!["unique assignment target"],
                RecoveryStrategy::SkipToken,
                Some("newline"),
            );
        }
        self.expect(TokenKind::Equal, "schema_mapping", vec!["="]);
        let (expr, expr_end) = if self.at(TokenKind::Newline) || self.at(TokenKind::Eof) {
            self.error_current(
                "parse.schema_mapping_expression",
                "schema mapping assignment value is missing",
                "schema_mapping",
                vec!["schema mapping expression"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
            let token = self.current().clone();
            if self.at(TokenKind::Newline) {
                self.bump();
            }
            (
                Expr {
                    kind: ExprKind::Missing,
                    span: self.source.span(token.range),
                },
                token.range,
            )
        } else {
            self.parse_schema_mapping_assignment_expr()
        };
        let source = schema_mapping_expr_source_text(self.source, &expr);
        let mut end = expr_end;
        let inverse_converter = if self.at_ident_text("inverse") {
            let inverse_start = self.bump().range;
            let name = self.expect_name_path("schema_mapping", "inverse converter");
            let inverse_end = self.previous().map_or(inverse_start, |token| token.range);
            Some(SchemaMappingInverseConverter {
                name: name.unwrap_or_default(),
                span: self.source.span(inverse_start.cover(inverse_end)),
            })
        } else {
            None
        };
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }
        SchemaMappingAssignment {
            target,
            source,
            expr,
            inverse_converter,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_schema_mapping_assignment_expr(&mut self) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        let mut match_depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0
                && match_depth == 0
                && (self.at(TokenKind::Newline)
                    || (self.at_ident_text("inverse") && !tokens.is_empty()))
            {
                break;
            }
            let token = self.bump();
            end = token.range;
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                TokenKind::Match => match_depth += 1,
                TokenKind::End if match_depth > 0 => {
                    match_depth = match_depth.saturating_sub(1);
                }
                _ => {}
            }
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.invalid_token",
                    message: "invalid token in expression".to_string(),
                    span: Some(self.source.span(token.range)),
                    parser_context: "schema_mapping",
                    unexpected: UnexpectedToken {
                        kind: token.kind.label().to_string(),
                        text: token.text.clone(),
                    },
                    expected: vec!["expression"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline".to_string()),
                        dropped_token_count: 1,
                    },
                    repair_candidates: Vec::new(),
                });
            } else {
                tokens.push(token);
            }
        }
        let (expr, diagnostics) = ExprParser::new(self.source, "schema_mapping", &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }

    fn parse_codec_decl(&mut self) -> CodecDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Codec, "codec_declaration", vec!["codec"])
            .range;
        let name = self.expect_ident("codec_declaration", "codec name");
        self.expect(TokenKind::For, "codec_declaration", vec!["for"]);
        let schema = self.expect_name_path("codec_declaration", "schema name");
        let directions = self.parse_codec_direction_list();
        let header_cursor = self.cursor;
        let header_end = self.expect_newline("codec_declaration").range;
        if self.cursor == header_cursor {
            self.skip_to_next_line();
        }

        let mut implementations = Vec::new();
        let mut implemented_directions = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            if let Some(clause) =
                self.parse_codec_implementation_clause(&directions, &mut implemented_directions)
            {
                implementations.push(clause);
            }
        }

        for direction in &directions {
            if !implemented_directions.contains(direction) {
                self.error_current(
                    "parse.codec_missing_implementation",
                    format!(
                        "codec declaration lists `{}` but has no implementation clause",
                        direction.as_str()
                    ),
                    "codec_declaration",
                    vec!["derive", "decode", "encode"],
                    RecoveryStrategy::InsertToken,
                    Some("end"),
                );
            }
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close codec declaration",
                "codec_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(header_end, |token| token.range);
        CodecDecl {
            visibility,
            name,
            schema,
            directions,
            implementations,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_codec_direction_list(&mut self) -> Vec<CodecDirection> {
        let mut directions = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.current().clone();
            if let Some(direction) = self.codec_direction_from_current() {
                self.bump();
                if directions.contains(&direction) {
                    self.error_at_token(
                        &token,
                        DiagnosticRequest {
                            id: "parse.codec_duplicate_direction",
                            message: format!(
                                "codec direction `{}` is listed more than once",
                                direction.as_str()
                            ),
                            parser_context: "codec_declaration",
                            expected: vec!["decode", "encode", "newline"],
                            strategy: RecoveryStrategy::SkipToken,
                            anchor: Some("newline"),
                            repair_candidates: Vec::new(),
                        },
                    );
                } else {
                    directions.push(direction);
                }
            } else {
                self.bump();
                self.error_at_token(
                    &token,
                    DiagnosticRequest {
                        id: "parse.codec_unknown_direction",
                        message: "codec direction must be `decode` or `encode`".to_string(),
                        parser_context: "codec_declaration",
                        expected: vec!["decode", "encode", "newline"],
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline"),
                        repair_candidates: Vec::new(),
                    },
                );
            }
        }
        if directions.is_empty() {
            self.error_current(
                "parse.codec_empty_directions",
                "codec declaration lists no directions",
                "codec_declaration",
                vec!["decode", "encode"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
        }
        directions
    }

    fn parse_codec_implementation_clause(
        &mut self,
        declared_directions: &[CodecDirection],
        implemented_directions: &mut Vec<CodecDirection>,
    ) -> Option<CodecImplementationClause> {
        let start = self.current().range;
        let (direction, direction_token, kind) = if self.eat(TokenKind::Derive).is_some() {
            let Some((direction, direction_token)) =
                self.parse_codec_clause_direction("derive direction")
            else {
                self.skip_to_next_line();
                return None;
            };
            (direction, direction_token, CodecImplementationKind::Derive)
        } else if let Some(direction) = self.codec_direction_from_current() {
            let direction_token = self.bump();
            self.expect(TokenKind::With, "codec_implementation", vec!["with"]);
            let function = self.expect_ident("codec_implementation", "implementation function");
            (
                direction,
                direction_token,
                CodecImplementationKind::With { function },
            )
        } else {
            self.error_current(
                "parse.codec_implementation_clause",
                "expected a codec implementation clause",
                "codec_implementation",
                vec!["derive", "decode", "encode", "end"],
                RecoveryStrategy::SynchronizeToAnchor,
                Some("newline"),
            );
            self.skip_to_next_line();
            return None;
        };

        let end = self.expect_newline("codec_implementation").range;
        if !declared_directions.contains(&direction) {
            self.error_at_token(
                &direction_token,
                DiagnosticRequest {
                    id: "parse.codec_unlisted_implementation",
                    message: format!(
                        "codec implementation clause uses `{}` but the declaration head does not list it",
                        direction.as_str()
                    ),
                    parser_context: "codec_implementation",
                    expected: vec![direction.as_str()],
                    strategy: RecoveryStrategy::SkipToken,
                    anchor: Some("newline"),
                    repair_candidates: Vec::new(),
                },
            );
        }
        if implemented_directions.contains(&direction) {
            self.error_at_token(
                &direction_token,
                DiagnosticRequest {
                    id: "parse.codec_duplicate_implementation",
                    message: format!(
                        "codec implementation for `{}` is listed more than once",
                        direction.as_str()
                    ),
                    parser_context: "codec_implementation",
                    expected: vec!["end"],
                    strategy: RecoveryStrategy::SkipToken,
                    anchor: Some("newline"),
                    repair_candidates: Vec::new(),
                },
            );
        } else {
            implemented_directions.push(direction);
        }

        Some(CodecImplementationClause {
            direction,
            kind,
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_codec_clause_direction(
        &mut self,
        expected: &'static str,
    ) -> Option<(CodecDirection, Token)> {
        if let Some(direction) = self.codec_direction_from_current() {
            let token = self.bump();
            return Some((direction, token));
        }
        self.error_current(
            "parse.codec_unknown_direction",
            format!("expected {expected} `decode` or `encode`"),
            "codec_implementation",
            vec!["decode", "encode"],
            RecoveryStrategy::InsertToken,
            Some("newline"),
        );
        None
    }

    fn codec_direction_from_current(&self) -> Option<CodecDirection> {
        match self.current().kind {
            TokenKind::Decode => Some(CodecDirection::Decode),
            TokenKind::Encode => Some(CodecDirection::Encode),
            _ => None,
        }
    }

    fn parse_type_variant(&mut self) -> TypeVariantDecl {
        let start = self.current().range;
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let name = self.expect_ident("type_variant", "variant name");
        let (field_delimiter, fields) = if self.eat(TokenKind::LParen).is_some() {
            let fields = self.parse_type_variant_fields();
            self.expect(TokenKind::RParen, "type_variant", vec![")"]);
            (Some(TypeVariantFieldDelimiter::Tuple), fields)
        } else if self.eat(TokenKind::LBrace).is_some() {
            let fields = self.parse_type_variant_fields_until(TokenKind::RBrace);
            self.expect(TokenKind::RBrace, "type_variant", vec!["}"]);
            (Some(TypeVariantFieldDelimiter::Record), fields)
        } else {
            (None, Vec::new())
        };
        let end = self.expect_newline("type_variant").range;
        TypeVariantDecl {
            visibility,
            name,
            field_delimiter,
            fields,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_type_variant_fields(&mut self) -> Vec<TypeVariantField> {
        self.parse_type_variant_fields_until(TokenKind::RParen)
    }

    fn parse_type_variant_fields_until(&mut self, close: TokenKind) -> Vec<TypeVariantField> {
        let mut fields = Vec::new();
        let mut positional_index = 0usize;
        while !self.at(close) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let (name, ty) = if self.at(TokenKind::Ident) && self.peek_at(TokenKind::Colon) {
                let name = self
                    .expect_ident("type_variant", "variant field name")
                    .unwrap_or_default();
                self.expect(TokenKind::Colon, "type_variant", vec![":"]);
                let ty = self.collect_type_until("type_variant", &[TokenKind::Comma, close]);
                (name, ty)
            } else {
                let name = if positional_index == 0 {
                    "value".to_string()
                } else {
                    format!("_{positional_index}")
                };
                let ty = self.collect_type_until("type_variant", &[TokenKind::Comma, close]);
                positional_index += 1;
                (name, ty)
            };
            let end = self.previous().map_or(start, |token| token.range);
            fields.push(TypeVariantField {
                name,
                ty,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        fields
    }

    fn parse_named_header(
        &mut self,
        keyword: TokenKind,
        context: &'static str,
    ) -> (ModuleDecl, UseDecl) {
        let start = self.expect(keyword, context, vec!["keyword"]).range;
        let name = self.parse_module_name(context);
        let end = self.expect_newline(context).range;
        let span = self.source.span(start.cover(end));
        (
            ModuleDecl {
                name: name.clone(),
                span: span.clone(),
            },
            UseDecl {
                name,
                package: None,
                span,
            },
        )
    }

    fn parse_use_declarations(&mut self) -> Vec<UseDecl> {
        let mut uses = Vec::new();
        loop {
            self.eat_newlines();
            if !self.at(TokenKind::Use) {
                return uses;
            }
            uses.push(self.parse_use_declaration());
        }
    }

    fn parse_use_declaration(&mut self) -> UseDecl {
        let start = self
            .expect(TokenKind::Use, "use_declaration", vec!["use"])
            .range;
        let name = self.parse_module_name("use_declaration");
        let package = if self.eat(TokenKind::From).is_some() {
            let token = self.expect(TokenKind::String, "use_declaration", vec!["package"]);
            Some(UsePackage {
                name: unquote_string_token(&token.text),
                span: self.source.span(token.range),
            })
        } else {
            None
        };
        let end = self.expect_newline("use_declaration").range;
        UseDecl {
            name,
            package,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_function_like(&mut self, kind: FunctionKind) -> FunctionDecl {
        let start = self.current().range;
        let header = self.parse_function_header(kind);
        let return_decl = self.parse_function_return_and_effects(kind);
        self.expect_newline(Self::function_context(kind));

        let contracts = self.parse_contracts();
        let (body, end_present) = self.parse_function_body();

        if !end_present {
            self.report_missing_function_end(kind);
        }

        let end = self.previous().map_or(start, |token| token.range);
        FunctionDecl {
            kind,
            visibility: header.visibility,
            name: header.name,
            params: header.params,
            return_binding: return_decl.binding,
            return_type: return_decl.ty,
            effects: return_decl.effects,
            contracts,
            body,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    fn parse_function_header(&mut self, kind: FunctionKind) -> FunctionHeader {
        let visibility = match kind {
            FunctionKind::Function => {
                if self.eat(TokenKind::Pub).is_some() {
                    Visibility::Public
                } else {
                    Visibility::Private
                }
            }
            FunctionKind::Test => Visibility::Private,
        };
        self.expect(
            match kind {
                FunctionKind::Function => TokenKind::Fn,
                FunctionKind::Test => TokenKind::Test,
            },
            Self::function_context(kind),
            vec![match kind {
                FunctionKind::Function => "fn",
                FunctionKind::Test => "test",
            }],
        );
        let name = self.expect_ident(Self::function_context(kind), "declaration name");
        self.expect(TokenKind::LParen, Self::parameter_context(kind), vec!["("]);
        let params = self.parse_params();
        self.expect(TokenKind::RParen, Self::parameter_context(kind), vec![")"]);
        FunctionHeader {
            visibility,
            name,
            params,
        }
    }

    fn parse_function_return_and_effects(&mut self, kind: FunctionKind) -> FunctionReturn {
        let return_context = Self::return_context(kind);
        let (binding, ty) = if self.eat(TokenKind::Arrow).is_some() {
            let return_binding = if self.at(TokenKind::Ident) && self.peek_at(TokenKind::Colon) {
                let name = self.bump();
                let colon = self.expect(TokenKind::Colon, return_context, vec![":"]);
                Some(crate::ResultBinding {
                    name: name.text,
                    span: self.source.span(name.range.cover(colon.range)),
                })
            } else {
                None
            };
            let return_type = self.collect_return_type_until(
                return_context,
                &[TokenKind::Effects, TokenKind::Newline, TokenKind::Eof],
            );
            (return_binding, Some(return_type))
        } else {
            (None, None)
        };
        let effects = if self.eat(TokenKind::Effects).is_some() {
            Some(self.parse_effect_list())
        } else {
            None
        };
        FunctionReturn {
            binding,
            ty,
            effects,
        }
    }

    fn parse_function_body(&mut self) -> (Vec<BodyLine>, bool) {
        let mut body = Vec::new();
        let mut end_present = false;
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                end_present = true;
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            body.push(self.parse_body_line());
        }
        (body, end_present)
    }

    fn report_missing_function_end(&mut self, kind: FunctionKind) {
        self.error_current(
            "parse.expected_end",
            match kind {
                FunctionKind::Function => {
                    "expected `end` to close function declaration".to_string()
                }
                FunctionKind::Test => "expected `end` to close test declaration".to_string(),
            },
            "function_body",
            vec!["end"],
            RecoveryStrategy::CloseBlock,
            Some("end"),
        );
    }

    fn function_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_declaration",
            FunctionKind::Test => "test_declaration",
        }
    }

    fn parameter_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_parameters",
            FunctionKind::Test => "test_parameters",
        }
    }

    fn return_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_return",
            FunctionKind::Test => "test_return",
        }
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let name = self.expect_ident("function_parameters", "parameter name");
            let mut is_variadic = false;
            let ty = self.eat(TokenKind::Colon).map(|_| {
                if self.eat_variadic_marker() {
                    is_variadic = true;
                }
                self.collect_type_until(
                    "function_parameters",
                    &[TokenKind::Comma, TokenKind::RParen, TokenKind::Eof],
                )
            });
            let end = self.previous().map_or(start, |token| token.range);
            params.push(Param {
                name: name.unwrap_or_default(),
                ty,
                is_variadic,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    fn eat_variadic_marker(&mut self) -> bool {
        if self.at(TokenKind::Dot)
            && self.peek_at(TokenKind::Dot)
            && self.peek_kind(2) == Some(TokenKind::Dot)
        {
            self.bump();
            self.bump();
            self.bump();
            true
        } else {
            false
        }
    }

    fn parse_effect_list(&mut self) -> Vec<String> {
        self.expect(TokenKind::LBracket, "effect_declaration", vec!["["]);
        let mut effects = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.at(TokenKind::Eof) {
            if let Some(effect) = self.expect_ident("effect_declaration", "effect name") {
                effects.push(effect);
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "effect_declaration", vec!["]"]);
        effects
    }

    fn parse_contract(&mut self) -> ContractClause {
        let start_token = self.bump();
        let kind = match start_token.kind {
            TokenKind::Require => ContractKind::Require,
            TokenKind::Ensure => ContractKind::Ensure,
            _ => ContractKind::Invariant,
        };
        let (text, predicate_tokens, end) = self.collect_until_newline();
        self.diagnostics.extend(
            ContractPredicateParser::new(
                self.source,
                "contract_predicate",
                "parse.contract_predicate",
                &predicate_tokens,
            )
            .parse(),
        );
        ContractClause {
            kind,
            text,
            span: self.source.span(start_token.range.cover(end)),
        }
    }

    fn parse_contracts(&mut self) -> Vec<ContractClause> {
        let mut contracts = Vec::new();
        loop {
            self.eat_newlines();
            if !self.at_contract_clause() {
                return contracts;
            }
            contracts.push(self.parse_contract());
        }
    }

    fn at_contract_clause(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Require | TokenKind::Ensure | TokenKind::Invariant
        )
    }

    fn parse_body_line(&mut self) -> BodyLine {
        let start = self.current().range;
        if self.at(TokenKind::Let) {
            self.bump();
            let pattern = self.parse_let_pattern();
            let annotation = if self.eat(TokenKind::Colon).is_some() {
                Some(self.collect_type_until(
                    "let_statement",
                    &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                ))
            } else {
                None
            };
            self.expect(TokenKind::Equal, "let_statement", vec!["="]);
            let (expr, end) = self.parse_expr_for_body_line("let_statement");
            BodyLine::Let {
                pattern,
                annotation,
                expr,
                span: self.source.span(start.cover(end)),
            }
        } else {
            let (expr, end) = self.parse_expr_for_body_line("expression_line");
            BodyLine::Expr {
                expr,
                span: self.source.span(start.cover(end)),
            }
        }
    }

    fn parse_module_name(&mut self, context: &'static str) -> String {
        let mut name = self
            .expect_ident(context, "module name")
            .unwrap_or_else(|| "<missing>".to_string());
        while self.at(TokenKind::Dot) || self.at(TokenKind::DoubleColon) {
            let delimiter = self.bump();
            if let Some(segment) = self.expect_ident(context, "module name segment") {
                name.push_str(&delimiter.text);
                name.push_str(&segment);
            }
        }
        name
    }

    fn collect_type_until(&mut self, _context: &'static str, stop: &[TokenKind]) -> String {
        let mut parts = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && stop.iter().any(|kind| self.at(*kind)) {
                break;
            }
            let token = self.current().clone();
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace | TokenKind::Less => {
                    depth += 1;
                }
                TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Greater => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            parts.push(self.bump().text);
        }
        normalize_type_text(parts)
    }

    fn collect_return_type_until(&mut self, context: &'static str, stop: &[TokenKind]) -> String {
        let mut ty = self.collect_type_until(context, stop);
        if return_type_can_take_effects(&ty)
            && self.at(TokenKind::Effects)
            && (self.after_effect_clause_is(TokenKind::Effects)
                || self.after_effect_clause_is(TokenKind::Newline)
                || self.after_effect_clause_is(TokenKind::Eof))
        {
            let effects = self.collect_effect_clause_text();
            if !effects.is_empty() {
                ty.push(' ');
                ty.push_str(&effects);
            }
        }
        ty
    }

    fn after_effect_clause_is(&self, expected: TokenKind) -> bool {
        if !self.at(TokenKind::Effects) {
            return false;
        }
        let mut cursor = self.cursor + 1;
        if !self
            .tokens
            .get(cursor)
            .is_some_and(|token| token.kind == TokenKind::LBracket)
        {
            return false;
        }
        cursor += 1;
        let mut depth = 1usize;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return self
                            .tokens
                            .get(cursor + 1)
                            .is_some_and(|next| next.kind == expected);
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    fn collect_effect_clause_text(&mut self) -> String {
        let mut parts = Vec::new();
        if !self.at(TokenKind::Effects) {
            return String::new();
        }
        parts.push(self.bump().text);
        if !self.at(TokenKind::LBracket) {
            return normalize_collected_text(parts);
        }
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) {
            match self.current().kind {
                TokenKind::LBracket => depth += 1,
                TokenKind::RBracket => {
                    depth = depth.saturating_sub(1);
                    parts.push(self.bump().text);
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                _ => {}
            }
            parts.push(self.bump().text);
        }
        normalize_collected_text(parts)
    }

    fn collect_until_newline(&mut self) -> (String, Vec<Token>, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut parts = Vec::new();
        let mut tokens = Vec::new();
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            tokens.push(token);
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }
        (
            parts
                .join(" ")
                .replace(" :: ", "::")
                .replace(" (", "(")
                .replace("( ", "(")
                .replace(" )", ")")
                .replace(" . ", ".")
                .replace("[ ", "[")
                .replace(" ]", "]")
                .replace(" ,", ","),
            tokens,
            start.cover(end),
        )
    }

    fn parse_expr_for_body_line(&mut self, context: &'static str) -> (Expr, TextRange) {
        if self.at(TokenKind::Match) {
            self.parse_match_expr_for_body_line(context)
        } else {
            self.parse_expr_until_newline(context)
        }
    }

    fn parse_let_pattern(&mut self) -> Pattern {
        let start = self.current().range;
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        while !self.at(TokenKind::Eof) && !self.at(TokenKind::Newline) {
            if depth == 0 && (self.at(TokenKind::Colon) || self.at(TokenKind::Equal)) {
                break;
            }
            let token = self.bump();
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            tokens.push(token);
        }

        if tokens.is_empty() {
            self.error_current(
                "parse.expected_pattern",
                "expected let pattern",
                "let_statement",
                vec!["pattern"],
                RecoveryStrategy::InsertToken,
                Some("="),
            );
            return Pattern {
                kind: PatternKind::Wildcard,
                span: self.source.span(start),
            };
        }

        let (pattern, diagnostics) =
            ExprParser::new(self.source, "let_statement", &tokens).parse_pattern_only();
        self.diagnostics.extend(diagnostics);
        pattern
    }

    fn parse_match_expr_for_body_line(&mut self, context: &'static str) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        let mut match_depth = 0usize;
        while !self.at(TokenKind::Eof) {
            let token = self.bump();
            end = token.range;
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.invalid_token",
                    message: "invalid token in expression".to_string(),
                    span: Some(self.source.span(token.range)),
                    parser_context: context,
                    unexpected: UnexpectedToken {
                        kind: token.kind.label().to_string(),
                        text: token.text.clone(),
                    },
                    expected: vec!["expression"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("end".to_string()),
                        dropped_token_count: 1,
                    },
                    repair_candidates: Vec::new(),
                });
                continue;
            }
            if token.kind == TokenKind::Match {
                match_depth += 1;
            }
            if token.kind == TokenKind::End {
                match_depth = match_depth.saturating_sub(1);
                tokens.push(token);
                if match_depth == 0 {
                    if self.at(TokenKind::Newline) {
                        end = self.bump().range;
                    }
                    break;
                }
                continue;
            }
            tokens.push(token);
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }

    fn parse_expr_until_newline(&mut self, context: &'static str) -> (Expr, TextRange) {
        let start = self.current().range;
        let mut end = start;
        let mut tokens = Vec::new();
        let mut depth = 0usize;
        let mut match_depth = 0usize;
        while !self.at(TokenKind::Eof) {
            if depth == 0 && match_depth == 0 && self.at(TokenKind::Newline) {
                break;
            }
            let token = self.bump();
            end = token.range;
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                TokenKind::Match => match_depth += 1,
                TokenKind::End if match_depth > 0 => {
                    match_depth = match_depth.saturating_sub(1);
                }
                _ => {}
            }
            if token.kind == TokenKind::Invalid {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.invalid_token",
                    message: "invalid token in expression".to_string(),
                    span: Some(self.source.span(token.range)),
                    parser_context: context,
                    unexpected: UnexpectedToken {
                        kind: token.kind.label().to_string(),
                        text: token.text.clone(),
                    },
                    expected: vec!["expression"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline".to_string()),
                        dropped_token_count: 1,
                    },
                    repair_candidates: Vec::new(),
                });
            } else if token.kind != TokenKind::Newline {
                tokens.push(token);
            } else {
                end = token.range;
            }
        }
        if self.at(TokenKind::Newline) {
            end = self.bump().range;
        }

        let (expr, diagnostics) = ExprParser::new(self.source, context, &tokens).parse();
        self.diagnostics.extend(diagnostics);
        (expr, start.cover(end))
    }

    fn expect_ident(&mut self, context: &'static str, expected: &'static str) -> Option<String> {
        if self.at(TokenKind::Ident) {
            Some(self.bump().text)
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            None
        }
    }

    fn expect_ident_text(
        &mut self,
        text: &'static str,
        context: &'static str,
        expected: &'static str,
    ) -> Token {
        if self.at_ident_text(text) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn eat_ident_text(
        &mut self,
        text: &str,
        context: &'static str,
        expected: &'static str,
    ) -> Option<String> {
        if self.at(TokenKind::Ident) && self.current().text == text {
            Some(self.bump().text)
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            None
        }
    }

    fn expect_name_path(
        &mut self,
        context: &'static str,
        expected: &'static str,
    ) -> Option<String> {
        let mut segments = vec![self.expect_ident(context, expected)?];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if let Some(segment) = self.expect_ident(context, expected) {
                segments.push(segment);
            } else {
                break;
            }
        }
        Some(segments.join("::"))
    }

    fn expect_newline(&mut self, context: &'static str) -> Token {
        if self.at(TokenKind::Newline) {
            self.bump()
        } else if self.at(TokenKind::Eof) {
            self.current().clone()
        } else {
            self.error_current(
                "parse.expected_newline",
                "expected a newline",
                context,
                vec!["newline"],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn expect(
        &mut self,
        kind: TokenKind,
        context: &'static str,
        expected: Vec<&'static str>,
    ) -> Token {
        if self.at(kind) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_token",
                format!("expected {}", expected.join(" or ")),
                context,
                expected,
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        parser_context: &'static str,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let current = self.current().clone();
        self.error_at_token(
            &current,
            DiagnosticRequest {
                id,
                message: message.into(),
                parser_context,
                expected,
                strategy,
                anchor,
                repair_candidates: Vec::new(),
            },
        );
    }

    fn error_at_token(&mut self, token: &Token, request: DiagnosticRequest) {
        self.diagnostics.push(ParseDiagnostic {
            id: request.id,
            message: request.message,
            span: Some(self.source.span(token.range)),
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
        });
    }

    fn synchronize_to_item(&mut self) {
        let start = self.cursor;
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Pub)
            && !self.at(TokenKind::Fn)
            && !self.at(TokenKind::Type)
            && !self.at(TokenKind::Schema)
            && !self.at(TokenKind::Codec)
            && !self.at(TokenKind::Test)
            && !self.at(TokenKind::End)
        {
            self.bump();
        }
        let at_eof = self.at(TokenKind::Eof);
        let anchor = match self.current().kind {
            TokenKind::Pub => Some("pub".to_string()),
            TokenKind::Fn => Some("fn".to_string()),
            TokenKind::Type => Some("type".to_string()),
            TokenKind::Schema => Some("schema".to_string()),
            TokenKind::Codec => Some("codec".to_string()),
            TokenKind::Test => Some("test".to_string()),
            TokenKind::End => Some("end".to_string()),
            TokenKind::Eof => None,
            _ => None,
        };
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
            if anchor.is_some() || at_eof {
                last.recovery.anchor = anchor;
            }
        }
    }

    fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn skip_to_next_line(&mut self) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            self.bump();
        }
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn peek_at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| token.kind == kind)
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind)
    }

    fn at_ident_text(&self, text: &str) -> bool {
        self.at(TokenKind::Ident) && self.current().text == text
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn previous(&self) -> Option<&Token> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        token
    }
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

impl<'a> ExprParser<'a> {
    fn new(source: &'a SourceFile, context: &'static str, tokens: &'a [Token]) -> Self {
        Self {
            source,
            context,
            tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> (Expr, Vec<ParseDiagnostic>) {
        let expr = self.parse_expr(0);
        self.report_trailing_tokens(
            "parse.expected_newline",
            "expected a newline before this token",
        );
        (expr, self.diagnostics)
    }

    fn parse_pattern_only(mut self) -> (Pattern, Vec<ParseDiagnostic>) {
        let pattern = self.parse_pattern();
        self.report_trailing_tokens_with_expected(
            "parse.pattern",
            "expected the pattern to end before this token",
            vec!["pattern end"],
            None,
        );
        (pattern, self.diagnostics)
    }

    fn report_trailing_tokens(&mut self, id: &'static str, message: &'static str) {
        self.report_trailing_tokens_with_expected(id, message, vec!["newline"], Some("newline"));
    }

    fn report_trailing_tokens_with_expected(
        &mut self,
        id: &'static str,
        message: &'static str,
        expected: Vec<&'static str>,
        anchor: Option<&'static str>,
    ) {
        if self.cursor < self.tokens.len() {
            self.error_current(id, message, expected, RecoveryStrategy::InsertToken, anchor);
        }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Expr {
        let mut lhs = self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                let token = self.bump();
                lhs = Expr {
                    span: self.source.span(lhs_range(&lhs).cover(token.range)),
                    kind: ExprKind::Try(Box::new(lhs)),
                };
                continue;
            }

            let Some((op, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.parse_expr(right_bp);
            let span = self.source.span(lhs_range(&lhs).cover(lhs_range(&rhs)));
            lhs = Expr {
                kind: ExprKind::Binary {
                    op,
                    left: Box::new(lhs),
                    right: Box::new(rhs),
                },
                span,
            };
        }

        lhs
    }

    fn parse_prefix(&mut self) -> Expr {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) {
            let token = self.bump();
            let op = if token.kind == TokenKind::Not {
                PrefixOp::Not
            } else {
                PrefixOp::Negate
            };
            let expr = self.parse_expr(13);
            return Expr {
                span: self.source.span(token.range.cover(lhs_range(&expr))),
                kind: ExprKind::Prefix {
                    op,
                    expr: Box::new(expr),
                },
            };
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            if self.call_type_arguments_start(&expr) {
                expr = self.parse_call_type_apply(expr);
                continue;
            }
            if self.at(TokenKind::LParen) {
                expr = self.parse_call_postfix(expr);
                continue;
            }
            if self.at(TokenKind::Dot) {
                expr = self.parse_field_postfix(expr);
                continue;
            }
            if self.at(TokenKind::Question) {
                expr = self.parse_try_postfix(expr);
                continue;
            }
            break;
        }
        expr
    }

    fn call_type_arguments_start(&self, expr: &Expr) -> bool {
        self.at(TokenKind::Less)
            && matches!(expr.kind, ExprKind::NamePath(_))
            && self.angle_type_arguments_are_followed_by_call()
    }

    fn parse_call_type_apply(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        self.parse_type_apply(expr, start, TokenKind::Greater)
    }

    fn parse_type_apply(&mut self, expr: Expr, start: TextRange, closing: TokenKind) -> Expr {
        let (type_args, end) = self.parse_type_argument_list(closing);
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::TypeApply {
                callee: Box::new(expr),
                type_args,
            },
        }
    }

    fn parse_call_postfix(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        self.bump();
        let mut args = Vec::new();
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            args.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            if self.at(TokenKind::RParen) || self.is_at_end() {
                break;
            }
            self.error_current(
                "parse.call_argument",
                "call argument is missing `,` or `)`",
                vec![",", ")"],
                RecoveryStrategy::InsertToken,
                Some(","),
            );
        }
        let end = self.eat(TokenKind::RParen).map_or_else(
            || lhs_range(args.last().unwrap_or(&expr)),
            |token| token.range,
        );
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::Call {
                callee: Box::new(expr),
                args,
            },
        }
    }

    fn parse_field_postfix(&mut self, expr: Expr) -> Expr {
        let start = lhs_range(&expr);
        let dot = self.bump();
        let (field, field_range) = if self.at(TokenKind::Ident) {
            let field = self.bump();
            (field.text, field.range)
        } else {
            self.error_current(
                "parse.field_access",
                "field access is missing a field name",
                vec!["field name"],
                RecoveryStrategy::InsertToken,
                None,
            );
            (String::new(), dot.range)
        };
        Expr {
            span: self.source.span(start.cover(field_range)),
            kind: ExprKind::FieldAccess {
                base: Box::new(expr),
                field,
                field_span: self.source.span(field_range),
            },
        }
    }

    fn parse_try_postfix(&mut self, expr: Expr) -> Expr {
        let token = self.bump();
        Expr {
            span: self.source.span(lhs_range(&expr).cover(token.range)),
            kind: ExprKind::Try(Box::new(expr)),
        }
    }

    fn angle_type_arguments_are_followed_by_call(&self) -> bool {
        if !self.at(TokenKind::Less) {
            return false;
        }
        let mut cursor = self.cursor + 1;
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::LParen => paren_depth += 1,
                TokenKind::RParen => paren_depth = paren_depth.saturating_sub(1),
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => brace_depth = brace_depth.saturating_sub(1),
                TokenKind::LBracket => bracket_depth += 1,
                TokenKind::RBracket => bracket_depth = bracket_depth.saturating_sub(1),
                TokenKind::Less => angle_depth += 1,
                TokenKind::Greater
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    return self
                        .tokens
                        .get(cursor + 1)
                        .is_some_and(|next| next.kind == TokenKind::LParen);
                }
                TokenKind::Greater => angle_depth = angle_depth.saturating_sub(1),
                TokenKind::Newline | TokenKind::Eof => return false,
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    fn parse_type_argument_list(&mut self, close: TokenKind) -> (Vec<String>, TextRange) {
        let start = self.bump();
        let mut args = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut end = start.range;

        while !self.is_at_end() {
            let token = self.bump();
            end = token.range;
            match token.kind {
                kind if kind == close
                    && paren_depth == 0
                    && brace_depth == 0
                    && bracket_depth == 0
                    && angle_depth == 0 =>
                {
                    if !current.is_empty() {
                        args.push(normalize_type_text(vec![current]));
                    }
                    return (args, end);
                }
                TokenKind::Comma
                    if paren_depth == 0
                        && brace_depth == 0
                        && bracket_depth == 0
                        && angle_depth == 0 =>
                {
                    args.push(normalize_type_text(vec![current]));
                    current = String::new();
                }
                TokenKind::LParen => {
                    paren_depth += 1;
                    current.push_str(&token.text);
                }
                TokenKind::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                    current.push_str(&token.text);
                }
                TokenKind::LBrace => {
                    brace_depth += 1;
                    current.push_str(&token.text);
                }
                TokenKind::RBrace => {
                    brace_depth = brace_depth.saturating_sub(1);
                    current.push_str(&token.text);
                }
                TokenKind::LBracket => {
                    bracket_depth += 1;
                    current.push_str(&token.text);
                }
                TokenKind::RBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    current.push_str(&token.text);
                }
                TokenKind::Less => {
                    angle_depth += 1;
                    current.push_str(&token.text);
                }
                TokenKind::Greater => {
                    angle_depth = angle_depth.saturating_sub(1);
                    current.push_str(&token.text);
                }
                _ => current.push_str(&token.text),
            }
        }

        if !current.is_empty() {
            args.push(normalize_type_text(vec![current]));
        }
        self.error_current(
            "parse.type_argument_list",
            "type argument list is missing its closing delimiter",
            vec![if close == TokenKind::Greater {
                ">"
            } else {
                "]"
            }],
            RecoveryStrategy::CloseBlock,
            Some(if close == TokenKind::Greater {
                ">"
            } else {
                "]"
            }),
        );
        (args, end)
    }

    fn parse_primary(&mut self) -> Expr {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return self.missing_expr();
        };

        match token.kind {
            TokenKind::Underscore | TokenKind::Hole => self.parse_hole_primary(token),
            TokenKind::String => self.parse_literal_primary(token, ExprKind::StringLiteral),
            TokenKind::Int => self.parse_literal_primary(token, ExprKind::IntLiteral),
            TokenKind::Float => self.parse_literal_primary(token, ExprKind::FloatLiteral),
            TokenKind::Ident => self.parse_name_path(),
            TokenKind::LParen => self.parse_group_or_unit_primary(),
            TokenKind::LBrace => self.parse_record_or_dict_primary(),
            TokenKind::LBracket => self.parse_list(),
            TokenKind::Match => self.parse_match(),
            _ => self.parse_missing_primary(token),
        }
    }

    fn parse_hole_primary(&mut self, token: Token) -> Expr {
        self.bump();
        let name = token
            .text
            .strip_prefix('_')
            .and_then(|suffix| (!suffix.is_empty()).then(|| suffix.to_string()));
        let satisfy = self.parse_satisfy_clause();
        Expr {
            kind: ExprKind::Hole { name, satisfy },
            span: self.source.span(token.range),
        }
    }

    fn parse_literal_primary(
        &mut self,
        token: Token,
        kind: impl FnOnce(String) -> ExprKind,
    ) -> Expr {
        self.bump();
        Expr {
            kind: kind(token.text),
            span: self.source.span(token.range),
        }
    }

    fn parse_group_or_unit_primary(&mut self) -> Expr {
        let start = self.bump();
        if let Some(end) = self.eat(TokenKind::RParen) {
            return Expr {
                kind: ExprKind::Unit,
                span: self.source.span(start.range.cover(end.range)),
            };
        }
        let expr = self.parse_expr(0);
        let end = self
            .eat(TokenKind::RParen)
            .map_or_else(|| lhs_range(&expr), |token| token.range);
        Expr {
            span: self.source.span(start.range.cover(end)),
            ..expr
        }
    }

    fn parse_record_or_dict_primary(&mut self) -> Expr {
        if self.peek_kind(1) == Some(TokenKind::RBrace)
            || (self.peek_kind(1) == Some(TokenKind::Ident)
                && self.peek_kind(2) == Some(TokenKind::Colon))
        {
            self.parse_record()
        } else {
            self.parse_dict()
        }
    }

    fn parse_missing_primary(&mut self, token: Token) -> Expr {
        self.bump();
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(token.range),
        }
    }

    fn parse_match(&mut self) -> Expr {
        let start = self.bump().range;
        let scrutinee = self.parse_expr(0);
        self.eat_newlines();
        let mut arms = Vec::new();
        while !self.at(TokenKind::End) && !self.is_at_end() {
            if self.at(TokenKind::Newline) {
                self.bump();
                continue;
            }
            let arm_start = self.current().range;
            let pattern = self.parse_pattern();
            self.expect_expr_token(
                TokenKind::FatArrow,
                "parse.match_arm",
                "match arm is missing `=>`",
                vec!["=>"],
            );
            let expr = self.parse_expr(0);
            let arm_end = lhs_range(&expr);
            arms.push(MatchArm {
                pattern,
                expr,
                span: self.source.span(arm_start.cover(arm_end)),
            });
            self.eat_newlines();
        }
        let end = self.eat(TokenKind::End).map_or_else(
            || {
                arms.last().map_or(lhs_range(&scrutinee), |arm| {
                    TextRange::new(arm.span.start.offset, arm.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_pattern(&mut self) -> Pattern {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return Pattern {
                kind: PatternKind::Wildcard,
                span: self.source.span(TextRange::at(self.source.len())),
            };
        };
        match token.kind {
            TokenKind::Underscore => {
                self.bump();
                Pattern {
                    kind: PatternKind::Wildcard,
                    span: self.source.span(token.range),
                }
            }
            TokenKind::String => {
                self.bump();
                Pattern {
                    kind: PatternKind::StringLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Int => {
                self.bump();
                Pattern {
                    kind: PatternKind::IntLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::Float => {
                self.bump();
                Pattern {
                    kind: PatternKind::FloatLiteral(token.text),
                    span: self.source.span(token.range),
                }
            }
            TokenKind::LParen => {
                let start = self.bump().range;
                if let Some(end) = self.eat(TokenKind::RParen) {
                    Pattern {
                        kind: PatternKind::Unit,
                        span: self.source.span(start.cover(end.range)),
                    }
                } else {
                    self.error_current(
                        "parse.pattern",
                        "unsupported parenthesized pattern",
                        vec!["pattern"],
                        RecoveryStrategy::SkipToken,
                        None,
                    );
                    Pattern {
                        kind: PatternKind::Wildcard,
                        span: self.source.span(start),
                    }
                }
            }
            TokenKind::LBrace => self.parse_record_pattern(),
            TokenKind::Ident => self.parse_name_pattern(),
            _ => {
                self.error_current(
                    "parse.pattern",
                    "expected a match pattern",
                    vec!["pattern"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
                Pattern {
                    kind: PatternKind::Wildcard,
                    span: self.source.span(token.range),
                }
            }
        }
    }

    fn parse_record_pattern(&mut self) -> Pattern {
        let start = self.bump().range;
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.current().range;
            let name = if self.at(TokenKind::Ident) {
                self.bump().text
            } else {
                self.error_current(
                    "parse.pattern",
                    "record pattern field is missing a name",
                    vec!["field name"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
                String::new()
            };
            self.expect_expr_token(
                TokenKind::Colon,
                "parse.pattern",
                "record pattern field is missing `:`",
                vec![":"],
            );
            let pattern = self.parse_pattern();
            let span = self.source.span(field_start.cover(pattern_range(&pattern)));
            fields.push(PatternField {
                name,
                pattern,
                span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                fields.last().map_or(start, |field| {
                    TextRange::new(field.span.start.offset, field.span.end.offset)
                })
            },
            |token| token.range,
        );
        Pattern {
            kind: PatternKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_name_pattern(&mut self) -> Pattern {
        let start = self.current().range;
        let mut end = start;
        let mut segments = vec![self.bump().text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                let segment = self.bump();
                end = segment.range;
                segments.push(segment.text);
            } else {
                break;
            }
        }
        if segments == ["true"] {
            return Pattern {
                kind: PatternKind::BoolLiteral(true),
                span: self.source.span(start.cover(end)),
            };
        }
        if segments == ["false"] {
            return Pattern {
                kind: PatternKind::BoolLiteral(false),
                span: self.source.span(start.cover(end)),
            };
        }
        let is_constructor = segments.len() > 1
            || segments
                .last()
                .and_then(|name| name.chars().next())
                .is_some_and(char::is_uppercase);
        if !is_constructor {
            return Pattern {
                kind: PatternKind::Binding(segments.remove(0)),
                span: self.source.span(start.cover(end)),
            };
        }
        let mut args = Vec::new();
        if self.eat(TokenKind::LParen).is_some() {
            while !self.at(TokenKind::RParen) && !self.is_at_end() {
                args.push(self.parse_pattern());
                if self.eat(TokenKind::Comma).is_none() {
                    break;
                }
            }
            if let Some(close) = self.eat(TokenKind::RParen) {
                end = close.range;
            }
        }
        Pattern {
            kind: PatternKind::Constructor {
                name: segments,
                args,
            },
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_satisfy_clause(&mut self) -> Option<SatisfyClause> {
        if !self.at_ident_text("satisfy") {
            return None;
        }
        let start = self.bump().range;
        let (candidate, candidate_span) = if self.at(TokenKind::Ident) {
            let token = self.bump();
            let span = self.source.span(token.range);
            (Some(token.text), Some(span))
        } else {
            self.error_current(
                "parse.satisfy_candidate",
                "satisfy clause is missing a candidate binding",
                vec!["candidate binding"],
                RecoveryStrategy::InsertToken,
                Some("=>"),
            );
            (None, None)
        };
        let mut end = if let Some(token) = self.eat(TokenKind::FatArrow) {
            token.range
        } else {
            self.error_current(
                "parse.satisfy_arrow",
                "satisfy clause is missing `=>`",
                vec!["=>"],
                RecoveryStrategy::InsertToken,
                None,
            );
            candidate_span.as_ref().map_or(start, |span| {
                TextRange::new(span.start.offset, span.end.offset)
            })
        };
        let mut parts = Vec::new();
        let mut predicate_tokens = Vec::new();
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(self.cursor) {
            if depth == 0
                && matches!(
                    token.kind,
                    TokenKind::Comma
                        | TokenKind::RParen
                        | TokenKind::RBracket
                        | TokenKind::RBrace
                        | TokenKind::Eof
                )
            {
                break;
            }
            match token.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    depth = depth.saturating_sub(1);
                }
                _ => {}
            }
            let token = self.bump();
            end = token.range;
            parts.push(token.text.clone());
            predicate_tokens.push(token);
        }
        self.diagnostics.extend(
            ContractPredicateParser::new(
                self.source,
                "satisfy_predicate",
                "parse.satisfy_predicate",
                &predicate_tokens,
            )
            .parse(),
        );
        Some(SatisfyClause {
            candidate,
            candidate_span,
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        })
    }

    fn parse_name_path(&mut self) -> Expr {
        let start = self.current().range;
        let mut end = start;
        let mut segments = vec![self.bump().text];
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                let segment = self.bump();
                end = segment.range;
                segments.push(segment.text);
            } else {
                break;
            }
        }
        if segments == ["true"] {
            return Expr {
                kind: ExprKind::BoolLiteral(true),
                span: self.source.span(start.cover(end)),
            };
        }
        if segments == ["false"] {
            return Expr {
                kind: ExprKind::BoolLiteral(false),
                span: self.source.span(start.cover(end)),
            };
        }
        Expr {
            kind: ExprKind::NamePath(segments),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_list(&mut self) -> Expr {
        let start = self.bump().range;
        let mut items = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.is_at_end() {
            items.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBracket).map_or_else(
            || items.last().map_or(start, lhs_range),
            |token| token.range,
        );
        Expr {
            kind: ExprKind::List(items),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_record(&mut self) -> Expr {
        let start = self.bump().range;
        let mut fields = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let field_start = self.current().range;
            let name = if self.at(TokenKind::Ident) {
                self.bump().text
            } else {
                self.bump();
                String::new()
            };
            self.eat(TokenKind::Colon);
            let expr = self.parse_expr(0);
            let field_span = self.source.span(field_start.cover(lhs_range(&expr)));
            fields.push(RecordField {
                name,
                expr,
                span: field_span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                fields.last().map_or(start, |field| {
                    TextRange::new(field.span.start.offset, field.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_dict(&mut self) -> Expr {
        let start = self.bump().range;
        let mut entries = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            let entry_start = self.current().range;
            let key = self.parse_expr(0);
            self.eat(TokenKind::Colon);
            let value = self.parse_expr(0);
            let entry_span = self.source.span(entry_start.cover(lhs_range(&value)));
            entries.push(DictEntry {
                key,
                value,
                span: entry_span,
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || {
                entries.last().map_or(start, |entry| {
                    TextRange::new(entry.span.start.offset, entry.span.end.offset)
                })
            },
            |token| token.range,
        );
        Expr {
            kind: ExprKind::Dict(entries),
            span: self.source.span(start.cover(end)),
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::PipeGreater => Some((BinaryOp::PipeGreater, 1, 2)),
            TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
            TokenKind::And => Some((BinaryOp::And, 5, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 7, 8)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 7, 8)),
            TokenKind::Less => Some((BinaryOp::Less, 9, 10)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 9, 10)),
            TokenKind::Greater => Some((BinaryOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 9, 10)),
            TokenKind::Plus => Some((BinaryOp::Add, 11, 12)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 11, 12)),
            TokenKind::Star => Some((BinaryOp::Multiply, 13, 14)),
            TokenKind::Slash => Some((BinaryOp::Divide, 13, 14)),
            _ => None,
        }
    }

    fn missing_expr(&self) -> Expr {
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(TextRange::at(self.source.len())),
        }
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind)
    }

    fn at_ident_text(&self, text: &str) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == TokenKind::Ident && token.text == text)
    }

    fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .unwrap_or_else(|| Token::eof(self.source.len()));
        self.error_at_token(
            &token,
            DiagnosticRequest {
                id,
                message: message.into(),
                parser_context: self.context,
                expected,
                strategy,
                anchor,
                repair_candidates: Vec::new(),
            },
        );
    }

    fn error_at_token(&mut self, token: &Token, request: DiagnosticRequest) {
        self.diagnostics.push(ParseDiagnostic {
            id: request.id,
            message: request.message,
            span: Some(self.source.span(token.range)),
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
        });
    }

    fn expect_expr_token(
        &mut self,
        kind: TokenKind,
        id: &'static str,
        message: impl Into<String>,
        expected: Vec<&'static str>,
    ) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            self.error_current(id, message, expected, RecoveryStrategy::InsertToken, None);
            None
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}

struct ContractPredicateParser<'a> {
    source: &'a SourceFile,
    context: &'static str,
    diagnostic_id: &'static str,
    tokens: &'a [Token],
    cursor: usize,
    diagnostics: Vec<ParseDiagnostic>,
}

impl<'a> ContractPredicateParser<'a> {
    fn new(
        source: &'a SourceFile,
        context: &'static str,
        diagnostic_id: &'static str,
        tokens: &'a [Token],
    ) -> Self {
        Self {
            source,
            context,
            diagnostic_id,
            tokens,
            cursor: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> Vec<ParseDiagnostic> {
        if self.tokens.is_empty() {
            self.error_current(
                "contract predicate is empty",
                vec!["contract predicate"],
                RecoveryStrategy::InsertToken,
                None,
            );
            return self.diagnostics;
        }

        self.parse_predicate(0);
        if !self.is_at_end() {
            if self.at(TokenKind::PipeGreater) {
                self.error_current(
                    "pipeline syntax is not allowed in a contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
            } else {
                self.error_current(
                    "unexpected token in contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
            }
        }
        self.diagnostics
    }

    fn parse_predicate(&mut self, min_bp: u8) {
        self.parse_prefix();

        loop {
            if self.at(TokenKind::Question) {
                self.error_current(
                    "`?` is not allowed in a contract predicate",
                    vec!["contract predicate operator", "end of predicate"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
                continue;
            }

            let Some((_, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            self.bump();
            self.parse_predicate(right_bp);
        }
    }

    fn parse_prefix(&mut self) {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) {
            self.bump();
            self.parse_predicate(13);
            return;
        }
        self.parse_postfix();
    }

    fn parse_postfix(&mut self) {
        self.parse_primary();
        loop {
            if self.at(TokenKind::LParen) {
                self.parse_call_args();
                continue;
            }
            if self.at(TokenKind::Dot) {
                self.bump();
                if self.at(TokenKind::Ident) {
                    self.bump();
                } else {
                    self.error_current(
                        "contract predicate field access is missing a field name",
                        vec!["field name"],
                        RecoveryStrategy::InsertToken,
                        None,
                    );
                }
                continue;
            }
            break;
        }
    }

    fn parse_call_args(&mut self) {
        self.bump();
        if self.eat(TokenKind::RParen).is_some() {
            return;
        }
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            self.parse_predicate(0);
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        if self.eat(TokenKind::RParen).is_none() {
            self.error_current(
                "contract predicate call is missing `)`",
                vec![")"],
                RecoveryStrategy::InsertToken,
                None,
            );
        }
    }

    fn parse_primary(&mut self) {
        if self.is_at_end() {
            self.error_current(
                "contract predicate is missing an expression",
                vec!["contract predicate atom"],
                RecoveryStrategy::InsertToken,
                None,
            );
            return;
        }

        match self.current().kind {
            TokenKind::String | TokenKind::Int | TokenKind::Float | TokenKind::Ident => {
                self.parse_name_path_or_literal();
            }
            TokenKind::LParen => {
                self.bump();
                if self.eat(TokenKind::RParen).is_some() {
                    return;
                }
                self.parse_predicate(0);
                if self.eat(TokenKind::RParen).is_none() {
                    self.error_current(
                        "contract predicate grouping is missing `)`",
                        vec![")"],
                        RecoveryStrategy::InsertToken,
                        None,
                    );
                }
            }
            TokenKind::Hole | TokenKind::Underscore => {
                self.error_current(
                    "hole syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::LBracket => {
                self.error_current(
                    "list syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::LBrace => {
                self.error_current(
                    "record syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::Match => {
                self.error_current(
                    "`match` is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::PipeGreater => {
                self.error_current(
                    "pipeline syntax is not allowed in a contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            TokenKind::Invalid => {
                self.error_current(
                    "invalid token in contract predicate",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
            _ => {
                self.error_current(
                    "expected a contract predicate atom",
                    vec!["contract predicate atom"],
                    RecoveryStrategy::SkipToken,
                    None,
                );
                self.bump();
            }
        }
    }

    fn parse_name_path_or_literal(&mut self) {
        self.bump();
        while self.at(TokenKind::DoubleColon) {
            self.bump();
            if self.at(TokenKind::Ident) {
                self.bump();
            } else {
                self.error_current(
                    "qualified contract predicate name is missing a segment",
                    vec!["name segment"],
                    RecoveryStrategy::InsertToken,
                    None,
                );
                break;
            }
        }
    }

    fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::Or => Some((BinaryOp::Or, 3, 4)),
            TokenKind::And => Some((BinaryOp::And, 5, 6)),
            TokenKind::EqualEqual => Some((BinaryOp::Equal, 7, 8)),
            TokenKind::BangEqual => Some((BinaryOp::NotEqual, 7, 8)),
            TokenKind::Less => Some((BinaryOp::Less, 9, 10)),
            TokenKind::LessEqual => Some((BinaryOp::LessEqual, 9, 10)),
            TokenKind::Greater => Some((BinaryOp::Greater, 9, 10)),
            TokenKind::GreaterEqual => Some((BinaryOp::GreaterEqual, 9, 10)),
            TokenKind::Plus => Some((BinaryOp::Add, 11, 12)),
            TokenKind::Minus => Some((BinaryOp::Subtract, 11, 12)),
            TokenKind::Star => Some((BinaryOp::Multiply, 13, 14)),
            TokenKind::Slash => Some((BinaryOp::Divide, 13, 14)),
            _ => None,
        }
    }

    fn error_current(
        &mut self,
        message: impl Into<String>,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let token = self
            .tokens
            .get(self.cursor)
            .cloned()
            .unwrap_or_else(|| Token::eof(self.source.len()));
        self.diagnostics.push(ParseDiagnostic {
            id: self.diagnostic_id,
            message: message.into(),
            span: Some(self.source.span(token.range)),
            parser_context: self.context,
            unexpected: UnexpectedToken {
                kind: token.kind.label().to_string(),
                text: token.text,
            },
            expected,
            recovery: Recovery {
                strategy,
                anchor: anchor.map(str::to_string),
                dropped_token_count: usize::from(strategy == RecoveryStrategy::SkipToken),
            },
            repair_candidates: Vec::new(),
        });
    }

    fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn bump(&mut self) -> Token {
        let token = self.current().clone();
        self.cursor += 1;
        token
    }
}

fn lhs_range(expr: &Expr) -> TextRange {
    TextRange::new(expr.span.start.offset, expr.span.end.offset)
}

fn pattern_range(pattern: &Pattern) -> TextRange {
    TextRange::new(pattern.span.start.offset, pattern.span.end.offset)
}

fn schema_mapping_expr_text(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Missing => "<missing>".to_string(),
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(schema_mapping_expr_text)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", schema_mapping_expr_text(callee))
        }
        ExprKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, schema_mapping_expr_text(&field.expr)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        ExprKind::FieldAccess { base, field, .. } => {
            format!("{}.{field}", schema_mapping_expr_text(base))
        }
        ExprKind::Try(inner) => format!("{}?", schema_mapping_expr_text(inner)),
        ExprKind::List(items) => {
            let items = items
                .iter()
                .map(schema_mapping_expr_text)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        ExprKind::Dict(entries) => {
            let entries = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{}: {}",
                        schema_mapping_expr_text(&entry.key),
                        schema_mapping_expr_text(&entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {entries} }}")
        }
        ExprKind::Match { .. } => "match".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            format!(
                "{}<{}>",
                schema_mapping_expr_text(callee),
                type_args.join(", ")
            )
        }
        ExprKind::Prefix { op, expr } => match op {
            PrefixOp::Not => format!("not {}", schema_mapping_expr_text(expr)),
            PrefixOp::Negate => format!("-{}", schema_mapping_expr_text(expr)),
        },
        ExprKind::Binary { op, left, right } => {
            format!(
                "{} {} {}",
                schema_mapping_expr_text(left),
                binary_op_text(*op),
                schema_mapping_expr_text(right)
            )
        }
        ExprKind::Hole { name, .. } => format!("_{}", name.as_deref().unwrap_or("")),
    }
}

fn schema_mapping_expr_source_text(source: &SourceFile, expr: &Expr) -> String {
    if matches!(expr.kind, ExprKind::Missing) {
        return schema_mapping_expr_text(expr);
    }
    source
        .text()
        .get(expr.span.start.offset..expr.span.end.offset)
        .map(canonical_schema_mapping_expr_text)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| schema_mapping_expr_text(expr))
}

fn canonical_schema_mapping_expr_text(text: &str) -> String {
    let source = SourceFile::new("<schema-mapping-expression>", text);
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Newline | TokenKind::Eof
            )
        })
        .collect::<Vec<_>>();
    let mut out = String::new();
    for (index, token) in tokens.iter().enumerate() {
        if schema_mapping_space_before(&tokens, index, &out) {
            out.push(' ');
        }
        out.push_str(&token.text);
    }
    out
}

fn schema_mapping_space_before(tokens: &[Token], index: usize, out: &str) -> bool {
    if out.is_empty() || index == 0 {
        return false;
    }
    let prev = tokens[index - 1].kind;
    let current = tokens[index].kind;
    if matches!(
        current,
        TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::Comma
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::DoubleColon
            | TokenKind::Question
    ) || matches!(
        prev,
        TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot | TokenKind::DoubleColon
    ) {
        return false;
    }
    if current == TokenKind::RBrace {
        return prev != TokenKind::LBrace;
    }
    if matches!(
        prev,
        TokenKind::Comma | TokenKind::Colon | TokenKind::LBrace
    ) {
        return true;
    }
    if current == TokenKind::LParen {
        return !matches!(
            prev,
            TokenKind::Ident
                | TokenKind::Hole
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
        );
    }
    if current == TokenKind::LBracket {
        return !matches!(
            prev,
            TokenKind::Ident
                | TokenKind::Hole
                | TokenKind::RParen
                | TokenKind::RBracket
                | TokenKind::RBrace
        );
    }
    if current == TokenKind::LBrace {
        return !matches!(
            prev,
            TokenKind::LParen
                | TokenKind::LBracket
                | TokenKind::Comma
                | TokenKind::Colon
                | TokenKind::Equal
        );
    }
    true
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::PipeGreater => "|>",
        BinaryOp::Or => "or",
        BinaryOp::And => "and",
        BinaryOp::Equal => "==",
        BinaryOp::NotEqual => "!=",
        BinaryOp::Less => "<",
        BinaryOp::LessEqual => "<=",
        BinaryOp::Greater => ">",
        BinaryOp::GreaterEqual => ">=",
        BinaryOp::Add => "+",
        BinaryOp::Subtract => "-",
        BinaryOp::Multiply => "*",
        BinaryOp::Divide => "/",
    }
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
}

fn normalize_type_text(parts: Vec<String>) -> String {
    normalize_collected_text(parts)
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">")
}

fn unquote_string_token(text: &str) -> String {
    text.strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text)
        .to_string()
}
