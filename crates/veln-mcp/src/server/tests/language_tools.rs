use super::*;
use crate::language_resources::LanguageTopic;
use std::collections::BTreeSet;

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
    let exponent_limit = search(
        &mut server,
        serde_json::from_str(r#"{"query":"and","limit":1e0}"#).unwrap(),
    );
    assert_eq!(
        exponent_limit["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .len(),
        1
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
        json!({"query": "schema", "scope": "other"}),
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

#[test]
fn search_docs_scopes_package_candidates_and_orders_all_by_rank_then_uri() {
    let workspace = TempWorkspace::new("package-tool-scopes");
    write_documented_workspace(&workspace);
    let mut server = initialized_server(&workspace);

    let before = search(&mut server, json!({"query": "depfixture"}));
    assert!(
        before["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|result| !result["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-doc:///package/example%2Fdep/"))
    );

    let check = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(check["isError"], false, "{check:#}");

    let package = search(
        &mut server,
        json!({"query": "depfixture", "scope": "package", "limit": 20}),
    );
    let package_results = package["structuredContent"]["results"].as_array().unwrap();
    assert_eq!(package["structuredContent"]["scope"], "package");
    assert!(package_results.iter().all(|result| {
        result["uri"]
            .as_str()
            .unwrap()
            .starts_with("veln-doc:///package/example%2Fdep/snapshot/")
    }));
    assert!(package_results.iter().any(|result| {
        result["title"] == "Veln package documentation: example/dep"
            && result["summary"] == "Transport dependency documentation."
    }));

    let stdlib = search(
        &mut server,
        json!({"query": "depfixture", "scope": "stdlib"}),
    );
    assert_eq!(stdlib["structuredContent"]["results"], json!([]));

    let language = search(
        &mut server,
        json!({"query": "depfixture", "scope": "language"}),
    );
    assert!(
        language["structuredContent"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|result| !result["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-doc:///package/"))
    );

    let all = search(
        &mut server,
        json!({"query": "depfixture", "scope": "all", "limit": 50}),
    );
    let all_uris = result_uris(&all);
    let unique = all_uris.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), all_uris.len());
    assert!(
        unique
            .iter()
            .any(|uri| { uri.starts_with("veln-doc:///package/example%2Fdep/snapshot/") })
    );
    let mut sorted = all_uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(all_uris, sorted);
}

#[test]
fn stdlib_search_uses_package_catalog_fields_and_ranking() {
    let workspace = TempWorkspace::new("stdlib-tool-ranking");
    let mut server = initialized_server(&workspace);

    struct Case {
        query: &'static str,
        title: &'static str,
        excerpt: &'static str,
        uri_segment: &'static str,
        first_result: bool,
    }

    let cases = [
        Case {
            query: "std",
            title: "Veln package documentation: std",
            excerpt: "std",
            uri_segment: "/index",
            first_result: true,
        },
        Case {
            query: "prelude",
            title: "Veln package module: prelude",
            excerpt: "Veln package module: prelude",
            uri_segment: "/module/",
            first_result: true,
        },
        Case {
            query: "Veln package declaration: function byte",
            title: "Veln package declaration: function byte",
            excerpt: "Veln package declaration: function byte",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "standard-library",
            title: "Veln package declaration: function receive_frame_stream_id",
            excerpt: "standard-library",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "toolchain supplied",
            title: "Veln package documentation: std",
            excerpt: "Standard library APIs supplied by the Veln toolchain.",
            uri_segment: "/index",
            first_result: true,
        },
        Case {
            query: "fn byte(value: Int) -> Result<Byte, String>",
            title: "Veln package declaration: function byte",
            excerpt: "fn byte(value: Int) -> Result<Byte, String>",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "byte-oriented package APIs",
            title: "Veln package declaration: function byte",
            excerpt: "Builds a validated byte value for byte-oriented package APIs.",
            uri_segment: "/declaration/",
            first_result: true,
        },
    ];

    for case in cases {
        let limit = if case.first_result { 1 } else { 50 };
        let result = search(
            &mut server,
            json!({"query": case.query, "scope": "stdlib", "limit": limit}),
        );
        let results = result["structuredContent"]["results"].as_array().unwrap();
        assert_eq!(result["structuredContent"]["scope"], "stdlib");
        let matched = if case.first_result {
            &results[0]
        } else {
            results
                .iter()
                .find(|result| result["title"] == case.title)
                .unwrap_or_else(|| panic!("missing stdlib result for {}: {result:#}", case.query))
        };
        assert_eq!(matched["title"], case.title);
        assert_eq!(matched["excerpt"], case.excerpt);
        assert!(
            matched["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-doc:///package/std/snapshot/")
        );
        assert!(
            matched["uri"].as_str().unwrap().contains(case.uri_segment),
            "{result:#}"
        );
    }
}

#[test]
fn search_docs_all_orders_equal_rank_language_stdlib_and_package_candidates_by_uri() {
    let workspace = TempWorkspace::new("package-tool-cross-scope-order");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/overlap\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use prelude from \"example/overlap\"\n\nfn main() -> Int\n  prelude::answer()\nend\n",
    );
    write_workspace_with_named_dependency(
        &workspace,
        "example/overlap",
        "Aaa package documentation.",
        "sharedterm",
        "prelude",
        "Shared package docs.",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_language_resources(
        vec![veln_repo_language_reference::RenderedResource {
            uri: "veln-doc:///language/snapshot/test/index".to_string(),
            name: "language-index".to_string(),
            title: "Veln Language Reference".to_string(),
            description: None,
            mime_type: veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
            text: "# Veln Language Reference\n".to_string(),
        }],
        vec![LanguageTopic {
            uri: "veln-doc:///language/snapshot/test/topic/package-module-prelude".to_string(),
            id: "package-module-prelude".to_string(),
            title: "Veln package module: prelude".to_string(),
            summary: "Language overlap fixture.".to_string(),
            keywords: Vec::new(),
            body: String::new(),
        }],
    );
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    for scope in ["language", "stdlib", "package"] {
        let result = search(
            &mut server,
            json!({"query": "Veln package module: prelude", "scope": scope, "limit": 20}),
        );
        assert!(
            !result["structuredContent"]["results"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{scope} should have candidates"
        );
    }

    let all = search(
        &mut server,
        json!({"query": "Veln package module: prelude", "scope": "all", "limit": 20}),
    );
    let all_uris = result_uris(&all);
    let unique = all_uris.iter().collect::<BTreeSet<_>>();
    assert_eq!(unique.len(), all_uris.len());
    assert!(
        all_uris
            .iter()
            .any(|uri| uri.starts_with("veln-doc:///language/snapshot/test/topic/"))
    );
    assert!(
        all_uris
            .iter()
            .any(|uri| uri.starts_with("veln-doc:///package/std/snapshot/"))
    );
    assert!(
        all_uris
            .iter()
            .any(|uri| uri.starts_with("veln-doc:///package/example%2Foverlap/snapshot/"))
    );
    let mut sorted = all_uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(all_uris, sorted);
}

#[test]
fn package_search_uses_catalog_field_tiers_and_retains_distinct_snapshots() {
    let workspace = TempWorkspace::new("package-tool-ranking");
    write_documented_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    let title = search(
        &mut server,
        json!({"query": "Veln package declaration: function answer", "scope": "package"}),
    );
    assert_eq!(
        title["structuredContent"]["results"][0]["title"],
        "Veln package declaration: function answer"
    );

    let keyword = search(
        &mut server,
        json!({"query": "protocol", "scope": "package", "limit": 10}),
    );
    assert!(
        result_titles(&keyword)
            .iter()
            .any(|title| title.contains("api"))
    );

    let signature = search(
        &mut server,
        json!({"query": "answer Int", "scope": "package", "limit": 10}),
    );
    assert_eq!(
        signature["structuredContent"]["results"][0]["title"],
        "Veln package declaration: function answer"
    );
    assert_eq!(
        signature["structuredContent"]["results"][0]["excerpt"],
        "fn answer() -> Int"
    );

    let documentation = search(
        &mut server,
        json!({"query": "stable catalog text", "scope": "package"}),
    );
    assert_eq!(
        documentation["structuredContent"]["results"][0]["title"],
        "Veln package declaration: function answer"
    );

    let before = result_uris(&search(
        &mut server,
        json!({"query": "stable catalog text", "scope": "package", "limit": 20}),
    ));
    assert_eq!(before.len(), 1);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );
    assert_eq!(
        result_uris(&search(
            &mut server,
            json!({"query": "stable catalog text", "scope": "package", "limit": 20}),
        )),
        before
    );

    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    write_dependency(&workspace, "changed catalog text");
    refresh_workspace(&mut server);
    assert_eq!(
        server.definition_tool(&json!({"source":"main.veln","line":5,"column":3}))["isError"],
        false
    );
    let after = result_uris(&search(
        &mut server,
        json!({"query": "catalog text", "scope": "package", "limit": 20}),
    ));
    assert_eq!(after.len(), 2);
    assert!(before.iter().all(|uri| after.contains(uri)));
}

#[test]
fn read_doc_accepts_exact_package_documentation_reads_and_rejects_boundaries() {
    let workspace = TempWorkspace::new("package-tool-read");
    write_documented_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    let index_uri = package_search_uri(&mut server, "package", "example/dep");
    let index = assert_doc_tool_equals_resource(&mut server, &index_uri);
    let module_uri = linked_package_doc_uri(index["text"].as_str().unwrap(), "/module/");
    let module = assert_doc_tool_equals_resource(&mut server, &module_uri);
    let declaration_uri = linked_package_doc_uri(module["text"].as_str().unwrap(), "/declaration/");
    assert_doc_tool_equals_resource(&mut server, &declaration_uri);

    let source_uri = listed_dependency_source_uri(&mut server);
    let source_read = read_doc(&mut server, &source_uri);
    assert_eq!(source_read["isError"], true);
    assert_eq!(
        source_read["structuredContent"]["code"],
        "resource_not_found"
    );

    let wrong_digest = index_uri.replace(
        index_uri
            .split("/documentation/")
            .nth(1)
            .unwrap()
            .split('/')
            .next()
            .unwrap(),
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let missing = read_doc(&mut server, &wrong_digest);
    assert_eq!(missing["isError"], true);
    assert_eq!(missing["structuredContent"]["code"], "resource_not_found");
    assert!(missing["structuredContent"].get("text").is_none());
}

#[test]
fn status_only_package_documentation_is_readable_but_not_searchable() {
    let workspace = TempWorkspace::new("package-tool-status");
    write_workspace_with_failed_dependency_documentation(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    let status_uri = listed_package_documentation_status_uri(&mut server);
    let status = assert_doc_tool_equals_resource(&mut server, &status_uri);
    assert!(
        status["text"]
            .as_str()
            .unwrap()
            .contains("package_doc.unresolved_schema_reference")
    );

    let search_result = search(
        &mut server,
        json!({"query": "unresolved_schema_reference", "scope": "package"}),
    );
    assert_eq!(search_result["structuredContent"]["results"], json!([]));
    let unpublished_index = status_uri.replace("/status", "/index");
    let unpublished = read_doc(&mut server, &unpublished_index);
    assert_eq!(unpublished["isError"], true);
    assert_eq!(
        unpublished["structuredContent"]["code"],
        "resource_not_found"
    );
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

fn refresh_workspace(server: &mut Server) {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"refresh","method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
}

fn result_uris(response: &Value) -> Vec<String> {
    response["structuredContent"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["uri"].as_str().unwrap().to_string())
        .collect()
}

fn result_titles(response: &Value) -> Vec<String> {
    response["structuredContent"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["title"].as_str().unwrap().to_string())
        .collect()
}

fn package_search_uri(server: &mut Server, scope: &str, query: &str) -> String {
    search(server, json!({"query": query, "scope": scope, "limit": 1}))["structuredContent"]
        ["results"][0]["uri"]
        .as_str()
        .unwrap()
        .to_string()
}

fn assert_doc_tool_equals_resource(server: &mut Server, uri: &str) -> Value {
    let resource = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":"resource-read","method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap();
    let doc = read_doc(server, uri);
    assert_eq!(doc["isError"], false, "{doc:#}");
    let structured = &doc["structuredContent"];
    assert_eq!(structured["uri"], uri);
    assert_eq!(
        structured["mimeType"],
        resource["result"]["contents"][0]["mimeType"]
    );
    assert_eq!(
        structured["text"],
        resource["result"]["contents"][0]["text"]
    );
    structured.clone()
}

fn linked_package_doc_uri(text: &str, segment: &str) -> String {
    let mut rest = text;
    while let Some(start) = rest.find("veln-doc:///package/") {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        let uri = &after_start[..end];
        if uri.contains(segment) {
            return uri.to_string();
        }
        rest = &after_start[end..];
    }
    panic!("missing package documentation URI containing {segment}");
}

fn listed_dependency_source_uri(server: &mut Server) -> String {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"listed-source","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with("veln-pkg:///example%2Fdep/snapshot/")
                .then(|| uri.to_string())
        })
        .unwrap()
}

fn listed_package_documentation_status_uri(server: &mut Server) -> String {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"listed-status","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with("veln-doc:///package/example%2Fdep/snapshot/")
                .then(|| uri.to_string())
        })
        .unwrap()
}

fn write_documented_workspace(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use api from \"example/dep\"\n\nfn main() -> Int\n  api::answer()\nend\n",
    );
    write_dependency(workspace, "stable catalog text");
}

fn write_dependency(workspace: &TempWorkspace, doc_line: &str) {
    write_workspace_with_named_dependency(
        workspace,
        "example/dep",
        "Transport dependency documentation.",
        "protocol, depfixture",
        "api",
        doc_line,
    );
}

fn write_workspace_with_named_dependency(
    workspace: &TempWorkspace,
    identity: &str,
    description: &str,
    keywords: &str,
    module_name: &str,
    doc_line: &str,
) {
    workspace.write(
        "vendor/dep/veln.toml",
        &format!(
            concat!(
                "[package]\n",
                "name = \"{}\"\n",
                "description = \"{}\"\n",
                "keywords = \"{}\"\n\n",
                "[lib]\n",
                "exports = [\"{}.veln\"]\n",
            ),
            identity, description, keywords, module_name
        ),
    );
    workspace.write(
        &format!("vendor/dep/{module_name}.veln"),
        &format!(
            "## Module {module_name} handles packet transport.\n\npub fn byte() -> Int\n  1\nend\n\n## {doc_line}\npub fn answer() -> Int\n  1\nend\n"
        ),
    );
}

fn write_workspace_with_failed_dependency_documentation(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use api from \"example/dep\"\n\nfn main() -> Int\n  api::answer()\nend\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"api.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/api.veln",
        concat!(
            "## Missing schema {@schema Missing}.\n",
            "pub fn answer() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
}
