use super::*;

fn write_handler_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "handler run() handles Work\r\n",
            "  go() => 1\r\n",
            "end\r\n\r\n",
            "fn first() -> Int\r\n",
            "  \"😀\" + handle 1 with run()\r\n",
            "end\r\n\r\n",
            "fn second() -> Int\r\n",
            "  handle 2 with run()\r\n",
            "end\r\n",
        ),
    );
}

#[test]
fn references_page_workspace_handlers_with_unicode_scalar_coordinates() {
    let workspace = TempWorkspace::new("references-workspace-handler");
    write_handler_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":6, "column":23, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {"uri":uri,"range":{"start":{"line":6,"column":23},"end":{"line":6,"column":26}}},
            {"uri":crate::definition::path_to_uri(&workspace.path("main.veln")),"range":{"start":{"line":10,"column":17},"end":{"line":10,"column":20}}}
        ])
    );

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9,
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":1,"column":9},"end":{"line":1,"column":12}})
    );
    let first_cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":first_cursor}));
    assert_eq!(
        second["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":6,"column":23},"end":{"line":6,"column":26}})
    );
    let second_cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":second_cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":10,"column":17},"end":{"line":10,"column":20}})
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}

#[test]
fn workspace_handler_reference_failures_preserve_results_resources_selection_and_cursor() {
    let workspace = TempWorkspace::new("references-workspace-handler-failure-state");
    write_handler_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9
    }));
    let seeded = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "page_size":1
    }));
    let cursor = seeded["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let resources = all_resource_state(&mut server);
    let selection = server.selection_result();

    let invalid_path = server.references_tool(&json!({
        "source":"missing.veln", "line":1, "column":1
    }));
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    let invalid_position = server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1
    }));
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    let invalid_cursor = server.references_tool(&json!({"cursor":format!("{cursor}x")}));
    assert_eq!(
        invalid_cursor["structuredContent"]["code"],
        "invalid_cursor"
    );
    let missing_resource = server
        .handle_request(json!({
            "jsonrpc":"2.0", "id":"missing-handler-resource",
            "method":"resources/read",
            "params":{"uri":"veln-pkg:///missing/snapshot/main.veln"}
        }))
        .unwrap();
    assert_eq!(
        missing_resource["error"]["data"]["code"],
        "resource_not_found"
    );

    assert_eq!(all_resource_state(&mut server), resources);
    assert_eq!(server.selection_result(), selection);
    let continuation = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(continuation["isError"], false);
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let after = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9
    }));
    assert_eq!(after, before);
}

#[test]
fn references_reject_ambiguous_recovered_and_non_bare_workspace_handlers() {
    let workspace = TempWorkspace::new("references-workspace-handler-boundaries");
    workspace.write("veln.toml", "");
    workspace.write(
        "first.veln",
        "mod shared\n\nhandler run() handles Work\n  go() => 1\nend\n",
    );
    workspace.write(
        "second.veln",
        concat!(
            "mod shared\n\n",
            "handler run() handles Work\n  go() => 2\nend\n\n",
            "handler keep() handles Work @\n  go() => 3\nend\n\n",
            "fn use() -> Int\n  handle 1 with run()\nend\n\n",
            "fn qualified() -> Int\n  handle 1 with other::run()\nend\n\n",
            "fn recovered() -> Int\n  handle 1 with keep(1 2)\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    for (source, line, column) in [
        ("first.veln", 3, 9),
        ("second.veln", 7, 9),
        ("second.veln", 12, 17),
        ("second.veln", 16, 24),
        ("second.veln", 20, 18),
    ] {
        let result = server.references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}
