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

#[test]
fn rejected_public_value_does_not_publish_unrelated_private_diagnostics() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-private-diagnostic-boundary\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[
            ("main.veln", "pub let exposed: Int = \"wrong\"\n"),
            ("internal.veln", "let hidden: Bool = 0\n"),
        ],
    );

    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].gate, "module_value");
    assert_eq!(diagnostics[0].code, "type.mismatch");
    assert_eq!(
        diagnostics[0].span.as_ref().unwrap().source_uri,
        source_uri(
            "module-value-private-diagnostic-boundary",
            result.snapshot_digest(),
            "main.veln",
        )
    );
}

#[test]
fn dependency_failure_keeps_only_the_exported_values_causal_diagnostics() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-causal-diagnostic-boundary\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[
            (
                "main.veln",
                "use causal\npub let exposed: Int = causal::broken\n",
            ),
            ("causal.veln", "pub let broken: Int = \"wrong\"\n"),
            ("unrelated.veln", "let hidden: Bool = 0\n"),
        ],
    );

    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert!(!diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.gate == "module_value"
            && diagnostic
                .span
                .as_ref()
                .is_some_and(|span| !span.source_uri.ends_with("/unrelated.veln"))
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "type.mismatch"
            && diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.source_uri.ends_with("/causal.veln"))
    }));
}

#[test]
fn public_module_value_refinement_diagnostic_uses_module_value_gate() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-refinement-gate\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[(
            "main.veln",
            concat!(
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub let refined: State::Hidden = Hidden\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert_eq!(result.status().diagnostics.len(), 1);
    assert_eq!(result.status().diagnostics[0].gate, "module_value");
    assert_eq!(
        result.status().diagnostics[0].code,
        "type.variant_refinement_private"
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

fn classification_work_for_invalid_values(count: usize) -> PackageDocSemanticClassificationWork {
    let source = (0..count)
        .map(|index| format!("pub let value_{index}: Int = \"wrong\"\n"))
        .collect::<String>();
    reset_package_doc_semantic_classification_work();
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-classification-scaling\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[("main.veln", source.as_str())],
    );
    assert!(result.catalog().is_none());
    assert_eq!(result.status().diagnostics.len(), count);
    take_package_doc_semantic_classification_work()
}

#[test]
fn module_value_diagnostic_classification_scales_with_declarations_and_diagnostics() {
    let original = classification_work_for_invalid_values(32);
    let adjacent = classification_work_for_invalid_values(64);
    eprintln!("module-value classification work: 32={original:?}, 64={adjacent:?}");

    assert_eq!(original.exported_module_value_lookups, 32);
    assert_eq!(adjacent.exported_module_value_lookups, 64);
    assert!(original.cause_index_insertions > 0);
    assert!(original.diagnostic_lookups > 0);
    assert!(adjacent.cause_index_insertions <= original.cause_index_insertions * 2);
    assert!(adjacent.diagnostic_lookups <= original.diagnostic_lookups * 2);
}
