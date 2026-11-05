use super::super::*;

#[test]
fn typed_module_value_call_targets_select_the_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "fn use_answer() -> Int\n",
            "  answer<String>()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let not_callable = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.not_callable")
        .collect::<Vec<_>>();

    assert_eq!(not_callable.len(), 1, "{diagnostics:#?}");
    let diagnostic = not_callable[0];
    assert_diagnostic_span(diagnostic, 3, 3, 3, 17);
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "name.unresolved"
            || diagnostic.message != "unresolved call_target `answer`"
    }));
    assert_eq!(
        diagnostic.details.as_object().and_then(|entries| {
            entries
                .iter()
                .find(|(name, _)| name == "actual")
                .and_then(|(_, value)| value.as_text())
        }),
        Some("module_value")
    );
    assert_eq!(
        diagnostic.related,
        vec![veln_diagnostics::JsonValue::object([
            (
                "kind",
                veln_diagnostics::JsonValue::string("module_value_declaration"),
            ),
            (
                "message",
                veln_diagnostics::JsonValue::string("Module value `answer` is declared here."),
            ),
            (
                "span",
                crate::diagnostics::span_json(&module.module_values[0].span),
            ),
        ])]
    );
}

#[test]
fn unavailable_module_value_calls_reserve_names_without_selecting_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let invalid: Int = 1 + 2\n",
            "let dependent: Int = invalid\n",
            "let independent: Int = 1\n",
            "fn reads_invalid() -> String\n",
            "  invalid\n",
            "end\n",
            "fn calls_invalid() -> String\n",
            "  invalid(missing_argument)\n",
            "end\n",
            "fn calls_dependent() -> String\n",
            "  dependent()\n",
            "end\n",
            "fn reads_dependent() -> String\n",
            "  dependent\n",
            "end\n",
            "fn reads_independent() -> String\n",
            "  independent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.not_callable"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `missing_argument`"
        }),
        "{diagnostics:#?}"
    );
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(17)
    );
}

#[test]
fn typed_unavailable_module_value_calls_reserve_names_and_check_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let invalid: Int = 1 + 2\n",
            "fn calls_invalid() -> String\n",
            "  invalid<String>(missing_argument)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.not_callable"
                && !(diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved call_target `invalid`")
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `missing_argument`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn available_qualified_module_value_calls_select_the_declaration_after_local_shadowing() {
    let module = merged_modules_with_identities(vec![(
        "app::config",
        SourceFile::new(
            "main.veln",
            concat!(
                "let answer: Int = 1\n",
                "fn local_shadow(answer: fn() -> Int) -> Int\n",
                "  answer()\n",
                "end\n",
                "fn leaf_call() -> Int\n",
                "  config::answer()\n",
                "end\n",
                "fn complete_call() -> Int\n",
                "  app::config::answer()\n",
                "end\n",
            ),
        ),
    )]);

    let diagnostics = analyze_surface_module(&module);
    let not_callable = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.not_callable")
        .collect::<Vec<_>>();

    assert_eq!(not_callable.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        not_callable
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        vec![Some(6), Some(9)]
    );
    assert!(not_callable.iter().all(|diagnostic| {
        diagnostic.related.first()
            == Some(&veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("module_value_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string("Module value `answer` is declared here."),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].span),
                ),
            ]))
    }));
}

#[test]
fn typed_qualified_module_value_calls_select_the_same_declaration() {
    let module = merged_modules_with_identities(vec![(
        "app::config",
        SourceFile::new(
            "main.veln",
            concat!(
                "let answer: Int = 1\n",
                "fn leaf_call() -> Int\n",
                "  config::answer<String>()\n",
                "end\n",
                "fn complete_call() -> Int\n",
                "  app::config::answer<String>()\n",
                "end\n",
            ),
        ),
    )]);

    let diagnostics = analyze_surface_module(&module);
    let not_callable = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.not_callable")
        .collect::<Vec<_>>();

    assert_eq!(not_callable.len(), 2, "{diagnostics:#?}");
    assert!(not_callable.iter().all(|diagnostic| {
        diagnostic.related.first()
            == Some(&veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("module_value_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string("Module value `answer` is declared here."),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].span),
                ),
            ]))
    }));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "name.unresolved"
            || (!diagnostic.message.contains("config::answer")
                && !diagnostic.message.contains("app::config::answer"))
    }));
}

#[test]
fn typed_local_callable_shadowing_prevents_module_value_selection() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 1\n",
            "fn local_shadow(answer: fn() -> Int) -> Int\n",
            "  answer<String>()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.not_callable"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.related.iter().all(|related| {
                related.as_object().is_none_or(|fields| {
                    fields.iter().all(|(name, value)| {
                        name != "kind" || value.as_text() != Some("module_value_declaration")
                    })
                })
            })
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn colliding_module_value_calls_never_select_a_declaration_or_callable() {
    fn assert_collision(prefix: &str, suffix: &str) {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "{prefix}{suffix}let dependent: Int = shared\nlet independent: Int = 1\nfn reads_shared() -> String\n  shared\nend\nfn calls_shared() -> Int\n  shared(\"wrong\")\nend\nfn calls_dependent() -> Int\n  dependent()\nend\nfn reads_independent() -> String\n  independent\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.not_callable"),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.duplicate"),
            "{diagnostics:#?}"
        );
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();
        assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
        assert!(
            mismatches[0]
                .message
                .contains("expected `String`, but found `Int`"),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.related.iter().all(|related| {
                    related.as_object().is_none_or(|fields| {
                        fields.iter().all(|(name, value)| {
                            name != "kind" || value.as_text() != Some("module_value_declaration")
                        })
                    })
                })
            }),
            "{diagnostics:#?}"
        );
    }

    let function = concat!("fn shared(value: Int) -> Int\n", "  value\n", "end\n",);
    let alias = concat!(
        "fn original(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "pub fn shared = original\n",
    );
    let value = "let shared: Int = 1\n";
    assert_collision(function, value);
    assert_collision(value, function);
    assert_collision(alias, value);
    assert_collision(value, alias);
    assert_collision("let shared: Int = 1\n", "let shared: String = \"one\"\n");
    assert_collision("let shared: String = \"one\"\n", "let shared: Int = 1\n");
}
