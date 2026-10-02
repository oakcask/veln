use super::*;

#[test]
fn parses_variant_refinements_across_nested_type_forms() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Envelope\n",
            "  Wrapped(value: protocol::Result<List<domain::Item>, Wrapper<State::Ready | State::Closed>>::Ok | protocol::Result<List<Int>, DecodeError>::Err)\n",
            "end\n",
            "fn transition(state: protocol::Connection::Connected | protocol::Connection::Closed, nested: List<Connection::Connected | Connection::Closed>, callback: fn(Connection::Connected | Connection::Closed) -> Connection::Disconnected, record: {state: Connection::Connected | Connection::Closed}) -> Result<Int, DecodeError>::Ok\n",
            "  let current: Connection::Connected | Connection::Connected = state\n",
            "  current\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let SyntaxItem::Type(envelope) = &output.tree.items[0] else {
        panic!("expected type declaration");
    };
    let payload_union = &envelope.variants[0].fields[0].ty_refinements[0];
    assert_eq!(payload_union.alternatives.len(), 2);
    let ok = &payload_union.alternatives[0];
    assert_eq!(ok.base.segments, ["protocol", "Result"]);
    assert_eq!(ok.type_arguments.len(), 2);
    assert_eq!(ok.type_arguments[0].ty, "List<domain::Item>");
    assert_eq!(
        ok.type_arguments[0].ty_paths[0].segments,
        ["domain", "Item"]
    );
    assert_eq!(
        ok.type_arguments[1].ty,
        "Wrapper<State::Ready | State::Closed>"
    );
    assert_eq!(ok.type_arguments[1].ty_refinements.len(), 1);
    assert_eq!(
        ok.type_arguments[1].ty_refinements[0].alternatives[0].variant,
        "Ready"
    );
    assert_eq!(
        ok.type_arguments[1].ty_refinements[0].alternatives[1].variant,
        "Closed"
    );
    assert_eq!(ok.variant, "Ok");
    assert_eq!(
        &source.text()[ok.variant_span.start.offset..ok.variant_span.end.offset],
        "Ok"
    );
    assert_eq!(
        &source.text()
            [payload_union.pipe_spans[0].start.offset..payload_union.pipe_spans[0].end.offset],
        "|"
    );

    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected function declaration");
    };
    assert_eq!(function.params[0].ty_refinements[0].alternatives.len(), 2);
    assert_eq!(function.params[1].ty_refinements[0].alternatives.len(), 2);
    assert_eq!(function.params[2].ty_refinements.len(), 2);
    assert_eq!(function.params[3].ty_refinements[0].alternatives.len(), 2);
    assert_eq!(
        function.return_type_refinements[0].alternatives[0].variant,
        "Ok"
    );
    let BodyLine::Let {
        annotation_refinements,
        ..
    } = &function.body[0]
    else {
        panic!("expected annotated let");
    };
    assert_eq!(annotation_refinements[0].alternatives.len(), 2);
    assert_eq!(
        annotation_refinements[0].alternatives[0].variant,
        "Connected"
    );
    assert_eq!(
        annotation_refinements[0].alternatives[1].variant,
        "Connected"
    );
}

#[test]
fn rejects_malformed_variant_refinement_forms() {
    let cases = [
        "Connection::Connected | Int",
        "Int | Connection::Connected",
        "Connection::Connected |",
        "| Connection::Connected",
        "Connection::Connected || Connection::Closed",
        "Connection::",
        "::Connected",
        "Connection::Connected<Int>",
        "Connection::Connected<Int> | Connection::Closed",
        "Result<>::Ok",
        "Result<Int> Ok",
        "Result<Int>>::Ok",
        "Result<Int>>::Ok | Result<Int>>::Err",
        "List<Connection::>",
        "{state: Connection::}",
        "fn(Connection::) -> ()",
        "State<, Int>::Ready",
        "State<Int,, Error>::Ready",
        "State<Int,>::Ready",
        "Result<Int>::Ok<Error>",
        "Result<Int>::Ok::Bad",
    ];

    for annotation in cases {
        let source = SourceFile::new(
            "main.veln",
            format!("fn invalid(value: {annotation}) -> ()\n  ()\nend\n"),
        );
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.variant_refinement_type"),
            "expected variant refinement diagnostic for `{annotation}`: {:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn formats_variant_unions_idempotently_without_reordering_or_deduplicating() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn preserve(value: Alias::Ready|protocol::State::Closed |Alias::Ready) -> List<State::Open| State::Closed>\n",
            "  let nested: {callback: fn(State::Open|State::Closed) -> State::Open} = value\n",
            "  nested\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);

    let formatted = format_tree(&parsed.tree);
    assert!(formatted.contains("Alias::Ready | protocol::State::Closed | Alias::Ready"));
    assert!(formatted.contains("List<State::Open | State::Closed>"));
    assert!(formatted.contains("fn(State::Open | State::Closed) -> State::Open"));

    let reparsed = parse(&SourceFile::new("main.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn parses_variant_refinements_in_explicit_call_type_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn invoke(value: State) -> ()\n",
            "  sink<State::Ready | State::Closed, Result<Int, Error>::Ok>(value)\n",
            "end\n",
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call");
    };
    let ExprKind::TypeApply {
        type_arg_refinements,
        ..
    } = &callee.kind
    else {
        panic!("expected explicit type application");
    };
    assert_eq!(type_arg_refinements.len(), 2);
    assert_eq!(type_arg_refinements[0][0].alternatives.len(), 2);
    assert_eq!(type_arg_refinements[1][0].alternatives[0].variant, "Ok");
}

#[test]
fn parses_deeply_nested_variant_refinements_with_linear_structure() {
    const DEPTH: usize = 256;
    let mut annotation = "Leaf::Value".to_string();
    for _ in 0..DEPTH {
        annotation = format!("Layer<{annotation}>::Wrapped");
    }
    let source = SourceFile::new(
        "main.veln",
        format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n"),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(function.params[0].ty_refinements.len(), DEPTH + 1);
    assert_eq!(
        function.params[0].ty_refinements[0].alternatives[0].type_arguments[0]
            .ty_refinements
            .len(),
        DEPTH
    );
    let argument_count = function.params[0]
        .ty_refinements
        .iter()
        .flat_map(|refinement| &refinement.alternatives)
        .map(|alternative| alternative.type_arguments.len())
        .sum::<usize>();
    assert_eq!(argument_count, DEPTH);
}

#[test]
fn parses_large_variant_union_in_written_order() {
    const ALTERNATIVES: usize = 2_048;
    let annotation = (0..ALTERNATIVES)
        .map(|index| format!("State::Variant{index}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let source = SourceFile::new(
        "main.veln",
        format!("fn many(value: {annotation}) -> ()\n  ()\nend\n"),
    );

    crate::parser::reset_refinement_grouping_candidate_lookups();
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let alternatives = &first_function(&output).params[0].ty_refinements[0].alternatives;
    assert_eq!(alternatives.len(), ALTERNATIVES);
    assert_eq!(alternatives[0].variant, "Variant0");
    assert_eq!(
        alternatives[ALTERNATIVES - 1].variant,
        format!("Variant{}", ALTERNATIVES - 1)
    );
    assert!(
        crate::parser::refinement_grouping_candidate_lookups() <= ALTERNATIVES,
        "union grouping must perform at most one candidate lookup per alternative"
    );
}

#[test]
fn parses_exact_fused_generic_closers_before_a_variant() {
    let source = SourceFile::new(
        "main.veln",
        "fn exact(value: Result<List<Int>>::Ok) -> ()\n  ()\nend\n",
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let alternative = &first_function(&output).params[0].ty_refinements[0].alternatives[0];
    assert_eq!(alternative.base.segments, ["Result"]);
    assert_eq!(alternative.type_arguments.len(), 1);
    assert_eq!(
        &source.text()[alternative.type_arguments[0].span.start.offset
            ..alternative.type_arguments[0].span.end.offset],
        "List<Int>"
    );
}
