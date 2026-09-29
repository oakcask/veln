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

    let prepared_begin_local = server.handle_message(&prepare_rename_request(&main_uri, 11, 6));
    assert_eq!(prepared_begin_local.len(), 1);
    assert!(
        prepared_begin_local[0].contains(
            r#""result":{"start":{"line":11,"character":2},"end":{"line":11,"character":10}}"#
        ),
        "{}",
        prepared_begin_local[0]
    );

    let prepared_deferred_use =
        server.handle_message(&prepare_rename_request(&main_uri, 9, 13));
    assert_eq!(prepared_deferred_use.len(), 1);
    assert!(
        prepared_deferred_use[0].contains(
            r#""result":{"start":{"line":9,"character":11},"end":{"line":9,"character":19}}"#
        ),
        "{}",
        prepared_deferred_use[0]
    );

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

#[test]
fn cleanup_region_navigation_uses_innermost_shadowing_binding() {
    let mut server = Server::default();
    let project = TempProject::new("cleanup-region-shadowing-navigation");
    let source = concat!(
        "fn read(value: Int) -> Int\n",
        "  let value = value\n",
        "  begin\n",
        "    let value = value\n",
        "    defer\n",
        "      let value = value\n",
        "      value\n",
        "    end\n",
        "    value\n",
        "  end\n",
        "  value\n",
        "end\n",
    );
    project.write("main.veln", source);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let cases = [
        (1, 14, 0, 8, 13, 2),
        (10, 4, 1, 6, 11, 3),
        (8, 6, 3, 8, 13, 3),
        (6, 8, 5, 10, 15, 2),
    ];
    for (line, character, declaration_line, declaration_start, declaration_end, edit_count) in cases
    {
        let expected_definition = format!(
            r#""range":{{"start":{{"line":{declaration_line},"character":{declaration_start}}},"end":{{"line":{declaration_line},"character":{declaration_end}}}}}"#
        );
        let definition = server.handle_message(&definition_request(&main_uri, line, character));
        assert_eq!(definition.len(), 1, "{line}:{character}");
        assert!(
            definition[0].contains(&expected_definition),
            "{line}:{character}: {}",
            definition[0]
        );

        let references = server.handle_message(&references_request(&main_uri, line, character));
        assert_eq!(references.len(), 1, "{line}:{character}");
        assert_eq!(
            references[0].matches(r#""uri":"file:"#).count(),
            edit_count,
            "{line}:{character}: {}",
            references[0]
        );

        let rename = server.handle_message(&rename_request(&main_uri, line, character, "renamed"));
        assert_eq!(rename.len(), 1, "{line}:{character}");
        assert_eq!(
            rename[0].matches(r#""newText":"renamed""#).count(),
            edit_count,
            "{line}:{character}: {}",
            rename[0]
        );
    }
}

#[test]
fn cleanup_region_type_annotations_support_function_and_handler_navigation() {
    let mut server = Server::default();
    let project = TempProject::new("cleanup-region-type-navigation");
    let source = concat!(
        "type Resource\n",
        "  Ready\n",
        "end\n",
        "\n",
        "effect Ask\n",
        "  value() -> Resource\n",
        "end\n",
        "\n",
        "fn work(input: Resource) -> Resource\n",
        "  defer\n",
        "    let deferred: Resource = input\n",
        "    ()\n",
        "  end\n",
        "  let begun: Resource = begin\n",
        "    let nested: Resource = input\n",
        "    nested\n",
        "  end\n",
        "  begun\n",
        "end\n",
        "\n",
        "handler ask(seed: Resource) handles Ask\n",
        "  value() => begin\n",
        "    defer\n",
        "      let deferred: Resource = seed\n",
        "      ()\n",
        "    end\n",
        "    let clause: Resource = seed\n",
        "    clause\n",
        "  end\n",
        "end\n",
    );
    project.write("main.veln", source);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(10, 20), (14, 18), (23, 24), (26, 18)] {
        let definition = server.handle_message(&definition_request(&main_uri, line, character));
        assert_eq!(definition.len(), 1, "{line}:{character}");
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":0,"character":5},"end":{"line":0,"character":13}}"#
            ),
            "{line}:{character}: {}",
            definition[0]
        );
    }

    let references = server.handle_message(&references_request(&main_uri, 26, 18));
    assert_eq!(references.len(), 1);
    for range in [
        r#""range":{"start":{"line":10,"character":18},"end":{"line":10,"character":26}}"#,
        r#""range":{"start":{"line":14,"character":16},"end":{"line":14,"character":24}}"#,
        r#""range":{"start":{"line":23,"character":20},"end":{"line":23,"character":28}}"#,
        r#""range":{"start":{"line":26,"character":16},"end":{"line":26,"character":24}}"#,
    ] {
        assert!(references[0].contains(range), "{range}: {}", references[0]);
    }

    let rename = server.handle_message(&rename_request(&main_uri, 26, 18, "Handle"));
    assert_eq!(rename.len(), 1);
    assert_eq!(rename[0].matches(r#""newText":"Handle""#).count(), 10);
}
