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

#[derive(Clone, Copy)]
enum RejectedFunctionBinderBound {
    Applied,
    Colon,
}

struct FunctionBinderRecoveryPlan {
    stop: usize,
    owned_closers: usize,
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

        if self.at_function_binder_closer() {
            self.error_current(
                "parse.empty_function_binder_list",
                "function binder list cannot be empty",
                Self::function_context(kind),
                vec!["type binder", "effect"],
                RecoveryStrategy::None,
                Some(">"),
            );
            self.consume_function_binder_closers(1);
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
                if self.at_function_binder_closer() {
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
            || self.at_function_binder_closer()
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
        let bound = if invalid.kind == TokenKind::Less {
            RejectedFunctionBinderBound::Applied
        } else {
            RejectedFunctionBinderBound::Colon
        };
        self.report_function_binder_error(
            &invalid,
            "parse.function_binder_bound",
            "function type binders do not accept bounds or type arguments",
            vec![",", ">"],
            kind,
        );
        self.skip_function_binder_bound(bound);
    }

    fn finish_function_binder_entry(
        &mut self,
        kind: FunctionKind,
        start: TextRange,
        state: &mut FunctionBinderState,
    ) -> FunctionBinderBoundary {
        if self.eat(TokenKind::Comma).is_some() {
            if self.at_function_binder_closer() {
                self.close_function_binders(start, state);
                return FunctionBinderBoundary::Closed;
            }
            return FunctionBinderBoundary::Continue;
        }
        if self.at_function_binder_closer() {
            self.close_function_binders(start, state);
            return FunctionBinderBoundary::Closed;
        }

        self.recover_function_binder_separator(kind);
        if self.at(TokenKind::Comma) {
            self.bump();
            return FunctionBinderBoundary::Continue;
        }
        if self.at_function_binder_closer() {
            self.close_function_binders(start, state);
            return FunctionBinderBoundary::Closed;
        }
        FunctionBinderBoundary::Unclosed
    }

    fn close_function_binders(&mut self, start: TextRange, state: &mut FunctionBinderState) {
        let end = self.consume_function_binder_closers(1);
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

    fn skip_function_binder_bound(&mut self, bound: RejectedFunctionBinderBound) {
        self.apply_function_binder_recovery_plan(self.function_binder_recovery_plan(bound));
    }

    fn at_function_binder_closer(&self) -> bool {
        closing_angle_count(self.current().kind) > 0
    }

    fn consume_function_binder_closers(&mut self, owned: usize) -> TextRange {
        let token = self.current().clone();
        let surplus = closing_angle_count(token.kind) - owned;
        let consumed = TextRange::new(token.range.start, token.range.start + owned);
        if surplus == 0 {
            self.bump();
            return consumed;
        }

        self.tokens[self.cursor] = Token {
            kind: match surplus {
                1 => TokenKind::Greater,
                2 => TokenKind::ShiftRight,
                _ => TokenKind::ShiftRightLogical,
            },
            text: ">".repeat(surplus),
            range: TextRange::new(token.range.start + owned, token.range.end),
        };
        consumed
    }

    fn function_binder_recovery_plan(
        &self,
        bound: RejectedFunctionBinderBound,
    ) -> FunctionBinderRecoveryPlan {
        let mut angle_depth = match bound {
            RejectedFunctionBinderBound::Applied => 1,
            RejectedFunctionBinderBound::Colon => 0,
        };
        let mut parenthesis_depth = 0usize;
        let mut parameter_boundary = None;

        for cursor in self.cursor..self.tokens.len() {
            let kind = self.tokens[cursor].kind;
            if matches!(kind, TokenKind::Newline | TokenKind::Eof) {
                return FunctionBinderRecoveryPlan {
                    stop: parameter_boundary.unwrap_or(cursor),
                    owned_closers: 0,
                };
            }

            if parenthesis_depth == 0 {
                if matches!(bound, RejectedFunctionBinderBound::Colon)
                    && angle_depth == 0
                    && kind == TokenKind::Comma
                {
                    return FunctionBinderRecoveryPlan {
                        stop: parameter_boundary.unwrap_or(cursor),
                        owned_closers: 0,
                    };
                }

                let close_count = closing_angle_count(kind);
                let closes_bound = match bound {
                    RejectedFunctionBinderBound::Applied => close_count >= angle_depth,
                    RejectedFunctionBinderBound::Colon => close_count > angle_depth,
                };
                if close_count > 0 && closes_bound {
                    return FunctionBinderRecoveryPlan {
                        stop: cursor,
                        owned_closers: angle_depth,
                    };
                }

                if kind == TokenKind::LParen {
                    if parameter_boundary.is_none()
                        && (matches!(bound, RejectedFunctionBinderBound::Applied)
                            || angle_depth == 0)
                        && self.function_parameter_list_boundary_at(cursor)
                    {
                        parameter_boundary = Some(cursor);
                    }
                    parenthesis_depth += 1;
                } else if kind == TokenKind::Less {
                    angle_depth += 1;
                } else if close_count > 0 {
                    angle_depth -= close_count;
                }
            } else if kind == TokenKind::LParen {
                parenthesis_depth += 1;
            } else if kind == TokenKind::RParen {
                parenthesis_depth -= 1;
            }
        }

        FunctionBinderRecoveryPlan {
            stop: parameter_boundary.unwrap_or(self.tokens.len()),
            owned_closers: 0,
        }
    }

    fn apply_function_binder_recovery_plan(&mut self, plan: FunctionBinderRecoveryPlan) {
        while self.cursor < plan.stop {
            self.bump();
        }
        if plan.owned_closers > 0 {
            self.consume_function_binder_closers(plan.owned_closers);
        }
    }

    fn function_parameter_list_boundary_at(&self, cursor: usize) -> bool {
        if self.tokens[cursor].kind != TokenKind::LParen
            || cursor
                .checked_sub(1)
                .and_then(|previous| self.tokens.get(previous))
                .is_some_and(|token| token.kind == TokenKind::Fn)
        {
            return false;
        }

        let mut depth = 0usize;
        for (offset, token) in self.tokens[cursor..].iter().enumerate() {
            match token.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        let following = self.tokens.get(cursor + offset + 1);
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
