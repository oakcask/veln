use veln_source::{SourceFile, TextRange};

use crate::{Lexed, Token, TokenKind};

pub fn lex(source: &SourceFile) -> Lexed {
    let text = source.text();
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        match ch {
            ' ' | '\t' | '\r' => tokens.push(read_whitespace(text, start, ch, &mut chars)),
            '\n' => tokens.push(token(TokenKind::Newline, "\n", start, start + 1)),
            '/' if chars.peek().is_some_and(|(_, next)| *next == '/') => {
                tokens.push(read_comment(text, start, &mut chars));
            }
            '"' => tokens.push(read_string(text, start, &mut chars)),
            '0'..='9' => tokens.push(read_number(text, start, ch, &mut chars)),
            'A'..='Z' | 'a'..='z' => {
                tokens.push(read_ident_or_keyword(text, start, ch, &mut chars))
            }
            '_' => tokens.push(read_underscore_or_ident(text, start, &mut chars)),
            '(' => tokens.push(token(TokenKind::LParen, "(", start, start + 1)),
            ')' => tokens.push(token(TokenKind::RParen, ")", start, start + 1)),
            '[' => tokens.push(token(TokenKind::LBracket, "[", start, start + 1)),
            ']' => tokens.push(token(TokenKind::RBracket, "]", start, start + 1)),
            '{' => tokens.push(token(TokenKind::LBrace, "{", start, start + 1)),
            '}' => tokens.push(token(TokenKind::RBrace, "}", start, start + 1)),
            ',' => tokens.push(token(TokenKind::Comma, ",", start, start + 1)),
            '.' => tokens.push(token(TokenKind::Dot, ".", start, start + 1)),
            ':' if chars.peek().is_some_and(|(_, next)| *next == ':') => {
                chars.next();
                tokens.push(token(TokenKind::DoubleColon, "::", start, start + 2));
            }
            ':' => tokens.push(token(TokenKind::Colon, ":", start, start + 1)),
            '-' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::Arrow, "->", start, start + 2));
            }
            '-' => tokens.push(token(TokenKind::Minus, "-", start, start + 1)),
            '=' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::FatArrow, "=>", start, start + 2));
            }
            '=' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::EqualEqual, "==", start, start + 2));
            }
            '=' => tokens.push(token(TokenKind::Equal, "=", start, start + 1)),
            '!' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::BangEqual, "!=", start, start + 2));
            }
            '<' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::LessEqual, "<=", start, start + 2));
            }
            '<' => tokens.push(token(TokenKind::Less, "<", start, start + 1)),
            '>' if chars.peek().is_some_and(|(_, next)| *next == '=') => {
                chars.next();
                tokens.push(token(TokenKind::GreaterEqual, ">=", start, start + 2));
            }
            '>' => tokens.push(token(TokenKind::Greater, ">", start, start + 1)),
            '|' if chars.peek().is_some_and(|(_, next)| *next == '>') => {
                chars.next();
                tokens.push(token(TokenKind::PipeGreater, "|>", start, start + 2));
            }
            '?' => tokens.push(token(TokenKind::Question, "?", start, start + 1)),
            '+' => tokens.push(token(TokenKind::Plus, "+", start, start + 1)),
            '*' => tokens.push(token(TokenKind::Star, "*", start, start + 1)),
            '/' => tokens.push(token(TokenKind::Slash, "/", start, start + 1)),
            _ => tokens.push(token(
                TokenKind::Invalid,
                ch.to_string(),
                start,
                start + ch.len_utf8(),
            )),
        }
    }

    tokens.push(Token::eof(source.len()));
    Lexed { tokens }
}
fn read_whitespace(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    while let Some((index, next)) = chars.peek().copied() {
        if matches!(next, ' ' | '\t' | '\r') {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    Token {
        kind: TokenKind::Whitespace,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_comment(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start;
    while let Some((index, next)) = chars.peek().copied() {
        if next == '\n' {
            break;
        }
        chars.next();
        end = index + next.len_utf8();
    }
    Token {
        kind: TokenKind::Comment,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_string(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + 1;
    let mut escaped = false;
    for (index, ch) in chars.by_ref() {
        end = index + ch.len_utf8();
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            break;
        }
    }
    Token {
        kind: TokenKind::String,
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_number(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    let mut is_float = false;
    while let Some((index, next)) = chars.peek().copied() {
        if next.is_ascii_digit() {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    if chars.peek().is_some_and(|(_, next)| *next == '.') {
        let mut lookahead = chars.clone();
        lookahead.next();
        if lookahead
            .peek()
            .is_some_and(|(_, next)| next.is_ascii_digit())
        {
            is_float = true;
            chars.next();
            end += 1;
            while let Some((index, next)) = chars.peek().copied() {
                if next.is_ascii_digit() {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
        }
    }
    Token {
        kind: if is_float {
            TokenKind::Float
        } else {
            TokenKind::Int
        },
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_ident_or_keyword(
    text: &str,
    start: usize,
    first: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + first.len_utf8();
    while let Some((index, next)) = chars.peek().copied() {
        if is_ident_continue(next) {
            chars.next();
            end = index + next.len_utf8();
        } else {
            break;
        }
    }
    let token_text = &text[start..end];
    let kind = match token_text {
        "pub" => TokenKind::Pub,
        "fn" => TokenKind::Fn,
        "test" => TokenKind::Test,
        "effects" => TokenKind::Effects,
        "let" => TokenKind::Let,
        "end" => TokenKind::End,
        "require" => TokenKind::Require,
        "ensure" => TokenKind::Ensure,
        "invariant" => TokenKind::Invariant,
        "mod" => TokenKind::Mod,
        "use" => TokenKind::Use,
        "match" => TokenKind::Match,
        "or" => TokenKind::Or,
        "and" => TokenKind::And,
        "not" => TokenKind::Not,
        _ => TokenKind::Ident,
    };
    Token {
        kind,
        text: token_text.to_string(),
        range: TextRange::new(start, end),
    }
}

fn read_underscore_or_ident(
    text: &str,
    start: usize,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Token {
    let mut end = start + 1;
    let mut has_suffix = false;
    while let Some((index, next)) = chars.peek().copied() {
        if is_ident_continue(next) {
            chars.next();
            end = index + next.len_utf8();
            has_suffix = true;
        } else {
            break;
        }
    }
    Token {
        kind: if has_suffix {
            TokenKind::Hole
        } else {
            TokenKind::Underscore
        },
        text: text[start..end].to_string(),
        range: TextRange::new(start, end),
    }
}

fn token(kind: TokenKind, text: impl Into<String>, start: usize, end: usize) -> Token {
    Token {
        kind,
        text: text.into(),
        range: TextRange::new(start, end),
    }
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
