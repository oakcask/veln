#[test]
fn handler_context_parameter_does_not_bind_same_named_operation_heading() {
    let mut server = Server::default();
    let project = TempProject::new("handler-context-operation-heading-isolation");
    project.write(
        "main.veln",
        concat!(
            "fn callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "effect Adjust\n",
            "  callback(value: Int) -> Int\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
            "  callback(value) => callback(value)\n",
            "  amount(value) => callback(value)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 10, 4));
    let references = server.handle_message(&references_request(&main_uri, 10, 4));
    let rename = server.handle_message(&rename_request(&main_uri, 10, 4, "project"));
    let body_definition = server.handle_message(&definition_request(&main_uri, 10, 22));

    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(r#""result":null"#),
        "{}",
        definition[0]
    );
    assert_eq!(references.len(), 1);
    assert!(
        references[0].contains(r#""result":[]"#),
        "{}",
        references[0]
    );
    assert_eq!(rename.len(), 1);
    assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
    assert_eq!(body_definition.len(), 1);
    assert!(
        body_definition[0].contains(
            r#""range":{"start":{"line":9,"character":15},"end":{"line":9,"character":23}}"#
        ),
        "{}",
        body_definition[0]
    );
}

#[test]
fn invalid_handler_bindings_do_not_enter_lsp_navigation() {
    let mut server = Server::default();
    let project = TempProject::new("invalid-handler-binding-navigation");
    project.write(
        "main.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "  echo(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(Callback: fn(Int) -> Int) handles Adjust\n",
            "  amount(value) => Callback(value)\n",
            "  echo(Result) => Callback(Result)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));

    let responses = server.handle_message(&initialize_request(&root_uri));
    let publish = publish_for_uri(&responses, &main_uri);
    assert!(
        publish.contains(r#""code":"name.invalid_case""#),
        "{publish}"
    );

    for (line, character) in [(5, 15), (6, 19), (7, 7), (7, 27)] {
        let definition = server.handle_message(&definition_request(&main_uri, line, character));
        assert_eq!(definition.len(), 1);
        assert!(
            definition[0].contains(r#""result":null"#),
            "{}",
            definition[0]
        );

        let references = server.handle_message(&references_request(&main_uri, line, character));
        assert_eq!(references.len(), 1);
        assert!(
            references[0].contains(r#""result":[]"#),
            "{}",
            references[0]
        );

        let prepare_rename =
            server.handle_message(&prepare_rename_request(&main_uri, line, character));
        assert_eq!(prepare_rename.len(), 1);
        assert!(
            prepare_rename[0].contains(r#""result":null"#),
            "{}",
            prepare_rename[0]
        );

        let rename = server.handle_message(&rename_request(&main_uri, line, character, "fixed"));
        assert_eq!(rename.len(), 1);
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
    }
}

#[test]
fn companion_private_function_rename_includes_target_function_alias_target() {
    let mut server = Server::default();
    let project = TempProject::new("rename-function-alias-target");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn advance = increment\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "bump"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"bump""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":4,"character":17},"end":{"line":4,"character":26}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_lsp_rejects_companion_function_values_and_aliases() {
    let mut server = Server::default();
    let project = TempProject::new("reject-companion-function-values");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "pub fn expose = math::increment\n",
            "\n",
            "test companion() -> ()\n",
            "  let mapper: fn(Int) -> Int = math::increment\n",
            "  ()\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let alias_definition = server.handle_message(&definition_request(&companion_uri, 2, 23));
    let value_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 37));
    let value_rename = server.handle_message(&rename_request(&companion_uri, 5, 37, "bump"));

    assert!(
        alias_definition[0].contains(r#""result":null"#),
        "{}",
        alias_definition[0]
    );
    assert!(
        value_prepare[0].contains(r#""result":null"#),
        "{}",
        value_prepare[0]
    );
    assert!(
        value_rename[0].contains(r#""changes":{}"#),
        "{}",
        value_rename[0]
    );
}

#[test]
fn companion_private_function_requests_ignore_comment_and_string_origins() {
    let mut server = Server::default();
    let project = TempProject::new("request-origin");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  \"math::increment(2)\"\n",
            "  # math::increment(3)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let string_definition = server.handle_message(&definition_request(&companion_uri, 4, 10));
    let comment_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 11));
    let comment_rename = server.handle_message(&rename_request(&companion_uri, 5, 11, "advance"));

    assert!(
        string_definition[0].contains(r#""result":null"#),
        "{}",
        string_definition[0]
    );
    assert!(
        comment_prepare[0].contains(r#""result":null"#),
        "{}",
        comment_prepare[0]
    );
    assert!(
        comment_rename[0].contains(r#""changes":{}"#),
        "{}",
        comment_rename[0]
    );
}

#[test]
fn companion_private_function_lsp_rejects_other_private_boundaries() {
    let mut server = Server::default();
    let project = TempProject::new("rejected-private-boundaries");
    project.write(
        "math.veln",
        "use support\n\nfn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "support.veln",
        "fn private_helper(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "other.test.veln",
        "use math\n\ntest wrong() -> Int\n  math::increment(1)\nend\n",
    );
    project.write(
        "math_test.veln",
        "use math\n\ntest integration() -> Int\n  math::increment(1)\nend\n",
    );
    project.write(
        "math.test.veln",
        "use support\n\ntest transitive() -> Int\n  support::private_helper(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    server.handle_message(&initialize_request(&root_uri));

    let wrong_uri = path_to_uri(&project.root.join("other.test.veln"));
    let integration_uri = path_to_uri(&project.root.join("math_test.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));

    let wrong = server.handle_message(&definition_request(&wrong_uri, 3, 10));
    let integration = server.handle_message(&definition_request(&integration_uri, 3, 10));
    let transitive = server.handle_message(&definition_request(&companion_uri, 3, 13));

    assert!(wrong[0].contains(r#""result":null"#), "{}", wrong[0]);
    assert!(
        integration[0].contains(r#""result":null"#),
        "{}",
        integration[0]
    );
    assert!(
        transitive[0].contains(r#""result":null"#),
        "{}",
        transitive[0]
    );
}

#[test]
fn companion_private_function_definition_uses_open_document_overlay() {
    let mut server = Server::default();
    let project = companion_private_function_project("definition-overlay");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  value + 1\nend\n"}}}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&definition_request(&companion_uri, 3, 10));

    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn dependency_definition_round_trips_through_retained_virtual_document() {
    let mut server = Server::default();
    let retained_text = concat!(
        "pub fn exposed(value: Int) -> Int\r\n",
        "  value + 1\r\n",
        "end\r\n\r\n",
        "fn secret(value: Int) -> Int\r\n",
        "  value\r\n",
        "end\r\n",
    );
    let project = path_dependency_project("dependency-virtual-document", "vendor/lib", retained_text);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    project.write(
        "vendor/lib/math.veln",
        "pub fn changed() -> Int\n  0\nend\n",
    );
    project.write(
        "main.veln",
        "use math from \"example/pkg\"\n\npub fn main() -> Int\n  math::changed()\nend\n",
    );
    let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
    assert_eq!(definition.len(), 1);
    let virtual_uri = package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
    assert_not_contains_json(&virtual_uri, "vendor");
    assert_not_contains_json(&virtual_uri, "veln-lsp-");
    assert_contains_json(
        &definition[0],
        r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#,
    );

    assert_dependency_virtual_document_boundaries(&mut server, &main_uri, &virtual_uri, retained_text);
}

fn path_dependency_project(name: &str, source_root: &str, retained_text: &str) -> TempProject {
    let project = TempProject::new(name);
    project.write(
        "veln.toml",
        &format!("[package]\nname = \"app\"\n\n[dependencies.\"example/pkg\"]\npath = \"{source_root}\"\n"),
    );
    project.write("main.veln", dependency_main_text());
    project.write(
        &format!("{source_root}/veln.toml"),
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"./math.veln\"]\n",
        ),
    );
    project.write(&format!("{source_root}/math.veln"), retained_text);
    project
}

fn dependency_main_text() -> &'static str {
    concat!(
        "use math from \"example/pkg\"\n\n",
        "pub fn main() -> Int\n",
        "  math::exposed(1)\n",
        "  math::secret(1)\n",
        "end\n",
    )
}

fn assert_dependency_virtual_document_boundaries(
    server: &mut Server,
    main_uri: &str,
    virtual_uri: &str,
    retained_text: &str,
) {
    assert_virtual_document_text(server, "3", virtual_uri, retained_text);
    let prepare_rename = server.handle_message(&prepare_rename_request(main_uri, 3, 10));
    assert_null_result(&prepare_rename[0]);
    let rename = server.handle_message(&rename_request(main_uri, 3, 10, "renamed"));
    assert_contains_json(&rename[0], r#""changes":{}"#);
    let private_definition = server.handle_message(&definition_request(main_uri, 4, 10));
    assert_null_result(&private_definition[0]);
    assert_dependency_reference_boundary(server, main_uri);
    assert_rejected_virtual_document_uris(server, virtual_uri);
}

fn assert_dependency_reference_boundary(server: &mut Server, main_uri: &str) {
    for include_declaration in [false, true] {
        let references =
            server.handle_message(&references_request_with_declaration(main_uri, 3, 10, include_declaration));
        assert_empty_result_array(&references[0]);
    }
}

fn assert_rejected_virtual_document_uris(server: &mut Server, virtual_uri: &str) {
    for rejected_uri in [
        format!("{virtual_uri}/missing"),
        virtual_uri.replacen("%2F", "%2f", 1),
    ] {
        let rejected = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
        ));
        assert_invalid_params(&rejected[0]);
    }
}

#[test]
fn path_vendor_and_mirror_dependencies_share_retained_virtual_uris() {
    let mut retained_virtual_uri = None;

    for (source_field, source_root) in [
        ("path", "path/lib"),
        ("vendor", "vendor/lib"),
        ("mirror", "mirror/example/pkg"),
    ] {
        let retained_text = "pub fn exposed(value: Int) -> Int\r\n  value + 1\r\nend\r\n";
        let (project, manifest_text) =
            dependency_source_project(source_field, source_root, retained_text);

        if let Some(virtual_uri) = retained_virtual_uri.as_deref() {
            assert_retained_uri_resolves_for_source(
                &project,
                &manifest_text,
                source_field,
                virtual_uri,
                retained_text,
            );
            continue;
        }

        let virtual_uri =
            assert_first_dependency_source_round_trip(&project, source_root, retained_text);
        retained_virtual_uri = Some(virtual_uri);
    }

    assert!(retained_virtual_uri.is_some());
}

fn dependency_source_project(
    source_field: &str,
    source_root: &str,
    retained_text: &str,
) -> (TempProject, String) {
    let project = TempProject::new(&format!("dependency-virtual-document-{source_field}"));
    let manifest_text = format!(
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\n",
            "{} = \"{}\"\n",
        ),
        source_field, source_root
    );
    project.write("veln.toml", &manifest_text);
    project.write("main.veln", dependency_source_main_text());
    project.write(
        &format!("{source_root}/veln.toml"),
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(&format!("{source_root}/math.veln"), retained_text);
    project.write(
        &format!("{source_root}/hidden.veln"),
        "pub fn published(value: Int) -> Int\n  value\nend\n",
    );
    (project, manifest_text)
}

fn dependency_source_main_text() -> &'static str {
    concat!(
        "use math from \"example/pkg\"\n\n",
        "use hidden from \"example/pkg\"\n\n",
        "pub fn main() -> Int\n",
        "  math::exposed(1)\n",
        "  math::secret(1)\n",
        "  hidden::published(1)\n",
        "end\n",
    )
}

fn assert_retained_uri_resolves_for_source(
    project: &TempProject,
    manifest_text: &str,
    source_field: &str,
    virtual_uri: &str,
    retained_text: &str,
) {
    let manifest = parse_manifest_text("veln.toml", manifest_text);
    let dependencies = retained_direct_dependencies(&project.root, &manifest);
    assert_eq!(dependencies.len(), 1, "{source_field}");
    let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(Vec::new(), dependencies);
    assert_eq!(
        snapshot.resolve_virtual_source(virtual_uri),
        Some(retained_text.as_bytes()),
        "{source_field}"
    );
}

fn assert_first_dependency_source_round_trip(
    project: &TempProject,
    source_root: &str,
    retained_text: &str,
) -> String {
    let mut server = Server::default();
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    let definition = server.handle_message(&definition_request(&main_uri, 5, 10));
    let virtual_uri = package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
    assert_not_contains_json(&virtual_uri, source_root);
    assert_virtual_document_text(&mut server, "3", &virtual_uri, retained_text);
    let private_definition = server.handle_message(&definition_request(&main_uri, 6, 10));
    assert_null_result(&private_definition[0]);
    let unexported_definition = server.handle_message(&definition_request(&main_uri, 7, 12));
    assert_null_result(&unexported_definition[0]);
    virtual_uri
}

#[test]
fn git_dependency_subdir_definition_uses_retained_virtual_document() {
    let mut server = Server::default();
    let remote_url = "https://example.invalid/mono.git";
    let retained_text = concat!(
        "pub fn exposed(value: Int) -> Int\r\n",
        "  value + 1\r\n",
        "end\r\n\r\n",
        "fn secret(value: Int) -> Int\r\n",
        "  value\r\n",
        "end\r\n",
    );
    let (project, package_root) = git_dependency_project(remote_url, retained_text);

    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    project.write(
        &package_root.join("math.veln").display().to_string(),
        "pub fn changed() -> Int\n  0\nend\n",
    );

    let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
    let virtual_uri = package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
    assert_not_contains_json(&virtual_uri, ".veln/package/git");
    assert_not_contains_json(&virtual_uri, "packages/lib");
    assert_git_dependency_virtual_document_boundary(&mut server, &main_uri, &virtual_uri, retained_text);
}

fn git_dependency_project(
    remote_url: &str,
    retained_text: &str,
) -> (TempProject, std::path::PathBuf) {
    let project = TempProject::new("git-dependency-virtual-document");
    let repository_root = materialized_git_repository_root(&project.root, remote_url);
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/lib\"\n",
        ),
    );
    project.write("main.veln", dependency_main_text());
    let package_root = repository_root.join("packages/lib");
    project.write(
        &package_root.join("veln.toml").display().to_string(),
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        &package_root.join("math.veln").display().to_string(),
        retained_text,
    );
    (project, package_root)
}

fn assert_git_dependency_virtual_document_boundary(
    server: &mut Server,
    main_uri: &str,
    virtual_uri: &str,
    retained_text: &str,
) {
    assert_virtual_document_text(server, "3", virtual_uri, retained_text);
    let private_definition = server.handle_message(&definition_request(main_uri, 4, 10));
    assert_null_result(&private_definition[0]);
}

#[test]
fn standard_library_definition_round_trips_through_embedded_virtual_document() {
    let mut server = Server::default();
    let project = standard_library_virtual_document_project();
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let prelude_uri = assert_standard_prelude_navigation(&mut server, &main_uri);
    assert_standard_import_navigation(&mut server, &main_uri);

    assert_virtual_document_text(
        &mut server,
        "3",
        &prelude_uri,
        standard_source_text("prelude.veln"),
    );

    let private_definition = server.handle_message(&definition_request(&main_uri, 24, 12));
    assert!(private_definition[0].contains(r#""result":null"#));
    let prepare_rename = server.handle_message(&prepare_rename_request(&main_uri, 3, 4));
    let rename = server.handle_message(&rename_request(&main_uri, 3, 4, "renamed"));
    assert!(prepare_rename[0].contains(r#""result":null"#));
    assert!(rename[0].contains(r#""changes":{}"#));

    for rejected_uri in [
        format!("{prelude_uri}/missing"),
        prelude_uri.replacen("veln-pkg", "VELN-pkg", 1),
    ] {
        let rejected = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
            ));
        assert_invalid_params(&rejected[0]);
    }
}
