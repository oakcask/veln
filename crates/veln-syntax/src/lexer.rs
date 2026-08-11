use veln_literals::{IntegerLiteralError, parse_integer_literal};
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
    if first == '0'
        && chars
            .peek()
            .is_some_and(|(_, next)| matches!(*next, 'b' | 'B' | 'x' | 'X'))
    {
        return read_prefixed_integer_candidate(text, start, first, chars);
    }

    read_decimal_number(text, start, first, chars)
}

fn read_prefixed_integer_candidate(
    text: &str,
    start: usize,
    first: char,
    chars: &mut CharIter<'_>,
) -> Token {
    let mut end = start + first.len_utf8();
    consume_next(chars, &mut end);
    consume_while(chars, &mut end, |next| {
        next.is_ascii_alphanumeric() || next == '_'
    });
    consume_fraction(chars, &mut end);

    let candidate = &text[start..end];
    let kind = match parse_integer_literal(candidate) {
        Ok(_) | Err(IntegerLiteralError::OutOfRange { .. }) => TokenKind::Int,
        Err(_) => TokenKind::MalformedInt,
    };
    token(kind, candidate, start, end)
}

fn read_decimal_number(text: &str, start: usize, first: char, chars: &mut CharIter<'_>) -> Token {
    let mut end = start + first.len_utf8();
    consume_while(chars, &mut end, |next| next.is_ascii_digit());
    let kind = if consume_fraction(chars, &mut end) {
        TokenKind::Float
    } else {
        TokenKind::Int
    };
    token(kind, &text[start..end], start, end)
}

fn consume_fraction(chars: &mut CharIter<'_>, end: &mut usize) -> bool {
    if !chars.peek().is_some_and(|(_, next)| *next == '.') {
        return false;
    }

    let mut lookahead = chars.clone();
    lookahead.next();
    if !lookahead
        .peek()
        .is_some_and(|(_, next)| next.is_ascii_digit())
    {
        return false;
    }

    consume_next(chars, end);
    consume_while(chars, end, |next| next.is_ascii_digit());
    true
}

fn consume_while(chars: &mut CharIter<'_>, end: &mut usize, predicate: impl Fn(char) -> bool) {
    while chars.peek().is_some_and(|(_, next)| predicate(*next)) {
        consume_next(chars, end);
    }
}

fn consume_next(chars: &mut CharIter<'_>, end: &mut usize) {
    let (index, next) = chars
        .next()
        .expect("peeked character must remain available");
    *end = index + next.len_utf8();
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
    TokenKind::KEYWORDS
        .iter()
        .copied()
        .find(|kind| kind.label() == text)
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
    let Some((kind, spelling)) = symbol_kind(ch, chars) else {
        return token(
            TokenKind::Invalid,
            ch.to_string(),
            start,
            start + ch.len_utf8(),
        );
    };
    for _ in ch.len_utf8()..spelling.len() {
        chars.next();
    }
    token(kind, spelling, start, start + spelling.len())
}

fn symbol_kind(ch: char, chars: &mut CharIter<'_>) -> Option<(TokenKind, &'static str)> {
    TokenKind::PUNCTUATION
        .iter()
        .copied()
        .filter_map(|kind| {
            let spelling = kind.label();
            if !spelling.starts_with(ch) || !matches_symbol_tail(spelling, ch, chars.clone()) {
                return None;
            }
            Some((kind, spelling))
        })
        .max_by_key(|(_, spelling)| spelling.len())
}

fn matches_symbol_tail(spelling: &str, first: char, mut chars: CharIter<'_>) -> bool {
    for expected in spelling[first.len_utf8()..].chars() {
        if !chars.next().is_some_and(|(_, actual)| actual == expected) {
            return false;
        }
    }
    true
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

#[cfg(test)]
mod tests {
    use veln_source::SourceFile;

    use crate::{TokenKind, lex};

    #[test]
    fn public_keyword_table_matches_lexer_keyword_mapping() {
        for kind in TokenKind::KEYWORDS {
            let token = lex_single(kind.label());
            assert_eq!(
                token,
                *kind,
                "keyword `{}` has wrong token kind",
                kind.label()
            );
        }
    }

    #[test]
    fn public_punctuation_table_matches_lexer_symbol_mapping() {
        for kind in TokenKind::PUNCTUATION {
            let token = lex_single(kind.label());
            assert_eq!(
                token,
                *kind,
                "symbol `{}` has wrong token kind",
                kind.label()
            );
        }
    }

    fn lex_single(spelling: &str) -> TokenKind {
        let source = SourceFile::new("token.veln", spelling);
        let lexed = lex(&source);
        assert_eq!(lexed.tokens.len(), 2);
        lexed.tokens[0].kind
    }
}
