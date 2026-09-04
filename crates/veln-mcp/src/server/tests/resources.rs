use super::*;
use std::collections::BTreeSet;
use veln_analysis::CapturedDependencyProject;
use veln_project::{Project, parse_manifest_text};
use veln_source::SourceFile;

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
fn resource_uris_are_exact_and_state_is_preserved_across_tools() {
    let workspace = TempWorkspace::new("resource-state");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let mut server = initialized_server(&workspace);
    let before = standard_library_resource_state(&mut server);

    exercise_resource_state_preserving_operations(&mut server);

    let after = standard_library_resource_state(&mut server);
    assert_eq!(after, before);
}

#[test]
fn successful_saved_project_tools_admit_dependency_resources_until_shutdown() {
    let workspace = TempWorkspace::new("dependency-resource-lifecycle");
    write_workspace_with_dependency(&workspace, "before");
    let mut server = initialized_server(&workspace);

    let check = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(check["isError"], false, "{check:#}");
    let before = dependency_resource_state(&mut server, "example/dep", "dep.veln");

    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", &dependency_source("after"));
    refresh_workspace(&mut server);
    let after_refresh = dependency_resource_state(&mut server, "example/dep", "dep.veln");
    assert_eq!(after_refresh, before);

    let definition = server.definition_tool(&json!({"source":"main.veln","line":5,"column":3}));
    assert_eq!(definition["isError"], false, "{definition:#}");
    let after_definition = dependency_resource_states(&mut server, "example/dep", "dep.veln");
    assert_eq!(
        dependency_resource_texts(&after_definition),
        BTreeSet::from([dependency_source("before"), dependency_source("after")])
    );

    let uris = dependency_resource_uris(&mut server, "example/dep");
    assert_eq!(uris.len(), 3);
    for uri in uris {
        assert_eq!(
            read_resource(&mut server, 60, &uri)["result"]["contents"][0]["uri"],
            uri
        );
    }
}

#[test]
fn each_successful_saved_project_tool_admits_dependency_resources() {
    struct Case {
        name: &'static str,
        call: fn(&mut Server) -> Value,
    }

    let cases = [
        Case {
            name: "check_project",
            call: |server| server.check_project_tool(&json!({"project":"."})),
        },
        Case {
            name: "definition",
            call: |server| {
                server.definition_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
        Case {
            name: "references",
            call: |server| {
                server.references_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_workspace_with_dependency(&workspace, "admitted");
        let mut server = initialized_server(&workspace);
        let before = all_resource_state(&mut server);

        let result = (case.call)(&mut server);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_ne!(all_resource_state(&mut server), before, "{}", case.name);

        let states = dependency_resource_states(&mut server, "example/dep", "dep.veln");
        assert_eq!(states.len(), 1, "{}: {states:#?}", case.name);
        assert_eq!(states[0]["read"]["text"], dependency_source("admitted"));
    }
}

#[test]
fn dependency_resource_admission_deduplicates_and_preserves_state_on_failures() {
    let workspace = TempWorkspace::new("dependency-resource-dedup");
    write_workspace_with_dependency(&workspace, "stable");
    let mut server = initialized_server(&workspace);

    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let listed_once = dependency_resource_uris(&mut server, "example/dep");
    assert_eq!(listed_once.len(), 2);
    let before = all_resource_state(&mut server);

    let repeat = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(repeat["isError"], false, "{repeat:#}");
    assert_eq!(
        dependency_resource_uris(&mut server, "example/dep"),
        listed_once
    );

    let invalid_position =
        server.definition_tool(&json!({"source":"main.veln","line":200,"column":1}));
    assert_eq!(invalid_position["isError"], true);
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    assert_eq!(all_resource_state(&mut server), before);

    let missing = read_resource(
        &mut server,
        70,
        "veln-pkg:///example%2Fdep/snapshot/missing/dep.veln",
    );
    assert_eq!(missing["error"]["data"]["code"], "resource_not_found");
}

#[test]
fn dependency_resource_capacity_is_atomic() {
    let mut resources = LanguageResources::checked().unwrap();
    let boundary = (0..255)
        .map(|index| synthetic_dependency_project(&format!("example/dep{index}"), "body"))
        .collect::<Vec<_>>();
    resources.admit_dependencies(&boundary).unwrap();
    let before = resources.list_result();

    let over_capacity = vec![synthetic_dependency_project("example/overflow", "body")];
    let error = resources.admit_dependencies(&over_capacity).unwrap_err();
    assert_eq!(error, crate::language_resources::ResourceCapacityError);
    assert_eq!(resources.list_result(), before);

    let mut resources = LanguageResources::checked().unwrap();
    let boundary = (0..254)
        .map(|index| synthetic_dependency_project(&format!("example/multi{index}"), "body"))
        .collect::<Vec<_>>();
    resources.admit_dependencies(&boundary).unwrap();
    let before = resources.list_result();

    let over_capacity = vec![
        synthetic_dependency_project("example/overflow-a", "body"),
        synthetic_dependency_project("example/overflow-b", "body"),
    ];
    let error = resources.admit_dependencies(&over_capacity).unwrap_err();
    assert_eq!(error, crate::language_resources::ResourceCapacityError);
    assert_eq!(resources.list_result(), before);
    assert!(!resource_uri_prefix_is_listed(
        &resources,
        "example/overflow-a"
    ));
    assert!(!resource_uri_prefix_is_listed(
        &resources,
        "example/overflow-b"
    ));
}

#[test]
fn saved_project_capacity_failures_match_advertised_result_schemas() {
    struct Case {
        name: &'static str,
        call: fn(&mut Server) -> Value,
    }

    let cases = [
        Case {
            name: "check_project",
            call: |server| server.check_project_tool(&json!({"project":"."})),
        },
        Case {
            name: "definition",
            call: |server| {
                server.definition_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
        Case {
            name: "references",
            call: |server| {
                server.references_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_workspace_with_dependency(&workspace, "overflow");
        let mut server = initialized_server(&workspace);
        let boundary = (0..255)
            .map(|index| synthetic_dependency_project(&format!("example/full{index}"), "body"))
            .collect::<Vec<_>>();
        server
            .language_resources
            .admit_dependencies(&boundary)
            .unwrap();
        let before = all_resource_state(&mut server);

        let result = (case.call)(&mut server);
        assert_eq!(result["isError"], true, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["code"], "resource_capacity",
            "{}: {result:#}",
            case.name
        );
        assert!(
            schema::tool(case.name)
                .unwrap()
                .accepts_result(&result["structuredContent"]),
            "{}: {result:#}",
            case.name
        );
        assert_eq!(all_resource_state(&mut server), before, "{}", case.name);
    }
}

fn exercise_resource_state_preserving_operations(server: &mut Server) {
    refresh_workspace(server);
    check_project(server);
    assert_failed_refresh_preserves_resources(server);
    assert_doc_tools_preserve_resources(server);
    assert_failed_resource_requests_preserve_resources(server);
}

fn refresh_workspace(server: &mut Server) {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
}

fn check_project(server: &mut Server) {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"check_project","arguments":{"project":"."}}}))
        .unwrap();
}

fn assert_failed_refresh_preserves_resources(server: &mut Server) {
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
}

fn assert_doc_tools_preserve_resources(server: &mut Server) {
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
}

fn assert_failed_resource_requests_preserve_resources(server: &mut Server) {
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

#[test]
fn dependency_resource_rejections_are_exact_and_preserve_state() {
    let workspace = TempWorkspace::new("dependency-resource-rejections");
    write_workspace_with_dependency(&workspace, "retained");
    let mut server = initialized_server(&workspace);
    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");

    let before = all_resource_state(&mut server);
    let dep_uri = dependency_resource_uris(&mut server, "example/dep")
        .into_iter()
        .find(|uri| uri.ends_with("/dep.veln"))
        .unwrap();
    let digest = dep_uri
        .strip_prefix("veln-pkg:///example%2Fdep/snapshot/")
        .unwrap()
        .split('/')
        .next()
        .unwrap();
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let rejected = [
        format!("veln-pkg:///missing%2Fdep/snapshot/{digest}/dep.veln"),
        format!("veln-pkg:///example%2Fdep/snapshot/{wrong_digest}/dep.veln"),
        format!("veln-pkg:///example%2Fdep/snapshot/{digest}/missing.veln"),
        format!("veln-pkg:///example%2Fdep/snapshot/{digest}/dep_test.veln"),
        format!("veln-pkg:/example%2Fdep/snapshot/{digest}/dep.veln"),
        dep_uri.replacen("veln-pkg", "VELN-pkg", 1),
        dep_uri.replace("dep.veln", "dep%2Eveln"),
        format!("{dep_uri}?x=1"),
    ];

    assert_unknown_resource_reads_rejected(&mut server, rejected);
    assert_eq!(all_resource_state(&mut server), before);
    assert_eq!(
        read_resource(&mut server, 80, &dep_uri)["result"]["contents"][0]["text"],
        dependency_source("retained")
    );
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

fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"all-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

fn dependency_resource_state(server: &mut Server, identity: &str, name: &str) -> Value {
    dependency_resource_states(server, identity, name)
        .into_iter()
        .next()
        .unwrap()
}

fn dependency_resource_states(server: &mut Server, identity: &str, name: &str) -> Vec<Value> {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"dep-list","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|resource| {
            resource["uri"].as_str().unwrap().starts_with(&format!(
                "veln-pkg:///{}/snapshot/",
                identity.replace('/', "%2F")
            )) && resource["name"] == name
        })
        .cloned()
        .collect::<Vec<_>>();
    resources
        .into_iter()
        .map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            let read = read_resource(server, 50, uri)["result"]["contents"][0].clone();
            json!({"metadata": resource, "read": read})
        })
        .collect()
}

fn dependency_resource_texts(states: &[Value]) -> BTreeSet<String> {
    states
        .iter()
        .map(|state| state["read"]["text"].as_str().unwrap().to_string())
        .collect()
}

fn dependency_resource_uris(server: &mut Server, identity: &str) -> Vec<String> {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"dep-uris","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect()
}

fn resource_uri_prefix_is_listed(resources: &LanguageResources, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    resources.list_result()["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

fn write_workspace_with_dependency(workspace: &TempWorkspace, body: &str) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\nend\n",
    );
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", &dependency_source(body));
    workspace.write(
        "vendor/dep/private.veln",
        "fn private_value() -> Int\n  1\nend\n",
    );
    workspace.write(
        "vendor/dep/dep_test.veln",
        "test excluded() -> Int\n  1\nend\n",
    );
}

fn dependency_manifest(identity: &str) -> String {
    format!("[package]\nname = \"{identity}\"\n\n[lib]\nexports = [\"dep.veln\"]\n")
}

fn dependency_source(body: &str) -> String {
    format!("pub fn value() -> Int\n  # {body}\n  1\nend\n")
}

fn synthetic_dependency_project(identity: &str, body: &str) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: identity.to_string(),
        source: format!("vendor/{identity}"),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new("dep.veln", dependency_source(body))],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &dependency_manifest(identity),
            )),
        }),
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
