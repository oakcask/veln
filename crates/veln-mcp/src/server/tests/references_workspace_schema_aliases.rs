use super::references_support::{
    all_resource_state, assert_package_declaration, assert_reference_ranges,
    assert_snapshot_changed_without_references_or_scope, dependency_resource_is_listed,
    references_result, write_workspace_with_dependency_and_sources,
};
use super::*;
use std::cell::Cell;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

#[test]
fn references_keep_workspace_schema_aliases_inside_selected_project() {
    let workspace = TempWorkspace::new("references-schema-alias-project-isolation");
    for project in ["app_a", "app_b", "app_a/nested"] {
        workspace.write(&format!("{project}/veln.toml"), "");
        workspace.write(
            &format!("{project}/main.veln"),
            concat!(
                "pub schema Packet\n",
                "  value: Int\n",
                "end\n\n",
                "pub schema AliasPacket = Packet\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode AliasPacket from view at byte_offset(0)?\n",
                "end\n",
            ),
        );
    }
    workspace.write(
        "app_a/worker.veln",
        concat!(
            "use main\n\n",
            "fn read_owned(view: ByteView) -> ()\n",
            "  decode main::AliasPacket from view at byte_offset(0)?\n",
            "end\n",
        ),
    );

    for source in ["app_a/main.veln", "app_a/./main.veln"] {
        let result = references_result(&workspace, source, 5, 12);
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(
            result["structuredContent"]["scope"],
            json!({
                "mode": "project",
                "generation": 0,
                "project": "app_a",
                "project_wide": true
            })
        );
        assert_reference_ranges(
            &result,
            &[
                ("app_a/main.veln", 8, 10, 8, 21),
                ("app_a/worker.veln", 4, 16, 4, 27),
            ],
            "workspace schema alias project isolation",
        );
    }

    let with_declaration = initialized_server(&workspace).references_tool(&json!({
        "source": "app_a/main.veln",
        "line": 5,
        "column": 12,
        "include_declaration": true
    }));
    assert_eq!(with_declaration["isError"], false, "{with_declaration:#}");
    assert_reference_ranges(
        &with_declaration,
        &[
            ("app_a/main.veln", 5, 12, 5, 23),
            ("app_a/main.veln", 8, 10, 8, 21),
            ("app_a/worker.veln", 4, 16, 4, 27),
        ],
        "workspace schema alias declaration inclusion",
    );
}

#[test]
fn references_include_unused_eligible_workspace_schema_alias_declaration() {
    let workspace = TempWorkspace::new("references-unused-workspace-schema-alias");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
    );

    let result = initialized_server(&workspace).references_tool(&json!({
        "source": "main.veln",
        "line": 5,
        "column": 12,
        "include_declaration": true
    }));
    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("main.veln", 5, 12, 5, 23)],
        "unused eligible workspace schema alias declaration",
    );
}

#[test]
fn references_keep_anonymous_sources_isolated_for_workspace_schema_alias_selections() {
    let workspace = TempWorkspace::new("references-anonymous-schema-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        concat!(
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "  encode AliasPacket from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
    );
    workspace.write(
        "loose.veln",
        concat!(
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "  encode AliasPacket from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "loose.veln", 5, 12);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("loose.veln", 8, 10, 8, 21),
            ("loose.veln", 9, 10, 9, 21),
            ("loose.veln", 13, 11, 13, 22),
        ],
        "anonymous schema isolation",
    );
}

#[test]
fn references_keep_descendant_package_sources_isolated_for_workspace_schema_alias_selections() {
    let workspace = TempWorkspace::new("references-descendant-package-schema-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "  encode AliasPacket from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
    );
    workspace.write("nested/veln.toml", "");
    workspace.write(
        "nested/main.veln",
        concat!(
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "  encode AliasPacket from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "nested/main.veln", 5, 12);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "nested/main.veln",
            "project_wide": false
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("nested/main.veln", 8, 10, 8, 21),
            ("nested/main.veln", 9, 10, 9, 21),
            ("nested/main.veln", 13, 11, 13, 22),
        ],
        "descendant package schema isolation",
    );
}

#[test]
fn references_project_capture_exhausts_retries_for_workspace_schema_alias_selection() {
    let workspace = TempWorkspace::new("references-workspace-schema-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        concat!(
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
        None,
    );
    let (mut server, before_resources, before_selection) =
        admitted_server_with_captured_state(&workspace);
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let main = root.join("main.veln");
        fs::remove_file(&main).unwrap();
        let field = if attempt % 2 == 0 { "value" } else { "other" };
        fs::write(
            &main,
            format!(
                "pub schema Packet\n  {field}: Int\nend\n\npub schema AliasPacket = Packet\n\nfn read(view: ByteView) -> ()\n  decode AliasPacket from view at byte_offset(0)?\nend\n\nschema Frame\n  nested: AliasPacket\nend\n"
            ),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":5,"column":12}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_keep_ineligible_workspace_schema_aliases_empty_with_project_scope() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
    }

    let cases = [
        Case {
            name: "private schema alias target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 12,
        },
        Case {
            name: "missing schema alias target",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "pub schema AliasPacket = Missing\n"),
            ],
            source: "main.veln",
            line: 1,
            column: 12,
        },
        Case {
            name: "wrong-kind schema alias target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub type Packet\nend\n\npub schema AliasPacket = Packet\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "ambiguous schema alias target in first import order",
            files: vec![
                ("veln.toml", ""),
                ("a/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("b/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                (
                    "main.veln",
                    "use a::wire\nuse b::wire\n\npub schema AliasPacket = wire::Packet\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "ambiguous schema alias target in reverse import order",
            files: vec![
                ("veln.toml", ""),
                ("a/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("b/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                (
                    "main.veln",
                    "use b::wire\nuse a::wire\n\npub schema AliasPacket = wire::Packet\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "cyclic schema alias target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub schema FirstAlias = SecondAlias\npub schema SecondAlias = FirstAlias\n",
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 12,
        },
        Case {
            name: "schema alias-chain target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema FirstAlias = Packet\npub schema AliasPacket = FirstAlias\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 12,
        },
        Case {
            name: "invalid-casing schema alias",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema alias_packet = Packet\n\nfn read(view: ByteView) -> ()\n  decode alias_packet from view at byte_offset(0)?\nend\n",
                ),
            ],
            source: "main.veln",
            line: 8,
            column: 10,
        },
        Case {
            name: "recovery schema alias",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "pub schema AliasPacket =\n\nfn read(view: ByteView) -> ()\n  decode AliasPacket from view at byte_offset(0)?\nend\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 10,
        },
        Case {
            name: "bare imported workspace schema alias",
            files: vec![
                ("veln.toml", ""),
                (
                    "aliases.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                ),
                (
                    "main.veln",
                    "use aliases\n\nfn read(view: ByteView) -> ()\n  decode AliasPacket from view at byte_offset(0)?\nend\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 10,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        for include_declaration in [false, true] {
            let result = initialized_server(&workspace).references_tool(&json!({
                "source": case.source,
                "line": case.line,
                "column": case.column,
                "include_declaration": include_declaration
            }));
            assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
            assert_eq!(
                result["structuredContent"]["references"],
                json!([]),
                "{}, include_declaration={include_declaration}: {result:#}",
                case.name
            );
            assert_eq!(
                result["structuredContent"]["scope"],
                json!({
                    "mode": "project",
                    "generation": 0,
                    "project": ".",
                    "project_wide": true
                }),
                "{}, include_declaration={include_declaration}: {result:#}",
                case.name
            );
        }
    }
}

#[test]
fn references_keep_file_declaration_for_an_independently_selected_workspace_project() {
    let workspace = TempWorkspace::new("references-workspace-project-identity");
    workspace.write(
        "app/veln.toml",
        "[dependencies]\nlibrary = { path = \"../library\" }\n",
    );
    workspace.write("app/main.veln", "fn main() -> Int\n  0\nend\n");
    workspace.write("library/veln.toml", "");
    workspace.write(
        "library/main.veln",
        "pub schema Packet\n  value: Int\nend\n",
    );

    let result = initialized_server(&workspace).references_tool(&json!({
        "source": "library/main.veln",
        "line": 1,
        "column": 12,
        "include_declaration": true
    }));
    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("library/main.veln", 1, 12, 1, 18)],
        "independently selected workspace project declaration",
    );
    assert_eq!(result["structuredContent"]["scope"]["project"], "library");
}

#[test]
fn references_include_standard_library_schema_aliases_with_project_scope() {
    let workspace = TempWorkspace::new("references-standard-library-schema-alias");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use schemas from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode schemas::AliasPacket from view at byte_offset(0)?\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"schemas.veln\"]\n",
        [PackageSnapshotSource::new(
            "schemas.veln",
            b"pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":19}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("main.veln", 4, 19, 4, 30)],
        "standard-library schema alias",
    );
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );

    let with_declaration = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 19,
        "include_declaration": true
    }));
    assert_eq!(with_declaration["isError"], false, "{with_declaration:#}");
    assert_package_declaration(
        &with_declaration,
        "/schemas.veln",
        5,
        12,
        5,
        23,
        "standard-library schema alias declaration",
    );
}

#[test]
fn references_paginate_bare_standard_library_schema_aliases_stably() {
    let workspace = TempWorkspace::new("references-bare-standard-library-schema-alias-pagination");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "schema Host\n",
            "  nested: AliasPacket\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode AliasPacket from view at byte_offset(0)?\n",
            "  encode AliasPacket from packet\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
        )],
    );

    let first = server.references_tool(&json!({
        "source": "main.veln",
        "line": 2,
        "column": 12,
        "page_size": 2,
        "include_declaration": true
    }));
    assert_eq!(first["isError"], false, "{first:#}");
    assert_reference_ranges(
        &first,
        &[("main.veln", 2, 11, 2, 22), ("main.veln", 6, 10, 6, 21)],
        "bare standard-library schema alias first page",
    );
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .expect("non-final alias page cursor")
        .to_owned();

    workspace.write("main.veln", "fn changed() -> Int\n  1\nend\n");
    let second = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(second["isError"], false, "{second:#}");
    let second_references = second["structuredContent"]["references"]
        .as_array()
        .expect("continuation references");
    assert_eq!(second_references.len(), 2, "{second:#}");
    assert!(
        second_references[0]["uri"]
            .as_str()
            .unwrap()
            .starts_with("file://")
    );
    assert_eq!(
        second_references[0]["range"],
        json!({
            "start": {"line": 7, "column": 10},
            "end": {"line": 7, "column": 21}
        })
    );
    assert_package_declaration(
        &second,
        "/prelude.veln",
        5,
        12,
        5,
        23,
        "bare standard-library schema alias paginated declaration",
    );
    assert_eq!(
        second["structuredContent"]["scope"],
        first["structuredContent"]["scope"]
    );
    assert!(second["structuredContent"].get("next_cursor").is_none());
}

fn admitted_server_with_captured_state(workspace: &TempWorkspace) -> (Server, Value, Value) {
    let mut server = initialized_server(workspace);
    let admitted = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(admitted["isError"], false, "{admitted:#}");
    assert!(dependency_resource_is_listed(&mut server, "example/dep"));
    let resources = all_resource_state(&mut server);
    let selection = server.selection_result();
    (server, resources, selection)
}
