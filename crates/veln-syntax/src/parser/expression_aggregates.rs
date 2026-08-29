use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_list(&mut self) -> Expr {
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

    pub(super) fn parse_record(&mut self) -> Expr {
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

    pub(super) fn parse_dict(&mut self) -> Expr {
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

    pub(super) fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        match self.tokens.get(self.cursor)?.kind {
            TokenKind::PipeGreater => Some((BinaryOp::PipeGreater, 1, 2)),
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

    pub(super) fn missing_expr(&self) -> Expr {
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(TextRange::at(self.source.len())),
        }
    }

    pub(super) fn missing_expr_at_current(&self) -> Expr {
        let range = self
            .tokens
            .get(self.cursor)
            .map_or_else(|| TextRange::at(self.source.len()), |token| token.range);
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(range),
        }
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

    pub(super) fn at_contextual_identifier(&self) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| is_contextual_identifier(token.kind))
    }

    pub(super) fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind)
    }

    pub(super) fn at_ident_text(&self, text: &str) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == TokenKind::Ident && token.text == text)
    }

    pub(super) fn error_current(
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

    pub(super) fn error_at_token(&mut self, token: &Token, request: DiagnosticRequest) {
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

    pub(super) fn expect_expr_token(
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

    pub(super) fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    pub(super) fn previous(&self) -> Option<&Token> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    pub(super) fn is_at_end(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    pub(super) fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    pub(super) fn bump(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        self.cursor += 1;
        token
    }
}
