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
        ["workspace_projects", "refresh_workspace", "check_project"]
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
    ];
    for request in requests {
        let response = server.handle_request(request).unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }
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
