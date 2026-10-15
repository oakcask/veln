use super::*;

#[test]
fn recovered_union_visibility_failure_quarantines_all_publication_outputs() {
    let source = recovered_union_visibility_source();
    let parsed = parse(&source);
    assert_eq!(
        parsed
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "parse.variant_refinement_type")
            .count(),
        4,
        "{:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);

    assert_recovered_union_visibility_diagnostics(&source, &module);
    assert_recovered_union_publication_quarantine(&module);
}

fn recovered_union_visibility_source() -> SourceFile {
    SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type Arg\n",
            "  pub Value\n",
            "end\n",
            "pub type State<A>\n",
            "  Hidden(A)\n",
            "  Private(A)\n",
            "end\n",
            "pub fn exposed(value: State::hidden | State<main::Arg>::Private) -> State::hidden | State<main::Arg>::Private\n",
            "  Hidden(Value)\n",
            "end\n",
            "pub fn exposed_alias = exposed\n",
        ),
    )
}

fn assert_recovered_union_visibility_diagnostics(source: &SourceFile, module: &SurfaceModule) {
    let diagnostics = analyze_surface_module(module);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 4, "{diagnostics:#?}");
    let observed = visibility
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    for (diagnostic, (written_type, boundary)) in observed.iter().zip([
        ("State::hidden", "public_parameter"),
        ("State<main::Arg>::Private", "public_parameter"),
        ("State::hidden", "public_result"),
        ("State<main::Arg>::Private", "public_result"),
    ]) {
        let variant = if written_type.ends_with("hidden") {
            "Hidden"
        } else {
            "Private"
        };
        let path_kind = if boundary == "public_parameter" {
            "parameter"
        } else {
            "result"
        };
        assert!(
            diagnostic.contains(&format!(
                "\"written_type\":\"{written_type}\",\"base_type\":\"main::State\",\"variant\":\"{variant}\",\"boundary\":\"{boundary}\",\"exposure_path\":[{{\"kind\":\"{path_kind}\""
            )),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("\"kind\":\"private_variant_declaration\""),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains(&format!("\"kind\":\"{boundary}_boundary\"")),
            "{diagnostic}"
        );
    }
    let private_offsets = source
        .text()
        .match_indices("::Private")
        .map(|(offset, _)| offset + "::".len())
        .collect::<Vec<_>>();
    let expected_primary_offsets = [
        source
            .text()
            .find("hidden")
            .expect("parameter recovered variant"),
        private_offsets[0],
        source
            .text()
            .rfind("hidden")
            .expect("result recovered variant"),
        private_offsets[1],
    ];
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
        expected_primary_offsets,
        "{diagnostics:#?}"
    );
}

fn assert_recovered_union_publication_quarantine(module: &SurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    let signature = environment.function("exposed").expect("recovery signature");
    assert_eq!(signature.params, [Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("exposed_alias").is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "exposed_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }

    let lowered = lower_checked_surface_module(module);
    assert!(
        lowered.core.is_none(),
        "invalid declaration must not be checked"
    );
    assert!(
        lowered.ir.is_none(),
        "invalid declaration must not produce IR"
    );
}

#[test]
fn recovered_root_unions_ignore_source_trivia_at_each_public_boundary() {
    let cases = [
        (
            "compact_parameter",
            "pub fn exposed(value: State::hidden|State::Private) -> ()\n  ()\nend\n",
            "public_parameter",
        ),
        (
            "spaced_parameter",
            "pub fn exposed(value: State::hidden    |      State::Private) -> ()\n  ()\nend\n",
            "public_parameter",
        ),
        (
            "multiline_parameter",
            "pub fn exposed(value: State::hidden |\n    State::Private) -> ()\n  ()\nend\n",
            "public_parameter",
        ),
        (
            "compact_result",
            "pub fn exposed() -> State::hidden|State::Private\n  Hidden\nend\n",
            "public_result",
        ),
        (
            "spaced_result",
            "pub fn exposed() -> State::hidden    |      State::Private\n  Hidden\nend\n",
            "public_result",
        ),
    ];

    for (case, signature, boundary) in cases {
        assert_recovered_root_union_case(case, signature, boundary);
    }
}

fn assert_recovered_root_union_case(case: &str, signature: &str, boundary: &str) {
    let source_text = format!(
        "mod main\npub type State\n  Hidden\n  Private\nend\n{signature}pub fn exposed_alias = exposed\n"
    );
    let source = SourceFile::new("main.veln", source_text.clone());
    let parsed = parse(&source);
    assert!(
        parsed
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.variant_refinement_type"),
        "case={case} diagnostics={:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);

    assert_recovered_root_union_diagnostics(case, boundary, &source_text, &module);
    assert_recovered_root_union_publication(case, &module);
}

fn assert_recovered_root_union_diagnostics(
    case: &str,
    boundary: &str,
    source_text: &str,
    module: &SurfaceModule,
) {
    let diagnostics = analyze_surface_module(module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(
        diagnostics.len(),
        2,
        "case={case} diagnostics={diagnostics:#?}"
    );
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    for (diagnostic, (written_type, variant)) in observed
        .iter()
        .zip([("State::hidden", "Hidden"), ("State::Private", "Private")])
    {
        assert!(
            diagnostic.contains(&format!(
                "\"written_type\":\"{written_type}\",\"base_type\":\"main::State\",\"variant\":\"{variant}\",\"boundary\":\"{boundary}\""
            )),
            "case={case} diagnostic={diagnostic}"
        );
    }
    let expected_offsets = ["State::hidden", "State::Private"].map(|alternative| {
        source_text.find(alternative).expect("union alternative") + "State::".len()
    });
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().expect("primary span").start.offset)
            .collect::<Vec<_>>(),
        expected_offsets,
        "case={case}"
    );
}

fn assert_recovered_root_union_publication(case: &str, module: &SurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    let signature = environment.function("exposed").expect("recovery signature");
    assert!(
        signature.params.iter().all(|ty| *ty == Type::Unknown),
        "case={case}"
    );
    assert_eq!(signature.return_type, Type::Unknown, "case={case}");
    assert!(
        !environment.function_is_published_for_test("exposed"),
        "case={case}"
    );
    assert!(
        environment.function("exposed_alias").is_none(),
        "case={case}"
    );
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "exposed_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "case={case} name={name}"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "case={case} name={name}"
        );
    }
    let lowered = lower_checked_surface_module(module);
    assert!(lowered.core.is_none(), "case={case}");
    assert!(lowered.ir.is_none(), "case={case}");
}

#[test]
fn generic_lowercase_final_recovery_keeps_complete_identity_and_quarantines_publication() {
    let long_argument = format!("Argument{}", "Component".repeat(32));
    let written_type = format!("State<Result<{long_argument},  String>>::hidden");
    assert!(written_type.len() > 256);
    let source_text = format!(
        "mod main\npub type {long_argument}\n  pub Value\nend\npub type State<A>\n  Hidden(A)\n  pub Visible(A)\nend\npub fn exposed_parameter(value: {written_type}) -> ()\n  ()\nend\npub fn exposed_result() -> {written_type}\n  Hidden(Ok(Value))\nend\npub fn parameter_alias = exposed_parameter\npub fn parameter_alias_chain = parameter_alias\npub fn result_alias = exposed_result\npub fn result_alias_chain = result_alias\npub fn accepted(value: State<Result<{long_argument},  String>>::Visible) -> State<Result<{long_argument},  String>>::Visible\n  value\nend\nfn private_parameter(value: State<Result<{long_argument},  String>>::Hidden) -> ()\n  ()\nend\nfn private_result() -> State<Result<{long_argument},  String>>::Hidden\n  Hidden(Ok(Value))\nend\n"
    );
    let source = SourceFile::new("main.veln", source_text.clone());
    let parsed = parse(&source);
    assert_eq!(
        parsed
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>(),
        [
            "parse.variant_refinement_type",
            "parse.variant_refinement_type"
        ],
        "{:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
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
        [
            "name.invalid_case",
            "name.invalid_case",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );
    for (diagnostic, boundary) in [relevant[2], relevant[3]]
        .into_iter()
        .zip(["public_parameter", "public_result"])
    {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains(&format!("\"written_type\":\"{written_type}\"")),
            "{json}"
        );
        assert!(json.contains("\"base_type\":\"main::State\""), "{json}");
        assert!(json.contains("\"variant\":\"Hidden\""), "{json}");
        assert!(
            json.contains(&format!("\"boundary\":\"{boundary}\"")),
            "{json}"
        );
        assert!(
            json.contains("\"kind\":\"private_variant_declaration\""),
            "{json}"
        );
        assert!(
            json.contains(&format!("\"kind\":\"{boundary}_boundary\"")),
            "{json}"
        );
        let span = diagnostic.span.as_ref().expect("privacy primary span");
        assert_eq!(&source_text[span.start.offset..span.end.offset], "hidden");
    }

    let environment = TypeEnvironment::from_module(&module);
    for name in ["exposed_parameter", "exposed_result"] {
        assert!(!environment.function_is_published_for_test(name), "{name}");
    }
    for name in [
        "parameter_alias",
        "parameter_alias_chain",
        "result_alias",
        "result_alias_chain",
    ] {
        assert!(environment.function(name).is_none(), "{name}");
    }
    for name in ["accepted", "private_parameter", "private_result"] {
        assert!(environment.function(name).is_some(), "{name}");
    }
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in [
        "exposed_parameter",
        "exposed_result",
        "parameter_alias",
        "parameter_alias_chain",
        "result_alias",
        "result_alias_chain",
    ] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }
}

#[test]
fn public_and_private_generic_refinement_controls_remain_checked() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State<A>\n",
            "  Hidden(A)\n",
            "  pub Visible(A)\n",
            "end\n",
            "pub fn accepted(value: State<Result<Int, String>>::Visible) -> State<Result<Int, String>>::Visible\n",
            "  value\n",
            "end\n",
            "fn private_parameter(value: State<Result<Int, String>>::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
            "fn private_result(value: State<Result<Int, String>>::Hidden) -> State<Result<Int, String>>::Hidden\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    assert!(environment.function_is_published_for_test("accepted"));
    assert!(environment.function("private_parameter").is_some());
    assert!(environment.function("private_result").is_some());
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}
