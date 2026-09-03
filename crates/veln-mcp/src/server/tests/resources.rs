use super::*;

#[test]
fn initialize_advertises_immutable_resources() {
    let workspace = TempWorkspace::new("resources-initialize");
    let selection = Selection::discover(&workspace.root).unwrap();
    let mut server = Server {
        base: WorkspaceBase::open(workspace.root.clone()).unwrap(),
        selection,
        initialized: false,
        language_resources: LanguageResources::checked().unwrap(),
    };
    let response = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}))
        .unwrap();
    assert_eq!(
        response["result"]["capabilities"]["resources"],
        json!({"listChanged": false, "subscribe": false})
    );
}

#[test]
fn resources_list_returns_sorted_language_reference_metadata() {
    let workspace = TempWorkspace::new("resources-list");
    let mut server = initialized_server(&workspace);
    let response = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list","params":{"_meta":{"progressToken":"list"}}}))
        .unwrap();
    let resources = response["result"]["resources"].as_array().unwrap();
    assert_eq!(response["result"].get("nextCursor"), None);
    assert!(resources.len() > 1);
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut sorted = uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(uris, sorted);

    let index = &resources[0];
    assert_eq!(index["name"], "language-index");
    assert_eq!(index["title"], "Veln Language Reference");
    assert_eq!(
        index["mimeType"],
        veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE
    );
    assert!(index.get("description").is_none());
    let topic = resources
        .iter()
        .find(|resource| resource["name"] == "lexical-structure")
        .unwrap();
    assert_eq!(topic["title"], "Lexical Structure And Grammar");
    assert_eq!(
        topic["description"],
        "Source files use ASCII keyword and punctuation tokens, hash comments, identifiers, holes, literals, and the complete executable source grammar."
    );
}

#[test]
fn resources_read_returns_complete_markdown_for_listed_uris() {
    let workspace = TempWorkspace::new("resources-read");
    let mut server = initialized_server(&workspace);
    let list = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap();
    let resources = list["result"]["resources"].as_array().unwrap();
    for resource in resources {
        let uri = resource["uri"].as_str().unwrap();
        let read = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":uri}}),
            )
            .unwrap();
        assert_eq!(read["result"]["contents"][0]["uri"], uri);
        assert_eq!(
            read["result"]["contents"][0]["mimeType"],
            resource["mimeType"]
        );
        assert!(
            read["result"]["contents"][0]["text"]
                .as_str()
                .unwrap()
                .len()
                > 20
        );
    }

    let index_uri = resources[0]["uri"].as_str().unwrap();
    let index = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":index_uri}}),
        )
        .unwrap();
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(index_text.starts_with("# Veln Language Reference\n\n"));
    for resource in &resources[1..] {
        assert!(index_text.contains(resource["uri"].as_str().unwrap()));
    }

    let topic_uri = resources
        .iter()
        .find(|resource| resource["name"] == "lexical-structure")
        .unwrap()["uri"]
        .as_str()
        .unwrap();
    let topic = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":topic_uri}}),
        )
        .unwrap();
    let topic_text = topic["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(topic_text.contains("```ebnf\nModule        ::= ModuleHeader? UseDecl* Item*\n```"));
    assert!(topic_text.contains("### Accepted source-surface case"));
    assert!(topic_text.contains("#### main.veln"));
    assert!(topic_text.contains("- comments"));
}

#[test]
fn resource_uris_are_exact_and_state_is_preserved_across_tools() {
    let workspace = TempWorkspace::new("resource-state");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let mut server = initialized_server(&workspace);
    let before = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap()["result"]
        .clone();
    let first_uri = before["resources"][0]["uri"].as_str().unwrap().to_string();
    let first_read = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":first_uri}}),
        )
        .unwrap()["result"]
        .clone();

    server
        .handle_request(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
    server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"check_project","arguments":{"project":"."}}}))
        .unwrap();

    let after = server
        .handle_request(json!({"jsonrpc":"2.0","id":5,"method":"resources/list"}))
        .unwrap()["result"]
        .clone();
    let second_read = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":6,"method":"resources/read","params":{"uri":first_uri}}),
        )
        .unwrap()["result"]
        .clone();
    assert_eq!(after, before);
    assert_eq!(second_read, first_read);
}

#[test]
fn resources_reject_malformed_params_and_unknown_uris() {
    let workspace = TempWorkspace::new("resource-rejections");
    let mut server = initialized_server(&workspace);
    for params in [
        Value::Null,
        json!([]),
        json!({"cursor":"next"}),
        json!({"unknown":true}),
        json!({"_meta":null}),
    ] {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":1,"method":"resources/list","params":params}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }
    for params in [
        Value::Null,
        json!([]),
        json!({}),
        json!({"uri":null}),
        json!({"uri":1}),
        json!({"uri":"veln-doc:///language/snapshot/wrong/index","unknown":true}),
    ] {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":params}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }
    for uri in [
        "veln-doc:///language/snapshot/wrong/index",
        "veln-doc:///language/snapshot/wrong/topic/lexical-structure",
        "veln-doc:///language/snapshot/wrong/topic/missing",
        "veln-doc:///language/snapshot/wrong/topic/lexical-structure?x=1",
    ] {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":uri}}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32002, "{uri}");
        assert_eq!(response["error"]["data"]["code"], "resource_not_found");
        assert!(response.get("result").is_none());
    }
}
