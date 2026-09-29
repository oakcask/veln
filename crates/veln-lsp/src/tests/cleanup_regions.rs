#[test]
fn cleanup_regions_format_stably_and_preserve_navigation_ranges() {
    let mut server = Server::default();
    let project = TempProject::new("cleanup-region-navigation");
    let source = concat!(
        "fn cleanup(value: Int) -> ()\n",
        "\t()\n",
        "end\n",
        "\n",
        "fn choose(input: Int) -> Int\n",
        "\tlet outer = input\n",
        "\tlet result = begin\n",
        "\t\tlet captured = outer\n",
        "\t\tdefer\n",
        "\t\t\tcleanup(captured)\n",
        "\t\tend\n",
        "\t\tcaptured\n",
        "\tend\n",
        "\tresult\n",
        "end\n",
    );
    project.write("main.veln", source);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 9, 13));
    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":7,"character":6},"end":{"line":7,"character":14}}"#
        ),
        "{}",
        definition[0]
    );

    let references = server.handle_message(&references_request(&main_uri, 9, 13));
    assert_eq!(references.len(), 1);
    for range in [
        r#""range":{"start":{"line":7,"character":6},"end":{"line":7,"character":14}}"#,
        r#""range":{"start":{"line":9,"character":11},"end":{"line":9,"character":19}}"#,
        r#""range":{"start":{"line":11,"character":2},"end":{"line":11,"character":10}}"#,
    ] {
        assert!(references[0].contains(range), "{range}: {}", references[0]);
    }

    let rename = server.handle_message(&rename_request(&main_uri, 9, 13, "saved"));
    assert_eq!(rename.len(), 1);
    assert_eq!(rename[0].matches(r#""newText":"saved""#).count(), 3);

    let formatting = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":{{"textDocument":{{"uri":"{main_uri}"}},"options":{{"tabSize":2,"insertSpaces":true}}}}}}"#
    ));
    assert_eq!(formatting, [response("3", "[]")]);
}
