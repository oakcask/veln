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
}

#[test]
fn references_resolve_supported_workspace_symbol_classes() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
        ranges: Vec<(&'static str, usize, usize, usize, usize)>,
    }

    let cases = [
        Case {
            name: "type with imported qualified references",
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
        Case {
            name: "constructor with collisions",
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
        Case {
            name: "callable value binding",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "type Item\n",
                        "  byte(value: Int)\n",
                        "end\n\n",
                        "fn caller(byte: fn() -> Int) -> Int\n",
                        "  byte()\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            ranges: vec![("main.veln", 6, 3, 6, 7)],
        },
        Case {
            name: "handler context parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Adjust\n",
                        "  amount(value: Int) -> Int\n",
                        "  echo(value: Int) -> Int\n",
                        "end\n\n",
                        "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                        "  amount(value) => callback(value)\n",
                        "  echo(value) => callback(value)\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 22,
            ranges: vec![("main.veln", 7, 20, 7, 28), ("main.veln", 8, 18, 8, 26)],
        },
        Case {
            name: "handler operation clause parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "effect Choose\n",
                        "  pick(value: Bool) -> Int\n",
                        "end\n\n",
                        "handler choose() handles Choose\n",
                        "  pick(value) => match value\n",
                        "    true => value\n",
                        "    value => value\n",
                        "    false => record.value\n",
                        "  end\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 7,
            column: 16,
            ranges: vec![("main.veln", 6, 24, 6, 29), ("main.veln", 7, 13, 7, 18)],
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"]["project_wide"], true,
            "{}: {result:#}",
            case.name
        );
        assert_reference_ranges(&result, &case.ranges, case.name);
    }
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
fn references_reject_recovery_package_and_unsupported_symbols() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
    }

    let cases = [
        Case {
            name: "recovery value binding",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "fn main(Bad: Int) -> Int\n  Bad\nend\n"),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
        },
        Case {
            name: "package type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use example::dep\n\nfn read(input: dep::Item) -> dep::Item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type Item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "schema",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "schema Packet\n  format binary\n  value: UInt8\nend\n\nfn main() -> Int\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 1,
            column: 8,
        },
        Case {
            name: "effect operation",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task]\n  perform Task::run()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 17,
        },
        Case {
            name: "no symbol",
            files: vec![("main.veln", "fn main() -> Int\n  1\nend\n")],
            source: "main.veln",
            line: 2,
            column: 3,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}

#[test]
fn references_preserve_unicode_coordinates_and_token_end_exclusion() {
    let workspace = TempWorkspace::new("references-coordinate-boundaries");
    workspace.write(
        "main.veln",
        "fn main() -> Int\r\n  main()\r\n  let emoji = \"🙂\"\r\nend\r\n",
    );

    let selected = references_result(&workspace, "main.veln", 2, 4);
    assert_eq!(selected["isError"], false, "{selected:#}");
    assert_reference_ranges(
        &selected,
        &[("main.veln", 2, 3, 2, 7)],
        "unicode coordinate selection",
    );

    let token_end = references_result(&workspace, "main.veln", 2, 7);
    assert_eq!(token_end["isError"], false, "{token_end:#}");
    assert_eq!(token_end["structuredContent"]["references"], json!([]));

    let invalid = references_result(&workspace, "main.veln", 3, 21);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
}

#[test]
fn references_use_single_file_scope_for_sources_outside_selected_projects() {
    let workspace = TempWorkspace::new("references-single-file");
    workspace.write("app/veln.toml", "");
    workspace.write("app/main.veln", "fn selected() -> Int\n  selected()\nend\n");
    workspace.write("loose.veln", "fn helper() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  helper()\nend\n");

    let result = references_result(&workspace, "loose.veln", 2, 4);
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
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 1, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("loose.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 9}})
    );
}

#[test]
fn references_do_not_expose_function_shaped_recovery_records() {
    let workspace = TempWorkspace::new("references-recovery-boundary");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "test Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 6, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(result["structuredContent"]["scope"]["project_wide"], true);
}

#[test]
fn references_report_invalid_positions_and_schema_coordinate_failures() {
    let workspace = TempWorkspace::new("references-invalid-position");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");

    let invalid = references_result(&workspace, "main.veln", 5, 1);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    assert!(
        invalid["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "references",
                "arguments": {
                    "source": "main.veln",
                    "line": 2.0000000000000001,
                    "column": 4
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");
    assert!(non_integer.get("result").is_none(), "{non_integer:#}");
}

#[test]
fn references_reject_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("references-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = references_result(&workspace, source, 1, 1);
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{source}"
        );
    }

    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    let mut server = Server {
        base,
        selection,
        initialized: true,
        language_resources: LanguageResources::checked().unwrap(),
    };
    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );
}

fn references_result(workspace: &TempWorkspace, source: &str, line: usize, column: usize) -> Value {
    initialized_server(workspace)
        .references_tool(&json!({"source": source, "line": line, "column": column}))
}

fn assert_reference_ranges(
    result: &Value,
    expected: &[(&str, usize, usize, usize, usize)],
    name: &str,
) {
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: references must be an array: {result:#}"));
    assert_eq!(
        references.len(),
        expected.len(),
        "{name}: unexpected reference count: {result:#}"
    );
    for (reference, (file, start_line, start_column, end_line, end_column)) in
        references.iter().zip(expected)
    {
        assert!(
            reference["uri"].as_str().unwrap().ends_with(file),
            "{name}: {reference:#}"
        );
        assert_eq!(
            reference["range"],
            json!({
                "start": {"line": start_line, "column": start_column},
                "end": {"line": end_line, "column": end_column}
            }),
            "{name}: {reference:#}"
        );
    }
}
