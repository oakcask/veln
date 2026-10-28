#[test]
fn server_presents_callsite_modifier_builtin_and_signature() {
    let mut server = Server::default();
    let project = TempProject::new("callsite-presentation");
    project.write(
        "main.veln",
        concat!(
            "fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  located(\"hello\")\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let initialized = server.handle_message(&initialize_request(&root_uri));
    assert!(initialized[0].contains(r#""completionProvider""#));
    assert!(initialized[0].contains(r#""signatureHelpProvider""#));

    let builtin = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":20,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{main_uri}"}},"position":{{"line":1,"character":2}}}}}}"#
    ));
    assert!(builtin[0].contains(r#""label":"callsite","kind":6"#), "{}", builtin[0]);
    assert!(builtin[0].contains("built-in SourceLocation local"));

    let ordinary = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":21,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{main_uri}"}},"position":{{"line":4,"character":2}}}}}}"#
    ));
    assert!(ordinary[0].contains(r#""result":[]"#), "{}", ordinary[0]);

    let modifier = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":23,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{main_uri}"}},"position":{{"line":3,"character":20}}}}}}"#
    ));
    assert!(
        modifier[0].contains(r#""label":"callsite","kind":14"#),
        "{}",
        modifier[0]
    );

    let signature = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":22,"method":"textDocument/signatureHelp","params":{{"textDocument":{{"uri":"{main_uri}"}},"position":{{"line":7,"character":17}}}}}}"#
    ));
    assert!(
        signature[0].contains("fn located(message: String) -&gt; SourceLocation callsite")
            || signature[0].contains("fn located(message: String) -> SourceLocation callsite"),
        "{}",
        signature[0]
    );
    assert!(signature[0].contains(r#""parameters":[{"label":"message: String"}]"#));

    let semantic_tokens = server.handle_message(&semantic_tokens_request(&main_uri));
    assert!(
        semantic_tokens[0].contains(r#""result":{"data":["#),
        "{}",
        semantic_tokens[0]
    );
}

#[test]
fn server_presents_explicit_import_alias_hover_and_qualified_completion() {
    let mut server = Server::default();
    let project = TempProject::new("explicit-import-alias-presentation");
    project.write("alpha.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write(
        "app.veln",
        "mod app\nuse alpha as calc\nfn main() -> Int\n  calc::value()\nend\n",
    );
    project.write("collision_a.veln", "mod collision\nuse alpha as calc\n");
    project.write("collision_b.veln", "mod collision\nuse app as calc\n");
    project.write(
        "collision_use.veln",
        "mod collision\nfn main() -> Int\n  calc::value()\nend\n",
    );
    project.write(
        "invalid.veln",
        "use alpha as Calc\nfn main() -> Int\n  Calc::value()\nend\n",
    );
    project.write(
        "recovered.veln",
        "use alpha as calc as nested\nfn main() -> Int\n  calc::value()\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let initialized = server.handle_message(&initialize_request(&root_uri));
    assert!(initialized[0].contains(r#""hoverProvider":true"#));

    let alias_hover = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":40,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{app_uri}"}},"position":{{"line":1,"character":13}}}}}}"#
    ));
    assert!(alias_hover[0].contains("import alias calc"), "{}", alias_hover[0]);
    assert!(alias_hover[0].contains("package: workspace"), "{}", alias_hover[0]);
    assert!(alias_hover[0].contains("target module: alpha"), "{}", alias_hover[0]);

    let member_hover = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":41,"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{app_uri}"}},"position":{{"line":3,"character":8}}}}}}"#
    ));
    assert!(member_hover[0].contains("function calc::value"), "{}", member_hover[0]);

    let completion = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":42,"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{app_uri}"}},"position":{{"line":3,"character":8}}}}}}"#
    ));
    assert!(completion[0].contains(r#""label":"value","kind":3"#), "{}", completion[0]);
    assert!(completion[0].contains("module alpha via calc"), "{}", completion[0]);

    for (id, path, line, character) in [
        (43, "collision_use.veln", 2, 8),
        (44, "invalid.veln", 2, 8),
        (45, "recovered.veln", 2, 8),
    ] {
        let uri = path_to_uri(&project.root.join(path));
        let hover = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/hover","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
        ));
        assert!(hover[0].contains(r#""result":null"#), "{}", hover[0]);
        let completion = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/completion","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
        ));
        assert!(completion[0].contains(r#""result":[]"#), "{}", completion[0]);
    }
}

#[test]
fn server_resolves_signature_help_through_a_deep_public_alias_chain() {
    let mut server = Server::default();
    let project = TempProject::new("callsite-deep-alias-presentation");
    let mut source = String::from(
        "pub fn located(message: String) -> SourceLocation callsite\n  callsite\nend\n",
    );
    source.push_str("pub fn alias_0 = located\n");
    for index in 1..70 {
        source.push_str(&format!("pub fn alias_{index} = alias_{}\n", index - 1));
    }
    source.push_str("fn caller() -> SourceLocation\n  alias_69(\"hello\")\nend\n");
    project.write("main.veln", &source);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let signature = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":24,"method":"textDocument/signatureHelp","params":{{"textDocument":{{"uri":"{main_uri}"}},"position":{{"line":74,"character":18}}}}}}"#
    ));

    assert!(
        signature[0].contains("fn located(message: String) -&gt; SourceLocation callsite")
            || signature[0].contains("fn located(message: String) -> SourceLocation callsite"),
        "{}",
        signature[0]
    );
}
