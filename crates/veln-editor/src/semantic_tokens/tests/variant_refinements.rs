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

#[test]
fn collector_preserves_variant_refinements_across_presentation_parse_limit() {
    let accepted = refinement_boundary_source(veln_syntax::PRESENTATION_PARSE_STRUCTURE_LIMIT - 1);
    let rejected = refinement_boundary_source(veln_syntax::PRESENTATION_PARSE_STRUCTURE_LIMIT);
    assert!(presentation_parse_structure_is_bounded(
        &lex(&accepted).tokens
    ));
    assert_physical_lines_are_bounded(&rejected);
    assert_boundary_projection_is_bounded(&rejected);

    let accepted_prefix = refinement_prefix(&accepted);
    let rejected_prefix = refinement_prefix(&rejected);
    assert_eq!(accepted_prefix, rejected_prefix);
    assert_refinement_prefix(&accepted, &accepted_prefix);
    assert_refinement_prefix(&rejected, &rejected_prefix);
}

fn assert_boundary_projection_is_bounded(source: &SourceFile) {
    let tokens = lex(source).tokens;
    assert!(!presentation_parse_structure_is_bounded(&tokens));
    let projected = bounded_variant_refinement_projection(source, &tokens)
        .expect("rejected expression line has a bounded presentation projection");
    assert!(presentation_parse_structure_is_bounded(
        &lex(&projected).tokens
    ));
}

fn assert_physical_lines_are_bounded(source: &SourceFile) {
    let tokens = lex(source).tokens;
    let mut line_start = 0;
    for (index, token) in tokens.iter().enumerate() {
        if matches!(token.kind, TokenKind::Newline | TokenKind::Eof) {
            assert!(
                presentation_parse_structure_is_bounded(&tokens[line_start..index]),
                "every physical line must remain independently bounded"
            );
            line_start = index + 1;
        }
    }
}

fn refinement_prefix(source: &SourceFile) -> Vec<SemanticToken> {
    let prefix_end = source
        .text()
        .find("  (\n")
        .expect("multiline stress expression");
    collect_semantic_tokens(source)
        .into_iter()
        .filter(|token| token.span.end.offset <= prefix_end)
        .collect()
}

fn assert_refinement_prefix(source: &SourceFile, tokens: &[SemanticToken]) {
    let expected = [
        ("domain", SemanticTokenType::Type),
        ("State", SemanticTokenType::Type),
        ("Ready", SemanticTokenType::EnumMember),
        ("|", SemanticTokenType::Operator),
        ("alias", SemanticTokenType::Type),
        ("State", SemanticTokenType::Type),
        ("Closed", SemanticTokenType::EnumMember),
        ("api", SemanticTokenType::Type),
        ("State", SemanticTokenType::Type),
        ("Ready", SemanticTokenType::EnumMember),
    ];
    let projected = tokens
        .iter()
        .filter_map(|token| {
            let text = &source.text()[token.span.start.offset..token.span.end.offset];
            expected
                .iter()
                .any(|(expected, _)| *expected == text)
                .then_some((text, token.kind.token_type, token.modifiers.bits()))
        })
        .take(expected.len())
        .collect::<Vec<_>>();
    assert_eq!(
        projected,
        expected
            .iter()
            .map(|(text, token_type)| (*text, *token_type, 0))
            .collect::<Vec<_>>()
    );

    let constructor_ready = tokens
        .iter()
        .filter(|token| &source.text()[token.span.start.offset..token.span.end.offset] == "Ready")
        .nth(2)
        .expect("constructor variant token");
    assert_ne!(
        constructor_ready.kind.token_type,
        SemanticTokenType::EnumMember
    );
    assert_eq!(constructor_ready.modifiers.bits(), 0);
}

fn refinement_boundary_source(operator_count: usize) -> SourceFile {
    let mut text = concat!(
        "fn project(value: domain::State::Ready | alias::State::Closed) -> api::State::Ready\n",
        "  domain::State::Ready(value)\n",
        "  (\n",
    )
    .to_string();
    let mut remaining = operator_count;
    while remaining > 0 {
        let line_operators = remaining.min(64);
        text.push_str("    0");
        text.push_str(&" + 0".repeat(line_operators));
        text.push('\n');
        remaining -= line_operators;
    }
    text.push_str("  )\nend\n");
    SourceFile::new("main.veln", text)
}
