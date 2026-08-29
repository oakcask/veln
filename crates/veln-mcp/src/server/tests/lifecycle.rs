use super::*;

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
