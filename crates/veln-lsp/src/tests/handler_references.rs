#[test]
fn workspace_handler_references_preserve_utf16_crlf_and_declaration_policy() {
    let project = TempProject::new("workspace-handler-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        concat!(
            "handler run(value: Int) handles Work\r\n",
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
fn workspace_handler_references_reject_ambiguous_recovered_and_non_bare_paths() {
    let project = TempProject::new("workspace-handler-reference-boundaries");
    project.write("veln.toml", "");
    project.write(
        "first.veln",
        "mod shared\n\nhandler run() handles Work\n  go() => 1\nend\n",
    );
    project.write(
        "second.veln",
        concat!(
            "mod shared\n\n",
            "handler run() handles Work\n  go() => 2\nend\n\n",
            "handler keep() handles Work @\n  go() => 3\nend\n\n",
            "fn use() -> Int\n  handle 1 with run()\nend\n\n",
            "fn qualified() -> Int\n  handle 1 with other::run()\nend\n\n",
            "fn recovered() -> Int\n  handle 1 with keep(1 2)\nend\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let first_uri = path_to_uri(&project.root.join("first.veln"));
    let second_uri = path_to_uri(&project.root.join("second.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for request in [
        references_request_with_declaration(&first_uri, 2, 8, true),
        references_request_with_declaration(&second_uri, 6, 8, true),
        references_request_with_declaration(&second_uri, 11, 16, true),
        references_request_with_declaration(&second_uri, 15, 23, true),
        references_request_with_declaration(&second_uri, 19, 17, true),
    ] {
        assert_eq!(server.handle_message(&request), [response("2", "[]")]);
    }
}

#[test]
fn workspace_handler_references_reject_each_invalid_path_without_losing_valid_selection() {
    let project = TempProject::new("workspace-handler-reference-invalid-paths");
    project.write("veln.toml", "");
    project.write(
        "declaration.veln",
        "mod shared\n\nhandler stable() handles Work\n  go() => 1\nend\n",
    );
    for (path, expression) in [
        ("unresolved.veln", "missing()"),
        ("incomplete.veln", "stable("),
        ("recovered.veln", "stable(1 2)"),
    ] {
        project.write(
            path,
            &format!("mod shared\n\nfn use() -> Int\n  handle 1 with {expression}\nend\n"),
        );
    }
    project.write(
        "valid.veln",
        "mod shared\n\nfn use() -> Int\n  handle 1 with stable()\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let valid_uri = path_to_uri(&project.root.join("valid.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for path in ["unresolved.veln", "incomplete.veln", "recovered.veln"] {
        let uri = path_to_uri(&project.root.join(path));
        assert_eq!(
            server.handle_message(&references_request_with_declaration(&uri, 3, 16, true)),
            [response("2", "[]")],
            "{path}",
        );
    }
    assert_eq!(
        server.handle_message(&references_request_with_declaration(
            &valid_uri, 3, 16, false,
        )),
        [response(
            "2",
            &format!(
                "[{{\"uri\":\"{valid_uri}\",\"range\":{{\"start\":{{\"line\":3,\"character\":16}},\"end\":{{\"line\":3,\"character\":22}}}}}}]"
            ),
        )],
    );
}

#[test]
fn workspace_handler_references_reject_resolved_qualified_workspace_and_package_paths() {
    let project = TempProject::new("workspace-handler-reference-resolved-qualified");
    project.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    project.write(
        "main.veln",
        concat!(
            "use other\n",
            "use dep from \"example/dep\"\n\n",
            "handler stable() handles Work\n  go() => 1\nend\n\n",
            "fn valid() -> Int\n  handle 1 with stable()\nend\n\n",
            "fn workspace_import() -> Int\n  handle 2 with other::run()\nend\n\n",
            "fn package_import() -> Int\n  handle 3 with dep::run()\nend\n",
        ),
    );
    project.write(
        "other.veln",
        "pub handler run() handles Work\n  go() => 2\nend\n",
    );
    project.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    project.write(
        "vendor/dep/dep.veln",
        "pub handler run() handles Work\n  go() => 3\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(12, 23), (16, 21)] {
        assert_eq!(
            server.handle_message(&references_request_with_declaration(
                &main_uri, line, character, true,
            )),
            [response("2", "[]")],
        );
    }
    let valid = server.handle_message(&references_request_with_declaration(
        &main_uri, 8, 16, false,
    ));
    assert!(valid[0].contains("\"line\":8,\"character\":16"));
}

#[test]
fn workspace_handler_references_filter_other_modules_and_symbol_classes_at_adapter() {
    let project = TempProject::new("workspace-handler-reference-collisions");
    project.write("veln.toml", "");
    project.write(
        "selected.veln",
        concat!(
            "mod shared\n\n",
            "handler run() handles Work\n  go() => 1\nend\n\n",
            "fn use() -> Int\n  handle 1 with run()\nend\n",
        ),
    );
    project.write(
        "other.veln",
        concat!(
            "mod other\n\n",
            "handler run() handles Work\n  go() => 2\nend\n\n",
            "fn use() -> Int\n  handle 2 with run()\nend\n",
        ),
    );
    project.write(
        "symbols.veln",
        concat!(
            "mod shared\n\n",
            "effect run\n  run() -> Int\nend\n\n",
            "type run\n  run\nend\n\n",
            "fn run() -> Int\n  1\nend\n\n",
            "handler wrapper(run: Int) handles Work\n",
            "  go(run: Int) => run\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let selected_uri = path_to_uri(&project.root.join("selected.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));
    let result = server.handle_message(&references_request_with_declaration(
        &selected_uri, 2, 8, false,
    ));
    assert_eq!(
        result,
        [response(
            "2",
            &format!(
                "[{{\"uri\":\"{selected_uri}\",\"range\":{{\"start\":{{\"line\":7,\"character\":16}},\"end\":{{\"line\":7,\"character\":19}}}}}}]"
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
            "handler run() handles Work\n  go() => 1\nend\n\n",
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
        [response("2", "[]")]
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
