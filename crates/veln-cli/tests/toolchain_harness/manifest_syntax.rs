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

#[path = "manifest_syntax/document.rs"]
mod document;
#[path = "manifest_syntax/lexer.rs"]
mod lexer;
#[path = "manifest_syntax/strings.rs"]
mod strings;

use document::DocumentParser;
use lexer::Lexer;
use strings::{decode_toml_string, is_prohibited_control};

#[cfg(test)]
#[path = "manifest_syntax/tests.rs"]
mod tests;
