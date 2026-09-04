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
fn resources_list_returns_sorted_resource_metadata() {
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
    let prelude = resources
        .iter()
        .find(|resource| resource["name"] == "prelude.veln")
        .unwrap();
    assert!(
        prelude["uri"]
            .as_str()
            .unwrap()
            .starts_with("veln-pkg:///std/snapshot/")
    );
    assert_eq!(
        prelude["title"],
        "Veln standard library source: prelude.veln"
    );
    assert_eq!(prelude["mimeType"], "text/x-veln; charset=utf-8");
    assert!(prelude.get("description").is_none());
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
    for resource in resources.iter().filter(|resource| {
        resource["uri"]
            .as_str()
            .unwrap()
            .starts_with("veln-doc:///language/")
            && resource["name"] != "language-index"
    }) {
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
        .filter(|uri| uri.starts_with("veln-doc:///language/") && !uri.ends_with("/index"))
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
fn standard_library_resources_match_captured_distribution_sources() {
    let workspace = TempWorkspace::new("standard-resources");
    let mut server = initialized_server(&workspace);
    let list = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap();
    let resources = list["result"]["resources"].as_array().unwrap();
    let std_resources = resources
        .iter()
        .filter(|resource| {
            resource["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///std/snapshot/")
        })
        .collect::<Vec<_>>();
    let bundle = veln_stdlib::package_bundle();

    assert_eq!(std_resources.len(), bundle.files.len());
    for file in bundle.files {
        let resource = std_resources
            .iter()
            .find(|resource| resource["name"] == file.path)
            .unwrap();
        assert_eq!(
            resource["title"],
            format!("Veln standard library source: {}", file.path)
        );
        assert_eq!(resource["mimeType"], "text/x-veln; charset=utf-8");
        assert!(resource.get("description").is_none());

        let read = server
            .handle_request(json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":resource["uri"]}}))
            .unwrap();
        assert_eq!(read["result"]["contents"][0]["uri"], resource["uri"]);
        assert_eq!(
            read["result"]["contents"][0]["mimeType"],
            resource["mimeType"]
        );
        assert_eq!(read["result"]["contents"][0]["text"], file.text);
    }
}

#[test]
fn resource_uris_are_exact_and_state_is_preserved_across_tools() {
    let workspace = TempWorkspace::new("resource-state");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let mut server = initialized_server(&workspace);
    let before = standard_library_resource_state(&mut server);

    server
        .handle_request(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
    server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"check_project","arguments":{"project":"."}}}))
        .unwrap();
    let failed_refresh_params = json!({"name":"refresh_workspace","arguments":{}});
    let failed_refresh = server
        .call_tool_with_refresh(Some(&failed_refresh_params), |selection| {
            selection.refresh_with(|| Err(io::Error::other("injected discovery failure")))
        })
        .unwrap();
    assert_eq!(failed_refresh["isError"], true);
    assert_eq!(
        failed_refresh["structuredContent"]["code"],
        "generation_failed"
    );
    let search = server
        .handle_request(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"search_docs","arguments":{"query":"schema","limit":1}}}))
        .unwrap();
    assert_eq!(search["result"]["isError"], false);
    let read_doc = server
        .handle_request(json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"read_doc","arguments":{"uri":"veln-doc:///language/snapshot/wrong/index"}}}))
        .unwrap();
    assert_eq!(read_doc["result"]["isError"], true);
    assert_eq!(
        read_doc["result"]["structuredContent"]["code"],
        "resource_not_found"
    );
    let invalid_tool = server
        .handle_request(json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"search_docs","arguments":{"query":"schema","limit":0}}}))
        .unwrap();
    assert_eq!(invalid_tool["error"]["code"], -32602);
    let missing_resource = server
        .handle_request(json!({"jsonrpc":"2.0","id":10,"method":"resources/read","params":{"uri":"veln-pkg:///std/snapshot/missing/prelude.veln"}}))
        .unwrap();
    assert_eq!(
        missing_resource["error"]["data"]["code"],
        "resource_not_found"
    );
    let invalid_read = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":11,"method":"resources/read","params":{"uri":null}}),
        )
        .unwrap();
    assert_eq!(invalid_read["error"]["code"], -32602);

    let after = standard_library_resource_state(&mut server);
    assert_eq!(after, before);
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
    let std_resource = server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["name"] == "prelude.veln")
        .unwrap()["uri"]
        .as_str()
        .unwrap()
        .to_string();
    let std_digest = std_resource
        .strip_prefix("veln-pkg:///std/snapshot/")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let wrong_std_digest = std_resource.replace(std_digest, wrong_digest);
    let unknown_std_identity = format!("veln-pkg:///other/snapshot/{std_digest}/prelude.veln");
    let absent_std_path = format!("veln-pkg:///std/snapshot/{std_digest}/missing.veln");
    let test_shaped_std_path = format!("veln-pkg:///std/snapshot/{std_digest}/prelude_test.veln");
    let malformed_std_spelling = format!("veln-pkg:/std/snapshot/{std_digest}/prelude.veln");
    let noncanonical_std_scheme = std_resource.replacen("veln-pkg", "VELN-pkg", 1);
    let noncanonical_std_percent = std_resource.replace("prelude.veln", "prelude%2Eveln");
    let noncanonical_std_query = format!("{std_resource}?x=1");
    let unknown_uris = [
        format!("veln-doc:///language/snapshot/{wrong_digest}/index"),
        format!("veln-doc:///language/snapshot/{wrong_digest}/topic/lexical-structure"),
        unknown_topic,
        noncanonical_topic_case,
        noncanonical_topic_percent,
        noncanonical_query,
        noncanonical_fragment,
        noncanonical_authority,
        wrong_std_digest,
        unknown_std_identity,
        absent_std_path,
        test_shaped_std_path,
        malformed_std_spelling,
        noncanonical_std_scheme,
        noncanonical_std_percent,
        noncanonical_std_query,
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

fn standard_library_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"state-list","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|resource| {
            resource["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///std/snapshot/")
        })
        .cloned()
        .collect::<Vec<_>>();
    let reads = resources
        .iter()
        .map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            server
                .handle_request(json!({"jsonrpc":"2.0","id":"state-read","method":"resources/read","params":{"uri":uri}}))
                .unwrap()["result"]["contents"][0]
                .clone()
        })
        .collect::<Vec<_>>();
    json!({"resources": resources, "reads": reads})
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
    let standard_library = crate::language_resources::StandardLibraryResources::checked()
        .unwrap()
        .resources;
    resources.extend(
        standard_library
            .iter()
            .map(crate::language_resources::PublishedResource::metadata),
    );
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
