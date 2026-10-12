use super::*;

impl<'a> ContractPredicateParser<'a> {
    const MAX_PAREN_NESTING: usize = 128;

    pub(super) fn new(
        source: &'a SourceFile,
        context: &'static str,
        diagnostic_id: &'static str,
        tokens: &'a [Token],
        predicate: &'a str,
        allow_remainder: bool,
    ) -> Self {
        let token_text_ranges = predicate_token_text_ranges(predicate, tokens);
        Self {
            source,
            context,
            diagnostic_id,
            tokens,
            token_text_ranges,
            allow_remainder,
            cursor: 0,
            diagnostics: Vec::new(),
            perform_effect_spans: Vec::new(),
            remainder_spans: Vec::new(),
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
            let predicate_tokens = self.predicate_tokens();
            return ContractPredicateOutput {
                diagnostics: self.diagnostics,
                perform_effect_spans: self.perform_effect_spans,
                remainder_spans: self.remainder_spans,
                predicate_tokens,
            };
        }

        if let Some(index) = excessive_parenthesis_nesting(self.tokens, Self::MAX_PAREN_NESTING) {
            self.cursor = index;
            self.error_current(
                "contract predicate nesting exceeds the supported limit of 128 parenthesized expressions",
                vec!["shallower contract predicate"],
                RecoveryStrategy::SkipToken,
                None,
            );
            let predicate_tokens = self.predicate_tokens();
            return ContractPredicateOutput {
                diagnostics: self.diagnostics,
                perform_effect_spans: self.perform_effect_spans,
                remainder_spans: self.remainder_spans,
                predicate_tokens,
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
        let operations = self
            .remainder_spans
            .iter()
            .enumerate()
            .map(|(index, remainder)| (remainder.operation_range.clone(), index))
            .collect::<HashMap<_, _>>();
        for remainder in &mut self.remainder_spans {
            remainder.left_remainder = operations.get(&remainder.left_range).copied();
        }
        let predicate_tokens = self.predicate_tokens();
        ContractPredicateOutput {
            diagnostics: self.diagnostics,
            perform_effect_spans: self.perform_effect_spans,
            remainder_spans: self.remainder_spans,
            predicate_tokens,
        }
    }

    fn predicate_tokens(&self) -> Vec<crate::PredicateTokenSpan> {
        self.tokens
            .iter()
            .zip(&self.token_text_ranges)
            .map(|(token, text_range)| crate::PredicateTokenSpan {
                text_range: text_range.clone(),
                source_span: self.source.span(token.range),
            })
            .collect()
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
        let left_span = self.source.span(left_first.range.cover(left_last.range));
        let operation_span = self.source.span(left_first.range.cover(right_last.range));
        let Some(left_range) =
            token_slice_text_range(&self.token_text_ranges, expression_start, operator_index)
        else {
            return;
        };
        let Some(right_range) =
            token_slice_text_range(&self.token_text_ranges, right_start, expression_end)
        else {
            return;
        };
        let Some(operation_range) =
            token_slice_text_range(&self.token_text_ranges, expression_start, expression_end)
        else {
            return;
        };
        self.remainder_spans.push(PredicateRemainder {
            left_range,
            right_range,
            operation_range,
            left_remainder: None,
            operator_span: self.source.span(operator.range),
            left_span,
            right_span: self.source.span(right_first.range.cover(right_last.range)),
            operation_span,
        });
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
                self.parse_grouped_primary();
            }
            TokenKind::Hole | TokenKind::Underscore => {
                self.reject_primary("hole syntax is not allowed in a contract predicate");
            }
            TokenKind::LBracket => {
                self.reject_primary("list syntax is not allowed in a contract predicate");
            }
            TokenKind::LBrace => {
                self.reject_primary("record syntax is not allowed in a contract predicate");
            }
            TokenKind::Match => {
                self.reject_primary("`match` is not allowed in a contract predicate");
            }
            TokenKind::PipeGreater => {
                self.reject_primary("pipeline syntax is not allowed in a contract predicate");
            }
            TokenKind::Invalid => {
                self.reject_primary("invalid token in contract predicate");
            }
            _ => {
                self.reject_primary("expected a contract predicate atom");
            }
        }
    }

    fn parse_grouped_primary(&mut self) {
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

    fn reject_primary(&mut self, message: &'static str) {
        self.error_current(
            message,
            vec!["contract predicate atom"],
            RecoveryStrategy::SkipToken,
            None,
        );
        self.bump();
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

fn token_slice_text_range(
    ranges: &[std::ops::Range<usize>],
    start: usize,
    end: usize,
) -> Option<std::ops::Range<usize>> {
    Some(ranges.get(start)?.start..ranges.get(end.checked_sub(1)?)?.end)
}

fn predicate_token_text_ranges(predicate: &str, tokens: &[Token]) -> Vec<std::ops::Range<usize>> {
    let mut cursor = 0;
    tokens
        .iter()
        .map(|token| {
            let relative = predicate[cursor..]
                .find(&token.text)
                .expect("normalized predicate must retain every source token");
            let start = cursor + relative;
            let end = start + token.text.len();
            cursor = end;
            start..end
        })
        .collect()
}

fn excessive_parenthesis_nesting(tokens: &[Token], limit: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::LParen => {
                depth += 1;
                if depth > limit {
                    return Some(index);
                }
            }
            TokenKind::RParen => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    None
}
