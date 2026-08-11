use std::path::Path;

use super::manifest_error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Delimiter {
    Square,
    Brace,
}

impl Delimiter {
    fn closing(self) -> char {
        match self {
            Self::Square => ']',
            Self::Brace => '}',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringForm {
    Basic,
    Literal,
    MultilineBasic,
    MultilineLiteral,
}

impl StringForm {
    fn quote(self) -> char {
        match self {
            Self::Basic | Self::MultilineBasic => '"',
            Self::Literal | Self::MultilineLiteral => '\'',
        }
    }

    fn is_multiline(self) -> bool {
        matches!(self, Self::MultilineBasic | Self::MultilineLiteral)
    }

    fn is_basic(self) -> bool {
        matches!(self, Self::Basic | Self::MultilineBasic)
    }
}

#[derive(Clone, Debug)]
struct DecodedChar {
    value: char,
    source_line: usize,
    escaped: bool,
}

#[derive(Clone, Debug)]
struct DecodedString {
    chars: Vec<DecodedChar>,
}

impl DecodedString {
    fn text(&self) -> String {
        self.chars
            .iter()
            .map(|decoded| {
                let _source_line = decoded.source_line;
                decoded.value
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct StringToken<'a> {
    decoded: Result<DecodedString, SyntaxError>,
    source: &'a str,
}

#[derive(Clone, Debug)]
struct SyntaxError {
    line: usize,
    message: String,
}

#[derive(Clone, Debug)]
enum TokenKind<'a> {
    Atom(&'a str),
    String(StringToken<'a>),
    Open(Delimiter),
    Close(Delimiter),
    Equals,
    Comma,
    Comment,
    Newline,
}

#[derive(Clone, Debug)]
struct Token<'a> {
    kind: TokenKind<'a>,
    line: usize,
    start: usize,
    end: usize,
}

#[derive(Debug)]
pub(super) enum Statement<'a> {
    Section {
        name: String,
        line: usize,
    },
    Assignment {
        key: &'a str,
        line: usize,
        value: Value<'a>,
    },
}

#[derive(Debug)]
pub(super) struct Value<'a> {
    raw: &'a str,
    line: usize,
    tokens: Vec<Token<'a>>,
    unterminated: Option<(Delimiter, usize)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManifestPolicyFinding {
    pub(crate) field: String,
    pub(crate) line: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) spelling: String,
    pub(crate) category: &'static str,
}

impl Value<'_> {
    pub(super) fn line(&self) -> usize {
        self.line
    }

    pub(super) fn raw(&self) -> &str {
        self.raw
    }

    pub(super) fn parse_string(&self, path: &Path) -> String {
        if self.tokens.len() != 1 {
            manifest_error(path, self.line, "expected string");
        }
        let TokenKind::String(token) = &self.tokens[0].kind else {
            manifest_error(path, self.line, "expected string");
        };
        let _source_spelling = token.source;
        token
            .decoded
            .as_ref()
            .unwrap_or_else(|error| {
                manifest_error(path, error.line, &error.message);
            })
            .text()
    }

    pub(super) fn is_string(&self) -> bool {
        self.tokens.len() == 1 && matches!(self.tokens[0].kind, TokenKind::String(_))
    }

    pub(super) fn parse_string_array(&self, path: &Path) -> Vec<String> {
        let Some(Token {
            kind: TokenKind::Open(Delimiter::Square),
            ..
        }) = self.tokens.first()
        else {
            manifest_error(path, self.line, "expected string array");
        };

        let complete = self.unterminated.is_none();
        let end = if complete {
            self.tokens.len().saturating_sub(1)
        } else {
            self.tokens.len()
        };
        let mut index = 1;
        let mut values = Vec::new();
        let mut expect_value = true;
        let mut trailing_comma = false;

        while index < end {
            while index < end
                && matches!(
                    self.tokens[index].kind,
                    TokenKind::Newline | TokenKind::Comment
                )
            {
                index += 1;
            }
            if index == end {
                break;
            }

            let token = &self.tokens[index];
            if expect_value {
                match &token.kind {
                    TokenKind::String(string) => {
                        values.push(
                            string
                                .decoded
                                .as_ref()
                                .unwrap_or_else(|error| {
                                    manifest_error(path, error.line, &error.message)
                                })
                                .text(),
                        );
                        expect_value = false;
                        trailing_comma = false;
                    }
                    _ => manifest_error(path, token.line, "expected string array element"),
                }
            } else if matches!(token.kind, TokenKind::Comma) {
                expect_value = true;
                trailing_comma = true;
            } else if matches!(token.kind, TokenKind::String(_)) {
                manifest_error(path, token.line, "expected `,` before string array element");
            } else {
                manifest_error(path, token.line, "expected `,` after string array element");
            }
            index += 1;
        }

        if let Some((delimiter, line)) = self.unterminated {
            manifest_error(
                path,
                line,
                format!("unterminated container; expected `{}`", delimiter.closing()),
            );
        }
        if expect_value && !values.is_empty() && !trailing_comma {
            manifest_error(path, self.line, "expected string array element");
        }
        values
    }

    pub(super) fn json_error_line(&self, byte_offset: usize) -> usize {
        self.line
            + self.raw.as_bytes()[..byte_offset.min(self.raw.len())]
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
    }

    pub(super) fn report_unterminated(&self, path: &Path) -> ! {
        let (delimiter, line) = self
            .unterminated
            .expect("unterminated value should retain its innermost opener");
        manifest_error(
            path,
            line,
            format!("unterminated container; expected `{}`", delimiter.closing()),
        )
    }

    pub(super) fn is_unterminated(&self) -> bool {
        self.unterminated.is_some()
    }

    fn collect_policy_findings(&self, field: &str, findings: &mut Vec<ManifestPolicyFinding>) {
        for token in &self.tokens {
            let TokenKind::String(string) = &token.kind else {
                continue;
            };
            let decoded = string.decoded.as_ref().unwrap_or_else(|error| {
                manifest_error(Path::new("case.toml"), error.line, &error.message)
            });
            for decoded_char in &decoded.chars {
                if decoded_char.escaped && matches!(decoded_char.value, '\n' | '\r') {
                    findings.push(ManifestPolicyFinding {
                        field: field.to_string(),
                        line: decoded_char.source_line,
                        start: token.start,
                        end: token.end,
                        spelling: if decoded_char.value == '\n' {
                            "escape-produced LF".to_string()
                        } else {
                            "escape-produced CR".to_string()
                        },
                        category: "escape-produced-line-break",
                    });
                }
            }
            let text = decoded.text();
            for spelling in forbidden_decoded_spellings(&text) {
                findings.push(ManifestPolicyFinding {
                    field: field.to_string(),
                    line: token.line,
                    start: token.start,
                    end: token.end,
                    spelling,
                    category: "decoded-line-break-spelling",
                });
            }
        }
    }
}

pub(crate) fn parse_document<'a>(path: &Path, text: &'a str) -> Vec<Statement<'a>> {
    let tokens = Lexer::new(path, text).lex();
    DocumentParser::new(path, text, tokens).parse()
}

pub(crate) fn manifest_policy_findings(path: &Path, text: &str) -> Vec<ManifestPolicyFinding> {
    let mut section = String::new();
    let mut findings = Vec::new();
    for statement in parse_document(path, text) {
        match statement {
            Statement::Section { name, .. } => section = name,
            Statement::Assignment { key, value, .. } => {
                let field = if section.is_empty() {
                    key.to_string()
                } else {
                    format!("{section}.{key}")
                };
                value.collect_policy_findings(&field, &mut findings);
            }
        }
    }
    findings.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then(left.start.cmp(&right.start))
            .then(left.end.cmp(&right.end))
            .then(left.category.cmp(right.category))
            .then(left.field.cmp(&right.field))
            .then(left.spelling.cmp(&right.spelling))
    });
    findings
}

fn forbidden_decoded_spellings(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut findings = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            index += 1;
            continue;
        }
        if let Some(byte) = bytes.get(index + 1) {
            if matches!(*byte, b'n' | b'r') {
                findings.push(text[index..index + 2].to_string());
                index += 2;
                continue;
            }
        }
        if index + 6 <= bytes.len()
            && matches!(bytes.get(index + 1), Some(b'u'))
            && matches_forbidden_hex(&text[index + 2..index + 6])
        {
            findings.push(text[index..index + 6].to_string());
            index += 6;
            continue;
        }
        if index + 10 <= bytes.len()
            && matches!(bytes.get(index + 1), Some(b'U'))
            && matches_forbidden_wide_hex(&text[index + 2..index + 10])
        {
            findings.push(text[index..index + 10].to_string());
            index += 10;
            continue;
        }
        index += 1;
    }
    findings
}

fn matches_forbidden_hex(digits: &str) -> bool {
    digits.eq_ignore_ascii_case("000a") || digits.eq_ignore_ascii_case("000d")
}

fn matches_forbidden_wide_hex(digits: &str) -> bool {
    digits.eq_ignore_ascii_case("0000000a") || digits.eq_ignore_ascii_case("0000000d")
}

struct DocumentParser<'p, 'a> {
    path: &'p Path,
    text: &'a str,
    tokens: Vec<Token<'a>>,
    index: usize,
}

impl<'p, 'a> DocumentParser<'p, 'a> {
    fn new(path: &'p Path, text: &'a str, tokens: Vec<Token<'a>>) -> Self {
        Self {
            path,
            text,
            tokens,
            index: 0,
        }
    }

    fn parse(mut self) -> Vec<Statement<'a>> {
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

struct Lexer<'p, 'a> {
    path: &'p Path,
    text: &'a str,
    offset: usize,
    line: usize,
}

impl<'p, 'a> Lexer<'p, 'a> {
    fn new(path: &'p Path, text: &'a str) -> Self {
        Self {
            path,
            text,
            offset: 0,
            line: 1,
        }
    }

    fn lex(mut self) -> Vec<Token<'a>> {
        let mut tokens = Vec::new();
        while self.offset < self.text.len() {
            let start = self.offset;
            let line = self.line;
            let ch = self.peek_char().expect("offset should be in text");
            let kind = match ch {
                ' ' | '\t' => {
                    self.next_char();
                    continue;
                }
                '\n' => {
                    self.next_char();
                    TokenKind::Newline
                }
                '\r' => {
                    if !self.text[self.offset..].starts_with("\r\n") {
                        manifest_error(self.path, self.line, "lone carriage return in manifest");
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
                '"' | '\'' => TokenKind::String(self.lex_string(ch)),
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
            };
            tokens.push(Token {
                kind,
                line,
                start,
                end: self.offset,
            });
        }
        tokens
    }

    fn lex_string(&mut self, quote: char) -> StringToken<'a> {
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
                if let Err(error) = decode_toml_string(raw, opening_line, form, 0) {
                    manifest_error(self.path, error.line, error.message);
                }
                manifest_error(self.path, opening_line, "unterminated manifest string");
            };
            if ch == '\r' {
                if !self.text[self.offset..].starts_with("\r\n") {
                    manifest_error(self.path, self.line, "lone carriage return in manifest");
                }
                if !multiline {
                    self.report_single_line_newline_or_pending_string_error(
                        start,
                        opening_line,
                        form,
                    );
                }
                self.offset += 2;
                self.line += 1;
                continue;
            }
            if ch == '\n' {
                if !multiline {
                    self.report_single_line_newline_or_pending_string_error(
                        start,
                        opening_line,
                        form,
                    );
                }
                self.next_char();
                continue;
            }
            if form.is_basic() && ch == '\\' {
                self.next_char();
                if self.peek_char() == Some('\r') && !self.text[self.offset..].starts_with("\r\n") {
                    manifest_error(self.path, self.line, "lone carriage return in manifest");
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
        StringToken {
            decoded,
            source: raw,
        }
    }

    fn report_single_line_newline_or_pending_string_error(
        &self,
        start: usize,
        opening_line: usize,
        form: StringForm,
    ) -> ! {
        let raw = &self.text[start..self.offset];
        if let Err(error) = decode_toml_string(raw, opening_line, form, 0) {
            manifest_error(self.path, error.line, error.message);
        }
        manifest_error(self.path, opening_line, "newline in single-line string");
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

fn decode_toml_string(
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

    let mut chars = Vec::new();
    let mut offset = 0;
    while offset < content.len() {
        let ch = content[offset..]
            .chars()
            .next()
            .expect("content has a char");
        if ch == '\r' {
            if !content[offset..].starts_with("\r\n") {
                return Err(SyntaxError {
                    line,
                    message: "lone carriage return in manifest string".to_string(),
                });
            }
            if !form.is_multiline() {
                return Err(SyntaxError {
                    line,
                    message: "newline in single-line string".to_string(),
                });
            }
            chars.push(DecodedChar {
                value: '\n',
                source_line: line,
                escaped: false,
            });
            offset += 2;
            line += 1;
            continue;
        }
        if ch == '\n' {
            if !form.is_multiline() {
                return Err(SyntaxError {
                    line,
                    message: "newline in single-line string".to_string(),
                });
            }
            chars.push(DecodedChar {
                value: '\n',
                source_line: line,
                escaped: false,
            });
            offset += 1;
            line += 1;
            continue;
        }
        if is_prohibited_control(ch) {
            return Err(SyntaxError {
                line,
                message: "prohibited control character in manifest string".to_string(),
            });
        }
        if !form.is_basic() || ch != '\\' {
            chars.push(DecodedChar {
                value: ch,
                source_line: line,
                escaped: false,
            });
            offset += ch.len_utf8();
            continue;
        }

        let escape_line = line;
        offset += 1;
        if form.is_multiline() {
            let mut lookahead = offset;
            while matches!(content.as_bytes().get(lookahead), Some(b' ' | b'\t')) {
                lookahead += 1;
            }
            if content[lookahead..].starts_with("\r\n") || content[lookahead..].starts_with('\n') {
                offset = lookahead;
                consume_physical_newline(content, &mut offset, &mut line);
                loop {
                    match content.as_bytes().get(offset) {
                        Some(b' ' | b'\t') => offset += 1,
                        Some(b'\n') => {
                            offset += 1;
                            line += 1;
                        }
                        Some(b'\r') if content[offset..].starts_with("\r\n") => {
                            offset += 2;
                            line += 1;
                        }
                        _ => break,
                    }
                }
                continue;
            }
        }

        let Some(escaped) = content[offset..].chars().next() else {
            return Err(SyntaxError {
                line: escape_line,
                message: "unterminated manifest string escape".to_string(),
            });
        };
        offset += escaped.len_utf8();
        let value = match escaped {
            'b' => '\u{08}',
            't' => '\t',
            'n' => '\n',
            'f' => '\u{0c}',
            'r' => '\r',
            '"' => '"',
            '\\' => '\\',
            'u' => decode_unicode_escape(content, &mut offset, &mut line, escape_line, 4)?,
            'U' => decode_unicode_escape(content, &mut offset, &mut line, escape_line, 8)?,
            _ => {
                return Err(SyntaxError {
                    line: escape_line,
                    message: format!("unsupported manifest string escape `{escaped}`"),
                });
            }
        };
        chars.push(DecodedChar {
            value,
            source_line: escape_line,
            escaped: true,
        });
    }

    if form.is_multiline() && closing_quotes >= 4 {
        let closing_line = opening_line + raw.bytes().filter(|byte| *byte == b'\n').count();
        for _ in 0..closing_quotes - 3 {
            chars.push(DecodedChar {
                value: form.quote(),
                source_line: closing_line,
                escaped: false,
            });
        }
    }
    Ok(DecodedString { chars })
}

fn decode_unicode_escape(
    content: &str,
    offset: &mut usize,
    line: &mut usize,
    escape_line: usize,
    width: usize,
) -> Result<char, SyntaxError> {
    let start = *offset;
    let mut digits = String::with_capacity(width);
    for _ in 0..width {
        let Some(ch) = content[*offset..].chars().next() else {
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
                line: *line,
                message: "invalid hexadecimal digit in Unicode escape".to_string(),
            });
        }
        digits.push(ch);
        *offset += ch.len_utf8();
    }
    let codepoint = u32::from_str_radix(&digits, 16).expect("Unicode digits were validated");
    char::from_u32(codepoint).ok_or_else(|| SyntaxError {
        line: escape_line,
        message: format!("Unicode escape is not a scalar value at byte {start}"),
    })
}

fn consume_physical_newline(content: &str, offset: &mut usize, line: &mut usize) {
    if content[*offset..].starts_with("\r\n") {
        *offset += 2;
    } else {
        *offset += 1;
    }
    *line += 1;
}

fn is_prohibited_control(ch: char) -> bool {
    matches!(ch, '\u{0000}'..='\u{0008}' | '\u{000a}'..='\u{001f}' | '\u{007f}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_scan_provenance_covers_toml_and_nested_json_string_tokens() {
        let source = r#"value = ["\n", '\n', {"json":"\u000A", "nested":["\\n"]}]
physical = """
line
break"""
# "ignored\r"
"#;
        let tokens = Lexer::new(Path::new("case.toml"), source).lex();
        let strings = tokens
            .iter()
            .filter_map(|token| match &token.kind {
                TokenKind::String(string) => Some(string),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            strings
                .iter()
                .map(|string| string.source)
                .collect::<Vec<_>>(),
            [
                r#""\n""#,
                r#"'\n'"#,
                r#""json""#,
                r#""\u000A""#,
                r#""nested""#,
                r#""\\n""#,
                "\"\"\"\nline\nbreak\"\"\"",
            ]
        );
        assert_eq!(strings[0].decoded.as_ref().unwrap().text(), "\n");
        assert_eq!(strings[1].decoded.as_ref().unwrap().text(), r#"\n"#);
        assert_eq!(strings[3].decoded.as_ref().unwrap().text(), "\n");
        assert_eq!(strings[5].decoded.as_ref().unwrap().text(), r#"\n"#);

        let physical = strings[6].decoded.as_ref().unwrap();
        assert_eq!(physical.text(), "line\nbreak");
        assert_eq!(
            physical
                .chars
                .iter()
                .map(|decoded| (decoded.value, decoded.source_line))
                .collect::<Vec<_>>(),
            [
                ('l', 3),
                ('i', 3),
                ('n', 3),
                ('e', 3),
                ('\n', 3),
                ('b', 4),
                ('r', 4),
                ('e', 4),
                ('a', 4),
                ('k', 4),
            ]
        );
    }

    #[test]
    fn policy_scan_provenance_retains_escape_lines_and_local_decode_errors() {
        let source = "first = \"\"\"\nphysical\n\\u000A\"\"\"\ninvalid = \"bad\\q\"\n";
        let strings = Lexer::new(Path::new("case.toml"), source)
            .lex()
            .into_iter()
            .filter_map(|token| match token.kind {
                TokenKind::String(string) => Some(string),
                _ => None,
            })
            .collect::<Vec<_>>();

        let decoded = strings[0].decoded.as_ref().unwrap();
        assert_eq!(decoded.text(), "physical\n\n");
        assert_eq!(
            decoded
                .chars
                .iter()
                .filter(|decoded| decoded.value == '\n')
                .map(|decoded| decoded.source_line)
                .collect::<Vec<_>>(),
            [2, 3]
        );

        assert_eq!(strings[1].source, r#""bad\q""#);
        let error = strings[1].decoded.as_ref().unwrap_err();
        assert_eq!(error.line, 4);
        assert_eq!(error.message, "unsupported manifest string escape `q`");
    }
}
