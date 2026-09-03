use super::*;
use std::collections::BTreeSet;

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
    let expected = expected_resource_metadata();
    assert_eq!(response["result"].get("nextCursor"), None);
    assert_eq!(resources, &expected);
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
    let listed_uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap().to_string())
        .collect::<BTreeSet<_>>();
    let mut emitted_uris = BTreeSet::new();
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
        emitted_uris.extend(extract_resource_uris(
            read["result"]["contents"][0]["text"].as_str().unwrap(),
        ));
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

    let linked_uris = listed_uris
        .iter()
        .filter(|uri| !uri.ends_with("/index"))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(emitted_uris, linked_uris);
    for uri in emitted_uris {
        let read = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":5,"method":"resources/read","params":{"uri":uri}}),
            )
            .unwrap();
        assert_eq!(read["result"]["contents"][0]["uri"], uri);
    }
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
    let digest = veln_repo_language_reference::checked_catalog_digest();
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let unknown_topic = format!("veln-doc:///language/snapshot/{digest}/topic/missing");
    let noncanonical_topic_case =
        format!("veln-doc:///language/snapshot/{digest}/topic/Lexical-Structure");
    let noncanonical_topic_percent =
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical%2Dstructure");
    let noncanonical_query =
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical-structure?x=1");
    let noncanonical_fragment =
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical-structure#section");
    let noncanonical_authority =
        format!("veln-doc://host/language/snapshot/{digest}/topic/lexical-structure");
    let unknown_uris = [
        format!("veln-doc:///language/snapshot/{wrong_digest}/index"),
        format!("veln-doc:///language/snapshot/{wrong_digest}/topic/lexical-structure"),
        unknown_topic,
        noncanonical_topic_case,
        noncanonical_topic_percent,
        noncanonical_query,
        noncanonical_fragment,
        noncanonical_authority,
    ];
    for uri in unknown_uris {
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

fn expected_resource_metadata() -> Vec<Value> {
    let catalog: Value =
        serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes()).unwrap();
    let digest = veln_repo_language_reference::checked_catalog_digest();
    let base = format!("veln-doc:///language/snapshot/{digest}");
    let mut resources = vec![json!({
        "uri": format!("{base}/index"),
        "name": "language-index",
        "title": "Veln Language Reference",
        "mimeType": veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
    })];
    for topic in catalog["topics"].as_array().unwrap() {
        let id = topic["id"].as_str().unwrap();
        resources.push(json!({
            "uri": format!("{base}/topic/{id}"),
            "name": id,
            "title": topic["title"].as_str().unwrap(),
            "description": topic["summary"].as_str().unwrap(),
            "mimeType": veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
        }));
    }
    resources.sort_by(|left, right| {
        left["uri"]
            .as_str()
            .unwrap()
            .as_bytes()
            .cmp(right["uri"].as_str().unwrap().as_bytes())
    });
    resources
}

fn extract_resource_uris(text: &str) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("veln-doc:///language/snapshot/") {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        uris.insert(after_start[..end].to_string());
        rest = &after_start[end..];
    }
    uris
}
