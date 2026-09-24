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
fn workspace_handler_references_include_a_declaration_without_occurrences() {
    let project = TempProject::new("workspace-handler-declaration-only-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        "handler run() handles Work\n  go() => 1\nend\n",
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
