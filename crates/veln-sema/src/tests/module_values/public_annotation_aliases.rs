use super::*;
use veln_diagnostics::JsonValue;

#[test]
fn public_module_value_annotations_preserve_private_written_alias_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub type Public\n",
            "  pub Value(value: State::Hidden)\n",
            "end\n",
            "pub type Secret = Public\n",
            "pub type Box<T>\n",
            "  pub Box(value: T)\n",
            "end\n",
            "pub let direct: Secret = 0\n",
            "pub let generic: Box<Secret> = 0\n",
            "pub let function: fn(Secret) -> Secret = 0\n",
            "pub let record: {field: Secret} = 0\n",
            "let private_boundary: Secret = 0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = merged_modules_with_identities(vec![("main", source)]);
    let environment = crate::types::TypeEnvironment::from_module(&module);
    let written = environment
        .annotation_type_descriptor_in_owner("Secret", 0, Some("main"), None)
        .expect("written alias resolves");
    assert_eq!(written.visibility, veln_ast::Visibility::Private);
    assert_eq!(written.declaration_identity(), "main::Secret");
    assert_eq!(
        environment
            .adts
            .terminal_descriptor_for_alias(written)
            .expect("alias retains terminal descriptor")
            .visibility,
        veln_ast::Visibility::Public,
    );

    let diagnostics = analyze_surface_module(&module);
    let exposure = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
        .collect::<Vec<_>>();
    assert_eq!(exposure.len(), 4, "{diagnostics:#?}");
    for (diagnostic, value) in exposure.iter().zip(&module.module_values) {
        let name = value.name.as_deref().expect("named value");
        assert_eq!(
            diagnostic.details,
            JsonValue::object([
                ("boundary", JsonValue::string("public_module_value")),
                ("name", JsonValue::string(name)),
                ("written_type", JsonValue::string("Secret")),
                ("resolved_type", JsonValue::string("main::Secret")),
            ]),
        );
        assert_eq!(
            diagnostic.related,
            vec![
                JsonValue::object([
                    ("kind", JsonValue::string("private_type_declaration")),
                    (
                        "message",
                        JsonValue::string("Private type `main::Secret` is declared here."),
                    ),
                    (
                        "span",
                        crate::diagnostics::span_json(&module.aliases[0].span)
                    ),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("public_module_value_boundary")),
                    (
                        "message",
                        JsonValue::string(format!(
                            "Public module value `{name}` exposes this type."
                        )),
                    ),
                    (
                        "span",
                        crate::diagnostics::span_json(&value.annotation_span)
                    ),
                ]),
            ],
        );
    }
}

#[test]
fn public_module_value_annotations_still_check_private_alias_targets() {
    let module = merged_modules_with_identities(vec![(
        "main",
        SourceFile::new(
            "main.veln",
            concat!(
                "type Private\n",
                "  pub Value\n",
                "end\n",
                "pub type Alias = Private\n",
                "pub type Public\n",
                "  pub Ready\n",
                "end\n",
                "pub type Allowed = Public\n",
                "pub let rejected: Alias = Value\n",
                "pub let accepted: Allowed = Ready\n",
            ),
        ),
    )]);
    let diagnostics = analyze_surface_module(&module);
    let exposure = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
        .collect::<Vec<_>>();
    assert_eq!(exposure.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        exposure[0].details,
        JsonValue::object([
            ("boundary", JsonValue::string("public_module_value")),
            ("name", JsonValue::string("rejected")),
            ("written_type", JsonValue::string("Alias")),
            ("resolved_type", JsonValue::string("main::Private")),
        ]),
    );
    assert_eq!(
        exposure[0].related[0],
        JsonValue::object([
            ("kind", JsonValue::string("private_type_declaration")),
            (
                "message",
                JsonValue::string("Private type `main::Private` is declared here."),
            ),
            ("span", crate::diagnostics::span_json(&module.types[0].span)),
        ]),
    );
}
