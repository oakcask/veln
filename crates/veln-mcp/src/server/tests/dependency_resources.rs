use super::*;
use std::collections::BTreeSet;
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{
    PackageDocGeneratorContract, PackageDocResult, VirtualSourceCatalog,
    render_package_documentation,
};
use veln_project::{
    PackageIdentity, PackageSnapshotSource, Project, capture_embedded_package_snapshot,
    parse_manifest_text,
};
use veln_source::SourceFile;

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
fn saved_project_dependency_resources_list_with_complete_sorted_metadata() {
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
    let expected_base = expected_resource_metadata();

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_workspace_with_dependency(&workspace, "listed");
        let mut server = initialized_server_with_embedded_resources(&workspace);

        let result = (case.call)(&mut server);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"resources/list"}))
            .unwrap();
        let resources = response["result"]["resources"].as_array().unwrap();
        let mut expected = expected_base.clone();
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

        assert_eq!(response["result"].get("nextCursor"), None, "{}", case.name);
        assert_eq!(resources, &expected, "{}", case.name);
        assert_resource_uris_are_unique(resources, case.name);
        assert_resources_are_sorted(resources);
    }
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
fn failed_dependency_documentation_publishes_only_status_resource() {
    let workspace = TempWorkspace::new("dependency-doc-status");
    write_workspace_with_dependency_source(
        &workspace,
        concat!(
            "## Missing reference {@schema Missing}.\n",
            "pub fn value() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let listed = listed_resources(&mut server);
    let rendered = expected_dependency_documentation_resources(
        "example/dep",
        [
            (
                "dep.veln",
                concat!(
                    "## Missing reference {@schema Missing}.\n",
                    "pub fn value() -> Int\n",
                    "  1\n",
                    "end\n",
                )
                .to_string(),
            ),
            (
                "private.veln",
                "fn private_value() -> Int\n  1\nend\n".to_string(),
            ),
        ],
    );

    assert_eq!(rendered.len(), 1);
    let status = &rendered[0];
    assert_eq!(status.name, "example-dep-documentation-status");
    assert!(status.listed);
    assert!(status.uri.ends_with("/status"));
    assert!(listed.iter().any(|resource| resource["uri"] == status.uri));
    let read = read_resource(&mut server, 102, &status.uri);
    assert_eq!(read["result"]["contents"][0]["text"], status.text);
    assert!(
        status
            .text
            .contains("package_doc.unresolved_schema_reference")
    );

    let unpublished = status.uri.replace("/status", "/index");
    assert_unknown_resource_reads_rejected(&mut server, [unpublished]);
    assert!(!dependency_resource_uris(&mut server, "example/dep").is_empty());
}

#[test]
fn dependency_documentation_resource_rejections_are_exact() {
    let workspace = TempWorkspace::new("dependency-doc-rejections");
    write_workspace_with_dependency(&workspace, "retained-doc");
    let mut server = initialized_server(&workspace);
    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let before = all_resource_state(&mut server);

    let index_uri = listed_dependency_documentation_uri(&mut server, "example/dep");
    let (snapshot_digest, doc_digest) = documentation_uri_digests_for(&index_uri, "example%2Fdep");
    let declaration_uri = linked_dependency_declaration_uri(&mut server, &index_uri);
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let rejected = [
        index_uri.replace(snapshot_digest, wrong_digest),
        index_uri.replace(doc_digest, wrong_digest),
        declaration_uri.replace(declaration_uri.rsplit('/').next().unwrap(), "missing"),
        format!(
            "veln-doc:///package/missing%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/index"
        ),
        format!(
            "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/module/missing"
        ),
        format!(
            "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/status"
        ),
        index_uri.replacen("veln-doc", "VELN-doc", 1),
        index_uri.replace("/index", "/Index"),
        format!("{index_uri}?x=1"),
    ];

    assert_unknown_resource_reads_rejected(&mut server, rejected);
    assert_eq!(all_resource_state(&mut server), before);
    assert_eq!(
        read_resource(&mut server, 103, &index_uri)["result"]["contents"][0]["uri"],
        index_uri
    );
}

#[test]
fn dependency_documentation_snapshots_coexist_and_remain_retained() {
    let workspace = TempWorkspace::new("dependency-doc-coexistence");
    write_workspace_with_dependency(&workspace, "old-doc");
    let mut server = initialized_server(&workspace);
    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let first_uri = listed_dependency_documentation_uri(&mut server, "example/dep");
    let first_text = read_resource(&mut server, 104, &first_uri)["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();

    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", &dependency_source("new-doc"));
    refresh_workspace(&mut server);
    let second = server.definition_tool(&json!({"source":"main.veln","line":5,"column":3}));
    assert_eq!(second["isError"], false, "{second:#}");

    let doc_uris = dependency_documentation_uris(&mut server, "example/dep");
    assert_eq!(doc_uris.len(), 2, "{doc_uris:#?}");
    assert!(doc_uris.contains(&first_uri));
    assert_eq!(
        read_resource(&mut server, 105, &first_uri)["result"]["contents"][0]["text"],
        first_text
    );
    assert!(doc_uris.iter().any(|uri| {
        uri != &first_uri
            && read_resource(&mut server, 106, uri)["result"]["contents"][0]["text"] != first_text
    }));
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
    single_dependency_over_capacity_is_atomic();
    multiple_dependency_over_capacity_is_atomic();
}

fn single_dependency_over_capacity_is_atomic() {
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
}

fn multiple_dependency_over_capacity_is_atomic() {
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
        let mut server = initialized_server_with_embedded_resources(&workspace);
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
fn assert_resources_are_sorted(resources: &[Value]) {
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut sorted = uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(uris, sorted);
}
fn read_resource(server: &mut Server, id: u64, uri: &str) -> Value {
    server
        .handle_request(
            json!({"jsonrpc":"2.0","id":id,"method":"resources/read","params":{"uri":uri}}),
        )
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
    write_workspace_with_dependency_source(workspace, &dependency_source(body));
}

fn write_workspace_with_dependency_source(workspace: &TempWorkspace, source: &str) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\nend\n",
    );
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", source);
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
            .filter(|resource| resource.listed)
            .map(crate::language_resources::PublishedResource::metadata),
    );
    sort_resource_metadata(&mut resources);
    resources
}

fn expected_dependency_resource_metadata(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<Value> {
    let identity = PackageIdentity::new(identity).unwrap();
    let manifest = dependency_manifest(identity.as_str());
    let source_inputs = sources.into_iter().collect::<Vec<_>>();
    let snapshot = capture_embedded_package_snapshot(
        manifest.as_bytes(),
        source_inputs
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let catalog = VirtualSourceCatalog::new([(identity.clone(), snapshot.clone())]).unwrap();
    snapshot
        .sources()
        .iter()
        .enumerate()
        .map(|(source_index, source)| {
            let entry = catalog.entry_for_source(0, source_index).unwrap();
            json!({
                "uri": entry.uri(),
                "name": source.path(),
                "title": format!("Veln package source: {}: {}", identity.as_str(), source.path()),
                "mimeType": "text/x-veln; charset=utf-8",
            })
        })
        .collect()
}

fn expected_dependency_documentation_metadata(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<Value> {
    expected_dependency_documentation_resources(identity, sources)
        .into_iter()
        .filter(|resource| resource.listed)
        .map(|resource| {
            let mut value = json!({
                "uri": resource.uri,
                "name": resource.name,
                "title": resource.title,
                "mimeType": resource.mime_type,
            });
            if let Some(description) = resource.description {
                value["description"] = json!(description);
            }
            value
        })
        .collect()
}

fn expected_dependency_documentation_resources(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<veln_language_service::RenderedPackageDocResource> {
    let identity = PackageIdentity::new(identity).unwrap();
    let manifest = parse_manifest_text("veln.toml", &dependency_manifest(identity.as_str()));
    let source_inputs = sources.into_iter().collect::<Vec<_>>();
    let snapshot = capture_embedded_package_snapshot(
        manifest.source_bytes.as_slice(),
        source_inputs
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        PackageDocGeneratorContract::new(veln_repo_mcp_standard_library_docs::GENERATOR_CONTRACT),
    );
    render_package_documentation(&result)
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

fn listed_resources(server: &mut Server) -> Vec<Value> {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"listed","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone()
}

fn listed_dependency_documentation_uri(server: &mut Server, identity: &str) -> String {
    dependency_documentation_uris(server, identity)
        .into_iter()
        .next()
        .unwrap()
}

fn dependency_documentation_uris(server: &mut Server, identity: &str) -> Vec<String> {
    let prefix = format!(
        "veln-doc:///package/{}/snapshot/",
        identity.replace('/', "%2F")
    );
    listed_resources(server)
        .into_iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect()
}

fn documentation_uri_digests_for<'a>(
    index_uri: &'a str,
    encoded_identity: &str,
) -> (&'a str, &'a str) {
    let prefix = format!("veln-doc:///package/{encoded_identity}/snapshot/");
    let rest = index_uri.strip_prefix(&prefix).unwrap();
    let mut parts = rest.split('/');
    let snapshot_digest = parts.next().unwrap();
    assert_eq!(parts.next(), Some("documentation"));
    (snapshot_digest, parts.next().unwrap())
}

fn linked_dependency_declaration_uri(server: &mut Server, index_uri: &str) -> String {
    let index = read_resource(server, 107, index_uri);
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    let module_uri = extract_uris_with_prefix(index_text, "veln-doc:///package/")
        .into_iter()
        .find(|uri| uri.contains("/module/"))
        .unwrap();
    let module = read_resource(server, 108, &module_uri);
    let module_text = module["result"]["contents"][0]["text"].as_str().unwrap();
    extract_uris_with_prefix(module_text, "veln-doc:///package/")
        .into_iter()
        .find(|uri| uri.contains("/declaration/"))
        .unwrap()
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

fn assert_resource_uris_are_unique(resources: &[Value], case: &str) {
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(uris.len(), resources.len(), "{case}");
}
