use super::*;

#[test]
fn public_module_values_publish_names_and_annotations_without_initializers() {
    let result = generate_fixture("module-value-publication");
    let catalog = catalog_or_panic(&result);
    let declaration = catalog.modules[0]
        .declarations
        .iter()
        .find(|declaration| declaration.name == "service_name")
        .expect("public module value declaration");

    assert_eq!(declaration.kind, "module_value");
    assert_eq!(declaration.signature, "let service_name: String");
    assert_eq!(declaration.doc, ["Published service name."]);
    assert!(
        result
            .declaration_uri_for("main", "module_value", "service_name")
            .is_some()
    );

    let canonical = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(!canonical.contains("initializer-secret"));
    assert!(!canonical.contains("private_name"));
    assert!(!canonical.contains("private-secret"));
}

#[test]
fn rejected_public_module_value_fails_the_complete_catalog() {
    let result = generate_fixture("module-value-publication-rejected");

    assert!(result.catalog().is_none());
    assert!(
        result
            .declaration_uri_for("main", "module_value", "accepted")
            .is_none()
    );
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "module_value" && diagnostic.code == "type.mismatch"
    }));
    let canonical = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(!canonical.contains("modules"));
}

#[test]
fn dependency_rejected_public_module_value_fails_the_complete_catalog() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-publication-dependency-rejected\"\n",
            "[lib]\n",
            "exports = [\"alpha.veln\"]\n",
        ),
        &[
            (
                "alpha.veln",
                "use beta\npub let left: Int = beta::right\npub let healthy: Bool = true\n",
            ),
            ("beta.veln", "use alpha\npub let right: Int = alpha::left\n"),
        ],
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .declaration_uri_for("alpha", "module_value", "healthy")
            .is_none()
    );
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "module_value" && diagnostic.code == "value.module_initializer_cycle"
    }));
}
