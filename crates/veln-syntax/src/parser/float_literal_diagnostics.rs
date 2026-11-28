use veln_literals::{FloatLiteralError, parse_float_literal};

use super::*;

pub(super) fn float_literal_diagnostics(
    source: &SourceFile,
    tokens: &[Token],
) -> Vec<ParseDiagnostic> {
    tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Float)
        .filter_map(|token| {
            let error = parse_float_literal(&token.text).err()?;
            let (message, expected) = match error {
                FloatLiteralError::Overflow => (
                    "Float literal is too large to round to a finite Float value",
                    vec!["finite Float value"],
                ),
                FloatLiteralError::Underflow => (
                    "nonzero Float literal is too small to round to a nonzero Float value",
                    vec!["zero or nonzero Float value"],
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
