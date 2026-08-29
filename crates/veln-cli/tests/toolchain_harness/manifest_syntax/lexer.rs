use super::*;

pub(super) struct Lexer<'p, 'a> {
    path: &'p Path,
    text: &'a str,
    offset: usize,
    line: usize,
}

impl<'p, 'a> Lexer<'p, 'a> {
    pub(super) fn new(path: &'p Path, text: &'a str) -> Self {
        Self {
            path,
            text,
            offset: 0,
            line: 1,
        }
    }

    pub(super) fn lex(self) -> Lexed<'a> {
        let path = self.path;
        let Lexed {
            tokens,
            boundary_error,
        } = self.lex_with_boundary();
        if let Some(error) = boundary_error {
            manifest_error(path, error.line, error.message);
        }
        Lexed {
            tokens,
            boundary_error: None,
        }
    }

    pub(super) fn lex_with_boundary(mut self) -> Lexed<'a> {
        let mut tokens = Vec::new();
        while self.offset < self.text.len() {
            while matches!(self.peek_char(), Some(' ' | '\t')) {
                self.next_char();
            }
            if self.offset >= self.text.len() {
                break;
            }
            let start = self.offset;
            let line = self.line;
            let ch = self.peek_char().expect("offset should be in text");
            let kind = match self.lex_token_kind(ch, start, line) {
                Ok(kind) => kind,
                Err(error) => {
                    return Lexed {
                        tokens,
                        boundary_error: Some(error),
                    };
                }
            };
            tokens.push(Token {
                kind,
                line,
                start,
                end: self.offset,
            });
        }
        Lexed {
            tokens,
            boundary_error: None,
        }
    }

    fn lex_token_kind(
        &mut self,
        ch: char,
        start: usize,
        _line: usize,
    ) -> Result<TokenKind<'a>, SyntaxError> {
        Ok(match ch {
            ' ' | '\t' => {
                self.next_char();
                unreachable!("layout should be skipped before lexing token kind");
            }
            '\n' => {
                self.next_char();
                TokenKind::Newline
            }
            '\r' => {
                if !self.text[self.offset..].starts_with("\r\n") {
                    return Err(SyntaxError {
                        line: self.line,
                        message: "lone carriage return in manifest".to_string(),
                    });
                }
                self.offset += 2;
                self.line += 1;
                TokenKind::Newline
            }
            '#' => {
                while !matches!(self.peek_char(), None | Some('\n' | '\r')) {
                    self.next_char();
                }
                TokenKind::Comment
            }
            '[' => {
                self.next_char();
                TokenKind::Open(Delimiter::Square)
            }
            ']' => {
                self.next_char();
                TokenKind::Close(Delimiter::Square)
            }
            '{' => {
                self.next_char();
                TokenKind::Open(Delimiter::Brace)
            }
            '}' => {
                self.next_char();
                TokenKind::Close(Delimiter::Brace)
            }
            '=' => {
                self.next_char();
                TokenKind::Equals
            }
            ',' => {
                self.next_char();
                TokenKind::Comma
            }
            '"' | '\'' => TokenKind::String(self.lex_string(ch)?),
            _ => {
                while let Some(ch) = self.peek_char() {
                    if matches!(
                        ch,
                        ' ' | '\t'
                            | '\n'
                            | '\r'
                            | '#'
                            | '['
                            | ']'
                            | '{'
                            | '}'
                            | '='
                            | ','
                            | '"'
                            | '\''
                    ) {
                        break;
                    }
                    self.next_char();
                }
                TokenKind::Atom(&self.text[start..self.offset])
            }
        })
    }

    fn lex_string(&mut self, quote: char) -> Result<StringToken<'a>, SyntaxError> {
        let start = self.offset;
        let opening_line = self.line;
        let multiline =
            self.text[self.offset..].starts_with(if quote == '"' { "\"\"\"" } else { "'''" });
        let form = match (quote, multiline) {
            ('"', false) => StringForm::Basic,
            ('\'', false) => StringForm::Literal,
            ('"', true) => StringForm::MultilineBasic,
            ('\'', true) => StringForm::MultilineLiteral,
            _ => unreachable!(),
        };
        self.offset += if multiline { 3 } else { 1 };
        let closing_quotes = loop {
            let Some(ch) = self.peek_char() else {
                let raw = &self.text[start..self.offset];
                decode_toml_string(raw, opening_line, form, 0)?;
                return Err(SyntaxError {
                    line: opening_line,
                    message: "unterminated manifest string".to_string(),
                });
            };
            if ch == '\r' {
                if !self.text[self.offset..].starts_with("\r\n") {
                    return Err(SyntaxError {
                        line: self.line,
                        message: "lone carriage return in manifest".to_string(),
                    });
                }
                if !multiline {
                    return Err(self.single_line_newline_or_pending_string_error(
                        start,
                        opening_line,
                        form,
                    ));
                }
                self.offset += 2;
                self.line += 1;
                continue;
            }
            if ch == '\n' {
                if !multiline {
                    return Err(self.single_line_newline_or_pending_string_error(
                        start,
                        opening_line,
                        form,
                    ));
                }
                self.next_char();
                continue;
            }
            if form.is_basic() && ch == '\\' {
                self.next_char();
                if self.peek_char() == Some('\r') && !self.text[self.offset..].starts_with("\r\n") {
                    return Err(SyntaxError {
                        line: self.line,
                        message: "lone carriage return in manifest".to_string(),
                    });
                }
                if self.peek_char().is_some() {
                    self.next_char();
                }
                continue;
            }
            if ch != quote {
                self.next_char();
                continue;
            }

            let run_start = self.offset;
            while self.peek_char() == Some(quote) {
                self.next_char();
            }
            let run = (self.offset - run_start) / quote.len_utf8();
            if !multiline {
                self.offset = run_start + quote.len_utf8();
                break 1;
            }
            if run >= 3 {
                break run;
            }
        };

        let raw = &self.text[start..self.offset];
        let decoded = decode_toml_string(raw, opening_line, form, closing_quotes);
        Ok(StringToken {
            decoded,
            source: raw,
        })
    }

    fn single_line_newline_or_pending_string_error(
        &self,
        start: usize,
        opening_line: usize,
        form: StringForm,
    ) -> SyntaxError {
        let raw = &self.text[start..self.offset];
        if let Err(error) = decode_toml_string(raw, opening_line, form, 0) {
            return error;
        }
        SyntaxError {
            line: opening_line,
            message: "newline in single-line string".to_string(),
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.text[self.offset..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
        }
        Some(ch)
    }
}
