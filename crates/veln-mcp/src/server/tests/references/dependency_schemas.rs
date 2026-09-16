use super::*;

#[test]
fn references_return_direct_dependency_schema_operations_from_selected_project() {
    let workspace = TempWorkspace::new("references-dependency-schema-project");
    write_schema_dependency_workspace(&workspace, DependencySchemaSource::Path);
    workspace.write(
        "other.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn other(view: ByteView) -> ()\n",
            "  decode dep::Packet from view at byte_offset(0)?\n",
            "  decode dep::Packet junk from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "broken_decode.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode dep::Packet from view byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "broken_encode.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn write(packet: {value: Int}) -> ()\n",
            "  encode dep::Packet junk from packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 4, 16);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 4, 15, 4, 21),
            ("main.veln", 5, 15, 5, 21),
            ("other.veln", 4, 15, 4, 21),
        ],
        "direct dependency schema operations",
    );

    let recovered = references_result(&workspace, "other.veln", 5, 16);
    assert_eq!(recovered["isError"], false, "{recovered:#}");
    assert_eq!(
        recovered["structuredContent"]["references"],
        json!([]),
        "{recovered:#}"
    );
    for source in ["broken_decode.veln", "broken_encode.veln"] {
        let recovered = references_result(&workspace, source, 4, 16);
        assert_eq!(recovered["isError"], false, "{source}: {recovered:#}");
        assert_eq!(
            recovered["structuredContent"]["references"],
            json!([]),
            "{source}: {recovered:#}"
        );
    }
}

#[test]
fn references_keep_sibling_selected_projects_isolated_for_dependency_schemas() {
    let workspace = TempWorkspace::new("references-dependency-schema-sibling-projects");
    let project_manifest = "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n";
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let source = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn read(view: ByteView) -> ()\n",
        "  decode dep::Packet from view at byte_offset(0)?\n",
        "  decode dep::Alias from view at byte_offset(0)?\n",
        "end\n",
    );
    for project in ["left", "right"] {
        workspace.write(&format!("{project}/veln.toml"), project_manifest);
        workspace.write(&format!("{project}/main.veln"), source);
        workspace.write(
            &format!("{project}/vendor/dep/veln.toml"),
            dependency_manifest,
        );
        workspace.write(
            &format!("{project}/vendor/dep/dep.veln"),
            "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
        );
    }

    let result = references_result(&workspace, "left/main.veln", 4, 16);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": "left",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[("left/main.veln", 4, 15, 4, 21)],
        "sibling project dependency schema isolation",
    );

    let alias = references_result(&workspace, "left/main.veln", 5, 16);
    assert_eq!(alias["isError"], false, "{alias:#}");
    assert_eq!(
        alias["structuredContent"]["scope"],
        result["structuredContent"]["scope"]
    );
    assert_reference_ranges(
        &alias,
        &[("left/main.veln", 5, 15, 5, 20)],
        "sibling project dependency schema alias isolation",
    );
}

#[test]
fn references_do_not_borrow_dependency_context_for_descendant_or_anonymous_sources() {
    struct Case {
        name: &'static str,
        source: &'static str,
        files: Vec<(&'static str, &'static str)>,
    }

    let operation = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn read(view: ByteView) -> ()\n",
        "  decode dep::Packet from view at byte_offset(0)?\n",
        "  decode dep::Alias from view at byte_offset(0)?\n",
        "end\n",
    );
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let dependency_source = "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n";
    let cases = [
        Case {
            name: "descendant project",
            source: "nested/main.veln",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                ("main.veln", operation),
                ("nested/veln.toml", ""),
                ("nested/main.veln", operation),
                ("vendor/dep/veln.toml", dependency_manifest),
                ("vendor/dep/dep.veln", dependency_source),
            ],
        },
        Case {
            name: "anonymous source",
            source: "loose.veln",
            files: vec![
                (
                    "app/veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                ("app/main.veln", operation),
                ("app/vendor/dep/veln.toml", dependency_manifest),
                ("app/vendor/dep/dep.veln", dependency_source),
                ("loose.veln", operation),
            ],
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }

        let result = references_result(&workspace, case.source, 4, 16);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"],
            json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": case.source,
                "project_wide": false
            }),
            "{}: {result:#}",
            case.name
        );
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );

        let alias = references_result(&workspace, case.source, 5, 16);
        assert_eq!(alias["isError"], false, "{}: {alias:#}", case.name);
        assert_eq!(
            alias["structuredContent"]["scope"], result["structuredContent"]["scope"],
            "{}: {alias:#}",
            case.name
        );
        assert_eq!(
            alias["structuredContent"]["references"],
            json!([]),
            "{}: {alias:#}",
            case.name
        );
    }
}

#[test]
fn references_exclude_unselected_descendant_sources_from_parent_project_scope() {
    let workspace = TempWorkspace::new("references-dependency-schema-descendant-exclusion");
    let operation = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn read(view: ByteView) -> ()\n",
        "  decode dep::Packet from view at byte_offset(0)?\n",
        "  decode dep::Alias from view at byte_offset(0)?\n",
        "end\n",
    );
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write("main.veln", operation);
    workspace.write("nested/veln.toml", "");
    workspace.write("nested/main.veln", operation);
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
    );

    let result = references_result(&workspace, "main.veln", 4, 16);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[("main.veln", 4, 15, 4, 21)],
        "parent project excludes descendant package sources",
    );

    let alias = references_result(&workspace, "main.veln", 5, 16);
    assert_eq!(alias["isError"], false, "{alias:#}");
    assert_eq!(
        alias["structuredContent"]["scope"],
        result["structuredContent"]["scope"]
    );
    assert_reference_ranges(
        &alias,
        &[("main.veln", 5, 15, 5, 20)],
        "parent project excludes descendant schema-alias operations",
    );
}

#[test]
fn references_return_empty_for_dependency_schema_operation_boundaries() {
    let workspace = TempWorkspace::new("references-dependency-schema-boundaries");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n\n",
            "[dependencies.\"other/dep\"]\npath = \"vendor/other\"\n\n",
            "[dependencies.\"bridge/dep\"]\npath = \"vendor/bridge\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use public from \"example/dep\"\n",
            "use private from \"example/dep\"\n",
            "use hidden from \"example/dep\"\n",
            "use mismatch from \"other/dep\"\n",
            "use transitive from \"transitive/dep\"\n",
            "\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode private::Private from view at byte_offset(0)?\n",
            "  decode hidden::Hidden from view at byte_offset(0)?\n",
            "  decode mismatch::Public from view at byte_offset(0)?\n",
            "  decode transitive::Public from view at byte_offset(0)?\n",
            "  decode public::badSchema from view at byte_offset(0)?\n",
            "  decode public::Missing from view at byte_offset(0)?\n",
            "  decode public::Alias from view at byte_offset(0)?\n",
            "  decode public::AliasChain from view at byte_offset(0)?\n",
            "  decode public::CollidingAlias from view at byte_offset(0)?\n",
            "  decode private::PrivateAlias from view at byte_offset(0)?\n",
            "  decode hidden::HiddenAlias from view at byte_offset(0)?\n",
            "  decode mismatch::OtherAlias from view at byte_offset(0)?\n",
            "  decode transitive::TransitiveAlias from view at byte_offset(0)?\n",
            "  decode public::badAlias from view at byte_offset(0)?\n",
            "  decode public::CrossModuleAlias from view at byte_offset(0)?\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: public::Public\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"public.veln\", \"private.veln\", \"core.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/public.veln",
        concat!(
            "use core\n\n",
            "pub schema Public\n  value: Int\nend\n\n",
            "pub schema badSchema\n  value: Int\nend\n\n",
            "pub schema Alias = Public\n\n",
            "pub schema AliasChain = Alias\n\n",
            "pub schema Other\n  value: Int\nend\n\n",
            "pub schema CollisionTarget\n  value: Int\nend\n\n",
            "pub schema CollisionTarget = Other\n\n",
            "pub schema CollidingAlias = CollisionTarget\n\n",
            "pub schema badAlias = Public\n\n",
            "pub schema CrossModuleAlias = core::Packet\n\n",
            "fn package_operations(view: ByteView, value: {value: Int}) -> ()\n",
            "  decode Public from view at byte_offset(0)?\n",
            "  encode Public from value\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/core.veln",
        "pub schema Packet\n  value: Int\nend\n",
    );
    workspace.write(
        "vendor/dep/private.veln",
        "schema Private\n  value: Int\nend\n\npub schema PrivateAlias = Private\n",
    );
    workspace.write(
        "vendor/dep/hidden.veln",
        "pub schema Hidden\n  value: Int\nend\n\npub schema HiddenAlias = Hidden\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"other.veln\"]\n",
    );
    workspace.write(
        "vendor/other/other.veln",
        concat!(
            "pub schema Public\n  value: Int\nend\n\n",
            "pub schema OtherAlias = Public\n",
        ),
    );
    workspace.write(
        "vendor/bridge/veln.toml",
        concat!(
            "[package]\nname = \"bridge/dep\"\n\n",
            "[lib]\nexports = [\"bridge.veln\"]\n\n",
            "[dependencies.\"transitive/dep\"]\npath = \"../transitive\"\n",
        ),
    );
    workspace.write(
        "vendor/bridge/bridge.veln",
        concat!(
            "use public from \"transitive/dep\"\n\n",
            "pub fn consume(view: ByteView) -> ()\n",
            "  decode public::Public from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/transitive/veln.toml",
        "[package]\nname = \"transitive/dep\"\n\n[lib]\nexports = [\"public.veln\"]\n",
    );
    workspace.write(
        "vendor/transitive/public.veln",
        concat!(
            "pub schema Public\n  value: Int\nend\n\n",
            "pub schema TransitiveAlias = Public\n",
        ),
    );
    workspace.write(
        "recovery.veln",
        concat!(
            "use public from \"example/dep\"\n\n",
            "fn read(Public: Int, view: ByteView) -> ()\n",
            "  decode Public from view at byte_offset(0)?\n",
            "end\n",
        ),
    );

    for (name, source, line, column) in [
        ("private", "main.veln", 8, 19),
        ("non-exported", "main.veln", 9, 18),
        ("mismatched import", "main.veln", 10, 20),
        ("transitive", "main.veln", 11, 22),
        ("invalid casing", "main.veln", 12, 18),
        ("unresolved", "main.veln", 13, 18),
        ("package alias chain", "main.veln", 15, 18),
        ("alias target collision", "main.veln", 16, 18),
        ("private alias target", "main.veln", 17, 19),
        ("non-exported alias", "main.veln", 18, 18),
        ("mismatched alias import", "main.veln", 19, 20),
        ("transitive alias", "main.veln", 20, 22),
        ("invalid-casing alias", "main.veln", 21, 18),
        ("valid cross-module alias target", "main.veln", 22, 18),
        ("package composition", "main.veln", 26, 19),
        ("module qualifier", "main.veln", 12, 10),
        ("recovery", "recovery.veln", 4, 10),
    ] {
        let result = references_result(&workspace, source, line, column);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(
            result["structuredContent"]["scope"],
            json!({
                "mode": "project",
                "generation": 0,
                "project": ".",
                "project_wide": true
            }),
            "{name}: {result:#}"
        );
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{name}: {result:#}"
        );
    }

    let alias = references_result(&workspace, "main.veln", 14, 18);
    assert_eq!(alias["isError"], false, "{alias:#}");
    assert_reference_ranges(
        &alias,
        &[("main.veln", 14, 18, 14, 23)],
        "direct dependency schema alias operation",
    );
}

#[test]
fn references_keep_recovered_dependency_schema_alias_declarations_empty() {
    let workspace = TempWorkspace::new("references-recovered-dependency-schema-alias");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode dep::Alias from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"alias.veln\", \"schema.veln\"]\n",
        ),
    );
    workspace.write("vendor/dep/alias.veln", "mod dep\n\npub schema Alias =\n");
    workspace.write(
        "vendor/dep/schema.veln",
        "mod dep\n\npub schema Alias\n  value: Int\nend\n",
    );

    let result = references_result(&workspace, "main.veln", 4, 16);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
}

#[test]
fn references_keep_recovered_duplicate_dependency_schema_aliases_empty() {
    let workspace = TempWorkspace::new("references-recovered-duplicate-dependency-schema-alias");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode dep::Alias from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"valid.veln\", \"recovered.veln\"]\n",
        ),
    );
    workspace.write(
        "vendor/dep/valid.veln",
        concat!(
            "mod dep\n\n",
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
        ),
    );
    workspace.write(
        "vendor/dep/recovered.veln",
        "mod dep\n\npub schema Alias =\n",
    );

    let result = references_result(&workspace, "main.veln", 4, 16);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
}

#[test]
fn references_keep_dependency_schema_aliases_behind_invalid_imports_empty() {
    for (name, imports, line) in [
        (
            "duplicate",
            concat!(
                "use dep from \"example/dep\"\n",
                "use dep from \"example/dep\"\n",
            ),
            5,
        ),
        ("recovered", "use dep from \"example/dep\" unexpected\n", 4),
    ] {
        let workspace =
            TempWorkspace::new(&format!("references-dependency-schema-alias-{name}-import"));
        workspace.write(
            "veln.toml",
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
        );
        workspace.write(
            "main.veln",
            &format!(
                "{imports}\nfn read(view: ByteView) -> ()\n  decode dep::Alias from view at byte_offset(0)?\nend\n"
            ),
        );
        workspace.write(
            "vendor/dep/veln.toml",
            concat!(
                "[package]\nname = \"example/dep\"\n\n",
                "[lib]\nexports = [\"dep.veln\"]\n",
            ),
        );
        workspace.write(
            "vendor/dep/dep.veln",
            concat!(
                "pub schema Packet\n  value: Int\nend\n\n",
                "pub schema Alias = Packet\n",
            ),
        );

        let result = references_result(&workspace, "main.veln", line, 16);

        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{name}: {result:#}"
        );
        assert_eq!(
            result["structuredContent"]["scope"],
            json!({
                "mode": "project",
                "generation": 0,
                "project": ".",
                "project_wide": true
            }),
            "{name}: {result:#}"
        );
    }
}

#[test]
fn references_accept_all_direct_dependency_schema_source_kinds() {
    for source_kind in [
        DependencySchemaSource::Path,
        DependencySchemaSource::Vendor,
        DependencySchemaSource::Mirror,
        DependencySchemaSource::Git,
    ] {
        let workspace = TempWorkspace::new(source_kind.name());
        write_schema_dependency_workspace(&workspace, source_kind);

        let result = references_result(&workspace, "main.veln", 6, 16);

        assert_eq!(
            result["isError"],
            false,
            "{}: {result:#}",
            source_kind.name()
        );
        assert_reference_ranges(
            &result,
            &[("main.veln", 6, 15, 6, 20), ("main.veln", 7, 15, 7, 20)],
            source_kind.name(),
        );
    }
}

#[derive(Clone, Copy)]
enum DependencySchemaSource {
    Path,
    Vendor,
    Mirror,
    Git,
}

impl DependencySchemaSource {
    fn name(self) -> &'static str {
        match self {
            Self::Path => "dependency-schema-path",
            Self::Vendor => "dependency-schema-vendor",
            Self::Mirror => "dependency-schema-mirror",
            Self::Git => "dependency-schema-local-git",
        }
    }

    fn dependency_table(self) -> &'static str {
        match self {
            Self::Path => "[dependencies.\"example/dep\"]\npath = \"vendor/path-dep\"\n",
            Self::Vendor => "[dependencies.\"example/dep\"]\nvendor = \"vendor/vendor-dep\"\n",
            Self::Mirror => "[dependencies.\"example/dep\"]\nmirror = \"mirror/example/dep\"\n",
            Self::Git => concat!(
                "[dependencies.\"example/dep\"]\n",
                "git = \"https://example.invalid/schema-dep.git\"\n",
                "rev = \"abc123\"\n",
            ),
        }
    }
}

fn write_schema_dependency_workspace(
    workspace: &TempWorkspace,
    source_kind: DependencySchemaSource,
) {
    workspace.write("veln.toml", source_kind.dependency_table());
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode dep::Packet from view at byte_offset(0)?\n",
            "  encode dep::Packet from packet\n",
            "  decode dep::Alias from view at byte_offset(0)?\n",
            "  encode dep::Alias from packet\n",
            "end\n",
        ),
    );
    let dependency_root = match source_kind {
        DependencySchemaSource::Path => "vendor/path-dep".to_string(),
        DependencySchemaSource::Vendor => "vendor/vendor-dep".to_string(),
        DependencySchemaSource::Mirror => "mirror/example/dep".to_string(),
        DependencySchemaSource::Git => veln_project::materialized_git_repository_root(
            &workspace.root,
            "https://example.invalid/schema-dep.git",
        )
        .strip_prefix(&workspace.root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/"),
    };
    workspace.write(
        &format!("{dependency_root}/veln.toml"),
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        &format!("{dependency_root}/dep.veln"),
        concat!(
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n\n",
            "fn package_operations(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
}
