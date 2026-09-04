use super::*;

pub(super) fn decode_toml_string(
    raw: &str,
    opening_line: usize,
    form: StringForm,
    closing_quotes: usize,
) -> Result<DecodedString, SyntaxError> {
    if form.is_multiline() && closing_quotes > 5 {
        return Err(SyntaxError {
            line: opening_line + raw.bytes().filter(|byte| *byte == b'\n').count(),
            message: "invalid multiline string quote run".to_string(),
        });
    }
    let opening_len = if form.is_multiline() { 3 } else { 1 };
    let closing_len = if closing_quotes == 0 {
        0
    } else if form.is_multiline() {
        closing_quotes
    } else {
        1
    };
    let mut content = &raw[opening_len..raw.len().saturating_sub(closing_len)];
    let mut line = opening_line;
    if form.is_multiline() {
        if content.starts_with("\r\n") {
            content = &content[2..];
            line += 1;
        } else if content.starts_with('\n') {
            content = &content[1..];
            line += 1;
        }
    }

    let mut decoded = TomlStringDecoder::new(content, line, form).decode()?;

    if form.is_multiline() && closing_quotes >= 4 {
        let closing_line = opening_line + raw.bytes().filter(|byte| *byte == b'\n').count();
        for _ in 0..closing_quotes - 3 {
            decoded.chars.push(DecodedChar {
                value: form.quote(),
                source_line: closing_line,
                escaped: false,
            });
        }
    }
    Ok(decoded)
}

struct TomlStringDecoder<'a> {
    content: &'a str,
    form: StringForm,
    line: usize,
    offset: usize,
    chars: Vec<DecodedChar>,
}

impl<'a> TomlStringDecoder<'a> {
    fn new(content: &'a str, line: usize, form: StringForm) -> Self {
        Self {
            content,
            form,
            line,
            offset: 0,
            chars: Vec::new(),
        }
    }

    fn decode(mut self) -> Result<DecodedString, SyntaxError> {
        while self.offset < self.content.len() {
            self.decode_next_char()?;
        }
        Ok(DecodedString { chars: self.chars })
    }

    fn decode_next_char(&mut self) -> Result<(), SyntaxError> {
        let ch = self.peek_char().expect("content has a char");
        match ch {
            '\r' => self.decode_carriage_return(),
            '\n' => self.decode_line_feed(),
            prohibited if is_prohibited_control(prohibited) => Err(SyntaxError {
                line: self.line,
                message: "prohibited control character in manifest string".to_string(),
            }),
            '\\' if self.form.is_basic() => self.decode_escape(),
            plain => {
                self.push(plain, self.line, false);
                self.offset += plain.len_utf8();
                Ok(())
            }
        }
    }

    fn decode_carriage_return(&mut self) -> Result<(), SyntaxError> {
        if !self.remaining().starts_with("\r\n") {
            return Err(SyntaxError {
                line: self.line,
                message: "lone carriage return in manifest string".to_string(),
            });
        }
        self.decode_physical_newline(2)
    }

    fn decode_line_feed(&mut self) -> Result<(), SyntaxError> {
        self.decode_physical_newline(1)
    }

    fn decode_physical_newline(&mut self, width: usize) -> Result<(), SyntaxError> {
        if !self.form.is_multiline() {
            return Err(SyntaxError {
                line: self.line,
                message: "newline in single-line string".to_string(),
            });
        }
        self.push('\n', self.line, false);
        self.offset += width;
        self.line += 1;
        Ok(())
    }

    fn decode_escape(&mut self) -> Result<(), SyntaxError> {
        let escape_line = self.line;
        self.offset += 1;
        if self.form.is_multiline() && self.consume_continuation() {
            return Ok(());
        }

        let Some(escaped) = self.peek_char() else {
            return Err(SyntaxError {
                line: escape_line,
                message: "unterminated manifest string escape".to_string(),
            });
        };
        self.offset += escaped.len_utf8();
        let value = match escaped {
            'b' => '\u{08}',
            't' => '\t',
            'n' => '\n',
            'f' => '\u{0c}',
            'r' => '\r',
            '"' => '"',
            '\\' => '\\',
            'u' => self.decode_unicode_escape(escape_line, 4)?,
            'U' => self.decode_unicode_escape(escape_line, 8)?,
            _ => {
                return Err(SyntaxError {
                    line: escape_line,
                    message: format!("unsupported manifest string escape `{escaped}`"),
                });
            }
        };
        self.push(value, escape_line, true);
        Ok(())
    }

    fn consume_continuation(&mut self) -> bool {
        let mut lookahead = self.offset;
        while matches!(self.content.as_bytes().get(lookahead), Some(b' ' | b'\t')) {
            lookahead += 1;
        }
        if !self.content[lookahead..].starts_with("\r\n")
            && !self.content[lookahead..].starts_with('\n')
        {
            return false;
        }

        self.offset = lookahead;
        self.consume_newline();
        while self.consume_continuation_whitespace() {}
        true
    }

    fn consume_continuation_whitespace(&mut self) -> bool {
        match self.content.as_bytes().get(self.offset) {
            Some(b' ' | b'\t') => self.offset += 1,
            Some(b'\n') => self.consume_newline(),
            Some(b'\r') if self.remaining().starts_with("\r\n") => self.consume_newline(),
            _ => return false,
        }
        true
    }

    fn consume_newline(&mut self) {
        self.offset += if self.remaining().starts_with("\r\n") {
            2
        } else {
            1
        };
        self.line += 1;
    }

    fn decode_unicode_escape(
        &mut self,
        escape_line: usize,
        width: usize,
    ) -> Result<char, SyntaxError> {
        let start = self.offset;
        let mut digits = String::with_capacity(width);
        for _ in 0..width {
            let Some(ch) = self.peek_char() else {
                return Err(SyntaxError {
                    line: escape_line,
                    message: "incomplete Unicode escape".to_string(),
                });
            };
            if ch == '\n' || ch == '\r' {
                return Err(SyntaxError {
                    line: escape_line,
                    message: "incomplete Unicode escape".to_string(),
                });
            }
            if !ch.is_ascii_hexdigit() {
                return Err(SyntaxError {
                    line: self.line,
                    message: "invalid hexadecimal digit in Unicode escape".to_string(),
                });
            }
            digits.push(ch);
            self.offset += ch.len_utf8();
        }
        let codepoint = u32::from_str_radix(&digits, 16).expect("Unicode digits were validated");
        char::from_u32(codepoint).ok_or_else(|| SyntaxError {
            line: escape_line,
            message: format!("Unicode escape is not a scalar value at byte {start}"),
        })
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn remaining(&self) -> &'a str {
        &self.content[self.offset..]
    }

    fn push(&mut self, value: char, source_line: usize, escaped: bool) {
        self.chars.push(DecodedChar {
            value,
            source_line,
            escaped,
        });
    }
}

pub(super) fn is_prohibited_control(ch: char) -> bool {
    matches!(ch, '\u{0000}'..='\u{0008}' | '\u{000a}'..='\u{001f}' | '\u{007f}')
}
