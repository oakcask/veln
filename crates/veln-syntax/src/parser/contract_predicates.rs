use super::*;

impl<'a> ContractPredicateParser<'a> {
    pub(super) fn new(
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

    pub(super) fn parse(mut self) -> Vec<ParseDiagnostic> {
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

    pub(super) fn parse_predicate(&mut self, min_bp: u8) {
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
            TokenKind::String | TokenKind::Int | TokenKind::Float | TokenKind::Ident => {
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

    pub(super) fn parse_perform_contract_primary(&mut self) {
        self.bump();
        self.parse_name_path_or_literal();
        if self.at(TokenKind::LParen) {
            self.parse_call_args();
        }
    }

    pub(super) fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
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

    pub(super) fn eat(&mut self, kind: TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.bump())
        } else {
            None
        }
    }

    pub(super) fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    pub(super) fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    pub(super) fn bump(&mut self) -> Token {
        let token = self.current().clone();
        self.cursor += 1;
        token
    }
}
