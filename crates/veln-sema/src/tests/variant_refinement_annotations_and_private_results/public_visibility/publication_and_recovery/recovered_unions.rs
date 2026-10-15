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
