use super::super::*;

#[test]
fn lsp_encoding_uses_relative_positions_and_legend_indices() {
    let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");
    let semantic_tokens = collect_semantic_tokens(&source);
    let function_tokens = semantic_tokens
        .into_iter()
        .filter(|token| token.kind.token_type == SemanticTokenType::Function)
        .collect::<Vec<_>>();

    let encoded = encode_lsp_semantic_tokens(&function_tokens);

    assert_eq!(
        encoded,
        vec![
            LspSemanticToken {
                delta_line: 0,
                delta_start: 3,
                length: 4,
                token_type: token_type_index(SemanticTokenType::Function) as u32,
                token_modifiers: SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .bits(),
            },
            LspSemanticToken {
                delta_line: 1,
                delta_start: 2,
                length: 4,
                token_type: token_type_index(SemanticTokenType::Function) as u32,
                token_modifiers: 0,
            },
        ]
    );
}

#[test]
fn lsp_encoding_sorts_and_drops_overlapping_ranges() {
    let source = SourceFile::new("main.veln", "fn main()\nend\n");
    let outer = SemanticToken {
        span: source.span(veln_source::TextRange::new(0, 7)),
        kind: SemanticTokenKind {
            token_type: SemanticTokenType::Function,
        },
        modifiers: SemanticTokenModifiers::empty(),
    };
    let inner = SemanticToken {
        span: source.span(veln_source::TextRange::new(3, 7)),
        kind: SemanticTokenKind {
            token_type: SemanticTokenType::Function,
        },
        modifiers: SemanticTokenModifiers::empty(),
    };

    let encoded = encode_lsp_semantic_tokens(&[inner, outer]);

    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0].delta_start, 0);
    assert_eq!(encoded[0].length, 7);
}
