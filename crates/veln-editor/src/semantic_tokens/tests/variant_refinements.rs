use super::super::*;
use super::collect_text;

#[test]
fn collector_projects_variant_refinements_across_type_positions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Envelope\n",
            "  Wrapped(field: model::State::Ready | model::State::Closed)\n",
            "end\n",
            "schema Packet\n",
            "  state: wire::State::Ready | wire::State::Closed\n",
            "end\n",
            "effect Transition\n",
            "  move(value: api::State::Ready) -> api::State::Closed\n",
            "end\n",
            "handler gate(state: alias::State::Ready | alias::State::Closed) for Transition\n",
            "  move(value) => sink<alias::State::Ready | alias::State::Closed>(value)\n",
            "end\n",
            "fn transition(domain: Int, current: domain::State::Ready | domain::State::Closed, nested: Box<domain::State::Ready>, callback: fn(domain::State::Ready) -> domain::State::Closed, record: {state: domain::State::Ready | domain::State::Closed}) -> api::State::Ready\n",
            "  let local: alias::State::Ready | alias::State::Closed = current\n",
            "  sink<domain::State::Ready | domain::State::Closed>(local)\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    for base_segment in ["model", "wire", "api", "alias", "domain", "State"] {
        let matching = tokens
            .iter()
            .filter(|(text, _, _)| text == base_segment)
            .collect::<Vec<_>>();
        assert!(
            !matching.is_empty(),
            "missing base segment `{base_segment}`"
        );
        let base_occurrences = matching
            .iter()
            .filter(|(_, token_type, _)| *token_type == SemanticTokenType::Type)
            .collect::<Vec<_>>();
        let expected_base_count = matching.len() - usize::from(base_segment == "domain");
        assert_eq!(
            base_occurrences.len(),
            expected_base_count,
            "unexpected base classification for `{base_segment}`: {matching:?}"
        );
        assert!(
            base_occurrences
                .iter()
                .all(|(_, _, modifiers)| { *modifiers == SemanticTokenModifiers::empty().bits() }),
            "unexpected base classification for `{base_segment}`: {matching:?}"
        );
    }
    for variant in ["Ready", "Closed"] {
        let matching = tokens
            .iter()
            .filter(|(text, _, _)| text == variant)
            .collect::<Vec<_>>();
        assert!(!matching.is_empty(), "missing variant segment `{variant}`");
        assert!(matching.iter().all(|(_, token_type, modifiers)| {
            *token_type == SemanticTokenType::EnumMember
                && *modifiers == SemanticTokenModifiers::empty().bits()
        }));
    }
    let pipes = tokens
        .iter()
        .filter(|(text, _, _)| text == "|")
        .collect::<Vec<_>>();
    assert!(!pipes.is_empty());
    assert!(pipes.iter().all(|(_, token_type, modifiers)| {
        *token_type == SemanticTokenType::Operator
            && *modifiers == SemanticTokenModifiers::empty().bits()
    }));
}

#[test]
fn collector_keeps_constructor_expressions_distinct_from_refinement_variants() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn build(value: State::Ready) -> State::Ready\n",
            "  State::Ready(value)\n",
            "end\n",
        ),
    );

    let ready_tokens = collect_text(&source)
        .into_iter()
        .filter(|(text, _, _)| text == "Ready")
        .collect::<Vec<_>>();

    assert_eq!(ready_tokens.len(), 3);
    assert_eq!(ready_tokens[0].1, SemanticTokenType::EnumMember);
    assert_eq!(ready_tokens[1].1, SemanticTokenType::EnumMember);
    assert_ne!(ready_tokens[2].1, SemanticTokenType::EnumMember);
    assert!(
        ready_tokens
            .iter()
            .all(|(_, _, modifiers)| { *modifiers == SemanticTokenModifiers::empty().bits() })
    );
}
