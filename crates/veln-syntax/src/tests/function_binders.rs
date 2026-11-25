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
