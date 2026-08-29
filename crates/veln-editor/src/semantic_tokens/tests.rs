use super::*;

fn collect_text(source: &SourceFile) -> Vec<(String, SemanticTokenType, u32)> {
    collect_semantic_tokens(source)
        .into_iter()
        .map(|token| {
            (
                source.text()[token.span.start.offset..token.span.end.offset].to_string(),
                token.kind.token_type,
                token.modifiers.bits(),
            )
        })
        .collect()
}

#[path = "tests/declarations.rs"]
mod declarations;
#[path = "tests/encoding.rs"]
mod encoding;
#[path = "tests/expressions.rs"]
mod expressions;
