use super::*;

#[test]
fn invalid_imported_value_does_not_collide_with_a_valid_imported_function() {
    for reverse_imports in [false, true] {
        let imports = if reverse_imports {
            "use functions\nuse values\n"
        } else {
            "use values\nuse functions\n"
        };
        let module = merged_modules_with_identities(vec![
            (
                "values",
                SourceFile::new("values.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "functions",
                SourceFile::new("functions.veln", "pub fn shared() -> Int\n  2\nend\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared()\nend\n"),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().all(|diagnostic| {
                !matches!(
                    diagnostic.id.as_str(),
                    "name.ambiguous" | "type.not_callable"
                ) && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("shared"))
            }),
            "reverse_imports={reverse_imports}: {diagnostics:#?}"
        );
    }
}

#[test]
fn exact_companion_module_value_can_read_private_module_values_but_other_sources_cannot() {
    let module = merged_modules_with_identities(vec![
        (
            "math",
            SourceFile::new("math.veln", "let secret: Int = 42\n"),
        ),
        (
            "math.test",
            SourceFile::new(
                "math.test.veln",
                "use math\nlet observed: Int = math::secret\n",
            ),
        ),
        (
            "other",
            SourceFile::new("other.veln", "let marker: Int = 0\n"),
        ),
        (
            "other.test",
            SourceFile::new(
                "other.test.veln",
                "use math\nlet wrong_target: Int = math::secret\n",
            ),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                "use math\nlet ordinary: Int = math::secret\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.span.as_ref().is_none_or(|span| {
                span.file.as_str() != "math.test.veln" || diagnostic.id != "name.unresolved"
            })
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_companion_cannot_read_an_unavailable_private_module_value() {
    let module = merged_modules_with_identities(vec![
        (
            "math",
            SourceFile::new("math.veln", "let secret: Missing = 0\n"),
        ),
        (
            "math.test",
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "pub let observed: Int = math::secret\n",
                    "fn read_observed() -> String\n",
                    "  observed\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let mut environment = crate::types::TypeEnvironment::from_module(&module);
    let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
    let analysis =
        crate::analysis::check_module_values(&module, &mut environment, &mut variant_diagnostics);

    assert_eq!(
        analysis.invalid,
        [
            (None, Some("math".to_string()), "secret".to_string()),
            (None, Some("math.test".to_string()), "observed".to_string(),),
        ]
        .into_iter()
        .collect(),
    );
    assert!(matches!(
        environment.module_value_path_in_owner(
            &["math".to_string(), "secret".to_string()],
            Some("math.test"),
            None,
        ),
        crate::types::ModuleValueLookup::Missing
    ));
    assert!(matches!(
        environment.module_value_path_in_owner(&["observed".to_string()], Some("math.test"), None,),
        crate::types::ModuleValueLookup::Unavailable
    ));

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `math::secret`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.span.as_ref().is_none_or(|span| {
                span.file.as_str() != "math.test.veln" || diagnostic.id != "type.mismatch"
            })
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_module_value_annotations_reject_private_named_types_and_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  pub Visible\n",
            "  Hidden\n",
            "end\n",
            "pub let named: Secret = Value\n",
            "pub let refined: State::Hidden = Hidden\n",
            "pub let hidden_initializer: State = Hidden\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn public_module_value_visibility_recurses_through_annotation_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  Hidden\n",
            "  Other\n",
            "end\n",
            "pub type Box<T>\n",
            "  pub Box(value: T)\n",
            "end\n",
            "pub type Choice<T>\n",
            "  pub Left\n",
            "  pub Right\n",
            "end\n",
            "pub let named_generic: Box<Secret> = 0\n",
            "pub let named_function: fn(Secret) -> Secret = 0\n",
            "pub let named_record: {field: Secret} = 0\n",
            "pub let named_union: Choice<Int>::Left | Choice<Secret>::Right = 0\n",
            "pub let refined_generic: Box<State::Hidden> = 0\n",
            "pub let refined_function: fn(State::Hidden) -> State::Hidden = 0\n",
            "pub let refined_record: {field: State::Hidden} = 0\n",
            "pub let refined_union: State::Hidden | State::Other = 0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let private_named_types = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
        .collect::<Vec<_>>();
    assert_eq!(private_named_types.len(), 4, "{diagnostics:#?}");
    let union_private_type = private_named_types
        .iter()
        .find(|diagnostic| {
            let detail = |field: &str| {
                diagnostic.details.as_object().and_then(|entries| {
                    entries
                        .iter()
                        .find(|(entry, _)| entry == field)
                        .and_then(|(_, value)| value.as_text())
                })
            };
            detail("name") == Some("named_union") && detail("written_type") == Some("Secret")
        })
        .expect("private named type in the later union alternative");
    assert_diagnostic_span(union_private_type, 18, 49, 18, 55);
    let refinements = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(refinements.len(), 6, "{diagnostics:#?}");
    let rendered = format!("{refinements:#?}");
    for segment in [
        "type_argument",
        "function_parameter",
        "function_result",
        "record_field",
        "union_alternative",
    ] {
        assert!(rendered.contains(segment), "missing {segment}: {rendered}");
    }
}

#[test]
fn non_name_calls_are_rejected_and_quarantined_at_the_call_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Boxed\n",
            "  Box(value: Int)\n",
            "end\n",
            "let direct: Int = (1)()\n",
            "let nested: { value: Int } = { value: ((2))(3) }\n",
            "let payload: Boxed = Box((4)())\n",
            "let dependent: Int = direct\n",
            "let independent: Int = 1\n",
            "fn reads_direct() -> String\n",
            "  direct\n",
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
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 19, 4, 24);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 47);
    assert_diagnostic_span(unsupported[2], 6, 26, 6, 31);
    for (diagnostic, declaration) in unsupported.iter().zip(&module.module_values[..3]) {
        assert_eq!(
            diagnostic.related.first(),
            Some(&veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("module_value_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(format!(
                        "Module value `{}` is declared here.",
                        declaration.name.as_deref().expect("module value name")
                    )),
                ),
                ("span", crate::diagnostics::span_json(&declaration.span),),
            ]))
        );
    }

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(16)
    );
}

#[test]
fn module_value_annotations_are_checked_even_when_unused() {
    let source = SourceFile::new("main.veln", "let unused: String = 42\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `String`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_annotations_must_be_closed_and_quarantine_dependents() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let open: Option<T> = None\n",
            "let dependent: Option<T> = open\n",
            "fn read() -> String\n",
            "  dependent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unresolved_type_parameters = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .count();

    assert_eq!(unresolved_type_parameters, 2, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.mismatch" || !diagnostic.message.contains("found `Option<T>`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_function_annotations_validate_effect_rows_and_labels() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let unbound: fn() -> Int effects [...E] = 1\n",
            "let unknown: fn() -> Int effects [MissingAudit] = 1\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let unbound = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.row_unbound")
        .unwrap_or_else(|| panic!("expected unbound effect row: {diagnostics:#?}"));
    let unbound_span = unbound.span.as_ref().expect("unbound effect row span");
    assert_eq!(
        (unbound_span.start.line, unbound_span.start.column),
        (1, 35)
    );
    assert_eq!((unbound_span.end.line, unbound_span.end.column), (1, 39));

    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.unknown")
        .unwrap_or_else(|| panic!("expected unknown effect label: {diagnostics:#?}"));
    let unknown_span = unknown.span.as_ref().expect("unknown effect label span");
    assert_eq!(
        (unknown_span.start.line, unknown_span.start.column),
        (2, 35)
    );
    assert_eq!((unknown_span.end.line, unknown_span.end.column), (2, 47));
}

#[test]
fn module_value_annotations_and_initializers_use_existing_type_name_rules() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "end\n",
            "let good: State::Idle = State::Idle\n",
            "let wrong: Option<Int>::Absent = None\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.invalid_case"
                || !diagnostic.message.contains("module name `State`")
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.variant_refinement_unknown"
                && diagnostic.message.contains("variant `Absent`")
        }),
        "{diagnostics:#?}"
    );
}
