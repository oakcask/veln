use super::*;

pub(super) struct DocumentParser<'p, 'a> {
    path: &'p Path,
    text: &'a str,
    tokens: Vec<Token<'a>>,
    index: usize,
}

impl<'p, 'a> DocumentParser<'p, 'a> {
    pub(super) fn new(path: &'p Path, text: &'a str, tokens: Vec<Token<'a>>) -> Self {
        Self {
            path,
            text,
            tokens,
            index: 0,
        }
    }

    pub(super) fn parse(mut self) -> Vec<Statement<'a>> {
        let mut statements = Vec::new();
        loop {
            self.skip_statement_layout();
            let Some(token) = self.tokens.get(self.index) else {
                break;
            };
            if matches!(token.kind, TokenKind::Open(Delimiter::Square)) {
                statements.push(self.parse_section());
            } else {
                statements.push(self.parse_assignment());
            }
        }
        statements
    }

    fn parse_section(&mut self) -> Statement<'a> {
        let line = self.tokens[self.index].line;
        self.index += 1;
        let array = self
            .tokens
            .get(self.index)
            .is_some_and(|token| matches!(token.kind, TokenKind::Open(Delimiter::Square)));
        if array {
            self.index += 1;
        }
        let name = match self.tokens.get(self.index) {
            Some(Token {
                kind: TokenKind::Atom(name),
                ..
            }) => (*name).to_string(),
            _ => manifest_error(self.path, line, "expected section name"),
        };
        self.index += 1;
        self.expect_close(Delimiter::Square, line);
        if array {
            self.expect_close(Delimiter::Square, line);
        }
        self.expect_statement_end();
        let name = if array {
            format!("[[{name}]]")
        } else {
            format!("[{name}]")
        };
        Statement::Section { name, line }
    }

    fn parse_assignment(&mut self) -> Statement<'a> {
        let (key, line) = match self.tokens.get(self.index) {
            Some(Token {
                kind: TokenKind::Atom(key),
                line,
                ..
            }) => (*key, *line),
            Some(token) => manifest_error(self.path, token.line, "expected manifest key"),
            None => unreachable!(),
        };
        self.index += 1;
        match self.tokens.get(self.index) {
            Some(Token {
                kind: TokenKind::Equals,
                ..
            }) => self.index += 1,
            _ => manifest_error(self.path, line, "expected `key = value`"),
        }

        let first = self.tokens.get(self.index).unwrap_or_else(|| {
            manifest_error(self.path, line, "expected manifest value");
        });
        if matches!(first.kind, TokenKind::Newline | TokenKind::Comment) {
            manifest_error(self.path, line, "expected manifest value");
        }
        let value_line = first.line;
        let value_start = first.start;
        let token_start = self.index;
        let mut unterminated = None;

        if let TokenKind::Open(opening) = first.kind {
            let mut stack = vec![(opening, first.line)];
            self.index += 1;
            while let Some(token) = self.tokens.get(self.index) {
                match token.kind {
                    TokenKind::Open(delimiter) => stack.push((delimiter, token.line)),
                    TokenKind::Close(delimiter) => {
                        let Some((expected, _)) = stack.last().copied() else {
                            manifest_error(self.path, token.line, "unexpected closing delimiter");
                        };
                        if delimiter != expected {
                            manifest_error(
                                self.path,
                                token.line,
                                format!(
                                    "unexpected closing delimiter; expected `{}`",
                                    expected.closing()
                                ),
                            );
                        }
                        stack.pop();
                        self.index += 1;
                        if stack.is_empty() {
                            break;
                        }
                        continue;
                    }
                    _ => {}
                }
                self.index += 1;
            }
            if let Some(open) = stack.last().copied() {
                unterminated = Some(open);
            }
        } else {
            self.index += 1;
        }

        let token_end = self.index;
        let value_end = self.tokens[token_end.saturating_sub(1)].end;
        let value = Value {
            raw: &self.text[value_start..value_end],
            line: value_line,
            tokens: self.tokens[token_start..token_end].to_vec(),
            unterminated,
        };
        if value.unterminated.is_none() {
            self.expect_statement_end();
        }
        Statement::Assignment { key, line, value }
    }

    fn expect_close(&mut self, delimiter: Delimiter, opening_line: usize) {
        match self.tokens.get(self.index) {
            Some(Token {
                kind: TokenKind::Close(actual),
                ..
            }) if *actual == delimiter => self.index += 1,
            Some(token) => manifest_error(
                self.path,
                token.line,
                format!("expected `{}`", delimiter.closing()),
            ),
            None => manifest_error(
                self.path,
                opening_line,
                format!("expected `{}`", delimiter.closing()),
            ),
        }
    }

    fn expect_statement_end(&mut self) {
        if matches!(
            self.tokens.get(self.index).map(|token| &token.kind),
            Some(TokenKind::Comment)
        ) {
            self.index += 1;
        }
        match self.tokens.get(self.index) {
            Some(Token {
                kind: TokenKind::Newline,
                ..
            }) => self.index += 1,
            Some(token) => manifest_error(
                self.path,
                token.line,
                "unexpected token after completed manifest value",
            ),
            None => {}
        }
    }

    fn skip_statement_layout(&mut self) {
        while matches!(
            self.tokens.get(self.index).map(|token| &token.kind),
            Some(TokenKind::Newline | TokenKind::Comment)
        ) {
            self.index += 1;
        }
    }
}
