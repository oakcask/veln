use super::*;

#[test]
fn invalid_named_function_value_recovery_keeps_declaration_quarantined() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty\n",
            "end\n",
            "fn Broken(invalid: Empty::Missing, sibling: String) -> Int\n",
            "  0\n",
            "end\n",
            "fn caller() -> ()\n",
            "  let recovered: fn(Int, Int) -> String = Broken\n",
            "  let result: String = recovered(1, 2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let recovered = environment
        .local_function_value_recovery("Broken", None)
        .expect("invalid-cased function value recovery signature");
    assert_eq!(recovered.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(recovered.return_type, Type::Unknown);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.id.as_str(),
                    "name.invalid_case" | "type.variant_refinement_base"
                )
            })
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["name.invalid_case", "type.variant_refinement_base"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_declaration_annotations_quarantine_body_and_caller_types() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn needs_string(value: String) -> ()\n",
        "  ()\n",
        "end\n",
        "fn invalid_param(value: Empty::Missing) -> ()\n",
        "  needs_string(value)\n",
        "end\n",
        "fn invalid_return() -> Empty::Missing\n",
        "  \"not an int\"\n",
        "end\n",
        "fn caller() -> ()\n",
        "  invalid_param(\"not an int\")\n",
        "  let value: String = invalid_return()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
}

#[test]
fn ordinary_function_quarantine_reaches_every_body_signature_consumer() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn needs_int(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn invalid_param_to_sibling(invalid: Empty::Missing, sibling: String) -> ()\n",
        "  needs_int(sibling)\n",
        "end\n",
        "fn invalid_param_to_result(invalid: Empty::Missing) -> Int\n",
        "  \"not an int\"\n",
        "end\n",
        "fn invalid_result_to_param(sibling: String) -> Empty::Missing\n",
        "  needs_int(sibling)\n",
        "  ()\n",
        "end\n",
        "fn invalid_param_to_named_result(invalid: Empty::Missing) -> output: String\n",
        "ensure output % 2 == 0\n",
        "  \"not an int\"\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_effect_operation_quarantines_all_parameter_and_result_types() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "effect Broken\n",
        "  apply(Bad: Empty::Missing, sibling: String) -> String\n",
        "end\n",
        "fn caller() -> () effects [Broken]\n",
        "  let result: Int = perform Broken::apply(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_handler_quarantines_sibling_arguments_and_body_bindings() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler broken(Bad: Empty::Missing, sibling: String) for Ask\n",
        "  value() => sibling\n",
        "end\n",
        "fn caller() -> Int\n",
        "  handle perform Ask::value() with broken(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(diagnostics[1].id, "type.variant_refinement_base");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_constructor_quarantines_every_payload_type() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type Broken\n",
        "  Built(Bad: Empty::Missing, sibling: String)\n",
        "  Other\n",
        "  Other\n",
        "end\n",
        "fn caller() -> Broken\n",
        "  Built(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert_eq!(diagnostics[1].id, "name.duplicate");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_constructor_quarantine_reaches_alias_descriptor_views() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "pub type Broken\n",
        "  pub Built(invalid: Empty::Missing, sibling: String)\n",
        "  Other\n",
        "end\n",
        "pub type Alias = Broken\n",
        "pub type Transitive = Alias\n",
        "type DuplicateEvidence\n",
        "  Repeated\n",
        "  Repeated\n",
        "end\n",
        "type Control\n",
        "  Built(value: String)\n",
        "end\n",
        "fn direct() -> Broken\n",
        "  Broken::Built(1, 2)\n",
        "end\n",
        "fn aliased() -> Broken\n",
        "  Alias::Built(1, 2)\n",
        "end\n",
        "fn transitive() -> Broken\n",
        "  Transitive::Built(1, 2)\n",
        "end\n",
        "fn control() -> Control\n",
        "  Control::Built(1)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert_eq!(diagnostics[1].id, "name.duplicate");
    assert_eq!(diagnostics[2].id, "type.mismatch");
    assert_eq!(diagnostics[2].span.as_ref().unwrap().start.line, 26);
}

#[test]
fn invalid_schema_refinement_does_not_publish_generated_helpers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty\n",
            "end\n",
            "schema Packet\n",
            "  invalid: Empty::Missing\n",
            "  sibling: Int\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);

    for helper in [
        "byte_decode_packet",
        "byte_encode_packet",
        "validate_packet",
    ] {
        assert!(
            environment
                .function_path(&[helper.to_string()], None)
                .is_none(),
            "invalid schema must not expose partially typed helper `{helper}`"
        );
    }
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_return_refinement_keeps_unknown_result_binding_for_contract_validation() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn invalid_return() -> output: Empty::Missing\n",
        "ensure output.missing == 0\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert_eq!(diagnostics[1].id, "contract.field_missing");
    assert!(
        diagnostics[1]
            .details
            .to_json()
            .contains("\"reason\":\"missing_field\"")
    );
}

#[test]
fn invalid_return_is_quarantined_before_private_return_inference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty\n",
            "end\n",
            "fn invalid_return() -> Empty::Missing\n",
            "  \"not an int\"\n",
            "end\n",
            "fn inferred_caller()\n",
            "  invalid_return()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let inferred = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("inferred_caller"))
        .and_then(|function| environment.function_for(function))
        .expect("inferred caller signature");
    assert_eq!(inferred.return_type, Type::Unknown);
}

#[test]
fn base_environment_function_alias_copies_quarantined_signature() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "impl.veln",
            concat!("mod impl\n", "pub type Empty\n", "end\n",),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);

    let application = lower_surface_ast(
        &parse(&SourceFile::new(
            "api.veln",
            concat!(
                "mod api\n",
                "use impl\n",
                "fn broken(invalid: impl::Empty::Missing, sibling: String) -> Int\n",
                "  0\n",
                "end\n",
                "pub fn exposed = broken\n",
                "fn caller() -> ()\n",
                "  let result: String = exposed(1, 2)\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let environment =
        TypeEnvironment::from_module_with_base_for_test(&application, &dependency_environment);
    let broken = environment.function("broken").unwrap();
    assert_eq!(broken.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(broken.return_type, Type::Unknown);
    let alias = environment.function("exposed").unwrap();
    assert_eq!(alias.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(alias.return_type, Type::Unknown);

    let diagnostics =
        analyze_surface_module_with_base_for_test(&application, &dependency_environment);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}
