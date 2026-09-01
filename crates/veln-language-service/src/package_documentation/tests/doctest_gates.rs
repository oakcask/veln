use super::*;

#[test]
fn generated_static_gate_preserves_declaration_and_statement_origins() {
    let source = SourceFile::new(
        "sample.veln",
        concat!(
            "test original() -> ()\n",
            "  fn sample() -> Int\n",
            "    1\n",
            "  end\n",
            "  sample()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let generated = generated_doctest_static_gate_source(&source);

    assert_eq!(
        generated.source.text(),
        concat!(
            "fn sample() -> Int\n",
            "  1\n",
            "end\n",
            "test doctest_body() -> () effects [stdio]\n",
            "  sample()\n",
            "  ()\n",
            "end\n",
        )
    );
    let origins = generated
        .line_origins
        .iter()
        .map(|(generated_line, origin)| {
            (
                *generated_line,
                origin.original_span.start.line,
                origin.generated_content_column,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(origins, vec![(1, 2, 1), (2, 3, 1), (3, 4, 1), (5, 5, 3)]);
}

#[test]
fn executable_specification_fixture_observes_manifest_gate_failure() {
    let result = generate_fixture("package-catalog-manifest-gate");

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "manifest" && diagnostic.code == "package_doc.invalid_export"
    }));
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "module.invalid_source_path")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn executable_specification_fixture_observes_doctest_static_gate_failure() {
    let result = generate_fixture("package-catalog-doctest-static-gate");

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.gate == "doctest")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn executable_specification_fixture_observes_doctest_metadata_gate_failure() {
    let result = generate_fixture("package-catalog-doctest-output-metadata-gate");

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.code == "doctest.invalid_metadata"
            && diagnostic.message == "duplicate doctest output stream attribute"
    }));
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn doctest_validation_reports_extraction_and_static_gate_failures() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "## ```veln-output stream=stdout stream=stderr\n",
                "## output\n",
                "## ```\n",
                "## ```veln\n",
                "## let value: MissingType = missing_value\n",
                "## value\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "doctest.invalid_metadata"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.source_uri == source_uri("demo", result.snapshot_digest(), "main.veln")
                    && span.line == 4
            })
    }));
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.code != "doctest.invalid_metadata"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.source_uri == source_uri("demo", result.snapshot_digest(), "main.veln")
                    && span.line == 8
            })
    }));
}

#[test]
fn executable_specification_fixture_observes_nested_doctest_success() {
    let result = generate_fixture("package-catalog-nested-doctest");
    let catalog = catalog_or_panic(&result);

    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn executable_specification_fixture_observes_adr_lite_doctest_exclusion() {
    let result = generate_fixture("package-catalog-adr-lite-doctest-boundary");
    let catalog = catalog_or_panic(&result);
    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();

    assert_eq!(catalog.modules[0].declarations[0].doc, Vec::<String>::new());
    assert!(catalog.modules[0].declarations[0].doctests.is_empty());
    assert!(catalog.modules[0].declarations[0].references.is_empty());
    assert!(result.status().diagnostics.is_empty());
    assert!(!bytes.contains("@adr-lite"));
    assert!(!bytes.contains("MissingType"));
    assert!(!bytes.contains("PrivatePacket"));
}

#[test]
fn executable_specification_fixture_observes_schema_reference_import_gate() {
    let result = generate_fixture("package-catalog-schema-reference-import-gate");

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
fn parse_and_doctest_gates_are_package_atomic() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## @\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert_eq!(result.status().diagnostics[0].gate, "doctest");
}

#[test]
fn positive_doctest_must_pass_generated_static_analysis() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## let value: MissingType = missing_value\n",
                "## value\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.source_uri == source_uri("demo", result.snapshot_digest(), "main.veln")
                    && span.line == 2
            })
    }));
}

#[test]
fn declaration_positive_doctest_must_pass_generated_static_analysis() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## fn sample() -> MissingType\n",
                "## \tmissing_value\n",
                "## end\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.source_uri == source_uri("demo", result.snapshot_digest(), "main.veln")
                    && span.line == 3
            })
    }));
}

#[test]
fn declaration_doctest_statement_body_must_pass_generated_static_analysis() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## fn sample() -> Int\n",
                "## \t1\n",
                "## end\n",
                "## missing_value\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.span.as_ref().is_some_and(|span| {
                span.source_uri == source_uri("demo", result.snapshot_digest(), "main.veln")
                    && span.line == 5
            })
    }));
}

#[test]
fn declaration_doctest_with_nested_block_passes_static_gate() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## fn sample(flag: Bool) -> Int\n",
                "## \tif flag\n",
                "## \t\t1\n",
                "## \telse\n",
                "## \t\t2\n",
                "## \tend\n",
                "## end\n",
                "## sample(true)\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn declaration_doctest_with_nested_expression_and_alias_passes_static_gate() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## fn target() -> Int\n",
                "## \t1\n",
                "## end\n",
                "## pub fn alias = target\n",
                "## fn sample(flag: Bool) -> Int\n",
                "## \tlet value = if flag\n",
                "## \t\talias()\n",
                "## \telse\n",
                "## \t\t2\n",
                "## \tend\n",
                "## \tvalue\n",
                "## end\n",
                "## sample(true)\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn positive_doctest_can_reference_exported_public_api() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## value(1)\n",
                "## ```\n",
                "pub fn value(input: Int) -> Int\n",
                "\tinput\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn fail_doctest_rejects_semantic_only_diagnostic() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln fail\n",
                "## missing_value\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest" && diagnostic.code == "doctest.expected_failure_missing"
    }));
}

#[test]
fn alias_doctest_can_mix_endless_declaration_and_statement() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## pub type Raw\n",
                "## end\n",
                "## pub type Count = Raw\n",
                "## ()\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations[0].doctests.len(), 1);
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn private_declaration_doctest_does_not_gate_public_catalog() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## Public docs.\n",
                "pub fn visible() -> Int\n",
                "\t1\n",
                "end\n",
                "\n",
                "## Private doctest must not be checked or published.\n",
                "## ```veln\n",
                "## missing_value\n",
                "## ```\n",
                "fn hidden() -> Int\n",
                "\t0\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations.len(), 1);
    assert!(catalog.modules[0].declarations[0].doctests.is_empty());
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("missing_value")
    );
}

#[test]
fn duplicate_expected_output_stream_fails_generation() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "## ```veln-output stream=stdout\n",
                "## first\n",
                "## ```\n",
                "## ```veln-output stream=stdout\n",
                "## second\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest" && diagnostic.code == "doctest.duplicate_output"
    }));
}

#[test]
fn ambiguous_expected_output_stream_attribute_fails_generation() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## 1\n",
                "## ```\n",
                "## ```veln-output stream=stdout stream=stderr\n",
                "## mixed\n",
                "## ```\n",
                "pub fn value() -> Int\n",
                "\t1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "doctest"
            && diagnostic.code == "doctest.invalid_metadata"
            && diagnostic.message == "duplicate doctest output stream attribute"
    }));
}
