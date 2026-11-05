use super::super::*;

#[test]
fn constructor_arity_and_unresolved_names_keep_their_existing_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Active(count: Int)\n",
            "end\n",
            "let wrong_arity: State = Active()\n",
            "let unknown: Int = missing()\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "core.constructor_arity_mismatch"
                && diagnostic.message == "state constructor expects 1 argument, but got 0"
        }),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `missing`"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "value.module_initializer_unsupported"),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_cycles_report_once_per_component_on_a_closing_reference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let alpha: Int = beta\n",
            "let beta: Int = alpha\n",
            "let self_cycle: Int = self_cycle\n",
            "let independent: Int = 1\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let cycles = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .collect::<Vec<_>>();

    assert_eq!(cycles.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        cycles[0].span.as_ref(),
        Some(&module.module_values[1].initializer.span)
    );
    assert_eq!(
        cycles[1].span.as_ref(),
        Some(&module.module_values[2].initializer.span)
    );
    assert_eq!(
        cycles[0].details,
        veln_diagnostics::JsonValue::object([
            ("phase", veln_diagnostics::JsonValue::string("type_check")),
            ("name", veln_diagnostics::JsonValue::string("beta")),
            ("target", veln_diagnostics::JsonValue::string("alpha")),
        ])
    );
    assert_eq!(
        cycles[0].related,
        vec![
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `alpha` participates in this initializer cycle.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `beta` participates in this initializer cycle.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[1].span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_reference"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `alpha` depends on `beta` here.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].initializer.span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_reference"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `beta` depends on `alpha` here.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[1].initializer.span),
                ),
            ]),
        ]
    );
}

#[test]
fn module_value_cycle_selection_is_independent_of_declaration_order() {
    let forward = cycle_facts("let alpha: Int = beta\nlet beta: Int = alpha\n");
    let reversed = cycle_facts("let beta: Int = alpha\nlet alpha: Int = beta\n");

    assert_eq!(forward, reversed);
}

#[test]
fn module_value_cycle_diagnostics_are_independent_of_source_enumeration_order() {
    let analyze = |sources| {
        let module = merged_modules_with_identities(sources);
        let diagnostics = analyze_surface_module(&module);
        let cycle = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
            .expect("cycle diagnostic");
        veln_diagnostics::JsonValue::object([
            ("id", veln_diagnostics::JsonValue::string(&cycle.id)),
            (
                "severity",
                veln_diagnostics::JsonValue::string(format!("{:?}", cycle.severity)),
            ),
            (
                "kind",
                veln_diagnostics::JsonValue::string(format!("{:?}", cycle.kind)),
            ),
            (
                "message",
                veln_diagnostics::JsonValue::string(cycle.message.to_string()),
            ),
            (
                "span",
                cycle
                    .span
                    .as_ref()
                    .map(crate::diagnostics::span_json)
                    .unwrap_or(veln_diagnostics::JsonValue::Null),
            ),
            ("details", cycle.details.clone()),
            (
                "related",
                veln_diagnostics::JsonValue::array(cycle.related.clone()),
            ),
        ])
    };
    let alpha = || SourceFile::new("alpha.veln", "let alpha: Int = beta\n");
    let beta = || SourceFile::new("beta.veln", "let beta: Int = alpha\n");

    let forward = analyze(vec![("app", alpha()), ("app", beta())]);
    let reversed = analyze(vec![("app", beta()), ("app", alpha())]);

    assert_eq!(forward, reversed);
}

fn cycle_facts(text: &str) -> (String, String, Vec<String>) {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let cycle = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .expect("cycle diagnostic");
    (
        cycle_detail(cycle, "name"),
        cycle_detail(cycle, "target"),
        cycle_related_messages(cycle),
    )
}

pub(in crate::tests::module_values) fn cycle_detail(cycle: &Diagnostic, name: &str) -> String {
    cycle
        .details
        .as_object()
        .and_then(|entries| {
            entries
                .iter()
                .find(|(field, _)| field == name)
                .and_then(|(_, value)| value.as_text())
        })
        .expect("cycle detail")
        .to_string()
}

pub(in crate::tests::module_values) fn cycle_related_messages(cycle: &Diagnostic) -> Vec<String> {
    cycle
        .related
        .iter()
        .filter_map(|note| {
            note.as_object().and_then(|entries| {
                entries
                    .iter()
                    .find(|(field, _)| field == "message")
                    .and_then(|(_, value)| value.as_text())
                    .map(str::to_owned)
            })
        })
        .collect()
}

#[test]
fn module_values_share_the_function_value_namespace() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn answer() -> Int\n",
            "  1\n",
            "end\n",
            "let answer: Int = 2\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate module value declaration name `answer`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_module_values_are_quarantined_independently_of_source_order() {
    fn assert_quarantine(first: &str, second: &str) {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "{first}{second}let dependent: Int = shared\nlet independent: Int = 1\nfn reads_dependent() -> String\n  dependent\nend\nfn reads_independent() -> String\n  independent\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);
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
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "value.module_initializer_cycle"),
            "{diagnostics:#?}"
        );
    }

    assert_quarantine(
        "let shared: Int = shared\n",
        "let shared: String = \"one\"\n",
    );
    assert_quarantine(
        "let shared: String = \"one\"\n",
        "let shared: Int = shared\n",
    );
}

#[test]
fn invalid_cased_module_values_quarantine_their_dependents() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let BadName: Int = 1\n",
            "let dependent: Int = BadName\n",
            "let independent: Int = 1\n",
            "fn reads_invalid() -> String\n",
            "  BadName\n",
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
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(11)
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn callable_module_value_collisions_quarantine_dependents_but_keep_unrelated_facts() {
    for collision in [
        concat!("fn shared() -> Int\n", "  1\n", "end\n"),
        concat!(
            "fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn shared = original\n",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "{collision}let shared: Int = 1\nlet dependent: Int = shared\nlet independent: Int = 1\nfn reads_dependent() -> String\n  dependent\nend\nfn reads_independent() -> String\n  independent\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);
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
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.duplicate"),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn module_value_call_targets_do_not_fall_through_to_callable_lookup() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "fn use_answer() -> Int\n",
            "  answer()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.not_callable")
        .expect("non-callable module value diagnostic");
    assert_eq!(diagnostic.message, "module value `answer` is not callable");
    assert_diagnostic_span(diagnostic, 3, 3, 3, 9);
    let detail = |name: &str| {
        diagnostic.details.as_object().and_then(|entries| {
            entries
                .iter()
                .find(|(field, _)| field == name)
                .and_then(|(_, value)| value.as_text())
        })
    };
    assert_eq!(detail("phase"), Some("type_check"));
    assert_eq!(detail("constraint"), Some("call_target"));
    assert_eq!(detail("actual"), Some("module_value"));
    assert_eq!(detail("name"), Some("answer"));
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(
        diagnostic.related[0],
        veln_diagnostics::JsonValue::object([
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
        ])
    );
}
