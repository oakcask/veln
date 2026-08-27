#[test]
fn unselected_workspace_invalid_casing_does_not_publish_or_index() {
    let mut server = Server::default();
    let selected = TempProject::new("selected-valid-casing-root");
    let unselected = TempProject::new("unselected-invalid-casing-root");
    selected.write("main.veln", "fn good() -> Int\n  good()\nend\n");
    unselected.write("main.veln", "fn Bad() -> Int\n  Bad()\nend\n");
    let selected_root_uri = path_to_uri(&selected.root);
    let selected_main_uri = path_to_uri(&selected.root.join("main.veln"));
    let unselected_main_uri = path_to_uri(&unselected.root.join("main.veln"));

    let responses = server.handle_message(&initialize_request(&selected_root_uri));

    let publish = publish_for_uri(&responses, &selected_main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(
        responses
            .iter()
            .all(|response| !response.contains(&unselected_main_uri)),
        "{responses:#?}"
    );
    let definition = server.handle_message(&definition_request(&selected_main_uri, 1, 2));
    assert!(
        definition[0].contains(r#""result":{"uri":"file://"#),
        "{}",
        definition[0]
    );
    assert!(
        !definition[0].contains(&escape_json(&unselected_main_uri)),
        "{}",
        definition[0]
    );
}

#[test]
fn server_does_not_overlay_open_documents_owned_by_nested_manifest() {
    let mut server = Server::default();
    let project = TempProject::new("nested-open-document-overlay-boundary");
    project.write(
        "veln.toml",
        "[package]\nname = \"outer\"\n\n[lib]\nexports = [\"app.veln\", \"nested/hidden.veln\"]\n",
    );
    project.write("app.veln", "pub fn app() -> Int\n  1\nend\n");
    project.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    project.write("nested/hidden.veln", "pub fn hidden() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let manifest_uri = path_to_uri(&project.root.join("veln.toml"));
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{nested_uri}","text":"pub fn hidden() -> Int\n  2\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{app_uri}","version":2}},"contentChanges":[{{"text":"pub fn app() -> Int\n  1\nend\n"}}]}}}}"#
        ));

    let publish = publish_for_uri(&responses, &manifest_uri);
    assert!(
        publish.contains(r#""code":"manifest.unselected_export""#),
        "{publish}"
    );
}

#[test]
fn server_clears_stale_workspace_diagnostics_after_change() {
    let mut server = Server::default();
    let project = TempProject::new("workspace-diagnostics-change-clear");
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn main() -> Int\n  missing\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{main_uri}","version":2}},"contentChanges":[{{"text":"fn main() -> Int\n  1\nend\n"}}]}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
}

#[test]
fn server_analysis_respects_manifest_boundaries_and_owned_target_sources() {
    let mut server = Server::default();
    let project = TempProject::new("manifest-boundary-analysis");
    project.write("veln.toml", "[package]\nname = \"outer\"\n");
    project.write("app.veln", "pub fn app() -> Int\n\t1\nend\n");
    project.write("target/owned.veln", "pub fn owned() -> Int\n\t2\nend\n");
    project.write("nested/veln.toml", "malformed nested manifest");
    project.write("nested/hidden.veln", "this source must not be parsed");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let target_uri = path_to_uri(&project.root.join("target/owned.veln"));
    let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));

    assert!(publish_for_uri(&responses, &app_uri).contains(r#""diagnostics":[]"#));
    assert!(publish_for_uri(&responses, &target_uri).contains(r#""diagnostics":[]"#));
    assert!(
        responses
            .iter()
            .all(|response| !response.contains(&nested_uri)),
        "nested package source should not receive outer-project diagnostics"
    );
}

#[test]
fn server_clears_workspace_diagnostics_when_file_leaves_discovery() {
    let mut server = Server::default();
    let project = TempProject::new("workspace-diagnostics-left-discovery");
    project.write("main.veln", "fn main() -> Int\n  missing\nend\n");
    let root_uri = path_to_uri(&project.root);
    let main_path = project.root.join("main.veln");
    let main_uri = path_to_uri(&main_path);
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));
    fs::remove_file(main_path).expect("fixture source should be removable");

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{main_uri}"}}}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
}

#[test]
fn server_reports_cross_file_workspace_diagnostics() {
    let mut server = Server::default();
    let project = TempProject::new("cross-file-workspace-diagnostics");
    project.write(
        "app.veln",
        "use math\n\nfn main() -> Int\n  double(\"bad\")\nend\n",
    );
    project.write(
        "math.veln",
        "pub fn double(value: Int) -> Int\n  value * 2\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""code":"type.mismatch""#), "{publish}");
}

#[test]
fn server_publishes_same_leaf_files_from_multiple_roots_separately() {
    let mut server = Server::default();
    let alpha = TempProject::new("same-leaf-alpha-root");
    let beta = TempProject::new("same-leaf-beta-root");
    alpha.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
    beta.write("main.veln", "pub fn main() -> Int\n  \"bad\"\nend\n");
    let alpha_root_uri = path_to_uri(&alpha.root);
    let beta_root_uri = path_to_uri(&beta.root);
    let alpha_main_uri = path_to_uri(&alpha.root.join("main.veln"));
    let beta_main_uri = path_to_uri(&beta.root.join("main.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alpha_root_uri}","name":"alpha"}},{{"uri":"{beta_root_uri}","name":"beta"}}]}}}}"#
        ));

    let alpha_publish = publish_for_uri(&responses, &alpha_main_uri);
    let beta_publish = publish_for_uri(&responses, &beta_main_uri);
    assert!(
        alpha_publish.contains(r#""diagnostics":[]"#),
        "{alpha_publish}"
    );
    assert!(
        beta_publish.contains(r#""code":"type.mismatch""#),
        "{beta_publish}"
    );
}

#[test]
fn server_keeps_same_leaf_workspace_files_in_distinct_modules() {
    let mut server = Server::default();
    let project = TempProject::new("same-leaf-workspace-diagnostics");
    project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
    project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(
        !publish.contains("module.duplicate_source_path"),
        "{publish}"
    );
}

#[test]
fn server_overlays_same_leaf_workspace_files_by_relative_path() {
    let mut server = Server::default();
    let project = TempProject::new("same-leaf-workspace-overlay");
    project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
    project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let beta_uri = path_to_uri(&project.root.join("beta/item.veln"));
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{beta_uri}","text":"pub fn value() -> String\n  \"two\"\nend\n"}}}}}}"#
        ));

    let app_publish = publish_for_uri(&responses, &app_uri);
    let beta_publish = publish_for_uri(&responses, &beta_uri);
    assert!(
        app_publish.contains(r#""code":"type.mismatch""#),
        "{app_publish}"
    );
    assert!(
        beta_publish.contains(r#""diagnostics":[]"#),
        "{beta_publish}"
    );
    assert!(
        !app_publish.contains("module.duplicate_source_path"),
        "{app_publish}"
    );
}

#[test]
fn companion_private_function_definition_returns_target_declaration() {
    let mut server = Server::default();
    let project = companion_private_function_project("definition");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&definition_request(&companion_uri, 7, 10));

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(&format!(r#""uri":"{}""#, escape_json(&math_uri))));
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_prepare_rename_returns_reference_leaf() {
    let mut server = Server::default();
    let project = companion_private_function_project("prepare-rename");
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&prepare_rename_request(&companion_uri, 7, 10));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(
            r#""result":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_edits_target_and_matching_companion_references() {
    let mut server = Server::default();
    let project = companion_private_function_project("rename");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(&format!(r#""{}":["#, escape_json(&math_uri))),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(&format!(r#""{}":["#, escape_json(&companion_uri))),
        "{}",
        responses[0]
    );
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn rename_accepts_same_class_replacements_for_cased_symbols() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-success");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "effect Choose\n",
            "  pick(value: Bool) -> Bool\n",
            "end\n\n",
            "handler choose() handles Choose\n",
            "  pick(value) => value\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let type_rename = server.handle_message(&rename_request(&main_uri, 0, 5, "Entry"));
    let constructor_rename = server.handle_message(&rename_request(&main_uri, 1, 2, "Created"));
    let function_rename = server.handle_message(&rename_request(&main_uri, 12, 3, "adapt"));
    let value_rename = server.handle_message(&rename_request(&main_uri, 9, 7, "input"));

    assert_eq!(type_rename[0].matches(r#""newText":"Entry""#).count(), 3);
    assert_eq!(
        constructor_rename[0]
            .matches(r#""newText":"Created""#)
            .count(),
        2
    );
    assert_eq!(
        function_rename[0].matches(r#""newText":"adapt""#).count(),
        1
    );
    assert_eq!(value_rename[0].matches(r#""newText":"input""#).count(), 2);
}

#[test]
fn constructor_rename_keeps_cross_file_reference_at_declaration_offset() {
    let mut server = Server::default();
    let project = TempProject::new("rename-constructor-cross-file-offset");
    project.write("f.veln", "pub type Flag\n  pub Done\nend\n");
    project.write("main.veln", "use f\n\nfn a()-> X\n  Done\nend\n");
    let root_uri = path_to_uri(&project.root);
    let flag_uri = path_to_uri(&project.root.join("f.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let rename = server.handle_message(&rename_request(&flag_uri, 1, 6, "Ready"));

    assert_eq!(rename.len(), 1);
    assert_eq!(rename[0].matches(r#""newText":"Ready""#).count(), 2);
    assert!(rename[0].contains("f.veln"), "{}", rename[0]);
    assert!(rename[0].contains("main.veln"), "{}", rename[0]);
    assert!(
        rename[0].contains(r#""line":3,"character":2"#),
        "{}",
        rename[0]
    );
}

#[test]
fn constructor_rename_covers_bare_nullary_expression_and_pattern() {
    let mut server = Server::default();
    let project = TempProject::new("rename-nullary-constructor-forms");
    project.write(
        "main.veln",
        concat!(
            "type Status\n",
            "  Ready\n",
            "  Waiting\n",
            "end\n\n",
            "fn ready() -> Status\n",
            "  Ready\n",
            "end\n\n",
            "fn observe(status: Status) -> Bool\n",
            "  match status\n",
            "    Ready => true\n",
            "    Waiting => false\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let rename = server.handle_message(&rename_request(&main_uri, 1, 2, "Created"));

    assert_eq!(rename.len(), 1);
    assert_eq!(rename[0].matches(r#""newText":"Created""#).count(), 3);
    assert!(
        rename[0].contains(r#""line":6,"character":2"#),
        "{}",
        rename[0]
    );
    assert!(
        rename[0].contains(r#""line":11,"character":4"#),
        "{}",
        rename[0]
    );
    assert!(
        !rename[0].contains(r#""line":12,"character":4"#),
        "{}",
        rename[0]
    );
}

#[test]
fn rename_rejects_class_changing_replacements_for_cased_symbols() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-invalid-case");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "effect Choose\n",
            "  pick(value: Bool) -> Bool\n",
            "end\n\n",
            "handler choose() handles Choose\n",
            "  pick(value) => value\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let cases = [
        (0, 5, "entry", "type", "ascii_uppercase"),
        (1, 2, "created", "constructor", "ascii_uppercase"),
        (12, 3, "Adapt", "function", "ascii_lowercase"),
        (9, 7, "Input", "value_binding", "ascii_lowercase"),
    ];
    for (line, character, new_name, symbol_class, required_initial) in cases {
        let responses =
            server.handle_message(&rename_request(&main_uri, line, character, new_name));

        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(r#""code":-32602"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(r#""code":"rename.invalid_case""#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""symbol_class":"{symbol_class}""#)),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""requested_name":"{new_name}""#)),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""required_initial":"{required_initial}""#)),
            "{}",
            responses[0]
        );
        assert!(!responses[0].contains(r#""changes""#), "{}", responses[0]);
    }
}

#[test]
fn rename_excludes_same_named_non_type_namespace_tokens() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-namespace-boundary");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "schema Item\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "effect Item\n",
            "  Item() -> Int\n",
            "end\n\n",
            "fn main(input: Item) -> Item\n",
            "  input\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(4, 7), (9, 7), (10, 2)] {
        let prepare_rename =
            server.handle_message(&prepare_rename_request(&main_uri, line, character));
        assert_eq!(prepare_rename.len(), 1);
        assert!(
            prepare_rename[0].contains(r#""result":null"#),
            "{}",
            prepare_rename[0]
        );

        let rename = server.handle_message(&rename_request(&main_uri, line, character, "Entry"));
        assert_eq!(rename.len(), 1);
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
        assert!(
            !rename[0].contains(r#""rename.invalid_case""#),
            "{}",
            rename[0]
        );
    }

    let type_rename = server.handle_message(&rename_request(&main_uri, 13, 15, "Entry"));
    assert_eq!(type_rename[0].matches(r#""newText":"Entry""#).count(), 3);
    assert!(
        !type_rename[0].contains(r#""line":4,"character":7"#),
        "{}",
        type_rename[0]
    );
    assert!(
        !type_rename[0].contains(r#""line":9,"character":7"#),
        "{}",
        type_rename[0]
    );
    assert!(
        !type_rename[0].contains(r#""line":10,"character":2"#),
        "{}",
        type_rename[0]
    );
}

#[test]
fn type_rename_requires_unique_visible_type_identity() {
    let mut server = Server::default();
    let project = TempProject::new("rename-type-semantic-identity");
    project.write("left.veln", "pub type Item\n  Left\nend\n");
    project.write("right.veln", "pub type Item\n  Right\nend\n");
    project.write(
        "main.veln",
        concat!(
            "use left\n",
            "use right\n",
            "\n",
            "fn bare(value: Item) -> Item\n",
            "  value\n",
            "end\n",
            "\n",
            "fn left_value(value: left::Item) -> left::Item\n",
            "  value\n",
            "end\n",
            "\n",
            "fn right_value(value: right::Item) -> right::Item\n",
            "  value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let ambiguous_prepare = server.handle_message(&prepare_rename_request(&main_uri, 3, 15));
    let ambiguous_rename = server.handle_message(&rename_request(&main_uri, 3, 15, "Entry"));
    let qualified_rename = server.handle_message(&rename_request(&main_uri, 11, 29, "RightEntry"));

    assert_eq!(ambiguous_prepare.len(), 1);
    assert!(
        ambiguous_prepare[0].contains(r#""result":null"#),
        "{}",
        ambiguous_prepare[0]
    );
    assert_eq!(ambiguous_rename.len(), 1);
    assert!(
        ambiguous_rename[0].contains(r#""changes":{}"#),
        "{}",
        ambiguous_rename[0]
    );
    assert_eq!(qualified_rename.len(), 1);
    assert_eq!(
        qualified_rename[0]
            .matches(r#""newText":"RightEntry""#)
            .count(),
        3
    );
    assert!(
        qualified_rename[0].contains("right.veln"),
        "{}",
        qualified_rename[0]
    );
    assert!(
        !qualified_rename[0].contains("left.veln"),
        "{}",
        qualified_rename[0]
    );
    assert!(
        !qualified_rename[0].contains(r#""line":3,"character":15"#),
        "{}",
        qualified_rename[0]
    );
}

