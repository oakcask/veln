use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_schema_decl(&mut self) -> SchemaDecl {
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

        let body = self.parse_schema_body();
        if !body.end_present {
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
            format: body.format,
            fields: body.fields,
            validations: body.validations,
            span: self.source.span(start.cover(end)),
            end_present: body.end_present,
        }
    }

    pub(super) fn parse_schema_body(&mut self) -> SchemaBody {
        let mut body = SchemaBody::default();
        loop {
            self.eat_newlines();
            if self.eat_schema_end() {
                body.end_present = true;
                break;
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            self.parse_schema_body_clause(&mut body);
        }
        body
    }

    pub(super) fn eat_schema_end(&mut self) -> bool {
        if !self.at(TokenKind::End) {
            return false;
        }
        self.bump();
        self.eat(TokenKind::Newline);
        true
    }

    pub(super) fn parse_schema_body_clause(&mut self, body: &mut SchemaBody) {
        if self.at(TokenKind::Format) {
            if body.format.is_none() && !body.fields.is_empty() {
                self.error_current(
                    "parse.schema_field_before_format",
                    "schema field appears before a format clause",
                    "schema_field",
                    vec!["format"],
                    RecoveryStrategy::InsertToken,
                    Some("format"),
                );
            }
            let duplicate = body.format.is_some();
            let clause = self.parse_schema_format_clause(duplicate);
            if !duplicate {
                body.format = Some(clause);
            }
        } else if self.at_ident_text("validate") && !self.peek_at(TokenKind::Colon) {
            body.validations
                .push(self.parse_schema_validation_clause(body.format.is_some()));
        } else if self.at_ident_text("map") && !self.peek_at(TokenKind::Colon) {
            self.parse_removed_schema_mapping_clause();
        } else {
            body.fields.push(self.parse_schema_field());
        }
    }

    pub(super) fn parse_schema_format_clause(&mut self, duplicate: bool) -> SchemaFormatClause {
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

    pub(super) fn parse_schema_field(&mut self) -> SchemaField {
        let start = self.current().range;
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
        let type_start = self.current().range;
        let (ty, ty_paths) = self.collect_type_paths_until(
            "schema_field",
            &[TokenKind::Where, TokenKind::Newline, TokenKind::Eof],
        );
        if schema_repeated_field_type_missing_semicolon(&ty) {
            let type_end = self.previous().map_or(type_start, |token| token.range);
            self.diagnostics.push(ParseDiagnostic {
                id: "parse.schema_repeat_semicolon",
                message: "expected `;` between repeated schema payload and count expression"
                    .to_string(),
                span: Some(self.source.span(type_start.cover(type_end))),
                parser_context: "schema_field",
                unexpected: UnexpectedToken {
                    kind: "schema field type".to_string(),
                    text: ty.clone(),
                },
                expected: vec![";"],
                recovery: Recovery {
                    strategy: RecoveryStrategy::InsertToken,
                    anchor: Some("]".to_string()),
                    dropped_token_count: 0,
                },
                repair_candidates: Vec::new(),
            });
        }
        let where_clause = self
            .eat(TokenKind::Where)
            .map(|where_token| self.parse_schema_field_where_clause(where_token));
        let end = self.expect_newline("schema_field").range;
        SchemaField {
            name,
            ty,
            ty_paths,
            where_clause,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_schema_field_where_clause(
        &mut self,
        where_token: Token,
    ) -> SchemaFieldWhereClause {
        let start = where_token.range;
        let (parts, predicate_tokens, _, end) = self.collect_line_parts_and_tokens();
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
            let predicate_text = normalize_collected_text(parts.clone());
            if !is_byte_view_multiple_predicate_text(&predicate_text) {
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
        }
        SchemaFieldWhereClause {
            predicate: normalize_collected_text(parts),
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_schema_validation_clause(
        &mut self,
        has_format: bool,
    ) -> SchemaValidationClause {
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
        let (parts, predicate_tokens, _, end) = self.collect_line_parts_and_tokens();
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

    pub(super) fn parse_removed_schema_mapping_clause(&mut self) {
        let start = self.cursor;
        let token = self.current().clone();
        self.error_at_token(
            &token,
            DiagnosticRequest {
                id: "parse.schema_mapping_removed",
                message: "schema mapping clauses are no longer accepted".to_string(),
                parser_context: "schema_declaration",
                expected: vec!["field", "validate", "end"],
                strategy: RecoveryStrategy::SynchronizeToAnchor,
                anchor: Some("newline"),
                repair_candidates: Vec::new(),
            },
        );
        self.skip_to_next_line();
        while self.schema_mapping_assignment_line() {
            self.skip_to_next_line();
        }
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
        }
    }

    pub(super) fn parse_removed_codec_decl(&mut self) {
        let start = self.cursor;
        if self.at(TokenKind::Pub) {
            self.bump();
        }
        let token = self.current().clone();
        self.error_at_token(
            &token,
            DiagnosticRequest {
                id: "parse.codec_declaration_removed",
                message: "codec declarations are no longer accepted; use ordinary functions plus explicit schema decode and encode expressions".to_string(),
                parser_context: "codec_declaration",
                expected: vec!["fn", "schema", "decode expression", "encode expression"],
                strategy: RecoveryStrategy::SynchronizeToAnchor,
                anchor: Some("end"),
                repair_candidates: Vec::new(),
            },
        );
        self.skip_to_next_line();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                break;
            }
            if self.at(TokenKind::Pub)
                || self.at(TokenKind::Fn)
                || self.at(TokenKind::Test)
                || self.at(TokenKind::Type)
                || self.at(TokenKind::Schema)
                || self.at(TokenKind::Codec)
            {
                break;
            }
            self.skip_to_next_line();
        }
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
        }
    }

    pub(super) fn schema_mapping_assignment_line(&self) -> bool {
        if self.at(TokenKind::Eof)
            || self.at(TokenKind::End)
            || self.at(TokenKind::Format)
            || (self.at_ident_text("map") && !self.peek_at(TokenKind::Colon))
            || (self.at_ident_text("validate") && !self.peek_at(TokenKind::Colon))
        {
            return false;
        }
        let mut offset = 0usize;
        while let Some(kind) = self.peek_kind(offset) {
            match kind {
                TokenKind::Equal => return true,
                TokenKind::Colon | TokenKind::Newline | TokenKind::Eof => return false,
                _ => offset += 1,
            }
        }
        false
    }

    pub(super) fn parse_type_variant(&mut self) -> TypeVariantDecl {
        let start = self.current().range;
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let (name, name_span) = self.expect_covered_name("type_variant", "variant name");
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
            name_span,
            field_delimiter,
            fields,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_type_variant_fields(&mut self) -> Vec<TypeVariantField> {
        self.parse_type_variant_fields_until(TokenKind::RParen)
    }

    pub(super) fn parse_type_variant_fields_until(
        &mut self,
        close: TokenKind,
    ) -> Vec<TypeVariantField> {
        let mut fields = Vec::new();
        let mut positional_index = 0usize;
        while !self.at(close) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let (name, ty, ty_paths) =
                if self.at(TokenKind::Ident) && self.peek_at(TokenKind::Colon) {
                    let name = self
                        .expect_ident("type_variant", "variant field name")
                        .unwrap_or_default();
                    self.expect(TokenKind::Colon, "type_variant", vec![":"]);
                    let (ty, ty_paths) =
                        self.collect_type_paths_until("type_variant", &[TokenKind::Comma, close]);
                    (name, ty, ty_paths)
                } else {
                    let name = if positional_index == 0 {
                        "value".to_string()
                    } else {
                        format!("_{positional_index}")
                    };
                    let (ty, ty_paths) =
                        self.collect_type_paths_until("type_variant", &[TokenKind::Comma, close]);
                    positional_index += 1;
                    (name, ty, ty_paths)
                };
            let end = self.previous().map_or(start, |token| token.range);
            fields.push(TypeVariantField {
                name,
                ty,
                ty_paths,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        fields
    }

    pub(super) fn parse_named_header(
        &mut self,
        keyword: TokenKind,
        context: &'static str,
    ) -> (ModuleDecl, UseDecl) {
        let start = self.expect(keyword, context, vec!["keyword"]).range;
        let (name, name_spans) = self.parse_written_module_path(context, keyword == TokenKind::Mod);
        let end = self.expect_newline(context).range;
        let span = self.source.span(start.cover(end));
        (
            ModuleDecl {
                name: name.clone(),
                name_spans: name_spans.clone(),
                span: span.clone(),
            },
            UseDecl {
                name,
                name_spans,
                package: None,
                span,
            },
        )
    }
}
