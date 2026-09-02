use super::*;

#[test]
fn declaration_uri_lookup_accepts_navigation_name_locations() {
    let source_text = concat!(
        "pub type Choice\n",
        "\tpub Some(value: Int)\n",
        "end\n",
        "\n",
        "pub fn value() -> Int\n",
        "\tSome(1)\n",
        "end\n",
        "\n",
        "pub fn caller() -> Int\n",
        "\tvalue()\n",
        "end\n",
    );
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[("main.veln", source_text)],
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source).tree;
    let SyntaxItem::Type(type_decl) = &parsed.items[0] else {
        panic!("expected type declaration");
    };
    let SyntaxItem::Function(function) = &parsed.items[1] else {
        panic!("expected function declaration");
    };
    let catalog = catalog_or_panic(&result);
    let type_uri = catalog.modules[0].declarations[0].uri.as_str();
    let function_uri = catalog.modules[0].declarations[1].uri.as_str();
    let snapshot = crate::EffectiveProjectSnapshot::new(vec![source.clone()]);
    let constructor_navigation = crate::navigate(
        &snapshot,
        crate::SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column: 3,
        },
    )
    .expect("constructor navigation");
    let function_navigation = crate::navigate(
        &snapshot,
        crate::SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 10,
            column: 3,
        },
    )
    .expect("function navigation");

    for (span, uri) in [
        (
            name_span_in(&source, &type_decl.span, "Choice").expect("type name span"),
            type_uri,
        ),
        (
            name_span_in(&source, &function.span, "value").expect("function name span"),
            function_uri,
        ),
    ] {
        assert_eq!(
            result.declaration_uri_for_location(&NavigationLocation {
                source: NavigationSource::Package {
                    uri: source_uri("demo", result.snapshot_digest(), "main.veln"),
                },
                span,
            }),
            Some(uri)
        );
    }
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: source_uri("demo", result.snapshot_digest(), "main.veln"),
            },
            span: constructor_navigation.definition.span.clone(),
        }),
        Some(type_uri)
    );
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: source_uri("demo", result.snapshot_digest(), "main.veln"),
            },
            span: function_navigation.definition.span.clone(),
        }),
        Some(function_uri)
    );
}

#[test]
fn private_documentation_references_do_not_fail_public_catalog() {
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
                "## Private docs mention {@schema Missing}.\n",
                "fn hidden() -> Int\n",
                "\t0\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].declarations.len(), 1);
    assert!(result.status().diagnostics.is_empty());
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("Missing")
    );
}

#[test]
fn package_bytes_and_generator_contract_change_document_identity() {
    let base_manifest = "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n";
    let first = generate(
        base_manifest,
        &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
    );
    let changed_source = generate(
        base_manifest,
        &[("main.veln", "pub fn value() -> Int\n\t2\nend\n")],
    );
    let snapshot = capture_embedded_package_snapshot(
        base_manifest.as_bytes(),
        [PackageSnapshotSource::new(
            "main.veln",
            b"pub fn value() -> Int\n\t1\nend\n",
        )],
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", base_manifest);
    let identity = PackageIdentity::new("demo").unwrap();
    let changed_contract = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-b"),
    );

    assert_ne!(first.snapshot_digest(), changed_source.snapshot_digest());
    assert_ne!(first.doc_digest(), changed_source.doc_digest());
    assert_eq!(first.snapshot_digest(), changed_contract.snapshot_digest());
    assert_ne!(first.doc_digest(), changed_contract.doc_digest());
}

#[test]
fn manifest_must_match_captured_snapshot_bytes() {
    let snapshot_manifest = "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n";
    let swapped_manifest = "[package]\nname = \"demo\"\n[lib]\nexports = [\"other.veln\"]\n";
    let snapshot = capture_embedded_package_snapshot(
        snapshot_manifest.as_bytes(),
        [
            PackageSnapshotSource::new("main.veln", b"pub fn value() -> Int\n\t1\nend\n"),
            PackageSnapshotSource::new("other.veln", b"pub fn other() -> Int\n\t2\nend\n"),
        ],
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", swapped_manifest);
    let identity = PackageIdentity::new("demo").unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == "package_doc.manifest_snapshot_mismatch" })
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn package_documentation_reparses_manifest_from_the_captured_snapshot() {
    let manifest_text = "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n";
    let snapshot = capture_embedded_package_snapshot(
        manifest_text.as_bytes(),
        [
            PackageSnapshotSource::new("main.veln", b"pub fn visible() -> Int\n\t1\nend\n"),
            PackageSnapshotSource::new("private.veln", b"pub fn secret() -> Int\n\t2\nend\n"),
        ],
    )
    .unwrap();
    let mut manifest = parse_manifest_text("veln.toml", manifest_text);
    let private_span = manifest.lib.exports[0].path_span.clone();
    manifest.lib.exports[0] = veln_project::ManifestExport {
        path: "private.veln".to_string(),
        path_span: private_span,
    };
    manifest.package.fields.push(veln_project::ManifestField {
        key: "repository".to_string(),
        key_span: manifest.package.fields[0].key_span.clone(),
        value: "hidden".to_string(),
        value_span: manifest.package.fields[0].value_span.clone(),
    });
    let identity = PackageIdentity::new("demo").unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules.len(), 1);
    assert_eq!(catalog.modules[0].source_path, "main.veln");
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("secret")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("hidden")
    );
}

#[test]
fn manifest_location_uses_snapshot_canonical_source_uri() {
    let manifest_text =
        "[package]\nname = \"other\"\n[lib]\nexports = [\"main.veln\", \"missing.veln\"]\n";
    let snapshot = capture_embedded_package_snapshot(
        manifest_text.as_bytes(),
        [PackageSnapshotSource::new(
            "main.veln",
            b"pub fn visible() -> Int\n\t1\nend\n",
        )],
    )
    .unwrap();
    let first_manifest = parse_manifest_text("veln.toml", manifest_text);
    let relocated_manifest = parse_manifest_text("relocated/veln.toml", manifest_text);
    let identity = PackageIdentity::new("demo").unwrap();
    let first = PackageDocResult::generate(
        &identity,
        &snapshot,
        &first_manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );
    let relocated = PackageDocResult::generate(
        &identity,
        &snapshot,
        &relocated_manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

    assert_eq!(first.canonical_bytes(), relocated.canonical_bytes());
    assert_eq!(first.doc_digest(), relocated.doc_digest());
    assert_eq!(first.status_uri(), relocated.status_uri());
    assert!(first.status().diagnostics.iter().all(|diagnostic| {
        diagnostic.span.as_ref().is_none_or(|span| {
            span.source_uri == source_uri("demo", first.snapshot_digest(), "veln.toml")
        })
    }));
    assert!(
        !std::str::from_utf8(first.canonical_bytes())
            .unwrap()
            .contains("relocated")
    );
}

#[test]
fn renderer_only_stability_follows_canonical_result_bytes() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
    );
    let same_renderer_bytes = result.canonical_bytes().to_vec();
    assert_eq!(result.doc_digest(), doc_digest(&same_renderer_bytes));
}

#[test]
fn generation_failure_returns_status_without_partial_catalog() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"missing.veln\"]\n",
        &[(
            "main.veln",
            "## Bad reference {@schema Missing}.\npub fn value() -> Int\n\t1\nend\n",
        )],
    );

    assert!(result.catalog().is_none());
    assert!(matches!(
        result.kind(),
        PackageDocResultKind::Status(PackageDocGenerationStatus {
            state: PackageDocGeneration::Failed,
            ..
        })
    ));
    let status = result.status();
    assert_eq!(status.diagnostics.len(), 2);
    assert_eq!(status.diagnostics[0].gate, "documentation_reference");
    assert_eq!(status.diagnostics[1].gate, "export");
    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(bytes.contains("\"state\":\"failed\""));
    assert!(!bytes.contains("\"modules\""));
}

#[test]
fn manifest_gate_rejects_unvalidated_manifest_without_partial_catalog() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"demo\"\n",
            "[modules]\n",
            "\"main.veln\" = \"main\"\n",
            "[lib]\n",
            "exports = [\"main.test.veln\", \"../escape.veln\"]\n",
            "[dependencies.\"example\"]\n",
            "git = \"https://example.invalid/repo.git\"\n",
        ),
        &[
            ("main.veln", "pub fn value() -> Int\n\t1\nend\n"),
            ("main.test.veln", "pub fn helper() -> Int\n\t1\nend\n"),
        ],
    );

    assert!(result.catalog().is_none());
    let diagnostics = result
        .status()
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
        .collect::<Vec<_>>();
    assert!(diagnostics.contains(&("manifest", "package_doc.unsupported_manifest_section")));
    assert!(diagnostics.contains(&("manifest", "package_doc.invalid_export")));
    assert!(diagnostics.contains(&("manifest", "package_doc.missing_git_selector")));
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"modules\"")
    );
}

#[test]
fn executable_specification_fixture_observes_successful_catalog() {
    let result = generate_fixture("package-catalog-api-success");
    let catalog = catalog_or_panic(&result);

    assert_eq!(catalog.modules.len(), 2);
    assert_eq!(catalog.modules[0].name, "main");
    assert!(result.doc_digest().len() == 64);
    assert!(
        catalog.modules[0]
            .declarations
            .iter()
            .any(|declaration| declaration.name == "visible")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("hidden_helper")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("leaked_test_api")
    );
}

#[test]
fn integration_test_exports_fail_manifest_gate() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main_test.veln\"]\n",
        &[
            ("main.veln", "pub fn value() -> Int\n\t1\nend\n"),
            (
                "main_test.veln",
                "pub fn leaked_test_api() -> Int\n\t@\nend\n",
            ),
        ],
    );

    assert!(result.catalog().is_none());
    assert!(
        result
            .status()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "package_doc.invalid_export")
    );
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("leaked_test_api")
    );
}

#[test]
fn invalid_source_module_export_fails_manifest_gate_without_partial_catalog() {
    let manifest_text = "[package]\nname = \"demo\"\n[lib]\nexports = [\"bad-name.veln\"]\n";
    let source_text = "pub fn leaked() -> Int\n\t1\nend\n";
    let snapshot = capture_embedded_package_snapshot(
        manifest_text.as_bytes(),
        [PackageSnapshotSource::new(
            "bad-name.veln",
            source_text.as_bytes(),
        )],
    )
    .unwrap();
    let manifest = parse_manifest_text("veln.toml", manifest_text);
    let identity = PackageIdentity::new("demo").unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

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
    assert!(
        !std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("leaked")
    );
    assert_eq!(snapshot.sources()[0].path(), "bad-name.veln");
    assert_eq!(snapshot.sources()[0].bytes(), source_text.as_bytes());
}

#[test]
fn source_path_casing_export_failure_is_package_atomic() {
    let result = generate_fixture("package-catalog-source-path-casing-gate");

    assert!(result.catalog().is_none());
    assert!(matches!(
        result.kind(),
        PackageDocResultKind::Status(PackageDocGenerationStatus {
            state: PackageDocGeneration::Failed,
            ..
        })
    ));
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].gate, "manifest");
    assert_eq!(diagnostics[0].code, "name.invalid_case");
    assert_eq!(
        diagnostics[0].message,
        "module name `Api` must start with an ASCII lowercase letter"
    );
    assert_eq!(
        diagnostics[0]
            .span
            .as_ref()
            .map(|span| span.source_uri.as_str()),
        Some(
            source_uri(
                "package-catalog-source-path-casing-gate",
                result.snapshot_digest(),
                "Api.veln"
            )
            .as_str()
        )
    );
    assert_eq!(
        result.declaration_uri_for("main", "function", "visible"),
        None
    );
    assert_eq!(
        result.declaration_uri_for("Api", "function", "leaked"),
        None
    );
    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(bytes.contains("\"state\":\"failed\""));
    assert!(!bytes.contains("\"modules\""));
    assert!(!bytes.contains("\"exported_modules\""));
    assert!(!bytes.contains("visible"));
    assert!(!bytes.contains("leaked"));
    assert!(!bytes.contains("package_doc.invalid_export"));
}
