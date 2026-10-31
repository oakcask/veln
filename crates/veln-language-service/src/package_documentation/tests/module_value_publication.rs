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
    assert_eq!(diagnostics[0].gate, "module_value", "{diagnostics:#?}");
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

fn captured_shared_value_dependency() -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: "b".to_string(),
        source: "vendor/b".to_string(),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new("shared.veln", "pub let answer: Int = 1\n")],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                concat!(
                    "[package]\n",
                    "name = \"b\"\n",
                    "[lib]\n",
                    "exports = [\"shared.veln\"]\n",
                ),
            )),
        }),
    }
}

const DEPENDENCY_BACKED_VALUE_MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"a\"\n",
    "[lib]\n",
    "exports = [\"main.veln\"]\n",
    "[dependencies.\"b\"]\n",
    "path = \"vendor/b\"\n",
);

#[test]
fn dependency_backed_public_module_value_publishes_with_declaration_lookup() {
    let dependency = captured_shared_value_dependency();
    let result = generate_with_dependencies(
        DEPENDENCY_BACKED_VALUE_MANIFEST,
        &[(
            "main.veln",
            "use shared from \"b\"\npub let exposed: Int = shared::answer\n",
        )],
        &[dependency],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].name, "exposed");
    assert_eq!(
        catalog.modules[0].declarations[0].signature,
        "let exposed: Int"
    );
    assert!(
        result
            .declaration_uri_for("main", "module_value", "exposed")
            .is_some()
    );
}

#[test]
fn invalid_dependency_backed_public_module_value_fails_atomically() {
    let dependency = captured_shared_value_dependency();
    let result = generate_with_dependencies(
        DEPENDENCY_BACKED_VALUE_MANIFEST,
        &[(
            "main.veln",
            concat!(
                "use shared from \"b\"\n",
                "pub let exposed: Int = shared::answer\n",
                "pub let broken: Bool = shared::answer\n",
            ),
        )],
        &[dependency],
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .declaration_uri_for("main", "module_value", "exposed")
            .is_none()
    );
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].gate, "module_value");
    assert_eq!(diagnostics[0].code, "type.mismatch");
    assert!(
        diagnostics[0]
            .span
            .as_ref()
            .is_some_and(|span| span.source_uri.ends_with("/main.veln"))
    );
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

#[derive(Clone, Copy)]
enum ClassificationWorkload {
    ExportedChain,
    IndependentInvalid,
    InvalidCasing,
}

fn classification_work(
    count: usize,
    workload: ClassificationWorkload,
) -> PackageDocSemanticClassificationWork {
    let source = match workload {
        ClassificationWorkload::ExportedChain => (0..count)
            .map(|index| {
                if index == 0 {
                    "pub let value_0: Int = \"wrong\"\n".to_string()
                } else {
                    format!("pub let value_{index}: Int = value_{}\n", index - 1)
                }
            })
            .collect::<String>(),
        ClassificationWorkload::IndependentInvalid => (0..count)
            .map(|index| format!("pub let value_{index}: Int = \"wrong-{index}\"\n"))
            .collect::<String>(),
        ClassificationWorkload::InvalidCasing => (0..count)
            .map(|index| format!("pub let BadValue{index}: Int = {index}\n"))
            .collect::<String>(),
    };
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
    let first_name = match workload {
        ClassificationWorkload::InvalidCasing => "BadValue0",
        ClassificationWorkload::ExportedChain | ClassificationWorkload::IndependentInvalid => {
            "value_0"
        }
    };
    assert!(
        result
            .declaration_uri_for("main", "module_value", first_name)
            .is_none()
    );
    let diagnostics = &result.status().diagnostics;
    let expected_code = match workload {
        ClassificationWorkload::ExportedChain | ClassificationWorkload::IndependentInvalid => {
            "type.mismatch"
        }
        ClassificationWorkload::InvalidCasing => "name.invalid_case",
    };
    let expected_count = match workload {
        ClassificationWorkload::ExportedChain => 1,
        ClassificationWorkload::IndependentInvalid | ClassificationWorkload::InvalidCasing => count,
    };
    assert_eq!(diagnostics.len(), expected_count, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.gate == "module_value" && diagnostic.code == expected_code
    }));
    let causal_offsets = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.span.as_ref().expect("causal source span").offset)
        .collect::<HashSet<_>>();
    assert_eq!(causal_offsets.len(), expected_count, "{diagnostics:#?}");
    take_package_doc_semantic_classification_work()
}

fn assert_linear_adjacent_work(
    original: PackageDocSemanticClassificationWork,
    adjacent: PackageDocSemanticClassificationWork,
) {
    assert!(original.exported_module_value_lookups > 0);
    assert!(original.diagnostic_lookups > 0);

    assert_eq!(original.exported_module_value_lookups, 32);
    assert_eq!(adjacent.exported_module_value_lookups, 64);
    assert!(adjacent.retained_values <= original.retained_values * 2);
    assert!(adjacent.retained_edges <= original.retained_edges * 2 + 1);
    assert!(adjacent.retained_direct_causes <= original.retained_direct_causes * 2);
    assert!(adjacent.visited_initializations <= original.visited_initializations * 2);
    assert!(adjacent.traversal_values <= original.traversal_values * 2);
    assert!(adjacent.traversal_edges <= original.traversal_edges * 2 + 1);
    assert!(adjacent.cause_processing <= original.cause_processing * 2);
    assert!(adjacent.cause_index_insertions <= original.cause_index_insertions * 2);
    assert!(adjacent.diagnostic_lookups <= original.diagnostic_lookups * 2);
}

#[test]
fn exported_invalid_chain_classification_is_linear_for_all_public_roots() {
    let original = classification_work(32, ClassificationWorkload::ExportedChain);
    let adjacent = classification_work(64, ClassificationWorkload::ExportedChain);
    eprintln!("exported chain classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert!(original.retained_values > 0);
    assert!(original.visited_initializations > 0);
    assert!(original.traversal_values > 0);
    assert!(original.cause_processing > 0);
    assert!(original.cause_index_insertions > 0);
    assert_eq!(original.retained_edges, 31);
    assert_eq!(adjacent.retained_edges, 63);
    assert_eq!(original.traversal_edges, 31);
    assert_eq!(adjacent.traversal_edges, 63);
    assert_eq!(original.casing_range_comparisons, 0);
    assert_eq!(adjacent.casing_range_comparisons, 0);
}

#[test]
fn independent_invalid_export_classification_is_linear() {
    let original = classification_work(32, ClassificationWorkload::IndependentInvalid);
    let adjacent = classification_work(64, ClassificationWorkload::IndependentInvalid);
    eprintln!("independent invalid classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert!(original.retained_values > 0);
    assert!(original.visited_initializations > 0);
    assert!(original.traversal_values > 0);
    assert!(original.cause_processing > 0);
    assert!(original.cause_index_insertions > 0);
    assert_eq!(original.retained_edges, 0);
    assert_eq!(adjacent.retained_edges, 0);
    assert_eq!(original.traversal_edges, 0);
    assert_eq!(adjacent.traversal_edges, 0);
    assert_eq!(original.casing_range_comparisons, 0);
    assert_eq!(adjacent.casing_range_comparisons, 0);
}

#[test]
fn invalid_casing_export_classification_uses_indexed_provenance_linearly() {
    let original = classification_work(32, ClassificationWorkload::InvalidCasing);
    let adjacent = classification_work(64, ClassificationWorkload::InvalidCasing);
    eprintln!("invalid casing classification work: 32={original:?}, 64={adjacent:?}");

    assert_linear_adjacent_work(original, adjacent);
    assert_eq!(original.casing_range_comparisons, 32);
    assert_eq!(adjacent.casing_range_comparisons, 64);
}
