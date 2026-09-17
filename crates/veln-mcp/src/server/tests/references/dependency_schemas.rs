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
            "end\n\n",
            "fn alias_target_boundaries(view: ByteView) -> ()\n",
            "  decode public::InvalidTargetAlias from view at byte_offset(0)?\n",
            "  decode public::OtherPackageAlias from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"public.veln\", \"private.veln\", \"core.veln\"]\n\n",
            "[dependencies.\"other/dep\"]\npath = \"../other\"\n",
        ),
    );
    workspace.write(
        "vendor/dep/public.veln",
        concat!(
            "use core\n",
            "use other from \"other/dep\"\n\n",
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
            "pub schema InvalidTargetAlias = badTarget\n\n",
            "pub schema OtherPackageAlias = other::Public\n\n",
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
        ("invalid-casing alias target", "main.veln", 30, 18),
        ("other-package alias target", "main.veln", 31, 18),
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

    let cross_module_alias = references_result(&workspace, "main.veln", 22, 18);
    assert_eq!(
        cross_module_alias["isError"], false,
        "{cross_module_alias:#}"
    );
    assert_reference_ranges(
        &cross_module_alias,
        &[("main.veln", 22, 18, 22, 34)],
        "cross-module direct dependency schema alias operation",
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
fn references_prefer_exact_dependency_schema_alias_imports() {
    let workspace = TempWorkspace::new("references-exact-dependency-schema-alias-import");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n\n",
            "[dependencies.\"other/dep\"]\npath = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::wire from \"example/dep\"\n",
            "use other::lib from \"other/dep\"\n\n",
            "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode lib::wire::Alias from view at byte_offset(0)?\n",
            "  encode wire::Alias from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"lib/wire.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/lib/wire.veln",
        concat!(
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
        ),
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"other/lib.veln\"]\n",
    );
    workspace.write(
        "vendor/other/other/lib.veln",
        "pub schema Other\n  value: Int\nend\n",
    );

    let result = references_result(&workspace, "main.veln", 5, 22);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("main.veln", 5, 21, 5, 26), ("main.veln", 6, 16, 6, 21)],
        "exact dependency schema alias import precedence",
    );
}

#[test]
fn references_resolve_dependency_schema_alias_targets_across_same_module_sources() {
    let workspace = TempWorkspace::new("references-schema-alias-same-module-sources");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::wire from \"example/dep\"\n\n",
            "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode wire::WirePacket from view at byte_offset(0)?\n",
            "  encode wire::WirePacket from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"lib/packet.veln\", \"lib/alias.veln\"]\n",
        ),
    );
    workspace.write(
        "vendor/dep/lib/packet.veln",
        "mod lib::wire\n\npub schema Packet\n  value: Int\nend\n",
    );
    workspace.write(
        "vendor/dep/lib/alias.veln",
        "mod lib::wire\n\npub schema WirePacket = Packet\n",
    );

    for (line, column) in [(4, 16), (5, 16)] {
        let result = references_result(&workspace, "main.veln", line, column);
        assert_eq!(result["isError"], false, "{result:#}");
        assert_reference_ranges(
            &result,
            &[("main.veln", 4, 16, 4, 26), ("main.veln", 5, 16, 5, 26)],
            "same-module cross-source dependency schema alias operations",
        );
    }
}

#[test]
fn references_reject_exact_workspace_and_dependency_schema_alias_import_collisions() {
    let workspace = TempWorkspace::new("references-exact-schema-alias-import-collision");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use workspace::wire\n",
            "use workspace::wire from \"example/dep\"\n\n",
            "fn operations(view: ByteView) -> ()\n",
            "  decode workspace::wire::Alias from view at byte_offset(0)?\n",
            "  decode workspace::wire::Fallback from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "workspace/wire.veln",
        concat!(
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
            "pub schema Fallback\n  value: Int\nend\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"workspace/wire.veln\"]\n",
        ),
    );
    workspace.write(
        "vendor/dep/workspace/wire.veln",
        concat!(
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
            "pub schema Fallback = Packet\n",
        ),
    );

    for (line, column) in [(5, 28), (6, 28)] {
        let result = references_result(&workspace, "main.veln", line, column);
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

#[test]
fn references_keep_non_exported_schema_alias_fallback_empty() {
    let workspace = TempWorkspace::new("references-hidden-schema-alias-fallback");
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
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"exported.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/exported.veln",
        "mod dep\n\npub schema Alias\n  value: Int\nend\n",
    );
    workspace.write(
        "vendor/dep/hidden.veln",
        concat!(
            "mod dep\n\n",
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
        ),
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
fn references_keep_cross_source_alias_and_schema_import_collision_empty() {
    let workspace = TempWorkspace::new("references-cross-source-alias-schema-collision");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/alias\"]\npath = \"vendor/alias\"\n\n",
            "[dependencies.\"example/schema\"]\npath = \"vendor/schema\"\n",
        ),
    );
    workspace.write(
        "alias_import.veln",
        "mod app\n\nuse a::wire from \"example/alias\"\n",
    );
    workspace.write(
        "operation.veln",
        concat!(
            "mod app\n\n",
            "use b::wire from \"example/schema\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode wire::Alias from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/alias/veln.toml",
        "[package]\nname = \"example/alias\"\n\n[lib]\nexports = [\"a/wire.veln\"]\n",
    );
    workspace.write(
        "vendor/alias/a/wire.veln",
        "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
    );
    workspace.write(
        "vendor/schema/veln.toml",
        "[package]\nname = \"example/schema\"\n\n[lib]\nexports = [\"b/wire.veln\"]\n",
    );
    workspace.write(
        "vendor/schema/b/wire.veln",
        "pub schema Alias\n  value: Int\nend\n",
    );

    let result = references_result(&workspace, "operation.veln", 6, 16);

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
fn references_keep_dependency_schema_alias_collision_blockers_empty() {
    for (name, exports, blocker_source) in [
        (
            "recovered-target",
            "[\"valid.veln\", \"blocker.veln\"]",
            "mod dep\n\npub schema Packet\n  recovered: Int\n",
        ),
        (
            "recovered-alias-name-schema",
            "[\"valid.veln\", \"blocker.veln\"]",
            "mod dep\n\npub schema Alias\n  recovered: Int\n",
        ),
        (
            "hidden-duplicate-alias",
            "[\"valid.veln\"]",
            "mod dep\n\npub schema Alias = Packet\n",
        ),
        (
            "hidden-target-name-alias",
            "[\"valid.veln\"]",
            concat!(
                "mod dep\n\n",
                "pub schema Other\n  value: Int\nend\n\n",
                "pub schema Packet = Other\n",
            ),
        ),
    ] {
        let workspace = TempWorkspace::new(&format!(
            "references-dependency-schema-alias-{name}-blocker"
        ));
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
            &format!("[package]\nname = \"example/dep\"\n\n[lib]\nexports = {exports}\n"),
        );
        workspace.write(
            "vendor/dep/valid.veln",
            concat!(
                "mod dep\n\n",
                "pub schema Packet\n  value: Int\nend\n\n",
                "pub schema Alias = Packet\n",
            ),
        );
        workspace.write("vendor/dep/blocker.veln", blocker_source);

        let result = references_result(&workspace, "main.veln", 4, 16);

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
                "[lib]\nexports = [\"alias.veln\", \"schema.veln\"]\n",
            ),
        );
        workspace.write(
            "vendor/dep/alias.veln",
            concat!(
                "mod dep\n\n",
                "pub schema Packet\n  value: Int\nend\n\n",
                "pub schema Alias = Packet\n",
            ),
        );
        workspace.write(
            "vendor/dep/schema.veln",
            "mod dep\n\npub schema Alias\n  value: Int\nend\n",
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
fn references_reject_dependency_schema_alias_target_package_source_selection() {
    let workspace = TempWorkspace::new("references-dependency-schema-alias-package-source");
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
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let admitted = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 16
    }));
    assert_eq!(admitted["isError"], false, "{admitted:#}");
    let resources = all_resource_state(&mut server);
    let package_source_uri = resources
        .as_array()
        .unwrap()
        .iter()
        .find_map(|resource| {
            resource["uri"].as_str().filter(|uri| {
                uri.contains("/example%2Fdep/snapshot/") && uri.ends_with("/dep.veln")
            })
        })
        .expect("dependency package source should be retained");

    let result = server.references_tool(&json!({
        "source": package_source_uri,
        "line": 5,
        "column": 20
    }));

    assert_eq!(result["isError"], true, "{result:#}");
    assert_eq!(result["structuredContent"]["code"], "invalid_path");
    let structured = result["structuredContent"].as_object().unwrap();
    assert!(!structured.contains_key("references"), "{result:#}");
    assert!(!structured.contains_key("scope"), "{result:#}");
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

        let schema = references_result(&workspace, "main.veln", 4, 16);

        assert_eq!(
            schema["isError"],
            false,
            "{}: {schema:#}",
            source_kind.name()
        );
        assert_reference_ranges(
            &schema,
            &[("main.veln", 4, 15, 4, 21), ("main.veln", 5, 15, 5, 21)],
            source_kind.name(),
        );

        let alias = references_result(&workspace, "main.veln", 6, 16);
        assert_eq!(alias["isError"], false, "{}: {alias:#}", source_kind.name());
        assert_reference_ranges(
            &alias,
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
            "  decode Alias from view at byte_offset(0)?\n",
            "  encode Alias from packet\n",
            "end\n",
        ),
    );
}
