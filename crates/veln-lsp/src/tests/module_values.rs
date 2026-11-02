fn assert_module_value_definition(
    server: &mut Server,
    uri: &str,
    line: usize,
    character: usize,
    values_uri: &str,
) {
    let definition = server.handle_message(&definition_request(uri, line, character));
    assert_eq!(definition.len(), 1);
    assert!(definition[0].contains(values_uri), "{}", definition[0]);
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":0,"character":8},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        definition[0]
    );
}

fn assert_module_value_references(
    server: &mut Server,
    uri: &str,
    line: usize,
    character: usize,
    expected_uris: &[&str],
) {
    let references = server.handle_message(&references_request_with_declaration(
        uri, line, character, true,
    ));
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].matches(r#""uri":"#).count(), 4, "{}", references[0]);
    for expected_uri in expected_uris {
        assert!(references[0].contains(expected_uri), "{}", references[0]);
    }
}

fn assert_module_value_rename(
    server: &mut Server,
    uri: &str,
    line: usize,
    character: usize,
    start: usize,
    expected_uris: &[&str],
) {
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
    for expected_uri in expected_uris {
        assert!(renamed[0].contains(expected_uri), "{}", renamed[0]);
    }
    assert_eq!(renamed[0].matches(r#""newText":"result""#).count(), 4);
}

struct ModuleValueNavigationUris {
    root: String,
    values: String,
    main: String,
    alias: String,
    other: String,
    functions: String,
}

impl ModuleValueNavigationUris {
    fn new(project: &TempProject) -> Self {
        Self {
            root: path_to_uri(&project.root),
            values: path_to_uri(&project.root.join("values.veln")),
            main: path_to_uri(&project.root.join("main.veln")),
            alias: path_to_uri(&project.root.join("alias.veln")),
            other: path_to_uri(&project.root.join("other.veln")),
            functions: path_to_uri(&project.root.join("functions.veln")),
        }
    }
}

fn write_module_value_navigation_project(project: &TempProject) {
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
}

fn module_value_navigation_server() -> (Server, TempProject, ModuleValueNavigationUris) {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("module-value-navigation");
    write_module_value_navigation_project(&project);
    let uris = ModuleValueNavigationUris::new(&project);
    server.handle_message(&initialize_request(&uris.root));
    (server, project, uris)
}

fn assert_distinct_equal_value_and_function_identities(
    server: &mut Server,
    uris: &ModuleValueNavigationUris,
) {
    let equal_definition = server.handle_message(&definition_request(&uris.main, 5, 31));
    assert!(equal_definition[0].contains(&uris.other), "{}", equal_definition[0]);
    assert!(!equal_definition[0].contains(&uris.values), "{}", equal_definition[0]);
    let equal_references = server.handle_message(&references_request_with_declaration(
        &uris.main, 5, 31, true,
    ));
    assert_eq!(equal_references[0].matches(r#""uri":"#).count(), 2);
    assert!(!equal_references[0].contains(&uris.values));

    let function_definition = server.handle_message(&definition_request(&uris.main, 6, 39));
    assert!(
        function_definition[0].contains(&uris.functions),
        "{}",
        function_definition[0]
    );
    assert!(!function_definition[0].contains(&uris.values));
}

#[test]
fn module_value_protocol_navigation_and_rename_share_identity() {
    let (mut server, _project, uris) = module_value_navigation_server();

    for (uri, line, character, start) in [
        (&uris.values, 0, 10, 8),
        (&uris.values, 1, 19, 18),
        (&uris.main, 4, 30, 29),
        (&uris.alias, 2, 29, 27),
    ] {
        let expected_uris = [uris.values.as_str(), uris.main.as_str(), uris.alias.as_str()];
        assert_module_value_definition(&mut server, uri, line, character, &uris.values);
        assert_module_value_references(&mut server, uri, line, character, &expected_uris);
        assert_module_value_rename(
            &mut server,
            uri,
            line,
            character,
            start,
            &expected_uris,
        );
    }
    assert_distinct_equal_value_and_function_identities(&mut server, &uris);
}

#[test]
fn standard_library_module_values_are_unsupported_by_navigation_protocols() {
    let manifest = "[package]\nname = \"std\"\n\n[lib]\nexports = [\"values.veln\"]\n";
    let standard_library = DirectDependencySnapshot::from_validated_standard_library(
        capture_embedded_package_snapshot(
            manifest.as_bytes(),
            [PackageSnapshotSource::new(
                "values.veln",
                b"pub let answer: Int = 42\n",
            )],
        )
        .unwrap(),
        parse_manifest_text("veln.toml", manifest),
    )
    .unwrap();
    let mut server = Server::default().with_standard_library(standard_library);
    let project = TempProject::new("standard-library-module-value-navigation-boundary");
    project.write(
        "main.veln",
        concat!(
            "use values from \"std\"\n\n",
            "let direct: Int = answer\n",
            "let qualified: Int = values::answer\n",
        ),
    );
    project.write(
        "alias.veln",
        concat!(
            "use values as config from \"std\"\n\n",
            "let aliased: Int = config::answer\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let alias_uri = path_to_uri(&project.root.join("alias.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (uri, line, character) in [
        (&main_uri, 2, 18),
        (&main_uri, 3, 29),
        (&alias_uri, 2, 28),
    ] {
        assert_null_result(
            &server.handle_message(&definition_request(uri, line, character))[0],
        );
        assert_empty_result_array(
            &server.handle_message(&references_request_with_declaration(
                uri, line, character, true,
            ))[0],
        );
    }
}

#[test]
fn alias_qualified_module_value_hover_is_unsupported() {
    let (mut server, _project, uris) = module_value_navigation_server();
    let hover = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":91,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":2,"character":29}}}}}}"#,
        uris.alias
    ));
    assert_null_result(&hover[0]);
}

#[test]
fn qualified_module_value_protocol_selection_precedes_lexical_bindings() {
    let (mut server, _project, values_uri, main_uri) =
        qualified_module_value_lexical_shadowing_server("answer");

    for (line, character) in [(3, 11), (8, 11)] {
        assert_qualified_module_value_navigation(
            &mut server,
            &main_uri,
            &values_uri,
            line,
            character,
        );
    }

    let (mut renamed_server, _renamed_project, renamed_values, renamed_main) =
        qualified_module_value_lexical_shadowing_server("renamed");
    let definition =
        renamed_server.handle_message(&definition_request(&renamed_main, 3, 11));
    assert!(definition[0].contains(&renamed_values), "{}", definition[0]);
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":0,"character":8},"end":{"line":0,"character":15}}"#
        ),
        "{}",
        definition[0]
    );
}

fn qualified_module_value_lexical_shadowing_server(
    value_name: &str,
) -> (Server, TempProject, String, String) {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("qualified-module-value-lexical-shadowing");
    project.write(
        "values.veln",
        &format!("pub let {value_name}: Int = 42\n"),
    );
    project.write(
        "main.veln",
        &format!(
            "use values\n\n\
             fn from_parameter(answer: Int) -> Int\n\
               values::{value_name} + answer\n\
             end\n\n\
             fn from_local() -> Int\n\
               let answer = 1\n\
               values::{value_name} + answer\n\
             end\n"
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    (server, project, values_uri, main_uri)
}

fn assert_qualified_module_value_navigation(
    server: &mut Server,
    main_uri: &str,
    values_uri: &str,
    line: usize,
    character: usize,
) {
    assert_module_value_definition(server, main_uri, line, character, values_uri);
    let references = server.handle_message(&references_request_with_declaration(
        main_uri, line, character, true,
    ));
    assert_eq!(references[0].matches(r#""uri":"#).count(), 3, "{}", references[0]);
    let prepared = server.handle_message(&prepare_rename_request(main_uri, line, character));
    assert!(prepared[0].contains(r#""start":{"line":"#), "{}", prepared[0]);
    let renamed = server.handle_message(&rename_request(
        main_uri,
        line,
        character,
        "renamed",
    ));
    assert_eq!(
        renamed[0].matches(r#""newText":"renamed""#).count(),
        3
    );
}

#[test]
fn package_distinct_module_value_protocol_navigation_keeps_identity() {
    let (mut server, _project, main_uri) = package_module_value_navigation_server();
    assert_package_module_value_definitions_are_distinct(&mut server, &main_uri);
    assert_package_module_value_references_are_distinct(&mut server, &main_uri);
    assert_package_module_value_is_not_renamable(&mut server, &main_uri);
}

#[test]
fn package_module_value_protocol_ignores_colliding_workspace_spans() {
    let (mut server, _project, values_uri, shared_uri, main_uri) =
        package_module_value_occurrence_provenance_server();

    for (uri, line, character) in [(&values_uri, 0, 9), (&shared_uri, 2, 29)] {
        assert_unresolved_module_value_protocols(&mut server, uri, line, character);
    }

    let definition = server.handle_message(&definition_request(&main_uri, 2, 29));
    assert!(definition[0].contains("veln-pkg:///example%2Fpkg/"));
    let references = server.handle_message(&references_request_with_declaration(
        &main_uri, 2, 29, true,
    ));
    assert_eq!(references[0].matches(r#""uri":"#).count(), 1);
    assert!(references[0].contains(&main_uri));
    assert!(!references[0].contains(&shared_uri));
}

fn package_module_value_occurrence_provenance_server(
) -> (Server, TempProject, String, String, String) {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("package-module-value-occurrence-provenance");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/dependency\"\n",
        ),
    );
    project.write("values.veln", "let x = answer\n");
    project.write(
        "shared.veln",
        "use foobar\n\nlet selected: Int = foobar::answer\n",
    );
    project.write(
        "main.veln",
        concat!(
            "use values from \"example/pkg\"\n\n",
            "let selected: Int = values::answer\n",
        ),
    );
    project.write(
        "vendor/dependency/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"values.veln\", \"shared.veln\"]\n",
        ),
    );
    project.write(
        "vendor/dependency/values.veln",
        "pub let answer: Int = 42\n",
    );
    project.write(
        "vendor/dependency/shared.veln",
        "use values\n\nlet selected: Int = values::answer\n",
    );

    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    let shared_uri = path_to_uri(&project.root.join("shared.veln"));
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    (server, project, values_uri, shared_uri, main_uri)
}

fn assert_unresolved_module_value_protocols(
    server: &mut Server,
    uri: &str,
    line: usize,
    character: usize,
) {
    assert_null_result(&server.handle_message(&definition_request(uri, line, character))[0]);
    assert_empty_result_array(
        &server.handle_message(&references_request_with_declaration(
            uri, line, character, true,
        ))[0],
    );
    assert_null_result(
        &server.handle_message(&prepare_rename_request(uri, line, character))[0],
    );
    assert_contains_json(
        &server.handle_message(&rename_request(uri, line, character, "renamed"))[0],
        r#""changes":{}"#,
    );
}

fn package_module_value_navigation_server() -> (Server, TempProject, String) {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("package-module-value-navigation");
    write_package_module_value_navigation_project(&project);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    (server, project, main_uri)
}

fn write_package_module_value_navigation_project(project: &TempProject) {
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
    write_package_module_value_dependency(project, "one", "one/pkg");
    write_package_module_value_dependency(project, "two", "two/pkg");
}

fn write_package_module_value_dependency(project: &TempProject, directory: &str, package: &str) {
    project.write(
        &format!("vendor/{directory}/veln.toml"),
        &format!("[package]\nname = \"{package}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"),
    );
    project.write(
        &format!("vendor/{directory}/shared.veln"),
        "pub let answer: Int = 42\n",
    );
}

fn assert_package_module_value_definitions_are_distinct(server: &mut Server, main_uri: &str) {
    let first = server.handle_message(&definition_request(main_uri, 3, 25));
    let second = server.handle_message(&definition_request(main_uri, 4, 27));
    assert!(first[0].contains("veln-pkg:///one%2Fpkg/"), "{}", first[0]);
    assert!(second[0].contains("veln-pkg:///two%2Fpkg/"), "{}", second[0]);
}

fn assert_package_module_value_references_are_distinct(server: &mut Server, main_uri: &str) {
    let first_references = server.handle_message(&references_request_with_declaration(
        main_uri, 3, 25, true,
    ));
    assert_eq!(
        first_references[0].matches(r#""uri":"#).count(),
        1,
        "{}",
        first_references[0]
    );
    assert!(first_references[0].contains(main_uri));
    assert!(first_references[0].contains(r#""line":3"#));
    assert!(!first_references[0].contains(r#""line":4"#));
    assert!(!first_references[0].contains("veln-pkg:///two%2Fpkg/"));
}

fn assert_package_module_value_is_not_renamable(server: &mut Server, main_uri: &str) {
    let prepared = server.handle_message(&prepare_rename_request(main_uri, 3, 25));
    assert_null_result(&prepared[0]);
    let renamed = server.handle_message(&rename_request(main_uri, 3, 25, "result"));
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

#[test]
fn module_value_protocol_rename_rejects_cross_module_complete_edit_conflicts() {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("module-value-cross-module-rename-conflicts");
    project.write("values.veln", "pub let answer: Int = 42\n");
    project.write(
        "capture.veln",
        concat!(
            "use values\n",
            "let target: Int = 1\n",
            "fn read() -> Int\n",
            "  answer\n",
            "end\n",
        ),
    );
    project.write("left.veln", "pub let target: Int = 1\n");
    project.write("right.veln", "pub let selected: Int = 42\n");
    project.write(
        "main.veln",
        concat!(
            "use left\n",
            "use right\n",
            "fn read() -> Int\n",
            "  target\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    let right_uri = path_to_uri(&project.root.join("right.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for uri in [&values_uri, &right_uri] {
        let response = server.handle_message(&rename_request(uri, 0, 10, "target"));
        assert_eq!(response.len(), 1);
        assert!(response[0].contains(r#""code":-32602"#), "{}", response[0]);
        assert!(
            response[0].contains(r#""code":"rename.conflict""#),
            "{}",
            response[0]
        );
        assert!(
            response[0].contains(r#""symbol_class":"value_binding""#),
            "{}",
            response[0]
        );
        assert!(
            response[0].contains(r#""requested_name":"target""#),
            "{}",
            response[0]
        );
        assert!(
            response[0].contains(r#""affected_scope":{"kind":"module","name":""#),
            "{}",
            response[0]
        );
        assert!(!response[0].contains(r#""changes""#), "{}", response[0]);
    }
}

#[test]
fn module_value_protocol_rename_rejects_bare_dependency_export_conflicts() {
    for (suffix, dependency_source) in [
        ("value", "pub let target: Int = 1\n"),
        ("function", "pub fn target() -> Int\n  1\nend\n"),
    ] {
        let mut server = server_with_empty_standard_library();
        let project = TempProject::new(&format!("module-value-dependency-conflict-{suffix}"));
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/dependency\"\n",
            ),
        );
        project.write("values.veln", "pub let answer: Int = 42\n");
        project.write(
            "main.veln",
            concat!(
                "use dependency from \"example/pkg\"\n",
                "use values\n\n",
                "fn read() -> Int\n",
                "  answer + target\n",
                "end\n",
            ),
        );
        project.write(
            "vendor/dependency/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"dependency.veln\"]\n",
            ),
        );
        project.write("vendor/dependency/dependency.veln", dependency_source);
        let root_uri = path_to_uri(&project.root);
        let values_uri = path_to_uri(&project.root.join("values.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let prepared = server.handle_message(&prepare_rename_request(&values_uri, 0, 10));
        assert!(!prepared[0].contains(r#""result":null"#), "{}", prepared[0]);
        let response = server.handle_message(&rename_request(&values_uri, 0, 10, "target"));
        assert!(response[0].contains(r#""code":"rename.conflict""#), "{}", response[0]);
        assert!(response[0].contains("veln-pkg:///example%2Fpkg/"), "{}", response[0]);
        assert!(!response[0].contains(r#""changes""#), "{}", response[0]);
    }
}

#[test]
fn module_value_protocol_rename_respects_owner_module_precedence() {
    let mut server = server_with_empty_standard_library();
    let project = TempProject::new("module-value-owner-precedence");
    project.write("imported.veln", "pub let target: Int = 1\n");
    project.write(
        "values.veln",
        concat!(
            "use imported\n\n",
            "pub let answer: Int = 42\n",
            "fn read() -> Int\n",
            "  answer\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let values_uri = path_to_uri(&project.root.join("values.veln"));
    server.handle_message(&initialize_request(&root_uri));
    let response = server.handle_message(&rename_request(&values_uri, 2, 10, "target"));
    assert_eq!(response[0].matches(r#""newText":"target""#).count(), 2, "{}", response[0]);
    assert!(!response[0].contains(r#""code":"rename.conflict""#));

    let mut renamed_server = server_with_empty_standard_library();
    let renamed_project = TempProject::new("module-value-owner-precedence-after-rename");
    renamed_project.write("imported.veln", "pub let target: Int = 1\n");
    renamed_project.write(
        "values.veln",
        concat!(
            "use imported\n\n",
            "pub let target: Int = 42\n",
            "fn read() -> Int\n",
            "  target\n",
            "end\n",
        ),
    );
    let renamed_root = path_to_uri(&renamed_project.root);
    let renamed_values = path_to_uri(&renamed_project.root.join("values.veln"));
    renamed_server.handle_message(&initialize_request(&renamed_root));
    let definition =
        renamed_server.handle_message(&definition_request(&renamed_values, 4, 3));
    assert!(definition[0].contains(&renamed_values), "{}", definition[0]);
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":2,"character":8},"end":{"line":2,"character":14}}"#
        ),
        "{}",
        definition[0]
    );
}
