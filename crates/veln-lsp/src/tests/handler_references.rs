#[test]
fn workspace_handler_references_preserve_utf16_crlf_and_declaration_policy() {
    let project = TempProject::new("workspace-handler-references");
    project.write("veln.toml", "");
    project.write(
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
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let without_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 23, false,
    ));
    assert_eq!(
        without_declaration,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":23}},\"end\":{{\"line\":5,\"character\":26}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":9,\"character\":16}},\"end\":{{\"line\":9,\"character\":19}}}}}}]"
                ),
                main_uri, main_uri,
            ),
        )]
    );

    let with_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 0, 8, true,
    ));
    assert_eq!(
        with_declaration,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":0,\"character\":8}},\"end\":{{\"line\":0,\"character\":11}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":23}},\"end\":{{\"line\":5,\"character\":26}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":9,\"character\":16}},\"end\":{{\"line\":9,\"character\":19}}}}}}]"
                ),
                main_uri, main_uri, main_uri,
            ),
        )]
    );
}

#[test]
fn direct_dependency_handler_navigation_returns_virtual_definition_and_workspace_references() {
    let project = TempProject::new("direct-dependency-handler-references");
    let dependency_source = concat!(
        "pub handler run() for Work\n  go() => 1\nend\n\n",
        "handler hidden() for Work\n  go() => 2\nend\n",
    );
    project.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    project.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    project.write("vendor/dep/dep.veln", dependency_source);
    project.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn first() -> Int\n  handle 1 with dep::run()\nend\n\n",
            "fn second() -> Int\n  handle 2 with dep::run()\nend\n",
            "\nfn private() -> Int\n  handle 3 with dep::hidden()\nend\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = server_with_empty_standard_library();
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 21));
    assert_eq!(definition.len(), 1);
    let definition_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(
        definition_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"),
        "{}",
        definition[0]
    );
    assert!(definition[0].contains(
        r#""range":{"start":{"line":0,"character":12},"end":{"line":0,"character":15}}"#
    ));
    assert_virtual_document_text(&mut server, "3", &definition_uri, dependency_source);

    for include_declaration in [false, true] {
        let references = server.handle_message(&references_request_with_declaration(
            &main_uri,
            7,
            21,
            include_declaration,
        ));
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].matches(&format!(r#""uri":"{main_uri}""#)).count(), 2);
        assert!(!references[0].contains("veln-pkg:"), "{}", references[0]);
    }

    let private_definition = server.handle_message(&definition_request(&main_uri, 11, 21));
    assert_eq!(private_definition, [response("2", "null")]);
    let private_references = server.handle_message(&references_request_with_declaration(
        &main_uri, 11, 21, true,
    ));
    assert_eq!(private_references, [response("2", "[]")]);

    let definition_after_rejection = server.handle_message(&definition_request(&main_uri, 3, 21));
    assert_eq!(definition_after_rejection, definition);
}

#[test]
fn standard_library_handler_navigation_returns_virtual_definition_and_workspace_references() {
    let (_project, main_uri, mut server) = standard_library_handler_navigation_server();
    let definition = assert_standard_library_handler_definition(&mut server, &main_uri);
    assert_standard_library_handler_alias_and_references(&mut server, &main_uri, &definition);
    assert_ineligible_standard_library_handler_is_empty(&mut server, &main_uri);
}

fn standard_library_handler_navigation_server() -> (TempProject, String, Server) {
    let handler_source = standard_library_handler_source();
    let standard_library = standard_library_handler_snapshot(handler_source);
    let project = TempProject::new("standard-library-handler-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        concat!(
            "use observe as trace from \"std\"\n\n",
            "fn first() -> Int\n  handle 1 with observe::noop()\nend\n\n",
            "fn second() -> Int\n  handle 2 with trace::noop()\nend\n\n",
            "fn canonical() -> Int\n  handle 3 with std::observe::noop()\nend\n\n",
            "fn private() -> Int\n  handle 4 with observe::hidden()\nend\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default().with_standard_library(standard_library);
    server.handle_message(&initialize_request(&root_uri));
    (project, main_uri, server)
}

fn standard_library_handler_snapshot(source: &str) -> DirectDependencySnapshot {
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

fn assert_standard_library_handler_definition(server: &mut Server, main_uri: &str) -> Vec<String> {
    let definition = server.handle_message(&definition_request(main_uri, 3, 25));
    assert_eq!(definition.len(), 1);
    let definition_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(definition_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert!(definition[0].contains(
        r#""range":{"start":{"line":0,"character":12},"end":{"line":0,"character":16}}"#
    ));
    assert_virtual_document_text(server, "3", &definition_uri, standard_library_handler_source());
    definition
}

fn assert_standard_library_handler_alias_and_references(
    server: &mut Server,
    main_uri: &str,
    definition: &[String],
) {
    let alias_definition = server.handle_message(&definition_request(main_uri, 7, 23));
    assert_eq!(alias_definition, definition);
    let canonical_definition = server.handle_message(&definition_request(main_uri, 11, 30));
    assert_eq!(canonical_definition, definition);
    for include_declaration in [false, true] {
        let references = server.handle_message(&references_request_with_declaration(
            main_uri,
            11,
            30,
            include_declaration,
        ));
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].matches(&format!(r#""uri":"{main_uri}""#)).count(), 3);
        assert!(!references[0].contains("veln-pkg:"), "{}", references[0]);
    }
}

fn assert_ineligible_standard_library_handler_is_empty(server: &mut Server, main_uri: &str) {
    assert_eq!(
        server.handle_message(&definition_request(main_uri, 15, 25)),
        [response("2", "null")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            main_uri, 15, 25, true,
        )),
        [response("2", "[]")]
    );
}

fn standard_library_handler_source() -> &'static str {
    concat!(
        "pub handler noop() for Observe\n  record() => 1\nend\n\n",
        "handler hidden() for Observe\n  record() => 2\nend\n\n",
        "fn implementation() -> Int\n",
        "  # handle 0 with noop()\n",
        "  let label: String = \"handle 0 with noop()\"\n",
        "  handle 0 with noop()\n",
        "end\n",
    )
}

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
                r#"{{"uri":"{standard_uri}","range":{{"start":{{"line":100,"character":12}},"end":{{"line":100,"character":18}}}}}}"#
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
    for rejected in [
        source_path.as_ref(),
        "crates/veln-stdlib/veln/net.veln",
        "build/veln-stdlib-",
        "stdlib_bundle.rs",
        "lowered/net.veln.bin",
    ] {
        assert!(!response.contains(rejected), "leaked {rejected}: {response}");
    }
}

#[test]
fn imported_workspace_handler_navigation_preserves_utf16_and_declaration_policy() {
    let project = TempProject::new("workspace-imported-handler-references");
    write_imported_handler_project(&project);
    let root_uri = path_to_uri(&project.root);
    let handler_uri = path_to_uri(&project.root.join("handler.veln"));
    let consumer_uri = path_to_uri(&project.root.join("consumer.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    assert_imported_handler_definition(&mut server, &consumer_uri, &handler_uri);
    assert_imported_handler_references(&mut server, &consumer_uri, &handler_uri);
    assert_ineligible_imported_handlers_are_empty(&mut server, &consumer_uri);
    assert_imported_handler_definition(&mut server, &consumer_uri, &handler_uri);
}

fn write_imported_handler_project(project: &TempProject) {
    project.write("veln.toml", "");
    project.write(
        "handler.veln",
        concat!(
            "mod library::fx\r\n\r\n",
            "pub handler run() for Work\r\n  go() => 1\r\nend\r\n\r\n",
            "fn local() -> Int\r\n  \"😀\" + handle 0 with run()\r\nend\r\n",
            "\r\nhandler hidden() for Work\r\n  go() => 2\r\nend\r\n\r\n",
            "pub fn work() -> Int\r\n  4\r\nend\r\n",
        ),
    );
    project.write(
        "consumer.veln",
        concat!(
            "mod consumer\r\n\r\nuse library::fx\r\n\r\n",
            "fn full() -> Int\r\n  handle 1 with library::fx::run()\r\nend\r\n\r\n",
            "fn alias() -> Int\r\n  handle 2 with fx::run()\r\nend\r\n",
            "\r\nfn private() -> Int\r\n  handle 3 with library::fx::hidden()\r\nend\r\n\r\n",
            "fn wrong_kind() -> Int\r\n  handle 4 with library::fx::work()\r\nend\r\n",
        ),
    );
}

fn assert_imported_handler_definition(server: &mut Server, consumer_uri: &str, handler_uri: &str) {
    let definition = server.handle_message(&definition_request(consumer_uri, 5, 29));
    assert_eq!(definition.len(), 1);
    assert!(definition[0].contains(handler_uri), "{}", definition[0]);
    assert!(definition[0].contains(
        r#""range":{"start":{"line":2,"character":12},"end":{"line":2,"character":15}}"#
    ));
}

fn assert_imported_handler_references(server: &mut Server, consumer_uri: &str, handler_uri: &str) {
    let without_declaration = server.handle_message(&references_request_with_declaration(
        consumer_uri, 9, 20, false,
    ));
    assert_eq!(without_declaration.len(), 1);
    for expected in [
        format!(r#"{{"uri":"{consumer_uri}","range":{{"start":{{"line":5,"character":29}},"end":{{"line":5,"character":32}}}}}}"#),
        format!(r#"{{"uri":"{consumer_uri}","range":{{"start":{{"line":9,"character":20}},"end":{{"line":9,"character":23}}}}}}"#),
        format!(r#"{{"uri":"{handler_uri}","range":{{"start":{{"line":7,"character":23}},"end":{{"line":7,"character":26}}}}}}"#),
    ] {
        assert!(without_declaration[0].contains(&expected), "{}", without_declaration[0]);
    }

    let with_declaration = server.handle_message(&references_request_with_declaration(
        handler_uri, 2, 12, true,
    ));
    assert!(with_declaration[0].contains(&format!(
        r#"{{"uri":"{handler_uri}","range":{{"start":{{"line":2,"character":12}},"end":{{"line":2,"character":15}}}}}}"#
    )));
}

fn assert_ineligible_imported_handlers_are_empty(server: &mut Server, consumer_uri: &str) {
    assert_eq!(
        server.handle_message(&definition_request(consumer_uri, 13, 29)),
        [response("2", "null")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            consumer_uri,
            13,
            29,
            true,
        )),
        [response("2", "[]")]
    );

    assert_eq!(
        server.handle_message(&definition_request(consumer_uri, 17, 29)),
        [response("2", "null")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            consumer_uri,
            17,
            29,
            true,
        )),
        [response("2", "[]")]
    );
}

struct RejectedImportedHandlerCase {
    name: &'static str,
    manifest: &'static str,
    files: &'static [(&'static str, &'static str)],
    source: &'static str,
    line: usize,
    character: usize,
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
            line: 3,
            character: 29,
        },
        RejectedImportedHandlerCase {
            name: "ambiguous-import",
            manifest: "",
            files: &[
                ("first/fx.veln", "pub handler run() for Work\n  go() => 1\nend\n"),
                ("second/fx.veln", "pub handler run() for Work\n  go() => 2\nend\n"),
                (
                    "case.veln",
                    "use first::fx\nuse second::fx\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            character: 20,
        },
        RejectedImportedHandlerCase {
            name: "duplicate-import",
            manifest: "",
            files: &[
                ("first/fx.veln", "pub handler run() for Work\n  go() => 1\nend\n"),
                (
                    "case.veln",
                    "use first::fx\nuse first::fx\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 4,
            character: 20,
        },
        RejectedImportedHandlerCase {
            name: "recovered-import",
            manifest: "",
            files: &[
                ("first/fx.veln", "pub handler run() for Work\n  go() => 1\nend\n"),
                (
                    "case.veln",
                    "use first::fx unexpected\n\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 3,
            character: 20,
        },
        RejectedImportedHandlerCase {
            name: "duplicate-declaration",
            manifest: "",
            files: &[
                ("first.veln", "mod shared\n\nhandler run() for Work\n  go() => 1\nend\n"),
                (
                    "case.veln",
                    "mod shared\n\nhandler run() for Work\n  go() => 2\nend\n\nfn invalid() -> Int\n  handle 1 with run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 7,
            character: 16,
        },
        RejectedImportedHandlerCase {
            name: "recovered-declaration",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work @\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run()\nend\n",
            )],
            source: "case.veln",
            line: 5,
            character: 16,
        },
        RejectedImportedHandlerCase {
            name: "direct-dependency",
            manifest: "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
            files: &[
                ("vendor/dep/veln.toml", "[package]\nname = \"example/dep\"\n"),
                ("vendor/dep/dep.veln", "pub handler run() for Work\n  go() => 1\nend\n"),
                (
                    "case.veln",
                    "use dep from \"example/dep\"\n\nfn invalid() -> Int\n  handle 1 with dep::run()\nend\n",
                ),
            ],
            source: "case.veln",
            line: 3,
            character: 21,
        },
        RejectedImportedHandlerCase {
            name: "unresolved-path",
            manifest: "",
            files: &[(
                "case.veln",
                "fn invalid() -> Int\n  handle 1 with missing::run()\nend\n",
            )],
            source: "case.veln",
            line: 1,
            character: 25,
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
            line: 3,
            character: 22,
        },
        RejectedImportedHandlerCase {
            name: "incomplete-application",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run(\nend\n",
            )],
            source: "case.veln",
            line: 5,
            character: 16,
        },
        RejectedImportedHandlerCase {
            name: "recovered-application",
            manifest: "",
            files: &[(
                "case.veln",
                "handler run() for Work\n  go() => 1\nend\n\nfn invalid() -> Int\n  handle 1 with run(1 2)\nend\n",
            )],
            source: "case.veln",
            line: 5,
            character: 16,
        },
    ];

    for case in cases {
        let project = TempProject::new(case.name);
        project.write("veln.toml", case.manifest);
        project.write(
            "stable.veln",
            "mod stable\n\npub handler keep() for Work\n  go() => 0\nend\n\nfn local() -> Int\n  handle 0 with keep()\nend\n",
        );
        project.write(
            "stable_consumer.veln",
            "use stable\n\nfn valid() -> Int\n  handle 0 with stable::keep()\nend\n",
        );
        for (path, source) in case.files {
            project.write(path, source);
        }

        let root_uri = path_to_uri(&project.root);
        let case_uri = path_to_uri(&project.root.join(case.source));
        let stable_declaration_uri = path_to_uri(&project.root.join("stable.veln"));
        let stable_uri = path_to_uri(&project.root.join("stable_consumer.veln"));
        let mut server = server_with_empty_standard_library();
        server.handle_message(&initialize_request(&root_uri));
        let valid_definition = definition_request(&stable_uri, 3, 24);
        let valid_references = references_request_with_declaration(&stable_uri, 3, 24, true);
        let definition_before = server.handle_message(&valid_definition);
        let references_before = server.handle_message(&valid_references);
        assert_eq!(definition_before.len(), 1, "{} valid definition", case.name);
        assert!(
            definition_before[0].contains(&stable_declaration_uri),
            "{} valid definition: {}",
            case.name,
            definition_before[0]
        );
        assert_eq!(references_before.len(), 1, "{} valid references", case.name);
        assert!(
            references_before[0].matches("\"uri\"").count() == 3,
            "{} valid references: {}",
            case.name,
            references_before[0]
        );

        assert_eq!(
            server.handle_message(&definition_request(
                &case_uri,
                case.line,
                case.character,
            )),
            [response("2", "null")],
            "{} definition",
            case.name,
        );
        assert_eq!(
            server.handle_message(&references_request_with_declaration(
                &case_uri,
                case.line,
                case.character,
                true,
            )),
            [response("2", "[]")],
            "{} references",
            case.name,
        );
        assert_eq!(server.handle_message(&valid_definition), definition_before, "{} retained definition", case.name);
        assert_eq!(server.handle_message(&valid_references), references_before, "{} retained references", case.name);
    }
}

#[test]
fn workspace_handler_references_include_a_declaration_without_occurrences() {
    let project = TempProject::new("workspace-handler-declaration-only-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        "handler run() for Work\n  go() => 1\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let without_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 0, 8, false,
    ));
    assert_eq!(without_declaration, [response("2", "[]")]);

    let with_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 0, 8, true,
    ));
    assert_eq!(
        with_declaration,
        [response(
            "2",
            &format!(
                "[{{\"uri\":\"{main_uri}\",\"range\":{{\"start\":{{\"line\":0,\"character\":8}},\"end\":{{\"line\":0,\"character\":11}}}}}}]"
            ),
        )]
    );
}

#[test]
fn workspace_handler_reference_failures_preserve_later_results() {
    let project = TempProject::new("workspace-handler-reference-failure-state");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        concat!(
            "handler run() for Work\n  go() => 1\nend\n\n",
            "fn use() -> Int\n  handle 1 with run()\nend\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let missing_uri = path_to_uri(&project.root.join("missing.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));
    let request = references_request_with_declaration(&main_uri, 0, 8, true);
    let before = server.handle_message(&request);

    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            &main_uri, 99, 0, true,
        )),
        [invalid_navigation_position_response("2")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            &missing_uri,
            0,
            0,
            true,
        )),
        [response("2", "[]")]
    );
    assert_eq!(server.handle_message(&request), before);
}
