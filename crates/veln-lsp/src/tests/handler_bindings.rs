#[test]
fn companion_private_function_rename_preserves_target_symbol_identity() {
    let mut server = Server::default();
    let project = TempProject::new("rename-target-identity");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  increment(value)\n",
            "  increment\n",
            "end\n",
            "\n",
            "fn apply(increment: fn(Int) -> Int) -> Int\n",
            "  increment(1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  math::increment\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
        ),
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
    assert!(
        !responses[0].contains(r#""line":4,"character":8"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_unrelated_text_and_qualified_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-source-isolation");
    project.write(
        "math.veln",
        concat!(
            "use support\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  increment(value)\n",
            "  support::increment(value)\n",
            "  \"increment(1)\"\n",
            "  value\n",
            "end\n",
        ),
    );
    project.write(
        "support.veln",
        "pub fn increment(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  increment(1)\n",
            "  math::increment\n",
            "  \"math::increment(2)\"\n",
            "  # math::increment(3)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":2,"character":3},"end":{"line":2,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":3,"character":2},"end":{"line":3,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":9,"character":8"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":4,"character":11"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":3"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":10,"character":9"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":10"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_target_references_after_nested_blocks() {
    let mut server = Server::default();
    let project = TempProject::new("rename-nested-target-blocks");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn use_nested(value: Int) -> Int\n",
            "  if value > 0\n",
            "    increment(value)\n",
            "  else\n",
            "    0\n",
            "  end\n",
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
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":4},"end":{"line":6,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":10,"character":2},"end":{"line":10,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_local_callable_bindings() {
    let mut server = Server::default();
    let project = TempProject::new("rename-local-callable-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int, identity: fn(Int) -> Int) -> Int\n",
            "  increment(value)\n",
            "  let increment = identity\n",
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
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":2},"end":{"line":5,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":6"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_unannotated_callable_parameter_shadow() {
    let mut server = Server::default();
    let project = TempProject::new("rename-unannotated-callable-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int, increment) -> Int\n",
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
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        !responses[0].contains(r#""line":5,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_limits_pattern_binding_shadow_to_branch() {
    let mut server = Server::default();
    let project = TempProject::new("rename-pattern-binding-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn branch(value: Int, identity: fn(Int) -> Int) -> Int\n",
            "  if value > 0\n",
            "    let {callback: increment} = {callback: identity}\n",
            "    increment(value)\n",
            "  else\n",
            "    increment(value)\n",
            "  end\n",
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
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        !responses[0].contains(r#""line":6,"character":20"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":4"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":9,"character":4},"end":{"line":9,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":11,"character":2},"end":{"line":11,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_record_fields() {
    let mut server = Server::default();
    let project = TempProject::new("rename-record-field-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn inspect(value: Int) -> Int\n",
            "  let record = {increment: value}\n",
            "  record.increment\n",
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
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        !responses[0].contains(r#""line":5,"character":16"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":9"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_same_named_let_initializer_reference() {
    let mut server = Server::default();
    let project = TempProject::new("rename-let-initializer-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int) -> Int\n",
            "  let increment = increment\n",
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
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":18},"end":{"line":5,"character":27}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":6"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_match_arm_pattern_bindings() {
    let mut server = Server::default();
    let project = TempProject::new("rename-match-arm-pattern-shadow");
    project.write(
        "math.veln",
        concat!(
            "type Choice\n",
            "  Use {callback: fn(Int) -> Int}\n",
            "  Skip\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(choice: Choice, value: Int) -> Int\n",
            "  match choice\n",
            "    Use {callback: increment} => increment(value)\n",
            "    Skip => increment(value)\n",
            "  end\n",
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
        !responses[0].contains(r#""line":11,"character":19"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":33"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":12},"end":{"line":12,"character":21}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_target_references_after_else_if() {
    let mut server = Server::default();
    let project = TempProject::new("rename-else-if-target-blocks");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(value: Int) -> Int\n",
            "  if value == 0\n",
            "    0\n",
            "  else if value == 1\n",
            "    increment(value)\n",
            "  else\n",
            "    2\n",
            "  end\n",
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
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":4},"end":{"line":8,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":2},"end":{"line":12,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

