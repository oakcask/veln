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
    assert_eq!(names, ["workspace_projects", "refresh_workspace"]);
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
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
        selection,
        initialized: true,
    };
    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"workspace_projects","arguments":{"unknown":true}}}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"refresh_workspace","arguments":[]}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":null}}),
    ];
    for request in requests {
        let response = server.handle_request(request).unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }
}

#[test]
fn initialize_requires_the_declared_wire_shape() {
    let workspace = TempWorkspace::new("initialize-wire-shape");
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
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
        base: workspace.root.clone(),
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
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
        selection,
        initialized: true,
    };

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
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
        selection,
        initialized: true,
    };

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
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
        selection,
        initialized: true,
    };
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
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: workspace.root.clone(),
        selection,
        initialized: true,
    };

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
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
