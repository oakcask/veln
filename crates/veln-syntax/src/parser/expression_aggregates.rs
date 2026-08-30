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
        let (fields, end) = self.parse_braced_items(
            start,
            |this| {
                let field_start = this.current().range;
                let name = if this.at(TokenKind::Ident) {
                    this.bump().text
                } else {
                    this.bump();
                    String::new()
                };
                this.eat(TokenKind::Colon);
                let expr = this.parse_expr(0);
                let field_span = this.source.span(field_start.cover(lhs_range(&expr)));
                RecordField {
                    name,
                    expr,
                    span: field_span,
                }
            },
            |field| TextRange::new(field.span.start.offset, field.span.end.offset),
        );
        Expr {
            kind: ExprKind::Record(fields),
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_dict(&mut self) -> Expr {
        let start = self.bump().range;
        let (entries, end) = self.parse_braced_items(
            start,
            |this| {
                let entry_start = this.current().range;
                let key = this.parse_expr(0);
                this.eat(TokenKind::Colon);
                let value = this.parse_expr(0);
                let entry_span = this.source.span(entry_start.cover(lhs_range(&value)));
                DictEntry {
                    key,
                    value,
                    span: entry_span,
                }
            },
            |entry| TextRange::new(entry.span.start.offset, entry.span.end.offset),
        );
        Expr {
            kind: ExprKind::Dict(entries),
            span: self.source.span(start.cover(end)),
        }
    }

    pub(super) fn parse_braced_items<T>(
        &mut self,
        start: TextRange,
        mut parse_item: impl FnMut(&mut Self) -> T,
        item_range: impl Fn(&T) -> TextRange,
    ) -> (Vec<T>, TextRange) {
        let mut items = Vec::new();
        while !self.at(TokenKind::RBrace) && !self.is_at_end() {
            items.push(parse_item(self));
            if self.eat(TokenKind::Comma).is_none() {
                break;
            }
        }
        let end = self.eat(TokenKind::RBrace).map_or_else(
            || items.last().map_or(start, item_range),
            |token| token.range,
        );
        (items, end)
    }

    pub(super) fn current_binary_op(&self) -> Option<(BinaryOp, u8, u8)> {
        binary_operator(self.tokens.get(self.cursor)?.kind, true)
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

    pub(super) fn at_contextual_identifier(&self) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| is_contextual_identifier(token.kind))
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
}
