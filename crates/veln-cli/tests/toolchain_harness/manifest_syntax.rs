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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManifestPolicyScan {
    pub(crate) findings: Vec<ManifestPolicyFinding>,
    pub(crate) error: Option<String>,
}

struct Lexed<'a> {
    tokens: Vec<Token<'a>>,
    boundary_error: Option<SyntaxError>,
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

    fn try_collect_policy_findings(
        &self,
        field: &str,
        findings: &mut Vec<ManifestPolicyFinding>,
    ) -> Result<(), SyntaxError> {
        let mut json_depth = 0usize;
        for token in &self.tokens {
            match token.kind {
                TokenKind::Open(Delimiter::Brace) if json_depth == 0 => {
                    json_depth = 1;
                    continue;
                }
                TokenKind::Open(Delimiter::Square)
                    if json_depth == 0 && field_accepts_json_array_root(field) =>
                {
                    json_depth = 1;
                    continue;
                }
                TokenKind::Open(_) if json_depth > 0 => {
                    json_depth += 1;
                    continue;
                }
                TokenKind::Close(_) if json_depth > 0 => {
                    json_depth = json_depth.saturating_sub(1);
                    continue;
                }
                _ => {}
            }
            let TokenKind::String(string) = &token.kind else {
                continue;
            };
            let json_string = json_depth > 0;
            let decoded = if json_string {
                decode_json_string(string.source, token.line)?
            } else {
                string.decoded.as_ref().map_err(Clone::clone)?.clone()
            };
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
        Ok(())
    }
}

fn field_accepts_json_array_root(field: &str) -> bool {
    matches!(
        field,
        "[[json_assert]].equals"
            | "[[mcp_assert]].equals"
            | "[[result_value_assert]].equals"
            | "[[binary_fixture]].field_path"
    )
}

pub(crate) fn parse_document<'a>(path: &Path, text: &'a str) -> Vec<Statement<'a>> {
    let tokens = Lexer::new(path, text).lex().tokens;
    DocumentParser::new(path, text, tokens).parse()
}

pub(crate) fn manifest_policy_findings(path: &Path, text: &str) -> Vec<ManifestPolicyFinding> {
    let scan = manifest_policy_scan(path, text);
    if let Some(error) = scan.error {
        manifest_error(path, 0, error);
    }
    scan.findings
}

pub(crate) fn manifest_policy_scan(path: &Path, text: &str) -> ManifestPolicyScan {
    let lexed = Lexer::new(path, text).lex_with_boundary();
    let boundary_error = lexed.boundary_error;
    let tokens = if boundary_error.is_some() {
        completed_statement_prefix(lexed.tokens)
    } else {
        lexed.tokens
    };
    let mut section = String::new();
    let mut findings = Vec::new();
    for statement in DocumentParser::new(path, text, tokens).parse() {
        match statement {
            Statement::Section { name, .. } => section = name,
            Statement::Assignment { key, value, .. } => {
                let field = if section.is_empty() {
                    key.to_string()
                } else {
                    format!("{section}.{key}")
                };
                if let Err(error) = value.try_collect_policy_findings(&field, &mut findings) {
                    sort_policy_findings(&mut findings);
                    return ManifestPolicyScan {
                        findings,
                        error: Some(error.message),
                    };
                }
            }
        }
    }
    sort_policy_findings(&mut findings);
    if let Some(error) = boundary_error {
        return ManifestPolicyScan {
            findings,
            error: Some(error.message),
        };
    }
    ManifestPolicyScan {
        findings,
        error: None,
    }
}

fn completed_statement_prefix<'a>(tokens: Vec<Token<'a>>) -> Vec<Token<'a>> {
    let mut depth = 0usize;
    let mut end = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Open(_) => depth += 1,
            TokenKind::Close(_) => depth = depth.saturating_sub(1),
            TokenKind::Newline if depth == 0 => end = index + 1,
            _ => {}
        }
    }
    if end < tokens.len() && trailing_tokens_form_complete_statement(&tokens[end..]) {
        end = tokens.len();
    }
    tokens[..end].to_vec()
}

fn trailing_tokens_form_complete_statement(tokens: &[Token<'_>]) -> bool {
    let tokens = tokens
        .iter()
        .filter(|token| !matches!(&token.kind, TokenKind::Comment))
        .collect::<Vec<_>>();
    let Some(first) = tokens.first() else {
        return false;
    };
    if matches!(&first.kind, TokenKind::Open(Delimiter::Square)) {
        let mut depth = 0usize;
        for token in &tokens {
            match &token.kind {
                TokenKind::Open(_) => depth += 1,
                TokenKind::Close(_) => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        return depth == 0
            && matches!(
                tokens.last().map(|token| &token.kind),
                Some(TokenKind::Close(Delimiter::Square))
            );
    }
    matches!(&first.kind, TokenKind::Atom(_))
        && tokens
            .iter()
            .position(|token| matches!(&token.kind, TokenKind::Equals))
            .is_some_and(|equals| {
                let value = &tokens[equals + 1..];
                !value.is_empty() && value_tokens_are_balanced(value)
            })
}

fn value_tokens_are_balanced(tokens: &[&Token<'_>]) -> bool {
    let mut stack = Vec::new();
    for token in tokens {
        match &token.kind {
            TokenKind::Open(delimiter) => stack.push(*delimiter),
            TokenKind::Close(delimiter) if stack.pop() != Some(*delimiter) => {
                return false;
            }
            TokenKind::Close(_) => {}
            _ => {}
        }
    }
    stack.is_empty()
}

fn sort_policy_findings(findings: &mut [ManifestPolicyFinding]) {
    findings.sort_by(|left, right| {
        left.line
            .cmp(&right.line)
            .then(left.start.cmp(&right.start))
            .then(left.end.cmp(&right.end))
            .then(left.category.cmp(right.category))
            .then(left.field.cmp(&right.field))
            .then(left.spelling.cmp(&right.spelling))
    });
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
        if let Some(byte) = bytes.get(index + 1)
            && matches!(*byte, b'n' | b'r')
        {
            findings.push(text[index..index + 2].to_string());
            index += 2;
            continue;
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

fn decode_json_string(raw: &str, line: usize) -> Result<DecodedString, SyntaxError> {
    if !raw.starts_with('"') || !raw.ends_with('"') || raw.len() < 2 {
        return Err(SyntaxError {
            line,
            message: "invalid JSON string".to_string(),
        });
    }
    let content = &raw[1..raw.len() - 1];
    let mut chars = Vec::new();
    let mut offset = 0;
    while offset < content.len() {
        let ch = content[offset..]
            .chars()
            .next()
            .expect("JSON string content has a char");
        if ch == '\\' {
            offset += 1;
            let Some(escaped) = content[offset..].chars().next() else {
                return Err(SyntaxError {
                    line,
                    message: "unterminated JSON string escape".to_string(),
                });
            };
            offset += escaped.len_utf8();
            let value = match escaped {
                '"' => '"',
                '\\' => '\\',
                '/' => '/',
                'b' => '\u{08}',
                'f' => '\u{0c}',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'u' => decode_json_unicode_escape(content, &mut offset, line)?,
                _ => {
                    return Err(SyntaxError {
                        line,
                        message: format!("unsupported JSON string escape `{escaped}`"),
                    });
                }
            };
            chars.push(DecodedChar {
                value,
                source_line: line,
                escaped: true,
            });
            continue;
        }
        if is_prohibited_control(ch) {
            return Err(SyntaxError {
                line,
                message: "prohibited control character in JSON string".to_string(),
            });
        }
        chars.push(DecodedChar {
            value: ch,
            source_line: line,
            escaped: false,
        });
        offset += ch.len_utf8();
    }
    Ok(DecodedString { chars })
}

fn decode_json_unicode_escape(
    content: &str,
    offset: &mut usize,
    line: usize,
) -> Result<char, SyntaxError> {
    let start = *offset;
    let mut digits = String::with_capacity(4);
    for _ in 0..4 {
        let Some(ch) = content[*offset..].chars().next() else {
            return Err(SyntaxError {
                line,
                message: "incomplete JSON Unicode escape".to_string(),
            });
        };
        if !ch.is_ascii_hexdigit() {
            return Err(SyntaxError {
                line,
                message: "invalid hexadecimal digit in JSON Unicode escape".to_string(),
            });
        }
        digits.push(ch);
        *offset += ch.len_utf8();
    }
    let codepoint = u16::from_str_radix(&digits, 16).expect("JSON Unicode digits were validated");
    if (0xd800..=0xdbff).contains(&codepoint) {
        if !content[*offset..].starts_with("\\u") {
            return Err(SyntaxError {
                line,
                message: format!("unpaired JSON high surrogate at byte {start}"),
            });
        }
        *offset += 2;
        let low = decode_json_unicode_unit(content, offset, line)?;
        if !(0xdc00..=0xdfff).contains(&low) {
            return Err(SyntaxError {
                line,
                message: "invalid JSON surrogate pair".to_string(),
            });
        }
        let high_value = u32::from(codepoint - 0xd800);
        let low_value = u32::from(low - 0xdc00);
        let scalar = 0x10000 + ((high_value << 10) | low_value);
        return char::from_u32(scalar).ok_or_else(|| SyntaxError {
            line,
            message: "JSON Unicode escape is not a scalar value".to_string(),
        });
    }
    if (0xdc00..=0xdfff).contains(&codepoint) {
        return Err(SyntaxError {
            line,
            message: format!("unpaired JSON low surrogate at byte {start}"),
        });
    }
    char::from_u32(u32::from(codepoint)).ok_or_else(|| SyntaxError {
        line,
        message: "JSON Unicode escape is not a scalar value".to_string(),
    })
}

fn decode_json_unicode_unit(
    content: &str,
    offset: &mut usize,
    line: usize,
) -> Result<u16, SyntaxError> {
    let mut digits = String::with_capacity(4);
    for _ in 0..4 {
        let Some(ch) = content[*offset..].chars().next() else {
            return Err(SyntaxError {
                line,
                message: "incomplete JSON Unicode escape".to_string(),
            });
        };
        if !ch.is_ascii_hexdigit() {
            return Err(SyntaxError {
                line,
                message: "invalid hexadecimal digit in JSON Unicode escape".to_string(),
            });
        }
        digits.push(ch);
        *offset += ch.len_utf8();
    }
    Ok(u16::from_str_radix(&digits, 16).expect("JSON Unicode digits were validated"))
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

    fn lex(self) -> Lexed<'a> {
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

    fn lex_with_boundary(mut self) -> Lexed<'a> {
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

fn is_prohibited_control(ch: char) -> bool {
    matches!(ch, '\u{0000}'..='\u{0008}' | '\u{000a}'..='\u{001f}' | '\u{007f}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_decoder_preserves_provenance_across_continuations_and_escapes() {
        let decoded = decode_toml_string(
            "\"\"\"\nfirst\\\n  second\\u0021\r\nthird\"\"\"",
            7,
            StringForm::MultilineBasic,
            3,
        )
        .expect("multiline basic string should decode");

        assert_eq!(decoded.text(), "firstsecond!\nthird");
        assert_eq!(
            decoded
                .chars
                .iter()
                .map(|decoded| (decoded.value, decoded.source_line, decoded.escaped))
                .collect::<Vec<_>>(),
            [
                ('f', 8, false),
                ('i', 8, false),
                ('r', 8, false),
                ('s', 8, false),
                ('t', 8, false),
                ('s', 9, false),
                ('e', 9, false),
                ('c', 9, false),
                ('o', 9, false),
                ('n', 9, false),
                ('d', 9, false),
                ('!', 9, true),
                ('\n', 9, false),
                ('t', 10, false),
                ('h', 10, false),
                ('i', 10, false),
                ('r', 10, false),
                ('d', 10, false),
            ]
        );
    }

    #[test]
    fn policy_scan_provenance_covers_toml_and_nested_json_string_tokens() {
        let source = r#"value = ["\n", '\n', {"json":"\u000A", "nested":["\\n"]}]
physical = """
line
break"""
# "ignored\r"
"#;
        let tokens = Lexer::new(Path::new("case.toml"), source).lex().tokens;
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
            .tokens
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
