use super::*;

fn visibility_diagnostics(source: &str) -> Vec<Diagnostic> {
    diagnostics_for(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect()
}

#[test]
fn public_parameters_and_results_reject_private_refinement_declarations() {
    let diagnostics = visibility_diagnostics(concat!(
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "pub fn private_base_parameter(value: Secret::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_base_result() -> Secret::Hidden\n",
        "  Hidden\n",
        "end\n",
        "pub fn private_variant_parameter(value: State::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant_result() -> State::Hidden\n",
        "  Hidden\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(observed[0].contains(
        "\"details\":{\"written_type\":\"Secret::Hidden\",\"base_type\":\"Secret\",\"variant\":\"Hidden\",\"boundary\":\"public_parameter\",\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"}]}"
    ));
    assert!(observed[0].contains("\"kind\":\"private_type_declaration\""));
    assert!(observed[0].contains("\"kind\":\"public_parameter_boundary\""));
    assert!(observed[1].contains("\"boundary\":\"public_result\""));
    assert!(observed[1].contains("\"exposure_path\":[{\"kind\":\"result\"}]"));
    assert!(observed[2].contains("\"kind\":\"private_variant_declaration\""));
    assert!(observed[3].contains("\"kind\":\"public_result_boundary\""));

    let primary_spans = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.span.as_ref().expect("visibility span"))
        .map(|span| (span.start.line, span.start.column, span.end.column))
        .collect::<Vec<_>>();
    assert_eq!(
        primary_spans,
        [(8, 38, 44), (11, 33, 39), (14, 48, 54), (17, 43, 49)]
    );
}

#[test]
fn structural_positions_and_public_adt_payloads_reject_private_refinements() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "pub type Box<A>\n",
        "  pub Boxed(value: A)\n",
        "end\n",
        "pub type Alias = State\n",
        "pub type Envelope\n",
        "  pub Wrapped(value: Box<State::Hidden | State::Hidden>)\n",
        "  pub Callback(value: fn(State::Hidden) -> Box<State::Hidden>)\n",
        "end\n",
        "pub fn expose(value: Box<Alias::Hidden>) -> fn(State::Hidden) -> Box<State::Hidden>\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 7, "{diagnostics:#?}");
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(observed[0].contains("\"boundary\":\"public_adt_payload\""));
    assert!(observed[0].contains("\"kind\":\"adt_payload\""));
    assert!(observed[0].contains("\"kind\":\"type_argument\",\"index\":0"));
    assert!(observed[0].contains("\"kind\":\"union_alternative\",\"index\":0"));
    assert!(observed[1].contains("\"kind\":\"union_alternative\",\"index\":1"));
    assert!(observed[2].contains("\"boundary\":\"public_adt_payload\""));
    assert!(observed[2].contains("\"kind\":\"function_parameter\",\"index\":0"));
    assert!(observed[3].contains("\"kind\":\"function_result\""));
    assert!(observed[4].contains("\"boundary\":\"public_parameter\""));
    assert!(observed[4].contains("\"kind\":\"type_argument\",\"index\":0"));
    assert!(observed[4].contains("\"kind\":\"type_alias_declaration\""));
    assert!(observed[5].contains("\"kind\":\"function_parameter\",\"index\":0"));
    assert!(observed[6].contains("\"kind\":\"function_result\""));
    assert!(observed[6].contains("\"kind\":\"type_argument\",\"index\":0"));
}

#[test]
fn function_and_adt_visibility_diagnostics_follow_source_order() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn expose(value: State::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub type Envelope\n",
        "  pub Wrapped(value: State::Hidden)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let boundaries = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>();
    assert!(boundaries[0].contains("\"boundary\":\"public_parameter\""));
    assert!(boundaries[1].contains("\"boundary\":\"public_adt_payload\""));
}

#[test]
fn recursive_public_adt_visibility_terminates_and_reports_nested_alias_exposure() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub type Alias = State\n",
        "pub type Box<A>\n",
        "  pub Boxed(value: A)\n",
        "end\n",
        "pub type Recursive\n",
        "  pub Next(value: Recursive)\n",
        "  pub Leak(value: Box<Alias::Hidden>)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let observed = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    assert!(observed.contains("\"kind\":\"type_argument\",\"index\":0"));
    assert!(observed.contains("\"kind\":\"type_alias_declaration\""));
    assert!(observed.contains("\"type\":\"Recursive\",\"variant\":\"Leak\""));
}

#[test]
fn invalid_public_adt_keeps_local_identity_but_is_not_importable() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod api\n",
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub type Box<A>\n",
            "  pub Boxed(value: A)\n",
            "end\n",
            "pub type Envelope\n",
            "  pub Wrapped(value: Box<State::Hidden>)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let environment = TypeEnvironment::from_module(&module);
    assert!(
        environment
            .adts
            .descriptor_for_type_path("Envelope", 0, Some("api"), &environment.uses)
            .is_some(),
        "same-module recovery keeps the unambiguous source identity"
    );

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new("consumer.veln", "mod consumer\nuse api\n")).tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    assert!(
        consumer_environment
            .adts
            .descriptor_for_type_path(
                "api::Envelope",
                0,
                Some("consumer"),
                &consumer_environment.uses,
            )
            .is_none(),
        "visibility-invalid ADT must not enter an imported public signature"
    );
}

#[test]
fn record_fields_report_distinct_nested_paths_and_alias_hops() {
    let diagnostics = visibility_diagnostics(concat!(
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub type State\n",
        "  Hidden\n",
        "  pub Ready\n",
        "end\n",
        "pub type Inner = State\n",
        "pub type Outer = Inner\n",
        "pub fn expose(value: {first: State::Hidden, nested: {second: Outer::Hidden | State::Hidden}}) -> {result: Secret::Hidden}\n",
        "  {result: Secret::Hidden}\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(observed[0].contains(concat!(
        "\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"},",
        "{\"kind\":\"record_field\",\"name\":\"first\"}]"
    )));
    assert!(observed[1].contains(concat!(
        "\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"},",
        "{\"kind\":\"record_field\",\"name\":\"nested\"},",
        "{\"kind\":\"record_field\",\"name\":\"second\"}]"
    )));
    assert!(observed[1].contains("\"kind\":\"type_alias_declaration\""));
    assert_eq!(observed[1].matches("type_alias_declaration").count(), 2);
    assert!(observed[2].contains("\"written_type\":\"State::Hidden\""));
    assert!(observed[3].contains(concat!(
        "\"exposure_path\":[{\"kind\":\"result\"},",
        "{\"kind\":\"record_field\",\"name\":\"result\"}]"
    )));

    let primary_text = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("visibility span");
            (span.start.column, span.end.column)
        })
        .collect::<Vec<_>>();
    assert_eq!(primary_text, [(30, 43), (62, 91), (62, 91), (107, 121)]);
}

#[test]
fn deep_record_field_path_is_materialized_for_the_visibility_diagnostic() {
    const DEPTH: usize = 64;
    let mut annotation = String::with_capacity(DEPTH * 8 + 13);
    for _ in 0..DEPTH {
        annotation.push_str("{next: ");
    }
    annotation.push_str("State::Hidden");
    for _ in 0..DEPTH {
        annotation.push('}');
    }
    let diagnostics = visibility_diagnostics(&format!(
        "pub type State\n  Hidden\nend\npub fn expose(value: {annotation}) -> ()\n  ()\nend\n"
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let observed = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    assert_eq!(
        observed.matches("record_field").count(),
        DEPTH,
        "{observed}"
    );
    assert!(observed.contains("\"name\":\"next\""), "{observed}");
}

#[test]
fn public_structural_refinements_are_checked_while_private_functions_remain_valid() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "  pub Ready\n",
            "end\n",
            "pub fn accepted(value: {state: State::Ready}) -> {state: State::Ready}\n",
            "  value\n",
            "end\n",
            "fn private_allowed(value: {state: State::Hidden}) -> {state: State::Hidden}\n",
            "  value\n",
            "end\n",
            "pub fn function_record_boundary(value: {callback: fn() -> {state: State::Hidden, nested: {again: State::Hidden}}, sibling: Int}) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 1, "{diagnostics:#?}");
    for diagnostic in visibility {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(json.contains("\"kind\":\"function_result\""), "{json}");
    }
    assert!(TypeEnvironment::from_module(&module).function_is_published_for_test("accepted"));
    assert!(
        !TypeEnvironment::from_module(&module)
            .function_is_published_for_test("function_record_boundary")
    );
}

#[test]
fn square_bracket_nested_records_do_not_extend_record_visibility() {
    let diagnostics = diagnostics_for(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn boundary(value: {wrapped: [{inside: State::Hidden}], direct: State::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.invalid_annotation", "type.variant_refinement_private"],
        "{diagnostics:#?}"
    );
    let visibility = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(visibility.contains("\"written_type\":\"State::Hidden\""));
    assert!(visibility.contains(concat!(
        "\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"},",
        "{\"kind\":\"record_field\",\"name\":\"direct\"}]"
    )));
}

#[test]
fn function_field_effect_rows_do_not_hide_later_record_refinements() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn exposed(value: {callback: fn() -> {inside: State::Hidden} effects [stdio, net], state: State::Hidden}) -> () effects [stdio, net]\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let nested = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    assert!(nested.contains("\"kind\":\"function_result\""), "{nested}");
    let observed = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(observed.contains(concat!(
        "\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"},",
        "{\"kind\":\"record_field\",\"name\":\"state\"}]"
    )));
    assert!(observed.contains("\"written_type\":\"State::Hidden\""));
}

#[test]
fn function_field_non_refinement_prefix_does_not_capture_later_record_refinement() {
    fn direct_visibility(source: &str) -> (Vec<Diagnostic>, veln_source::SourceSpan) {
        let source = SourceFile::new("main.veln", source);
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        let direct_span = module.functions[0].params[0].ty_refinements[0].span.clone();
        let diagnostics = analyze_surface_module(&module)
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .collect();
        (diagnostics, direct_span)
    }

    let prefix = concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn expose(value: {callback: fn() -> State::Hidden<Int>, direct: State::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let control = concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn expose(value: {callback: fn() -> Int, direct: State::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
    );

    let (prefix_diagnostics, prefix_direct_span) = direct_visibility(prefix);
    let (control_diagnostics, control_direct_span) = direct_visibility(control);
    assert_eq!(prefix_diagnostics.len(), 1, "{prefix_diagnostics:#?}");
    assert_eq!(control_diagnostics.len(), 1, "{control_diagnostics:#?}");
    assert_eq!(
        prefix_diagnostics[0].span.as_ref(),
        Some(&prefix_direct_span)
    );
    assert_eq!(
        control_diagnostics[0].span.as_ref(),
        Some(&control_direct_span)
    );

    for diagnostic in [&prefix_diagnostics[0], &control_diagnostics[0]] {
        let observed = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(observed.contains("\"written_type\":\"State::Hidden\""));
        assert!(observed.contains(concat!(
            "\"exposure_path\":[{\"kind\":\"parameter\",\"name\":\"value\"},",
            "{\"kind\":\"record_field\",\"name\":\"direct\"}]"
        )));
    }

    let prefix_module = lower_surface_ast(&parse(&SourceFile::new("main.veln", prefix)).tree);
    let lowered = lower_checked_surface_module(&prefix_module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn unique_casing_recovery_retains_direct_visibility_identity() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn private_base_parameter(value: secret::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_base_result() -> secret::Hidden\n",
        "  Secret::Hidden\n",
        "end\n",
        "pub fn private_variant_parameter(value: state::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant_result() -> state::Hidden\n",
        "  State::Hidden\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_private"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relevant.len(), 8, "{diagnostics:#?}");
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "name.invalid_case",
            "name.invalid_case",
            "name.invalid_case",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );

    let visibility = relevant
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(visibility[0].contains(
        "\"written_type\":\"secret::Hidden\",\"base_type\":\"Secret\",\"variant\":\"Hidden\",\"boundary\":\"public_parameter\""
    ));
    assert!(visibility[1].contains(
        "\"written_type\":\"secret::Hidden\",\"base_type\":\"Secret\",\"variant\":\"Hidden\",\"boundary\":\"public_result\""
    ));
    assert!(visibility[2].contains(
        "\"written_type\":\"state::Hidden\",\"base_type\":\"State\",\"variant\":\"Hidden\",\"boundary\":\"public_parameter\""
    ));
    assert!(visibility[3].contains(
        "\"written_type\":\"state::Hidden\",\"base_type\":\"State\",\"variant\":\"Hidden\",\"boundary\":\"public_result\""
    ));
}

#[test]
fn exact_duplicate_variants_do_not_create_visibility_identity_or_quarantine() {
    for public_duplicate_first in [true, false] {
        let variants = if public_duplicate_first {
            "  pub Hidden\n  Hidden\n"
        } else {
            "  Hidden\n  pub Hidden\n"
        };
        let source = SourceFile::new(
            "main.veln",
            format!(
                "pub type State\n{variants}end\n\
                 pub fn expose_parameter(value: State::Hidden) -> ()\n  ()\nend\n\
                 pub fn expose_result() -> State::Hidden\n  Hidden\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let environment = TypeEnvironment::from_module(&module);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            ["name.duplicate"],
            "order public-first={public_duplicate_first}: {diagnostics:#?}"
        );
        for function in ["expose_parameter", "expose_result"] {
            assert!(
                environment.function_is_published_for_test(function),
                "{function} must keep the same recovery publication state when public-first={public_duplicate_first}"
            );
        }
    }
}

#[test]
fn duplicate_union_alternative_preserves_independent_visibility_failure() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Hidden\n",
            "  Hidden\n",
            "  Private\n",
            "end\n",
            "pub fn expose(value: State::Hidden | State::Private) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.duplicate", "type.variant_refinement_private"],
        "{diagnostics:#?}"
    );
    let visibility = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(
        visibility.contains("\"variant\":\"Private\""),
        "{visibility}"
    );
    assert!(!environment.function_is_published_for_test("expose"));
}

#[test]
fn non_unique_casing_recovery_does_not_manufacture_visibility_identity() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Hidden\n",
        "end\n",
        "type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn expose(value: state::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
        "{diagnostics:#?}"
    );
}

mod import_visibility;

#[test]
fn colliding_implicit_import_qualifiers_do_not_select_by_type_leaf() {
    for first_with_state_first in [true, false] {
        let first = SourceFile::new("first-shared.veln", "pub type State\n  Hidden\nend\n");
        let second = SourceFile::new("second-shared.veln", "pub type Other\n  pub Ready\nend\n");
        let consumer = SourceFile::new(
            "consumer.veln",
            concat!(
                "use first::shared from \"first\"\n",
                "use second::shared from \"second\"\n",
                "pub fn expose(value: shared::State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );
        let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&consumer).tree,
            "consumer".to_string(),
            consumer.span(veln_source::TextRange::new(0, 0)),
        );
        let sources = if first_with_state_first {
            vec![
                ("first::shared", first),
                ("second::shared", second),
                ("consumer", consumer),
            ]
        } else {
            vec![
                ("second::shared", second),
                ("first::shared", first),
                ("consumer", consumer),
            ]
        };
        let mut project = merged_modules_with_identities(sources);
        project.module = consumer_identity.module;
        let environment = TypeEnvironment::from_module(&project);
        let diagnostics = analyze_surface_module(&project);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
            "order={first_with_state_first} diagnostics={diagnostics:#?}"
        );
        let signature = environment.function("expose").expect("recovery signature");
        assert_eq!(signature.params, [Type::Unknown]);
        assert_eq!(signature.return_type, Type::unit());
        assert!(environment.function_is_published_for_test("expose"));
        let (_, lowered) = check_project_surface_module(&project);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn exact_import_path_precedes_a_colliding_implicit_qualifier() {
    for exact_module_first in [true, false] {
        let exact = SourceFile::new("shared.veln", "pub type State\n  Hidden\nend\n");
        let implicit = SourceFile::new("nested-shared.veln", "pub type State\n  pub Hidden\nend\n");
        let consumer = SourceFile::new(
            "consumer.veln",
            concat!(
                "use shared\n",
                "use nested::shared from \"dependency\"\n",
                "pub fn expose(value: shared::State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );
        let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&consumer).tree,
            "consumer".to_string(),
            consumer.span(veln_source::TextRange::new(0, 0)),
        );
        let sources = if exact_module_first {
            vec![
                ("shared", exact),
                ("nested::shared", implicit),
                ("consumer", consumer),
            ]
        } else {
            vec![
                ("nested::shared", implicit),
                ("shared", exact),
                ("consumer", consumer),
            ]
        };
        let mut project = merged_modules_with_identities(sources);
        project.module = consumer_identity.module;
        let environment = TypeEnvironment::from_module(&project);
        let diagnostics = analyze_surface_module(&project);
        let visibility = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .collect::<Vec<_>>();
        assert_eq!(visibility.len(), 1, "{diagnostics:#?}");
        let json = veln_diagnostics::diagnostic_to_json(visibility[0]).to_json();
        assert!(json.contains("\"written_type\":\"shared::State::Hidden\""));
        assert!(json.contains("\"file\":\"shared.veln\""));
        assert!(!environment.function_is_published_for_test("expose"));
        let (_, lowered) = check_project_surface_module(&project);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn alias_qualifier_keeps_colliding_implicit_imports_unresolved_in_all_orders() {
    for imports_reversed in [false, true] {
        for sources_reversed in [false, true] {
            let project = alias_import_collision_project(
                AliasImportCollision::Ambiguous,
                imports_reversed,
                sources_reversed,
            );
            let environment = TypeEnvironment::from_module(&project);
            let diagnostics = analyze_surface_module(&project);
            assert!(
                diagnostics
                    .iter()
                    .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
                "imports_reversed={imports_reversed} sources_reversed={sources_reversed} diagnostics={diagnostics:#?}"
            );
            let signature = environment.function("expose").expect("recovery signature");
            assert_eq!(signature.params, [Type::Unknown]);
            assert_eq!(signature.return_type, Type::unit());
            assert!(environment.function_is_published_for_test("expose"));
            let (_, lowered) = check_project_surface_module(&project);
            assert!(lowered.core.is_none());
            assert!(lowered.ir.is_none());
        }
    }
}

#[test]
fn alias_qualifier_prefers_exact_import_over_implicit_collision_in_all_orders() {
    for imports_reversed in [false, true] {
        for sources_reversed in [false, true] {
            let project = alias_import_collision_project(
                AliasImportCollision::ExactPrecedence,
                imports_reversed,
                sources_reversed,
            );
            let environment = TypeEnvironment::from_module(&project);
            let diagnostics = analyze_surface_module(&project);
            let visibility = diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
                .collect::<Vec<_>>();
            assert_eq!(
                visibility.len(),
                1,
                "imports_reversed={imports_reversed} sources_reversed={sources_reversed} diagnostics={diagnostics:#?}"
            );
            let json = veln_diagnostics::diagnostic_to_json(visibility[0]).to_json();
            assert!(
                json.contains("\"written_type\":\"Facade::Hidden\""),
                "{json}"
            );
            assert!(
                json.contains("\"kind\":\"type_alias_declaration\""),
                "{json}"
            );
            assert!(json.contains("\"file\":\"shared.veln\""), "{json}");
            assert!(!environment.function_is_published_for_test("expose"));
            let (_, lowered) = check_project_surface_module(&project);
            assert!(lowered.core.is_none());
            assert!(lowered.ir.is_none());
        }
    }
}

#[derive(Clone, Copy)]
enum AliasImportCollision {
    Ambiguous,
    ExactPrecedence,
}

fn alias_import_collision_project(
    collision: AliasImportCollision,
    imports_reversed: bool,
    sources_reversed: bool,
) -> SurfaceModule {
    let (first_name, first, second_name, second, first_import, second_import) = match collision {
        AliasImportCollision::Ambiguous => (
            "first::shared",
            SourceFile::new("first-shared.veln", "pub type State\n  Hidden\nend\n"),
            "second::shared",
            SourceFile::new("second-shared.veln", "pub type Other\n  pub Ready\nend\n"),
            "use first::shared from \"first\"\n",
            "use second::shared from \"second\"\n",
        ),
        AliasImportCollision::ExactPrecedence => (
            "shared",
            SourceFile::new("shared.veln", "pub type State\n  Hidden\nend\n"),
            "nested::shared",
            SourceFile::new("nested-shared.veln", "pub type State\n  pub Hidden\nend\n"),
            "use shared\n",
            "use nested::shared from \"dependency\"\n",
        ),
    };
    let imports = if imports_reversed {
        format!("{second_import}{first_import}")
    } else {
        format!("{first_import}{second_import}")
    };
    let consumer = SourceFile::new(
        "consumer.veln",
        format!(
            "{imports}pub type Facade = shared::State\n\
             pub fn expose(value: Facade::Hidden) -> ()\n  ()\nend\n"
        ),
    );
    let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&consumer).tree,
        "consumer".to_string(),
        consumer.span(veln_source::TextRange::new(0, 0)),
    );
    let mut sources = if sources_reversed {
        vec![(second_name, second), (first_name, first)]
    } else {
        vec![(first_name, first), (second_name, second)]
    };
    sources.push(("consumer", consumer));
    let mut project = merged_modules_with_identities(sources);
    project.module = consumer_identity.module;
    project
}

fn assert_colliding_recovery_signatures(environment: &TypeEnvironment) {
    for name in ["exact", "invalid_arity", "recovered_case"] {
        let signature = environment
            .function(name)
            .unwrap_or_else(|| panic!("{name} recovery signature"));
        assert_eq!(signature.params, [Type::Unknown], "{name}");
        assert_eq!(signature.return_type, Type::Unknown, "{name}");
        assert!(!environment.function_is_published_for_test(name));
    }
}

fn assert_colliding_visibility_diagnostics(diagnostics: &[Diagnostic]) {
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 3, "{diagnostics:#?}");
    for (observed, written_type) in visibility.iter().zip([
        "imported::State<Int>::Hidden",
        "imported::State::Hidden",
        "imported::state<Int>::Hidden",
    ]) {
        for expected in [
            format!("\"written_type\":\"{written_type}\""),
            "\"base_type\":\"imported::State\"".to_string(),
            "\"variant\":\"Hidden\"".to_string(),
            "\"boundary\":\"public_parameter\"".to_string(),
            "\"kind\":\"private_variant_declaration\"".to_string(),
            "\"file\":\"imported.veln\"".to_string(),
            "\"kind\":\"public_parameter_boundary\"".to_string(),
            "\"file\":\"consumer.veln\"".to_string(),
        ] {
            assert!(observed.contains(&expected), "{observed}");
        }
    }
}

fn assert_colliding_downstream_publication(environment: &TypeEnvironment) {
    let downstream = lower_surface_ast(
        &parse(&SourceFile::new(
            "downstream.veln",
            "mod downstream\nuse consumer\n",
        ))
        .tree,
    );
    let downstream = TypeEnvironment::from_module_with_base_for_test(&downstream, environment);
    for name in ["exact", "invalid_arity", "recovered_case"] {
        assert!(
            downstream
                .function_path(
                    &["consumer".to_string(), name.to_string()],
                    Some("downstream"),
                )
                .is_none(),
            "{name} must not enter the package environment"
        );
    }
    assert!(
        downstream
            .function_path(
                &["consumer".to_string(), "local_control".to_string()],
                Some("downstream"),
            )
            .is_some()
    );
}

fn colliding_qualified_refinement_project(imported_first: bool) -> SurfaceModule {
    let imported = SourceFile::new(
        "imported.veln",
        concat!("pub type State<A>\n", "  Hidden(A)\n", "end\n",),
    );
    let consumer = SourceFile::new(
        "consumer.veln",
        concat!(
            "use imported\n",
            "pub type State<A>\n",
            "  pub Hidden(A)\n",
            "end\n",
            "pub fn exact(value: imported::State<Int>::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn invalid_arity(value: imported::State::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn recovered_case(value: imported::state<Int>::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn local_control(value: State<Int>::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let consumer_identity = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&consumer).tree,
        "consumer".to_string(),
        consumer.span(veln_source::TextRange::new(0, 0)),
    );
    let sources = if imported_first {
        vec![("imported", imported), ("consumer", consumer)]
    } else {
        vec![("consumer", consumer), ("imported", imported)]
    };
    let mut project = merged_modules_with_identities(sources);
    project.module = consumer_identity.module;
    project
}

mod union_visibility;

mod publication_and_recovery;
