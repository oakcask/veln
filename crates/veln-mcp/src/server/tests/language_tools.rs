use super::*;
use crate::language_resources::LanguageTopic;
use std::collections::BTreeSet;
use veln_analysis::CapturedDependencyProject;
use veln_project::{PackageSnapshotSource, Project, parse_manifest_text};
use veln_source::SourceFile;

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
    let workspace = TempWorkspace::new("stdlib-tool-fields");
    let mut server = initialized_server(&workspace);
    install_test_standard_library_docs(&mut server);

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
            query: "stdlib-keyword-fixture",
            title: "Veln package declaration: function byte_fixture",
            excerpt: "stdlib-keyword-fixture",
            uri_segment: "/declaration/",
            first_result: false,
        },
        Case {
            query: "module-doc-token",
            title: "Veln package module: prelude",
            excerpt: "Fixture module documentation mentions module-doc-token.",
            uri_segment: "/module/",
            first_result: true,
        },
        Case {
            query: "Veln package declaration: function byte_fixture",
            title: "Veln package declaration: function byte_fixture",
            excerpt: "Veln package declaration: function byte_fixture",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "signatureonly",
            title: "Veln package declaration: function signature_best",
            excerpt: "fn signature_best(signatureonly: Int) -> Int",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "constructor-doc-token",
            title: "Veln package declaration: type FixtureEnvelope",
            excerpt: "Constructor docs mention constructor-doc-token.",
            uri_segment: "/declaration/",
            first_result: true,
        },
        Case {
            query: "value >= 7",
            title: "Veln package declaration: function contract_best",
            excerpt: "value >= 7",
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
fn package_search_ranking_tiers_win_against_lower_tier_candidates() {
    let workspace = TempWorkspace::new("package-tool-tier-ranking");
    write_ranking_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    struct Case {
        query: &'static str,
        winner: &'static str,
        lower_tier_match: &'static str,
    }

    let cases = [
        Case {
            query: "example/ranking",
            winner: "Veln package documentation: example/ranking",
            lower_tier_match: "Veln package declaration: function lower_identity_doc",
        },
        Case {
            query: "Veln package declaration: function prefix",
            winner: "Veln package declaration: function prefix_best",
            lower_tier_match: "Veln package declaration: function lower_pref_doc",
        },
        Case {
            query: "rankthree",
            winner: "Veln package declaration: function rankthree",
            lower_tier_match: "Veln package declaration: function lower_three_doc",
        },
        Case {
            query: "signatureonly",
            winner: "Veln package declaration: function signature_best",
            lower_tier_match: "Veln package declaration: function lower_signature_doc",
        },
    ];

    for case in cases {
        let result = search(
            &mut server,
            json!({"query": case.query, "scope": "package", "limit": 20}),
        );
        let titles = result_titles(&result);
        assert_eq!(titles[0], case.winner, "{}: {result:#}", case.query);
        assert!(
            titles.iter().any(|title| title == case.lower_tier_match),
            "{}: {result:#}",
            case.query
        );
    }

    let rank_five = search(
        &mut server,
        json!({"query": "declaration-doc-only", "scope": "package", "limit": 20}),
    );
    assert_eq!(
        rank_five["structuredContent"]["results"][0]["title"],
        "Veln package declaration: function docs_only"
    );
}

#[test]
fn package_search_covers_all_package_fields_and_exclusion_boundaries() {
    let workspace = TempWorkspace::new("package-tool-field-boundaries");
    write_ranking_workspace(&workspace);
    write_workspace_package_documentation_only(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );

    let module_uri = package_search_uri(&mut server, "package", "rankmodule");
    let module_id = module_uri.rsplit('/').next().unwrap().to_string();
    let declaration_uri = package_search_uri(
        &mut server,
        "package",
        "Veln package declaration: function rankthree",
    );
    let declaration_id = declaration_uri.rsplit('/').next().unwrap().to_string();

    struct FieldCase {
        query: String,
        title: &'static str,
    }

    let cases = [
        FieldCase {
            query: "example/ranking".to_string(),
            title: "Veln package documentation: example/ranking",
        },
        FieldCase {
            query: "rankmodule".to_string(),
            title: "Veln package module: rankmodule",
        },
        FieldCase {
            query: module_id,
            title: "Veln package module: rankmodule",
        },
        FieldCase {
            query: "rankthree".to_string(),
            title: "Veln package declaration: function rankthree",
        },
        FieldCase {
            query: declaration_id,
            title: "Veln package declaration: function rankthree",
        },
        FieldCase {
            query: "pkg-keyword-token".to_string(),
            title: "Veln package declaration: function prefix_best",
        },
        FieldCase {
            query: "ranking package description token".to_string(),
            title: "Veln package documentation: example/ranking",
        },
        FieldCase {
            query: "signatureonly".to_string(),
            title: "Veln package declaration: function signature_best",
        },
        FieldCase {
            query: "module-doc-token".to_string(),
            title: "Veln package module: rankmodule",
        },
        FieldCase {
            query: "declaration-doc-only".to_string(),
            title: "Veln package declaration: function docs_only",
        },
        FieldCase {
            query: "constructor-doc-token".to_string(),
            title: "Veln package declaration: type FixtureEnvelope",
        },
        FieldCase {
            query: "value >= 7".to_string(),
            title: "Veln package declaration: function contract_best",
        },
    ];

    for case in cases {
        let result = search(
            &mut server,
            json!({"query": case.query, "scope": "package", "limit": 20}),
        );
        assert_eq!(
            result["structuredContent"]["results"][0]["title"], case.title,
            "{result:#}"
        );
    }

    for excluded in [
        "source-only-token",
        "doctest_only_token",
        "output-only-token",
        "workspace-package-doc-token",
    ] {
        let result = search(
            &mut server,
            json!({"query": excluded, "scope": "all", "limit": 50}),
        );
        assert_eq!(
            result["structuredContent"]["results"],
            json!([]),
            "{excluded}: {result:#}"
        );
    }

    write_workspace_with_failed_dependency_documentation(&workspace);
    refresh_workspace(&mut server);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );
    let status = search(
        &mut server,
        json!({"query": "unresolved_schema_reference", "scope": "package"}),
    );
    assert_eq!(status["structuredContent"]["results"], json!([]));
    let diagnostic_text = search(
        &mut server,
        json!({"query": "DiagnosticOnlyToken", "scope": "package"}),
    );
    assert_eq!(diagnostic_text["structuredContent"]["results"], json!([]));
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
fn package_tool_state_is_preserved_across_capacity_and_capture_failures() {
    let workspace = TempWorkspace::new("package-tool-failure-state");
    write_documented_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let before_capacity = package_tool_state(&mut server, "std", "std");

    let boundary = (0..255)
        .map(|index| synthetic_dependency_project(&format!("example/full{index}"), "body"))
        .collect::<Vec<_>>();
    server
        .language_resources
        .admit_dependencies(&boundary)
        .unwrap();
    let after_boundary = package_tool_state(&mut server, "std", "std");
    assert_eq!(after_boundary, before_capacity);

    let capacity = server
        .language_resources
        .admit_dependencies(&[synthetic_dependency_project(
            "example/overflow",
            "overflow rejected text",
        )]);
    assert_eq!(
        capacity.unwrap_err(),
        crate::language_resources::ResourceCapacityError
    );
    assert_eq!(
        package_tool_state(&mut server, "std", "std"),
        before_capacity
    );
    let rejected = search(
        &mut server,
        json!({"query": "overflow rejected text", "scope": "package", "limit": 50}),
    );
    assert_eq!(rejected["structuredContent"]["results"], json!([]));

    let capture_workspace = TempWorkspace::new("package-tool-capture-failure-state");
    write_documented_workspace(&capture_workspace);
    let mut capture_server = initialized_server(&capture_workspace);
    assert_eq!(
        capture_server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );
    let before_capture =
        package_tool_state(&mut capture_server, "example/dep", "stable catalog text");
    fs::remove_dir_all(capture_workspace.path("vendor/dep")).unwrap();
    write_dependency(&capture_workspace, "capture rejected text");
    refresh_workspace(&mut capture_server);
    let changed_path = capture_workspace.path("vendor/dep/api.veln");
    let mut capture_change = 0;
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        capture_change += 1;
        fs::write(
            &changed_path,
            format!(
                concat!(
                    "## Changed during capture mentions capture rejected text.\n",
                    "pub fn answer() -> Int\n",
                    "\t{}\n",
                    "end\n",
                ),
                capture_change + 2
            ),
        )
        .unwrap();
    });
    let capture = capture_server.check_project_tool(&json!({"project":"."}));
    assert_eq!(capture["isError"], true, "{capture:#}");
    assert_eq!(capture["structuredContent"]["code"], "snapshot_changed");
    assert_eq!(
        package_tool_state(&mut capture_server, "example/dep", "stable catalog text"),
        before_capture
    );
    let rejected = search(
        &mut capture_server,
        json!({"query": "capture rejected text", "scope": "package", "limit": 50}),
    );
    assert_eq!(rejected["structuredContent"]["results"], json!([]));
}

#[test]
fn package_tool_state_preserves_exact_reads_across_refresh_and_replacement() {
    let workspace = TempWorkspace::new("package-tool-refresh-state");
    write_documented_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.check_project_tool(&json!({"project":"."}))["isError"],
        false
    );
    let before = package_tool_state(&mut server, "example/dep", "stable catalog text");

    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    write_dependency(&workspace, "changed catalog text");
    refresh_workspace(&mut server);
    assert_eq!(
        package_tool_state(&mut server, "example/dep", "stable catalog text"),
        before
    );

    let definition = server.definition_tool(&json!({"source":"main.veln","line":5,"column":3}));
    assert_eq!(definition["isError"], false, "{definition:#}");
    assert_eq!(
        search(
            &mut server,
            json!({"query": "stable catalog text", "scope": "package", "limit": 50}),
        )["structuredContent"],
        before["search"]
    );
    let after_reads = package_doc_exact_reads(&mut server, "example/dep");
    for read in before["reads"].as_array().unwrap() {
        assert!(after_reads.iter().any(|after| after == read), "{read:#}");
    }
    let changed = package_tool_state(&mut server, "example/dep", "changed catalog text");
    assert_eq!(changed["search"]["results"].as_array().unwrap().len(), 1);
    assert!(changed["reads"].as_array().unwrap().iter().any(|read| {
        read["text"]
            .as_str()
            .unwrap()
            .contains("changed catalog text")
    }));
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
    let before = package_tool_state(&mut server, "example/dep", "stable catalog text");

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

    let (snapshot_digest, doc_digest) = package_documentation_uri_digests(&index_uri);
    let wrong_digest = index_uri.replace(
        index_uri
            .split("/documentation/")
            .nth(1)
            .unwrap()
            .split('/')
            .next()
            .unwrap(),
        WRONG_DIGEST,
    );
    let rejection_cases = [
        (
            "unknown",
            format!(
                "veln-doc:///package/missing%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/index"
            ),
        ),
        (
            "noncanonical-scheme",
            index_uri.replacen("veln-doc", "VELN-doc", 1),
        ),
        ("noncanonical-path", index_uri.replace("/index", "/Index")),
        (
            "wrong-snapshot",
            index_uri.replace(snapshot_digest, WRONG_DIGEST),
        ),
        ("wrong-documentation-digest", wrong_digest),
        (
            "missing-module",
            format!(
                "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/module/missing"
            ),
        ),
        (
            "missing-declaration",
            format!(
                "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/declaration/missing"
            ),
        ),
        ("unpublished-status", index_uri.replace("/index", "/status")),
        (
            "unpublished-module",
            module_uri.replace("/module/", "/module/missing-"),
        ),
        (
            "unpublished-declaration",
            declaration_uri.replace("/declaration/", "/declaration/missing-"),
        ),
        ("query-bearing", format!("{index_uri}?x=1")),
    ];
    for (case, uri) in rejection_cases {
        assert_read_doc_resource_not_found(&mut server, case, &uri);
    }
    assert_eq!(
        package_tool_state(&mut server, "example/dep", "stable catalog text"),
        before
    );
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
    assert_read_doc_resource_not_found(&mut server, "unpublished-index", &unpublished_index);
    assert_read_doc_resource_not_found(
        &mut server,
        "unpublished-module",
        &status_uri.replace("/status", "/module/missing"),
    );
    assert_read_doc_resource_not_found(
        &mut server,
        "unpublished-declaration",
        &status_uri.replace("/status", "/declaration/missing"),
    );
}

const WRONG_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

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

fn assert_read_doc_resource_not_found(server: &mut Server, case: &str, uri: &str) {
    let missing = read_doc(server, uri);
    assert_eq!(missing["isError"], true, "{case}: {missing:#}");
    assert_eq!(
        missing["structuredContent"]["code"], "resource_not_found",
        "{case}: {missing:#}"
    );
    assert_eq!(
        missing["structuredContent"]["details"]["uri"], uri,
        "{case}: {missing:#}"
    );
    assert!(
        missing["structuredContent"].get("text").is_none(),
        "{case}: {missing:#}"
    );
}

fn package_tool_state(server: &mut Server, identity: &str, query: &str) -> Value {
    let search = search(
        server,
        json!({"query": query, "scope": "package", "limit": 50}),
    )["structuredContent"]
        .clone();
    let exact_reads = package_doc_exact_reads(server, identity);
    json!({"search": search, "reads": exact_reads})
}

fn package_doc_exact_reads(server: &mut Server, identity: &str) -> Vec<Value> {
    let prefix = format!(
        "veln-doc:///package/{}/snapshot/",
        identity.replace('/', "%2F")
    );
    let mut uris = server
        .handle_request(
            json!({"jsonrpc":"2.0","id":"package-doc-state-list","method":"resources/list"}),
        )
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect::<BTreeSet<_>>();
    let mut index_linked = Vec::new();
    for uri in &uris {
        let read = assert_doc_tool_equals_resource(server, uri);
        index_linked.extend(
            extract_package_doc_uris(read["text"].as_str().unwrap())
                .into_iter()
                .filter(|uri| uri.contains("/module/") || uri.contains("/declaration/")),
        );
    }
    uris.extend(index_linked);
    let mut module_linked = Vec::new();
    for uri in uris.iter().filter(|uri| uri.contains("/module/")) {
        let read = assert_doc_tool_equals_resource(server, uri);
        module_linked.extend(
            extract_package_doc_uris(read["text"].as_str().unwrap())
                .into_iter()
                .filter(|uri| uri.contains("/declaration/")),
        );
    }
    uris.extend(module_linked);
    uris.into_iter()
        .map(|uri| assert_doc_tool_equals_resource(server, &uri))
        .collect()
}

fn package_documentation_uri_digests(index_uri: &str) -> (&str, &str) {
    let rest = index_uri
        .strip_prefix("veln-doc:///package/example%2Fdep/snapshot/")
        .unwrap();
    let mut parts = rest.split('/');
    let snapshot_digest = parts.next().unwrap();
    assert_eq!(parts.next(), Some("documentation"));
    let documentation_digest = parts.next().unwrap();
    (snapshot_digest, documentation_digest)
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

fn extract_package_doc_uris(text: &str) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("veln-doc:///package/") {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        uris.insert(after_start[..end].to_string());
        rest = &after_start[end..];
    }
    uris
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

fn install_test_standard_library_docs(server: &mut Server) {
    server.language_resources.replace_test_standard_library(
        concat!(
            "[package]\n",
            "name = \"std\"\n",
            "description = \"Synthetic stdlib fixture description.\"\n",
            "keywords = \"stdlib-keyword-fixture\"\n",
            "[lib]\n",
            "exports = [\"prelude.veln\"]\n",
        ),
        [PackageSnapshotSource::new(
            "prelude.veln",
            concat!(
                "## Fixture module documentation mentions module-doc-token.\n",
                "mod prelude\n",
                "\n",
                "## Type docs.\n",
                "pub type FixtureEnvelope\n",
                "\t## Constructor docs mention constructor-doc-token.\n",
                "\tpub Wrap(value: Int)\n",
                "end\n",
                "\n",
                "pub fn byte_fixture(value: Int) -> Int\n",
                "\tvalue\n",
                "end\n",
                "\n",
                "pub fn signature_best(signatureonly: Int) -> Int\n",
                "\tsignatureonly\n",
                "end\n",
                "\n",
                "pub fn contract_best(value: Int) -> output: Int\n",
                "\trequire value >= 7\n",
                "\tvalue\n",
                "end\n",
            )
            .as_bytes(),
        )],
    );
}

fn write_ranking_workspace(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/ranking\"]\npath = \"vendor/ranking\"\n",
    );
    workspace.write(
        "main.veln",
        "use rankmodule from \"example/ranking\"\n\nfn main() -> Int\n  rankmodule::rankthree()\nend\n",
    );
    workspace.write(
        "vendor/ranking/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example/ranking\"\n",
            "description = \"ranking package description token\"\n",
            "keywords = \"pkg-keyword-token\"\n",
            "[lib]\n",
            "exports = [\"rankmodule.veln\"]\n",
        ),
    );
    workspace.write(
        "vendor/ranking/rankmodule.veln",
        concat!(
            "## Module docs mention module-doc-token.\n",
            "mod rankmodule\n",
            "\n",
            "# source-only-token\n",
            "\n",
            "## Type docs.\n",
            "pub type FixtureEnvelope\n",
            "\t## Constructor docs mention constructor-doc-token.\n",
            "\tpub Wrap(value: Int)\n",
            "end\n",
            "\n",
            "pub fn rankthree() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "## Documentation mentions example/ranking.\n",
            "pub fn lower_identity_doc() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "pub fn prefix_best() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "## Documentation mentions Veln package declaration: function prefix.\n",
            "pub fn lower_pref_doc() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "## Documentation mentions rankthree.\n",
            "pub fn lower_three_doc() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "pub fn signature_best(signatureonly: Int) -> Int\n",
            "\tsignatureonly\n",
            "end\n",
            "\n",
            "## Documentation mentions signatureonly.\n",
            "pub fn lower_signature_doc() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "## Documentation mentions declaration-doc-only.\n",
            "## ```veln\n",
            "## fn doctest_only_token() -> Int\n",
            "## \t1\n",
            "## end\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## output-only-token\n",
            "## ```\n",
            "pub fn docs_only() -> Int\n",
            "\t1\n",
            "end\n",
            "\n",
            "pub fn contract_best(value: Int) -> output: Int\n",
            "\trequire value >= 7\n",
            "\tvalue\n",
            "end\n",
        ),
    );
}

fn write_workspace_package_documentation_only(workspace: &TempWorkspace) {
    workspace.write(
        "package_only.veln",
        "## Workspace docs mention workspace-package-doc-token.\npub fn local_doc() -> Int\n  1\nend\n",
    );
}

fn synthetic_dependency_project(identity: &str, body: &str) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: identity.to_string(),
        source: format!("vendor/{identity}"),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new(
                "dep.veln",
                synthetic_dependency_source(body),
            )],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &synthetic_dependency_manifest(identity),
            )),
        }),
    }
}

fn synthetic_dependency_manifest(identity: &str) -> String {
    format!("[package]\nname = \"{identity}\"\n[lib]\nexports = [\"dep.veln\"]\n")
}

fn synthetic_dependency_source(body: &str) -> String {
    format!("pub fn value() -> Int\n\t# {body}\n\t1\nend\n")
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
            "## Missing schema {@schema DiagnosticOnlyToken}.\n",
            "pub fn answer() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
}
