const OLD_STANDARD_HANDLER: &str = "pub handler noop() for Observe\n  record() => 1\nend\n";
const NEW_STANDARD_HANDLER: &str = "\npub handler noop() for Observe\n  record() => 2\nend\n";

#[test]
fn standard_library_handler_navigation_keeps_owner_and_revision_at_lsp_boundary() {
    let (project, main_uri, workspace_uri) = standard_handler_owner_project();
    let (mut old_server, mut new_server) = standard_handler_owner_servers(&project);
    let (old_uri, new_uri) = assert_standard_handler_definitions(
        &mut old_server,
        &mut new_server,
        &main_uri,
    );

    assert_standard_handler_source_isolation(&mut old_server, &mut new_server, &old_uri, &new_uri);
    assert_colliding_handler_owners_stay_stable(
        &mut old_server,
        &mut new_server,
        &main_uri,
        &workspace_uri,
    );
    assert_standard_handler_references(&mut old_server, &main_uri);
    assert_standard_handler_references(&mut new_server, &main_uri);
}

fn standard_handler_owner_project() -> (TempProject, String, String) {
    let project = TempProject::new("standard-library-handler-owner-revision");
    project.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    project.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"observe.veln\"]\n",
    );
    project.write(
        "vendor/dep/observe.veln",
        "pub handler noop() for Observe\n  record() => 3\nend\n",
    );
    project.write(
        "observe.veln",
        concat!(
            "mod observe\n\n",
            "pub handler noop() for Observe\n  record() => 4\nend\n\n",
            "fn local() -> Int\n  handle 4 with noop()\nend\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use observe as standard_observe from \"std\"\n",
            "use observe as dependency_observe from \"example/dep\"\n",
            "# The same-spelled workspace owner is selected in observe.veln.\n\n",
            "fn standard() -> Int\n  handle 1 with standard_observe::noop()\nend\n\n",
            "fn dependency() -> Int\n  handle 2 with dependency_observe::noop()\nend\n",
        ),
    );
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let workspace_uri = path_to_uri(&project.root.join("observe.veln"));
    (project, main_uri, workspace_uri)
}

fn standard_handler_owner_servers(project: &TempProject) -> (Server, Server) {
    let root_uri = path_to_uri(&project.root);
    let mut old_server =
        Server::default().with_standard_library(test_handler_standard_library(OLD_STANDARD_HANDLER));
    let mut new_server =
        Server::default().with_standard_library(test_handler_standard_library(NEW_STANDARD_HANDLER));
    old_server.handle_message(&initialize_request(&root_uri));
    new_server.handle_message(&initialize_request(&root_uri));
    (old_server, new_server)
}

fn assert_standard_handler_definitions(
    old_server: &mut Server,
    new_server: &mut Server,
    main_uri: &str,
) -> (String, String) {
    let old_definition = old_server.handle_message(&definition_request(main_uri, 5, 34));
    let new_definition = new_server.handle_message(&definition_request(main_uri, 5, 34));
    let old_uri = extract_string_field(&old_definition[0], "uri").unwrap();
    let new_uri = extract_string_field(&new_definition[0], "uri").unwrap();
    assert!(old_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert!(new_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert_ne!(old_uri, new_uri);
    assert!(old_definition[0].contains(
        r#""range":{"start":{"line":0,"character":12},"end":{"line":0,"character":16}}"#
    ));
    assert!(new_definition[0].contains(
        r#""range":{"start":{"line":1,"character":12},"end":{"line":1,"character":16}}"#
    ));
    (old_uri, new_uri)
}

fn assert_standard_handler_source_isolation(
    old_server: &mut Server,
    new_server: &mut Server,
    old_uri: &str,
    new_uri: &str,
) {
    assert_virtual_document_text(old_server, "41", old_uri, OLD_STANDARD_HANDLER);
    assert_virtual_document_text(new_server, "42", new_uri, NEW_STANDARD_HANDLER);
    let old_cross_read = old_server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":43,"method":"veln/virtualDocument","params":{{"uri":"{new_uri}"}}}}"#
    ));
    let new_cross_read = new_server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":44,"method":"veln/virtualDocument","params":{{"uri":"{old_uri}"}}}}"#
    ));
    assert_contains_json(&old_cross_read[0], r#""code":-32602"#);
    assert_contains_json(&new_cross_read[0], r#""code":-32602"#);
}

fn assert_colliding_handler_owners_stay_stable(
    old_server: &mut Server,
    new_server: &mut Server,
    main_uri: &str,
    workspace_uri: &str,
) {
    let old_dependency = old_server.handle_message(&definition_request(main_uri, 9, 36));
    let new_dependency = new_server.handle_message(&definition_request(main_uri, 9, 36));
    assert_eq!(old_dependency, new_dependency);
    let dependency_uri = extract_string_field(&old_dependency[0], "uri").unwrap();
    assert!(dependency_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"));
    assert!(!old_dependency[0].contains("vendor/dep"), "{}", old_dependency[0]);
    let old_workspace = old_server.handle_message(&definition_request(workspace_uri, 2, 12));
    let new_workspace = new_server.handle_message(&definition_request(workspace_uri, 2, 12));
    assert_eq!(old_workspace, new_workspace);
    assert_eq!(
        extract_string_field(&old_workspace[0], "uri"),
        Some(workspace_uri.to_owned())
    );
}

fn assert_standard_handler_references(server: &mut Server, main_uri: &str) {
    let references = server.handle_message(&references_request_with_declaration(
        main_uri, 5, 34, true,
    ));
    assert_eq!(
        references,
        [response(
            "2",
            &format!(
                r#"[{{"uri":"{main_uri}","range":{{"start":{{"line":5,"character":34}},"end":{{"line":5,"character":38}}}}}}]"#
            ),
        )]
    );
    assert!(!references[0].contains("veln-pkg:"), "{}", references[0]);
    assert!(!references[0].contains("vendor/dep"), "{}", references[0]);
}

fn test_handler_standard_library(source: &str) -> DirectDependencySnapshot {
    let manifest = "[package]\nname = \"std\"\n\n[lib]\nexports = [\"observe.veln\"]\n";
    let snapshot = capture_embedded_package_snapshot(
        manifest.as_bytes(),
        [PackageSnapshotSource::new("observe.veln", source.as_bytes())],
    )
    .unwrap();
    DirectDependencySnapshot::from_validated_standard_library(
        snapshot,
        parse_manifest_text("veln.toml", manifest),
    )
    .unwrap()
}
