use veln_source::{SourceFile, TextRange};

use crate::{Lexed, Token, TokenKind};

type CharIter<'a> = std::iter::Peekable<std::str::CharIndices<'a>>;

pub fn lex(source: &SourceFile) -> Lexed {
    let text = source.text();
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        match ch {
            ' ' | '\t' | '\r' => tokens.push(read_whitespace(text, start, ch, &mut chars)),
            '\n' => tokens.push(token(TokenKind::Newline, "\n", start, start + 1)),
            '#' => tokens.push(read_comment(text, start, &mut chars)),
            '"' => tokens.push(read_string(text, start, &mut chars)),
            '0'..='9' => tokens.push(read_number(text, start, ch, &mut chars)),
            'A'..='Z' | 'a'..='z' => {
                tokens.push(read_ident_or_keyword(text, start, ch, &mut chars))
            }
            '_' => tokens.push(read_underscore_or_ident(text, start, &mut chars)),
            _ => tokens.push(read_symbol_token(start, ch, &mut chars)),
        }
    }

    tokens.push(Token::eof(source.len()));
    Lexed { tokens }
}

fn read_whitespace(text: &str, start: usize, first: char, chars: &mut CharIter<'_>) -> Token {
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

fn read_comment(text: &str, start: usize, chars: &mut CharIter<'_>) -> Token {
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

fn read_string(text: &str, start: usize, chars: &mut CharIter<'_>) -> Token {
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

fn read_number(text: &str, start: usize, first: char, chars: &mut CharIter<'_>) -> Token {
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

fn read_ident_or_keyword(text: &str, start: usize, first: char, chars: &mut CharIter<'_>) -> Token {
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
    let kind = keyword_kind(token_text).unwrap_or(TokenKind::Ident);
    Token {
        kind,
        text: token_text.to_string(),
        range: TextRange::new(start, end),
    }
}

fn keyword_kind(text: &str) -> Option<TokenKind> {
    let kind = match text {
        "pub" => TokenKind::Pub,
        "fn" => TokenKind::Fn,
        "type" => TokenKind::Type,
        "schema" => TokenKind::Schema,
        "format" => TokenKind::Format,
        "test" => TokenKind::Test,
        "effects" => TokenKind::Effects,
        "let" => TokenKind::Let,
        "end" => TokenKind::End,
        "require" => TokenKind::Require,
        "ensure" => TokenKind::Ensure,
        "invariant" => TokenKind::Invariant,
        "mod" => TokenKind::Mod,
        "use" => TokenKind::Use,
        "from" => TokenKind::From,
        "match" => TokenKind::Match,
        "or" => TokenKind::Or,
        "and" => TokenKind::And,
        "not" => TokenKind::Not,
        _ => return None,
    };
    Some(kind)
}

fn read_underscore_or_ident(text: &str, start: usize, chars: &mut CharIter<'_>) -> Token {
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

fn read_symbol_token(start: usize, ch: char, chars: &mut CharIter<'_>) -> Token {
    if let Some((next, kind)) = two_char_symbol_kind(ch, chars.peek().map(|(_, next)| *next)) {
        chars.next();
        return token(
            kind,
            format!("{ch}{next}"),
            start,
            start + ch.len_utf8() + next.len_utf8(),
        );
    }

    let kind = match ch {
        '(' => TokenKind::LParen,
        ')' => TokenKind::RParen,
        '[' => TokenKind::LBracket,
        ']' => TokenKind::RBracket,
        '{' => TokenKind::LBrace,
        '}' => TokenKind::RBrace,
        ',' => TokenKind::Comma,
        '.' => TokenKind::Dot,
        ':' => TokenKind::Colon,
        '-' => TokenKind::Minus,
        '=' => TokenKind::Equal,
        '<' => TokenKind::Less,
        '>' => TokenKind::Greater,
        '?' => TokenKind::Question,
        '+' => TokenKind::Plus,
        '*' => TokenKind::Star,
        '/' => TokenKind::Slash,
        _ => TokenKind::Invalid,
    };
    token(kind, ch.to_string(), start, start + ch.len_utf8())
}

fn two_char_symbol_kind(ch: char, next: Option<char>) -> Option<(char, TokenKind)> {
    let next = next?;
    let kind = match (ch, next) {
        (':', ':') => TokenKind::DoubleColon,
        ('-', '>') => TokenKind::Arrow,
        ('=', '>') => TokenKind::FatArrow,
        ('=', '=') => TokenKind::EqualEqual,
        ('!', '=') => TokenKind::BangEqual,
        ('<', '=') => TokenKind::LessEqual,
        ('>', '=') => TokenKind::GreaterEqual,
        ('|', '>') => TokenKind::PipeGreater,
        _ => return None,
    };
    Some((next, kind))
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
