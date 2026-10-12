use super::*;

impl<'a> ContractPredicateParser<'a> {
    pub(super) fn new(
        source: &'a SourceFile,
        context: &'static str,
        diagnostic_id: &'static str,
        tokens: &'a [Token],
        allow_remainder: bool,
    ) -> Self {
        Self {
            source,
            context,
            diagnostic_id,
            tokens,
            allow_remainder,
            cursor: 0,
            diagnostics: Vec::new(),
            perform_effect_spans: Vec::new(),
            remainder_spans: Vec::new(),
            completed_remainder_ranges: HashSet::new(),
        }
    }

    pub(super) fn parse(mut self) -> ContractPredicateOutput {
        if self.tokens.is_empty() {
            self.error_current(
                "contract predicate is empty",
                vec!["contract predicate"],
                RecoveryStrategy::InsertToken,
                None,
            );
            return ContractPredicateOutput {
                diagnostics: self.diagnostics,
                perform_effect_spans: self.perform_effect_spans,
                remainder_spans: self.remainder_spans,
            };
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
        self.remainder_spans
            .sort_by_key(|remainder| remainder.operator_span.start.offset);
        ContractPredicateOutput {
            diagnostics: self.diagnostics,
            perform_effect_spans: self.perform_effect_spans,
            remainder_spans: self.remainder_spans,
        }
    }

    pub(super) fn parse_predicate(&mut self, min_bp: u8) {
        let expression_start = self.cursor;
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

            let Some((op, left_bp, right_bp)) = self.current_binary_op() else {
                break;
            };
            if left_bp < min_bp {
                break;
            }
            let operator_index = self.cursor;
            self.bump();
            let right_start = self.cursor;
            self.parse_predicate(right_bp);
            if op == BinaryOp::Remainder {
                self.record_remainder(expression_start, operator_index, right_start, self.cursor);
            }
        }
    }

    fn record_remainder(
        &mut self,
        expression_start: usize,
        operator_index: usize,
        right_start: usize,
        expression_end: usize,
    ) {
        let Some(left_first) = self.tokens.get(expression_start) else {
            return;
        };
        let Some(left_last) = operator_index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
        else {
            return;
        };
        let Some(right_first) = self.tokens.get(right_start) else {
            return;
        };
        let Some(right_last) = expression_end
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
        else {
            return;
        };
        let Some(operator) = self.tokens.get(operator_index) else {
            return;
        };
        let normalized = |tokens: &[Token]| {
            normalize_collected_text(tokens.iter().map(|token| token.text.clone()).collect())
        };
        let left_span = self.source.span(left_first.range.cover(left_last.range));
        let operation_span = self.source.span(left_first.range.cover(right_last.range));
        let left_is_prior_remainder = self
            .completed_remainder_ranges
            .contains(&(left_span.start.offset, left_span.end.offset));
        self.remainder_spans.push(PredicateRemainder {
            left_text: if left_is_prior_remainder {
                String::new()
            } else {
                normalized(&self.tokens[expression_start..operator_index])
            },
            left_is_prior_remainder,
            right_text: normalized(&self.tokens[right_start..expression_end]),
            operator_span: self.source.span(operator.range),
            left_span,
            right_span: self.source.span(right_first.range.cover(right_last.range)),
            operation_span,
        });
        self.completed_remainder_ranges
            .insert((left_first.range.start, right_last.range.end));
    }

    pub(super) fn parse_prefix(&mut self) {
        if self.at(TokenKind::Not) || self.at(TokenKind::Minus) || self.at(TokenKind::Tilde) {
            self.bump();
            self.parse_predicate(25);
            return;
        }
        self.parse_postfix();
    }

    pub(super) fn parse_postfix(&mut self) {
        self.parse_primary();
        loop {
            if self.at(TokenKind::LParen) {
                self.parse_call_args();
                continue;
            }
            if self.at(TokenKind::Dot) {
                self.bump();
                if matches!(self.current().kind, TokenKind::Ident | TokenKind::Callsite) {
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

    pub(super) fn parse_call_args(&mut self) {
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

    pub(super) fn parse_primary(&mut self) {
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
            TokenKind::String
            | TokenKind::Int
            | TokenKind::Float
            | TokenKind::Ident
            | TokenKind::Callsite => {
                self.parse_name_path_or_literal();
            }
            TokenKind::Perform => {
                self.parse_perform_contract_primary();
            }
            TokenKind::MalformedInt => {
                self.bump();
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

    pub(super) fn parse_name_path_or_literal(&mut self) {
        self.bump();
        while self.at(TokenKind::DoubleColon) {
            self.bump();
            if matches!(self.current().kind, TokenKind::Ident | TokenKind::Callsite) {
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

    pub(super) fn parse_perform_contract_primary(&mut self) {
        if let Some(span) = self.complete_perform_effect_span() {
            self.perform_effect_spans.push(span);
        }
        self.bump();
        self.parse_name_path_or_literal();
        if self.at(TokenKind::LParen) {
            self.parse_call_args();
        }
    }

    fn complete_perform_effect_span(&self) -> Option<SourceSpan> {
        let effect = self.tokens.get(self.cursor + 1)?;
        let separator = self.tokens.get(self.cursor + 2)?;
        let operation = self.tokens.get(self.cursor + 3)?;
        let open = self.tokens.get(self.cursor + 4)?;
        if !matches!(effect.kind, TokenKind::Ident | TokenKind::Callsite)
            || separator.kind != TokenKind::DoubleColon
            || !matches!(operation.kind, TokenKind::Ident | TokenKind::Callsite)
            || open.kind != TokenKind::LParen
        {
            return None;
        }

        let mut depth = 0usize;
        for token in &self.tokens[self.cursor + 4..] {
            match token.kind {
                TokenKind::LParen => depth += 1,
                TokenKind::RParen => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(self.source.span(effect.range));
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub(super) fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        let kind = self.tokens.get(self.cursor)?.kind;
        if kind == TokenKind::Percent && !self.allow_remainder {
            return None;
        }
        binary_operator(kind, false)
    }

    pub(super) fn error_current(
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
}
