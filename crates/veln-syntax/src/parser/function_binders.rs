use super::*;

#[derive(Default)]
struct FunctionBinderState {
    type_binders: Vec<TypeBinder>,
    effect_binder: Option<EffectBinder>,
    names: std::collections::HashSet<String>,
    saw_effect: bool,
}

impl FunctionBinderState {
    fn finish(self) -> (Vec<TypeBinder>, Option<EffectBinder>) {
        (self.type_binders, self.effect_binder)
    }
}

enum FunctionBinderBoundary {
    Continue,
    Closed,
    Unclosed,
}

impl<'a> Parser<'a> {
    pub(super) fn report_generic_function_callsite(
        &mut self,
        kind: FunctionKind,
        type_binders: &[TypeBinder],
        callsite: &Option<SourceSpan>,
    ) {
        if type_binders.is_empty() {
            return;
        }
        let Some(callsite) = callsite else {
            return;
        };
        self.diagnostics.push(ParseDiagnostic {
            id: "parse.generic_function_callsite",
            message: "ordinary generic functions cannot use the `callsite` modifier".to_string(),
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

    pub(super) fn report_generic_function_variadic(
        &mut self,
        kind: FunctionKind,
        type_binders: &[TypeBinder],
        params: &[Param],
    ) {
        if type_binders.is_empty() {
            return;
        }
        let Some(param) = params.iter().find(|param| param.is_variadic) else {
            return;
        };
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

    pub(super) fn parse_function_binders(
        &mut self,
        kind: FunctionKind,
    ) -> (Vec<TypeBinder>, Option<EffectBinder>, bool) {
        if !self.at(TokenKind::Less) {
            return (Vec::new(), None, false);
        }
        let start = self.bump().range;
        let mut state = FunctionBinderState::default();

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
            let (type_binders, effect_binder) = state.finish();
            return (type_binders, effect_binder, false);
        }

        let mut closed = false;
        loop {
            if !self.parse_function_binder_entry(kind, &mut state) {
                if self.at(TokenKind::Comma) {
                    self.bump();
                    continue;
                }
                if self.at(TokenKind::Greater) {
                    self.close_function_binders(start, &mut state);
                    closed = true;
                    break;
                }
                break;
            }
            match self.finish_function_binder_entry(kind, start, &mut state) {
                FunctionBinderBoundary::Continue => {}
                FunctionBinderBoundary::Closed => {
                    closed = true;
                    break;
                }
                FunctionBinderBoundary::Unclosed => break,
            }
        }

        self.report_generic_test_declaration(kind, &state.type_binders);

        let abandoned_at_binder_boundary =
            !closed && (self.at(TokenKind::Newline) || self.at(TokenKind::Eof));
        let (type_binders, effect_binder) = state.finish();
        (type_binders, effect_binder, abandoned_at_binder_boundary)
    }

    fn parse_function_binder_entry(
        &mut self,
        kind: FunctionKind,
        state: &mut FunctionBinderState,
    ) -> bool {
        if self.at(TokenKind::Effect) {
            self.parse_function_effect_binder(kind, state);
        } else if self.at(TokenKind::Ident) {
            self.parse_function_type_binder(kind, state);
        } else {
            if self.at_function_binder_synchronization_anchor() {
                let invalid = self.current().clone();
                self.report_function_binder_error(
                    &invalid,
                    "parse.function_binder_entry",
                    "function binder entry must be a type name or `effect E`",
                    vec!["type binder", "effect"],
                    kind,
                );
                return false;
            }
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
        true
    }

    fn at_function_binder_synchronization_anchor(&self) -> bool {
        self.at(TokenKind::Comma)
            || self.at(TokenKind::Greater)
            || self.at(TokenKind::LParen)
            || self.at(TokenKind::Newline)
            || self.at(TokenKind::Eof)
    }

    fn parse_function_effect_binder(
        &mut self,
        kind: FunctionKind,
        state: &mut FunctionBinderState,
    ) {
        let effect = self.bump();
        if state.saw_effect {
            self.report_function_binder_error(
                &effect,
                "parse.multiple_effect_binders",
                "function binder list accepts at most one effect binder",
                vec![">"],
                kind,
            );
        }
        state.saw_effect = true;
        let name_token = self.current().clone();
        let Some(name) = self.expect_ident(Self::function_context(kind), "effect row variable")
        else {
            return;
        };
        if !state.names.insert(name.clone()) {
            self.report_duplicate_function_binder(&name_token, &name, kind);
        }
        if state.effect_binder.is_none() {
            state.effect_binder = Some(EffectBinder {
                name,
                span: self.source.span(effect.range.cover(name_token.range)),
            });
        }
    }

    fn parse_function_type_binder(&mut self, kind: FunctionKind, state: &mut FunctionBinderState) {
        let binder = self.bump();
        if state.saw_effect {
            self.report_function_binder_error(
                &binder,
                "parse.type_binder_after_effect",
                "ordinary type binders must precede the effect binder",
                vec![">"],
                kind,
            );
        }
        if !state.names.insert(binder.text.clone()) {
            self.report_duplicate_function_binder(&binder, &binder.text, kind);
        }
        state.type_binders.push(TypeBinder {
            name: binder.text,
            span: self.source.span(binder.range),
        });
        if self.at(TokenKind::Colon) || self.at(TokenKind::Less) {
            self.reject_function_binder_bound(kind);
        }
    }

    fn reject_function_binder_bound(&mut self, kind: FunctionKind) {
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

    fn finish_function_binder_entry(
        &mut self,
        kind: FunctionKind,
        start: TextRange,
        state: &mut FunctionBinderState,
    ) -> FunctionBinderBoundary {
        if self.eat(TokenKind::Comma).is_some() {
            if self.at(TokenKind::Greater) {
                self.close_function_binders(start, state);
                return FunctionBinderBoundary::Closed;
            }
            return FunctionBinderBoundary::Continue;
        }
        if self.at(TokenKind::Greater) {
            self.close_function_binders(start, state);
            return FunctionBinderBoundary::Closed;
        }

        self.recover_function_binder_separator(kind);
        if self.at(TokenKind::Comma) {
            self.bump();
            return FunctionBinderBoundary::Continue;
        }
        if self.at(TokenKind::Greater) {
            self.close_function_binders(start, state);
            return FunctionBinderBoundary::Closed;
        }
        FunctionBinderBoundary::Unclosed
    }

    fn close_function_binders(&mut self, start: TextRange, state: &mut FunctionBinderState) {
        let end = self.bump().range;
        if let Some(binder) = state.effect_binder.as_mut() {
            binder.span = self.source.span(start.cover(end));
        }
    }

    fn recover_function_binder_separator(&mut self, kind: FunctionKind) {
        self.error_current(
            "parse.function_binder_separator",
            "function binders must be separated by `,` and closed by `>`",
            Self::function_context(kind),
            vec![",", ">"],
            RecoveryStrategy::SynchronizeToAnchor,
            Some("function binder list"),
        );
        while !self.at_function_binder_synchronization_anchor() {
            self.bump();
        }
    }

    fn report_generic_test_declaration(&mut self, kind: FunctionKind, binders: &[TypeBinder]) {
        if kind != FunctionKind::Test || binders.is_empty() {
            return;
        }
        let binder = &binders[0];
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

    fn skip_invalid_function_binder_entry(&mut self) {
        while !self.at_function_binder_synchronization_anchor() {
            self.bump();
        }
    }

    fn skip_applied_function_binder(&mut self) {
        let has_balanced_close = self.applied_function_binder_has_balanced_close();
        let mut depth = 1usize;
        let mut parenthesis_depth = 0usize;
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            if self.at(TokenKind::LParen) {
                if !has_balanced_close
                    && parenthesis_depth == 0
                    && self.at_function_parameter_list_boundary()
                {
                    break;
                }
                parenthesis_depth += 1;
            } else if self.at(TokenKind::RParen) && parenthesis_depth > 0 {
                parenthesis_depth -= 1;
            } else if parenthesis_depth == 0 && self.at(TokenKind::Less) {
                depth += 1;
            } else if parenthesis_depth == 0 && self.at(TokenKind::Greater) {
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

    fn applied_function_binder_has_balanced_close(&self) -> bool {
        let mut angle_depth = 1usize;
        let mut parenthesis_depth = 0usize;
        for token in &self.tokens[self.cursor..] {
            match token.kind {
                TokenKind::LParen => parenthesis_depth += 1,
                TokenKind::RParen if parenthesis_depth > 0 => parenthesis_depth -= 1,
                TokenKind::Less if parenthesis_depth == 0 => angle_depth += 1,
                TokenKind::Greater if parenthesis_depth == 0 => {
                    angle_depth -= 1;
                    if angle_depth == 0 {
                        return true;
                    }
                }
                TokenKind::Newline | TokenKind::Eof => return false,
                _ => {}
            }
        }
        false
    }

    fn at_function_parameter_list_boundary(&self) -> bool {
        if !self.at(TokenKind::LParen)
            || self
                .previous()
                .is_some_and(|token| token.kind == TokenKind::Fn)
        {
            return false;
        }

        let mut depth = 0usize;
        for (offset, token) in self.tokens[self.cursor..].iter().enumerate() {
            match token.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        let following = self.tokens.get(self.cursor + offset + 1);
                        return following.is_some_and(|token| {
                            matches!(
                                token.kind,
                                TokenKind::Arrow
                                    | TokenKind::Effects
                                    | TokenKind::Callsite
                                    | TokenKind::Newline
                                    | TokenKind::Eof
                            )
                        });
                    }
                }
                TokenKind::Newline | TokenKind::Eof => return false,
                _ => {}
            }
        }
        false
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
}
