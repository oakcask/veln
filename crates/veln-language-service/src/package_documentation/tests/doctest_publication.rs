use super::*;

#[test]
fn doctest_metadata_gate_reports_once_at_original_source_position() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## Function docs.\n",
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "## ```veln-output stream=stdout stream=stderr\n",
                "## one\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let diagnostics = result
        .status()
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "doctest.invalid_metadata")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 1);
    let span = diagnostics[0].span.as_ref().unwrap();
    assert_eq!(span.line, 5);
    assert_eq!(span.column, 1);
    assert_eq!(span.offset, 41);
}

#[test]
fn ignored_doctest_does_not_capture_adjacent_expected_output() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln ignore\n",
                "## missing_value\n",
                "## ```\n",
                "## ```veln-output stream=stdout\n",
                "## ignored\n",
                "## ```\n",
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert_eq!(
        catalog.modules[0].declarations[0].doctests[0].expected_output,
        []
    );
}

#[test]
fn expected_output_after_prose_or_ignored_fence_does_not_attach_to_previous_doctest() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## stdio::println(\"first\")\n",
                "## ```\n",
                "## Prose ends the pending output association.\n",
                "## ```veln-output stream=stdout\n",
                "## first\n",
                "## ```\n",
                "## ```veln ignore\n",
                "## missing_value\n",
                "## ```\n",
                "## ```veln-output stream=stderr\n",
                "## ignored\n",
                "## ```\n",
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let doctests = &catalog.modules[0].declarations[0].doctests;
    assert_eq!(doctests.len(), 2);
    assert!(doctests[0].expected_output.is_empty());
    assert!(doctests[1].expected_output.is_empty());
}

#[test]
fn hidden_doctest_setup_does_not_gate_public_catalog() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## > let setup: MissingType = missing_value\n",
                "## 1\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let doctest = &catalog.modules[0].declarations[0].doctests[0];
    assert_eq!(doctest.code, "1");
    assert!(result.status().diagnostics.is_empty());
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("MissingType")
    );
}

#[test]
fn constructor_documentation_reference_failure_is_package_atomic() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "pub type Choice\n",
                "\t## Missing constructor reference {@schema Missing}.\n",
                "\tpub Some(value: Int)\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "documentation_reference"
            && diagnostic.code == "package_doc.unresolved_schema_reference"
    }));
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn duplicate_semantic_identity_fails_the_package() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "pub fn value() -> Int\n\t1\nend\n",
                "pub fn value() -> Int\n\t2\nend\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "package_doc.duplicate_semantic_identity" })
    );
}

#[test]
fn declaration_id_collision_fails_the_package() {
    let result = generate_with_forced_declaration_id(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "pub fn first() -> Int\n\t1\nend\n",
                "pub fn second() -> Int\n\t2\nend\n",
            ),
        )],
        "forced-declaration-id",
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "package_doc.declaration_id_collision")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn fail_doctest_is_published_when_generated_parse_diagnostic_matches() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln fail\n",
                "## @\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let doctest = &catalog.modules[0].declarations[0].doctests[0];
    assert_eq!(doctest.expected_error.as_deref(), None);
    assert!(doctest.should_fail);
    assert_eq!(doctest.code, "@");
    assert!(result.status().diagnostics.is_empty());
    assert!(
        std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"should_fail\":true")
    );
}

#[test]
fn fail_doctest_must_produce_generated_parse_diagnostic() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln fail\n",
                "## fn sample() -> Int\n",
                "## \t1\n",
                "## end\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "doctest.expected_failure_missing")
    );
}

#[test]
fn exported_parse_failure_keeps_virtual_source_catalog_resolvable() {
    let manifest_text = "[package]\nname = \"owner/package\"\n[lib]\nexports = [\"main.veln\"]\n";
    let source_text = "pub fn broken() -> ()\n  @\nend\n";
    let snapshot = capture_embedded_package_snapshot(
        manifest_text.as_bytes(),
        [PackageSnapshotSource::new(
            "main.veln",
            source_text.as_bytes(),
        )],
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", manifest_text);
    let identity = PackageIdentity::new("owner/package").unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );
    let virtual_sources = crate::VirtualSourceCatalog::new([(
        PackageIdentity::new("owner/package").unwrap(),
        snapshot,
    )])
    .unwrap();
    let entry = virtual_sources.entries().next().unwrap();

    assert!(result.catalog().is_none());
    assert_eq!(result.status().diagnostics[0].gate, "parse");
    assert!(
        result.status().diagnostics[0]
            .span
            .as_ref()
            .unwrap()
            .source_uri
            .starts_with("veln-pkg:///owner%2Fpackage/snapshot/")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
    assert_eq!(
        virtual_sources.resolve(entry.uri()),
        Some(source_text.as_bytes())
    );
}

#[test]
fn hidden_doctest_setup_and_adr_lite_are_not_published() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## @adr\n",
                "## id: hidden\n",
                "## status: accepted\n",
                "## scope: docs\n",
                "## context: hidden\n",
                "## decision: hidden\n",
                "## consequences: hidden\n",
                "\n",
                "## ```veln\n",
                "## > let setup: Int = 1\n",
                "## fn sample() -> Int\n",
                "## \t1\n",
                "## end\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(!bytes.contains("@adr"));
    assert!(!bytes.contains("let setup"));
    assert!(bytes.contains("fn sample()"));
}

#[test]
fn adr_lite_doc_block_doctests_are_not_gate_inputs() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## @adr-lite\n",
                "## id: local-note\n",
                "## status: accepted\n",
                "## context: private\n",
                "## decision: private\n",
                "## consequences: private\n",
                "## Invalid examples in ADR-lite blocks are private metadata.\n",
                "## ```veln\n",
                "## fn hidden() -> MissingType\n",
                "## \tmissing_value\n",
                "## end\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(catalog.modules[0].declarations[0].doc.is_empty());
    assert!(catalog.modules[0].declarations[0].doctests.is_empty());
    assert!(result.status().diagnostics.is_empty());
    assert!(!bytes.contains("@adr-lite"));
    assert!(!bytes.contains("MissingType"));
}
