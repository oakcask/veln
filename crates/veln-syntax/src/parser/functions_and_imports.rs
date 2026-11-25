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
        let diagnostic_count = self.diagnostics.len();
        let start = self
            .expect(TokenKind::Use, "use_declaration", vec!["use"])
            .range;
        let (name, name_spans) = self.parse_written_module_path("use_declaration", true);
        let alias = self.parse_use_alias();
        let package = self.parse_use_package();
        self.recover_use_declaration_end();
        let end = self.expect_newline("use_declaration").range;
        UseDecl {
            name,
            name_spans,
            alias,
            package,
            recovered: self.diagnostics.len() != diagnostic_count,
            span: self.source.span(start.cover(end)),
        }
    }

    fn parse_use_alias(&mut self) -> Option<UseAlias> {
        if !self.at_ident_text("as") {
            return None;
        }
        self.bump();
        if is_contextual_identifier(self.current().kind)
            || self.at(TokenKind::Hole)
            || self.at(TokenKind::Underscore)
        {
            let token = self.bump();
            Some(UseAlias {
                name: token.text,
                span: self.source.span(token.range),
            })
        } else {
            self.error_current(
                "parse.expected_identifier",
                "expected import alias",
                "use_declaration",
                vec!["alias"],
                RecoveryStrategy::InsertToken,
                Some("newline"),
            );
            None
        }
    }

    fn parse_use_package(&mut self) -> Option<UsePackage> {
        if self.eat(TokenKind::From).is_some() {
            if self.at(TokenKind::String) {
                let token = self.bump();
                Some(UsePackage {
                    name: unquote_string_token(&token.text),
                    span: self.source.span(token.range),
                })
            } else {
                self.expect(TokenKind::String, "use_declaration", vec!["package"]);
                None
            }
        } else {
            None
        }
    }

    fn recover_use_declaration_end(&mut self) {
        if !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            self.error_current(
                "parse.expected_newline",
                "expected a newline after import declaration",
                "use_declaration",
                vec!["newline"],
                RecoveryStrategy::SynchronizeToAnchor,
                Some("newline"),
            );
            while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
                self.bump();
            }
        }
    }

    pub(super) fn parse_function_like(&mut self, kind: FunctionKind) -> FunctionDecl {
        let start = self.current().range;
        let header = self.parse_function_header(kind);
        let mut return_decl = self.parse_function_return_and_effects(kind);
        if !header.type_binders.is_empty()
            && let Some(callsite) = return_decl.callsite.as_ref()
        {
            self.diagnostics.push(ParseDiagnostic {
                id: "parse.generic_function_callsite",
                message: "ordinary generic functions cannot use the `callsite` modifier"
                    .to_string(),
                span: Some(callsite.clone()),
                parser_context: Self::function_context(kind),
                unexpected: UnexpectedToken {
                    kind: "callsite".to_string(),
                    text: "callsite".to_string(),
                },
                expected: vec!["ordinary fixed-arity function"],
                recovery: Recovery {
                    strategy: RecoveryStrategy::None,
                    anchor: Some("function declaration".to_string()),
                    dropped_token_count: 0,
                },
                repair_candidates: Vec::new(),
            });
        }
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
            type_binders: header.type_binders,
            effect_binder: header.effect_binder,
            params: header.params,
            return_binding: return_decl.binding,
            return_type: return_decl.ty,
            return_type_span: return_decl.ty_span,
            return_type_paths: return_decl.ty_paths,
            return_type_refinements: return_decl.ty_refinements,
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
        let (type_binders, effect_binder) = self.parse_function_binders(kind);
        self.expect(TokenKind::LParen, Self::parameter_context(kind), vec!["("]);
        let params = self.parse_params();
        self.expect(TokenKind::RParen, Self::parameter_context(kind), vec![")"]);
        if !type_binders.is_empty()
            && let Some(param) = params.iter().find(|param| param.is_variadic)
        {
            self.diagnostics.push(ParseDiagnostic {
                id: "parse.generic_function_variadic",
                message: "ordinary generic functions cannot be variadic".to_string(),
                span: Some(param.span.clone()),
                parser_context: Self::function_context(kind),
                unexpected: UnexpectedToken {
                    kind: "variadic parameter".to_string(),
                    text: param.name.clone(),
                },
                expected: vec!["fixed-arity parameter"],
                recovery: Recovery {
                    strategy: RecoveryStrategy::None,
                    anchor: Some("function parameter".to_string()),
                    dropped_token_count: 0,
                },
                repair_candidates: Vec::new(),
            });
        }
        FunctionHeader {
            visibility,
            name,
            name_span,
            type_binders,
            effect_binder,
            params,
        }
    }

    pub(super) fn parse_function_binders(
        &mut self,
        kind: FunctionKind,
    ) -> (Vec<TypeBinder>, Option<EffectBinder>) {
        if !self.at(TokenKind::Less) {
            return (Vec::new(), None);
        }
        let start = self.bump().range;
        let mut type_binders = Vec::new();
        let mut effect_binder = None;
        let mut names = std::collections::HashSet::new();
        let mut saw_effect = false;

        if self.at(TokenKind::Greater) {
            self.error_current(
                "parse.empty_function_binder_list",
                "function binder list cannot be empty",
                Self::function_context(kind),
                vec!["type binder", "effect"],
                RecoveryStrategy::None,
                Some(">"),
            );
            self.bump();
            return (type_binders, effect_binder);
        }

        loop {
            if self.at(TokenKind::Effect) {
                let effect = self.bump();
                if saw_effect {
                    self.error_current(
                        "parse.multiple_effect_binders",
                        "function binder list accepts at most one effect binder",
                        Self::function_context(kind),
                        vec![">"],
                        RecoveryStrategy::None,
                        Some("function binder list"),
                    );
                }
                saw_effect = true;
                let name_token = self.current().clone();
                let name = self.expect_ident(Self::function_context(kind), "effect row variable");
                if let Some(name) = name {
                    if !names.insert(name.clone()) {
                        self.report_duplicate_function_binder(&name_token, &name, kind);
                    }
                    if effect_binder.is_none() {
                        effect_binder = Some(EffectBinder {
                            name,
                            span: self.source.span(effect.range.cover(name_token.range)),
                        });
                    }
                }
            } else if self.at(TokenKind::Ident) {
                let binder = self.bump();
                if saw_effect {
                    self.report_function_binder_error(
                        &binder,
                        "parse.type_binder_after_effect",
                        "ordinary type binders must precede the effect binder",
                        vec![">"],
                        kind,
                    );
                }
                if !names.insert(binder.text.clone()) {
                    self.report_duplicate_function_binder(&binder, &binder.text, kind);
                }
                type_binders.push(TypeBinder {
                    name: binder.text,
                    span: self.source.span(binder.range),
                });
                if self.at(TokenKind::Colon) || self.at(TokenKind::Less) {
                    let invalid = self.bump();
                    let applied_binder = invalid.kind == TokenKind::Less;
                    self.report_function_binder_error(
                        &invalid,
                        "parse.function_binder_bound",
                        "function type binders do not accept bounds or type arguments",
                        vec![",", ">"],
                        kind,
                    );
                    if applied_binder {
                        self.skip_applied_function_binder();
                    } else {
                        self.skip_invalid_function_binder_entry();
                    }
                }
            } else {
                let invalid = self.bump();
                self.report_function_binder_error(
                    &invalid,
                    "parse.function_binder_entry",
                    "function binder entry must be a type name or `effect E`",
                    vec!["type binder", "effect"],
                    kind,
                );
                self.skip_invalid_function_binder_entry();
            }

            if self.eat(TokenKind::Comma).is_some() {
                if self.at(TokenKind::Greater) {
                    let end = self.bump().range;
                    if let Some(binder) = effect_binder.as_mut() {
                        binder.span = self.source.span(start.cover(end));
                    }
                    break;
                }
                continue;
            }
            if self.at(TokenKind::Greater) {
                let end = self.bump().range;
                if let Some(binder) = effect_binder.as_mut() {
                    binder.span = self.source.span(start.cover(end));
                }
                break;
            }

            self.error_current(
                "parse.function_binder_separator",
                "function binders must be separated by `,` and closed by `>`",
                Self::function_context(kind),
                vec![",", ">"],
                RecoveryStrategy::SynchronizeToAnchor,
                Some("function binder list"),
            );
            while !self.at(TokenKind::Comma)
                && !self.at(TokenKind::Greater)
                && !self.at(TokenKind::LParen)
                && !self.at(TokenKind::Newline)
                && !self.at(TokenKind::Eof)
            {
                self.bump();
            }
            if self.at(TokenKind::Comma) {
                self.bump();
                continue;
            }
            if self.at(TokenKind::Greater) {
                self.bump();
            }
            break;
        }

        if kind == FunctionKind::Test && !type_binders.is_empty() {
            let binder = &type_binders[0];
            self.diagnostics.push(ParseDiagnostic {
                id: "parse.generic_test_declaration",
                message: "test declarations cannot declare ordinary type binders".to_string(),
                span: Some(binder.span.clone()),
                parser_context: Self::function_context(kind),
                unexpected: UnexpectedToken {
                    kind: "type binder".to_string(),
                    text: binder.name.clone(),
                },
                expected: vec!["nongeneric test declaration"],
                recovery: Recovery {
                    strategy: RecoveryStrategy::None,
                    anchor: Some("test declaration".to_string()),
                    dropped_token_count: 0,
                },
                repair_candidates: Vec::new(),
            });
        }

        (type_binders, effect_binder)
    }

    fn skip_invalid_function_binder_entry(&mut self) {
        while !self.at(TokenKind::Comma)
            && !self.at(TokenKind::Greater)
            && !self.at(TokenKind::LParen)
            && !self.at(TokenKind::Newline)
            && !self.at(TokenKind::Eof)
        {
            self.bump();
        }
    }

    fn skip_applied_function_binder(&mut self) {
        let mut depth = 1usize;
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::Less) {
                depth += 1;
            } else if self.at(TokenKind::Greater) {
                depth -= 1;
                self.bump();
                if depth == 0 {
                    break;
                }
                continue;
            }
            self.bump();
        }
    }

    fn report_duplicate_function_binder(&mut self, token: &Token, name: &str, kind: FunctionKind) {
        self.report_function_binder_error(
            token,
            "parse.duplicate_function_binder",
            format!("duplicate function binder `{name}`"),
            vec!["unique binder name"],
            kind,
        );
    }

    fn report_function_binder_error(
        &mut self,
        token: &Token,
        id: &'static str,
        message: impl Into<String>,
        expected: Vec<&'static str>,
        kind: FunctionKind,
    ) {
        self.error_at_token(
            token,
            DiagnosticRequest {
                id,
                message: message.into(),
                parser_context: Self::function_context(kind),
                expected,
                strategy: RecoveryStrategy::None,
                anchor: Some("function binder list"),
                repair_candidates: Vec::new(),
            },
        );
    }

    pub(super) fn parse_function_return_and_effects(
        &mut self,
        kind: FunctionKind,
    ) -> FunctionReturn {
        let return_type = self.parse_function_return_clause(kind);
        let (effects, effect_spans, effects_recovered) = self.parse_function_effects();
        let callsite = self.parse_callsite_modifier(kind);
        FunctionReturn {
            binding: return_type.binding,
            ty: return_type.ty,
            ty_span: return_type.span,
            ty_paths: return_type.paths,
            ty_refinements: return_type.refinements,
            effects,
            effect_spans,
            effects_recovered,
            callsite,
        }
    }

    fn parse_function_return_clause(&mut self, kind: FunctionKind) -> FunctionReturnType {
        let return_context = Self::return_context(kind);
        if self.eat(TokenKind::Arrow).is_none() {
            return FunctionReturnType {
                binding: None,
                ty: None,
                span: None,
                paths: Vec::new(),
                refinements: Vec::new(),
            };
        }

        let binding = if matches!(
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
        let type_start = self.current().range;
        let (ty, ty_paths, ty_refinements) = self.collect_return_type_until(
            return_context,
            &[
                TokenKind::Effects,
                TokenKind::Callsite,
                TokenKind::Newline,
                TokenKind::Eof,
            ],
        );
        let type_end = self.previous().map_or(type_start, |token| token.range);
        let type_span = self.source.span(type_start.cover(type_end));
        FunctionReturnType {
            binding,
            ty: Some(ty),
            span: Some(type_span),
            paths: ty_paths,
            refinements: ty_refinements,
        }
    }

    fn parse_function_effects(&mut self) -> (Option<Vec<String>>, Option<Vec<SourceSpan>>, bool) {
        if self.eat(TokenKind::Effects).is_none() {
            return (None, None, false);
        }

        let diagnostic_count = self.diagnostics.len();
        let labels = self.parse_effect_list();
        let (effects, spans): (Vec<_>, Vec<_>) = labels.into_iter().unzip();
        (
            Some(effects),
            Some(spans),
            self.diagnostics.len() > diagnostic_count,
        )
    }

    fn parse_callsite_modifier(&mut self, kind: FunctionKind) -> Option<SourceSpan> {
        if kind != FunctionKind::Function || !self.at(TokenKind::Callsite) {
            return None;
        }

        let modifier = self.bump();
        let span = self.source.span(modifier.range);
        while self.at(TokenKind::Callsite) {
            let duplicate = self.bump();
            let duplicate_span = self.source.span(duplicate.range);
            self.diagnostics
                .push(Self::duplicate_callsite_modifier_diagnostic(
                    duplicate,
                    duplicate_span,
                ));
        }
        Some(span)
    }

    fn duplicate_callsite_modifier_diagnostic(
        duplicate: Token,
        duplicate_span: SourceSpan,
    ) -> ParseDiagnostic {
        ParseDiagnostic {
            id: "parse.duplicate_callsite_modifier",
            message: "duplicate `callsite` function modifier".to_string(),
            span: Some(duplicate_span.clone()),
            parser_context: Self::function_context(FunctionKind::Function),
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
            &text,
            true,
        )
        .parse();
        self.diagnostics.extend(predicate_output.diagnostics);
        let callsite_reference_spans = predicate_tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.kind == TokenKind::Callsite
                    && !matches!(
                        index
                            .checked_sub(1)
                            .and_then(|previous| predicate_tokens.get(previous))
                            .map(|token| token.kind),
                        Some(TokenKind::Dot | TokenKind::DoubleColon)
                    )
                    && !matches!(
                        predicate_tokens.get(index + 1).map(|token| token.kind),
                        Some(TokenKind::DoubleColon)
                    )
            })
            .map(|(_, token)| self.source.span(token.range))
            .collect();
        let call_callee_spans = predicate_tokens
            .iter()
            .enumerate()
            .filter_map(|(open_index, token)| {
                if token.kind != TokenKind::LParen {
                    return None;
                }
                let end_index = open_index.checked_sub(1)?;
                if !matches!(
                    predicate_tokens[end_index].kind,
                    TokenKind::Ident | TokenKind::Callsite
                ) {
                    return None;
                }
                let mut start_index = end_index;
                while start_index >= 2
                    && predicate_tokens[start_index - 1].kind == TokenKind::DoubleColon
                    && matches!(
                        predicate_tokens[start_index - 2].kind,
                        TokenKind::Ident | TokenKind::Callsite
                    )
                {
                    start_index -= 2;
                }
                let callee = predicate_tokens[start_index..=end_index]
                    .iter()
                    .map(|token| token.text.as_str())
                    .collect::<String>();
                let range = predicate_tokens[start_index]
                    .range
                    .cover(predicate_tokens[end_index].range);
                Some((callee, self.source.span(range)))
            })
            .collect();
        ContractClause {
            kind,
            text,
            predicate_tokens: predicate_output.predicate_tokens,
            remainder_spans: predicate_output.remainder_spans,
            perform_effect_spans: predicate_output.perform_effect_spans,
            callsite_reference_spans,
            call_callee_spans,
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
            let (annotation, annotation_paths, annotation_refinements) =
                if self.eat(TokenKind::Colon).is_some() {
                    let (annotation, paths, refinements) = self.collect_type_paths_until(
                        "let_statement",
                        &[TokenKind::Equal, TokenKind::Newline, TokenKind::Eof],
                    );
                    (Some(annotation), paths, refinements)
                } else {
                    (None, Vec::new(), Vec::new())
                };
            self.expect(TokenKind::Equal, "let_statement", vec!["="]);
            let (expr, end) = self.parse_expr_for_body_line("let_statement");
            BodyLine::Let {
                pattern,
                annotation,
                annotation_paths: annotation_paths.into_boxed_slice(),
                annotation_refinements: annotation_refinements.into_boxed_slice(),
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
