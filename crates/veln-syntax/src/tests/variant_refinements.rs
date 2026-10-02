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
    assert_eq!(ok.type_arguments[0].ty_fragments, ["List<domain::Item>"]);
    assert_eq!(
        ok.type_arguments[0].ty_paths[0].segments,
        ["domain", "Item"]
    );
    assert_eq!(ok.type_arguments[1].ty_fragments, ["Wrapper<", ">"]);
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
fn parses_test_results_and_handler_parameters_as_variant_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Transition\n",
            "  move(value: State) -> State\n",
            "end\n",
            "handler gate(state: State::Ready | State::Closed) handles Transition\n",
            "  move(value) => value\n",
            "end\n",
            "test exact_result() -> Result<Int, Error>::Ok\n",
            "  Ok(1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[1] else {
        panic!("expected handler declaration");
    };
    let handler_union = &handler.params[0].ty_refinements[0];
    assert_eq!(handler_union.alternatives.len(), 2);
    assert_eq!(handler_union.alternatives[0].variant, "Ready");
    assert_eq!(handler_union.alternatives[1].variant, "Closed");
    assert_eq!(
        &source.text()
            [handler_union.pipe_spans[0].start.offset..handler_union.pipe_spans[0].end.offset],
        "|"
    );

    let SyntaxItem::Function(test) = &output.tree.items[2] else {
        panic!("expected test declaration");
    };
    assert_eq!(test.kind, FunctionKind::Test);
    let result = &test.return_type_refinements[0].alternatives[0];
    assert_eq!(result.base.segments, ["Result"]);
    assert_eq!(result.type_arguments.len(), 2);
    assert_eq!(result.variant, "Ok");
    assert_eq!(
        &source.text()[result.variant_span.start.offset..result.variant_span.end.offset],
        "Ok"
    );
}

#[test]
fn preserves_qualified_generic_types_without_a_final_variant() {
    for annotation in [
        "Prelude::Option<Int>",
        "Alias::Container<Int>",
        "alias::Container<Int>",
        "domain::Alias::Container<Int>",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("fn inspect(value: {annotation}) -> ()\n  ()\nend\n"),
        );

        let output = parse(&source);
        assert!(
            output.diagnostics.is_empty(),
            "qualified generic type `{annotation}` should remain parseable: {:#?}",
            output.diagnostics
        );
        assert!(first_function(&output).params[0].ty_refinements.is_empty());
    }
}

#[test]
fn rejects_malformed_variant_refinement_forms() {
    let cases = [
        "State::Ready |> State::Closed",
        "Connection::Connected | Int",
        "Int | Connection::Connected",
        "Connection::Connected |",
        "| Connection::Connected",
        "Connection::Connected || Connection::Closed",
        "Connection::",
        "::Connected",
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
        "State::Ready>",
        "State::Ready>>",
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
fn unmatched_generic_opener_recovers_before_the_next_function() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn broken(value: Result<Int, Error::Ok) -> ()\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn survivor() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "parse.variant_refinement_type"
                && diagnostic.message == "generic type arguments are missing a closing `>`"
        })
        .expect("the unmatched generic opener should be diagnosed");
    let span = diagnostic
        .span
        .as_ref()
        .expect("the enclosing annotation boundary should be highlighted");
    assert_eq!(&source.text()[span.start.offset..span.end.offset], ")");

    assert_eq!(output.tree.items.len(), 2, "{:#?}", output.diagnostics);
    let SyntaxItem::Function(survivor) = &output.tree.items[1] else {
        panic!("expected the following item to remain a function");
    };
    assert_eq!(survivor.name.as_deref(), Some("survivor"));
}

#[test]
fn deeply_nested_unmatched_generic_openers_recover_at_one_boundary() {
    const DEPTH: usize = 4_096;
    let annotation = format!("{}State::Ready", "Layer<".repeat(DEPTH));
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn broken(value: {annotation}) -> ()\n  ()\nend\n\nfn survivor() -> ()\n  ()\nend\n"
        ),
    );

    let output = parse(&source);
    let missing_closer_diagnostics = output
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "parse.variant_refinement_type"
                && diagnostic.message == "generic type arguments are missing a closing `>`"
        })
        .count();
    assert_eq!(missing_closer_diagnostics, 1, "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2, "{:#?}", output.diagnostics);
    let SyntaxItem::Function(survivor) = &output.tree.items[1] else {
        panic!("expected the following item to remain a function");
    };
    assert_eq!(survivor.name.as_deref(), Some("survivor"));
}

#[test]
fn unmatched_generic_opener_preserves_following_named_siblings() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn flat(first: Result<Int, second: String, third: Bool) -> ()\n",
            "  ()\n",
            "end\n",
            "fn nested(first: Wrapper<Result<Int, second: String) -> ()\n",
            "  ()\n",
            "end\n",
            "type Payload\n",
            "  Broken(first: Result<Int, second: String)\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let functions = output
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function) => Some(function),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(functions[0].params.len(), 3, "{:#?}", output.diagnostics);
    assert_eq!(functions[0].params[1].name, "second");
    assert_eq!(functions[0].params[1].ty.as_deref(), Some("String"));
    assert_eq!(functions[1].params.len(), 2, "{:#?}", output.diagnostics);
    assert_eq!(functions[1].params[1].name, "second");

    let SyntaxItem::Type(payload) = &output.tree.items[2] else {
        panic!("expected type declaration");
    };
    assert_eq!(
        payload.variants[0].fields.len(),
        2,
        "{:#?}",
        output.diagnostics
    );
    assert_eq!(payload.variants[0].fields[1].name, "second");
    assert_eq!(payload.variants[0].fields[1].ty, "String");

    assert_eq!(
        output
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "parse.variant_refinement_type"
                    && diagnostic.message == "generic type arguments are missing a closing `>`"
            })
            .count(),
        3,
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn deeply_nested_unmatched_generic_openers_preserve_a_following_parameter() {
    const DEPTH: usize = 4_096;
    let annotation = format!("{}Int", "Layer<".repeat(DEPTH));
    let source = SourceFile::new(
        "main.veln",
        format!("fn broken(first: {annotation}, survivor: String) -> ()\n  ()\nend\n"),
    );

    let output = parse(&source);
    let function = first_function(&output);
    assert_eq!(function.params.len(), 2, "{:#?}", output.diagnostics);
    assert_eq!(function.params[1].name, "survivor");
    assert_eq!(function.params[1].ty.as_deref(), Some("String"));
    assert_eq!(
        output
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
            .count(),
        1,
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_text_adjacent_to_variant_refinements() {
    let cases = [
        ("State::Ready State::Closed", "State"),
        ("State::Ready | State::Closed Int", "Int"),
        ("Int State::Ready", "Int"),
        ("List<State::Ready Int>", "Int"),
        ("List<State::Ready> Int", "Int"),
        ("fn(State::Ready Int) -> State::Closed", "Int"),
        ("{state: State::Ready Int}", "Int"),
        ("{state: State::Ready} Int", "Int"),
    ];

    for (annotation, unexpected) in cases {
        let source = SourceFile::new(
            "main.veln",
            format!("fn invalid(value: {annotation}) -> ()\n  ()\nend\n"),
        );
        let output = parse(&source);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.id == "parse.variant_refinement_type"
                    && diagnostic.message
                        == "variant refinement must be complete at its structural type position"
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected adjacent-text diagnostic for `{annotation}`: {:#?}",
                    output.diagnostics
                )
            });
        let span = diagnostic.span.as_ref().expect("diagnostic span");
        assert_eq!(
            &source.text()[span.start.offset..span.end.offset],
            unexpected,
            "{annotation}"
        );
    }
}

#[test]
fn rejects_type_arguments_after_a_structurally_complete_generic_refinement() {
    let source = SourceFile::new(
        "main.veln",
        "fn invalid(value: State<Int>::Ready<Payload>) -> ()\n  ()\nend\n",
    );

    let output = parse(&source);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.variant_refinement_type"
            && diagnostic.message
                == "variant refinement type arguments must precede the final `::Variant` segment"
    }));
}

#[test]
fn malformed_pipeline_separator_is_preserved_by_repeated_formatting() {
    let source = SourceFile::new(
        "main.veln",
        "fn invalid(value: State::Ready|>State::Closed) -> ()\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let diagnostic = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
        .expect("pipeline separator should be rejected as refinement syntax");
    assert_eq!(
        diagnostic.message,
        "`|>` cannot separate ADT variant refinement alternatives"
    );
    let span = diagnostic.span.as_ref().expect("diagnostic span");
    assert_eq!(&source.text()[span.start.offset..span.end.offset], "|>");

    let formatted = format_tree(&parsed.tree);
    assert!(
        formatted.contains("State::Ready |> State::Closed")
            && !formatted.contains("State::Ready | > State::Closed"),
        "{formatted}"
    );
    let reparsed = parse(&SourceFile::new("main.veln", formatted.clone()));
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn formatter_uses_structured_refinements_instead_of_display_text() {
    let source = SourceFile::new(
        "main.veln",
        "fn transition(state: State::Ready|State::Closed) -> ()\n  ()\nend\n",
    );
    let mut parsed = parse(&source);
    let SyntaxItem::Function(function) = &mut parsed.tree.items[0] else {
        panic!("expected function declaration");
    };
    function.params[0].ty_refinements[0].alternatives[0].variant = "ChangedInStructure".to_string();

    let formatted = format_tree(&parsed.tree);
    assert!(formatted.contains("State::ChangedInStructure | State::Closed"));
    assert!(!formatted.contains("State::Ready"));

    assert_eq!(canonical_type_text("State::Ready|Int"), "State::Ready|Int");
    assert_eq!(
        canonical_type_text("State::Ready|>State::Closed"),
        "State::Ready|>State::Closed"
    );
}

#[test]
fn rejects_lowercase_final_variants_after_generic_adt_bases() {
    let cases = [
        "fn invalid(value: State::ready) -> ()\n  ()\nend\n",
        "fn invalid(value: protocol::State::ready) -> ()\n  ()\nend\n",
        "fn invalid(value: Result<Int, Error>::ok) -> ()\n  ()\nend\n",
        "fn invalid(value: protocol::Result<Int, Error>::error) -> ()\n  ()\nend\n",
        "fn invalid(value: State) -> ()\n  sink<Result<Int, Error>::ok>(value)\nend\n",
    ];

    for text in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);
        let diagnostic = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
            .unwrap_or_else(|| {
                panic!(
                    "expected lowercase final variant diagnostic: {:#?}",
                    output.diagnostics
                )
            });

        assert_eq!(
            diagnostic.message,
            "variant refinement final segment must start with an ASCII uppercase letter"
        );
        let span = diagnostic
            .span
            .as_ref()
            .expect("diagnostic should have a span");
        let written_name = &source.text()[span.start.offset..span.end.offset];
        assert!(
            matches!(written_name, "ready" | "ok" | "error"),
            "diagnostic should point at the lowercase final segment, got `{written_name}`"
        );
    }
}

#[test]
fn rejects_refinement_nesting_beyond_the_lowering_and_wire_limit() {
    let mut annotation = "Leaf::Value".to_string();
    for _ in 0..4_096 {
        annotation = format!("Layer<{annotation}>::Wrapped");
    }
    let source = SourceFile::new(
        "main.veln",
        format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n"),
    );

    let output = parse(&source);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.variant_refinement_type"
            && diagnostic.message == "variant refinement types are nested too deeply"
    }));

    let function = first_function(&output);
    let mut refinements = function.params[0].ty_refinements.as_slice();
    let mut nesting = 0;
    while let Some(refinement) = refinements.first() {
        nesting += 1;
        refinements = refinement.alternatives[0]
            .type_arguments
            .first()
            .map_or(&[], |argument| argument.ty_refinements.as_slice());
    }
    assert!(nesting <= crate::MAX_VARIANT_REFINEMENT_NESTING + 1);
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
    assert!(
        format_tree(&output.tree)
            .contains("sink<State::Ready | State::Closed, Result<Int, Error>::Ok>(value)")
    );
}

#[test]
fn formats_nested_refinement_unions_in_every_explicit_call_type_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn invoke(value: State) -> ()\n",
            "  sink<State::Ready|State::Closed, Result<State::Ready|State::Closed, Error>::Ok>(value)\n",
            "end\n",
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let expected = concat!(
        "fn invoke(value: State) -> ()\n",
        "\tsink<State::Ready | State::Closed, Result<State::Ready | State::Closed, Error>::Ok>(value)\n",
        "end\n",
    );
    let formatted = format_tree(&output.tree);
    assert_eq!(formatted, expected);

    let reparsed = parse(&SourceFile::new("main.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
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
    let mut refinements = function.params[0].ty_refinements.as_slice();
    let mut refinement_count = 0;
    let mut argument_count = 0;
    loop {
        assert_eq!(refinements.len(), 1);
        assert_eq!(refinements[0].alternatives.len(), 1);
        refinement_count += 1;
        let alternative = &refinements[0].alternatives[0];
        let Some(argument) = alternative.type_arguments.first() else {
            break;
        };
        assert_eq!(alternative.type_arguments.len(), 1);
        argument_count += 1;
        refinements = &argument.ty_refinements;
    }
    assert_eq!(refinement_count, DEPTH + 1);
    assert_eq!(argument_count, DEPTH);
}

#[test]
fn nested_refinement_argument_materialization_scales_linearly() {
    let copied = [64, 128, 256].map(refinement_argument_token_copies);
    eprintln!("variant refinement argument token copies at depths 64, 128, and 256: {copied:?}");

    assert!(
        copied[0] > 0,
        "the metric must observe argument materialization"
    );
    assert!(
        copied[1] <= copied[0] * 2 + 16 && copied[2] <= copied[1] * 2 + 16,
        "doubling nesting depth must not cause quadratic token copying: {copied:?}"
    );
}

#[test]
fn nested_refinement_boundary_diagnostics_scale_linearly() {
    let depths = [
        crate::MAX_VARIANT_REFINEMENT_NESTING,
        crate::MAX_VARIANT_REFINEMENT_NESTING + 1,
    ];
    let work = depths.map(refinement_boundary_coverage_work);
    eprintln!(
        "variant refinement boundary coverage work at depths {} and {}: {work:?}",
        depths[0], depths[1]
    );

    assert!(
        work[0] > 0,
        "the metric must observe boundary coverage work"
    );
    assert!(
        work[1] <= work[0] + 16,
        "one additional nesting level must add only adjacent-linear boundary coverage work: {work:?}"
    );
}

fn refinement_boundary_coverage_work(depth: usize) -> usize {
    let mut annotation = "Leaf::Value".to_string();
    for _ in 0..depth {
        annotation = format!("Layer<{annotation}>::Wrapped");
    }
    let source = SourceFile::new(
        "main.veln",
        format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n"),
    );

    crate::parser::reset_refinement_boundary_coverage_work();
    let output = parse(&source);
    if depth <= crate::MAX_VARIANT_REFINEMENT_NESTING {
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    } else {
        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.variant_refinement_type"
                && diagnostic.message == "variant refinement types are nested too deeply"
        }));
    }
    crate::parser::refinement_boundary_coverage_work()
}

fn refinement_argument_token_copies(depth: usize) -> usize {
    let mut annotation = "Leaf::Value".to_string();
    for _ in 0..depth {
        annotation = format!("Layer<List<{annotation}>, domain::Marker>::Wrapped");
    }
    let source = SourceFile::new(
        "main.veln",
        format!("fn nested(value: {annotation}) -> ()\n  ()\nend\n"),
    );

    crate::parser::reset_refinement_argument_token_copies();
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    crate::parser::refinement_argument_token_copies()
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
