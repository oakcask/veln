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
        "use values\n\nlet qualified: Int = values::answer\n",
    );
    project.write(
        "alias.veln",
        "use values as config\n\nlet aliased: Int = config::answer\n",
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let alias_uri = path_to_uri(&project.root.join("alias.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 2, 30));
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
        &alias_uri, 2, 29, true,
    ));
    assert_eq!(references.len(), 1);
    for uri in [&values_uri, &main_uri, &alias_uri] {
        assert!(references[0].contains(uri), "{}", references[0]);
    }

    let prepared = server.handle_message(&prepare_rename_request(&alias_uri, 2, 29));
    assert!(
        prepared[0].contains(
            r#""result":{"start":{"line":2,"character":27},"end":{"line":2,"character":33}}"#
        ),
        "{}",
        prepared[0]
    );

    let renamed = server.handle_message(&rename_request(&alias_uri, 2, 29, "result"));
    for uri in [&values_uri, &main_uri, &alias_uri] {
        assert!(renamed[0].contains(uri), "{}", renamed[0]);
    }
    assert_eq!(renamed[0].matches(r#""newText":"result""#).count(), 4);
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
    assert!(first_references[0].contains(r#""line":3"#));
    assert!(!first_references[0].contains(r#""line":4"#));

    let prepared = server.handle_message(&prepare_rename_request(&main_uri, 3, 25));
    assert_null_result(&prepared[0]);
    let renamed = server.handle_message(&rename_request(&main_uri, 3, 25, "result"));
    assert_contains_json(&renamed[0], r#""changes":{}"#);
}
