use super::*;

fn write_handler_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "handler run(value: Int) for Work\r\n",
            "  go() => 1\r\n",
            "end\r\n\r\n",
            "fn first() -> Int\r\n",
            "  \"😀\" + handle 1 with run((1 + 2))\r\n",
            "end\r\n\r\n",
            "fn second() -> Int\r\n",
            "  handle 2 with run(2)\r\n",
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
fn imported_workspace_handler_navigation_pages_shared_scalar_locations() {
    let workspace = TempWorkspace::new("references-workspace-imported-handler");
    workspace.write("veln.toml", "");
    workspace.write(
        "handler.veln",
        concat!(
            "mod library::fx\n\n",
            "pub handler run() for Work\n  go() => 1\nend\n\n",
            "fn local() -> Int\n  handle 0 with run()\nend\n",
            "\nhandler hidden() for Work\n  go() => 2\nend\n",
        ),
    );
    workspace.write(
        "consumer.veln",
        concat!(
            "mod consumer\n\nuse library::fx\n\n",
            "fn full() -> Int\n  handle 1 with library::fx::run()\nend\n\n",
            "fn alias() -> Int\n  handle 2 with fx::run()\nend\n",
            "\nfn private() -> Int\n  handle 3 with library::fx::hidden()\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let handler_uri = crate::definition::path_to_uri(&workspace.path("handler.veln"));
    let consumer_uri = crate::definition::path_to_uri(&workspace.path("consumer.veln"));

    let definition = server.definition_tool(&json!({
        "source":"consumer.veln", "line":6, "column":30
    }));
    assert_eq!(
        definition["structuredContent"]["definition"]["uri"],
        handler_uri
    );
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":3,"column":13},"end":{"line":3,"column":16}})
    );

    let without_declaration = server.references_tool(&json!({
        "source":"consumer.veln", "line":10, "column":21,
        "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {"uri":consumer_uri,"range":{"start":{"line":6,"column":30},"end":{"line":6,"column":33}}},
            {"uri":crate::definition::path_to_uri(&workspace.path("consumer.veln")),"range":{"start":{"line":10,"column":21},"end":{"line":10,"column":24}}},
            {"uri":handler_uri,"range":{"start":{"line":8,"column":17},"end":{"line":8,"column":20}}}
        ])
    );

    let first = server.references_tool(&json!({
        "source":"handler.veln", "line":3, "column":13,
        "include_declaration":true, "page_size":2
    }));
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(second["structuredContent"].get("next_cursor").is_none());

    let private_definition = server.definition_tool(&json!({
        "source":"consumer.veln", "line":14, "column":30
    }));
    assert!(private_definition["structuredContent"]["definition"].is_null());
    let private_references = server.references_tool(&json!({
        "source":"consumer.veln", "line":14, "column":30,
        "include_declaration":true
    }));
    assert_eq!(
        private_references["structuredContent"]["references"],
        json!([])
    );
}

#[test]
fn references_include_a_workspace_handler_declaration_without_occurrences() {
    let workspace = TempWorkspace::new("references-workspace-handler-declaration-only");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "handler run() for Work\n  go() => 1\nend\n");
    let mut server = initialized_server(&workspace);

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([])
    );

    let with_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "include_declaration":true
    }));
    assert_eq!(
        with_declaration["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {
                "start": {"line": 1, "column": 9},
                "end": {"line": 1, "column": 12}
            }
        }])
    );
}

fn exercise_handler_reference_failures(server: &mut Server, cursor: &str) {
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

    exercise_handler_reference_failures(&mut server, &cursor);

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
