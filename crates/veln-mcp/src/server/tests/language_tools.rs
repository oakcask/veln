use super::*;
use crate::language_resources::LanguageTopic;

#[test]
fn search_docs_ranks_exact_prefix_and_ties_by_uri_bytes() {
    let workspace = TempWorkspace::new("search-rank");
    let mut server = initialized_server(&workspace);

    let exact = search(&mut server, json!({"query": "schemas", "limit": 3}));
    let exact_results = exact["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(exact_results[0]["title"], "Schemas");
    assert_eq!(exact_results[0]["excerpt"], "schemas");

    let prefix = search(
        &mut server,
        json!({"query": "Types, Inference", "limit": 2}),
    );
    assert_eq!(
        prefix["structuredContent"]["results"][0]["title"],
        "Types, Inference, And Constructors"
    );

    let tied = search(&mut server, json!({"query": "source", "limit": 50}));
    let tied_results = tied["structuredContent"]["results"].as_array().unwrap();
    let uris = tied_results
        .iter()
        .map(|result| result["uri"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut sorted = uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(uris, sorted);
}

#[test]
fn search_docs_normalizes_case_unicode_whitespace_tokens_and_limits() {
    let workspace = TempWorkspace::new("search-normalization");
    let mut server = initialized_server(&workspace);

    let folded = search(
        &mut server,
        json!({"query": "SCHEMAS\u{2003}BINARY", "limit": 1}),
    );
    let results = folded["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Schemas");

    assert_eq!(
        veln_project::portable_normalized_case_fold("Straße ﬃ ᾷ").trim(),
        "strasse ffi ᾶι"
    );

    let empty = search(&mut server, json!({"query": "does-not-exist"}));
    assert_eq!(empty["structuredContent"]["results"], json!([]));

    let limited = search(&mut server, json!({"query": "and", "limit": 2.0}));
    assert_eq!(
        limited["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    server.language_resources = LanguageResources::for_test(
        vec![veln_repo_language_reference::RenderedResource {
            uri: "veln-doc:///language/snapshot/test/index".to_string(),
            name: "language-index".to_string(),
            title: "Veln Language Reference".to_string(),
            description: None,
            mime_type: veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
            text: "# Veln Language Reference\n".to_string(),
        }],
        vec![LanguageTopic {
            uri: "veln-doc:///language/snapshot/test/topic/unicode-boundary".to_string(),
            id: "unicode-boundary".to_string(),
            title: "Unicode Boundary".to_string(),
            summary: "Search normalization fixture.".to_string(),
            keywords: vec!["normalization".to_string()],
            body: "Café efficient handlers preserve matching behavior.".to_string(),
        }],
    );
    let normalized = search(
        &mut server,
        json!({"query": "Cafe\u{301}\u{2003}e\u{fb03}cient"}),
    );
    let results = normalized["structuredContent"]["results"]
        .as_array()
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["title"], "Unicode Boundary");
    assert_eq!(
        results[0]["excerpt"],
        "Café efficient handlers preserve matching behavior."
    );
}

#[test]
fn search_docs_uses_field_tiers_token_intersection_and_bounded_excerpts() {
    let workspace = TempWorkspace::new("search-fields");
    let mut server = initialized_server(&workspace);

    let title_or_keyword = search(&mut server, json!({"query": "effect rows"}));
    assert_eq!(
        title_or_keyword["structuredContent"]["results"][0]["title"],
        "Effects And Handlers"
    );
    assert_eq!(
        title_or_keyword["structuredContent"]["results"][0]["excerpt"],
        "Effects And Handlers"
    );

    let summary = search(&mut server, json!({"query": "format-neutral fields"}));
    assert_eq!(
        summary["structuredContent"]["results"][0]["title"],
        "Schemas"
    );
    assert!(
        summary["structuredContent"]["results"][0]["excerpt"]
            .as_str()
            .unwrap()
            .contains("format-neutral")
    );

    let body = search(&mut server, json!({"query": "does not duplicate"}));
    let result = &body["structuredContent"]["results"][0];
    assert_eq!(result["title"], "Expressions, Operators, And Patterns");
    assert!(result["excerpt"].as_str().unwrap().chars().count() <= 160);
    assert_eq!(result["prefix_truncated"], true);
    assert_eq!(result["suffix_truncated"], false);
}

#[test]
fn read_doc_matches_resource_reads_and_rejects_unknown_uris_as_tool_errors() {
    let workspace = TempWorkspace::new("read-doc");
    let mut server = initialized_server(&workspace);
    let list = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap();
    let resources = list["result"]["resources"].as_array().unwrap();
    let index = resources[0].clone();
    let topic = resources
        .iter()
        .find(|resource| resource["name"] == "lexical-structure")
        .unwrap()
        .clone();
    for resource in [index, topic] {
        let uri = resource["uri"].as_str().unwrap();
        let resource_read = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":uri}}),
            )
            .unwrap();
        let doc_read = server
            .handle_request(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_doc","arguments":{"uri":uri}}}))
            .unwrap();
        let structured = &doc_read["result"]["structuredContent"];
        assert_eq!(structured["uri"], uri);
        assert_eq!(structured["name"], resource["name"]);
        assert_eq!(structured["title"], resource["title"]);
        assert_eq!(structured.get("description"), resource.get("description"));
        assert_eq!(structured["mimeType"], resource["mimeType"]);
        assert_eq!(
            structured["text"],
            resource_read["result"]["contents"][0]["text"]
        );
        assert_eq!(
            structured["mimeType"],
            resource_read["result"]["contents"][0]["mimeType"]
        );
        assert_eq!(doc_read["result"]["isError"], false);
    }

    let digest = veln_repo_language_reference::checked_catalog_digest();
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let rejection_cases = [
        (
            "unknown",
            "veln-doc:///language/snapshot/missing/index".to_string(),
        ),
        (
            "noncanonical",
            format!("veln-doc:///language/snapshot/{digest}/topic/Lexical-Structure"),
        ),
        (
            "wrong-digest",
            format!("veln-doc:///language/snapshot/{wrong_digest}/topic/lexical-structure"),
        ),
        (
            "non-language",
            format!("veln-doc:///package/snapshot/{digest}/index"),
        ),
        (
            "unknown-topic",
            format!("veln-doc:///language/snapshot/{digest}/topic/missing"),
        ),
    ];
    for (case, uri) in rejection_cases {
        let missing = server
            .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_doc","arguments":{"uri":uri}}}))
            .unwrap();
        assert!(missing.get("error").is_none(), "{case}");
        assert_eq!(missing["result"]["isError"], true, "{case}");
        assert_eq!(
            missing["result"]["structuredContent"]["code"], "resource_not_found",
            "{case}"
        );
        assert_eq!(
            missing["result"]["structuredContent"]["details"]["uri"],
            uri
        );
        assert!(
            missing["result"]["structuredContent"].get("text").is_none(),
            "{case}"
        );
    }
}

#[test]
fn language_doc_tools_preserve_state_across_refresh_and_analysis() {
    let workspace = TempWorkspace::new("language-tool-state");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "pub fn main() -> Int\n\t1\nend\n");
    let mut server = initialized_server(&workspace);

    let before_search =
        search(&mut server, json!({"query": "schema"}))["structuredContent"].clone();
    let uri = before_search["results"][0]["uri"]
        .as_str()
        .unwrap()
        .to_string();
    let before_read = read_doc(&mut server, &uri)["structuredContent"].clone();

    server
        .handle_request(json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
    server
        .handle_request(json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"check_project","arguments":{"project":"."}}}))
        .unwrap();

    let after_search = search(&mut server, json!({"query": "schema"}))["structuredContent"].clone();
    let after_read = read_doc(&mut server, &uri)["structuredContent"].clone();
    assert_eq!(after_search, before_search);
    assert_eq!(after_read, before_read);
}

#[test]
fn language_doc_tool_inputs_use_checked_schema_rejection() {
    let workspace = TempWorkspace::new("language-tool-invalid-input");
    let mut server = initialized_server(&workspace);
    for arguments in [
        json!({}),
        json!({"query": ""}),
        json!({"query": " \n\t"}),
        json!({"query": "x".repeat(257)}),
        json!({"query": "schema", "limit": 0}),
        json!({"query": "schema", "limit": 51}),
        json!({"query": "schema", "scope": "all"}),
        json!({"query": "schema", "unknown": true}),
        Value::Null,
    ] {
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_docs","arguments":arguments}}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }
    for arguments in [
        json!({}),
        json!({"uri": null}),
        json!({"uri": "veln-doc:///language/snapshot/missing/index", "unknown": true}),
        Value::Null,
    ] {
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_doc","arguments":arguments}}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
        assert!(response.get("result").is_none());
    }
}

fn search(server: &mut Server, arguments: Value) -> Value {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"search","method":"tools/call","params":{"name":"search_docs","arguments":arguments}}))
        .unwrap()["result"]
        .clone()
}

fn read_doc(server: &mut Server, uri: &str) -> Value {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"read","method":"tools/call","params":{"name":"read_doc","arguments":{"uri":uri}}}))
        .unwrap()["result"]
        .clone()
}
