use super::*;

impl<'a> Classifier<'a> {
    pub(super) fn classify_current_token(&self) -> Option<SemanticToken> {
        let token = &self.tokens[self.cursor];
        match token.kind {
            TokenKind::Whitespace
            | TokenKind::Newline
            | TokenKind::Eof
            | TokenKind::Invalid
            | TokenKind::MalformedInt => None,
            TokenKind::Comment => Some(self.simple(token, SemanticTokenType::Comment)),
            TokenKind::String => Some(self.simple(token, SemanticTokenType::String)),
            TokenKind::Int | TokenKind::Float => {
                Some(self.simple(token, SemanticTokenType::Number))
            }
            TokenKind::Hole | TokenKind::Underscore => Some(self.modified(
                token,
                SemanticTokenType::Variable,
                &[SemanticTokenModifier::Hole],
            )),
            TokenKind::Pub
            | TokenKind::Fn
            | TokenKind::Type
            | TokenKind::Schema
            | TokenKind::Codec
            | TokenKind::For
            | TokenKind::Decode
            | TokenKind::Encode
            | TokenKind::Derive
            | TokenKind::With
            | TokenKind::Format
            | TokenKind::Where
            | TokenKind::Test
            | TokenKind::Effect
            | TokenKind::Effects
            | TokenKind::Perform
            | TokenKind::Handler
            | TokenKind::Handles
            | TokenKind::Handle
            | TokenKind::Let
            | TokenKind::End
            | TokenKind::Require
            | TokenKind::Ensure
            | TokenKind::Invariant
            | TokenKind::Mod
            | TokenKind::Use
            | TokenKind::From
            | TokenKind::At
            | TokenKind::Match
            | TokenKind::If
            | TokenKind::Else
            | TokenKind::Or
            | TokenKind::And
            | TokenKind::Not => Some(self.simple(token, SemanticTokenType::Keyword)),
            TokenKind::Ident => Some(self.classify_ident(token)),
            TokenKind::LParen
            | TokenKind::RParen
            | TokenKind::LBracket
            | TokenKind::RBracket
            | TokenKind::LBrace
            | TokenKind::RBrace
            | TokenKind::Comma
            | TokenKind::Semicolon
            | TokenKind::Colon
            | TokenKind::Dot
            | TokenKind::DoubleColon
            | TokenKind::Arrow
            | TokenKind::FatArrow
            | TokenKind::PipeGreater
            | TokenKind::Pipe
            | TokenKind::Ampersand
            | TokenKind::Caret
            | TokenKind::Tilde
            | TokenKind::ShiftLeft
            | TokenKind::ShiftRight
            | TokenKind::ShiftRightLogical
            | TokenKind::Question
            | TokenKind::Equal
            | TokenKind::EqualEqual
            | TokenKind::BangEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash => Some(self.simple(token, SemanticTokenType::Operator)),
        }
    }

    pub(super) fn classify_ident(&self, token: &Token) -> SemanticToken {
        if matches!(token.text.as_str(), "true" | "false") {
            return self.simple(token, SemanticTokenType::Keyword);
        }
        if token.text == "satisfy"
            && matches!(
                self.previous_significant_kind(),
                Some(TokenKind::Hole | TokenKind::Underscore)
            )
        {
            return self.simple(token, SemanticTokenType::Keyword);
        }
        if self.previous_significant_kind() == Some(TokenKind::Dot) {
            return self.simple(token, SemanticTokenType::Property);
        }
        if self.next_significant_kind() == Some(TokenKind::Equal) {
            return self.simple(token, SemanticTokenType::Property);
        }
        if self.next_significant_kind() == Some(TokenKind::Colon) {
            return self.simple(token, SemanticTokenType::Property);
        }
        if is_type_name(&token.text) {
            return self.simple(token, SemanticTokenType::Type);
        }
        if self.next_significant_kind() == Some(TokenKind::LParen)
            || self.function_names.contains(&token.text)
            || is_prelude_function(&token.text)
        {
            let modifiers = if is_prelude_function(&token.text) {
                SemanticTokenModifiers::empty().with(SemanticTokenModifier::DefaultLibrary)
            } else {
                SemanticTokenModifiers::empty()
            };
            return self.token(token, SemanticTokenType::Function, modifiers);
        }
        if self.params.contains(&token.text) {
            return self.modified(
                token,
                SemanticTokenType::Parameter,
                &[SemanticTokenModifier::Readonly],
            );
        }
        if self.locals.contains(&token.text) {
            return self.modified(
                token,
                SemanticTokenType::Variable,
                &[SemanticTokenModifier::Readonly],
            );
        }
        self.simple(token, SemanticTokenType::Variable)
    }

    pub(super) fn simple(&self, token: &Token, token_type: SemanticTokenType) -> SemanticToken {
        self.token(token, token_type, SemanticTokenModifiers::empty())
    }

    pub(super) fn modified(
        &self,
        token: &Token,
        token_type: SemanticTokenType,
        modifiers: &[SemanticTokenModifier],
    ) -> SemanticToken {
        let modifiers = modifiers
            .iter()
            .fold(SemanticTokenModifiers::empty(), |set, modifier| {
                set.with(*modifier)
            });
        self.token(token, token_type, modifiers)
    }

    pub(super) fn token(
        &self,
        token: &Token,
        token_type: SemanticTokenType,
        modifiers: SemanticTokenModifiers,
    ) -> SemanticToken {
        SemanticToken {
            span: self.source.span(token.range),
            kind: SemanticTokenKind { token_type },
            modifiers,
        }
    }

    pub(super) fn at(&self, kind: TokenKind) -> bool {
        self.tokens
            .get(self.cursor)
            .is_some_and(|token| token.kind == kind)
    }

    pub(super) fn eat(
        &mut self,
        kind: TokenKind,
        semantic_tokens: &mut Vec<SemanticToken>,
    ) -> bool {
        if self.at(kind) {
            let token = &self.tokens[self.cursor];
            if let Some(classified) = self.classify_current_token() {
                semantic_tokens.push(classified);
            } else if !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline) {
                semantic_tokens.push(self.simple(token, SemanticTokenType::Operator));
            }
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    pub(super) fn skip_trivia(&mut self) {
        while self
            .tokens
            .get(self.cursor)
            .is_some_and(|token| matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        {
            self.cursor += 1;
        }
    }

    pub(super) fn previous_significant_kind(&self) -> Option<TokenKind> {
        self.tokens[..self.cursor]
            .iter()
            .rev()
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
            .map(|token| token.kind)
    }

    pub(super) fn next_significant_kind(&self) -> Option<TokenKind> {
        self.tokens
            .iter()
            .skip(self.cursor + 1)
            .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
            .map(|token| token.kind)
    }
}
