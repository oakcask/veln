use super::*;

#[test]
fn retains_and_canonically_formats_function_binders() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn transform<T,U,effect E,>(value:T,callback:fn(T)->U effects [...E])->U effects [...E]\n",
            "  callback(value)\n",
            "end\n",
        ),
    );
    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(
        function
            .type_binders
            .iter()
            .map(|binder| binder.name.as_str())
            .collect::<Vec<_>>(),
        vec!["T", "U"]
    );
    assert_eq!(
        function
            .type_binders
            .iter()
            .map(|binder| &source.text()[binder.span.start.offset..binder.span.end.offset])
            .collect::<Vec<_>>(),
        vec!["T", "U"]
    );
    assert_eq!(
        function
            .effect_binder
            .as_ref()
            .map(|binder| binder.name.as_str()),
        Some("E")
    );
    assert_eq!(
        function
            .effect_binder
            .as_ref()
            .map(|binder| { &source.text()[binder.span.start.offset..binder.span.end.offset] }),
        Some("<T,U,effect E,>")
    );
    let formatted = concat!(
        "pub fn transform<T, U, effect E>(value: T, callback: fn(T) -> U effects [...E]) -> U effects [...E]\n",
        "\tcallback(value)\n",
        "end\n",
    );
    assert_eq!(format_tree(&output.tree), formatted);

    let reparsed_source = SourceFile::new("formatted.veln", formatted);
    let reparsed = parse(&reparsed_source);
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn accepts_single_multiple_and_effect_only_binder_lists() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn identity<T>(value: T) -> T\n  value\nend\n",
            "fn choose<T, U>(left: T, right: U) -> T\n  left\nend\n",
            "fn apply<effect E>(callback: fn() -> () effects [...E]) -> () effects [...E]\n  callback()\nend\n",
        ),
    );
    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let functions = output
        .tree
        .items
        .iter()
        .map(|item| match item {
            SyntaxItem::Function(function) => function.as_ref(),
            _ => panic!("expected function declaration"),
        })
        .collect::<Vec<_>>();
    assert_eq!(functions[0].type_binders.len(), 1);
    assert_eq!(functions[1].type_binders.len(), 2);
    assert!(functions[2].type_binders.is_empty());
    assert_eq!(
        functions[2]
            .effect_binder
            .as_ref()
            .map(|binder| binder.name.as_str()),
        Some("E")
    );
}

#[test]
fn rejects_invalid_function_binder_entries_and_ordering() {
    let cases = [
        (
            "fn empty<>() -> ()\n  ()\nend\n",
            "parse.empty_function_binder_list",
        ),
        (
            "fn duplicate<T, T>() -> ()\n  ()\nend\n",
            "parse.duplicate_function_binder",
        ),
        (
            "fn cross<T, effect T>() -> ()\n  ()\nend\n",
            "parse.duplicate_function_binder",
        ),
        (
            "fn order<effect E, T>() -> ()\n  ()\nend\n",
            "parse.type_binder_after_effect",
        ),
        (
            "fn rows<effect E, effect F>() -> ()\n  ()\nend\n",
            "parse.multiple_effect_binders",
        ),
        (
            "fn bound<T: Trait>() -> ()\n  ()\nend\n",
            "parse.function_binder_bound",
        ),
        (
            "fn applied<T<Int>>() -> ()\n  ()\nend\n",
            "parse.function_binder_bound",
        ),
        (
            "fn placeholder<_>() -> ()\n  ()\nend\n",
            "parse.function_binder_entry",
        ),
    ];

    for (text, expected_id) in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == expected_id),
            "expected {expected_id} for {text:?}: {:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn rejects_generic_test_variadic_and_callsite_declarations() {
    let cases = [
        (
            "test generic<T>() -> ()\n  ()\nend\n",
            "parse.generic_test_declaration",
        ),
        (
            "fn variadic<T>(values: ...T) -> ()\n  ()\nend\n",
            "parse.generic_function_variadic",
        ),
        (
            "fn located<T>() -> () callsite\n  ()\nend\n",
            "parse.generic_function_callsite",
        ),
    ];

    for (text, expected_id) in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == expected_id),
            "expected {expected_id}: {:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn invalid_binders_recover_before_following_functions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn invalid<T: Trait>(value: T) -> T\n  value\nend\n",
            "fn valid() -> Int\n  1\nend\n",
        ),
    );
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.function_binder_bound")
    );
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected following function declaration");
    };
    assert_eq!(function.name.as_deref(), Some("valid"));
}

#[test]
fn unclosed_binder_list_preserves_following_function_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn invalid<\n", "fn valid() -> Int\n  1\nend\n",),
    );
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.function_binder_entry"),
        "{:#?}",
        output.diagnostics
    );
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::Function(invalid) = &output.tree.items[0] else {
        panic!("expected recovered function declaration");
    };
    assert_eq!(invalid.name.as_deref(), Some("invalid"));
    assert!(invalid.params.is_empty());
    assert!(invalid.body.is_empty());
    let SyntaxItem::Function(valid) = &output.tree.items[1] else {
        panic!("expected following function declaration");
    };
    assert_eq!(valid.name.as_deref(), Some("valid"));
    assert_eq!(valid.return_type.as_deref(), Some("Int"));
    assert!(valid.end_present);
}

#[test]
fn retained_unclosed_binders_preserve_following_function_declaration() {
    let cases = [
        ("fn invalid<T\n", Some("T"), None),
        ("fn invalid<effect E\n", None, Some("E")),
    ];

    for (invalid_header, expected_type_binder, expected_effect_binder) in cases {
        let source = SourceFile::new(
            "main.veln",
            format!("{invalid_header}fn valid() -> Int\n  1\nend\n"),
        );
        let output = parse(&source);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.function_binder_separator"),
            "{:#?}",
            output.diagnostics
        );
        assert_eq!(output.tree.items.len(), 2);
        let SyntaxItem::Function(invalid) = &output.tree.items[0] else {
            panic!("expected recovered function declaration");
        };
        assert_eq!(invalid.name.as_deref(), Some("invalid"));
        assert_eq!(
            invalid
                .type_binders
                .first()
                .map(|binder| binder.name.as_str()),
            expected_type_binder
        );
        assert_eq!(
            invalid
                .effect_binder
                .as_ref()
                .map(|binder| binder.name.as_str()),
            expected_effect_binder
        );
        assert!(invalid.params.is_empty());
        assert!(invalid.return_type.is_none());
        assert!(invalid.body.is_empty());
        assert!(!invalid.end_present);

        let SyntaxItem::Function(valid) = &output.tree.items[1] else {
            panic!("expected following function declaration");
        };
        assert_eq!(valid.name.as_deref(), Some("valid"));
        assert_eq!(valid.return_type.as_deref(), Some("Int"));
        assert_eq!(valid.body.len(), 1);
        assert!(valid.end_present);
    }
}

#[test]
fn invalid_binder_at_parameter_boundary_preserves_signature() {
    let source = SourceFile::new(
        "main.veln",
        "fn invalid<T,(value: Int) -> Int\n  value\nend\n",
    );
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.function_binder_entry"),
        "{:#?}",
        output.diagnostics
    );
    assert_recovered_invalid_signature(&output);
}

#[test]
fn applied_binder_recovery_prefers_a_balanced_close_over_nested_parentheses() {
    let cases = [
        (
            "fn invalid<T<Int(value: Int) -> Int\n  value\nend\n",
            vec![
                ("parse.function_binder_bound", "<"),
                ("parse.function_binder_separator", "("),
            ],
        ),
        (
            "fn invalid<T<Foo(Int) -> Int> >(value: Int) -> Int\n  value\nend\n",
            vec![("parse.function_binder_bound", "<")],
        ),
    ];

    for (text, expected_diagnostics) in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);

        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let span = diagnostic.span.as_ref().expect("expected a primary span");
                    (
                        diagnostic.id,
                        &source.text()[span.start.offset..span.end.offset],
                    )
                })
                .collect::<Vec<_>>(),
            expected_diagnostics,
            "{:#?}",
            output.diagnostics
        );
        assert_recovered_invalid_signature(&output);
    }
}

#[test]
fn applied_binder_recovery_consumes_packed_closers_by_owner() {
    let cases = [
        "fn invalid<T<Foo(Int) -> Int>>(value: Int) -> Int\n  value\nend\n",
        "fn invalid<T<Foo<Bar(Int) -> Int>>>(value: Int) -> Int\n  value\nend\n",
    ];

    for text in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);

        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let span = diagnostic.span.as_ref().expect("expected a primary span");
                    (
                        diagnostic.id,
                        &source.text()[span.start.offset..span.end.offset],
                    )
                })
                .collect::<Vec<_>>(),
            vec![("parse.function_binder_bound", "<")],
            "{:#?}",
            output.diagnostics
        );
        assert_recovered_invalid_signature(&output);
    }
}

#[test]
fn unclosed_applied_binder_skips_function_type_and_nested_parameter_candidates() {
    let cases = [
        "fn invalid<T<fn(Int) -> Int(value: Int) -> Int\n  value\nend\n",
        "fn invalid<T<Foo((Int) -> Int)(value: Int) -> Int\n  value\nend\n",
    ];

    for text in cases {
        let source = SourceFile::new("main.veln", text);
        let output = parse(&source);

        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| {
                    let span = diagnostic.span.as_ref().expect("expected a primary span");
                    (
                        diagnostic.id,
                        &source.text()[span.start.offset..span.end.offset],
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("parse.function_binder_bound", "<"),
                ("parse.function_binder_separator", "("),
            ],
            "{:#?}",
            output.diagnostics
        );
        let separator = output
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "parse.function_binder_separator")
            .expect("expected a binder separator diagnostic");
        assert_eq!(
            separator
                .span
                .as_ref()
                .expect("expected a primary span")
                .start
                .offset,
            text.find("(value: Int)")
                .expect("expected the retained parameter list")
        );
        assert_recovered_invalid_signature(&output);
    }
}

#[test]
fn applied_binder_with_nested_unit_type_recovers_at_parameter_list() {
    let source = SourceFile::new(
        "main.veln",
        "fn invalid<T<()> >(value: Int) -> Int\n  value\nend\n",
    );
    let output = parse(&source);

    assert_eq!(
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>(),
        vec!["parse.function_binder_bound"]
    );
    assert_recovered_invalid_signature(&output);
}

fn assert_recovered_invalid_signature(output: &ParseOutput) {
    let function = first_function(output);
    assert_eq!(function.type_binders.len(), 1);
    assert_eq!(function.type_binders[0].name, "T");
    assert_eq!(function.params.len(), 1);
    assert_eq!(function.params[0].name, "value");
    assert_eq!(function.params[0].ty.as_deref(), Some("Int"));
    assert_eq!(function.return_type.as_deref(), Some("Int"));
    assert_eq!(function.body.len(), 1);
    assert!(matches!(
        &function.body[0],
        BodyLine::Expr { expr, .. }
            if matches!(&expr.kind, ExprKind::NamePath { segments, .. } if segments == &vec!["value".to_string()])
    ));
    assert!(function.end_present);
}

#[test]
fn multiple_effect_binder_diagnostic_marks_redundant_keyword() {
    let source = SourceFile::new(
        "main.veln",
        "fn rows<effect E, effect>() -> ()\n  ()\nend\n",
    );
    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.multiple_effect_binders")
        .expect("expected a multiple effect binder diagnostic");
    let span = diagnostic.span.as_ref().expect("expected a primary span");
    assert_eq!(&source.text()[span.start.offset..span.end.offset], "effect");
}

#[test]
fn missing_binder_separator_reports_the_unexpected_entry_and_recovers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn invalid<T U>(value: T) -> T\n  value\nend\n",
            "fn valid() -> Int\n  1\nend\n",
        ),
    );
    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.function_binder_separator")
        .expect("expected a binder separator diagnostic");
    let span = diagnostic.span.as_ref().expect("expected a primary span");
    assert_eq!(&source.text()[span.start.offset..span.end.offset], "U");
    assert_eq!(
        diagnostic.recovery.strategy,
        RecoveryStrategy::SynchronizeToAnchor
    );
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected following function declaration");
    };
    assert_eq!(function.name.as_deref(), Some("valid"));
}

#[test]
fn generic_declarations_preserve_typed_calls_and_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compare<T>(left: Int, right: Int) -> Bool\n",
            "  module::identity<Vec<T>>([])\n",
            "  left < right\n",
            "end\n",
        ),
    );
    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr: call, .. } = &function.body[0] else {
        panic!("expected typed call");
    };
    assert!(matches!(
        &call.kind,
        ExprKind::Call { callee, .. }
            if matches!(&callee.kind, ExprKind::TypeApply { type_args, .. } if type_args == &vec!["Vec<T>".to_string()])
    ));
    let BodyLine::Expr {
        expr: comparison, ..
    } = &function.body[1]
    else {
        panic!("expected comparison");
    };
    assert!(matches!(
        comparison.kind,
        ExprKind::Binary {
            op: BinaryOp::Less,
            ..
        }
    ));
}
