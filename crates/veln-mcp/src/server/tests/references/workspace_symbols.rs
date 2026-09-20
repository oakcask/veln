use super::*;

#[test]
fn references_return_sorted_project_function_locations_and_scope() {
    let workspace = TempWorkspace::new("references-project");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Token\n",
            "  Helper\n",
            "end\n\n",
            "fn helper(value: Int) -> Int\n",
            "  helper(value - 1)\n",
            "end\n\n",
            "fn main() -> Int\n",
            "  helper(1)\n",
            "end\n\n",
            "fn unrelated() -> Token\n",
            "  Helper()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 10, 4);
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
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 2, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("main.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 6, "column": 3}, "end": {"line": 6, "column": 9}})
    );
    assert_eq!(
        references[1]["range"],
        json!({"start": {"line": 10, "column": 3}, "end": {"line": 10, "column": 9}})
    );

    let constructor = references_result(&workspace, "main.veln", 14, 4);
    assert_eq!(constructor["isError"], false, "{constructor:#}");
    assert_reference_ranges(
        &constructor,
        &[("main.veln", 14, 3, 14, 9)],
        "constructor support",
    );

    let with_declaration = initialized_server(&workspace).references_tool(&json!({
        "source": "main.veln",
        "line": 10,
        "column": 4,
        "include_declaration": true
    }));
    assert_reference_ranges(
        &with_declaration,
        &[
            ("main.veln", 5, 4, 5, 10),
            ("main.veln", 6, 3, 6, 9),
            ("main.veln", 10, 3, 10, 9),
        ],
        "workspace function declaration inclusion",
    );

    let omitted = references_result(&workspace, "main.veln", 10, 4);
    let explicit_false = initialized_server(&workspace).references_tool(&json!({
        "source": "main.veln",
        "line": 10,
        "column": 4,
        "include_declaration": false
    }));
    assert_eq!(omitted, explicit_false);
}

#[test]
fn references_keep_ineligible_workspace_function_aliases_empty_with_declaration_inclusion() {
    let cases = [
        (
            "unresolved target",
            "pub fn missing_fn = missing\n",
            1usize,
            12usize,
        ),
        (
            "wrong-kind target",
            "pub type Target\nend\n\npub fn wrong_kind = Target\n",
            4,
            12,
        ),
        (
            "alias-chain target",
            "pub fn target = missing\npub fn chained = target\n",
            2,
            12,
        ),
        (
            "invalid-cased target",
            "fn Target() -> Int\n  0\nend\n\npub fn invalid = Target\n",
            5,
            12,
        ),
    ];

    for (name, source_text, line, column) in cases {
        let workspace = TempWorkspace::new(name);
        workspace.write("veln.toml", "");
        workspace.write("main.veln", source_text);

        for include_declaration in [None, Some(false), Some(true)] {
            let mut arguments = json!({
                "source": "main.veln",
                "line": line,
                "column": column
            });
            if let Some(include_declaration) = include_declaration {
                arguments["include_declaration"] = json!(include_declaration);
            }
            let result = initialized_server(&workspace).references_tool(&arguments);
            assert_eq!(result["isError"], false, "{name}: {result:#}");
            assert_eq!(
                result["structuredContent"]["references"],
                json!([]),
                "{name}, include_declaration={include_declaration:?}: {result:#}"
            );
        }
    }
}

#[test]
fn references_resolve_types_and_constructors() {
    let cases = [
        WorkspaceSymbolCase {
            name: "type selected at declaration",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "  None\n",
                        "end\n\n",
                        "fn make() -> Item\n",
                        "  Item::Some(1)\n",
                        "end\n\n",
                        "fn observe(input: Item) -> Int\n",
                        "  match input\n",
                        "    Item::None => 0\n",
                        "    Item::Some(value) => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 6,
            ranges: vec![
                ("main.veln", 6, 14, 6, 18),
                ("main.veln", 7, 3, 7, 7),
                ("main.veln", 10, 19, 10, 23),
                ("main.veln", 12, 5, 12, 9),
                ("main.veln", 13, 5, 13, 9),
            ],
        },
        WorkspaceSymbolCase {
            name: "type selected at bare use",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  Some(Int)\n",
                        "end\n\n",
                        "type Other\n",
                        "end\n\n",
                        "fn read(input: Item) -> Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 8,
            column: 19,
            ranges: vec![("main.veln", 8, 16, 8, 20), ("main.veln", 8, 25, 8, 29)],
        },
        WorkspaceSymbolCase {
            name: "type selected at qualified use with collisions",
            files: vec![
                ("veln.toml", ""),
                (
                    "helper.veln",
                    "pub type Item\n  pub Ready(Int)\nend\n\npub type Other\nend\n",
                ),
                (
                    "main.veln",
                    concat!(
                        "use helper\n\n",
                        "type Item\n",
                        "end\n\n",
                        "fn make() -> helper::Item\n",
                        "  helper::Item::Ready(1)\n",
                        "end\n\n",
                        "fn read(input: helper::Item) -> helper::Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    "pub type Item\nend\n\nfn read(input: Item) -> Item\n  input\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 22,
            ranges: vec![
                ("main.veln", 6, 22, 6, 26),
                ("main.veln", 7, 11, 7, 15),
                ("main.veln", 10, 24, 10, 28),
                ("main.veln", 10, 41, 10, 45),
            ],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at declaration",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at qualified call with collisions",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Left\n",
                        "  Thing(Int)\n",
                        "end\n\n",
                        "type Right\n",
                        "  Thing(Int)\n",
                        "end\n\n",
                        "effect Task\n",
                        "  Thing() -> Int\n",
                        "end\n\n",
                        "fn make() -> Left\n",
                        "  Left::Thing(1)\n",
                        "end\n\n",
                        "fn read(input: Left) -> Int\n",
                        "  match input\n",
                        "    Left::Thing(value) => value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 14,
            column: 9,
            ranges: vec![("main.veln", 14, 9, 14, 14), ("main.veln", 19, 11, 19, 16)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at nullary expression",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 4,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected at bare pattern",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Status\n",
                        "  Ready\n",
                        "  Waiting\n",
                        "end\n\n",
                        "fn ready() -> Status\n",
                        "  Ready\n",
                        "end\n\n",
                        "fn observe(status: Status) -> Bool\n",
                        "  match status\n",
                        "    Ready => true\n",
                        "    Waiting => false\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 12,
            column: 7,
            ranges: vec![("main.veln", 7, 3, 7, 8), ("main.veln", 12, 5, 12, 10)],
        },
        WorkspaceSymbolCase {
            name: "constructor selected through imported alias",
            files: vec![
                ("veln.toml", ""),
                ("app/math.veln", "pub type Result\n  pub Done\nend\n"),
                (
                    "main.veln",
                    concat!(
                        "use app::math\n\n",
                        "fn make() -> math::Result\n",
                        "  math::Result::Done\n",
                        "end\n\n",
                        "fn read(input: math::Result) -> Bool\n",
                        "  match input\n",
                        "    math::Result::Done => true\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 17,
            ranges: vec![("main.veln", 4, 17, 4, 21), ("main.veln", 9, 19, 9, 23)],
        },
    ];

    assert_workspace_symbol_cases(cases);
}
