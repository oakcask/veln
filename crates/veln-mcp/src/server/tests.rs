use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn lifecycle_lists_and_calls_only_implemented_tools() {
    let workspace = TempWorkspace::new("lifecycle");
    workspace.write("alpha/veln.toml", "");
    workspace.write("beta/deep/veln.toml", "");
    let responses = run_lifecycle_session(&workspace);

    assert_eq!(responses.len(), 4);
    assert_implemented_tool_names(&responses[1]);
    assert_eq!(
        responses[2]["result"]["structuredContent"],
        json!({
            "generation": 0,
            "roots": ["alpha", "beta/deep"]
        })
    );
    assert_eq!(responses[3]["result"]["structuredContent"]["generation"], 1);
}

fn run_lifecycle_session(workspace: &TempWorkspace) -> Vec<Value> {
    let input = lifecycle_messages()
        .into_iter()
        .map(|message| message.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut output = Vec::new();
    run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
    parse_responses(output)
}

fn lifecycle_messages() -> [Value; 5] {
    [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{"_meta":{"progressToken":"list"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"workspace_projects","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}),
    ]
}

fn parse_responses(output: Vec<u8>) -> Vec<Value> {
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect()
}

fn assert_implemented_tool_names(response: &Value) {
    let names = response["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "workspace_projects",
            "refresh_workspace",
            "check_project",
            "definition"
        ]
    );
}

#[test]
fn client_root_information_does_not_change_selection() {
    let workspace = TempWorkspace::new("client-roots");
    workspace.write("nested/veln.toml", "");
    let variants = [
        json!({"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}),
        json!({"protocolVersion":"2025-06-18","capabilities":{"roots":{"listChanged":true}},"clientInfo":{"name":"test","version":"1"},"rootUri":"file:///unrelated"}),
        json!({"protocolVersion":"2025-06-18","capabilities":{"roots":{"listChanged":false}},"clientInfo":{"name":"test","version":"1"},"roots":[{"uri":"file:///nested","name":"nested"}]}),
    ];

    for params in variants {
        let input = format!(
            "{}\n{}\n",
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":params}),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"workspace_projects","arguments":{}}})
        );
        let mut output = Vec::new();
        run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
        let response = String::from_utf8(output).unwrap();
        let selection = serde_json::from_str::<Value>(response.lines().nth(1).unwrap()).unwrap();
        assert_eq!(
            selection["result"]["structuredContent"]["roots"],
            json!(["nested"])
        );
    }
}

#[test]
fn invalid_tool_inputs_are_protocol_invalid_params() {
    let workspace = TempWorkspace::new("invalid-input");
    let mut server = initialized_server(&workspace);
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"workspace_projects","arguments":{"unknown":true}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_workspace","arguments":[]}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":null}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"check_project","arguments":{"project":null}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"check_project","arguments":{"source":null}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"check_project","arguments":{"unknown":true}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"definition","arguments":{"source":"main.veln","line":1,"column":1,"unknown":true}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"definition","arguments":{"source":null,"line":1,"column":1}}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"definition","arguments":{"source":"main.veln","line":0,"column":1}}}),
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"definition","arguments":{"source":"main.veln","line":1}}}),
    ];
    for request in requests {
        let response = server.handle_request(request).unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }
}

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
                    "type Token\n  byte(Int)\nend\n\nfn main() -> Token\n  byte(1)\nend\n",
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
    workspace.write("app/math.veln", "pub type Token\n  pub byte(Int)\nend\n");
    workspace.write(
        "app/main.veln",
        "use math\n\nfn main() -> Token\n  math::byte(1)\nend\n",
    );
    workspace.write("loose.veln", "fn main() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  1\nend\n");
    workspace.write("app/nested/veln.toml", "");
    workspace.write(
        "app/nested/main.veln",
        "use math\n\nfn main() -> Token\n  math::byte(1)\nend\n",
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
fn definition_distinguishes_no_symbol_from_invalid_positions_and_uses_canonical_uri() {
    let workspace = TempWorkspace::new("definition coordinates and uri");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn helper() -> Int\r\n  1\r\nend\r\n\r\nfn main() -> Int\r\n  \"😀\" + helper()\r\nend\r\n");

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

    for column in [8, 15] {
        let no_symbol = definition_result(&workspace, "main.veln", 6, column);
        assert_eq!(no_symbol["isError"], false, "{column}: {no_symbol:#}");
        assert_eq!(
            no_symbol["structuredContent"]["definition"],
            Value::Null,
            "{column}"
        );
    }
    for (line, column) in [(9, 1), (1, 20)] {
        let invalid = definition_result(&workspace, "main.veln", line, column);
        assert_eq!(invalid["isError"], true, "{line}:{column} {invalid:#}");
        assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    }
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
    let huge_position = initialized_server(&workspace).definition_tool(&json!({
        "source": "main.veln",
        "line": u64::MAX,
        "column": 1
    }));
    assert_eq!(huge_position["isError"], true, "{huge_position:#}");
    assert_eq!(
        huge_position["structuredContent"]["code"],
        "invalid_position"
    );
    let above_u64_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":18446744073709551616,"column":1}"#)
            .unwrap();
    let above_u64 = initialized_server(&workspace).definition_tool(&above_u64_arguments);
    assert_eq!(above_u64["isError"], true, "{above_u64:#}");
    assert_eq!(above_u64["structuredContent"]["code"], "invalid_position");

    let huge_positive_exponent_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":1e9223372036854775807,"column":1}"#)
            .unwrap();
    let huge_positive_exponent =
        initialized_server(&workspace).definition_tool(&huge_positive_exponent_arguments);
    assert_eq!(
        huge_positive_exponent["isError"], true,
        "{huge_positive_exponent:#}"
    );
    assert_eq!(
        huge_positive_exponent["structuredContent"]["code"],
        "invalid_position"
    );

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
    assert!(result["structuredContent"]["definition"].is_null());
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

#[test]
fn check_project_selection_table_reports_success_and_stable_domain_failures() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        arguments: Value,
        expect_error: Option<&'static str>,
        expect_mode: Option<&'static str>,
        expect_project: Option<&'static str>,
    }

    let cases = [
        Case {
            name: "explicit manifest project",
            files: vec![("veln.toml", ""), ("main.veln", clean_source())],
            arguments: json!({"project": "."}),
            expect_error: None,
            expect_mode: Some("project"),
            expect_project: Some("."),
        },
        Case {
            name: "inferred single manifest project",
            files: vec![("app/veln.toml", ""), ("app/main.veln", clean_source())],
            arguments: json!({}),
            expect_error: None,
            expect_mode: Some("project"),
            expect_project: Some("app"),
        },
        Case {
            name: "ambiguous manifest projects",
            files: vec![
                ("zeta/veln.toml", ""),
                ("zeta/main.veln", clean_source()),
                ("alpha/veln.toml", ""),
                ("alpha/main.veln", clean_source()),
            ],
            arguments: json!({}),
            expect_error: Some("project_ambiguous"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "anonymous single source",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"project": ".", "source": "main.veln"}),
            expect_error: None,
            expect_mode: Some("single_file"),
            expect_project: Some("."),
        },
        Case {
            name: "anonymous requires source",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"project": "."}),
            expect_error: Some("source_required"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "unselected project",
            files: vec![("app/veln.toml", ""), ("app/main.veln", clean_source())],
            arguments: json!({"project": "missing"}),
            expect_error: Some("project_not_selected"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "manifest source combination",
            files: vec![("veln.toml", ""), ("main.veln", clean_source())],
            arguments: json!({"project": ".", "source": "main.veln"}),
            expect_error: Some("invalid_query"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "anonymous without explicit project",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"source": "main.veln"}),
            expect_error: Some("source_required"),
            expect_mode: None,
            expect_project: None,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in &case.files {
            workspace.write(path, text);
        }
        let result = check_project_result(&workspace, case.arguments);
        if let Some(code) = case.expect_error {
            assert_eq!(result["isError"], true, "{}", case.name);
            assert_eq!(result["structuredContent"]["code"], code, "{}", case.name);
        } else {
            assert_eq!(result["isError"], false, "{}", case.name);
            assert_eq!(
                result["structuredContent"]["analysis"]["mode"],
                case.expect_mode.unwrap(),
                "{}",
                case.name
            );
            assert_eq!(
                result["structuredContent"]["analysis"]["project"],
                case.expect_project.unwrap(),
                "{}",
                case.name
            );
        }
    }
}

#[test]
fn check_project_does_not_reclassify_selection_before_refresh() {
    let workspace = TempWorkspace::new("selection-fixed-before-refresh");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    fs::remove_file(workspace.path("veln.toml")).unwrap();
    let mut server = Server {
        base,
        selection,
        initialized: true,
    };

    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "."}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn anonymous_check_project_ignores_manifest_added_before_refresh() {
    let workspace = TempWorkspace::new("anonymous-manifest-added-before-refresh");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    workspace.write(
        "veln.toml",
        "[lib]\nexports = [\"main.veln\", \"extra.veln\"]\n",
    );
    workspace.write("extra.veln", mismatch_source());
    let mut server = Server {
        base,
        selection,
        initialized: true,
    };

    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
    assert_eq!(
        result["structuredContent"]["analysis"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "main.veln",
            "project_wide": false
        })
    );
}

#[test]
fn anonymous_check_project_does_not_expand_companion_named_source() {
    let workspace = TempWorkspace::new("anonymous-companion-shaped-source");
    workspace.write("main.test.veln", "fn companion_entry() -> Int\n  1\nend\n");
    workspace.write("main.veln", mismatch_source());

    let result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "main.test.veln"}),
    );

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["span"]["file"] != "main.veln" && diagnostic["id"] != "type.mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[cfg(unix)]
#[test]
fn selected_project_root_symlink_replacement_reports_snapshot_changed() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("selected-root-symlink-replacement");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(workspace.path("alpha")).unwrap();
    workspace.write("outside/veln.toml", "");
    workspace.write("outside/main.veln", clean_source());
    symlink(workspace.path("outside"), workspace.path("alpha")).unwrap();

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "alpha"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn selected_project_root_directory_replacement_reports_snapshot_changed() {
    let workspace = TempWorkspace::new("selected-root-directory-replacement");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(workspace.path("alpha")).unwrap();
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "alpha"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[cfg(unix)]
#[test]
fn anonymous_workspace_base_symlink_replacement_reports_snapshot_changed() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("anonymous-base-symlink-replacement");
    let outside = TempWorkspace::new("anonymous-base-symlink-replacement-outside");
    workspace.write("main.veln", clean_source());
    outside.write("main.veln", mismatch_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(&workspace.root).unwrap();
    symlink(&outside.root, &workspace.root).unwrap();

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn anonymous_workspace_base_directory_replacement_reports_snapshot_changed() {
    let workspace = TempWorkspace::new("anonymous-base-directory-replacement");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", mismatch_source());

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn check_project_rejects_source_path_boundaries_before_analysis() {
    let workspace = TempWorkspace::new("source-boundaries");
    workspace.write("main.veln", clean_source());
    workspace.write("notes.txt", "not source");
    workspace.mkdir("directory.veln");

    let cases = [
        (
            "absolute",
            json!({"project": ".", "source": workspace.root.join("main.veln").to_string_lossy()}),
        ),
        (
            "escaping",
            json!({"project": ".", "source": "../main.veln"}),
        ),
        ("missing", json!({"project": ".", "source": "missing.veln"})),
        (
            "non regular",
            json!({"project": ".", "source": "directory.veln"}),
        ),
        ("non veln", json!({"project": ".", "source": "notes.txt"})),
    ];

    for (name, arguments) in cases {
        let result = check_project_result(&workspace, arguments);
        assert_eq!(result["isError"], true, "{name}");
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{name}"
        );
    }
}

#[cfg(unix)]
#[test]
fn check_project_rejects_symlink_traversing_sources() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("source-symlink");
    workspace.write("real/main.veln", clean_source());
    symlink(workspace.root.join("real"), workspace.root.join("linked")).unwrap();

    let result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "linked/main.veln"}),
    );

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "invalid_path");

    let parent_result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "linked/../real/main.veln"}),
    );
    assert_eq!(parent_result["isError"], true);
    assert_eq!(parent_result["structuredContent"]["code"], "invalid_path");
}

#[cfg(unix)]
#[test]
fn manifest_check_project_does_not_read_symlinked_project_sources() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-source-symlink");
    workspace.write("alpha/veln.toml", "");
    workspace.write("outside/bad.veln", mismatch_source());
    symlink(
        workspace.path("outside/bad.veln"),
        workspace.path("alpha/main.veln"),
    )
    .unwrap();

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[cfg(unix)]
#[test]
fn manifest_check_project_does_not_read_symlinked_project_directories() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-directory-symlink");
    workspace.write("alpha/veln.toml", "");
    workspace.write("outside/bad.veln", mismatch_source());
    symlink(workspace.path("outside"), workspace.path("alpha/linked")).unwrap();

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[cfg(target_os = "linux")]
#[test]
fn manifest_check_project_ignores_symlinked_nested_manifest_boundary() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-nested-symlink-boundary");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    workspace.mkdir("alpha/nested");
    symlink(
        workspace.path("alpha/veln.toml"),
        workspace.path("alpha/nested/veln.toml"),
    )
    .unwrap();
    workspace.write("alpha/nested/bad.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 1, "by_severity": {"error": 1}, "by_kind": {"type": 1}})
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn check_project_fails_closed_without_handle_relative_capture_support() {
    let workspace = TempWorkspace::new("no-handle-relative-capture-support");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", clean_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn manifest_check_project_stops_at_non_utf8_nested_manifest_boundary() {
    let workspace = TempWorkspace::new("manifest-non-utf8-nested-boundary");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    workspace.write_bytes("alpha/nested/veln.toml", b"not utf8: \xff");
    workspace.write("alpha/nested/bad.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[test]
fn anonymous_check_project_analyzes_only_the_selected_source() {
    let workspace = TempWorkspace::new("anonymous-isolation");
    workspace.write("clean.veln", clean_source());
    workspace.write("broken.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": ".", "source": "clean.veln"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
    assert_eq!(
        result["structuredContent"]["analysis"]["project_wide"],
        false
    );
}

#[test]
fn check_project_returns_structured_language_diagnostics_as_successful_tool_result() {
    let workspace = TempWorkspace::new("structured-diagnostics");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["id"] == "type.mismatch"
                && diagnostic["severity"] == "error"
                && diagnostic["span"]["start"]["line"] == 2
                && diagnostic["span"]["start"]["column"].as_u64().unwrap() >= 3
                && diagnostic.get("details").is_some()
                && diagnostic.get("related").is_some()
        }),
        "{diagnostics:#?}"
    );
    assert_eq!(
        result["structuredContent"]["summary"]["by_severity"]["error"],
        1
    );
}

#[test]
fn check_project_uses_captured_materialized_git_dependency() {
    let workspace = TempWorkspace::new("captured-materialized-git-dependency");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"https://example.invalid/foo.git\"\n",
            "rev = \"abc123\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use foo from \"github.com/oakcask/foo\"\n\n",
            "fn main() -> Int\n",
            "  add_one(1)\n",
            "end\n",
        ),
    );
    let materialized = veln_project::materialized_git_repository_root(
        &workspace.root,
        "https://example.invalid/foo.git",
    );
    let dependency_root = materialized
        .strip_prefix(&workspace.root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    workspace.write(
        &format!("{dependency_root}/veln.toml"),
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/foo\"\n\n",
            "[lib]\n",
            "exports = [\"foo.veln\"]\n",
        ),
    );
    workspace.write(
        &format!("{dependency_root}/foo.veln"),
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

fn check_project_result(workspace: &TempWorkspace, arguments: Value) -> Value {
    let mut server = initialized_server(workspace);
    server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": arguments}),
        ))
        .unwrap()
}

fn clean_source() -> &'static str {
    "fn main() -> Int\n  1\nend\n"
}

fn mismatch_source() -> &'static str {
    "fn main() -> Int\n  \"bad\"\nend\n"
}

#[test]
fn check_project_keeps_spanless_related_notes_without_panicking() {
    let workspace = TempWorkspace::new("spanless-related");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", integer_literal_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["id"] == "parse.integer_literal"
                && diagnostic["related"][0]["message"] == "Accepted integer form: 0 or 1."
                && diagnostic["related"][0].get("span").is_none()
        }),
        "{diagnostics:#?}"
    );
}

fn integer_literal_source() -> &'static str {
    "fn main() -> Int\n  0b102\nend\n"
}

#[test]
fn tool_calls_require_the_declared_wire_shape() {
    let workspace = TempWorkspace::new("tool-call-wire-shape");
    let mut server = initialized_server(&workspace);
    let invalid_params = [
        Value::Null,
        json!([]),
        json!({}),
        json!({"name": 1}),
        json!({"name": "unknown"}),
        json!({"name": "workspace_projects", "unknown": true}),
        json!({"name": "workspace_projects", "_meta": null}),
        json!({"name": "workspace_projects", "_meta": {"progressToken": null}}),
    ];

    for (index, params) in invalid_params.into_iter().enumerate() {
        let response = server
            .handle_request(json!({
                "jsonrpc": "2.0",
                "id": index,
                "method": "tools/call",
                "params": params
            }))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602, "{params}");
        assert!(response.get("result").is_none(), "{params}");
    }

    for params in [
        json!({"name": "workspace_projects"}),
        json!({"name": "workspace_projects", "_meta": {"progressToken": 1}}),
    ] {
        let response = server
            .handle_request(json!({
                "jsonrpc": "2.0",
                "id": "accepted",
                "method": "tools/call",
                "params": params
            }))
            .unwrap();
        assert_eq!(response["result"]["structuredContent"]["generation"], 0);
        assert_eq!(response["result"]["isError"], false);
    }
}

#[test]
fn stdio_reports_parse_and_unknown_method_errors() {
    let workspace = TempWorkspace::new("protocol-errors");
    let input = [
        "not json".to_string(),
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string(),
        json!({"jsonrpc":"2.0","id":2,"method":"unknown"}).to_string(),
    ]
    .join("\n");
    let mut output = Vec::new();

    run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();

    let responses = parse_responses(output);
    assert_eq!(responses[0]["id"], Value::Null);
    assert_eq!(responses[0]["error"]["code"], -32700);
    assert_eq!(responses[1]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(responses[2]["id"], 2);
    assert_eq!(responses[2]["error"]["code"], -32601);
}

#[test]
fn initialize_requires_the_declared_wire_shape() {
    let workspace = TempWorkspace::new("initialize-wire-shape");
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: WorkspaceBase::open(workspace.root.clone()).unwrap(),
        selection,
        initialized: false,
    };
    let valid = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"},
            "_meta": {"progressToken": "startup"}
        }
    });
    let invalid_requests = [
        json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"initialize","params":{"protocolVersion":"2025-06-18","clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"},"_meta":{"progressToken":null}}}),
    ];
    for request in invalid_requests {
        let response = server.handle_request(request).unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }

    assert!(
        server
            .handle_request(valid.clone())
            .unwrap()
            .get("result")
            .is_some()
    );
    let repeated = server.handle_request(valid).unwrap();
    assert_eq!(repeated["error"]["code"], -32602);
    assert_eq!(repeated["error"]["message"], "Server already initialized");
    assert!(repeated.get("result").is_none());
}

#[test]
fn lifecycle_rejects_operations_before_initialize() {
    let workspace = TempWorkspace::new("lifecycle-before-initialize");
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: WorkspaceBase::open(workspace.root.clone()).unwrap(),
        selection,
        initialized: false,
    };

    let response = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}))
        .unwrap();
    assert_eq!(response["error"]["code"], -32602);
    assert_eq!(response["error"]["message"], "Server not initialized");
    assert!(response.get("result").is_none());
}

#[test]
fn request_ids_accept_strings_and_numbers_but_reject_null() {
    let workspace = TempWorkspace::new("request-ids");
    let mut server = initialized_server(&workspace);

    let null_response = server
        .handle_request(json!({"jsonrpc":"2.0","id":null,"method":"ping"}))
        .unwrap();
    assert_eq!(null_response["id"], Value::Null);
    assert_eq!(null_response["error"]["code"], -32600);

    for id in [
        json!("ok"),
        json!(0),
        json!(-1),
        json!(1),
        json!(1.5),
        json!(1e3),
    ] {
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":id,"method":"ping"}))
            .unwrap();
        assert_eq!(response["result"], json!({}));
    }

    let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":"ok","method":"ping","params":{"_meta":{"progressToken":7.5}}}))
            .unwrap();
    assert_eq!(response["result"], json!({}));
}

#[test]
fn stdio_preserves_numeric_request_id_spellings() {
    let workspace = TempWorkspace::new("large-request-ids");
    let input = [
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#,
            r#"{"jsonrpc":"2.0","id":7.5,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":1e3,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":18446744073709551616,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":18446744073709551617,"method":"ping"}"#,
        ]
        .join("\n");
    let mut output = Vec::new();
    run(workspace.root.clone(), input.as_bytes(), &mut output).unwrap();
    let response = String::from_utf8(output).unwrap();
    let lines = response.lines().collect::<Vec<_>>();

    assert_eq!(lines.len(), 5);
    assert!(lines[1].contains(r#""id":7.5"#), "{response}");
    assert!(lines[2].contains(r#""id":1e3"#), "{response}");
    assert!(
        lines[3].contains(r#""id":18446744073709551616"#),
        "{response}"
    );
    assert!(
        lines[4].contains(r#""id":18446744073709551617"#),
        "{response}"
    );
}

#[test]
fn malformed_idless_requests_return_invalid_request_with_null_id() {
    let workspace = TempWorkspace::new("malformed-idless");
    let mut server = initialized_server(&workspace);

    let notification = server.handle_request(json!({"jsonrpc":"2.0","method":"ping"}));
    assert!(notification.is_none());

    for request in [
        json!({"jsonrpc":"2.0","method":7}),
        json!({"jsonrpc":"2.0"}),
        json!({"method":"ping"}),
    ] {
        let response = server.handle_request(request).unwrap();
        assert_eq!(response["id"], Value::Null);
        assert_eq!(response["error"]["code"], -32600);
        assert!(response.get("result").is_none());
    }
}

#[test]
fn refresh_domain_failure_preserves_the_observable_selection() {
    let workspace = TempWorkspace::new("refresh-domain-failure");
    workspace.write("alpha/veln.toml", "");
    let mut server = initialized_server(&workspace);
    let refresh_params = json!({"name":"refresh_workspace","arguments":{}});

    let result = server
        .call_tool_with_refresh(Some(&refresh_params), |selection| {
            selection.refresh_with(|| Err(io::Error::other("injected discovery failure")))
        })
        .unwrap();
    assert_eq!(result["isError"], true);
    assert_eq!(
        result["structuredContent"],
        json!({
            "code": "generation_failed",
            "message": "workspace project discovery failed",
            "details": {}
        })
    );

    let list_params = json!({"name":"workspace_projects","arguments":{}});
    let following = server.call_tool(Some(&list_params)).unwrap();
    assert_eq!(
        following["structuredContent"],
        json!({"generation": 0, "roots": ["alpha"]})
    );
}

#[cfg(unix)]
#[test]
fn refresh_with_unrepresentable_manifest_root_reports_generation_failure() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let workspace = TempWorkspace::new("refresh-non-utf8-root");
    workspace.write("alpha/veln.toml", "");
    let mut server = initialized_server(&workspace);

    let unrepresentable_root = workspace.root.join(OsString::from_vec(vec![b'p', 0xff]));
    fs::create_dir_all(&unrepresentable_root).unwrap();
    fs::write(unrepresentable_root.join("veln.toml"), "").unwrap();

    let refresh_params = json!({"name":"refresh_workspace","arguments":{}});
    let result = server.call_tool(Some(&refresh_params)).unwrap();
    assert_eq!(
        result["structuredContent"],
        json!({
            "code": "generation_failed",
            "message": "workspace project discovery failed",
            "details": {}
        })
    );
    assert_eq!(result["isError"], true);

    let list_params = json!({"name":"workspace_projects","arguments":{}});
    let following = server.call_tool(Some(&list_params)).unwrap();
    assert_eq!(
        following["structuredContent"],
        json!({"generation": 0, "roots": ["alpha"]})
    );
}

struct TempWorkspace {
    root: PathBuf,
}

fn initialized_server(workspace: &TempWorkspace) -> Server {
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    Server {
        base,
        selection,
        initialized: true,
    }
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-mcp-server-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        self.write_bytes(relative, contents.as_bytes());
    }

    fn write_bytes(&self, relative: &str, contents: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn mkdir(&self, relative: &str) {
        fs::create_dir_all(self.root.join(relative)).unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
