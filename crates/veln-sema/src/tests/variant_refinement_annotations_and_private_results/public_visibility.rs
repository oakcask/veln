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

#[test]
fn inherited_private_declaration_provenance_is_retained() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dependency.veln",
            concat!(
                "mod shared\n",
                "type Secret\n",
                "  pub Hidden\n",
                "end\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let base = TypeEnvironment::from_module(&dependency);
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod shared\n",
                "pub fn private_base(value: Secret::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
                "pub fn private_variant(value: State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let diagnostics = analyze_surface_module_with_base_for_test(&consumer, &base)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(
        observed[0].contains(concat!(
            "\"kind\":\"private_type_declaration\"",
            ",\"message\":\"Private type `shared::Secret` is declared here.\"",
            ",\"span\":{\"file\":\"dependency.veln\",\"start\":{\"line\":2,\"column\":1"
        )),
        "{}",
        observed[0]
    );
    assert!(
        observed[0].contains(concat!(
            "\"kind\":\"public_parameter_boundary\"",
            ",\"message\":\"Public parameter `value` of function `private_base` exposes this refinement.\"",
            ",\"span\":{\"file\":\"consumer.veln\",\"start\":{\"line\":2,\"column\":21"
        )),
        "{}",
        observed[0]
    );
    assert!(
        observed[1].contains(concat!(
            "\"kind\":\"private_variant_declaration\"",
            ",\"message\":\"Private variant `shared::State::Hidden` is declared here.\"",
            ",\"span\":{\"file\":\"dependency.veln\",\"start\":{\"line\":6,\"column\":3"
        )),
        "{}",
        observed[1]
    );
    assert!(
        observed[1].contains(concat!(
            "\"kind\":\"public_parameter_boundary\"",
            ",\"message\":\"Public parameter `value` of function `private_variant` exposes this refinement.\"",
            ",\"span\":{\"file\":\"consumer.veln\",\"start\":{\"line\":5,\"column\":24"
        )),
        "{}",
        observed[1]
    );
}

#[test]
fn nested_and_alias_refinements_remain_out_of_scope() {
    let diagnostics = diagnostics_for(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub type Box<A>\n",
        "  pub Boxed(A)\n",
        "end\n",
        "pub type Alias = State\n",
        "pub fn alias(value: Alias::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn nested(value: {state: State::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn named_nested(value: Box<State::Hidden>) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn visibility_failure_quarantines_the_signature_and_blocks_lowering() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub fn exposed(value: State::Hidden, sibling: Int) -> State::Hidden\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("exposed").expect("function signature");
    assert_eq!(signature.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));

    let lowered = lower_checked_surface_module(&module);
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
fn visibility_failure_is_not_published_to_a_consumer_environment() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dep.veln",
            concat!(
                "mod dep\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub fn exposed(value: State::Hidden) -> State::Hidden\n",
                "  value\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);
    assert!(!dependency_environment.function_is_published_for_test("exposed"));

    let consumer =
        lower_surface_ast(&parse(&SourceFile::new("main.veln", "mod main\nuse dep\n")).tree);
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &dependency_environment);
    assert!(consumer_environment.function("exposed").is_none());
    assert!(
        consumer_environment
            .function_path(&["dep".to_string(), "exposed".to_string()], Some("main"))
            .is_none()
    );
    assert!(
        consumer_environment
            .unqualified_function_import_candidates("exposed", Some("main"))
            .is_empty()
    );
}

#[test]
fn visibility_failure_is_not_republished_through_function_aliases() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dep.veln",
            concat!(
                "mod dep\n",
                "pub type State\n",
                "  Hidden\n",
                "  pub Ready\n",
                "end\n",
                "pub fn exposed(value: State::Hidden) -> State::Hidden\n",
                "  value\n",
                "end\n",
                "pub fn first_alias = exposed\n",
                "pub fn second_alias = exposed\n",
                "pub fn accepted(value: State::Ready) -> State::Ready\n",
                "  value\n",
                "end\n",
                "pub fn accepted_alias = accepted\n",
            ),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);
    let exposed = dependency_environment
        .function("exposed")
        .expect("same-module recovery signature");
    assert_eq!(exposed.params, [Type::Unknown]);
    assert_eq!(exposed.return_type, Type::Unknown);
    assert!(!dependency_environment.function_is_published_for_test("exposed"));
    assert!(dependency_environment.function("first_alias").is_none());
    assert!(dependency_environment.function("second_alias").is_none());
    assert!(dependency_environment.function_is_published_for_test("accepted_alias"));

    let invalid_consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "invalid_consumer.veln",
            concat!(
                "mod invalid_consumer\n",
                "use dep\n",
                "fn consume(value: Int) -> Int\n",
                "  dep::first_alias(value)\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let (invalid_diagnostics, invalid_lowered) =
        check_project_surface_module_with_base_for_test(&invalid_consumer, &dependency_environment);
    assert!(!invalid_diagnostics.is_empty());
    assert!(
        invalid_lowered.core.is_none(),
        "an unpublished alias must not recover checked core"
    );
    assert!(
        invalid_lowered.ir.is_none(),
        "an unpublished alias must not recover IR"
    );

    let valid_consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "valid_consumer.veln",
            concat!(
                "mod valid_consumer\n",
                "use dep\n",
                "fn consume(value: State::Ready) -> State::Ready\n",
                "  dep::accepted_alias(value)\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let (valid_diagnostics, valid_lowered) =
        check_project_surface_module_with_base_for_test(&valid_consumer, &dependency_environment);
    assert!(valid_diagnostics.is_empty(), "{valid_diagnostics:#?}");
    assert!(valid_lowered.core.is_some());
    assert!(valid_lowered.ir.is_some());

    let bridge = lower_surface_ast(
        &parse(&SourceFile::new(
            "bridge.veln",
            concat!(
                "mod bridge\n",
                "use dep\n",
                "pub fn bridged_invalid = dep::first_alias\n",
                "pub fn bridged_valid = dep::accepted_alias\n",
            ),
        ))
        .tree,
    );
    let bridge_environment =
        TypeEnvironment::from_module_with_base_for_test(&bridge, &dependency_environment);
    assert!(bridge_environment.function("bridged_invalid").is_none());
    assert!(bridge_environment.function_is_published_for_test("bridged_valid"));

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "main.veln",
            "mod main\nuse dep\nuse bridge\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &bridge_environment);
    for (module, name) in [
        ("dep", "exposed"),
        ("dep", "first_alias"),
        ("dep", "second_alias"),
        ("bridge", "bridged_invalid"),
    ] {
        assert!(
            consumer_environment
                .function_path(&[module.to_string(), name.to_string()], Some("main"))
                .is_none(),
            "{module}::{name} must remain unpublished"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("main"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }
    assert!(
        consumer_environment
            .function_path(
                &["bridge".to_string(), "bridged_valid".to_string()],
                Some("main"),
            )
            .is_some(),
        "an alias chain to a valid public function must remain published"
    );
}

#[test]
fn unpublished_local_function_keeps_alias_target_shadowing() {
    let imported = lower_surface_ast(
        &parse(&SourceFile::new(
            "imported.veln",
            concat!(
                "mod imported\n",
                "pub fn exposed(value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let imported_environment = TypeEnvironment::from_module(&imported);

    let application = lower_surface_ast(
        &parse(&SourceFile::new(
            "application.veln",
            concat!(
                "mod application\n",
                "use imported\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub fn exposed(value: State::Hidden) -> State::Hidden\n",
                "  value\n",
                "end\n",
                "pub fn alias = exposed\n",
            ),
        ))
        .tree,
    );
    let application_environment =
        TypeEnvironment::from_module_with_base_for_test(&application, &imported_environment);

    let exposed = application_environment
        .function("exposed")
        .expect("local recovery signature must continue to shadow the imported function");
    assert_eq!(exposed.module_name.as_deref(), Some("application"));
    assert_eq!(exposed.params, [Type::Unknown]);
    assert!(
        application_environment
            .function_path(
                &["imported".to_string(), "exposed".to_string()],
                Some("application"),
            )
            .is_some(),
        "the imported namesake must remain available when explicitly qualified"
    );
    assert!(
        application_environment.function("alias").is_none(),
        "an unpublished local target must not be skipped in favor of an imported namesake"
    );
}
