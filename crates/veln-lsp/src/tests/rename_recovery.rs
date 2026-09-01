#[test]
fn recovery_rename_edits_declaration_and_linked_references() {
    let mut server = Server::default();
    let project = TempProject::new("rename-recovery-symbol-success");
    project.write(
        "main.veln",
        concat!(
            "type item\n",
            "  value(input: Int)\n",
            "end\n\n",
            "fn Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read_type(value: item) -> item\n",
            "  value\n",
            "end\n\n",
            "fn read_constructor() -> item\n",
            "  value(1)\n",
            "end\n\n",
            "fn read_function() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read_callback(Callback: fn() -> Int) -> Int\n",
            "  Callback\n",
            "  Callback()\n",
            "end\n\n",
            "fn read_local(input: Int) -> Int\n",
            "  let Local = input\n",
            "  Local\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let type_rename = server.handle_message(&rename_request(&main_uri, 8, 21, "Entry"));
    let constructor_rename = server.handle_message(&rename_request(&main_uri, 13, 4, "Value"));
    let function_rename = server.handle_message(&rename_request(&main_uri, 17, 4, "good"));
    let callable_rename = server.handle_message(&rename_request(&main_uri, 21, 4, "callback"));
    let local_rename = server.handle_message(&rename_request(&main_uri, 27, 4, "local"));

    assert_eq!(type_rename[0].matches(r#""newText":"Entry""#).count(), 4);
    assert_eq!(
        constructor_rename[0]
            .matches(r#""newText":"Value""#)
            .count(),
        2
    );
    assert_eq!(function_rename[0].matches(r#""newText":"good""#).count(), 3);
    assert_eq!(
        callable_rename[0]
            .matches(r#""newText":"callback""#)
            .count(),
        3
    );
    assert_eq!(local_rename[0].matches(r#""newText":"local""#).count(), 2);
    assert!(
        !local_rename[0].contains(r#""line":26,"character":14"#),
        "{}",
        local_rename[0]
    );
}

#[test]
fn recovery_rename_failures_return_no_workspace_edits() {
    let mut server = Server::default();
    let project = TempProject::new("rename-recovery-symbol-failure");
    project.write(
        "main.veln",
        concat!(
            "type item\n",
            "  value(input: Int)\n",
            "  Ready\n",
            "end\n\n",
            "type Entry\n",
            "  Existing\n",
            "end\n\n",
            "fn Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let invalid_case = server.handle_message(&rename_request(&main_uri, 0, 5, "entry"));
    let conflict = server.handle_message(&rename_request(&main_uri, 9, 3, "good"));

    assert!(
        invalid_case[0].contains(r#""code":"rename.invalid_case""#),
        "{}",
        invalid_case[0]
    );
    assert!(!invalid_case[0].contains(r#""changes""#), "{}", invalid_case[0]);
    assert!(
        conflict[0].contains(r#""code":"rename.conflict""#),
        "{}",
        conflict[0]
    );
    assert!(!conflict[0].contains(r#""changes""#), "{}", conflict[0]);
}
