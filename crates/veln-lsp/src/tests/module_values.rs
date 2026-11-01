#[test]
fn module_value_protocol_navigation_and_rename_share_identity() {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("module-value-navigation");
    project.write(
        "values.veln",
        "pub let answer: Int = 42\nlet direct: Int = answer\n",
    );
    project.write(
        "main.veln",
        concat!(
            "use values\n",
            "use other as equal\n",
            "use functions as callable\n\n",
            "let qualified: Int = values::answer\n",
            "let equal_data: Int = equal::answer\n",
            "let same_spelling: Int = callable::answer()\n",
        ),
    );
    project.write("other.veln", "pub let answer: Int = 42\n");
    project.write(
        "functions.veln",
        "pub fn answer() -> Int\n  42\nend\n",
    );
    project.write(
        "alias.veln",
        "use values as config\n\nlet aliased: Int = config::answer\n",
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let alias_uri = path_to_uri(&project.root.join("alias.veln"));
    let other_uri = path_to_uri(&project.root.join("other.veln"));
    let functions_uri = path_to_uri(&project.root.join("functions.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (uri, line, character, start) in [
        (&values_uri, 0, 10, 8),
        (&values_uri, 1, 19, 18),
        (&main_uri, 4, 30, 29),
        (&alias_uri, 2, 29, 27),
    ] {
        let definition = server.handle_message(&definition_request(uri, line, character));
        assert_eq!(definition.len(), 1);
        assert!(definition[0].contains(&values_uri), "{}", definition[0]);
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":0,"character":8},"end":{"line":0,"character":14}}"#
            ),
            "{}",
            definition[0]
        );

        let references = server.handle_message(&references_request_with_declaration(
            uri, line, character, true,
        ));
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].matches(r#""uri":"#).count(), 4, "{}", references[0]);
        for expected_uri in [&values_uri, &main_uri, &alias_uri] {
            assert!(references[0].contains(expected_uri), "{}", references[0]);
        }

        let prepared = server.handle_message(&prepare_rename_request(uri, line, character));
        assert!(
            prepared[0].contains(&format!(
                r#""result":{{"start":{{"line":{line},"character":{start}}},"end":{{"line":{line},"character":{}}}}}"#,
                start + 6
            )),
            "{}",
            prepared[0]
        );

        let renamed = server.handle_message(&rename_request(uri, line, character, "result"));
        for expected_uri in [&values_uri, &main_uri, &alias_uri] {
            assert!(renamed[0].contains(expected_uri), "{}", renamed[0]);
        }
        assert_eq!(renamed[0].matches(r#""newText":"result""#).count(), 4);
    }

    let equal_definition = server.handle_message(&definition_request(&main_uri, 5, 31));
    assert!(equal_definition[0].contains(&other_uri), "{}", equal_definition[0]);
    assert!(!equal_definition[0].contains(&values_uri), "{}", equal_definition[0]);
    let equal_references = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 31, true,
    ));
    assert_eq!(equal_references[0].matches(r#""uri":"#).count(), 2);
    assert!(!equal_references[0].contains(&values_uri));

    let function_definition = server.handle_message(&definition_request(&main_uri, 6, 39));
    assert!(
        function_definition[0].contains(&functions_uri),
        "{}",
        function_definition[0]
    );
    assert!(!function_definition[0].contains(&values_uri));
}

#[test]
fn package_distinct_module_value_protocol_navigation_keeps_identity() {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("package-module-value-navigation");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"one/pkg\"]\npath = \"vendor/one\"\n\n",
            "[dependencies.\"two/pkg\"]\npath = \"vendor/two\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use shared as first from \"one/pkg\"\n",
            "use shared as second from \"two/pkg\"\n\n",
            "let left: Int = first::answer\n",
            "let right: Int = second::answer\n",
        ),
    );
    for (directory, package) in [("one", "one/pkg"), ("two", "two/pkg")] {
        project.write(
            &format!("vendor/{directory}/veln.toml"),
            &format!(
                "[package]\nname = \"{package}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"
            ),
        );
        project.write(
            &format!("vendor/{directory}/shared.veln"),
            "pub let answer: Int = 42\n",
        );
    }
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let first = server.handle_message(&definition_request(&main_uri, 3, 25));
    let second = server.handle_message(&definition_request(&main_uri, 4, 27));
    assert!(first[0].contains("veln-pkg:///one%2Fpkg/"), "{}", first[0]);
    assert!(second[0].contains("veln-pkg:///two%2Fpkg/"), "{}", second[0]);
    let first_references = server.handle_message(&references_request_with_declaration(
        &main_uri, 3, 25, true,
    ));
    assert_eq!(
        first_references[0].matches(r#""uri":"#).count(),
        1,
        "{}",
        first_references[0]
    );
    assert!(first_references[0].contains(&main_uri));
    assert!(first_references[0].contains(r#""line":3"#));
    assert!(!first_references[0].contains(r#""line":4"#));
    assert!(!first_references[0].contains("veln-pkg:///two%2Fpkg/"));

    let prepared = server.handle_message(&prepare_rename_request(&main_uri, 3, 25));
    assert_null_result(&prepared[0]);
    let renamed = server.handle_message(&rename_request(&main_uri, 3, 25, "result"));
    assert_contains_json(&renamed[0], r#""changes":{}"#);
}

#[test]
fn module_value_protocol_rename_rejects_namespace_and_capture_conflicts() {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("module-value-rename-conflicts");
    project.write(
        "values.veln",
        concat!(
            "pub let answer: Int = 42\n",
            "pub let existing: Int = 1\n",
            "pub fn callable() -> Int\n",
            "  1\n",
            "end\n",
            "fn read() -> Int\n",
            "  let captured = 1\n",
            "  answer\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character, requested, symbol_class, scope) in [
        (0, 10, "existing", "value_binding", r#""kind":"module""#),
        (0, 10, "callable", "value_binding", r#""kind":"module""#),
        (2, 9, "existing", "function", r#""kind":"module""#),
        (0, 10, "captured", "value_binding", r#""kind":"lexical""#),
    ] {
        let response = server.handle_message(&rename_request(
            &values_uri,
            line,
            character,
            requested,
        ));
        assert_eq!(response.len(), 1);
        assert!(response[0].contains(r#""code":-32602"#), "{}", response[0]);
        assert!(
            response[0].contains(r#""code":"rename.conflict""#),
            "{}",
            response[0]
        );
        assert!(
            response[0].contains(&format!(r#""symbol_class":"{symbol_class}""#)),
            "{}",
            response[0]
        );
        assert!(
            response[0].contains(&format!(r#""requested_name":"{requested}""#)),
            "{}",
            response[0]
        );
        assert!(response[0].contains(scope), "{}", response[0]);
        assert!(!response[0].contains(r#""changes""#), "{}", response[0]);
    }
}
