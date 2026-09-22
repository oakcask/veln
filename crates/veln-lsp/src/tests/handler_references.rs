#[test]
fn workspace_handler_references_preserve_utf16_crlf_and_declaration_policy() {
    let project = TempProject::new("workspace-handler-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        concat!(
            "handler run() handles Work\r\n",
            "  go() => 1\r\n",
            "end\r\n\r\n",
            "fn first() -> Int\r\n",
            "  \"😀\" + handle 1 with run()\r\n",
            "end\r\n\r\n",
            "fn second() -> Int\r\n",
            "  handle 2 with run()\r\n",
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
