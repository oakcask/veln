use super::*;

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a SourceFile, tokens: Vec<Token>) -> Self {
        let diagnostics = integer_literal_diagnostics(source, &tokens);
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
            diagnostics,
        }
    }

    pub(super) fn parse(mut self) -> ParseOutput {
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
            if let Some(item) = self.parse_top_level_item() {
                items.push(item);
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

    pub(super) fn parse_top_level_item(&mut self) -> Option<SyntaxItem> {
        if self.at_public_alias_header() {
            return Some(SyntaxItem::PublicAlias(self.parse_public_alias()));
        }
        if self.at(TokenKind::Pub) {
            return self.parse_public_top_level_item();
        }

        self.parse_private_top_level_item()
    }

    pub(super) fn parse_private_top_level_item(&mut self) -> Option<SyntaxItem> {
        match self.current().kind {
            TokenKind::Fn => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Function),
            ))),
            TokenKind::Type => Some(SyntaxItem::Type(self.parse_type_decl())),
            TokenKind::Schema => Some(SyntaxItem::Schema(self.parse_schema_decl())),
            TokenKind::Effect => Some(SyntaxItem::Effect(self.parse_effect_decl())),
            TokenKind::Handler => Some(SyntaxItem::Handler(self.parse_handler_decl())),
            TokenKind::Codec => {
                self.parse_removed_codec_decl();
                None
            }
            TokenKind::Test => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Test),
            ))),
            _ => self.reject_top_level_token(),
        }
    }

    pub(super) fn parse_public_top_level_item(&mut self) -> Option<SyntaxItem> {
        match self.peek_kind(1) {
            Some(TokenKind::Type) => Some(SyntaxItem::Type(self.parse_type_decl())),
            Some(TokenKind::Schema) => Some(SyntaxItem::Schema(self.parse_schema_decl())),
            Some(TokenKind::Effect) => Some(SyntaxItem::Effect(self.parse_effect_decl())),
            Some(TokenKind::Handler) => Some(SyntaxItem::Handler(self.parse_handler_decl())),
            Some(TokenKind::Codec) => {
                self.parse_removed_codec_decl();
                None
            }
            _ => Some(SyntaxItem::Function(Box::new(
                self.parse_function_like(FunctionKind::Function),
            ))),
        }
    }

    pub(super) fn reject_top_level_token(&mut self) -> Option<SyntaxItem> {
        self.error_current(
            "parse.expected_item",
            "expected a function, test, type, effect, handler, or schema declaration",
            "module",
            vec!["pub", "fn", "test", "type", "effect", "handler", "schema"],
            RecoveryStrategy::SynchronizeToAnchor,
            Some("fn"),
        );
        self.synchronize_to_item();
        None
    }

    pub(super) fn at_public_alias_header(&self) -> bool {
        self.at(TokenKind::Pub)
            && matches!(
                (self.peek_kind(1), self.peek_kind(2), self.peek_kind(3)),
                (
                    Some(TokenKind::Fn | TokenKind::Type | TokenKind::Schema),
                    Some(TokenKind::Ident | TokenKind::Hole),
                    Some(TokenKind::Equal)
                )
            )
    }

    pub(super) fn parse_public_alias(&mut self) -> PublicAliasDecl {
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
        let (name, name_span) = self.expect_covered_name("public_alias", "public member name");
        self.expect(TokenKind::Equal, "public_alias", vec!["="]);
        let (target, target_spans) = self.parse_member_alias_target();
        let end = self.expect_newline("public_alias").range;
        PublicAliasDecl {
            kind,
            name,
            name_span,
            target,
            target_spans,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_member_alias_target(&mut self) -> (Vec<String>, Vec<SourceSpan>) {
        let mut segments = Vec::new();
        let mut spans = Vec::new();
        if let (Some(segment), Some(span)) = self.expect_covered_name("public_alias", "member path")
        {
            segments.push(segment);
            spans.push(span);
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if let (Some(segment), Some(span)) =
                self.expect_covered_name("public_alias", "member path segment")
            {
                segments.push(segment);
                spans.push(span);
            }
        }
        (segments, spans)
    }

    pub(super) fn parse_type_decl(&mut self) -> TypeDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Type, "type_declaration", vec!["type"])
            .range;
        let (name, name_span) = self.expect_covered_name("type_declaration", "type name");
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

        let (variants, end_present) =
            self.parse_declaration_body(|parser| parser.parse_type_variant());

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
            name_span,
            params,
            variants,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    pub(super) fn parse_type_params_until(&mut self, close: TokenKind) -> Vec<String> {
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

    pub(super) fn parse_declaration_body<T>(
        &mut self,
        mut parse_item: impl FnMut(&mut Self) -> T,
    ) -> (Vec<T>, bool) {
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            self.eat_newlines();
            if self.at(TokenKind::End) {
                self.bump();
                if self.at(TokenKind::Newline) {
                    self.bump();
                }
                return (items, true);
            }
            if self.at(TokenKind::Eof) {
                break;
            }
            items.push(parse_item(self));
        }
        (items, false)
    }

    pub(super) fn parse_effect_decl(&mut self) -> EffectDecl {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Effect, "effect_declaration", vec!["effect"])
            .range;
        let name = self.expect_ident("effect_declaration", "effect name");
        self.expect_newline("effect_declaration");

        let (operations, end_present) =
            self.parse_declaration_body(|parser| parser.parse_effect_operation_decl());

        if operations.is_empty() {
            self.error_current(
                "parse.effect_operation_required",
                "effect declaration requires at least one operation",
                "effect_declaration",
                vec!["operation declaration"],
                RecoveryStrategy::InsertToken,
                Some("end"),
            );
        }

        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close effect declaration",
                "effect_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }

        let end = self.previous().map_or(start, |token| token.range);
        EffectDecl {
            visibility,
            name,
            operations,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    pub(super) fn parse_effect_operation_decl(&mut self) -> EffectOperationDecl {
        let start = self.current().range;
        let name_span = self.source.span(start);
        let name = self.expect_ident("effect_operation", "operation name");
        self.expect(TokenKind::LParen, "effect_operation", vec!["("]);
        let params = self.parse_params_in_context("effect_operation", true);
        self.expect(TokenKind::RParen, "effect_operation", vec![")"]);
        let (return_type, return_type_paths) = if self.eat(TokenKind::Arrow).is_some() {
            let (ty, paths) = self.collect_return_type_until(
                "effect_operation",
                &[TokenKind::Newline, TokenKind::Eof],
            );
            (Some(ty), paths)
        } else {
            self.error_current(
                "parse.effect_operation_return",
                "effect operation is missing `->` and a result type",
                "effect_operation",
                vec!["->"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
            (None, Vec::new())
        };
        let end = self.expect_newline("effect_operation").range;
        EffectOperationDecl {
            name,
            name_span,
            params,
            return_type,
            return_type_paths,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_handler_decl(&mut self) -> HandlerDecl {
        let header = self.parse_handler_header();
        let body = self.parse_handler_body(header.start);
        HandlerDecl {
            visibility: header.visibility,
            name: header.name,
            params: header.params,
            effect: header.effect.path,
            effect_span: header.effect.span,
            effects: header.effect.effects,
            effect_spans: header.effect.effect_spans,
            operation_clauses: body.operation_clauses,
            span: self.source.span(header.start.cover(body.end)),
            end_present: body.end_present,
        }
    }

    pub(super) fn parse_handler_header(&mut self) -> HandlerHeader {
        let visibility = if self.eat(TokenKind::Pub).is_some() {
            Visibility::Public
        } else {
            Visibility::Private
        };
        let start = self
            .expect(TokenKind::Handler, "handler_declaration", vec!["handler"])
            .range;
        let name = self.expect_ident("handler_declaration", "handler name");
        self.expect(TokenKind::LParen, "handler_parameters", vec!["("]);
        let params = self.parse_params_in_context("handler_parameters", true);
        self.expect(TokenKind::RParen, "handler_parameters", vec![")"]);
        self.expect(TokenKind::Handles, "handler_declaration", vec!["handles"]);
        let effect = self.parse_handler_effect();
        self.expect_newline("handler_declaration");
        HandlerHeader {
            visibility,
            start,
            name,
            params,
            effect,
        }
    }

    pub(super) fn parse_handler_effect(&mut self) -> HandlerEffect {
        let effect_start = self.current().range;
        let path = self.parse_name_path_segments("handler_declaration", "handled effect");
        let effect_end = self.previous().map_or(effect_start, |token| token.range);
        let span = self.source.span(effect_start.cover(effect_end));
        let (effects, effect_spans) = if self.eat(TokenKind::Effects).is_some() {
            let labels = self.parse_effect_list();
            let (effects, spans): (Vec<_>, Vec<_>) = labels.into_iter().unzip();
            (Some(effects), Some(spans))
        } else {
            (None, None)
        };
        HandlerEffect {
            path,
            span,
            effects,
            effect_spans,
        }
    }

    pub(super) fn parse_handler_body(&mut self, start: TextRange) -> HandlerBody {
        let (operation_clauses, end_present) =
            self.parse_declaration_body(|parser| parser.parse_handler_operation_clause_decl());
        if !end_present {
            self.error_current(
                "parse.expected_end",
                "expected `end` to close handler declaration",
                "handler_declaration",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }
        let end = self.previous().map_or(start, |token| token.range);
        HandlerBody {
            operation_clauses,
            end,
            end_present,
        }
    }

    pub(super) fn parse_handler_operation_clause_decl(&mut self) -> HandlerOperationClauseDecl {
        let start = self.current().range;
        let operation_span = self.source.span(start);
        let operation = self.expect_ident("handler_operation_clause", "operation name");
        if self.at(TokenKind::Equal) {
            let equal = self.current().clone();
            self.error_at_token(
                &equal,
                DiagnosticRequest {
                    id: "parse.handler_operation_old_syntax",
                    message: "handler operation clause must bind operation parameters with `(` and evaluate an expression with `=>`".to_string(),
                    parser_context: "handler_operation_clause",
                    expected: vec!["("],
                    strategy: RecoveryStrategy::SynchronizeToAnchor,
                    anchor: Some("newline"),
                    repair_candidates: Vec::new(),
                },
            );
            self.skip_to_next_line();
            let body = Expr {
                kind: ExprKind::Missing,
                span: self.source.span(equal.range),
            };
            return HandlerOperationClauseDecl {
                operation,
                operation_span,
                params: Vec::new(),
                body,
                span: self.source.span(start.cover(equal.range)),
            };
        }
        self.expect(TokenKind::LParen, "handler_operation_clause", vec!["("]);
        let params = self.parse_handler_operation_params();
        self.expect(TokenKind::RParen, "handler_operation_clause", vec![")"]);
        self.expect(TokenKind::FatArrow, "handler_operation_clause", vec!["=>"]);
        let (body, body_range) = self.parse_expr_for_body_line("handler_operation_clause");
        HandlerOperationClauseDecl {
            operation,
            operation_span,
            params,
            body,
            span: self.source.span(start.cover(body_range)),
        }
    }

    pub(super) fn parse_handler_operation_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.at(TokenKind::RParen) || self.at(TokenKind::Eof) {
            return params;
        }
        loop {
            let start = self.current().range;
            let (name, name_span) =
                self.expect_covered_name("handler_operation_clause", "operation parameter");
            params.push(Param {
                name: name.unwrap_or_default(),
                name_span: name_span.unwrap_or_else(|| self.source.span(start)),
                ty: None,
                ty_span: None,
                ty_paths: Vec::new(),
                is_variadic: false,
                span: self.source.span(start),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
            if self.at(TokenKind::RParen) {
                self.error_current(
                    "parse.handler_operation_parameter",
                    "handler operation parameter list cannot end with a comma",
                    "handler_operation_clause",
                    vec!["operation parameter"],
                    RecoveryStrategy::InsertToken,
                    Some("parameter"),
                );
                break;
            }
        }
        params
    }
}
