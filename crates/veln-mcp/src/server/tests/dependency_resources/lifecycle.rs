use super::*;

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
                server.references_tool(&json!({
                    "source":"main.veln","line":4,"column":8,"include_declaration":true
                }))
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
fn saved_project_dependency_resources_list_with_complete_sorted_metadata() {
    let workspace = TempWorkspace::new("complete-metadata");
    write_workspace_with_dependency(&workspace, "listed");
    let mut server = initialized_server_with_embedded_resources(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let response = server
        .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
        .unwrap();
    let resources = response["result"]["resources"].as_array().unwrap();
    let mut expected = expected_resource_metadata();
    expected.extend(expected_dependency_resource_metadata(
        "example/dep",
        [
            ("dep.veln", dependency_source("listed")),
            (
                "private.veln",
                "fn private_value() -> Int\n  1\nend\n".to_string(),
            ),
        ],
    ));
    expected.extend(expected_dependency_documentation_metadata(
        "example/dep",
        [
            ("dep.veln", dependency_source("listed")),
            (
                "private.veln",
                "fn private_value() -> Int\n  1\nend\n".to_string(),
            ),
        ],
    ));
    sort_resource_metadata(&mut expected);

    assert_eq!(response["result"].get("nextCursor"), None);
    assert_eq!(resources, &expected);
    assert_resource_uris_are_unique(resources, "complete-metadata");
    assert_resources_are_sorted(resources);
}

#[test]
fn successful_dependency_documentation_resources_round_trip_from_rendered_result() {
    let workspace = TempWorkspace::new("dependency-doc-resources");
    write_workspace_with_dependency(&workspace, "documented");
    let mut server = initialized_server(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let listed = listed_resources(&mut server);
    let rendered = expected_dependency_documentation_resources(
        "example/dep",
        [
            ("dep.veln", dependency_source("documented")),
            (
                "private.veln",
                "fn private_value() -> Int\n  1\nend\n".to_string(),
            ),
        ],
    );

    for resource in &rendered {
        let read = read_resource(&mut server, 101, &resource.uri);
        assert_eq!(read["result"]["contents"][0]["uri"], resource.uri);
        assert_eq!(
            read["result"]["contents"][0]["mimeType"],
            resource.mime_type
        );
        assert_eq!(read["result"]["contents"][0]["text"], resource.text);
        assert_eq!(
            listed.iter().any(|listed| listed["uri"] == resource.uri),
            resource.listed,
            "{}",
            resource.uri
        );
    }

    let index = rendered.iter().find(|resource| resource.listed).unwrap();
    assert_eq!(index.name, "example-dep-documentation-index");
    assert!(index.text.contains("# Package Documentation: example/dep"));
    assert!(rendered.iter().any(|resource| !resource.listed
        && resource.uri.contains("/module/")
        && resource.text.contains("- Source path: dep.veln")));
    assert!(rendered.iter().any(|resource| !resource.listed
        && resource.uri.contains("/declaration/")
        && resource.text.contains("- Kind: function")));
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

fn exercise_resource_state_preserving_operations(server: &mut Server) {
    refresh_workspace(server);
    check_project(server);
    assert_failed_refresh_preserves_resources(server);
    assert_doc_tools_preserve_resources(server);
    assert_failed_resource_requests_preserve_resources(server);
}

pub(super) fn refresh_workspace(server: &mut Server) {
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
