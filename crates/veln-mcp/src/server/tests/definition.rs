use super::*;

#[test]
fn definition_resolves_the_supported_workspace_symbol_set() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
        definition_file: &'static str,
        definition_line: usize,
        definition_column: usize,
    }

    let cases = [
        Case {
            name: "function",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "fn helper() -> Int\n  1\nend\n\nfn main() -> Int\n  helper()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            definition_file: "main.veln",
            definition_line: 1,
            definition_column: 4,
        },
        Case {
            name: "constructor",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "type Token\n  Byte(Int)\nend\n\nfn main() -> Token\n  Byte(1)\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            definition_file: "main.veln",
            definition_line: 2,
            definition_column: 3,
        },
        Case {
            name: "handler context parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Adjust\n  amount(value: Int) -> Int\nend\n\nhandler adjust(callback: fn(Int) -> Int) handles Adjust\n  amount(value) => callback(value)\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 22,
            definition_file: "main.veln",
            definition_line: 5,
            definition_column: 16,
        },
        Case {
            name: "handler operation clause parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Choose\n  pick(value: Bool) -> Int\nend\n\nhandler choose() handles Choose\n  pick(value) => value\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 18,
            definition_file: "main.veln",
            definition_line: 6,
            definition_column: 8,
        },
        Case {
            name: "exact companion private function",
            files: vec![
                ("veln.toml", ""),
                ("math.veln", "fn helper() -> Int\n  1\nend\n"),
                (
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::helper()\nend\n",
                ),
            ],
            source: "math.test.veln",
            line: 4,
            column: 11,
            definition_file: "math.veln",
            definition_line: 1,
            definition_column: 4,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = definition_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        let location = &result["structuredContent"]["definition"];
        assert!(
            location["uri"]
                .as_str()
                .unwrap()
                .ends_with(case.definition_file),
            "{}: {location:#}",
            case.name
        );
        assert_eq!(
            location["range"]["start"]["line"], case.definition_line,
            "{}",
            case.name
        );
        assert_eq!(
            location["range"]["start"]["column"], case.definition_column,
            "{}",
            case.name
        );
    }
}

#[test]
fn definition_infers_project_and_isolates_other_sources_and_descendant_manifests() {
    let workspace = TempWorkspace::new("definition-scope");
    workspace.write("app/veln.toml", "");
    workspace.write("app/math.veln", "pub type Token\n  pub Byte(Int)\nend\n");
    workspace.write(
        "app/main.veln",
        "use math\n\nfn main() -> Token\n  math::Byte(1)\nend\n",
    );
    workspace.write("loose.veln", "fn main() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  1\nend\n");
    workspace.write("app/nested/veln.toml", "");
    workspace.write(
        "app/nested/main.veln",
        "use math\n\nfn main() -> Token\n  math::Byte(1)\nend\n",
    );

    let project = definition_result(&workspace, "app/main.veln", 4, 10);
    assert!(
        project["structuredContent"]["definition"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("app/math.veln")),
        "{project:#}"
    );
    for source in ["loose.veln", "app/nested/main.veln"] {
        let isolated = definition_result(
            &workspace,
            source,
            if source == "loose.veln" { 2 } else { 4 },
            if source == "loose.veln" { 4 } else { 10 },
        );
        assert_eq!(isolated["isError"], false, "{source}: {isolated:#}");
        assert_eq!(
            isolated["structuredContent"]["definition"],
            Value::Null,
            "{source}"
        );
    }
}

#[test]
fn definition_uses_canonical_uri_and_range() {
    let workspace = TempWorkspace::new("definition coordinates and uri");
    write_definition_coordinate_fixture(&workspace);

    let found = definition_result(&workspace, "main.veln", 6, 9);
    let uri = found["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert!(uri.starts_with("file:///"), "{uri}");
    assert!(
        uri.contains("definition%20coordinates%20and%20uri"),
        "{uri}"
    );
    assert_eq!(
        found["structuredContent"]["definition"]["range"]["start"],
        json!({"line": 1, "column": 4})
    );
    assert_eq!(
        found["structuredContent"]["definition"]["range"]["end"],
        json!({"line": 1, "column": 10})
    );
}

#[test]
fn definition_returns_null_when_no_symbol_is_selected() {
    let workspace = TempWorkspace::new("definition-no-symbol");
    write_definition_coordinate_fixture(&workspace);

    for column in [8, 15] {
        let no_symbol = definition_result(&workspace, "main.veln", 6, column);
        assert_eq!(no_symbol["isError"], false, "{column}: {no_symbol:#}");
        assert_eq!(
            no_symbol["structuredContent"]["definition"],
            Value::Null,
            "{column}"
        );
    }
}

#[test]
fn definition_rejects_invalid_positions() {
    let workspace = TempWorkspace::new("definition-invalid-position");
    write_definition_coordinate_fixture(&workspace);

    for (line, column) in [(9, 1), (1, 20)] {
        let invalid = definition_result(&workspace, "main.veln", line, column);
        assert_eq!(invalid["isError"], true, "{line}:{column} {invalid:#}");
        assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    }
    assert_invalid_definition_position(
        &workspace,
        json!({
            "source": "main.veln",
            "line": u64::MAX,
            "column": 1
        }),
    );
    let above_u64_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":18446744073709551616,"column":1}"#)
            .unwrap();
    assert_invalid_definition_position(&workspace, above_u64_arguments);

    let huge_positive_exponent_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":1e9223372036854775807,"column":1}"#)
            .unwrap();
    assert_invalid_definition_position(&workspace, huge_positive_exponent_arguments);
}

#[test]
fn definition_accepts_integral_coordinate_spelling() {
    let workspace = TempWorkspace::new("definition-integral-spelling");
    write_definition_coordinate_fixture(&workspace);

    for arguments in [
        json!({"source": "main.veln", "line": 6.0, "column": 9}),
        json!({"source": "main.veln", "line": 6, "column": 9e0}),
    ] {
        let integral_spelling = initialized_server(&workspace).definition_tool(&arguments);
        assert_eq!(integral_spelling["isError"], false, "{integral_spelling:#}");
        assert_eq!(
            integral_spelling["structuredContent"]["definition"]["range"]["start"],
            json!({"line": 1, "column": 4})
        );
    }
}

#[test]
fn definition_rejects_non_integer_coordinate_spelling() {
    let workspace = TempWorkspace::new("definition-non-integer-spelling");
    write_definition_coordinate_fixture(&workspace);

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "definition",
                "arguments": {
                    "source": "main.veln",
                    "line": 6.0000000000000001,
                    "column": 9
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");

    let huge_negative_exponent_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "definition",
                "arguments": {
                    "source": "main.veln",
                    "line": 1e-9223372036854775808,
                    "column": 9
                }
            }
        }"#,
    )
    .unwrap();
    let huge_negative_exponent = initialized_server(&workspace)
        .handle_request(huge_negative_exponent_request)
        .unwrap();
    assert_eq!(
        huge_negative_exponent["error"]["code"], -32602,
        "{huge_negative_exponent:#}"
    );
}

fn write_definition_coordinate_fixture(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn helper() -> Int\r\n  1\r\nend\r\n\r\nfn main() -> Int\r\n  \"😀\" + helper()\r\nend\r\n");
}

fn assert_invalid_definition_position(workspace: &TempWorkspace, arguments: Value) {
    let result = initialized_server(workspace).definition_tool(&arguments);
    assert_eq!(result["isError"], true, "{result:#}");
    assert_eq!(result["structuredContent"]["code"], "invalid_position");
}

#[test]
fn definition_rejects_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("definition-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = definition_result(&workspace, source, 1, 1);
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{source}"
        );
    }

    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    let server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server.definition_tool(&json!({"source":"main.veln","line":2,"column":4}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("definition")
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn definition_rejects_symlink_paths_and_spells_uris_from_the_resolved_base() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("definition-resolved-base");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper() -> Int\n  1\nend\n\nfn main() -> Int\n  helper()\nend\n",
    );
    symlink(workspace.path("main.veln"), workspace.path("linked.veln")).unwrap();
    let rejected = definition_result(&workspace, "linked.veln", 1, 1);
    assert_eq!(rejected["structuredContent"]["code"], "invalid_path");

    let alias = workspace.root.with_file_name(format!(
        "{}-alias",
        workspace.root.file_name().unwrap().to_string_lossy()
    ));
    symlink(&workspace.root, &alias).unwrap();
    let base = WorkspaceBase::open(alias.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    let server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server.definition_tool(&json!({"source":"main.veln","line":6,"column":4}));
    let uri = result["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    let resolved_name = workspace.root.file_name().unwrap().to_string_lossy();
    assert!(uri.contains(resolved_name.as_ref()), "{uri}");
    assert!(!uri.contains("-alias/main.veln"), "{uri}");
    fs::remove_file(alias).unwrap();
}

fn definition_result(workspace: &TempWorkspace, source: &str, line: usize, column: usize) -> Value {
    initialized_server(workspace)
        .definition_tool(&json!({"source": source, "line": line, "column": column}))
}
