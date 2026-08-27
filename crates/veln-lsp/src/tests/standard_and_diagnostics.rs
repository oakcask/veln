#[test]
fn ambiguous_bare_prelude_fallback_returns_no_definition() {
    let mut server = Server::default();
    let project = TempProject::new("ambiguous-bare-prelude-definition");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "math.veln",
        concat!(
            "use math from \"example/pkg\"\n\n",
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(r#""result":null"#),
        "{}",
        definition[0]
    );
}

#[test]
fn private_workspace_import_does_not_hide_bare_prelude_definition() {
    let mut server = Server::default();
    let project = TempProject::new("private-import-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use math\n\n",
            "pub fn main() -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.veln",
        "fn byte(value: Int) -> Result<Byte, String>\n  Ok(Byte(value))\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    let prelude_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        definition[0]
    );
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        definition[0]
    );
}

#[test]
fn invalid_imported_constructor_casing_falls_back_to_bare_prelude_function() {
    let mut server = Server::default();
    let project = TempProject::new("imported-constructor-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use model\n\n",
            "pub fn main() -> Token\n",
            "  byte(1)\n",
            "end\n",
        ),
    );
    project.write(
        "model.veln",
        concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    let prelude_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        definition[0]
    );
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        definition[0]
    );
}

#[test]
fn invalid_reexported_constructor_casing_does_not_hide_bare_prelude_function() {
    let mut server = Server::default();
    let project = TempProject::new("reexported-constructor-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use facade\n\n",
            "pub fn bare() -> Token\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn qualified() -> Token\n",
            "  facade::byte(2)\n",
            "end\n",
        ),
    );
    project.write(
        "facade.veln",
        concat!("use model\n\n", "pub type Token = model::Token\n"),
    );
    project.write(
        "model.veln",
        concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let bare = server.handle_message(&definition_request(&main_uri, 3, 4));
    assert_eq!(bare.len(), 1);
    let prelude_uri = extract_string_field(&bare[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        bare[0]
    );
    assert!(
        bare[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        bare[0]
    );

    let qualified = server.handle_message(&definition_request(&main_uri, 7, 10));
    assert_eq!(qualified.len(), 1);
    assert!(
        qualified[0].contains(r#""result":null"#),
        "{}",
        qualified[0]
    );
}

#[test]
fn retained_direct_dependencies_use_the_supplied_workspace_manifest() {
    let project = TempProject::new("retained-dependency-supplied-manifest");
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    let manifest = parse_manifest_text(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );

    let dependencies = retained_direct_dependencies(&project.root, &manifest);

    assert_eq!(dependencies.len(), 1);
    let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
        vec![SourceFile::new(
            "main.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "pub fn main() -> Int\n",
                "  math::exposed(1)\n",
                "end\n",
            ),
        )],
        dependencies,
    );
    assert!(
        navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 10,
            }
        )
        .is_some()
    );
}

#[test]
fn dependency_definition_requires_external_import_path() {
    let mut server = Server::default();
    let project = TempProject::new("dependency-import-boundary");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use math from \"other/pkg\"\n",
            "use other from \"example/pkg\"\n\n",
            "pub fn missing_import() -> Int\n",
            "  exposed(1)\n",
            "end\n\n",
            "pub fn wrong_package() -> Int\n",
            "  math::exposed(1)\n",
            "end\n\n",
            "pub fn wrong_module() -> Int\n",
            "  other::exposed(1)\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(4, 4), (8, 10), (12, 11)] {
        let response = server.handle_message(&definition_request(&main_uri, line, character));
        assert!(response[0].contains(r#""result":null"#), "{}", response[0]);
    }
}

#[test]
fn workspace_references_and_rename_ignore_dependency_sources() {
    let mut server = Server::default();
    let project = TempProject::new("dependency-reference-isolation");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  exposed(value - 1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let references = server.handle_message(&references_request(&math_uri, 0, 7));
    assert!(
        references[0].contains(
            r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        references[0]
    );
    assert!(
        !references[0].contains(r#""line":1,"character":2"#),
        "{}",
        references[0]
    );

    let rename = server.handle_message(&rename_request(&math_uri, 0, 7, "renamed"));
    assert!(
        rename[0].contains(
            r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        rename[0]
    );
    assert!(
        !rename[0].contains(r#""line":1,"character":2"#),
        "{}",
        rename[0]
    );
    assert!(!rename[0].contains("vendor"), "{}", rename[0]);
}

#[test]
fn companion_private_function_rename_uses_open_document_overlay() {
    let mut server = Server::default();
    let project = companion_private_function_project("rename-overlay");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  bump(value)\nend\n"}}}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\n  math::bump\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":6}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":3,"character":8},"end":{"line":3,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":4,"character":8"#),
        "{}",
        responses[0]
    );
}

#[test]
fn server_returns_full_semantic_tokens_for_open_document() {
    let mut server = Server::default();
    server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn main() -> Int\n  main()\nend\n"}}}"#,
        );

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""id":2"#));
    assert!(responses[0].contains(r#""data":["#));
}

#[test]
fn server_publishes_parse_diagnostics_for_open_document() {
    let mut server = Server::default();

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""method":"textDocument/publishDiagnostics""#));
    assert!(responses[0].contains(r#""source":"veln""#));
    assert!(responses[0].contains(r#""severity":1"#));
    assert!(responses[0].contains(r#""code":"parse."#));
}

#[test]
fn lsp_diagnostic_wire_fields_are_stable() {
    let diagnostic = Diagnostic::new(
        "parse.expected_item",
        Severity::Error,
        DiagnosticKind::Parse,
        "expected a function or test declaration",
        Some(SourceSpan {
            file: veln_source::SourcePath::new("main.veln"),
            start: veln_source::LineCol {
                line: 2,
                column: 3,
                offset: 4,
            },
            end: veln_source::LineCol {
                line: 2,
                column: 5,
                offset: 6,
            },
        }),
        JsonValue::Null,
    );

    assert_eq!(
        lsp_diagnostic_json(&diagnostic),
        concat!(
            "{\"range\":{\"start\":{\"line\":1,\"character\":2},",
            "\"end\":{\"line\":1,\"character\":4}},\"severity\":1,",
            "\"code\":\"parse.expected_item\",\"source\":\"veln\",",
            "\"message\":\"expected a function or test declaration\"}"
        )
    );
}

#[test]
fn lsp_diagnostic_wire_preserves_invalid_standard_symbol_case_boundary() {
    let diagnostic = invalid_standard_symbol_case_diagnostic();

    assert_eq!(
        lsp_diagnostic_json(&diagnostic),
        concat!(
            "{\"range\":{\"start\":{\"line\":0,\"character\":0},",
            "\"end\":{\"line\":0,\"character\":0}},\"severity\":1,",
            "\"code\":\"toolchain.invalid_symbol_case\",\"source\":\"veln\",",
            "\"message\":\"compiler-provided function `BadAdapter` from `compiler_adapter` ",
            "must start with an ASCII lowercase letter\"}"
        )
    );

    let published = publish_diagnostics_for_uri("file://main.veln", &[diagnostic]);
    assert!(published.contains(r#""code":"toolchain.invalid_symbol_case""#));
    assert!(!published.contains("name.invalid_case"));
}

#[test]
fn server_publishes_semantic_diagnostics_after_parse_succeeds() {
    let mut server = Server::default();

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"pub fn main() -> ()\n  stdio::println(\"hello\")\nend\n"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""code":"effect.missing_public""#));
}

#[test]
fn server_clears_diagnostics_for_closed_document() {
    let mut server = Server::default();
    server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""diagnostics":[]"#));
}

#[test]
fn server_reads_and_writes_content_length_frames() {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let input = format!("Content-Length: {}\r\n\r\n{request}", request.len());
    let mut output = Vec::new();
    let mut server = Server::default();

    server.run(input.as_bytes(), &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("Content-Length: "));
    assert!(output.contains(r#""id":1"#));
}

fn invalid_standard_symbol_case_diagnostic() -> Diagnostic {
    Diagnostic::new(
        "toolchain.invalid_symbol_case",
        Severity::Error,
        DiagnosticKind::Toolchain,
        "compiler-provided function `BadAdapter` from `compiler_adapter` must start with an ASCII lowercase letter",
        None,
        JsonValue::object([
            ("provider", JsonValue::string("compiler_adapter")),
            ("name", JsonValue::string("BadAdapter")),
            ("name_class", JsonValue::string("function")),
            ("required_initial", JsonValue::string("ascii_lowercase")),
        ]),
    )
}
