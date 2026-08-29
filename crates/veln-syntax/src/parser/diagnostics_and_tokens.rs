use super::*;

impl<'a> Parser<'a> {
    pub(super) fn expect_ident(
        &mut self,
        context: &'static str,
        expected: &'static str,
    ) -> Option<String> {
        if is_contextual_identifier(self.current().kind) {
            Some(self.bump().text)
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            None
        }
    }

    pub(super) fn expect_covered_name(
        &mut self,
        context: &'static str,
        expected: &'static str,
    ) -> (Option<String>, Option<SourceSpan>) {
        if is_contextual_identifier(self.current().kind) || self.at(TokenKind::Hole) {
            let token = self.bump();
            let span = self.source.span(token.range);
            (Some(token.text), Some(span))
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            (None, None)
        }
    }

    pub(super) fn expect_ident_text(
        &mut self,
        text: &'static str,
        context: &'static str,
        expected: &'static str,
    ) -> Token {
        if self.at_ident_text(text) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_identifier",
                format!("expected {expected}"),
                context,
                vec![expected],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    pub(super) fn expect_newline(&mut self, context: &'static str) -> Token {
        if self.at(TokenKind::Newline) {
            self.bump()
        } else if self.at(TokenKind::Eof) {
            self.current().clone()
        } else {
            self.error_current(
                "parse.expected_newline",
                "expected a newline",
                context,
                vec!["newline"],
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    pub(super) fn expect(
        &mut self,
        kind: TokenKind,
        context: &'static str,
        expected: Vec<&'static str>,
    ) -> Token {
        if self.at(kind) {
            self.bump()
        } else {
            self.error_current(
                "parse.expected_token",
                format!("expected {}", expected.join(" or ")),
                context,
                expected,
                RecoveryStrategy::InsertToken,
                None,
            );
            self.current().clone()
        }
    }

    pub(super) fn error_current(
        &mut self,
        id: &'static str,
        message: impl Into<String>,
        parser_context: &'static str,
        expected: Vec<&'static str>,
        strategy: RecoveryStrategy,
        anchor: Option<&'static str>,
    ) {
        let current = self.current().clone();
        self.error_at_token(
            &current,
            DiagnosticRequest {
                id,
                message: message.into(),
                parser_context,
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

    pub(super) fn synchronize_to_item(&mut self) {
        let start = self.cursor;
        while !self.at(TokenKind::Eof)
            && !self.at(TokenKind::Pub)
            && !self.at(TokenKind::Fn)
            && !self.at(TokenKind::Type)
            && !self.at(TokenKind::Schema)
            && !self.at(TokenKind::Codec)
            && !self.at(TokenKind::Test)
            && !self.at(TokenKind::End)
        {
            self.bump();
        }
        let at_eof = self.at(TokenKind::Eof);
        let anchor = match self.current().kind {
            TokenKind::Pub => Some("pub".to_string()),
            TokenKind::Fn => Some("fn".to_string()),
            TokenKind::Type => Some("type".to_string()),
            TokenKind::Schema => Some("schema".to_string()),
            TokenKind::Codec => Some("codec".to_string()),
            TokenKind::Test => Some("test".to_string()),
            TokenKind::End => Some("end".to_string()),
            TokenKind::Eof => None,
            _ => None,
        };
        if let Some(last) = self.diagnostics.last_mut() {
            last.recovery.dropped_token_count = self.cursor.saturating_sub(start);
            if anchor.is_some() || at_eof {
                last.recovery.anchor = anchor;
            }
        }
    }

    pub(super) fn eat_newlines(&mut self) {
        while self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    pub(super) fn skip_to_next_line(&mut self) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            self.bump();
        }
        if self.at(TokenKind::Newline) {
            self.bump();
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
        self.current().kind == kind
    }

    pub(super) fn peek_at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| token.kind == kind)
    }

    pub(super) fn peek_kind(&self, offset: usize) -> Option<TokenKind> {
        self.tokens
            .get(self.cursor + offset)
            .map(|token| token.kind)
    }

    pub(super) fn at_ident_text(&self, text: &str) -> bool {
        self.at(TokenKind::Ident) && self.current().text == text
    }

    pub(super) fn current(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    pub(super) fn previous(&self) -> Option<&Token> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    pub(super) fn bump(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::Eof {
            self.cursor += 1;
        }
        token
    }
}
