use super::*;

#[test]
fn successful_catalog_contains_only_exported_public_api_and_allowed_metadata() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"demo\"\n",
            "version = \"1.2.3\"\n",
            "description = \"Public docs.\"\n",
            "license = \"MIT\"\n",
            "authors = \"Ada, Bea\"\n",
            "keywords = \"docs, api\"\n",
            "repository = \"hidden\"\n",
            "[lib]\n",
            "exports = [\"main.veln\"]\n",
            "[dependencies.other]\n",
            "path = \"../other\"\n",
            "[tool.secret]\n",
            "token = \"hidden\"\n",
        ),
        &[
            (
                "main.veln",
                concat!(
                    "## Public type docs.\n",
                    "pub type ResultBox<A>\n",
                    "\t## Ready constructor docs mention {@schema Packet}.\n",
                    "\t## ```veln\n",
                    "\t## 1\n",
                    "\t## ```\n",
                    "\tpub Ready(value: A)\n",
                    "\tHidden(reason: String)\n",
                    "end\n",
                    "\n",
                    "## Public schema docs.\n",
                    "pub schema Packet\n",
                    "\tformat binary\n",
                    "\tvalue: UInt8\n",
                    "end\n",
                    "\n",
                    "## Function docs mention {@schema Packet}.\n",
                    "## ```veln\n",
                    "## fn sample() -> Int\n",
                    "## \t1\n",
                    "## end\n",
                    "## ```\n",
                    "## ```veln-output stream=stdout\n",
                    "## 1\n",
                    "## ```\n",
                    "## ```veln-output stream=stderr\n",
                    "## note\n",
                    "## ```\n",
                    "pub fn value(input: Int) -> output: Int\n",
                    "\trequire input >= 0\n",
                    "\tensure output >= input\n",
                    "\tinput\n",
                    "end\n",
                    "\n",
                    "pub type PacketAlias = Packet\n",
                    "\n",
                    "fn private_helper() -> Int\n",
                    "\t0\n",
                    "end\n",
                ),
            ),
            (
                "hidden.veln",
                "## Hidden module.\npub fn hidden_public() -> Int\n\t1\nend\n",
            ),
        ],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.metadata.package_name.as_deref(), Some("demo"));
    assert_eq!(catalog.metadata.version.as_deref(), Some("1.2.3"));
    assert_eq!(catalog.metadata.authors, ["Ada", "Bea"]);
    assert_eq!(catalog.metadata.keywords, ["docs", "api"]);
    assert_eq!(catalog.modules.len(), 1);
    assert_eq!(catalog.modules[0].name, "main");
    assert_eq!(catalog.modules[0].doc, Vec::<String>::new());
    assert_eq!(
        catalog.modules[0]
            .declarations
            .iter()
            .map(|declaration| (declaration.kind.as_str(), declaration.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("type", "ResultBox"),
            ("schema", "Packet"),
            ("function", "value"),
            ("alias", "PacketAlias"),
        ]
    );
    assert_eq!(catalog.modules[0].declarations[0].constructors.len(), 1);
    let constructor = &catalog.modules[0].declarations[0].constructors[0];
    assert_eq!(
        constructor.doc,
        [
            "Ready constructor docs mention {@schema Packet}.",
            "```veln",
            "1",
            "```"
        ]
    );
    assert_eq!(constructor.doctests.len(), 1);
    assert_eq!(constructor.references.len(), 1);
    assert_eq!(constructor.references[0].marker, "Packet");
    assert_eq!(catalog.modules[0].declarations[2].contracts.len(), 2);
    assert_eq!(catalog.modules[0].declarations[2].doctests.len(), 1);
    assert_eq!(
        catalog.modules[0].declarations[2].doctests[0].expected_output,
        [
            PackageDocExpectedOutput {
                stream: "stdout".to_string(),
                lines: vec!["1".to_string()],
            },
            PackageDocExpectedOutput {
                stream: "stderr".to_string(),
                lines: vec!["note".to_string()],
            },
        ]
    );
    let bytes = std::str::from_utf8(result.canonical_bytes()).unwrap();
    assert!(bytes.contains("\"doc\":[\"Function docs mention {@schema Packet}.\""));
    assert!(!bytes.contains("hidden_public"));
    assert!(!bytes.contains("private_helper"));
    assert!(!bytes.contains("repository"));
    assert!(!bytes.contains("token"));
}

#[test]
fn catalog_identity_digest_and_uris_are_deterministic() {
    let first = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
    );
    let second = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
    );

    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.doc_digest(), second.doc_digest());
    assert_eq!(first.doc_digest(), doc_digest(first.canonical_bytes()));
    assert_eq!(first.status_uri(), second.status_uri());
    let catalog = first.catalog().unwrap();
    assert!(catalog.index_uri.starts_with("veln-doc:///package/demo/"));
    assert_eq!(catalog.modules[0].id.len(), 64);
    assert_eq!(catalog.modules[0].declarations[0].id.len(), 64);
    assert_eq!(
        first.declaration_uri_for("main", "function", "value"),
        Some(catalog.modules[0].declarations[0].uri.as_str())
    );
}

#[test]
fn metadata_exported_modules_use_validated_normalized_exports() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"./main.veln\"]\n",
        &[("main.veln", "pub fn value() -> Int\n\t1\nend\n")],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].name, "main");
    assert_eq!(catalog.metadata.exported_modules, ["main"]);
}

#[test]
fn package_documentation_requires_validated_package_identity() {
    assert!(PackageIdentity::new("").is_err());
    let identity = PackageIdentity::new("owner/package").unwrap();
    let snapshot = capture_embedded_package_snapshot(
        b"[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        [PackageSnapshotSource::new(
            "main.veln",
            b"pub fn value() -> Int\n\t1\nend\n",
        )],
    )
    .unwrap();
    let manifest = parse_manifest_text(
        "veln.toml",
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
    );
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "manifest" && diagnostic.code == "package_doc.package_identity_mismatch"
    }));
}

#[test]
fn package_documentation_requires_manifest_package_name() {
    let identity = PackageIdentity::new("demo").unwrap();
    let snapshot = capture_embedded_package_snapshot(
        b"[package]\n[lib]\nexports = [\"main.veln\"]\n",
        [PackageSnapshotSource::new(
            "main.veln",
            b"pub fn value() -> Int\n\t1\nend\n",
        )],
    )
    .unwrap();
    let manifest =
        parse_manifest_text("veln.toml", "[package]\n[lib]\nexports = [\"main.veln\"]\n");
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new("contract-a"),
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "manifest" && diagnostic.code == "package_doc.missing_package_name"
    }));
}

#[test]
fn digest_transcript_uses_fixed_u64_part_lengths() {
    assert_eq!(
        digest_hex(b"test-domain\0", &[b"abc", b"def"]),
        "0cb6b848276df1a8558995a0f050e7b059723b9a32d1f1420fd2fea8df425a97"
    );
}

#[test]
fn module_id_transcript_uses_package_relative_source_path() {
    assert_eq!(
        module_id("main.veln"),
        "99fdce218d5cf98b5827059d4aceb6f6e0908da36f8ae511e5a7e5c20e794d58"
    );
    assert_eq!(
        module_id("nested/main.veln"),
        "02d8fbb7d36f15f834bd95986ebcefd743c9ceadf3c49e36aaecee2a86f4288e"
    );
}

#[test]
fn resolved_schema_references_keep_target_identity_and_uri() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"wire.veln\"]\n",
        &[
            (
                "main.veln",
                concat!(
                    "use wire\n",
                    "\n",
                    "pub type Choice\n",
                    "\t## Constructor uses {@schema wire::Packet}.\n",
                    "\tpub Ready(value: Int)\n",
                    "end\n",
                    "\n",
                    "## Uses {@schema wire::Packet}.\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            ),
            (
                "wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "\tformat binary\n",
                    "\tvalue: UInt8\n",
                    "end\n",
                ),
            ),
        ],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules[0].name, "main");
    let constructor = &catalog.modules[0].declarations[0].constructors[0];
    let function = &catalog.modules[0].declarations[1];
    let schema = &catalog.modules[1].declarations[0];
    assert_eq!(constructor.references.len(), 1);
    assert_eq!(constructor.references[0].target_declaration_id, schema.id);
    assert_eq!(constructor.references[0].target_uri, schema.uri);
    assert_eq!(function.references.len(), 1);
    assert_eq!(function.references[0].marker, "wire::Packet");
    assert_eq!(function.references[0].target_declaration_id, schema.id);
    assert_eq!(function.references[0].target_uri, schema.uri);
    assert!(
        std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("\"references\":[{\"kind\":\"schema\"")
    );
}

#[test]
fn qualified_schema_reference_requires_written_import() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"wire.veln\"]\n",
        &[
            (
                "main.veln",
                concat!(
                    "## Missing import {@schema wire::Packet}.\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            ),
            (
                "wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "\tformat binary\n",
                    "\tvalue: UInt8\n",
                    "end\n",
                ),
            ),
        ],
    );

    assert!(result.catalog().is_none());
    assert!(result.status().diagnostics.iter().any(|diagnostic| {
        diagnostic.gate == "documentation_reference"
            && diagnostic.code == "package_doc.unresolved_schema_reference"
    }));
}

#[test]
fn public_schema_alias_reference_resolves_to_public_schema_target() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\", \"facade.veln\", \"wire.veln\"]\n",
        &[
            (
                "main.veln",
                concat!(
                    "use facade\n",
                    "\n",
                    "## Uses alias {@schema facade::AliasPacket}.\n",
                    "pub fn value() -> Int\n",
                    "\t1\n",
                    "end\n",
                ),
            ),
            (
                "facade.veln",
                concat!(
                    "use wire\n",
                    "\n",
                    "pub schema AliasPacket = wire::Packet\n"
                ),
            ),
            (
                "wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "\tformat binary\n",
                    "\tvalue: UInt8\n",
                    "end\n",
                ),
            ),
        ],
    );

    let catalog = catalog_or_panic(&result);
    let function = catalog
        .modules
        .iter()
        .find(|module| module.name == "main")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.name == "value")
        .unwrap();
    let schema = catalog
        .modules
        .iter()
        .find(|module| module.name == "wire")
        .unwrap()
        .declarations
        .iter()
        .find(|declaration| declaration.name == "Packet")
        .unwrap();
    assert_eq!(function.references.len(), 1);
    assert_eq!(function.references[0].target_declaration_id, schema.id);
    assert_eq!(function.references[0].target_uri, schema.uri);
}

#[test]
fn effect_row_binder_is_part_of_public_function_signature_and_identity() {
    let result = generate(
        "[package]\nname = \"demo\"\n[lib]\nexports = [\"main.veln\"]\n",
        &[(
            "main.veln",
            concat!(
                "pub fn apply<effect E>(callback: fn(Int) -> Int effects [...E]) -> Int effects [stdio, ...E]\n",
                "\tcallback(1)\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let function = &catalog.modules[0].declarations[0];
    assert_eq!(
        function.signature,
        "fn apply<effect E>(callback: fn(Int) -> Int effects [...E]) -> Int effects [stdio, ...E]"
    );
    assert!(
        std::str::from_utf8(result.canonical_bytes())
            .unwrap()
            .contains("<effect E>")
    );
}

#[test]
fn declaration_uri_lookup_accepts_declaration_and_constructor_locations() {
    let source_text = concat!(
        "pub type Choice\n",
        "\tpub Some(value: Int)\n",
        "end\n",
        "\n",
        "pub fn value() -> Int\n",
        "\t1\n",
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
    let package_source_uri = source_uri("demo", result.snapshot_digest(), "main.veln");

    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: package_source_uri.clone()
            },
            span: type_decl.span.clone(),
        }),
        Some(type_uri)
    );
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: package_source_uri.clone()
            },
            span: type_decl.variants[0].span.clone(),
        }),
        Some(type_uri)
    );
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: package_source_uri,
            },
            span: function.span.clone(),
        }),
        Some(function_uri)
    );
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Workspace,
            span: function.span.clone(),
        }),
        None
    );
    assert_eq!(
        result.declaration_uri_for_location(&NavigationLocation {
            source: NavigationSource::Package {
                uri: source_uri("demo", "different-snapshot", "main.veln"),
            },
            span: function.span.clone(),
        }),
        None
    );
}
