use super::*;
use std::collections::BTreeSet;

#[test]
fn standard_library_package_documentation_round_trips_from_index_links() {
    let workspace = TempWorkspace::new("standard-doc-resources");
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let list = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap();
    let resources = list["result"]["resources"].as_array().unwrap();
    let index_resource = listed_documentation_index(resources);
    assert_index_metadata(index_resource);

    let index = read_resource(&mut server, 2, index_resource["uri"].as_str().unwrap());
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    let module_uri = assert_index_text_links_hidden_module(resources, index_text);

    let module = read_resource(&mut server, 3, &module_uri);
    let module_text = module["result"]["contents"][0]["text"].as_str().unwrap();
    let declaration_uri = assert_module_text_links_hidden_declaration(resources, module_text);

    let declaration = read_resource(&mut server, 4, &declaration_uri);
    let declaration_text = declaration["result"]["contents"][0]["text"]
        .as_str()
        .unwrap();
    assert!(declaration_text.contains("- Package identity: std"));
    assert!(declaration_text.contains("- Declaration id: "));
}

fn listed_documentation_index(resources: &[Value]) -> &Value {
    resources
        .iter()
        .find(|resource| resource["name"] == "std-documentation-index")
        .unwrap()
}

fn assert_index_metadata(index_resource: &Value) {
    assert!(
        index_resource["uri"]
            .as_str()
            .unwrap()
            .starts_with("veln-doc:///package/std/snapshot/")
    );
    assert_eq!(
        index_resource["mimeType"],
        veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE
    );
}

fn assert_index_text_links_hidden_module(resources: &[Value], index_text: &str) -> String {
    assert!(index_text.starts_with("# Package Documentation: std\n\n"));
    assert!(index_text.contains("- Package identity: std"));
    assert!(index_text.contains("- Exported modules: prelude"));
    let module_uri = linked_package_doc_uri(index_text, "/module/");
    assert_unlisted(resources, &module_uri);
    module_uri
}

fn assert_module_text_links_hidden_declaration(resources: &[Value], module_text: &str) -> String {
    assert!(module_text.starts_with("# Module "));
    assert!(module_text.contains("## Declarations"));
    let declaration_uri = linked_package_doc_uri(module_text, "/declaration/");
    assert_unlisted(resources, &declaration_uri);
    declaration_uri
}

fn linked_package_doc_uri(text: &str, segment: &str) -> String {
    extract_package_doc_uris(text)
        .into_iter()
        .find(|uri| uri.contains(segment))
        .unwrap()
}

fn assert_unlisted(resources: &[Value], uri: &str) {
    assert!(
        resources
            .iter()
            .all(|resource| resource["uri"].as_str().unwrap() != uri)
    );
}

#[test]
fn resource_templates_list_advertises_package_documentation_forms() {
    let workspace = TempWorkspace::new("resource-template-list");
    let mut server = initialized_server(&workspace);
    let response = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/templates/list","params":{"_meta":{"progressToken":"templates"}}}))
        .unwrap();
    let templates = response["result"]["resourceTemplates"].as_array().unwrap();

    assert_eq!(templates.len(), 2);
    assert_eq!(response["result"].get("nextCursor"), None);
    assert_eq!(templates[0]["name"], "package-documentation-module");
    assert_eq!(
        templates[0]["uriTemplate"],
        "veln-doc:///package/{package}/snapshot/{snapshot_digest}/documentation/{documentation_digest}/module/{module_id}"
    );
    assert_eq!(
        templates[0]["mimeType"],
        veln_language_service::PACKAGE_DOCUMENTATION_MARKDOWN_MEDIA_TYPE
    );
    assert_eq!(templates[1]["name"], "package-documentation-declaration");
    assert_eq!(
        templates[1]["uriTemplate"],
        "veln-doc:///package/{package}/snapshot/{snapshot_digest}/documentation/{documentation_digest}/declaration/{declaration_id}"
    );

    let invalid = server
        .handle_request(json!({"jsonrpc":"2.0","id":2,"method":"resources/templates/list","params":{"cursor":"next"}}))
        .unwrap();
    assert_eq!(invalid["error"]["code"], -32602);
}

#[test]
fn standard_library_package_documentation_rejections_are_exact() {
    let workspace = TempWorkspace::new("standard-doc-resource-rejections");
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let unknown_uris = rejected_standard_library_documentation_uris(&mut server);

    assert_unknown_resource_reads_rejected(&mut server, unknown_uris);
}

fn rejected_standard_library_documentation_uris(server: &mut Server) -> Vec<String> {
    let index_uri = listed_standard_library_documentation_index_uri(server);
    let (snapshot_digest, doc_digest) = documentation_uri_digests(&index_uri);
    let declaration_id = linked_declaration_id(server, &index_uri);
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";

    vec![
        index_uri.replace(snapshot_digest, wrong_digest),
        index_uri.replace(doc_digest, wrong_digest),
        format!(
            "veln-doc:///package/std/snapshot/{snapshot_digest}/documentation/{doc_digest}/declaration/{declaration_id}"
        )
        .replace(&declaration_id, "missing"),
        format!("veln-doc:///package/other/snapshot/{snapshot_digest}/documentation/{doc_digest}/index"),
        format!("veln-doc:///package/std/snapshot/{snapshot_digest}/documentation/{doc_digest}/module/missing"),
        format!("veln-doc:///package/std/snapshot/{snapshot_digest}/documentation/{doc_digest}/status"),
        index_uri.replacen("veln-doc", "VELN-doc", 1),
        index_uri.replace("/index", "/Index"),
        format!("{index_uri}?x=1"),
    ]
}

fn documentation_uri_digests(index_uri: &str) -> (&str, &str) {
    let rest = index_uri
        .strip_prefix("veln-doc:///package/std/snapshot/")
        .unwrap();
    let mut parts = rest.split('/');
    let snapshot_digest = parts.next().unwrap();
    assert_eq!(parts.next(), Some("documentation"));
    (snapshot_digest, parts.next().unwrap())
}

fn linked_declaration_id(server: &mut Server, index_uri: &str) -> String {
    let index = read_resource(server, 90, index_uri);
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    let module_uri = linked_package_doc_uri(index_text, "/module/");
    let module = read_resource(server, 91, &module_uri);
    let module_text = module["result"]["contents"][0]["text"].as_str().unwrap();
    linked_package_doc_uri(module_text, "/declaration/")
        .rsplit('/')
        .next()
        .unwrap()
        .to_string()
}

fn listed_standard_library_documentation_index_uri(server: &mut Server) -> String {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|resource| resource["name"] == "std-documentation-index")
        .unwrap()["uri"]
        .as_str()
        .unwrap()
        .to_string()
}

fn read_resource(server: &mut Server, id: u64, uri: &str) -> Value {
    server
        .handle_request(
            json!({"jsonrpc":"2.0","id":id,"method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap()
}
fn extract_package_doc_uris(text: &str) -> BTreeSet<String> {
    extract_uris_with_prefix(text, "veln-doc:///package/")
}

fn extract_uris_with_prefix(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find(prefix) {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        uris.insert(after_start[..end].to_string());
        rest = &after_start[end..];
    }
    uris
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
