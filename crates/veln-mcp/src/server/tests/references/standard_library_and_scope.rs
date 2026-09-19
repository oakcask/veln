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
            "schema Host\n",
            "  count: UInt8\n",
            "  nested: wire::Packet\n",
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
            b"pub schema Packet\n  value: Int\nend\n",
        )],
    );

    for (line, column) in [(5, 17), (6, 33), (7, 17), (11, 16), (12, 16)] {
        let result =
            server.references_tool(&json!({"source":"main.veln","line":line,"column":column}));

        assert_eq!(result["isError"], false, "{result:#}");
        assert_reference_ranges(
            &result,
            &[
                ("main.veln", 5, 17, 5, 23),
                ("main.veln", 6, 33, 6, 39),
                ("main.veln", 7, 17, 7, 23),
                ("main.veln", 11, 16, 11, 22),
                ("main.veln", 12, 16, 12, 22),
            ],
            "standard library schema",
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
    assert_eq!(complete["isError"], false, "{complete:#}");
    assert_eq!(first["isError"], false, "{first:#}");
    assert_eq!(
        first["structuredContent"]["scope"],
        complete["structuredContent"]["scope"]
    );

    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .expect("standard schema result should paginate")
        .to_owned();
    let second = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(second["isError"], false, "{second:#}");
    let mut paged = first["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .clone();
    paged.extend(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .clone(),
    );
    if let Some(cursor) = second["structuredContent"].get("next_cursor") {
        let third = server.references_tool(&json!({"cursor": cursor}));
        assert_eq!(third["isError"], false, "{third:#}");
        paged.extend(
            third["structuredContent"]["references"]
                .as_array()
                .unwrap()
                .clone(),
        );
    }
    assert_eq!(json!(paged), complete["structuredContent"]["references"]);
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

#[test]
fn references_return_standard_library_function_locations() {
    let std_workspace = TempWorkspace::new("references-standard-library-boundary");
    std_workspace.write("veln.toml", "");
    std_workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n\n",
            "fn first(value: Int) -> Int\n",
            "  math::exported(value)\n",
            "end\n\n",
            "fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(value))\n",
            "end\n",
        ),
    );
    let mut std_server = initialized_server(&std_workspace);
    std_server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn exported(value: Int) -> Int\n  value\nend\n",
        )],
    );

    let std_definition =
        std_server.definition_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_definition["isError"], false, "{std_definition:#}");
    let std_uri = std_definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap_or_else(|| {
            panic!("standard library definition should resolve: {std_definition:#}")
        });
    assert!(std_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert!(std_uri.ends_with("/math.veln"));
    let std_references =
        std_server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_references["isError"], false, "{std_references:#}");
    assert_reference_ranges(
        &std_references,
        &[
            ("main.veln", 4, 9, 4, 17),
            ("main.veln", 8, 40, 8, 48),
            ("main.veln", 9, 18, 9, 26),
        ],
        "standard library function",
    );
}

#[test]
fn references_keep_standard_library_function_collision_boundaries() {
    let workspace = TempWorkspace::new("references-standard-library-collisions");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n",
            "use dep from \"example/dep\"\n\n",
            "# exported mention\n",
            "pub fn exported(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "pub fn first(record: {exported: Int}, value: Int) -> Int\n",
            "  math::exported(value)\n",
            "  dep::exported(value)\n",
            "  record.exported\n",
            "  \"exported\"\n",
            "  value\n",
            "end\n\n",
            "pub fn second(value: Int) -> Int\n",
            "  let exported = value\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(exported))\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn exported(value: Int) -> Int\n  value\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            concat!(
                "pub fn exported(value: Int) -> Int\n",
                "  exported(value - 1)\n",
                "end\n",
            )
            .as_bytes(),
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":10,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 10, 9, 10, 17),
            ("main.veln", 19, 40, 19, 48),
            ("main.veln", 20, 18, 20, 26),
        ],
        "standard library collision boundary",
    );

    let alias_segment = server.references_tool(&json!({"source":"main.veln","line":1,"column":5}));
    assert_eq!(alias_segment["isError"], false, "{alias_segment:#}");
    assert_eq!(
        alias_segment["structuredContent"]["references"],
        json!([]),
        "{alias_segment:#}"
    );
}

#[test]
fn references_return_standard_library_type_locations() {
    let workspace = TempWorkspace::new("references-standard-library-type");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(items: Vec<Int>) -> Vec<Int>\n",
            "  items\n",
            "end\n\n",
            "fn second(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n",
            "  prelude::Vec::Empty()\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third(items: Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Vec\n  pub Empty\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":17}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 1, 17, 1, 20),
            ("main.veln", 1, 30, 1, 33),
            ("main.veln", 5, 27, 5, 30),
            ("main.veln", 5, 49, 5, 52),
            ("main.veln", 6, 12, 6, 15),
            ("other.veln", 1, 17, 1, 20),
            ("other.veln", 1, 39, 1, 42),
        ],
        "standard library type",
    );
}

#[test]
fn references_return_standard_library_constructor_locations() {
    let workspace = TempWorkspace::new("references-standard-library-constructor");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(input: Option<Int>) -> Int\n",
            "  match input\n",
            "    Some(value) => value\n",
            "    None => 0\n",
            "  end\n",
            "end\n\n",
            "fn second() -> Option<Int>\n",
            "  prelude::Option::Some(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third() -> Option<Int>\n  prelude::Some(2)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Option\n  pub Some(Int)\n  pub None\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":6}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 3, 5, 3, 9),
            ("main.veln", 9, 20, 9, 24),
            ("other.veln", 2, 12, 2, 16),
        ],
        "standard library constructor",
    );
}

#[test]
fn references_keep_anonymous_sources_isolated_for_new_symbol_classes() {
    let workspace = TempWorkspace::new("references-anonymous-type-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        "type Item\nend\n\nfn selected(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "loose.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "other.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );

    let result = references_result(&workspace, "loose.veln", 4, 19);
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
        &[("loose.veln", 4, 18, 4, 22), ("loose.veln", 4, 27, 4, 31)],
        "anonymous type isolation",
    );
}

#[test]
fn references_keep_descendant_package_sources_isolated_for_new_symbol_classes() {
    let workspace = TempWorkspace::new("references-descendant-package-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "type Item\nend\n\nfn selected(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write("nested/veln.toml", "");
    workspace.write(
        "nested/main.veln",
        "type Item\nend\n\nfn helper(input: Item) -> Item\n  input\nend\n",
    );

    let result = references_result(&workspace, "nested/main.veln", 4, 19);
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
            ("nested/main.veln", 4, 18, 4, 22),
            ("nested/main.veln", 4, 27, 4, 31),
        ],
        "descendant package type isolation",
    );
}

#[test]
fn references_keep_anonymous_sources_isolated_for_workspace_schema_selections() {
    let workspace = TempWorkspace::new("references-anonymous-schema-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: Packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "loose.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: Packet\n",
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

    let result = references_result(&workspace, "loose.veln", 1, 8);
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
            ("loose.veln", 6, 10, 6, 16),
            ("loose.veln", 7, 10, 7, 16),
            ("loose.veln", 11, 11, 11, 17),
        ],
        "anonymous schema isolation",
    );
}

#[test]
fn references_keep_descendant_package_sources_isolated_for_workspace_schema_selections() {
    let workspace = TempWorkspace::new("references-descendant-package-schema-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn selected(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: Packet\n",
            "end\n",
        ),
    );
    workspace.write("nested/veln.toml", "");
    workspace.write(
        "nested/main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn helper(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n\n",
            "schema Frame\n",
            "  nested: Packet\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "nested/main.veln", 1, 8);
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
            ("nested/main.veln", 6, 10, 6, 16),
            ("nested/main.veln", 7, 10, 7, 16),
            ("nested/main.veln", 11, 11, 11, 17),
        ],
        "descendant package schema isolation",
    );
}

#[test]
fn references_exclude_declarations_for_new_workspace_symbol_classes() {
    struct Case {
        name: &'static str,
        line: usize,
        column: usize,
        ranges: Vec<(&'static str, usize, usize, usize, usize)>,
    }

    let workspace = TempWorkspace::new("references-declaration-exclusion");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some(Int)\n",
            "end\n\n",
            "fn read(input: Item) -> output: Int\n",
            "  ensure output >= input\n",
            "  let current = input\n",
            "  current\n",
            "end\n\n",
            "fn make(value: Int) -> Item\n",
            "  Some(value)\n",
            "end\n\n",
            "effect Run\n",
            "  call(action: fn() -> Int) -> Int\n",
            "end\n\n",
            "handler run(callback: fn(Int) -> Int) handles Run\n",
            "  call(action) => callback(action())\n",
            "end\n",
        ),
    );

    let cases = [
        Case {
            name: "type declaration",
            line: 1,
            column: 6,
            ranges: vec![("main.veln", 5, 16, 5, 20), ("main.veln", 11, 24, 11, 28)],
        },
        Case {
            name: "constructor use",
            line: 12,
            column: 4,
            ranges: vec![("main.veln", 12, 3, 12, 7)],
        },
        Case {
            name: "function parameter use",
            line: 6,
            column: 21,
            ranges: vec![("main.veln", 6, 20, 6, 25), ("main.veln", 7, 17, 7, 22)],
        },
        Case {
            name: "result binding use",
            line: 6,
            column: 11,
            ranges: vec![("main.veln", 6, 10, 6, 16)],
        },
        Case {
            name: "handler context parameter use",
            line: 19,
            column: 20,
            ranges: vec![("main.veln", 20, 19, 20, 27)],
        },
        Case {
            name: "handler operation clause parameter use",
            line: 20,
            column: 29,
            ranges: vec![("main.veln", 20, 28, 20, 34)],
        },
    ];

    for case in cases {
        let result = references_result(&workspace, "main.veln", case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_reference_ranges(&result, &case.ranges, case.name);
    }
}
