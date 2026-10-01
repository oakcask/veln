use super::*;

impl<'a> Parser<'a> {
    pub(super) fn parse_use_declarations(&mut self) -> Vec<UseDecl> {
        let mut uses = Vec::new();
        loop {
            self.eat_newlines();
            if !self.at(TokenKind::Use) {
                return uses;
            }
            uses.push(self.parse_use_declaration());
        }
    }

    pub(super) fn parse_use_declaration(&mut self) -> UseDecl {
        let start = self
            .expect(TokenKind::Use, "use_declaration", vec!["use"])
            .range;
        let (name, name_spans) = self.parse_written_module_path("use_declaration", true);
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
            name_spans,
            package,
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_function_like(&mut self, kind: FunctionKind) -> FunctionDecl {
        let start = self.current().range;
        let header = self.parse_function_header(kind);
        let mut return_decl = self.parse_function_return_and_effects(kind);
        let diagnostic_count = self.diagnostics.len();
        self.expect_newline(Self::function_context(kind));
        if return_decl.effects.is_some() && self.diagnostics.len() > diagnostic_count {
            return_decl.effects_recovered = true;
        }

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
            name_span: header.name_span,
            effect_binder: header.effect_binder,
            params: header.params,
            return_binding: return_decl.binding,
            return_type: return_decl.ty,
            return_type_span: return_decl.ty_span,
            return_type_paths: return_decl.ty_paths,
            effects: return_decl.effects,
            effect_spans: return_decl.effect_spans,
            effects_recovered: return_decl.effects_recovered,
            callsite: return_decl.callsite,
            contracts,
            body,
            span: self.source.span(start.cover(end)),
            end_present,
        }
    }

    pub(super) fn parse_function_header(&mut self, kind: FunctionKind) -> FunctionHeader {
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
        let (name, name_span) = if self.at(TokenKind::Decode) {
            let token = self.bump();
            (Some(token.text), Some(self.source.span(token.range)))
        } else {
            self.expect_covered_name(Self::function_context(kind), "declaration name")
        };
        let effect_binder = self.parse_effect_binder(kind);
        self.expect(TokenKind::LParen, Self::parameter_context(kind), vec!["("]);
        let params = self.parse_params();
        self.expect(TokenKind::RParen, Self::parameter_context(kind), vec![")"]);
        FunctionHeader {
            visibility,
            name,
            name_span,
            effect_binder,
            params,
        }
    }

    pub(super) fn parse_effect_binder(&mut self, kind: FunctionKind) -> Option<EffectBinder> {
        if !self.at(TokenKind::Less) {
            return None;
        }
        let start = self.bump().range;
        if !self.at(TokenKind::Effect) {
            self.error_current(
                "parse.effect_binder",
                "function effect binder must use `<effect E>`",
                Self::function_context(kind),
                vec!["effect"],
                RecoveryStrategy::SynchronizeToAnchor,
                Some("("),
            );
            while !self.at(TokenKind::LParen)
                && !self.at(TokenKind::Newline)
                && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
            return None;
        }
        self.bump();
        let name = self.expect_ident(Self::function_context(kind), "effect row variable");
        let end = self
            .expect(TokenKind::Greater, Self::function_context(kind), vec![">"])
            .range;
        Some(EffectBinder {
            name: name.unwrap_or_default(),
            span: self.source.span(start.cover(end)),
        })
    }

    pub(super) fn parse_function_return_and_effects(
        &mut self,
        kind: FunctionKind,
    ) -> FunctionReturn {
        let return_context = Self::return_context(kind);
        let (binding, ty, ty_span, ty_paths) = if self.eat(TokenKind::Arrow).is_some() {
            let return_binding = if matches!(
                self.current().kind,
                TokenKind::Ident | TokenKind::Hole | TokenKind::Callsite
            ) && self.peek_at(TokenKind::Colon)
            {
                let name = self.bump();
                self.expect(TokenKind::Colon, return_context, vec![":"]);
                Some(crate::ResultBinding {
                    name: name.text,
                    span: self.source.span(name.range),
                })
            } else {
                None
            };
            let return_type_start = self.current().range;
            let (return_type, return_type_paths) = self.collect_return_type_until(
                return_context,
                &[
                    TokenKind::Effects,
                    TokenKind::Callsite,
                    TokenKind::Newline,
                    TokenKind::Eof,
                ],
            );
            let return_type_end = self
                .previous()
                .map_or(return_type_start, |token| token.range);
            let return_type_span = self.source.span(return_type_start.cover(return_type_end));
            (
                return_binding,
                Some(return_type),
                Some(return_type_span),
                return_type_paths,
            )
        } else {
            (None, None, None, Vec::new())
        };
        let (effects, effect_spans, effects_recovered) = if self.eat(TokenKind::Effects).is_some() {
            let diagnostic_count = self.diagnostics.len();
            let labels = self.parse_effect_list();
            let (effects, spans): (Vec<_>, Vec<_>) = labels.into_iter().unzip();
            (
                Some(effects),
                Some(spans),
                self.diagnostics.len() > diagnostic_count,
            )
        } else {
            (None, None, false)
        };
        let callsite = if kind == FunctionKind::Function && self.at(TokenKind::Callsite) {
            let modifier = self.bump();
            let span = self.source.span(modifier.range);
            while self.at(TokenKind::Callsite) {
                let duplicate = self.bump();
                let duplicate_span = self.source.span(duplicate.range);
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.duplicate_callsite_modifier",
                    message: "duplicate `callsite` function modifier".to_string(),
                    span: Some(duplicate_span.clone()),
                    parser_context: Self::function_context(kind),
                    unexpected: UnexpectedToken {
                        kind: duplicate.kind.label().to_string(),
                        text: duplicate.text,
                    },
                    expected: vec!["newline"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::SkipToken,
                        anchor: Some("newline".to_string()),
                        dropped_token_count: 1,
                    },
                    repair_candidates: vec![ParseRepairCandidate {
                        candidate_id: "parse.duplicate_callsite_modifier.remove".to_string(),
                        name: "Remove duplicate modifier".to_string(),
                        application_policy: "automatic".to_string(),
                        application_status: "available".to_string(),
                        edit_summary: "Remove the duplicate `callsite` modifier.".to_string(),
                        edits: vec![ParseRepairEdit {
                            span: duplicate_span,
                            replacement: String::new(),
                        }],
                    }],
                });
            }
            Some(span)
        } else {
            None
        };
        FunctionReturn {
            binding,
            ty,
            ty_span,
            ty_paths,
            effects,
            effect_spans,
            effects_recovered,
            callsite,
        }
    }

    pub(super) fn parse_function_body(&mut self) -> (Vec<BodyLine>, bool) {
        self.parse_declaration_body(|parser| parser.parse_body_line())
    }

    pub(super) fn report_missing_function_end(&mut self, kind: FunctionKind) {
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

    pub(super) fn function_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_declaration",
            FunctionKind::Test => "test_declaration",
        }
    }

    pub(super) fn parameter_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_parameters",
            FunctionKind::Test => "test_parameters",
        }
    }

    pub(super) fn return_context(kind: FunctionKind) -> &'static str {
        match kind {
            FunctionKind::Function => "function_return",
            FunctionKind::Test => "test_return",
        }
    }

    pub(super) fn parse_params(&mut self) -> Vec<Param> {
        self.parse_params_in_context("function_parameters", false)
    }

    pub(super) fn parse_params_in_context(
        &mut self,
        context: &'static str,
        require_types: bool,
    ) -> Vec<Param> {
        let mut params = Vec::new();
        while !self.at(TokenKind::RParen) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            let (name, name_span) = self.expect_covered_name(context, "parameter name");
            let mut is_variadic = false;
            let mut ty_span = None;
            let mut ty_paths = Vec::new();
            let ty = self.eat(TokenKind::Colon).map(|colon| {
                if self.eat_variadic_marker() {
                    is_variadic = true;
                }
                let ty_start = self.current().range;
                let (ty, paths) = self.collect_type_paths_until(
                    context,
                    &[TokenKind::Comma, TokenKind::RParen, TokenKind::Eof],
                );
                let ty_end = self.previous().map_or(colon.range, |token| token.range);
                ty_span = Some(self.source.span(ty_start.cover(ty_end)));
                ty_paths = paths;
                ty
            });
            if require_types && ty.is_none() {
                self.diagnostics.push(ParseDiagnostic {
                    id: "parse.effect_operation_parameter_type",
                    message: "effect operation parameter is missing a type annotation".to_string(),
                    span: Some(self.source.span(start)),
                    parser_context: context,
                    unexpected: UnexpectedToken {
                        kind: "identifier".to_string(),
                        text: name.clone().unwrap_or_default(),
                    },
                    expected: vec![":"],
                    recovery: Recovery {
                        strategy: RecoveryStrategy::InsertToken,
                        anchor: Some("parameter type".to_string()),
                        dropped_token_count: 0,
                    },
                    repair_candidates: Vec::new(),
                });
            }
            let end = self.previous().map_or(start, |token| token.range);
            params.push(Param {
                name: name.unwrap_or_default(),
                name_span: name_span.unwrap_or_else(|| self.source.span(start)),
                ty,
                ty_span,
                ty_paths,
                is_variadic,
                span: self.source.span(start.cover(end)),
            });
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        params
    }

    pub(super) fn eat_variadic_marker(&mut self) -> bool {
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

    pub(super) fn parse_effect_list(&mut self) -> Vec<(String, SourceSpan)> {
        self.expect(TokenKind::LBracket, "effect_declaration", vec!["["]);
        let mut effects = Vec::new();
        while !self.at(TokenKind::RBracket) && !self.at(TokenKind::Eof) {
            let start = self.current().range;
            if self.eat_variadic_marker() {
                if let Some(row) = self.expect_ident("effect_declaration", "effect row variable") {
                    let end = self.previous().map_or(start, |token| token.range);
                    effects.push((format!("...{row}"), self.source.span(start.cover(end))));
                }
            } else {
                let effect = self.parse_name_path_segments("effect_declaration", "effect name");
                if !effect.is_empty() {
                    let end = self.previous().map_or(start, |token| token.range);
                    effects.push((effect.join("::"), self.source.span(start.cover(end))));
                }
            }
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        self.expect(TokenKind::RBracket, "effect_declaration", vec!["]"]);
        effects
    }

    pub(super) fn parse_name_path_segments(
        &mut self,
        context: &'static str,
        expected_name: &'static str,
    ) -> Vec<String> {
        let mut segments = Vec::new();
        if let Some(segment) = self.expect_ident(context, expected_name) {
            segments.push(segment);
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if let Some(segment) = self.expect_ident(context, "path segment") {
                segments.push(segment);
            } else {
                break;
            }
        }
        segments
    }

    pub(super) fn parse_contract(&mut self) -> ContractClause {
        let start_token = self.bump();
        let kind = match start_token.kind {
            TokenKind::Require => ContractKind::Require,
            TokenKind::Ensure => ContractKind::Ensure,
            _ => ContractKind::Invariant,
        };
        let (text, predicate_tokens, end) = self.collect_until_newline();
        let predicate_output = ContractPredicateParser::new(
            self.source,
            "contract_predicate",
            "parse.contract_predicate",
            &predicate_tokens,
        )
        .parse();
        self.diagnostics.extend(predicate_output.diagnostics);
        let callsite_reference_spans = predicate_tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Callsite)
            .map(|token| self.source.span(token.range))
            .collect();
        ContractClause {
            kind,
            text,
            perform_effect_spans: predicate_output.perform_effect_spans,
            callsite_reference_spans,
            span: self.source.span(start_token.range.cover(end)),
        }
    }

    pub(super) fn parse_contracts(&mut self) -> Vec<ContractClause> {
        let mut contracts = Vec::new();
        loop {
            self.eat_newlines();
            if !self.at_contract_clause() {
                return contracts;
            }
            contracts.push(self.parse_contract());
        }
    }

    pub(super) fn at_contract_clause(&self) -> bool {
        matches!(
            self.current().kind,
            TokenKind::Require | TokenKind::Ensure | TokenKind::Invariant
        )
    }

    pub(super) fn parse_body_line(&mut self) -> BodyLine {
        let start = self.current().range;
        if self.at(TokenKind::Defer) {
            self.parse_defer_body_line()
        } else if self.at(TokenKind::Let) {
            self.bump();
            let pattern = self.parse_let_pattern();
            let (annotation, annotation_paths) = if self.eat(TokenKind::Colon).is_some() {
                let (annotation, paths) = self.collect_type_paths_until(
                    "let_statement",
                    &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                );
                (Some(annotation), paths)
            } else {
                (None, Vec::new())
            };
            self.expect(TokenKind::Equal, "let_statement", vec!["="]);
            let (expr, end) = self.parse_expr_for_body_line("let_statement");
            BodyLine::Let {
                pattern,
                annotation,
                annotation_paths,
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

    fn parse_defer_body_line(&mut self) -> BodyLine {
        let start = self.bump().range;
        let header_end = self.expect_newline("defer_statement").range;
        if self.cleanup_depth >= MAX_CLEANUP_NESTING {
            return self.recover_overdeep_defer_body_line(start, header_end);
        }
        self.cleanup_depth += 1;
        let (body, end_present) = self.parse_declaration_body(|parser| parser.parse_body_line());
        self.cleanup_depth -= 1;
        let end = self.previous().map_or(header_end, |token| token.range);
        let block_end = if end_present {
            self.tokens
                .get(self.cursor.saturating_sub(1))
                .filter(|token| token.kind == TokenKind::Newline)
                .and_then(|_| self.tokens.get(self.cursor.saturating_sub(2)))
                .map_or(end.start, |token| token.range.start)
        } else {
            end.end
        };
        if !end_present {
            self.error_current(
                "parse.defer_missing_end",
                "defer statement is missing `end`",
                "defer_statement",
                vec!["end"],
                RecoveryStrategy::CloseBlock,
                Some("end"),
            );
        }
        BodyLine::Defer {
            body,
            keyword_span: self.source.span(start),
            block_span: self.source.span(TextRange::new(
                header_end.end,
                block_end.max(header_end.end),
            )),
            span: self.source.span(start.cover(end)),
        }
    }

    fn recover_overdeep_defer_body_line(
        &mut self,
        start: TextRange,
        header_end: TextRange,
    ) -> BodyLine {
        self.error_current(
            "parse.cleanup_nesting_limit",
            "cleanup regions are nested too deeply",
            "defer_statement",
            vec!["less deeply nested cleanup regions"],
            RecoveryStrategy::CloseBlock,
            Some("end"),
        );
        let end = self.skip_overdeep_cleanup_region(header_end);
        BodyLine::Defer {
            body: Vec::new(),
            keyword_span: self.source.span(start),
            block_span: self.source.span(TextRange::new(header_end.end, end.start)),
            span: self.source.span(start.cover(end)),
        }
    }

    fn skip_overdeep_cleanup_region(&mut self, fallback: TextRange) -> TextRange {
        let mut depth = 1usize;
        let mut end = fallback;
        let mut previous_kind = None;
        let mut at_line_start = true;
        let mut delimiter_depth = 0usize;
        while !self.at(TokenKind::Eof) {
            let token = self.bump();
            let kind = token.kind;
            match kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => {
                    delimiter_depth += 1;
                }
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                    delimiter_depth = delimiter_depth.saturating_sub(1);
                }
                TokenKind::Match | TokenKind::Begin => depth += 1,
                TokenKind::Defer if at_line_start && delimiter_depth == 0 => depth += 1,
                TokenKind::If if previous_kind != Some(TokenKind::Else) => depth += 1,
                TokenKind::End => {
                    depth -= 1;
                    if depth == 0 {
                        end = token.range;
                        if self.at(TokenKind::Newline) {
                            self.bump();
                        }
                        return end;
                    }
                }
                _ => {}
            }
            at_line_start = kind == TokenKind::Newline;
            previous_kind = Some(kind);
            end = token.range;
        }
        end
    }
}
