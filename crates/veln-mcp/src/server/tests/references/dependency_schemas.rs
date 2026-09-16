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
            "pub schema Packet\n  value: Int\nend\n",
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
        "end\n",
    );
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let dependency_source = "pub schema Packet\n  value: Int\nend\n";
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
    }
}

#[test]
fn references_exclude_unselected_descendant_sources_from_parent_project_scope() {
    let workspace = TempWorkspace::new("references-dependency-schema-descendant-exclusion");
    let operation = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn read(view: ByteView) -> ()\n",
        "  decode dep::Packet from view at byte_offset(0)?\n",
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
        "pub schema Packet\n  value: Int\nend\n",
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
}

#[test]
fn references_return_empty_for_dependency_schema_operation_boundaries() {
    let workspace = TempWorkspace::new("references-dependency-schema-boundaries");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use public from \"example/dep\"\n",
            "use private from \"example/dep\"\n",
            "use hidden from \"example/dep\"\n",
            "use mismatch from \"other/dep\"\n",
            "use transitive from \"transitive/dep\"\n",
            "use standard from \"std\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode private::Private from view at byte_offset(0)?\n",
            "  decode hidden::Hidden from view at byte_offset(0)?\n",
            "  decode mismatch::Public from view at byte_offset(0)?\n",
            "  decode transitive::Public from view at byte_offset(0)?\n",
            "  decode public::badSchema from view at byte_offset(0)?\n",
            "  decode public::Missing from view at byte_offset(0)?\n",
            "  decode public::Alias from view at byte_offset(0)?\n",
            "  decode standard::Standard from view at byte_offset(0)?\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: public::Public\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"public.veln\", \"private.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/public.veln",
        concat!(
            "pub schema Public\n  value: Int\nend\n\n",
            "pub schema badSchema\n  value: Int\nend\n\n",
            "pub schema Alias = Public\n",
        ),
    );
    workspace.write(
        "vendor/dep/private.veln",
        "schema Private\n  value: Int\nend\n",
    );
    workspace.write(
        "vendor/dep/hidden.veln",
        "pub schema Hidden\n  value: Int\nend\n",
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
        ("private", "main.veln", 9, 19),
        ("non-exported", "main.veln", 10, 18),
        ("mismatched import", "main.veln", 11, 20),
        ("transitive", "main.veln", 12, 22),
        ("invalid casing", "main.veln", 13, 18),
        ("unresolved", "main.veln", 14, 18),
        ("package alias", "main.veln", 15, 18),
        ("standard library", "main.veln", 16, 20),
        ("package composition", "main.veln", 20, 19),
        ("module qualifier", "main.veln", 13, 10),
        ("recovery", "recovery.veln", 4, 10),
    ] {
        let result = references_result(&workspace, source, line, column);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
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

        let result = references_result(&workspace, "main.veln", 4, 16);

        assert_eq!(
            result["isError"],
            false,
            "{}: {result:#}",
            source_kind.name()
        );
        assert_reference_ranges(
            &result,
            &[("main.veln", 4, 15, 4, 21), ("main.veln", 5, 15, 5, 21)],
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
        "pub schema Packet\n  value: Int\nend\n",
    );
}
