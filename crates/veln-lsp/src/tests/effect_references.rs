#[test]
fn workspace_effect_references_preserve_utf16_crlf_and_declaration_policy() {
    let project = TempProject::new("workspace-effect-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        "effect Choose\r\n  pick() -> Int\r\nend\r\n\r\nfn choose() -> Int effects [Choose]\r\n  \"😀\" + perform Choose::pick()\r\nend\r\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let without_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 17, false,
    ));
    assert_eq!(
        without_declaration,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":4,\"character\":28}},\"end\":{{\"line\":4,\"character\":34}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":17}},\"end\":{{\"line\":5,\"character\":23}}}}}}]"
                ),
                main_uri,
                main_uri,
            ),
        )]
    );

    let with_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 0, 7, true,
    ));
    assert_eq!(with_declaration[0].matches(&main_uri).count(), 3);
    assert!(with_declaration[0].contains(
        r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":13}}"#
    ));
}
