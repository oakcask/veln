#[test]
fn standard_library_handler_references_keep_workspace_projects_isolated() {
    let project = TempProject::new("standard-library-handler-project-isolation");
    for root in ["left", "right"] {
        project.write(&format!("{root}/veln.toml"), "");
        project.write(
            &format!("{root}/main.veln"),
            concat!(
                "use net from \"std\"\n\n",
                "fn selected() -> Int\n",
                "  # handle 0 with net::system()\n",
                "  let label: String = \"handle 0 with net::system()\"\n",
                "  handle 1 with net::system()\n",
                "end\n",
            ),
        );
    }
    let left_root = path_to_uri(&project.root.join("left"));
    let right_root = path_to_uri(&project.root.join("right"));
    let mut server = Server::default();
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{left_root}","name":"left"}},{{"uri":"{right_root}","name":"right"}}]}}}}"#
    ));
    let standard_uri = embedded_standard_library_source_uri("net.veln");
    let connection_source = standard_source_text("http2/connection.veln");
    assert_eq!(
        connection_source.lines().nth(111),
        Some(
            "\thandle request_endpoint_sequence(endpoint, requests, handler) with net::system()"
        )
    );
    let connection_uri = embedded_standard_library_source_uri("http2/connection.veln");
    let connection_document = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":53,"method":"veln/virtualDocument","params":{{"uri":"{connection_uri}"}}}}"#
    ));
    assert_eq!(
        connection_document,
        [response(
            "53",
            &format!(r#""{}""#, escape_json(connection_source)),
        )]
    );
    assert_embedded_standard_library_paths_are_hidden_from_lsp(&connection_document[0]);

    for root in ["left", "right"] {
        assert_standard_handler_lsp_project_isolation(
            &project,
            &mut server,
            root,
            &standard_uri,
        );
    }
}

fn assert_standard_handler_lsp_project_isolation(
    project: &TempProject,
    server: &mut Server,
    root: &str,
    standard_uri: &str,
) {
    let uri = path_to_uri(&project.root.join(format!("{root}/main.veln")));
    let definition = server.handle_message(&definition_request(&uri, 5, 21));
    assert_eq!(
        definition,
        [response(
            "2",
            &format!(
                r#"{{"uri":"{standard_uri}","range":{{"start":{{"line":109,"character":12}},"end":{{"line":109,"character":18}}}}}}"#
            ),
        )]
    );
    let virtual_document = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":{},"method":"veln/virtualDocument","params":{{"uri":"{standard_uri}"}}}}"#,
        if root == "left" { "51" } else { "52" }
    ));
    assert_eq!(
        virtual_document,
        [response(
            if root == "left" { "51" } else { "52" },
            &format!(r#""{}""#, escape_json(standard_source_text("net.veln"))),
        )]
    );

    let references = server.handle_message(&references_request_with_declaration(
        &uri, 5, 21, true,
    ));
    assert_eq!(
        references,
        [response(
            "2",
            &format!(
                r#"[{{"uri":"{uri}","range":{{"start":{{"line":5,"character":21}},"end":{{"line":5,"character":27}}}}}}]"#
            ),
        )]
    );
    for response in definition
        .iter()
        .chain(references.iter())
        .chain(virtual_document.iter())
    {
        assert_embedded_standard_library_paths_are_hidden_from_lsp(response);
    }
}

fn embedded_standard_library_source_uri(source_path: &str) -> String {
    let bundle = veln_stdlib::package_bundle();
    let snapshot = capture_embedded_package_snapshot(
        bundle.manifest.as_bytes(),
        bundle
            .files
            .iter()
            .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
    )
    .unwrap();
    veln_language_service::VirtualSourceCatalog::new([(
        PackageIdentity::embedded_standard(),
        snapshot,
    )])
    .unwrap()
    .entries()
    .find(|entry| entry.uri().ends_with(&format!("/{source_path}")))
    .unwrap()
    .uri()
    .to_owned()
}

fn assert_embedded_standard_library_paths_are_hidden_from_lsp(response: &str) {
    let source_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../veln-stdlib/veln/net.veln")
        .canonicalize()
        .unwrap();
    let source_path = source_path.to_string_lossy();
    let rejected = [
        source_path.as_ref(),
        "crates/veln-stdlib/veln/net.veln",
        "build/veln-stdlib-",
        "stdlib_bundle.rs",
        "lowered/net.veln.bin",
    ];
    assert_eq!(
        rejected
            .into_iter()
            .find(|fragment| response.contains(fragment)),
        None,
        "leaked a standard-library source or build path: {response}",
    );
}
