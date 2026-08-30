use super::*;

impl<'a> ExprParser<'a> {
    pub(super) fn parse_primary(&mut self) -> Expr {
        let Some(token) = self.tokens.get(self.cursor).cloned() else {
            return self.missing_expr();
        };

        match token.kind {
            TokenKind::Underscore | TokenKind::Hole => self.parse_hole_primary(token),
            TokenKind::String => self.parse_literal_primary(token, ExprKind::StringLiteral),
            TokenKind::Int => self.parse_literal_primary(token, ExprKind::IntLiteral),
            TokenKind::Float => self.parse_literal_primary(token, ExprKind::FloatLiteral),
            TokenKind::Ident | TokenKind::Handler | TokenKind::Handles => self.parse_name_path(),
            TokenKind::Perform => self.parse_perform_primary(token),
            TokenKind::Handle => self.parse_handle_primary(token),
            TokenKind::Decode => self.parse_schema_decode_primary(token),
            TokenKind::Encode => self.parse_schema_encode_primary(token),
            TokenKind::LParen => self.parse_group_or_unit_primary(),
            TokenKind::LBrace => self.parse_record_or_dict_primary(),
            TokenKind::LBracket => self.parse_list(),
            TokenKind::Match => self.parse_match(),
            TokenKind::If => self.parse_if(),
            _ => self.parse_missing_primary(token),
        }
    }

    pub(super) fn parse_perform_primary(&mut self, token: Token) -> Expr {
        let start = token.range;
        self.bump();
        let effect_start = self.current().range;
        let mut path = self.parse_name_path_segments("perform_expression", "effect operation path");
        if path.len() < 2 {
            self.error_current(
                "parse.perform_expression",
                "perform expression requires `Effect::operation`",
                vec!["effect operation path"],
                RecoveryStrategy::InsertToken,
                Some("("),
            );
        }
        let operation = path.pop().unwrap_or_default();
        let effect_end = if path.is_empty() {
            effect_start
        } else {
            self.tokens
                .get(self.cursor.saturating_sub(3))
                .map_or(effect_start, |token| token.range)
        };
        let effect_span = self.source.span(effect_start.cover(effect_end));
        let operation_span = self
            .previous()
            .map(|token| self.source.span(token.range))
            .unwrap_or_else(|| self.source.span(start));
        self.expect_expr_token(
            TokenKind::LParen,
            "parse.perform_expression",
            "perform expression is missing `(`",
            vec!["("],
        );
        let (args, end) = self.parse_parenthesized_arguments(
            start,
            "parse.perform_argument",
            "perform argument is missing `,` or `)`",
        );
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::Perform {
                effect: path,
                effect_span,
                operation,
                operation_span,
                args,
            },
        }
    }

    pub(super) fn parse_handle_primary(&mut self, token: Token) -> Expr {
        let start = token.range;
        self.bump();
        let body = self.parse_expr(0);
        self.expect_expr_token(
            TokenKind::With,
            "parse.handle_expression",
            "handle expression is missing `with`",
            vec!["with"],
        );
        let handler_start = self.current().range;
        let handler = self.parse_name_path_segments("handle_expression", "handler name");
        let handler_end = self.previous().map_or(handler_start, |token| token.range);
        self.expect_expr_token(
            TokenKind::LParen,
            "parse.handle_expression",
            "handle expression is missing handler context arguments",
            vec!["("],
        );
        let (args, end) = self.parse_parenthesized_arguments(
            handler_end,
            "parse.handle_argument",
            "handler context argument is missing `,` or `)`",
        );
        Expr {
            span: self.source.span(start.cover(end)),
            kind: ExprKind::Handle {
                body: Box::new(body),
                handler,
                handler_span: self.source.span(handler_start.cover(handler_end)),
                args,
            },
        }
    }

    fn parse_parenthesized_arguments(
        &mut self,
        fallback_end: TextRange,
        diagnostic_id: &'static str,
        missing_separator_message: &'static str,
    ) -> (Vec<Expr>, TextRange) {
        let mut arguments = Vec::new();
        while !self.at(TokenKind::RParen) && !self.is_at_end() {
            arguments.push(self.parse_expr(0));
            if self.eat(TokenKind::Comma).is_some() {
                continue;
            }
            if self.at(TokenKind::RParen) || self.is_at_end() {
                break;
            }
            self.error_current(
                diagnostic_id,
                missing_separator_message,
                vec![",", ")"],
                RecoveryStrategy::InsertToken,
                Some(","),
            );
        }
        let end = self.eat(TokenKind::RParen).map_or_else(
            || arguments.last().map_or(fallback_end, lhs_range),
            |token| token.range,
        );
        (arguments, end)
    }

    pub(super) fn parse_schema_decode_primary(&mut self, token: Token) -> Expr {
        let start = token.range;
        self.bump();
        let schema = self.parse_schema_operation_path("decode");
        self.expect_expr_token(
            TokenKind::From,
            "parse.schema_decode_expression",
            "schema decode expression is missing `from`",
            vec!["from"],
        );
        let input = self.parse_expr(0);
        self.expect_expr_token(
            TokenKind::At,
            "parse.schema_decode_expression",
            "schema decode expression is missing `at`",
            vec!["at"],
        );
        let base = self.parse_expr(0);
        Expr {
            span: self.source.span(start.cover(lhs_range(&base))),
            kind: ExprKind::SchemaDecode {
                schema,
                input: Box::new(input),
                base: Box::new(base),
            },
        }
    }

    pub(super) fn parse_schema_encode_primary(&mut self, token: Token) -> Expr {
        let start = token.range;
        self.bump();
        let schema = self.parse_schema_operation_path("encode");
        self.expect_expr_token(
            TokenKind::From,
            "parse.schema_encode_expression",
            "schema encode expression is missing `from`",
            vec!["from"],
        );
        let value = self.parse_expr(0);
        Expr {
            span: self.source.span(start.cover(lhs_range(&value))),
            kind: ExprKind::SchemaEncode {
                schema,
                value: Box::new(value),
            },
        }
    }

    pub(super) fn parse_schema_operation_path(&mut self, operation: &str) -> Vec<String> {
        let mut segments = Vec::new();
        let (diagnostic_id, missing_message, incomplete_message) = match operation {
            "encode" => (
                "parse.schema_encode_expression",
                "schema encode expression is missing a schema path",
                "schema encode expression has an incomplete schema path",
            ),
            _ => (
                "parse.schema_decode_expression",
                "schema decode expression is missing a schema path",
                "schema decode expression has an incomplete schema path",
            ),
        };
        if self.at(TokenKind::Ident) {
            segments.push(self.bump().text);
        } else {
            self.error_current(
                diagnostic_id,
                missing_message,
                vec!["schema path"],
                RecoveryStrategy::InsertToken,
                Some("from"),
            );
        }
        while self.eat(TokenKind::DoubleColon).is_some() {
            if self.at(TokenKind::Ident) {
                segments.push(self.bump().text);
            } else {
                self.error_current(
                    diagnostic_id,
                    incomplete_message,
                    vec!["schema path segment"],
                    RecoveryStrategy::InsertToken,
                    Some("from"),
                );
                break;
            }
        }
        segments
    }

    pub(super) fn parse_hole_primary(&mut self, token: Token) -> Expr {
        self.bump();
        let name = token
            .text
            .strip_prefix('_')
            .and_then(|suffix| (!suffix.is_empty()).then(|| suffix.to_string()));
        let satisfy = self.parse_satisfy_clause();
        Expr {
            kind: ExprKind::Hole { name, satisfy },
            span: self.source.span(token.range),
        }
    }

    pub(super) fn parse_literal_primary(
        &mut self,
        token: Token,
        kind: impl FnOnce(String) -> ExprKind,
    ) -> Expr {
        self.bump();
        Expr {
            kind: kind(token.text),
            span: self.source.span(token.range),
        }
    }

    pub(super) fn parse_group_or_unit_primary(&mut self) -> Expr {
        let start = self.bump();
        if let Some(end) = self.eat(TokenKind::RParen) {
            return Expr {
                kind: ExprKind::Unit,
                span: self.source.span(start.range.cover(end.range)),
            };
        }
        let expr = self.parse_expr(0);
        let end = self
            .eat(TokenKind::RParen)
            .map_or_else(|| lhs_range(&expr), |token| token.range);
        Expr {
            span: self.source.span(start.range.cover(end)),
            ..expr
        }
    }

    pub(super) fn parse_record_or_dict_primary(&mut self) -> Expr {
        if self.peek_kind(1) == Some(TokenKind::RBrace)
            || (self.peek_kind(1) == Some(TokenKind::Ident)
                && self.peek_kind(2) == Some(TokenKind::Colon))
        {
            self.parse_record()
        } else {
            self.parse_dict()
        }
    }

    pub(super) fn parse_missing_primary(&mut self, token: Token) -> Expr {
        self.bump();
        Expr {
            kind: ExprKind::Missing,
            span: self.source.span(token.range),
        }
    }
}
