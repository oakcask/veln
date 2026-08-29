use super::*;

#[test]
pub(super) fn manifest_lsp_and_mcp_file_backed_equality_loads_immutable_case_operands() {
    let root = test_temp_root("lsp-mcp-file-backed-equality");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    let expected_text = text_dir.join("expected.txt");
    let expected_json = text_dir.join("expected.json");
    fs::write(&expected_text, "expected text\n").expect("text sidecar should be written");
    fs::write(&expected_json, r#"{"b":[2],"a":1}"#).expect("JSON sidecar should be written");

    let lsp_manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals_json_file = "case-text/expected.json"
"#,
    );
    let mcp_manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
equals_file = "case-text/expected.txt"
[[mcp_assert]]
id = 1
path = "/result/value"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        lsp_manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::EqualsJsonFile(
            parse_json(r#"{"b":[2],"a":1}"#).expect("expected JSON should parse")
        ))
    );
    assert_eq!(
        mcp_manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::EqualsFile(
            "expected text\n".to_string()
        ))
    );
    assert!(matches!(
        mcp_manifest.expectations.mcp_assertions[1].operation,
        Some(RpcAssertionOperation::EqualsJsonFile(_))
    ));

    fs::write(&expected_text, "modified workspace text\n")
        .expect("text sidecar should be changed after manifest loading");
    fs::write(&expected_json, r#"{"modified":true}"#)
        .expect("JSON sidecar should be changed after manifest loading");

    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"a":1,"b":[2]}}"#,
    ))
    .expect("LSP response should decode");
    evaluate_lsp_assertion(&lsp_messages, &lsp_manifest.expectations.lsp_assertions[0])
        .expect("LSP JSON file equality should use the discovered case snapshot");

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"expected text\n","value":{"a":1,"b":[2]}}}
"#,
    )
    .expect("MCP response should decode");
    for assertion in &mcp_manifest.expectations.mcp_assertions {
        evaluate_mcp_assertion(&mcp_messages, assertion, &case_dir)
            .expect("MCP file equality should use the discovered case snapshot");
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_lsp_and_mcp_file_equality_rejects_invalid_missing_and_duplicate_operands() {
    let root = test_temp_root("lsp-mcp-invalid-json-sidecar");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");

    for (command, section) in [("lsp", "lsp_assert"), ("mcp", "mcp_assert")] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals_json_file = \"case-text/invalid.json\"\n"
                ),
            )
        })
        .expect_err("invalid JSON sidecar should fail manifest loading");
        let message = panic_message(error);
        assert!(message.contains(section), "{message}");
        assert!(message.contains("0"), "{message}");
        assert!(message.contains("response id 1"), "{message}");
        assert!(message.contains("path `/result`"), "{message}");
        assert!(message.contains("equals_json_file"), "{message}");
        assert!(message.contains("invalid"), "{message}");

        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals_json_file = \"case-text/missing.json\"\n"
                ),
            )
        })
        .expect_err("missing JSON sidecar should fail manifest loading");
        let message = panic_message(error);
        assert!(message.contains("case file `case-text/missing.json`"));
        assert!(message.contains(section), "{message}");
        assert!(message.contains("0"), "{message}");
        assert!(message.contains("response id 1"), "{message}");
        assert!(message.contains("path `/result`"), "{message}");
        assert!(message.contains("equals_json_file"), "{message}");

        assert_manifest_parse_error(
            &format!(
                "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals = null\nequals_json_file = \"case-text/missing.json\"\n"
            ),
            "needs exactly one",
        );
    }

    let error = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals_file = \"case-text/missing.txt\"\n",
        )
    })
    .expect_err("missing MCP text sidecar should fail manifest loading");
    let message = panic_message(error);
    assert!(message.contains("case file `case-text/missing.txt`"));
    assert!(message.contains("mcp_assert 0 response id 1 path `/result` equals_file"));

    for (command, section, operation, expected) in [
        (
            "lsp",
            "lsp_assert",
            "equals_json_file",
            "lsp_assert `equals_json_file` must be a string case file reference",
        ),
        (
            "mcp",
            "mcp_assert",
            "equals_file",
            "mcp_assert `equals_file` must be a string case file reference",
        ),
        (
            "mcp",
            "mcp_assert",
            "equals_json_file",
            "mcp_assert `equals_json_file` must be a string case file reference",
        ),
    ] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\n{operation} = 1\n"
            ),
            expected,
        );
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn lsp_and_mcp_file_backed_equality_report_operation_specific_failures() {
    let expected_object = parse_json(r#"{"a":2}"#).expect("expected object should parse");
    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":"one","result":{"a":1}}"#,
    ))
    .expect("LSP response should decode");
    let lsp_assertion = LspAssertion {
        id: Some(JsonValue::String("one".to_string())),
        method: None,
        occurrence: None,
        path: "/result".to_string(),
        path_present: true,
        pointer_tokens: vec!["result".to_string()],
        operation: Some(RpcAssertionOperation::EqualsJsonFile(
            expected_object.clone(),
        )),
        operation_count: 1,
    };
    assert_eq!(
        evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
            .expect_err("different LSP JSON should fail"),
        "value mismatch: expected {\"a\":2}, got {\"a\":1}"
    );

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":"one","result":{"text":1,"value":{"a":1}}}
"#,
    )
    .expect("MCP response should decode");
    let mut mcp_assertion = McpAssertion {
        id: Some(JsonValue::String("one".to_string())),
        path: "/result/text".to_string(),
        path_present: true,
        pointer_tokens: vec!["result".to_string(), "text".to_string()],
        operation: Some(RpcAssertionOperation::EqualsFile("1".to_string())),
        operation_count: 1,
    };
    assert_eq!(
        evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
            .expect_err("non-string MCP value should fail"),
        "equals_file requires a selected JSON string"
    );

    mcp_assertion.path = "/result/value".to_string();
    mcp_assertion.pointer_tokens = vec!["result".to_string(), "value".to_string()];
    mcp_assertion.operation = Some(RpcAssertionOperation::EqualsJsonFile(expected_object));
    assert_eq!(
        evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
            .expect_err("different MCP JSON should fail"),
        "value mismatch: expected {\"a\":2}, got {\"a\":1}"
    );

    let context = CaseRunContext {
        case_dir: Path::new("file-backed-runtime-context"),
        run_number: 1,
    };
    let lsp_panic = std::panic::catch_unwind(|| {
        assert_lsp_assertions(
            &context,
            &lsp_frame(r#"{"jsonrpc":"2.0","id":"one","result":{"a":1}}"#),
            &[lsp_assertion],
        );
    })
    .expect_err("aggregated LSP assertion should fail");
    let message = panic_message(lsp_panic);
    assert!(message.contains("file-backed-runtime-context run 1"));
    assert!(message.contains("lsp_assert 0"));
    assert!(message.contains("response id \"one\""));
    assert!(message.contains("path \"/result\""));
    assert!(message.contains("value mismatch"));

    let mcp_panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(
            &context,
            r#"{"jsonrpc":"2.0","id":"one","result":{"text":1,"value":{"a":1}}}
"#,
            &[mcp_assertion],
            Path::new("."),
        );
    })
    .expect_err("aggregated MCP assertion should fail");
    let message = panic_message(mcp_panic);
    assert!(message.contains("file-backed-runtime-context run 1"));
    assert!(message.contains("mcp_assert 0"));
    assert!(message.contains("response id \"one\""));
    assert!(message.contains("path \"/result/value\""));
    assert!(message.contains("value mismatch"));
}

#[test]
pub(super) fn lsp_and_mcp_shared_operations_produce_the_same_result() {
    let expected_object = parse_json(r#"{"a":2}"#).expect("expected object should parse");
    let cases = [
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":{"a":1}}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Equals(expected_object.clone()),
            RpcAssertionOperation::Equals(expected_object.clone()),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":"haystack"}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Contains("needle".to_string()),
            RpcAssertionOperation::Contains("needle".to_string()),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":[1]}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Length(2),
            RpcAssertionOperation::Length(2),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Missing(true),
            RpcAssertionOperation::Missing(true),
        ),
    ];

    for (message, pointer_tokens, lsp_operation, mcp_operation) in cases {
        let messages = vec![parse_json(message).expect("response should parse")];
        let lsp_assertion = LspAssertion {
            id: Some(JsonValue::String("one".to_string())),
            method: None,
            occurrence: None,
            path: "/result/value".to_string(),
            path_present: true,
            pointer_tokens: pointer_tokens.clone(),
            operation: Some(lsp_operation),
            operation_count: 1,
        };
        let mcp_assertion = McpAssertion {
            id: Some(JsonValue::String("one".to_string())),
            path: "/result/value".to_string(),
            path_present: true,
            pointer_tokens,
            operation: Some(mcp_operation),
            operation_count: 1,
        };

        assert_eq!(
            evaluate_lsp_assertion(&messages, &lsp_assertion),
            evaluate_mcp_assertion(&messages, &mcp_assertion, Path::new("."))
        );
    }
}

#[test]
pub(super) fn manifest_contains_operations_parse_and_reject_invalid_forms_through_every_adapter() {
    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "value"
contains = "needle"
"#,
    );
    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value"
contains = "needle"
"#,
    );
    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
contains = "needle"
"#,
    );
    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
contains = "needle"
"#,
    );

    assert_eq!(
        json_manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        result_manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        lsp_manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        mcp_manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::Contains("needle".to_string()))
    );

    for (command, section, fields) in [
        ("check", "json_assert", "path = \"value\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result\"\n"),
        ("mcp", "mcp_assert", "id = 1\npath = \"/result\"\n"),
    ] {
        for (operations, expected) in [
            ("", "needs exactly one"),
            (
                "contains = \"needle\"\nequals = \"needle\"\n",
                "needs exactly one",
            ),
            (
                "contains = \"first\"\ncontains = \"second\"\n",
                "duplicate key `contains`",
            ),
            ("missing = false\n", "missing` must be true"),
            ("contains = 1\n", "expected string"),
        ] {
            assert_manifest_parse_error(
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{fields}{operations}"
                ),
                expected,
            );
        }
    }
}

#[test]
pub(super) fn common_length_and_workspace_uri_operations_cover_every_json_adapter() {
    let root = test_temp_root("common-json-operations");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");
    let uri = workspace_file_uri(&root, "main.veln").expect("workspace URI should resolve");

    let json_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "items"
length = 2
[[json_assert]]
path = "uri"
workspace_file_uri = "main.veln"
"#,
    );
    let json = parse_json(&format!(r#"{{"items":[1,2],"uri":{uri:?}}}"#))
        .expect("JSON adapter input should parse");
    let context = CaseRunContext {
        case_dir: Path::new("common-json-operations"),
        run_number: 1,
    };
    for (index, assertion) in json_manifest
        .expectations
        .json_assertions
        .iter()
        .enumerate()
    {
        assert_json_path_in_workspace(&context, &json, assertion, index, &root);
    }

    let result_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value.field_path"
length = 2
[[result_value_assert]]
value_path = "uri_rendered"
path = "value.id"
workspace_file_uri = "main.veln"
"#,
    );
    let rendered = parse_json(&format!(
        r#"{{"rendered":"RuntimeByteDiagnostic(offset, Cons(first, Cons(second, Nil)), facts, preview)","uri_rendered":"RuntimeDiagnostic({uri}, message, detail)"}}"#
    ))
    .expect("result-value adapter input should parse");
    for (index, assertion) in result_manifest
        .expectations
        .result_value_assertions
        .iter()
        .enumerate()
    {
        assert_result_value_path_in_workspace(&context, &rendered, assertion, index, &root);
    }

    let lsp_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/items"
length = 2
[[lsp_assert]]
method = "note"
path = "/params/uri"
workspace_file_uri = "main.veln"
"#,
    );
    let lsp_stdout = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":[1,2]}}"#),
        lsp_frame(&format!(
            r#"{{"jsonrpc":"2.0","method":"note","params":{{"uri":{uri:?}}}}}"#
        ))
    );
    assert_lsp_assertions_in_workspace(
        &context,
        &lsp_stdout,
        &lsp_manifest.expectations.lsp_assertions,
        &root,
    );

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"value\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result\"\n"),
    ] {
        for (operation, expected) in [
            ("length = \"2\"\n", "expected integer"),
            ("workspace_file_uri = 1\n", "expected string"),
            (
                "length = 1\nworkspace_file_uri = \"main.veln\"\n",
                "needs exactly one",
            ),
        ] {
            assert_manifest_parse_error(
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}{operation}"
                ),
                expected,
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn common_length_and_workspace_uri_operand_errors_report_assertion_context() {
    let root = test_temp_root("common-json-operation-operand-context");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, selectors, selector_fragment, path_fragment) in [
        (
            "check",
            "json_assert",
            "path = \"value\"\n",
            "",
            "path `value`",
        ),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
            "",
            "path `value`",
        ),
        (
            "lsp",
            "lsp_assert",
            "id = 1\npath = \"/result\"\n",
            "response id 1",
            "path `/result`",
        ),
        (
            "mcp",
            "mcp_assert",
            "id = 1\npath = \"/result\"\n",
            "response id 1",
            "path `/result`",
        ),
    ] {
        for (operation, operation_fragment, failed_fact) in [
            ("length = \"2\"\n", "length", "expected integer"),
            (
                "workspace_file_uri = 1\n",
                "workspace_file_uri",
                "expected string",
            ),
            (
                "workspace_file_uri = \"../main.veln\"\n",
                "workspace_file_uri",
                "must not contain",
            ),
        ] {
            let panic = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}{operation}"
                    ),
                )
            })
            .expect_err("invalid assertion operand should fail");
            let message = panic_message(panic);
            for expected in [
                section,
                "0",
                selector_fragment,
                path_fragment,
                operation_fragment,
                failed_fact,
            ] {
                assert!(
                    expected.is_empty() || message.contains(expected),
                    "expected `{expected}` in `{message}`"
                );
            }
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}
