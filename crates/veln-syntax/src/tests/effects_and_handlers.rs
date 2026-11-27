use super::*;

mod handler_recovery;

#[test]
fn parses_decode_as_an_explicit_module_member_name() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn decode(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn call(value: Int) -> Int\n",
            "  http2::frame::decode(value)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(decode) = &output.tree.items[0] else {
        panic!("expected decode function");
    };
    assert_eq!(decode.name.as_deref(), Some("decode"));
    let SyntaxItem::Function(call) = &output.tree.items[1] else {
        panic!("expected caller function");
    };
    let BodyLine::Expr { expr, .. } = &call.body[0] else {
        panic!("expected call expression");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Call { callee, .. }
            if matches!(&callee.kind, ExprKind::NamePath { segments, .. }
                if segments == &vec!["http2".to_string(), "frame".to_string(), "decode".to_string()])
    ));
}

#[test]
fn parses_and_formats_nominal_effect_operations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub effect Audit\n",
            "  record(user:String,count:Int)->String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::record(\"user\", 1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Effect(effect) = &output.tree.items[0] else {
        panic!("expected effect declaration");
    };
    assert_eq!(effect.name.as_deref(), Some("Audit"));
    assert_eq!(effect.operations[0].name.as_deref(), Some("record"));
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected function declaration");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Perform { effect, operation, recovered, args, .. }
            if effect == &vec!["Audit".to_string()]
                && operation == "record"
                && !recovered
                && args.len() == 2
    ));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub effect Audit\n",
            "\trecord(user: String, count: Int) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "\tperform Audit::record(\"user\", 1)\n",
            "end\n",
        )
    );
}

#[test]
fn records_recovery_for_unclosed_perform_arguments() {
    let source = SourceFile::new(
        "main.veln",
        "fn broken() -> Int\n  perform Choose::pick(\nend\n",
    );

    let output = parse(&source);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function declaration");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Perform {
            recovered: true,
            ..
        }
    ));
}

#[test]
fn keeps_perform_path_valid_when_only_arguments_are_recovered() {
    let source = SourceFile::new(
        "main.veln",
        "fn broken() -> Int\n  perform Choose::pick(1 2)\nend\n",
    );

    let output = parse(&source);
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.perform_argument")
    );
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function declaration");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Perform {
            effect,
            operation,
            recovered: false,
            args,
            ..
        } if effect == &["Choose"] && operation == "pick" && args.len() == 2
    ));
}

#[test]
fn records_recovery_for_incomplete_perform_paths_and_call_delimiters() {
    for text in [
        "fn broken() -> Int\n  perform Choose()\nend\n",
        "fn broken() -> Int\n  perform Choose::pick 1)\nend\n",
    ] {
        let output = parse(&SourceFile::new("main.veln", text));
        let SyntaxItem::Function(function) = &output.tree.items[0] else {
            panic!("expected function declaration");
        };
        let BodyLine::Expr { expr, .. } = &function.body[0] else {
            panic!("expected expression body");
        };
        assert!(matches!(
            &expr.kind,
            ExprKind::Perform {
                recovered: true,
                ..
            }
        ));
    }
}

#[test]
fn records_recovery_within_effect_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Clean\n",
            "  run() -> Int\n",
            "end\n\n",
            "effect Empty\n",
            "end\n\n",
            "effect Broken\n",
            "  run()\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let effects = output
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Effect(effect) => Some(effect),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(effects.len(), 3);
    assert!(!effects[0].recovered);
    assert!(effects[1].end_present);
    assert!(effects[1].recovered);
    assert!(effects[2].end_present);
    assert!(effects[2].recovered);
}

#[test]
fn parses_and_formats_effect_row_binder_and_tail() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn apply<effect E>(callback: fn(Int) -> Int effects [stdio, ...E]) -> Int effects [stdio, ...E]\n",
            "  callback(1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(
        function
            .effect_binder
            .as_ref()
            .map(|binder| binder.name.as_str()),
        Some("E")
    );
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["stdio".to_string(), "...E".to_string()]
    );
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub fn apply<effect E>(callback: fn(Int) -> Int effects [stdio, ...E]) -> Int effects [stdio, ...E]\n",
            "\tcallback(1)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_lexical_handler_declarations_and_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value()->Int\n",
            "end\n",
            "\n",
            "fn provide(ctx:Int)->Int\n",
            "  ctx\n",
            "end\n",
            "\n",
            "handler ask(ctx:Int) for Ask effects [stdio]\n",
            "  value() => provide(ctx)\n",
            "end\n",
            "\n",
            "fn main() -> Int effects [stdio]\n",
            "  handle perform Ask::value() with ask(41)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[2] else {
        panic!("expected handler declaration");
    };
    assert_eq!(handler.name.as_deref(), Some("ask"));
    assert_eq!(handler.effect, vec!["Ask".to_string()]);
    assert_eq!(
        handler.operation_clauses[0].operation.as_deref(),
        Some("value")
    );
    let SyntaxItem::Function(function) = &output.tree.items[3] else {
        panic!("expected main function");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Handle {
            body: HandleBody::Expression(body),
            entries,
            terminated: false,
        }
            if entries.len() == 1
                && entries[0].handler == vec!["ask".to_string()]
                && entries[0].args.len() == 1
                && matches!(&body.kind, ExprKind::Perform { operation, .. } if operation == "value")
    ));
    let formatted = format_tree(&output.tree);
    assert_eq!(
        formatted,
        concat!(
            "effect Ask\n",
            "\tvalue() -> Int\n",
            "end\n",
            "\n",
            "fn provide(ctx: Int) -> Int\n",
            "\tctx\n",
            "end\n",
            "\n",
            "handler ask(ctx: Int) for Ask effects [stdio]\n",
            "\tvalue() => provide(ctx)\n",
            "end\n",
            "\n",
            "fn main() -> Int effects [stdio]\n",
            "\thandle perform Ask::value() with ask(41)\n",
            "end\n",
        )
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn accepts_handles_in_ordinary_identifier_positions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use handles\n\n",
            "fn handles(handles: Int) -> { handles : Int }\n",
            "  let handles = handles::value(handles)\n",
            "  { handles: handles }\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.uses[0].name, "handles");
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function declaration");
    };
    assert_eq!(function.name.as_deref(), Some("handles"));
    assert_eq!(function.params[0].name, "handles");
}

#[test]
fn parses_and_formats_terminated_multiple_handler_forms() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn expression_body() -> Int\n",
            "  handle value() with\n",
            "    outer(first()),\n",
            "    inner(second()),\n",
            "  end\n",
            "end\n\n",
            "fn block_body() -> Int\n",
            "  handle\n",
            "    let value = first()\n",
            "    value\n",
            "  with\n",
            "    outer(),\n",
            "    inner()\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(expression) = &output.tree.items[0] else {
        panic!("expected expression-body function");
    };
    let BodyLine::Expr { expr, .. } = &expression.body[0] else {
        panic!("expected expression-body handle");
    };
    let ExprKind::Handle {
        body: HandleBody::Expression(body),
        entries,
        terminated: true,
    } = &expr.kind
    else {
        panic!("expected terminated expression handle");
    };
    assert!(matches!(body.kind, ExprKind::Call { .. }));
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].handler, vec!["outer"]);
    assert_eq!(entries[1].handler, vec!["inner"]);
    assert_eq!(entries[0].args.len(), 1);
    assert!(entries[0].span.start.offset < entries[0].args[0].span.start.offset);
    assert!(entries[0].args[0].span.end.offset < entries[0].span.end.offset);

    let SyntaxItem::Function(block) = &output.tree.items[1] else {
        panic!("expected block-body function");
    };
    let BodyLine::Expr { expr, .. } = &block.body[0] else {
        panic!("expected block-body handle");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Handle {
            body: HandleBody::Block { body, .. },
            entries,
            terminated: true,
        } if body.len() == 2 && entries.len() == 2
    ));

    let formatted = format_tree(&output.tree);
    assert_eq!(
        formatted,
        concat!(
            "fn expression_body() -> Int\n",
            "\thandle value() with\n",
            "\t\touter(first()),\n",
            "\t\tinner(second())\n",
            "\tend\n",
            "end\n\n",
            "fn block_body() -> Int\n",
            "\thandle\n",
            "\t\tlet value = first()\n",
            "\t\tvalue\n",
            "\twith\n",
            "\t\touter(),\n",
            "\t\tinner()\n",
            "\tend\n",
            "end\n",
        )
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn formatter_keeps_ineligible_nested_short_handler_chains_nested() {
    let cases = [
        (
            concat!(
                "fn commented() -> Int\n",
                "  handle (handle value() with inner()) with outer() # retain nested\n",
                "end\n",
            ),
            "handle handle value() with inner() with outer()  # retain nested",
        ),
        (
            concat!(
                "fn commented_complete_chain() -> Int\n",
                "  handle (handle (handle value() with inner()) with middle()) with outer() # retain all\n",
                "end\n",
            ),
            "handle handle handle value() with inner() with middle() with outer()  # retain all",
        ),
        (
            concat!(
                "fn multiline_complete_chain() -> Int\n",
                "  handle (\n",
                "    handle (handle value() with inner()) with middle()\n",
                "  ) with outer()\n",
                "end\n",
            ),
            "handle handle handle value() with inner() with middle() with outer()",
        ),
        (
            concat!(
                "fn mixed() -> Int\n",
                "  handle (handle (handle value() with\n",
                "    listed()\n",
                "  end) with middle()) with outer()\n",
                "end\n",
            ),
            "end with middle() with outer()",
        ),
        (
            concat!(
                "fn call_boundary() -> Int\n",
                "  handle wrap(handle value() with inner()) with outer()\n",
                "end\n",
            ),
            "handle wrap(handle value() with inner()) with outer()",
        ),
        (
            concat!(
                "fn mixed_other_direction() -> Int\n",
                "  handle (handle (handle value() with inner()) with middle()) with\n",
                "    listed()\n",
                "  end\n",
                "end\n",
            ),
            "handle handle handle value() with inner() with middle() with",
        ),
    ];

    for (source_text, retained) in cases {
        let source = SourceFile::new("main.veln", source_text);
        let output = parse(&source);
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

        let formatted = format_tree(&output.tree);
        assert!(formatted.contains(retained), "{formatted}");
    }
}

#[test]
fn terminated_handler_suppression_stops_at_an_independent_expression_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn independent_call() -> Int\n",
            "  handle wrap(handle (handle value() with inner()) with middle()) with\n",
            "    listed()\n",
            "  end\n",
            "end\n",
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let formatted = format_tree(&output.tree);
    assert!(
        formatted.contains(concat!(
            "wrap(handle value() with\n",
            "\t\tmiddle(),\n",
            "\t\tinner()\n",
            "\tend)"
        )),
        "{formatted}"
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn short_handler_comment_lookup_work_scales_with_candidates_not_comments() {
    let original = short_handler_comment_range_queries(32, 32);
    let larger = short_handler_comment_range_queries(64, 64);

    assert!(
        original.0 <= 2 * 32,
        "unexpected original range queries: {}",
        original.0
    );
    assert!(
        larger.0 <= 2 * 64,
        "unexpected larger range queries: {}",
        larger.0
    );
    assert!(
        larger.0 <= original.0.saturating_mul(2),
        "doubling comments and candidates grew range queries from {} to {}",
        original.0,
        larger.0
    );
    assert_eq!(
        original.1, 0,
        "unrelated comment keys entered a candidate range"
    );
    assert_eq!(
        larger.1, 0,
        "unrelated comment keys entered a candidate range"
    );
}

fn short_handler_comment_range_queries(comment_count: usize, chain_count: usize) -> (usize, usize) {
    let mut source_text = String::new();
    for index in 0..chain_count {
        source_text.push_str(&format!(
            "fn chain{index}() -> Int\n  handle (handle value() with inner()) with outer()\nend\n\n"
        ));
    }
    for index in 0..comment_count {
        source_text.push_str(&format!(
            "# unrelated {index}\nfn unrelated{index}() -> Int\n  {index}\nend\n\n"
        ));
    }

    let source = SourceFile::new("main.veln", source_text);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    crate::format::reset_comment_range_queries();
    let formatted = format_tree(&output.tree);
    assert_eq!(formatted.matches("\t\touter(),").count(), chain_count);
    (
        crate::format::comment_range_queries(),
        crate::format::comment_range_keys_inspected(),
    )
}

#[test]
fn terminated_handlers_accept_multiline_bodies_arguments_and_contextual_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn expression_body() -> Int\n",
            "  handle identity(\n",
            "    1\n",
            "  ) with\n",
            "    outer(\n",
            "      first()\n",
            "    )\n",
            "  end\n",
            "end\n\n",
            "fn block_body(handler: Int) -> Int\n",
            "  handle\n",
            "    handler\n",
            "  with\n",
            "    outer()\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2);

    let formatted = format_tree(&output.tree);
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn handled_block_body_accepts_contextual_handler_and_handle_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn contextual(handler: Int, record: Record) -> Int\n",
            "  handle\n",
            "    handler + record.handle\n",
            "  with\n",
            "    identity()\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function");
    };
    assert!(matches!(
        &function.body[0],
        BodyLine::Expr {
            expr: Expr {
                kind: ExprKind::Handle {
                    body: HandleBody::Block { body, .. },
                    ..
                },
                ..
            },
            ..
        } if body.len() == 1
    ));
}

#[test]
fn formats_terminated_handler_entries_with_comments_canonically() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn commented() -> Int\n",
            "  handle value() with\n",
            "    # outer first\n",
            "    outer( ), # retained entry comment\n",
            "\n",
            "    # inner shadows outer\n",
            "    inner( ),\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let formatted = format_tree(&output.tree);
    assert_eq!(
        formatted,
        concat!(
            "fn commented() -> Int\n",
            "\thandle value() with\n",
            "\t\t# outer first\n",
            "\t\touter(),  # retained entry comment\n",
            "\t\t# inner shadows outer\n",
            "\t\tinner()\n",
            "\tend\n",
            "end\n",
        )
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn preserves_trailing_comments_on_multiline_handler_entries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn commented() -> Int\n",
            "  handle 1 with\n",
            "    outer(begin\n",
            "      1\n",
            "    end), # outer entry\n",
            "    inner(begin\n",
            "      2\n",
            "    end), # inner entry\n",
            "  end # handler list\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let formatted = format_tree(&output.tree);
    assert_eq!(
        formatted,
        concat!(
            "fn commented() -> Int\n",
            "\thandle 1 with\n",
            "\t\touter(begin\n",
            "\t\t\t1\n",
            "\t\tend),  # outer entry\n",
            "\t\tinner(begin\n",
            "\t\t\t2\n",
            "\t\tend)  # inner entry\n",
            "\tend  # handler list\n",
            "end\n",
        )
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn terminated_handler_recovery_preserves_the_enclosing_function_end_and_next_item() {
    for invalid in [
        "handle value() with\nend",
        "handle value() with\n  outer()\n  inner()\nend",
        "handle value() with\n  outer\nend",
        "handle value() with outer(), inner()",
        "handle value() with\n  outer()",
        "handle\n  value()\nwith outer()\nend",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("fn broken() -> Int\n  {invalid}\nend\n\nfn survivor() -> Int\n  1\nend\n"),
        );
        let output = parse(&source);
        assert!(!output.diagnostics.is_empty(), "{invalid}");
        assert_eq!(
            output.tree.items.len(),
            2,
            "{invalid}: {:#?}",
            output.diagnostics
        );
        let SyntaxItem::Function(survivor) = &output.tree.items[1] else {
            panic!("expected following function for {invalid}");
        };
        assert_eq!(survivor.name.as_deref(), Some("survivor"), "{invalid}");
    }
}

#[test]
fn missing_handler_application_at_eof_reports_a_diagnostic_without_panicking() {
    let source = SourceFile::new("main.veln", "fn broken() -> Int\n  handle 1 with");

    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.handle_expression")
    );
}

#[test]
fn handled_block_missing_with_recovers_at_synthetic_boundaries_without_panicking() {
    for (source_text, expected_items) in [
        (
            concat!(
                "fn broken() -> Int\n",
                "  handle\n",
                "    1\n",
                "end\n\n",
                "fn survivor() -> Int\n",
                "  1\n",
                "end\n",
            ),
            2,
        ),
        ("fn broken() -> Int\n  handle\n    1", 1),
        (
            concat!(
                "fn broken() -> Int\n",
                "  handle\n",
                "    1\n",
                "fn survivor() -> Int\n",
                "  1\n",
                "end\n",
            ),
            2,
        ),
    ] {
        let output = parse(&SourceFile::new("main.veln", source_text));

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.handle_expression"),
            "{:#?}",
            output.diagnostics
        );
        assert_eq!(
            output.tree.items.len(),
            expected_items,
            "{:#?}",
            output.diagnostics
        );
        if expected_items == 2 {
            assert!(matches!(
                &output.tree.items[1],
                SyntaxItem::Function(function)
                    if function.name.as_deref() == Some("survivor")
            ));
        }
    }
}

#[test]
fn missing_handler_end_preserves_enclosing_control_flow_boundaries() {
    for (control, unexpected_diagnostic) in [
        (
            concat!(
                "if true\n",
                "    handle 1 with\n",
                "      outer()\n",
                "  else\n",
                "    2\n",
                "  end",
            ),
            "parse.if_missing_else",
        ),
        (
            concat!(
                "match 0\n",
                "    0 => handle 1 with\n",
                "      outer()\n",
                "    _ => 2\n",
                "  end",
            ),
            "parse.match_arm",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("fn broken() -> Int\n  {control}\nend\n\nfn survivor() -> Int\n  1\nend\n"),
        );
        let output = parse(&source);
        let diagnostic_ids = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>();

        assert!(
            diagnostic_ids.contains(&"parse.handle_missing_end"),
            "{:#?}",
            output.diagnostics
        );
        assert!(
            !diagnostic_ids.contains(&unexpected_diagnostic),
            "{:#?}",
            output.diagnostics
        );
        assert!(matches!(
            output.tree.items.as_slice(),
            [SyntaxItem::Function(broken), SyntaxItem::Function(survivor)]
                if broken.name.as_deref() == Some("broken")
                    && broken.end_present
                    && survivor.name.as_deref() == Some("survivor")
                    && survivor.end_present
        ));
    }
}

#[test]
fn terminated_handlers_preserve_module_values_and_nested_cleanup_blocks() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = handle 1 with\n",
            "  outer()\n",
            "end\n\n",
            "fn embedded() -> Int\n",
            "  consume(handle\n",
            "    defer\n",
            "      ()\n",
            "    end\n",
            "    1\n",
            "  with\n",
            "    outer()\n",
            "  end)\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2);
}

#[test]
fn unterminated_handler_recovery_stops_before_the_next_top_level_item() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn broken() -> Int\n",
            "  handle value() with\n",
            "    outer()\n",
            "fn survivor() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(!output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 2, "{:#?}", output.diagnostics);
    let SyntaxItem::Function(survivor) = &output.tree.items[1] else {
        panic!("expected following function");
    };
    assert_eq!(survivor.name.as_deref(), Some("survivor"));
}

#[test]
fn unterminated_handler_preserves_following_module_value_and_function_items() {
    let source_text = concat!(
        "fn broken() -> Int\n",
        "  handle 1 with\n",
        "    outer()\n",
        "let survivor: Int = handle 1 with\n",
        "  outer()\n",
        "end\n\n",
        "fn after() -> Int\n",
        "  survivor\n",
        "end\n",
    );
    let survivor_start = source_text.find("let survivor").expect("survivor offset");

    let output = parse(&SourceFile::new("main.veln", source_text));

    assert!(!output.diagnostics.is_empty());
    assert!(matches!(
        output.tree.items.as_slice(),
        [
            SyntaxItem::Function(broken),
            SyntaxItem::ModuleValue(survivor),
            SyntaxItem::Function(after),
        ] if broken.name.as_deref() == Some("broken")
            && survivor.name.as_deref() == Some("survivor")
            && after.name.as_deref() == Some("after")
    ));
    assert!(
        output.diagnostics.iter().all(|diagnostic| diagnostic
            .span
            .as_ref()
            .is_none_or(|span| span.start.offset <= survivor_start)),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn unterminated_handler_argument_preserves_both_function_boundaries() {
    let source_text = concat!(
        "fn broken() -> Int\n",
        "  handle 1 with\n",
        "    outer(\n",
        "  end\n",
        "end\n\n",
        "fn survivor() -> Int\n",
        "  1\n",
        "end\n",
    );
    let survivor_start = source_text.find("fn survivor").expect("survivor offset");
    let output = parse(&SourceFile::new("main.veln", source_text));

    assert!(!output.diagnostics.is_empty());
    assert!(matches!(
        output.tree.items.as_slice(),
        [SyntaxItem::Function(broken), SyntaxItem::Function(survivor)]
            if broken.name.as_deref() == Some("broken")
                && broken.end_present
                && survivor.name.as_deref() == Some("survivor")
                && survivor.end_present
    ));
    assert!(
        output.diagnostics.iter().all(|diagnostic| diagnostic
            .span
            .as_ref()
            .is_none_or(|span| span.end.offset <= survivor_start)),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn contextual_handler_names_remain_terminated_handler_entries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn contextual() -> Int\n",
            "  handle 1 with\n",
            "    handler(),\n",
            "    handler::qualified()\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected handle expression");
    };
    let ExprKind::Handle { entries, .. } = &expr.kind else {
        panic!("expected handle expression");
    };
    assert_eq!(entries[0].handler, ["handler"]);
    assert_eq!(entries[1].handler, ["handler", "qualified"]);
}

#[test]
fn malformed_terminated_handler_stops_before_handler_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn broken() -> Int\n",
            "  handle 1 with\n",
            "    outer()\n",
            "handler declared() for Example\n",
            "  value() => 1\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(!output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 2, "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[1] else {
        panic!("expected following handler declaration");
    };
    assert_eq!(handler.name.as_deref(), Some("declared"));
}

#[test]
fn terminated_handler_entries_and_arguments_have_exact_spans() {
    let cases = [
        (
            concat!(
                "fn spans() -> Int\n",
                "  handle body() with\n",
                "    outer(first(), second()),\n",
                "    ns::inner(third())\n",
                "  end\n",
                "end\n",
            ),
            [(43, 67, 3, 5, 3, 29), (73, 91, 4, 5, 4, 23)],
            [(43, 48, 3, 5, 3, 10), (73, 82, 4, 5, 4, 14)],
            [
                vec![(49, 56, 3, 11, 3, 18), (58, 66, 3, 20, 3, 28)],
                vec![(83, 90, 4, 15, 4, 22)],
            ],
        ),
        (
            concat!(
                "fn block() -> Int\n",
                "  handle\n",
                "    value()\n",
                "  with\n",
                "    outer(first(), second()),\n",
                "    ns::inner(third())\n",
                "  end\n",
                "end\n",
            ),
            [(50, 74, 5, 5, 5, 29), (80, 98, 6, 5, 6, 23)],
            [(50, 55, 5, 5, 5, 10), (80, 89, 6, 5, 6, 14)],
            [
                vec![(56, 63, 5, 11, 5, 18), (65, 73, 5, 20, 5, 28)],
                vec![(90, 97, 6, 15, 6, 22)],
            ],
        ),
    ];

    for (source_text, entry_spans, handler_spans, argument_spans) in cases {
        let output = parse(&SourceFile::new("spans.veln", source_text));
        assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        let SyntaxItem::Function(function) = &output.tree.items[0] else {
            panic!("expected function");
        };
        let BodyLine::Expr { expr, .. } = &function.body[0] else {
            panic!("expected handle expression");
        };
        let ExprKind::Handle { entries, .. } = &expr.kind else {
            panic!("expected handle expression");
        };
        for (index, entry) in entries.iter().enumerate() {
            assert_source_span(&entry.span, entry_spans[index]);
            assert_source_span(&entry.handler_span, handler_spans[index]);
            for (argument, expected) in entry.args.iter().zip(&argument_spans[index]) {
                assert_source_span(&argument.span, *expected);
            }
        }
    }
}

fn assert_source_span(
    span: &veln_source::SourceSpan,
    expected: (usize, usize, usize, usize, usize, usize),
) {
    assert_eq!(
        (
            span.start.offset,
            span.end.offset,
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column,
        ),
        expected,
    );
}

#[test]
fn qualified_handle_leaf_does_not_start_a_terminated_handler() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn qualified() -> Int\n",
            "  let callable = namespace::handle\n",
            "  callable()\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
}

#[test]
fn parses_terminated_handlers_inside_calls_and_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn embedded() -> { call: Int, field: Int }\n",
            "  {\n",
            "    call: consume(handle value() with\n",
            "      outer()\n",
            "    end),\n",
            "    field: handle value() with\n",
            "      outer(),\n",
            "    end,\n",
            "  }\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected record expression");
    };
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record");
    };
    assert_eq!(fields.len(), 2);
    assert!(matches!(
        fields[0].expr.kind,
        ExprKind::Call { ref args, .. }
            if matches!(args[0].kind, ExprKind::Handle { terminated: true, .. })
    ));
    assert!(matches!(
        fields[1].expr.kind,
        ExprKind::Handle {
            terminated: true,
            ..
        }
    ));
}

#[test]
fn handler_declaration_preserves_header_and_body_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub handler audit(ctx: Context) for telemetry::Audit effects [stdio, net]\n",
            "  record(message) => ctx.record(message)\n",
            "\n",
            "  flush() => ctx.flush()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[0] else {
        panic!("expected handler declaration");
    };
    assert_eq!(handler.visibility, Visibility::Public);
    assert_eq!(handler.name.as_deref(), Some("audit"));
    assert_eq!(handler.params.len(), 1);
    assert_eq!(handler.params[0].name, "ctx");
    assert_eq!(handler.effect, vec!["telemetry", "Audit"]);
    assert_eq!(
        handler.effects.as_deref(),
        Some(["stdio".to_string(), "net".to_string()].as_slice())
    );
    assert_eq!(handler.operation_clauses.len(), 2);
    assert_eq!(
        handler.operation_clauses[0].operation.as_deref(),
        Some("record")
    );
    assert_eq!(
        handler.operation_clauses[1].operation.as_deref(),
        Some("flush")
    );
    assert!(handler.end_present);
    assert_eq!(handler.span.start.line, 1);
    assert_eq!(handler.span.end.line, 6);
}

#[test]
fn rejects_trailing_comma_in_handler_operation_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Pick\n",
            "  next(step: Int) -> Int\n",
            "end\n",
            "\n",
            "handler pick() for Pick\n",
            "  next(step,) => step\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.handler_operation_parameter"
                && diagnostic.message == "handler operation parameter list cannot end with a comma"
                && diagnostic.parser_context == "handler_operation_clause"
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_old_handler_operation_syntax_with_one_migration_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "handler ask() for Ask\n",
            "  value = provide\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.handler_operation_old_syntax");
    assert_eq!(
        diagnostic.message,
        "handler operation clause must bind operation parameters with `(` and evaluate an expression with `=>`"
    );
    assert_eq!(diagnostic.parser_context, "handler_operation_clause");
    assert_eq!(diagnostic.unexpected.text, "=");
    assert_eq!(diagnostic.expected, vec!["("]);
    assert_eq!(
        diagnostic.recovery.strategy,
        RecoveryStrategy::SynchronizeToAnchor
    );
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("newline"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 10);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 9);
}

#[test]
fn rejects_effect_operation_parameter_without_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("effect Audit\n", "  record(user) -> String\n", "end\n"),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.effect_operation_parameter_type"
                && diagnostic.message == "effect operation parameter is missing a type annotation"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 2 && span.start.column == 10)
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn preserves_typed_parameter_metadata_after_missing_effect_parameter_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user, entries: ...audit::Result<core::String, core::Error>::Ok) -> String\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(
        output
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "parse.effect_operation_parameter_type")
            .count(),
        1,
        "{:#?}",
        output.diagnostics
    );
    let SyntaxItem::Effect(effect) = &output.tree.items[0] else {
        panic!("expected effect declaration");
    };
    let entries = &effect.operations[0].params[1];
    assert_eq!(entries.name, "entries");
    assert!(entries.is_variadic);
    assert_eq!(
        entries.ty.as_deref(),
        Some("audit::Result<core::String, core::Error>::Ok")
    );
    assert_eq!(entries.ty_paths.len(), 3);
    assert_eq!(entries.ty_refinements.len(), 1);
}

#[test]
fn rejects_effect_declaration_without_operations() {
    let source = SourceFile::new("main.veln", concat!("effect Audit\n", "end\n"));

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.effect_operation_required"
                && diagnostic.message == "effect declaration requires at least one operation"
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn records_recovery_for_effect_rows_and_handled_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn broken() -> Int effects [Choose @]\n",
            "  1\n",
            "end\n\n",
            "handler broken() for Choose @\n",
            "  pick() => 1\n",
            "end\n\n",
            "handler row_broken() for Choose effects [Choose @]\n",
            "  pick() => 1\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function");
    };
    let SyntaxItem::Handler(handler) = &output.tree.items[1] else {
        panic!("expected handler");
    };
    let SyntaxItem::Handler(row_broken) = &output.tree.items[2] else {
        panic!("expected handler");
    };

    assert!(function.effects_recovered);
    assert!(handler.effect_recovered);
    assert!(!row_broken.effect_recovered);
    assert!(row_broken.effects_recovered);
}
