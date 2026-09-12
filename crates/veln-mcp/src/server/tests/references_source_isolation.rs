use super::references::*;
use super::*;

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
        &[("loose.veln", 6, 10, 6, 16), ("loose.veln", 7, 10, 7, 16)],
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
