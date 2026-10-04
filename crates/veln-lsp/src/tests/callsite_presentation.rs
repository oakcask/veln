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
