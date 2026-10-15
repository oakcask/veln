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
fn recovered_final_variant_retains_visibility_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub fn expose(value: State::hidden) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.variant_refinement_type"),
        "{:#?}",
        parsed.diagnostics
    );
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_private"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "type.variant_refinement_private"],
        "{diagnostics:#?}"
    );
    let visibility = veln_diagnostics::diagnostic_to_json(relevant[1]).to_json();
    assert!(visibility.contains(
        "\"written_type\":\"State::hidden\",\"base_type\":\"State\",\"variant\":\"Hidden\""
    ));
}

#[test]
fn non_unique_final_variant_recovery_suppresses_visibility_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "  Hidden\n",
            "end\n",
            "pub fn expose(value: State::hidden) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
        "{diagnostics:#?}"
    );
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

#[test]
fn union_visibility_reports_distinct_paths_in_written_order() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  First\n",
        "  pub Public\n",
        "  Second\n",
        "end\n",
        "pub fn expose(value: State::Second | State::First | State::Second) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let variants = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .map(|json| {
            if json.contains("\"variant\":\"Second\"") {
                "Second"
            } else if json.contains("\"variant\":\"First\"") {
                "First"
            } else {
                panic!("missing variant detail: {json}")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(variants, ["Second", "First"]);
}

#[test]
fn union_visibility_keeps_distinct_generic_arguments_and_removes_exact_repeats() {
    let source = concat!(
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "end\n",
        "pub fn expose(value: State<Int>::Hidden | State<String>::Hidden | State<Int>::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 2, "{diagnostics:#?}");
    let hidden_starts = source
        .match_indices("Hidden")
        .map(|(offset, _)| offset)
        .collect::<Vec<_>>();
    assert_eq!(
        visibility
            .iter()
            .map(|diagnostic| diagnostic
                .span
                .as_ref()
                .expect("visibility span")
                .start
                .offset)
            .collect::<Vec<_>>(),
        hidden_starts[1..3],
        "{diagnostics:#?}"
    );
    for (diagnostic, written_type) in visibility
        .iter()
        .zip(["State<Int>::Hidden", "State<String>::Hidden"])
    {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains(&format!("\"written_type\":\"{written_type}\"")),
            "{json}"
        );
        assert!(json.contains("\"base_type\":\"State\""), "{json}");
        assert!(json.contains("\"variant\":\"Hidden\""), "{json}");
    }

    let union_base = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_union_base")
        .collect::<Vec<_>>();
    assert_eq!(union_base.len(), 1, "{diagnostics:#?}");
    let union_json = veln_diagnostics::diagnostic_to_json(union_base[0]).to_json();
    assert!(
        union_json.contains("\"expected_base_type\":\"State<Int>\""),
        "{union_json}"
    );
    assert!(
        union_json.contains("\"actual_base_type\":\"State<String>\""),
        "{union_json}"
    );
    assert_eq!(
        union_base[0]
            .span
            .as_ref()
            .expect("union base span")
            .start
            .offset,
        source
            .find("State<String>::Hidden")
            .expect("second alternative")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_refinements_and_private_function_refinements_remain_valid() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret\n",
        "  pub Hidden\n",
        "end\n",
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "pub fn accepted(value: State::Ready) -> State::Ready\n",
        "  value\n",
        "end\n",
        "fn private(value: State::Hidden) -> State::Hidden\n",
        "  value\n",
        "end\n",
        "fn private_base(value: Secret::Hidden) -> Secret::Hidden\n",
        "  value\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn invalid_generic_arity_retains_independent_visibility_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret<A>\n",
        "  pub Hidden(A)\n",
        "end\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "  pub Ready(A)\n",
        "end\n",
        "pub type Public<A>\n",
        "  pub Open(A)\n",
        "end\n",
        "pub fn private_base(value: Secret::Hidden | Public<Int>::Open) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant(value: State::Hidden | State<Int>::Ready) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let observed = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                || diagnostic.id == "type.variant_refinement_private"
                || diagnostic.id == "type.variant_union_base"
                || diagnostic.id == "type.variant_mismatch"
        })
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        [
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );
}

mod publication_and_recovery;
