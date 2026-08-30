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
        self.expect_name(context, expected, true)
    }

    pub(super) fn expect_name(
        &mut self,
        context: &'static str,
        expected: &'static str,
        allow_hole: bool,
    ) -> (Option<String>, Option<SourceSpan>) {
        if is_contextual_identifier(self.current().kind) || (allow_hole && self.at(TokenKind::Hole))
        {
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

    pub(super) fn skip_to_next_line(&mut self) {
        while !self.at(TokenKind::Newline) && !self.at(TokenKind::Eof) {
            self.bump();
        }
        if self.at(TokenKind::Newline) {
            self.bump();
        }
    }

    pub(super) fn peek_at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor + 1)
            .is_some_and(|token| token.kind == kind)
    }
}
