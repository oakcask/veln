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

    assert_resources_are_sorted(resources);
    assert_language_index_metadata(&resources[0]);
    assert_topic_metadata(resources);
    assert_standard_library_metadata(resources);
}

fn assert_resources_are_sorted(resources: &[Value]) {
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut sorted = uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(uris, sorted);
}

fn assert_language_index_metadata(index: &Value) {
    assert_eq!(index["name"], "language-index");
    assert_eq!(index["title"], "Veln Language Reference");
    assert_eq!(
        index["mimeType"],
        veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE
    );
    assert!(index.get("description").is_none());
}

fn assert_topic_metadata(resources: &[Value]) {
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

fn assert_standard_library_metadata(resources: &[Value]) {
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
    let emitted_uris = read_all_listed_resources(&mut server, resources);

    assert_language_index_links(&mut server, resources, &emitted_uris);
    assert_lexical_topic_content(&mut server, resources);
    assert_emitted_resource_uris_are_readable(&mut server, emitted_uris);
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
fn resources_reject_malformed_params_and_unknown_uris() {
    let workspace = TempWorkspace::new("resource-rejections");
    let mut server = initialized_server(&workspace);
    assert_list_params_rejected(&mut server);
    assert_read_params_rejected(&mut server);

    let unknown_uris = rejected_language_resource_uris()
        .into_iter()
        .chain(rejected_standard_library_resource_uris(&mut server));
    assert_unknown_resource_reads_rejected(&mut server, unknown_uris);
}

fn read_all_listed_resources(server: &mut Server, resources: &[Value]) -> BTreeSet<String> {
    let mut emitted_uris = BTreeSet::new();
    for resource in resources {
        emitted_uris.extend(assert_listed_resource_is_readable(server, resource));
    }
    emitted_uris
}

fn assert_listed_resource_is_readable(server: &mut Server, resource: &Value) -> BTreeSet<String> {
    let uri = resource["uri"].as_str().unwrap();
    let read = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap();
    let content = &read["result"]["contents"][0];
    assert_eq!(content["uri"], uri);
    assert_eq!(content["mimeType"], resource["mimeType"]);
    assert!(content["text"].as_str().unwrap().len() > 20);
    extract_resource_uris(content["text"].as_str().unwrap())
}

fn assert_language_index_links(
    server: &mut Server,
    resources: &[Value],
    emitted_uris: &BTreeSet<String>,
) {
    let index_uri = resources[0]["uri"].as_str().unwrap();
    let index = read_resource(server, 3, index_uri);
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(index_text.starts_with("# Veln Language Reference\n\n"));
    for uri in listed_language_topic_uris(resources) {
        assert!(index_text.contains(&uri));
    }
    assert_eq!(emitted_uris, &listed_language_topic_uris(resources));
}

fn assert_lexical_topic_content(server: &mut Server, resources: &[Value]) {
    let topic_uri = resources
        .iter()
        .find(|resource| resource["name"] == "lexical-structure")
        .unwrap()["uri"]
        .as_str()
        .unwrap();
    let topic = read_resource(server, 4, topic_uri);
    let topic_text = topic["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(topic_text.contains("```ebnf\nModule        ::= ModuleHeader? UseDecl* Item*\n```"));
    assert!(topic_text.contains("### Accepted source-surface case"));
    assert!(topic_text.contains("#### main.veln"));
    assert!(topic_text.contains("- comments"));
}

fn assert_emitted_resource_uris_are_readable(server: &mut Server, emitted_uris: BTreeSet<String>) {
    for uri in emitted_uris {
        let read = read_resource(server, 5, &uri);
        assert_eq!(read["result"]["contents"][0]["uri"], uri);
    }
}

fn listed_language_topic_uris(resources: &[Value]) -> BTreeSet<String> {
    resources
        .iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            (uri.starts_with("veln-doc:///language/") && resource["name"] != "language-index")
                .then(|| uri.to_string())
        })
        .collect()
}

fn read_resource(server: &mut Server, id: u64, uri: &str) -> Value {
    server
        .handle_request(
            json!({"jsonrpc":"2.0","id":id,"method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap()
}

fn assert_list_params_rejected(server: &mut Server) {
    for params in invalid_list_params() {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":1,"method":"resources/list","params":params}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }
}

fn invalid_list_params() -> [Value; 5] {
    [
        Value::Null,
        json!([]),
        json!({"cursor":"next"}),
        json!({"unknown":true}),
        json!({"_meta":null}),
    ]
}

fn assert_read_params_rejected(server: &mut Server) {
    for params in invalid_read_params() {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":params}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
    }
}

fn invalid_read_params() -> [Value; 6] {
    [
        Value::Null,
        json!([]),
        json!({}),
        json!({"uri":null}),
        json!({"uri":1}),
        json!({"uri":"veln-doc:///language/snapshot/wrong/index","unknown":true}),
    ]
}

fn rejected_language_resource_uris() -> Vec<String> {
    let digest = veln_repo_language_reference::checked_catalog_digest();
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    vec![
        format!("veln-doc:///language/snapshot/{wrong_digest}/index"),
        format!("veln-doc:///language/snapshot/{wrong_digest}/topic/lexical-structure"),
        format!("veln-doc:///language/snapshot/{digest}/topic/missing"),
        format!("veln-doc:///language/snapshot/{digest}/topic/Lexical-Structure"),
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical%2Dstructure"),
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical-structure?x=1"),
        format!("veln-doc:///language/snapshot/{digest}/topic/lexical-structure#section"),
        format!("veln-doc://host/language/snapshot/{digest}/topic/lexical-structure"),
    ]
}

fn rejected_standard_library_resource_uris(server: &mut Server) -> Vec<String> {
    let std_resource = listed_standard_library_resource_uri(server);
    let std_digest = std_resource_digest(&std_resource);
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    vec![
        std_resource.replace(std_digest, wrong_digest),
        format!("veln-pkg:///other/snapshot/{std_digest}/prelude.veln"),
        format!("veln-pkg:///std/snapshot/{std_digest}/missing.veln"),
        format!("veln-pkg:///std/snapshot/{std_digest}/prelude_test.veln"),
        format!("veln-pkg:/std/snapshot/{std_digest}/prelude.veln"),
        std_resource.replacen("veln-pkg", "VELN-pkg", 1),
        std_resource.replace("prelude.veln", "prelude%2Eveln"),
        format!("{std_resource}?x=1"),
    ]
}

fn listed_standard_library_resource_uri(server: &mut Server) -> String {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["name"] == "prelude.veln")
        .unwrap()["uri"]
        .as_str()
        .unwrap()
        .to_string()
}

fn std_resource_digest(std_resource: &str) -> &str {
    std_resource
        .strip_prefix("veln-pkg:///std/snapshot/")
        .unwrap()
        .split('/')
        .next()
        .unwrap()
}

fn assert_unknown_resource_reads_rejected(
    server: &mut Server,
    unknown_uris: impl IntoIterator<Item = String>,
) {
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
    let standard_library = crate::language_resources::StandardLibraryResources::checked()
        .unwrap()
        .resources;
    resources.extend(
        standard_library
            .iter()
            .filter(|resource| resource.listed)
            .map(crate::language_resources::PublishedResource::metadata),
    );
    sort_resource_metadata(&mut resources);
    resources
}

fn sort_resource_metadata(resources: &mut [Value]) {
    resources.sort_by(|left, right| {
        left["uri"]
            .as_str()
            .unwrap()
            .as_bytes()
            .cmp(right["uri"].as_str().unwrap().as_bytes())
    });
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
