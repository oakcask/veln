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
fn imported_workspace_handler_navigation_preserves_utf16_and_declaration_policy() {
    let project = TempProject::new("workspace-imported-handler-references");
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
    let root_uri = path_to_uri(&project.root);
    let handler_uri = path_to_uri(&project.root.join("handler.veln"));
    let consumer_uri = path_to_uri(&project.root.join("consumer.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&consumer_uri, 5, 29));
    assert_eq!(definition.len(), 1);
    assert!(definition[0].contains(&handler_uri), "{}", definition[0]);
    assert!(definition[0].contains(
        r#""range":{"start":{"line":2,"character":12},"end":{"line":2,"character":15}}"#
    ));

    let without_declaration = server.handle_message(&references_request_with_declaration(
        &consumer_uri, 9, 20, false,
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
        &handler_uri, 2, 12, true,
    ));
    assert!(with_declaration[0].contains(&format!(
        r#"{{"uri":"{handler_uri}","range":{{"start":{{"line":2,"character":12}},"end":{{"line":2,"character":15}}}}}}"#
    )));

    assert_eq!(
        server.handle_message(&definition_request(&consumer_uri, 13, 29)),
        [response("2", "null")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            &consumer_uri,
            13,
            29,
            true,
        )),
        [response("2", "[]")]
    );

    assert_eq!(
        server.handle_message(&definition_request(&consumer_uri, 17, 29)),
        [response("2", "null")]
    );
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            &consumer_uri,
            17,
            29,
            true,
        )),
        [response("2", "[]")]
    );

    let retained = server.handle_message(&definition_request(&consumer_uri, 9, 20));
    assert_eq!(retained.len(), 1);
    assert!(retained[0].contains(&handler_uri), "{}", retained[0]);
    assert!(retained[0].contains(
        r#""range":{"start":{"line":2,"character":12},"end":{"line":2,"character":15}}"#
    ));
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
            name: "standard-library",
            manifest: "",
            files: &[(
                "case.veln",
                "use observe from \"std\"\n\nfn invalid() -> Int\n  handle 1 with observe::noop()\nend\n",
            )],
            source: "case.veln",
            line: 3,
            character: 24,
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
        let mut server = Server::default();
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
