#[test]
fn companion_private_function_rename_rejects_suffix_qualified_references() {
    let mut server = Server::default();
    let project = TempProject::new("rename-qualified-path-boundary");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "other/math.veln",
        "pub fn increment(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "use other::math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  other::math::increment(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 4, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":4,"character":8},"end":{"line":4,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":15"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_result_binding_contract_scope() {
    let mut server = Server::default();
    let project = TempProject::new("rename-result-binding-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> increment: Int\n",
            "  ensure increment >= value\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        !responses[0].contains(r#""line":0,"character":28"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":1,"character":9"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":2,"character":2},"end":{"line":2,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_satisfy_candidate_scope() {
    let mut server = Server::default();
    let project = TempProject::new("rename-satisfy-candidate-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(fallback: Int) -> Int\n",
            "  _choice satisfy increment => increment > 0\n",
            "  increment(fallback)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        !responses[0].contains(r#""line":5,"character":19"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":32"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":2},"end":{"line":6,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_includes_handler_operation_clause_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-call");
    project.write(
        "math.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":9,"character":19},"end":{"line":9,"character":28}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_from_multiline_clause_call_covers_clause_body_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-multiline-call");
    project.write(
        "math.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => if value == 0\n",
            "    increment(value)\n",
            "  else\n",
            "    increment(value + 1)\n",
            "  end\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 10, 6, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":10,"character":4},"end":{"line":10,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":4},"end":{"line":12,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_skips_record_fields() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-field-isolation");
    project.write(
        "main.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => { value: value, other: 1 }.value + value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 5, 10, "amount_value"));

    assert_eq!(responses.len(), 1);
    assert_eq!(
        responses[0].matches(r#""newText":"amount_value""#).count(),
        3
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":9},"end":{"line":5,"character":14}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":28},"end":{"line":5,"character":33}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":54},"end":{"line":5,"character":59}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":21"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":46"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_covers_multiline_body_references() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-multiline-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => match value\n",
            "    true => value\n",
            "    value => value\n",
            "    false => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 5, 8, "input"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":12},"end":{"line":6,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":13},"end":{"line":8,"character":18}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":4"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":13"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_keeps_else_if_body_scope_bounded() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-else-if-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => if value == 0\n",
            "    value\n",
            "  else if value == 1\n",
            "    value\n",
            "  else\n",
            "    value\n",
            "  end\n",
            "  fallback(value) => value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 6, 8, "input"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 6);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":10},"end":{"line":8,"character":15}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":11,"character":4},"end":{"line":11,"character":9}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":13,"character":11"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":13,"character":21"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_definition_uses_multiline_body_scope() {
    let mut server = Server::default();
    let project = TempProject::new("definition-handler-operation-clause-multiline-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => match value\n",
            "    true => value\n",
            "    value => value\n",
            "    false => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&definition_request(&main_uri, 8, 15));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":7},"end":{"line":5,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    let shadowed = server.handle_message(&definition_request(&main_uri, 7, 15));
    assert_eq!(shadowed.len(), 1);
    assert!(shadowed[0].contains(r#""result":null"#), "{}", shadowed[0]);
}

#[test]
fn handler_context_callable_binding_shadows_top_level_function_in_clause_body() {
    let mut server = Server::default();
    let project = TempProject::new("handler-context-callable-binding");
    project.write(
        "main.veln",
        concat!(
            "fn callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "  echo(value: Int) -> Int\n",
            "  reset(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
            "  amount(value) => callback(value)\n",
            "  echo(value) => callback(value) + callback(1)\n",
            "  reset(callback) => callback\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 11, 21));
    let references = server.handle_message(&references_request(&main_uri, 10, 17));
    let context_rename = server.handle_message(&rename_request(&main_uri, 10, 17, "project"));
    let clause_rename = server.handle_message(&rename_request(&main_uri, 13, 8, "value"));

    assert_context_binding_definition(&definition[0]);
    assert_context_binding_references(&references[0]);
    assert_context_binding_rename(&context_rename[0]);
    assert_clause_binding_rename(&clause_rename[0]);
}

fn assert_context_binding_definition(response: &str) {
    assert_contains_json(
        response,
        r#""range":{"start":{"line":10,"character":15},"end":{"line":10,"character":23}}"#,
    );
}

fn assert_context_binding_references(response: &str) {
    for expected in [
        r#""range":{"start":{"line":11,"character":19},"end":{"line":11,"character":27}}"#,
        r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#,
        r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#,
    ] {
        assert_contains_json(response, expected);
    }
    assert_not_contains_json(response, r#""line":0,"character":3"#);
    assert_not_contains_json(response, r#""line":13,"character":7"#);
}

fn assert_context_binding_rename(response: &str) {
    assert_eq!(response.matches(r#""newText":"project""#).count(), 4);
    for expected in [
        r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#,
        r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#,
    ] {
        assert_contains_json(response, expected);
    }
    assert_not_contains_json(response, r#""line":13,"character":7"#);
}

fn assert_clause_binding_rename(response: &str) {
    assert_eq!(response.matches(r#""newText":"value""#).count(), 2);
    for expected in [
        r#""range":{"start":{"line":13,"character":8},"end":{"line":13,"character":16}}"#,
        r#""range":{"start":{"line":13,"character":21},"end":{"line":13,"character":29}}"#,
    ] {
        assert_contains_json(response, expected);
    }
}
