use super::*;

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
