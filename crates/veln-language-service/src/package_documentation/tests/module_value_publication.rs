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

fn assert_module_value_publication_failure(
    source: &str,
    declaration_name: &str,
    expected_code: &str,
) {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-publication-provenance\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[("main.veln", source)],
    );

    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert!(
        result
            .declaration_uri_for("main", "module_value", declaration_name)
            .is_none()
    );
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].gate, "module_value", "{diagnostics:#?}");
    assert_eq!(diagnostics[0].code, expected_code);
}

#[test]
fn shared_namespace_collisions_reject_public_module_values() {
    for source in [
        concat!(
            "fn shared() -> Int\n",
            "  1\n",
            "end\n",
            "pub let shared: Int = 1\n",
        ),
        concat!(
            "fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn shared = original\n",
            "pub let shared: Int = 1\n",
        ),
        concat!("pub let shared: Int = 1\n", "let shared: Int = 2\n",),
        concat!("let shared: Int = 1\n", "pub let shared: Int = 2\n",),
    ] {
        assert_module_value_publication_failure(source, "shared", "name.duplicate");
    }
}

#[test]
fn duplicate_public_module_value_occurrences_retain_every_owned_cause() {
    for source in [
        concat!(
            "pub let shared: Int = \"bad\"\n",
            "pub let shared: Int = 2\n",
        ),
        concat!(
            "pub let shared: Int = 2\n",
            "pub let shared: Int = \"bad\"\n",
        ),
    ] {
        let result = generate(
            concat!(
                "[package]\n",
                "name = \"duplicate-module-value-causes\"\n",
                "[lib]\n",
                "exports = [\"main.veln\"]\n",
            ),
            &[("main.veln", source)],
        );

        assert!(result.catalog().is_none(), "{:#?}", result.status());
        assert!(
            result
                .declaration_uri_for("main", "module_value", "shared")
                .is_none()
        );
        let diagnostics = &result.status().diagnostics;
        assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.gate == "module_value")
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<HashSet<_>>(),
            HashSet::from(["name.duplicate", "type.mismatch"]),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn shared_namespace_collision_causes_reach_exported_dependents() {
    for collision in [
        concat!(
            "fn shared() -> Int\n",
            "  1\n",
            "end\n",
            "let shared: Int = 1\n"
        ),
        concat!(
            "fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn shared = original\n",
            "let shared: Int = 1\n",
        ),
        concat!("let shared: Int = 1\n", "let shared: Int = 2\n"),
    ] {
        let source = format!("{collision}pub let exposed: Int = shared\nlet unrelated: Bool = 0\n");
        assert_module_value_publication_failure(&source, "exposed", "name.duplicate");
    }
}

#[test]
fn invalid_cased_public_module_value_uses_module_value_gate() {
    assert_module_value_publication_failure(
        "pub let BadValue: Int = 1\nlet unrelated: Bool = 0\n",
        "BadValue",
        "name.invalid_case",
    );
}

#[test]
fn mixed_source_and_visibility_diagnostics_keep_their_causal_gates() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"mixed-publication-gates\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[
            ("bad.veln", "pub let BadValue: Int = 1\n"),
            (
                "main.veln",
                concat!(
                    "pub type State\n",
                    "  Hidden\n",
                    "end\n",
                    "pub fn expose(value: State::Hidden) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [
            ("module_value", "name.invalid_case"),
            ("semantic_visibility", "type.variant_refinement_private"),
        ]
    );
}

#[test]
fn initializer_changes_preserve_declaration_identity_but_rotate_snapshot_identity() {
    let manifest = concat!(
        "[package]\n",
        "name = \"module-value-initializer-identity\"\n",
        "[lib]\n",
        "exports = [\"main.veln\"]\n",
    );
    let first = generate(
        manifest,
        &[("main.veln", "pub let value: String = \"first\"\n")],
    );
    let second = generate(
        manifest,
        &[("main.veln", "pub let value: String = \"second\"\n")],
    );
    let first_declaration = &catalog_or_panic(&first).modules[0].declarations[0];
    let second_declaration = &catalog_or_panic(&second).modules[0].declarations[0];

    assert_eq!(first_declaration.signature, "let value: String");
    assert_eq!(first_declaration.signature, second_declaration.signature);
    assert_eq!(first_declaration.id, second_declaration.id);
    assert_ne!(first.snapshot_digest(), second.snapshot_digest());
    assert_ne!(first.doc_digest(), second.doc_digest());
    assert_ne!(first.canonical_bytes(), second.canonical_bytes());
    assert!(
        !std::str::from_utf8(first.canonical_bytes())
            .unwrap()
            .contains("first")
    );
    assert!(
        !std::str::from_utf8(second.canonical_bytes())
            .unwrap()
            .contains("second")
    );
}
