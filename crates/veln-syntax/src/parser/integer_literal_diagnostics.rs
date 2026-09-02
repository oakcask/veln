use veln_literals::IntegerLiteralError;

use super::*;

pub(super) fn integer_literal_diagnostics(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<ParseDiagnostic> {
    tokens
        .iter()
        .filter(|token| matches!(token.kind, TokenKind::Int | TokenKind::MalformedInt))
        .filter_map(|token| {
            let error = parse_integer_literal(&token.text).err()?;
            let (message, range, expected) = integer_literal_error_details(token, error);
            let (strategy, dropped_token_count) =
                if matches!(error, IntegerLiteralError::OutOfRange { .. }) {
                    (RecoveryStrategy::None, 0)
                } else {
                    (RecoveryStrategy::SkipToken, 1)
                };
            Some(ParseDiagnostic {
                id: "parse.integer_literal",
                message,
                span: Some(source.span(range)),
                parser_context: "integer_literal",
                unexpected: UnexpectedToken {
                    kind: token.kind.label().to_string(),
                    text: token.text.clone(),
                },
                expected,
                recovery: Recovery {
                    strategy,
                    anchor: None,
                    dropped_token_count,
                },
                repair_candidates: Vec::new(),
            })
        })
        .collect()
}

fn integer_literal_error_details(
    token: &Token,
    error: IntegerLiteralError,
) -> (String, TextRange, Vec<&'static str>) {
    match error {
        IntegerLiteralError::MissingDigits { radix } => (
            format!(
                "{} integer literal requires at least one digit",
                radix.name()
            ),
            token.range,
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::UnsupportedUppercasePrefix { radix } => (
            format!(
                "uppercase {} integer literal prefix is unsupported",
                radix.name()
            ),
            literal_error_character_range(token, 1),
            vec![match radix {
                veln_literals::IntegerRadix::Binary => "lowercase `0b` prefix",
                veln_literals::IntegerRadix::Hexadecimal => "lowercase `0x` prefix",
                veln_literals::IntegerRadix::Decimal => "decimal integer",
            }],
        ),
        IntegerLiteralError::InvalidDigit {
            radix,
            byte_offset,
            character,
        } => (
            format!(
                "`{character}` is not a valid {} integer digit",
                radix.name()
            ),
            literal_error_character_range(token, byte_offset),
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::Separator { radix, byte_offset } => (
            format!(
                "digit separators are not supported in {} integer literals",
                radix.name()
            ),
            literal_error_character_range(token, byte_offset),
            vec![radix.accepted_digits()],
        ),
        IntegerLiteralError::PrefixedFloat { radix, .. } => (
            format!("{} floating-point literals are unsupported", radix.name()),
            token.range,
            vec!["integer literal"],
        ),
        IntegerLiteralError::OutOfRange { radix } => (
            format!(
                "{} integer literal exceeds the maximum Int value {}",
                radix.name(),
                i64::MAX
            ),
            token.range,
            vec!["Int value at or below 9223372036854775807"],
        ),
    }
}

fn literal_error_character_range(token: &Token, byte_offset: usize) -> TextRange {
    let start = token.range.start + byte_offset;
    let length = token.text[byte_offset..]
        .chars()
        .next()
        .map_or(0, char::len_utf8);
    TextRange::new(start, start + length)
}
