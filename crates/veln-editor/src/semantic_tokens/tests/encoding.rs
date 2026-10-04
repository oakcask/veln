use super::super::*;

#[test]
fn lsp_encoding_uses_relative_positions_and_legend_indices() {
    let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");
    let semantic_tokens = collect_semantic_tokens(&source);
    let function_tokens = semantic_tokens
        .into_iter()
        .filter(|token| token.kind.token_type == SemanticTokenType::Function)
        .collect::<Vec<_>>();

    let encoded = encode_lsp_semantic_tokens(&source, &function_tokens);

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

    let encoded = encode_lsp_semantic_tokens(&source, &[inner, outer]);

    assert_eq!(encoded.len(), 1);
    assert_eq!(encoded[0].delta_start, 0);
    assert_eq!(encoded[0].length, 7);
}

#[test]
fn lsp_encoding_uses_utf16_code_units_for_non_bmp_text() {
    let source = SourceFile::new("main.veln", "😀callsite\n");
    let emoji = SemanticToken {
        span: source.span(veln_source::TextRange::new(0, 4)),
        kind: SemanticTokenKind {
            token_type: SemanticTokenType::String,
        },
        modifiers: SemanticTokenModifiers::empty(),
    };
    let callsite = SemanticToken {
        span: source.span(veln_source::TextRange::new(4, 12)),
        kind: SemanticTokenKind {
            token_type: SemanticTokenType::Variable,
        },
        modifiers: SemanticTokenModifiers::empty(),
    };

    let encoded = encode_lsp_semantic_tokens(&source, &[emoji, callsite]);

    assert_eq!(encoded[0].delta_start, 0);
    assert_eq!(encoded[0].length, 2);
    assert_eq!(encoded[1].delta_start, 2);
    assert_eq!(encoded[1].length, 8);
}

#[test]
fn callsite_semantic_token_collection_handles_adjacent_input_sizes() {
    for function_count in [1_000, 2_000] {
        let mut text = String::new();
        for index in 0..function_count {
            text.push_str(&format!(
                "fn located{index}() -> SourceLocation callsite\n  callsite\nend\n"
            ));
        }
        let source = SourceFile::new("many.veln", text);
        let started = std::time::Instant::now();
        let tokens = collect_semantic_tokens(&source);
        let elapsed = started.elapsed();
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.kind.token_type == SemanticTokenType::Keyword)
                .count(),
            function_count * 3
        );
        eprintln!(
            "callsite semantic tokens: functions={function_count} tokens={} elapsed={elapsed:?}",
            tokens.len()
        );
    }
}

#[test]
fn callsite_scope_cursor_work_is_linear_across_adjacent_sizes() {
    let visits = [1_000, 2_000].map(|scope_count| {
        let scopes = (0..scope_count)
            .map(|index| (index * 10, index * 10 + 9))
            .collect::<Vec<_>>();
        let mut cursor = CallsiteScopeCursor::new(&scopes);
        for index in 0..scope_count {
            assert!(cursor.contains(index * 10 + 1, index * 10 + 2));
        }
        cursor.visits()
    });

    assert_eq!(visits, [2_999, 5_999]);
}
