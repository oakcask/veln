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
fn recurrent_imported_lookup_rejection_fails_the_complete_catalog_in_every_import_order() {
    for reverse_sources in [false, true] {
        let (b_path, owner_path) = if reverse_sources {
            ("order_2/b.veln", "order_0/owner.veln")
        } else {
            ("order_0/b.veln", "order_2/owner.veln")
        };
        let c_path = "order_1/c.veln";
        let b_module = b_path.trim_end_matches(".veln").replace('/', "::");
        let c_module = c_path.trim_end_matches(".veln").replace('/', "::");
        let owner_module = owner_path.trim_end_matches(".veln").replace('/', "::");
        for reverse_imports in [false, true] {
            let imports = if reverse_imports {
                format!("use {c_module} as c\nuse {b_module} as b\n")
            } else {
                format!("use {b_module} as b\nuse {c_module} as c\n")
            };
            let owner = format!(
                "{imports}pub let selected: Int = shared\npub let local: Int = selected\npub let independent: Int = 4\n"
            );
            let b = format!("use {owner_module} as owner\npub let shared: Int = owner::selected\n");
            let manifest = format!(
                "[package]\nname = \"module-value-recurrent-lookup\"\n[lib]\nexports = [\"{owner_path}\"]\n"
            );
            let sources = [
                (b_path, b.as_str()),
                (c_path, "pub let shared: Int = 1\n"),
                (owner_path, owner.as_str()),
            ];
            let result = generate(&manifest, &sources);

            assert!(result.catalog().is_none(), "{:#?}", result.status());
            assert!(
                result
                    .declaration_uri_for(&owner_module, "module_value", "selected")
                    .is_none()
            );
            assert!(
                result
                    .declaration_uri_for(&owner_module, "module_value", "independent")
                    .is_none()
            );
            assert!(
                result.status().diagnostics.iter().any(|diagnostic| {
                    diagnostic.gate == "module_value"
                        && matches!(
                            diagnostic.code.as_str(),
                            "name.ambiguous" | "name.unresolved"
                        )
                }),
                "{:#?}",
                result.status().diagnostics
            );
        }
    }
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

#[test]
fn quarantined_import_used_by_public_value_rejects_with_the_import_cause() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-import-recovery\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[
            (
                "main.veln",
                concat!(
                    "use helper as HTTP\n",
                    "pub let answer: Int = HTTP::number\n",
                    "pub let accepted: Int = 1\n",
                ),
            ),
            ("helper.veln", "pub fn number() -> Int\n  1\nend\n"),
        ],
    );

    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert!(
        result
            .declaration_uri_for("main", "module_value", "answer")
            .is_none()
    );
    assert!(
        result
            .declaration_uri_for("main", "module_value", "accepted")
            .is_none()
    );
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].gate, "module_value");
    assert_eq!(diagnostics[0].code, "name.invalid_case");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().line, 1);
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn unrelated_quarantined_import_does_not_reject_a_public_value() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-unrelated-import-recovery\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[
            (
                "main.veln",
                "use helper as HTTP\npub let accepted: Int = 1\n",
            ),
            ("helper.veln", "pub fn number() -> Int\n  1\nend\n"),
        ],
    );

    assert!(result.catalog().is_some(), "{:#?}", result.status());
    assert!(
        result
            .declaration_uri_for("main", "module_value", "accepted")
            .is_some()
    );
}

fn captured_invalid_same_path_dependency(source: &str) -> CapturedDependencyProject {
    captured_same_path_dependency("b", "vendor/b", source)
}

fn captured_same_path_dependency(
    package: &str,
    source_root: &str,
    source: &str,
) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: package.to_string(),
        source: source_root.to_string(),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new("main.veln", source)],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &format!("[package]\nname = \"{package}\"\n[lib]\nexports = [\"main.veln\"]\n"),
            )),
        }),
    }
}

fn captured_dependency_source_uri(
    dependency: &CapturedDependencyProject,
    source_path: &str,
) -> String {
    let project = dependency.project.as_ref().unwrap();
    let manifest = project.manifest.as_ref().unwrap();
    let snapshot = capture_embedded_package_snapshot(
        &manifest.source_bytes,
        project.files.iter().map(|source| {
            PackageSnapshotSource::new(source.path().as_str(), source.text().as_bytes())
        }),
    )
    .unwrap();
    source_uri(&dependency.package, snapshot.digest(), source_path)
}

#[test]
fn unrelated_dependency_value_with_colliding_source_span_does_not_reject_root() {
    for dependency_source in [
        "pub let fail: Int = \"x\"\n",
        "\npub let fail: Int = \"x\"\n",
    ] {
        let dependency = captured_invalid_same_path_dependency(dependency_source);
        let result = generate_with_dependencies(
            concat!(
                "[package]\n",
                "name = \"a\"\n",
                "[lib]\n",
                "exports = [\"main.veln\"]\n",
                "[dependencies.\"b\"]\n",
                "path = \"vendor/b\"\n",
            ),
            &[("main.veln", "pub let safe: Int = 123\n")],
            &[dependency],
        );

        assert!(result.catalog().is_some(), "{:#?}", result.status());
        assert!(
            result
                .declaration_uri_for("main", "module_value", "safe")
                .is_some()
        );
    }
}

#[test]
fn required_same_path_dependency_failure_keeps_dependency_location() {
    let dependency = captured_invalid_same_path_dependency("pub let fail: Int = \"x\"\n");
    let expected_source_uri = captured_dependency_source_uri(&dependency, "main.veln");
    let result = generate_with_dependencies(
        concat!(
            "[package]\n",
            "name = \"a\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
            "[dependencies.\"b\"]\n",
            "path = \"vendor/b\"\n",
        ),
        &[(
            "main.veln",
            concat!(
                "use main as dep from \"b\"\n",
                "pub let exposed: Int = dep::fail\n",
                "pub let accepted: Int = 1\n",
            ),
        )],
        &[dependency],
    );

    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert!(
        result
            .declaration_uri_for("main", "module_value", "accepted")
            .is_none()
    );
    assert!(
        result.status().diagnostics.iter().any(|diagnostic| {
            diagnostic.gate == "module_value"
                && diagnostic.code == "type.mismatch"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.source_uri == expected_source_uri)
        }),
        "{:#?}",
        result.status().diagnostics
    );
}

#[test]
fn required_dependency_callable_collision_keeps_dependency_identity() {
    let dependency = captured_invalid_same_path_dependency(concat!(
        "fn shared() -> Int\n",
        "  1\n",
        "end\n",
        "pub let shared: Int = 1\n",
    ));
    let expected_source_uri = captured_dependency_source_uri(&dependency, "main.veln");
    let result = generate_with_dependencies(
        concat!(
            "[package]\n",
            "name = \"a\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
            "[dependencies.\"b\"]\n",
            "path = \"vendor/b\"\n",
        ),
        &[(
            "main.veln",
            concat!(
                "use main as dep from \"b\"\n",
                "pub let exposed: Int = dep::shared\n",
            ),
        )],
        &[dependency],
    );

    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert!(
        result.status().diagnostics.iter().any(|diagnostic| {
            diagnostic.gate == "module_value"
                && diagnostic.code == "name.duplicate"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.source_uri == expected_source_uri)
        }),
        "{:#?}",
        result.status().diagnostics
    );
}

#[test]
fn identical_dependency_spans_do_not_share_callable_collision_causes() {
    let collision_source = concat!(
        "fn shared() -> Int\n",
        "  1\n",
        "end\n",
        "pub let shared: Int = 1\n",
    );
    let required = captured_same_path_dependency("b", "vendor/b", collision_source);
    let unrelated = captured_same_path_dependency("c", "vendor/c", collision_source);
    let required_uri = captured_dependency_source_uri(&required, "main.veln");
    let unrelated_uri = captured_dependency_source_uri(&unrelated, "main.veln");
    let result = generate_with_dependencies(
        concat!(
            "[package]\n",
            "name = \"a\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
            "[dependencies.\"b\"]\n",
            "path = \"vendor/b\"\n",
            "[dependencies.\"c\"]\n",
            "path = \"vendor/c\"\n",
        ),
        &[(
            "main.veln",
            concat!(
                "use main as required from \"b\"\n",
                "use main as unrelated from \"c\"\n",
                "pub let exposed: Int = required::shared\n",
            ),
        )],
        &[required, unrelated],
    );

    assert!(result.catalog().is_none(), "{:#?}", result.status());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [
            ("module_value", "name.unresolved"),
            ("module_value", "name.duplicate"),
        ]
    );
    assert_eq!(
        diagnostics[1].span.as_ref().unwrap().source_uri,
        required_uri
    );
    assert_eq!(
        diagnostics[0].span.as_ref().unwrap().source_uri,
        source_uri("a", result.snapshot_digest(), "main.veln")
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .is_none_or(|span| span.source_uri != unrelated_uri)
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

#[test]
fn reordered_private_refinement_causes_keep_their_declaration_gates() {
    let source = concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub fn expose(value: State::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub let refined: State::Hidden = Hidden\n",
    );
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"module-value-refinement-order\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[("main.veln", source)],
    );

    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let expected_uri = source_uri(
        "module-value-refinement-order",
        result.snapshot_digest(),
        "main.veln",
    );
    let function_offset = source.find("State::Hidden").unwrap() + "State::".len();
    let value_offset = source.rfind("State::Hidden").unwrap() + "State::".len();
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| (
                diagnostic.gate.as_str(),
                diagnostic.code.as_str(),
                diagnostic.span.as_ref().unwrap().source_uri.as_str(),
                diagnostic.span.as_ref().unwrap().line,
                diagnostic.span.as_ref().unwrap().column,
                diagnostic.span.as_ref().unwrap().offset,
            ))
            .collect::<Vec<_>>(),
        [
            (
                "semantic_visibility",
                "type.variant_refinement_private",
                expected_uri.as_str(),
                4,
                29,
                function_offset,
            ),
            (
                "module_value",
                "type.variant_refinement_private",
                expected_uri.as_str(),
                7,
                25,
                value_offset,
            ),
        ]
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
    DuplicateCallable,
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
        ClassificationWorkload::DuplicateCallable => {
            let functions = (0..count)
                .map(|index| format!("fn value_{index}() -> Int\n  {index}\nend\n"))
                .collect::<String>();
            let values = (0..count)
                .map(|index| format!("pub let value_{index}: Int = {index}\n"))
                .collect::<String>();
            format!("{functions}{values}")
        }
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
        ClassificationWorkload::ExportedChain
        | ClassificationWorkload::IndependentInvalid
        | ClassificationWorkload::DuplicateCallable => "value_0",
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
        ClassificationWorkload::DuplicateCallable => "name.duplicate",
    };
    let expected_count = match workload {
        ClassificationWorkload::ExportedChain => 1,
        ClassificationWorkload::IndependentInvalid | ClassificationWorkload::InvalidCasing => count,
        ClassificationWorkload::DuplicateCallable => count,
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

    assert_eq!(
        adjacent.exported_module_value_lookups,
        original.exported_module_value_lookups * 2
    );
    assert!(adjacent.retained_values <= original.retained_values * 2);
    assert!(adjacent.retained_edges <= original.retained_edges * 2 + 1);
    assert!(adjacent.retained_direct_causes <= original.retained_direct_causes * 2);
    assert!(adjacent.visited_initializations <= original.visited_initializations * 2);
    assert!(adjacent.traversal_values <= original.traversal_values * 2);
    assert!(adjacent.traversal_edges <= original.traversal_edges * 2 + 1);
    assert!(adjacent.cause_processing <= original.cause_processing * 2);
    assert!(adjacent.cause_index_insertions <= original.cause_index_insertions * 2);
    assert!(adjacent.diagnostic_lookups <= original.diagnostic_lookups * 2);
    assert!(adjacent.collision_identity_insertions <= original.collision_identity_insertions * 2);
    assert!(adjacent.collision_diagnostic_lookups <= original.collision_diagnostic_lookups * 2);
    assert!(
        adjacent.collision_association_iterations <= original.collision_association_iterations * 2
    );
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

#[test]
fn duplicate_callable_classification_uses_keyed_span_lookups_linearly() {
    let original = classification_work(32, ClassificationWorkload::DuplicateCallable);
    let adjacent = classification_work(64, ClassificationWorkload::DuplicateCallable);
    let larger = classification_work(128, ClassificationWorkload::DuplicateCallable);
    eprintln!(
        "duplicate callable classification work: 32={original:?}, 64={adjacent:?}, 128={larger:?}"
    );

    assert_linear_adjacent_work(original, adjacent);
    assert_linear_adjacent_work(adjacent, larger);
    assert_eq!(original.collision_identity_insertions, 64);
    assert_eq!(adjacent.collision_identity_insertions, 128);
    assert_eq!(larger.collision_identity_insertions, 256);
    assert_eq!(original.collision_diagnostic_lookups, 32);
    assert_eq!(adjacent.collision_diagnostic_lookups, 64);
    assert_eq!(larger.collision_diagnostic_lookups, 128);
    assert_eq!(original.collision_association_iterations, 32);
    assert_eq!(adjacent.collision_association_iterations, 64);
    assert_eq!(larger.collision_association_iterations, 128);
}

fn captured_collision_package_work(count: usize) -> PackageDocSemanticClassificationWork {
    let collision_source = concat!(
        "fn shared() -> Int\n",
        "  1\n",
        "end\n",
        "pub let shared: Int = 1\n",
    );
    let mut manifest = String::from(
        "[package]\nname = \"captured-collision-scaling\"\n[lib]\nexports = [\"main.veln\"]\n",
    );
    let mut imports = String::new();
    let mut values = String::new();
    let dependencies = (0..count)
        .map(|index| {
            let package = format!("p{index}");
            manifest.push_str(&format!(
                "[dependencies.\"{package}\"]\npath = \"vendor/{package}\"\n"
            ));
            imports.push_str(&format!("use main as dep_{index} from \"{package}\"\n"));
            values.push_str(&format!(
                "pub let value_{index}: Int = dep_{index}::shared\n"
            ));
            captured_same_path_dependency(&package, &format!("vendor/{package}"), collision_source)
        })
        .collect::<Vec<_>>();
    let source = format!("{imports}{values}");
    reset_package_doc_semantic_classification_work();
    let result =
        generate_with_dependencies(&manifest, &[("main.veln", source.as_str())], &dependencies);
    assert!(result.catalog().is_none(), "{:#?}", result.status());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "name.duplicate")
            .count(),
        count,
        "{:#?}",
        result.status().diagnostics
    );
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.gate == "module_value")
    );
    take_package_doc_semantic_classification_work()
}

#[test]
fn captured_identical_span_collision_association_is_linear() {
    let original = captured_collision_package_work(8);
    let adjacent = captured_collision_package_work(16);
    eprintln!("captured collision association work: 8={original:?}, 16={adjacent:?}");

    assert_eq!(original.collision_association_iterations, 8);
    assert_eq!(adjacent.collision_association_iterations, 16);
    assert!(
        adjacent.collision_association_iterations <= original.collision_association_iterations * 2
    );
}
