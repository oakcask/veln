use super::*;
use veln_project::PackageSnapshotSource;

#[test]
fn references_include_standard_library_schema_uses_with_project_scope() {
    let workspace = TempWorkspace::new("references-standard-library-schema-operation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use wire from \"std\"\n\n",
            "// 🙂\n",
            "schema Host\n",
            "  count: UInt8\n",
            "  nested🙂: wire::Packet\n",
            "  repeated: Repeat(count, wire::Packet)\n",
            "  array: [wire::Packet; count]\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode wire::Packet from view at byte_offset(0)?\n",
            "  encode wire::Packet from packet\n",
            "end\n",
            "\n",
            "fn noise() -> String\n",
            "  // 🙂 wire::Packet\n",
            "  \"wire::Packet\"\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"wire.veln\"]\n",
        [PackageSnapshotSource::new(
            "wire.veln",
            b"pub schema Packet\n  value: Int\nend\n\nfn package_internal(view: ByteView) -> ()\n  decode Packet from view at byte_offset(0)?\nend\n",
        )],
    );

    for (line, column) in [(6, 18), (7, 33), (8, 17), (12, 16), (13, 16)] {
        let result =
            server.references_tool(&json!({"source":"main.veln","line":line,"column":column}));

        assert_eq!(result["isError"], false, "{result:#}");
        assert_reference_ranges(
            &result,
            &[
                ("main.veln", 6, 18, 6, 24),
                ("main.veln", 7, 33, 7, 39),
                ("main.veln", 8, 17, 8, 23),
                ("main.veln", 12, 16, 12, 22),
                ("main.veln", 13, 16, 13, 22),
            ],
            "standard library schema",
        );
        assert!(
            !result.to_string().contains("veln-pkg:///std/snapshot/"),
            "{result:#}"
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
    }

    let with_declaration = server.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 18,
        "include_declaration": true
    }));
    assert_standard_library_schema_declaration(&with_declaration);
}

fn assert_standard_library_schema_declaration(result: &Value) {
    assert_package_declaration(
        result,
        "/wire.veln",
        1,
        12,
        1,
        18,
        "standard-library schema declaration",
    );
    let locations = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(locations.len(), 6, "{result:#}");
    assert_eq!(
        locations[..5]
            .iter()
            .map(|location| location["range"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!({"start":{"line":6,"column":18},"end":{"line":6,"column":24}}),
            json!({"start":{"line":7,"column":33},"end":{"line":7,"column":39}}),
            json!({"start":{"line":8,"column":17},"end":{"line":8,"column":23}}),
            json!({"start":{"line":12,"column":16},"end":{"line":12,"column":22}}),
            json!({"start":{"line":13,"column":16},"end":{"line":13,"column":22}}),
        ]
    );
    assert!(
        locations[..5]
            .iter()
            .all(|location| location["uri"].as_str().unwrap().starts_with("file://"))
    );
}

#[test]
fn references_include_unique_implicit_nested_standard_library_module_path() {
    let workspace = TempWorkspace::new("references-standard-library-schema-implicit-nested");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use alpha::wire from \"std\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode alpha::wire::Packet from view at byte_offset(0)?\n",
            "  decode wire::Packet from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"alpha/wire.veln\"]\n",
        [PackageSnapshotSource::new(
            "alpha/wire.veln",
            b"pub schema Packet\n  value: Int\nend\n",
        )],
    );

    for (line, column) in [(4, 23), (5, 16)] {
        let result = server.references_tool(&json!({
            "source": "main.veln",
            "line": line,
            "column": column,
        }));
        assert_eq!(result["isError"], false, "{result:#}");
        assert_reference_ranges(
            &result,
            &[("main.veln", 4, 23, 4, 29), ("main.veln", 5, 16, 5, 22)],
            "unique implicit nested standard-library schema",
        );
        assert!(!result.to_string().contains("veln-pkg:///std/snapshot/"));
    }
}

#[test]
fn references_keep_standard_library_schema_recovery_collisions_empty_for_all_roles() {
    let workspace = TempWorkspace::new("references-standard-library-schema-recovery-collision");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use wire from \"std\"\n\n",
            "schema Host\n",
            "  nested: wire::Packet\n",
            "  repeated: Repeat(count, wire::Packet)\n",
            "  array: [wire::Packet; count]\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode wire::Packet from view at byte_offset(0)?\n",
            "  encode wire::Packet from packet\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"wire.veln\", \"recovered.veln\"]\n",
        [
            PackageSnapshotSource::new(
                "wire.veln",
                b"mod wire\npub schema Packet\n  value: Int\nend\n",
            ),
            PackageSnapshotSource::new(
                "recovered.veln",
                b"mod wire\npub schema Packet\n  value: Int\n",
            ),
        ],
    );

    for (line, column) in [(4, 18), (5, 33), (6, 17), (10, 16), (11, 16)] {
        let result = server.references_tool(&json!({
            "source": "main.veln",
            "line": line,
            "column": column
        }));
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{result:#}"
        );
    }
}

#[test]
fn references_paginate_standard_library_schema_uses_without_changing_scope() {
    let workspace = TempWorkspace::new("references-standard-library-schema-pagination");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use wire from \"std\"\n\n",
            "schema Host\n",
            "  nested: wire::Packet\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode wire::Packet from view at byte_offset(0)?\n",
            "  encode wire::Packet from packet\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"wire.veln\"]\n",
        [PackageSnapshotSource::new(
            "wire.veln",
            b"pub schema Packet\n  value: Int\nend\n",
        )],
    );

    let complete = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 19,
        "page_size": 100
    }));
    let first = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 19,
        "page_size": 1
    }));
    let paged =
        collect_reference_pages(&mut server, &first, &complete["structuredContent"]["scope"]);
    assert_eq!(json!(paged), complete["structuredContent"]["references"]);
}

fn collect_reference_pages(
    server: &mut Server,
    first: &Value,
    expected_scope: &Value,
) -> Vec<Value> {
    assert_reference_page(first, expected_scope);
    let mut references = reference_locations(first);
    let mut cursor = next_reference_cursor(first).expect("standard schema result should paginate");
    loop {
        let page = server.references_tool(&json!({"cursor": cursor}));
        assert_reference_page(&page, expected_scope);
        references.extend(reference_locations(&page));
        let Some(next) = next_reference_cursor(&page) else {
            break;
        };
        cursor = next;
    }
    references
}

fn assert_reference_page(page: &Value, expected_scope: &Value) {
    assert_eq!(page["isError"], false, "{page:#}");
    assert_eq!(page["structuredContent"]["scope"], *expected_scope);
}

fn reference_locations(page: &Value) -> Vec<Value> {
    page["structuredContent"]["references"]
        .as_array()
        .expect("references should be an array")
        .clone()
}

fn next_reference_cursor(page: &Value) -> Option<String> {
    page["structuredContent"].get("next_cursor").map(|cursor| {
        cursor
            .as_str()
            .expect("continuation cursor should be a string")
            .to_owned()
    })
}

#[test]
fn references_reject_standard_library_schema_import_collisions_in_both_orders() {
    let cases = [
        (
            "duplicate",
            "use alpha::wire from \"std\"\nuse alpha::wire from \"std\"\n\n",
            "wire::Packet",
        ),
        (
            "conflicting",
            "use alpha::wire from \"std\"\nuse beta::wire from \"std\"\n\n",
            "wire::Packet",
        ),
        (
            "recovered",
            "use alpha::wire from \"std\" broken\n\n",
            "wire::Packet",
        ),
    ];

    for (name, imports, selected) in cases {
        for reverse in [false, true] {
            let workspace = TempWorkspace::new(&format!(
                "references-standard-library-schema-{name}-{}",
                if reverse { "reverse" } else { "forward" }
            ));
            workspace.write("veln.toml", "");
            let imports = if reverse && name == "conflicting" {
                "use beta::wire from \"std\"\nuse alpha::wire from \"std\"\n\n"
            } else {
                imports
            };
            let source = format!(
                "{imports}fn read(view: ByteView) -> ()\n  decode {selected} from view at byte_offset(0)?\nend\n"
            );
            workspace.write("main.veln", &source);
            let mut server = initialized_server(&workspace);
            server.language_resources.replace_test_standard_library(
                "[package]\nname = \"std\"\n\n[lib]\nexports = [\"alpha/wire.veln\", \"beta/wire.veln\"]\n",
                [
                    PackageSnapshotSource::new(
                        "alpha/wire.veln",
                        b"pub schema Packet\n  value: Int\nend\n",
                    ),
                    PackageSnapshotSource::new(
                        "beta/wire.veln",
                        b"pub schema Packet\n  value: Int\nend\n",
                    ),
                ],
            );

            let result = server.references_tool(&json!({
                "source": "main.veln",
                "line": if name == "recovered" { 4 } else { 5 },
                "column": 16
            }));
            assert_eq!(result["isError"], false, "{name} {reverse}: {result:#}");
            assert_eq!(
                result["structuredContent"]["references"],
                json!([]),
                "{name} {reverse}: {result:#}"
            );
        }
    }
}

#[test]
fn references_keep_standard_library_schema_uses_inside_selected_project() {
    let workspace = TempWorkspace::new("references-standard-library-schema-project-scope");
    for project in ["app_a", "app_b", "app_a/nested"] {
        workspace.write(&format!("{project}/veln.toml"), "");
        workspace.write(
            &format!("{project}/main.veln"),
            "use wire from \"std\"\n\nschema Host\n  nested: wire::Packet\nend\n",
        );
    }
    workspace.write(
        "app_a/worker.veln",
        "use wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"wire.veln\"]\n",
        [PackageSnapshotSource::new(
            "wire.veln",
            b"pub schema Packet\n  value: Int\nend\n",
        )],
    );

    let result = server.references_tool(&json!({
        "project": "app_a",
        "source": "app_a/main.veln",
        "line": 4,
        "column": 18
    }));
    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("app_a/main.veln", 4, 17, 4, 23),
            ("app_a/worker.veln", 4, 16, 4, 22),
        ],
        "standard library selected project scope",
    );
    assert_eq!(result["structuredContent"]["scope"]["project"], "app_a");
}
