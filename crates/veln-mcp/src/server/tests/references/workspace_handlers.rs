use super::*;

fn write_handler_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "handler run(value: Int) for Work\r\n",
            "  go() => 1\r\n",
            "end\r\n\r\n",
            "fn first() -> Int\r\n",
            "  \"😀\" + handle 1 with run((1 + 2))\r\n",
            "end\r\n\r\n",
            "fn second() -> Int\r\n",
            "  handle 2 with run(2)\r\n",
            "end\r\n",
        ),
    );
}

#[test]
fn references_page_workspace_handlers_with_unicode_scalar_coordinates() {
    let workspace = TempWorkspace::new("references-workspace-handler");
    write_handler_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":6, "column":23, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {"uri":uri,"range":{"start":{"line":6,"column":23},"end":{"line":6,"column":26}}},
            {"uri":crate::definition::path_to_uri(&workspace.path("main.veln")),"range":{"start":{"line":10,"column":17},"end":{"line":10,"column":20}}}
        ])
    );

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9,
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":1,"column":9},"end":{"line":1,"column":12}})
    );
    let first_cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":first_cursor}));
    assert_eq!(
        second["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":6,"column":23},"end":{"line":6,"column":26}})
    );
    let second_cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":second_cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":10,"column":17},"end":{"line":10,"column":20}})
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}

#[test]
fn references_page_direct_dependency_handlers_with_optional_virtual_declaration() {
    let workspace = TempWorkspace::new("references-direct-dependency-handler");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub handler run() for Work\n  go() => 1\nend\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn first() -> Int\n  handle 1 with dep::run()\nend\n\n",
            "fn second() -> Int\n  handle 2 with dep::run()\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let main_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let without = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":22,
        "include_declaration":false
    }));
    assert_eq!(
        without["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        without["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|location| location["uri"] == main_uri)
            .count(),
        2,
        "{without:#}"
    );
    assert!(!without.to_string().contains("veln-pkg:"), "{without:#}");

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":8, "column":22,
        "include_declaration":true, "page_size":1
    }));
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":cursor}));
    let cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let third = server.references_tool(&json!({"cursor":cursor}));
    let locations = [&first, &second, &third]
        .into_iter()
        .flat_map(|page| page["structuredContent"]["references"].as_array().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        locations
            .iter()
            .filter(|location| location["uri"]
                .as_str()
                .is_some_and(|uri| uri.starts_with("veln-pkg:///example%2Fdep/snapshot/")))
            .count(),
        1
    );
    assert_eq!(
        locations
            .iter()
            .filter(|location| location["uri"] == main_uri)
            .count(),
        2
    );
}

#[derive(Clone, Copy)]
enum HandlerDependencySource {
    Path,
    Vendor,
    Mirror,
    LocalGit,
    MaterializedGit,
}

impl HandlerDependencySource {
    fn name(self) -> &'static str {
        match self {
            Self::Path => "handler-dependency-path",
            Self::Vendor => "handler-dependency-vendor",
            Self::Mirror => "handler-dependency-mirror",
            Self::LocalGit => "handler-dependency-local-git",
            Self::MaterializedGit => "handler-dependency-materialized-git",
        }
    }

    fn dependency_table(self) -> &'static str {
        match self {
            Self::Path => "[dependencies.\"example/dep\"]\npath = \"vendor/path-dep\"\n",
            Self::Vendor => "[dependencies.\"example/dep\"]\nvendor = \"vendor/vendor-dep\"\n",
            Self::Mirror => "[dependencies.\"example/dep\"]\nmirror = \"mirror/example/dep\"\n",
            Self::LocalGit => concat!(
                "[dependencies.\"example/dep\"]\n",
                "git = \"vendor/git-dep\"\n",
                "rev = \"abc123\"\n",
            ),
            Self::MaterializedGit => concat!(
                "[dependencies.\"example/dep\"]\n",
                "git = \"https://example.invalid/handler-dep.git\"\n",
                "rev = \"abc123\"\n",
            ),
        }
    }
}

#[test]
fn direct_dependency_handler_references_cover_source_and_project_boundaries() {
    for source_kind in [
        HandlerDependencySource::Path,
        HandlerDependencySource::Vendor,
        HandlerDependencySource::Mirror,
        HandlerDependencySource::LocalGit,
        HandlerDependencySource::MaterializedGit,
    ] {
        assert_dependency_handler_source_boundaries(source_kind);
    }
}

fn assert_dependency_handler_source_boundaries(source_kind: HandlerDependencySource) {
    let workspace = TempWorkspace::new(source_kind.name());
    let dependency_roots = write_handler_source_boundary_projects(&workspace, source_kind);
    let mut server = initialized_server(&workspace);
    assert_eq!(
        server.selection_result()["roots"],
        json!(["left", "right"]),
        "{}",
        source_kind.name()
    );
    let definition = server.definition_tool(&json!({
        "source":"left/main.veln", "line":7, "column":22
    }));
    assert_eq!(definition["isError"], false, "{definition:#}");
    let left = dependency_handler_project_references(&mut server, source_kind, "left");
    let right = dependency_handler_project_references(&mut server, source_kind, "right");
    assert_handler_locations(&workspace, source_kind, "left", &left);
    assert_handler_locations(&workspace, source_kind, "right", &right);
    assert_dependency_paths_hidden(&dependency_roots, [&definition, &left, &right]);
}

fn write_handler_source_boundary_projects(
    workspace: &TempWorkspace,
    source_kind: HandlerDependencySource,
) -> Vec<String> {
    ["left", "right"]
        .into_iter()
        .map(|project| {
            workspace.write(
                &format!("{project}/veln.toml"),
                source_kind.dependency_table(),
            );
            workspace.write(
                &format!("{project}/main.veln"),
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "# handle 0 with dep::run()\n",
                    "let label: String = \"handle 0 with dep::run()\"\n\n",
                    "fn selected() -> Int\n  handle 1 with dep::run()\nend\n",
                ),
            );
            let dependency_root = handler_dependency_root(workspace, source_kind, project);
            workspace.write(
                &format!("{dependency_root}/veln.toml"),
                "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
            );
            workspace.write(
                &format!("{dependency_root}/dep.veln"),
                concat!(
                    "pub handler run() for Work\n  go() => 1\nend\n\n",
                    "fn dependency_body() -> Int\n  handle 0 with run()\nend\n",
                ),
            );
            dependency_root
        })
        .collect()
}

fn handler_dependency_root(
    workspace: &TempWorkspace,
    source_kind: HandlerDependencySource,
    project: &str,
) -> String {
    match source_kind {
        HandlerDependencySource::Path => format!("{project}/vendor/path-dep"),
        HandlerDependencySource::Vendor => format!("{project}/vendor/vendor-dep"),
        HandlerDependencySource::Mirror => format!("{project}/mirror/example/dep"),
        HandlerDependencySource::LocalGit => format!("{project}/vendor/git-dep"),
        HandlerDependencySource::MaterializedGit => veln_project::materialized_git_repository_root(
            &workspace.path(project),
            "https://example.invalid/handler-dep.git",
        )
        .strip_prefix(&workspace.root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/"),
    }
}

fn dependency_handler_project_references(
    server: &mut Server,
    source_kind: HandlerDependencySource,
    project: &str,
) -> Value {
    let references = server.references_tool(&json!({
        "source":format!("{project}/main.veln"), "line":7, "column":22,
        "include_declaration":true
    }));
    assert_eq!(references["isError"], false, "{references:#}");
    assert_eq!(
        references["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": project,
            "project_wide": true
        }),
        "{}: {references:#}",
        source_kind.name()
    );
    references
}

fn assert_handler_locations(
    workspace: &TempWorkspace,
    source_kind: HandlerDependencySource,
    project: &str,
    references: &Value,
) {
    let locations = references["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(locations.len(), 2, "{}: {references:#}", source_kind.name());
    let expected_workspace_uri =
        crate::definition::path_to_uri(&workspace.path(&format!("{project}/main.veln")));
    assert_eq!(
        locations
            .iter()
            .filter(|location| location["uri"] == expected_workspace_uri)
            .count(),
        1,
        "{}: {references:#}",
        source_kind.name()
    );
    assert_eq!(
        locations
            .iter()
            .filter(|location| location["uri"].as_str().unwrap().starts_with("veln-pkg:"))
            .count(),
        1,
        "{}: {references:#}",
        source_kind.name()
    );
}

fn assert_dependency_paths_hidden<const N: usize>(
    dependency_roots: &[String],
    responses: [&Value; N],
) {
    for dependency_root in dependency_roots {
        for response in responses {
            assert!(!response.to_string().contains(dependency_root));
        }
    }
}

#[test]
fn direct_dependency_handler_refresh_keeps_package_owners_and_revisions_distinct() {
    let workspace = TempWorkspace::new("references-dependency-handler-owner-refresh");
    write_handler_owner_refresh_project(&workspace);
    let mut server = initialized_server(&workspace);
    assert_workspace_handler_owner(&mut server);
    let first_uri = dependency_handler_definition_uri(&mut server, 13, 24, "example%2Ffirst");
    let second_uri = dependency_handler_definition_uri(&mut server, 17, 25, "example%2Fsecond");
    assert_ne!(first_uri, second_uri);

    workspace.write(
        "vendor/first/shared.veln",
        "pub handler run() for Work\n  go() => 3\nend\n",
    );
    assert_refreshed_handler_owner(&mut server, &first_uri, &second_uri);
}

fn write_handler_owner_refresh_project(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/first\"]\npath = \"vendor/first\"\n\n",
            "[dependencies.\"example/second\"]\npath = \"vendor/second\"\n",
        ),
    );
    for (root, package, value) in [
        ("vendor/first", "example/first", 1),
        ("vendor/second", "example/second", 2),
    ] {
        workspace.write(
            &format!("{root}/veln.toml"),
            &format!("[package]\nname = \"{package}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"),
        );
        workspace.write(
            &format!("{root}/shared.veln"),
            &format!("pub handler run() for Work\n  go() => {value}\nend\n"),
        );
    }
    workspace.write(
        "main.veln",
        concat!(
            "use shared as first from \"example/first\"\n",
            "use shared as second from \"example/second\"\n\n",
            "handler run() for Work\n  go() => 0\nend\n\n",
            "fn local() -> Int\n  handle 0 with run()\nend\n\n",
            "fn use_first() -> Int\n  handle 1 with first::run()\nend\n\n",
            "fn use_second() -> Int\n  handle 2 with second::run()\nend\n",
        ),
    );
}

fn assert_workspace_handler_owner(server: &mut Server) {
    let local = server.definition_tool(&json!({"source":"main.veln", "line":9, "column":17}));
    assert!(
        local["structuredContent"]["definition"]["uri"]
            .as_str()
            .unwrap()
            .starts_with("file://")
    );
}

fn dependency_handler_definition_uri(
    server: &mut Server,
    line: usize,
    column: usize,
    package: &str,
) -> String {
    let definition = server.definition_tool(&json!({
        "source":"main.veln", "line":line, "column":column
    }));
    let uri = definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(uri.contains(package), "{definition:#}");
    uri
}

fn assert_refreshed_handler_owner(server: &mut Server, first_uri: &str, second_uri: &str) {
    let first_after = server.definition_tool(&json!({
        "source":"main.veln", "line":13, "column":24
    }));
    let second_after = server.definition_tool(&json!({
        "source":"main.veln", "line":17, "column":25
    }));
    let first_after_uri = first_after["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert!(
        first_after_uri.contains("example%2Ffirst"),
        "{first_after:#}"
    );
    assert_ne!(first_after_uri, first_uri);
    assert_eq!(
        second_after["structuredContent"]["definition"]["uri"],
        second_uri
    );
    assert_eq!(
        first_after["structuredContent"]["definition"]["range"],
        json!({"start":{"line":1,"column":13},"end":{"line":1,"column":16}})
    );
}

#[test]
fn standard_library_handler_navigation_keeps_owner_and_revision_at_mcp_boundary() {
    let workspace = TempWorkspace::new("references-standard-handler-owner-revision");
    write_standard_handler_owner_project(&workspace);
    let old_source = "pub handler noop() for Observe\n  record() => 1\nend\n";
    let new_source = "\npub handler noop() for Observe\n  record() => 2\nend\n";
    let mut old_server = initialized_server(&workspace);
    let mut new_server = initialized_server(&workspace);
    install_handler_standard_library(&mut old_server, old_source);
    install_handler_standard_library(&mut new_server, new_source);
    let main_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let old = standard_handler_mcp_projection(&mut old_server, &main_uri, 1);
    let new = standard_handler_mcp_projection(&mut new_server, &main_uri, 2);
    assert_ne!(old, new);

    let old_dependency = old_server.definition_tool(&json!({
        "source":"main.veln", "line":10, "column":37
    }));
    let new_dependency = new_server.definition_tool(&json!({
        "source":"main.veln", "line":10, "column":37
    }));
    let old_dependency_uri = old_dependency["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert_eq!(
        new_dependency["structuredContent"]["definition"]["uri"],
        old_dependency_uri
    );
    assert!(
        old_dependency_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"),
        "{old_dependency:#}"
    );
    assert!(!old_dependency.to_string().contains("vendor/dep"));
    let old_workspace = old_server.definition_tool(&json!({
        "source":"observe.veln", "line":3, "column":13
    }));
    let new_workspace = new_server.definition_tool(&json!({
        "source":"observe.veln", "line":3, "column":13
    }));
    assert_eq!(old_workspace, new_workspace);
    assert!(
        old_workspace["structuredContent"]["definition"]["uri"]
            .as_str()
            .unwrap()
            .starts_with("file://"),
        "{old_workspace:#}"
    );

    let old_read = read_handler_resource(&mut old_server, &old);
    let new_read = read_handler_resource(&mut new_server, &new);
    assert_eq!(old_read["result"]["contents"][0]["text"], old_source);
    assert_eq!(new_read["result"]["contents"][0]["text"], new_source);
    for (server, rejected) in [(&mut old_server, &new), (&mut new_server, &old)] {
        let response = read_handler_resource(server, rejected);
        assert_eq!(response["error"]["data"]["code"], "resource_not_found");
    }
}

fn write_standard_handler_owner_project(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"observe.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/observe.veln",
        "pub handler noop() for Observe\n  record() => 3\nend\n",
    );
    workspace.write(
        "observe.veln",
        concat!(
            "mod observe\n\n",
            "pub handler noop() for Observe\n  record() => 4\nend\n\n",
            "fn local() -> Int\n  handle 4 with noop()\nend\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use observe as standard_observe from \"std\"\n",
            "use observe as dependency_observe from \"example/dep\"\n",
            "# The same-spelled workspace owner is selected in observe.veln.\n\n",
            "fn standard() -> Int\n  handle 1 with standard_observe::noop()\nend\n\n",
            "fn dependency() -> Int\n  handle 2 with dependency_observe::noop()\nend\n",
        ),
    );
}

fn install_handler_standard_library(server: &mut Server, source: &str) {
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"observe.veln\"]\n",
        [veln_project::PackageSnapshotSource::new(
            "observe.veln",
            source.as_bytes(),
        )],
    );
}

fn standard_handler_mcp_projection(
    server: &mut Server,
    main_uri: &str,
    declaration_line: usize,
) -> String {
    let definition = server.definition_tool(&json!({
        "source":"main.veln", "line":6, "column":35
    }));
    assert_eq!(definition["isError"], false, "{definition:#}");
    let location = &definition["structuredContent"]["definition"];
    let uri = location["uri"].as_str().unwrap().to_owned();
    assert!(
        uri.starts_with("veln-pkg:///std/snapshot/"),
        "{definition:#}"
    );
    assert_eq!(
        location["range"],
        json!({"start":{"line":declaration_line,"column":13},"end":{"line":declaration_line,"column":17}})
    );
    assert!(!definition.to_string().contains("vendor/dep"));
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":6, "column":35,
        "include_declaration":true
    }));
    let mut locations = references["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .clone();
    let mut expected_locations = vec![
        json!({
            "uri": uri,
            "range": {
                "start": {"line": declaration_line, "column": 13},
                "end": {"line": declaration_line, "column": 17}
            }
        }),
        json!({
            "uri": main_uri,
            "range": {
                "start": {"line": 6, "column": 35},
                "end": {"line": 6, "column": 39}
            }
        }),
    ];
    locations.sort_by(|left, right| left["uri"].as_str().cmp(&right["uri"].as_str()));
    expected_locations.sort_by(|left, right| left["uri"].as_str().cmp(&right["uri"].as_str()));
    assert_eq!(locations, expected_locations, "{references:#}");
    assert!(!references.to_string().contains("vendor/dep"));
    uri
}

fn read_handler_resource(server: &mut Server, uri: &str) -> Value {
    server
        .handle_request(json!({
            "jsonrpc":"2.0", "id":"read-standard-handler-source",
            "method":"resources/read", "params":{"uri":uri}
        }))
        .unwrap()
}

#[test]
fn transitive_dependency_handler_is_rejected_without_contaminating_direct_handler() {
    let workspace = TempWorkspace::new("references-transitive-dependency-handler");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/direct\"]\npath = \"vendor/direct\"\n",
    );
    workspace.write(
        "vendor/direct/veln.toml",
        concat!(
            "[package]\nname = \"example/direct\"\n\n",
            "[lib]\nexports = [\"shared.veln\"]\n\n",
            "[dependencies.\"example/transitive\"]\npath = \"../transitive\"\n",
        ),
    );
    workspace.write(
        "vendor/direct/shared.veln",
        "pub handler run() for Work\n  go() => 1\nend\n",
    );
    workspace.write(
        "vendor/transitive/veln.toml",
        "[package]\nname = \"example/transitive\"\n\n[lib]\nexports = [\"shared.veln\"]\n",
    );
    workspace.write(
        "vendor/transitive/shared.veln",
        "pub handler run() for Work\n  go() => 2\nend\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use shared as direct from \"example/direct\"\n",
            "use shared as transitive from \"example/transitive\"\n\n",
            "fn valid() -> Int\n  handle 1 with direct::run()\nend\n\n",
            "fn invalid() -> Int\n  handle 2 with transitive::run()\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let valid_request = json!({
        "source":"main.veln", "line":5, "column":25, "include_declaration":true
    });
    let valid_before = server.references_tool(&valid_request);
    assert_eq!(valid_before["isError"], false, "{valid_before:#}");
    assert!(valid_before.to_string().contains("example%2Fdirect"));

    let invalid_definition = server.definition_tool(&json!({
        "source":"main.veln", "line":9, "column":29
    }));
    let invalid_references = server.references_tool(&json!({
        "source":"main.veln", "line":9, "column":29, "include_declaration":true
    }));
    assert!(invalid_definition["structuredContent"]["definition"].is_null());
    assert_eq!(
        invalid_references["structuredContent"]["references"],
        json!([])
    );
    assert_eq!(server.references_tool(&valid_request), valid_before);
}

#[test]
fn dependency_handler_position_failures_do_not_publish_refreshed_resources() {
    let workspace = TempWorkspace::new("references-dependency-handler-position-atomicity");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub handler run() for Work\n  go() => 1\nend\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn valid() -> Int\n  handle 1 with dep::run()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":22, "include_declaration":true
    }));
    assert_eq!(before["isError"], false, "{before:#}");
    let resources_before = all_resource_state(&mut server);

    workspace.write(
        "vendor/dep/dep.veln",
        "\npub handler run() for Work\n  go() => 2\nend\n",
    );
    let invalid_references = server.references_tool(&json!({
        "source":"main.veln", "line":999, "column":1, "include_declaration":true
    }));
    assert_eq!(
        invalid_references["structuredContent"]["code"],
        "invalid_position"
    );
    assert_eq!(all_resource_state(&mut server), resources_before);

    let invalid_definition = server.definition_tool(&json!({
        "source":"main.veln", "line":999, "column":1
    }));
    assert_eq!(
        invalid_definition["structuredContent"]["code"],
        "invalid_position"
    );
    assert_eq!(all_resource_state(&mut server), resources_before);
}

#[test]
fn imported_workspace_handler_navigation_pages_shared_scalar_locations() {
    let workspace = TempWorkspace::new("references-workspace-imported-handler");
    write_imported_handler_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let handler_uri = crate::definition::path_to_uri(&workspace.path("handler.veln"));
    let consumer_uri = crate::definition::path_to_uri(&workspace.path("consumer.veln"));

    assert_imported_handler_definition(&mut server, &handler_uri);
    assert_imported_handler_references(&mut server, &handler_uri, &consumer_uri);
    assert_imported_handler_pagination(&mut server);
    assert_ineligible_imported_handler_results_are_empty(&mut server);
    assert_imported_handler_definition(&mut server, &handler_uri);
}

fn write_imported_handler_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "handler.veln",
        concat!(
            "mod library::fx\n\n",
            "pub handler run() for Work\n  go() => 1\nend\n\n",
            "fn local() -> Int\n  handle 0 with run()\nend\n",
            "\nhandler hidden() for Work\n  go() => 2\nend\n\n",
            "pub fn work() -> Int\n  4\nend\n",
        ),
    );
    workspace.write(
        "consumer.veln",
        concat!(
            "mod consumer\n\nuse library::fx\n\n",
            "fn full() -> Int\n  handle 1 with library::fx::run()\nend\n\n",
            "fn alias() -> Int\n  handle 2 with fx::run()\nend\n",
            "\nfn private() -> Int\n  handle 3 with library::fx::hidden()\nend\n\n",
            "fn wrong_kind() -> Int\n  handle 4 with library::fx::work()\nend\n",
        ),
    );
}

fn assert_imported_handler_definition(server: &mut Server, handler_uri: &str) {
    let definition = server.definition_tool(&json!({
        "source":"consumer.veln", "line":6, "column":30
    }));
    assert_eq!(
        definition["structuredContent"]["definition"]["uri"],
        handler_uri
    );
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":3,"column":13},"end":{"line":3,"column":16}})
    );
}

fn assert_imported_handler_references(server: &mut Server, handler_uri: &str, consumer_uri: &str) {
    let without_declaration = server.references_tool(&json!({
        "source":"consumer.veln", "line":10, "column":21,
        "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {"uri":consumer_uri,"range":{"start":{"line":6,"column":30},"end":{"line":6,"column":33}}},
            {"uri":consumer_uri,"range":{"start":{"line":10,"column":21},"end":{"line":10,"column":24}}},
            {"uri":handler_uri,"range":{"start":{"line":8,"column":17},"end":{"line":8,"column":20}}}
        ])
    );
}

fn assert_imported_handler_pagination(server: &mut Server) {
    let first = server.references_tool(&json!({
        "source":"handler.veln", "line":3, "column":13,
        "include_declaration":true, "page_size":2
    }));
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(second["structuredContent"].get("next_cursor").is_none());
}

fn assert_ineligible_imported_handler_results_are_empty(server: &mut Server) {
    let private_definition = server.definition_tool(&json!({
        "source":"consumer.veln", "line":14, "column":30
    }));
    assert!(private_definition["structuredContent"]["definition"].is_null());
    let private_references = server.references_tool(&json!({
        "source":"consumer.veln", "line":14, "column":30,
        "include_declaration":true
    }));
    assert_eq!(
        private_references["structuredContent"]["references"],
        json!([])
    );

    let wrong_kind_definition = server.definition_tool(&json!({
        "source":"consumer.veln", "line":18, "column":30
    }));
    assert!(wrong_kind_definition["structuredContent"]["definition"].is_null());
    let wrong_kind_references = server.references_tool(&json!({
        "source":"consumer.veln", "line":18, "column":30,
        "include_declaration":true
    }));
    assert_eq!(
        wrong_kind_references["structuredContent"]["references"],
        json!([])
    );
}

#[test]
fn imported_workspace_handler_navigation_rejects_ineligible_imports() {
    for (workspace_name, imports) in [
        (
            "references-workspace-imported-handler-ambiguous",
            "use first::fx\nuse second::fx\n",
        ),
        (
            "references-workspace-imported-handler-duplicate",
            "use first::fx\nuse first::fx\n",
        ),
        (
            "references-workspace-imported-handler-recovered",
            "use first::fx unexpected\n",
        ),
    ] {
        let workspace = TempWorkspace::new(workspace_name);
        workspace.write("veln.toml", "");
        workspace.write(
            "first/fx.veln",
            "pub handler run() for Work\n  go() => 1\nend\n",
        );
        workspace.write(
            "second/fx.veln",
            "pub handler run() for Work\n  go() => 2\nend\n",
        );
        workspace.write(
            "consumer.veln",
            &format!("{imports}\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n"),
        );
        let mut server = initialized_server(&workspace);
        let line = imports.lines().count() + 3;

        let definition = server.definition_tool(&json!({
            "source":"consumer.veln", "line":line, "column":21
        }));
        assert!(definition["structuredContent"]["definition"].is_null());
        let references = server.references_tool(&json!({
            "source":"consumer.veln", "line":line, "column":21,
            "include_declaration":true
        }));
        assert_eq!(
            references["structuredContent"]["references"],
            json!([]),
            "{workspace_name}"
        );
    }
}

struct RejectedImportedHandlerCase {
    name: &'static str,
    manifest: &'static str,
    files: &'static [(&'static str, &'static str)],
    source: &'static str,
    line: u64,
    column: u64,
}

#[test]
fn imported_workspace_handler_navigation_rejects_h4_boundaries_without_losing_state() {
    let cases = [
        RejectedImportedHandlerCase {
            name: "private",
            manifest: "",
            files: &[
                (
                    "private.veln",
                    "mod private::fx\n\nhandler hidden() for Work\n  go() => 1\nend\n",
                ),
                (
                    "case.veln",
                    "use private::fx\n\nfn invalid() -> Int\n  handle 1 with private::fx::hidden()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            column: 30,
        },
        RejectedImportedHandlerCase {
            name: "ambiguous-import",
            manifest: "",
            files: &[
                (
                    "first/fx.veln",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
                (
                    "second/fx.veln",
                    "pub handler run() for Work\n  go() => 2\nend\n",
                ),
                (
                    "case.veln",
                    "use first::fx\nuse second::fx\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 5,
            column: 21,
        },
        RejectedImportedHandlerCase {
            name: "duplicate-import",
            manifest: "",
            files: &[
                (
                    "first/fx.veln",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
                (
                    "case.veln",
                    "use first::fx\nuse first::fx\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 5,
            column: 21,
        },
        RejectedImportedHandlerCase {
            name: "recovered-import",
            manifest: "",
            files: &[
                (
                    "first/fx.veln",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
                (
                    "case.veln",
                    "use first::fx unexpected\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            column: 21,
        },
        RejectedImportedHandlerCase {
            name: "duplicate-declaration",
            manifest: "",
            files: &[
                (
                    "first.veln",
                    "mod shared\n\nhandler run() for Work\n  go() => 1\nend\n",
                ),
                (
                    "case.veln",
                    "mod shared\n\nhandler run() for Work\n  go() => 2\nend\n\nfn invalid() -> Int\n  handle 1 with run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 8,
            column: 17,
        },
        RejectedImportedHandlerCase {
            name: "recovered-declaration",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work @\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run()\nend\n",
            )],
            source: "case.veln",
            line: 6,
            column: 17,
        },
        RejectedImportedHandlerCase {
            name: "unretained-direct-dependency",
            manifest: "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
            files: &[
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
                (
                    "case.veln",
                    "use dep from \"example/dep\"\n\nfn invalid() -> Int\n  handle 1 with dep::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            column: 22,
        },
        RejectedImportedHandlerCase {
            name: "unavailable-standard-library-handler",
            manifest: "",
            files: &[(
                "case.veln",
                "use observe from \"std\"\n\nfn invalid() -> Int\n  handle 1 with observe::noop()\nend\n",
            )],
            source: "case.veln",
            line: 4,
            column: 25,
        },
        RejectedImportedHandlerCase {
            name: "unresolved-path",
            manifest: "",
            files: &[(
                "case.veln",
                "fn invalid() -> Int\n  handle 1 with missing::run()\nend\n",
            )],
            source: "case.veln",
            line: 2,
            column: 26,
        },
        RejectedImportedHandlerCase {
            name: "wrong-kind",
            manifest: "",
            files: &[
                ("wrong.veln", "mod wrong\n\npub fn run() -> Int\n  1\nend\n"),
                (
                    "case.veln",
                    "use wrong\n\nfn invalid() -> Int\n  handle 1 with wrong::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            column: 23,
        },
        RejectedImportedHandlerCase {
            name: "incomplete-application",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run(\nend\n",
            )],
            source: "case.veln",
            line: 6,
            column: 17,
        },
        RejectedImportedHandlerCase {
            name: "recovered-application",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run(1 2)\nend\n",
            )],
            source: "case.veln",
            line: 6,
            column: 17,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        workspace.write("veln.toml", case.manifest);
        workspace.write(
            "stable.veln",
            "mod stable\n\npub handler keep() for Work\n  go() => 0\nend\n\nfn local() -> Int\n  handle 0 with keep()\nend\n",
        );
        workspace.write(
            "stable_consumer.veln",
            "use stable\n\nfn valid() -> Int\n  handle 0 with stable::keep()\nend\n",
        );
        for (path, source) in case.files {
            workspace.write(path, source);
        }
        let mut server = initialized_server(&workspace);
        let valid = json!({"source":"stable_consumer.veln", "line":4, "column":25});
        let valid_with_declaration = json!({
            "source":"stable_consumer.veln", "line":4, "column":25,
            "include_declaration":true
        });
        let definition_before = server.definition_tool(&valid);
        let references_before = server.references_tool(&valid_with_declaration);
        assert_eq!(
            definition_before["structuredContent"]["definition"]["uri"],
            crate::definition::path_to_uri(&workspace.path("stable.veln")),
            "{} valid definition",
            case.name
        );
        assert_eq!(
            references_before["structuredContent"]["references"]
                .as_array()
                .unwrap()
                .len(),
            3,
            "{} valid references",
            case.name
        );

        let definition = server.definition_tool(&json!({
            "source":case.source, "line":case.line, "column":case.column
        }));
        assert!(
            definition["structuredContent"]["definition"].is_null(),
            "{} definition: {definition:#}",
            case.name
        );
        let references = server.references_tool(&json!({
            "source":case.source, "line":case.line, "column":case.column,
            "include_declaration":true
        }));
        assert_eq!(
            references["structuredContent"]["references"],
            json!([]),
            "{} references",
            case.name
        );
        assert_eq!(
            server.definition_tool(&valid),
            definition_before,
            "{} retained definition",
            case.name
        );
        assert_eq!(
            server.references_tool(&valid_with_declaration),
            references_before,
            "{} retained references",
            case.name
        );
    }
}

#[test]
fn references_include_a_workspace_handler_declaration_without_occurrences() {
    let workspace = TempWorkspace::new("references-workspace-handler-declaration-only");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "handler run() for Work\n  go() => 1\nend\n");
    let mut server = initialized_server(&workspace);

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([])
    );

    let with_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "include_declaration":true
    }));
    assert_eq!(
        with_declaration["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {
                "start": {"line": 1, "column": 9},
                "end": {"line": 1, "column": 12}
            }
        }])
    );
}

fn exercise_handler_reference_failures(server: &mut Server, cursor: &str) {
    let invalid_path = server.references_tool(&json!({
        "source":"missing.veln", "line":1, "column":1
    }));
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");

    let invalid_position = server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1
    }));
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );

    let invalid_cursor = server.references_tool(&json!({"cursor":format!("{cursor}x")}));
    assert_eq!(
        invalid_cursor["structuredContent"]["code"],
        "invalid_cursor"
    );

    let missing_resource = server
        .handle_request(json!({
            "jsonrpc":"2.0", "id":"missing-handler-resource",
            "method":"resources/read",
            "params":{"uri":"veln-pkg:///missing/snapshot/main.veln"}
        }))
        .unwrap();
    assert_eq!(
        missing_resource["error"]["data"]["code"],
        "resource_not_found"
    );
}

#[test]
fn workspace_handler_reference_failures_preserve_results_resources_selection_and_cursor() {
    let workspace = TempWorkspace::new("references-workspace-handler-failure-state");
    write_handler_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9
    }));
    let seeded = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9, "page_size":1
    }));
    let cursor = seeded["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let resources = all_resource_state(&mut server);
    let selection = server.selection_result();

    exercise_handler_reference_failures(&mut server, &cursor);

    assert_eq!(all_resource_state(&mut server), resources);
    assert_eq!(server.selection_result(), selection);
    let continuation = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(continuation["isError"], false);
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let after = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":9
    }));
    assert_eq!(after, before);
}
