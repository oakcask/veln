use std::collections::HashMap;
use veln_literals::{FloatLiteralError, parse_float_literal};

use super::*;

pub(super) fn validated_float_literals(
    tokens: &[Token],
) -> HashMap<(usize, usize), Result<u64, FloatLiteralError>> {
    tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Float)
        .map(|token| {
            (
                (token.range.start, token.range.end),
                parse_float_literal(&token.text).map(|literal| literal.value.to_bits()),
            )
        })
        .collect()
}

pub(super) fn float_literal_diagnostics(
    source: &SourceFile,
    tokens: &[Token],
    values: &HashMap<(usize, usize), Result<u64, FloatLiteralError>>,
) -> Vec<ParseDiagnostic> {
    tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Float)
        .filter_map(|token| {
            let error = values[&(token.range.start, token.range.end)].err()?;
            let (message, expected) = match error {
                FloatLiteralError::Overflow => (
                    "Float literal is too large to round to a finite Float value",
                    vec!["finite Float value"],
                ),
                FloatLiteralError::Underflow => (
                    "nonzero Float literal is too small to round to a nonzero Float value",
                    vec!["nonzero Float value"],
                ),
                FloatLiteralError::InvalidSyntax => return None,
            };
            Some(ParseDiagnostic {
                id: "parse.float_literal",
                message: message.to_string(),
                span: Some(source.span(token.range)),
                parser_context: "float_literal",
                unexpected: UnexpectedToken {
                    kind: token.kind.label().to_string(),
                    text: token.text.clone(),
                },
                expected,
                recovery: Recovery {
                    strategy: RecoveryStrategy::None,
                    anchor: None,
                    dropped_token_count: 0,
                },
                repair_candidates: Vec::new(),
            })
        })
        .collect()
}
