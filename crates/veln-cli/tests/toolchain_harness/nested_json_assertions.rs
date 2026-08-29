use super::*;

#[test]
pub(super) fn ordered_json_arrays_succeed_and_length_mismatches_keep_json_context() {
    let context = ordered_array_adapter_context();

    let actual = parse_json(r#"{"selected":[1,2]}"#).expect("JSON adapter input should parse");
    assert_json_path(
        &context,
        &actual,
        &JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json("[1,2]").expect("JSON success expectation should parse"),
            )),
        },
    );
    let panic = std::panic::catch_unwind(|| {
        assert_json_path(
            &context,
            &actual,
            &JsonAssertion {
                path: "selected".to_string(),
                operation: Some(ValueAssertionOperation::Equals(
                    parse_json("[1,2,3]").expect("JSON failure expectation should parse"),
                )),
            },
        )
    })
    .expect_err("JSON adapter should reject array length mismatch");
    assert!(panic_message(panic).contains("JSON path `selected` mismatch"));
}

#[test]
pub(super) fn ordered_json_arrays_succeed_and_length_mismatches_keep_result_value_context() {
    let context = ordered_array_adapter_context();

    let rendered = parse_json(r#"{"rendered":"Cons(1, Cons(2, Nil))"}"#)
        .expect("result-value adapter input should parse");
    assert_result_value_path(
        &context,
        &rendered,
        &ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json("[1,2]").expect("result-value success expectation should parse"),
            )),
        },
    );
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path(
            &context,
            &rendered,
            &ResultValueAssertion {
                value_path: "rendered".to_string(),
                path: "value".to_string(),
                operation: Some(ValueAssertionOperation::Equals(
                    parse_json("[1,2,3]").expect("result-value failure expectation should parse"),
                )),
            },
        )
    })
    .expect_err("result-value adapter should reject array length mismatch");
    assert!(panic_message(panic).contains("result value path `value` mismatch"));
}

#[test]
pub(super) fn ordered_json_arrays_succeed_and_length_mismatches_keep_lsp_context() {
    let context = ordered_array_adapter_context();

    let lsp_stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}"#);
    let lsp_success = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [1,2]
"#,
    );
    assert_lsp_assertions(&context, &lsp_stdout, &lsp_success);
    let lsp_failure = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [1,2,3]
"#,
    );
    let panic =
        std::panic::catch_unwind(|| assert_lsp_assertions(&context, &lsp_stdout, &lsp_failure))
            .expect_err("LSP adapter should reject array length mismatch");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.contains("value mismatch"));
}

#[test]
pub(super) fn ordered_json_arrays_succeed_and_length_mismatches_keep_mcp_context() {
    let context = ordered_array_adapter_context();

    let mcp_stdout = r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}
"#;
    let mcp_success = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [1,2]
"#,
    );
    assert_mcp_assertions(&context, mcp_stdout, &mcp_success, Path::new("."));
    let mcp_failure = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [1,2,3]
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(&context, mcp_stdout, &mcp_failure, Path::new("."))
    })
    .expect_err("MCP adapter should reject array length mismatch");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.contains("value mismatch"));
}

pub(super) fn kind_and_nested_adapter_context() -> CaseRunContext<'static> {
    CaseRunContext {
        case_dir: Path::new("json-kind-nested-adapters"),
        run_number: 1,
    }
}

#[test]
pub(super) fn kind_and_nested_json_mismatches_retain_json_context() {
    let context = kind_and_nested_adapter_context();

    let actual = parse_json(r#"{"selected":{"outer":{"nested":1}}}"#)
        .expect("JSON adapter input should parse");
    for expected in [r#""object""#, r#"{"outer":{"nested":2}}"#] {
        let assertion = JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(expected).expect("JSON expectation should parse"),
            )),
        };
        let panic = std::panic::catch_unwind(|| assert_json_path(&context, &actual, &assertion))
            .expect_err("JSON adapter should reject mismatched value");
        let message = panic_message(panic);
        assert!(message.contains("JSON path `selected` mismatch"));
        assert!(message.contains("value mismatch"));
    }
}

#[test]
pub(super) fn kind_and_nested_json_mismatches_retain_result_value_context() {
    let context = kind_and_nested_adapter_context();

    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    for expected in [
        r#""ByteOffset""#,
        r#"{"constructor":"ByteOffset","value":3}"#,
    ] {
        let assertion = ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(expected).expect("result-value expectation should parse"),
            )),
        };
        let panic =
            std::panic::catch_unwind(|| assert_result_value_path(&context, &rendered, &assertion))
                .expect_err("result-value adapter should reject mismatched value");
        let message = panic_message(panic);
        assert!(message.contains("result value path `value` mismatch"));
        assert!(message.contains("value mismatch"));
    }
}

#[test]
pub(super) fn kind_and_nested_json_mismatches_retain_lsp_context() {
    let context = kind_and_nested_adapter_context();

    let lsp_stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"nested":1}}}"#);
    let lsp_failures = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = "object"
[[lsp_assert]]
id = 1
path = "/result"
equals = {"outer":{"nested":2}}
"#,
    );
    let panic =
        std::panic::catch_unwind(|| assert_lsp_assertions(&context, &lsp_stdout, &lsp_failures))
            .expect_err("LSP adapter should reject mismatched values");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.matches("value mismatch").count() >= 2);
}

#[test]
pub(super) fn kind_and_nested_json_mismatches_retain_mcp_context() {
    let context = kind_and_nested_adapter_context();

    let mcp_stdout = r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"nested":1}}}
"#;
    let mcp_failures = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = "object"
[[mcp_assert]]
id = 1
path = "/result"
equals = {"outer":{"nested":2}}
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(&context, mcp_stdout, &mcp_failures, Path::new("."))
    })
    .expect_err("MCP adapter should reject mismatched values");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.matches("value mismatch").count() >= 2);
}

#[test]
pub(super) fn decoded_mcp_jsonl_assertions_cover_success_matrix() {
    let root = test_temp_root("mcp-jsonl");
    fs::write(root.join("main file.veln"), "").expect("workspace file should be written");
    let uri = path_to_file_uri(&root.join("main file.veln").canonicalize().unwrap());
    let stdout = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"alpha\",\"result\":{{\"value\":{{\"a/b\":\"slash\",\"m~n\":\"tilde\"}},\"items\":[\"first\",\"second\"],\"uri\":\"{uri}\",\"decimal\":1.0,\"exponent\":1e0}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"object\":{{\"z\":1,\"a\":2}}}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":9223372036854775808,\"result\":{{\"selected\":\"wide\"}}}}\n"
    );
    let source = r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/value/a~1b"
equals = "slash"
[[mcp_assert]]
id = "alpha"
path = "/result/value/m~0n"
equals = "tilde"
[[mcp_assert]]
id = "alpha"
path = "/result/items"
equals = ["first", "second"]
[[mcp_assert]]
id = "alpha"
path = "/result/items"
length = 2
[[mcp_assert]]
id = "alpha"
path = "/result/absent"
missing = true
[[mcp_assert]]
id = "alpha"
path = "/result/uri"
workspace_file_uri = "main file.veln"
[[mcp_assert]]
id = "alpha"
path = "/result/decimal"
equals = 1.0
[[mcp_assert]]
id = "alpha"
path = "/result/exponent"
equals = 1e0
[[mcp_assert]]
id = 2
path = "/result/object"
equals = {"a":2,"z":1}
[[mcp_assert]]
id = 9223372036854775808
path = "/result/selected"
equals = "wide"
"#;
    let assertions = parse_manifest(&root.join("case.toml"), source)
        .expectations
        .mcp_assertions;
    let context = CaseRunContext {
        case_dir: Path::new("decoded-mcp"),
        run_number: 1,
    };
    assert_mcp_assertions(&context, &stdout, &assertions, &root);

    let reversed = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/items"
equals = ["second", "first"]
"#,
    )
    .remove(0);
    let messages = decode_mcp_stdout(&stdout).expect("stream should decode");
    let error = evaluate_mcp_assertion(&messages, &reversed, &root)
        .expect_err("reversed array should fail equality");
    assert!(error.contains("value mismatch"));

    let integer_decimal = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/decimal"
equals = 1
"#,
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&messages, &integer_decimal, &root)
        .expect_err("integer spelling should not equal decimal spelling");
    assert!(error.contains("value mismatch"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn decoded_mcp_jsonl_rejection_matrix_reports_actionable_failures() {
    for (stdout, expected) in [
        ("{".to_string(), "invalid JSON"),
        ("[]\n".to_string(), "not a JSON-RPC object"),
    ] {
        let error = decode_mcp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }

    let root = test_temp_root("mcp-jsonl-reject");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let duplicate_messages = decode_mcp_stdout(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[\"first\",\"second\"],\"text\":\"value\"}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n",
    )
    .expect("stream should decode");
    let messages = decode_mcp_stdout(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[\"first\",\"second\"],\"text\":\"value\"}}\n",
    )
    .expect("stream should decode");

    for (fields, expected) in [
        (
            "id = 9\npath = \"/result\"\nmissing = true",
            "selected response was not found",
        ),
        (
            "id = 1\npath = \"/result/missing\"\nequals = null",
            "selected JSON path was not found",
        ),
        (
            "id = 1\npath = \"/result/missing\"\nlength = 0",
            "selected JSON path was not found",
        ),
        (
            "id = 1\npath = \"/result/text/value\"\nmissing = true",
            "invalid traversal",
        ),
        (
            "id = 1\npath = \"/result/text\"\nlength = 1",
            "requires a selected JSON array",
        ),
        (
            "id = 1\npath = \"/result/items\"\nlength = 1",
            "array length mismatch",
        ),
        (
            "id = 1\npath = \"/result/text\"\nmissing = true",
            "exists but should be missing",
        ),
    ] {
        let assertion = parsed_mcp_assertions(&format!(
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\n{fields}\n"
        ))
        .remove(0);
        let error = evaluate_mcp_assertion(&messages, &assertion, &root)
            .expect_err("assertion should report its failure");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
    let duplicate_assertion = parsed_mcp_assertions(
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals = null\n",
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&duplicate_messages, &duplicate_assertion, &root)
        .expect_err("duplicate selected id should fail");
    assert!(error.contains("matched 2 responses"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn decoded_lsp_stream_selectors_and_json_pointer_object_matrix_succeed() {
    let response = r#"{"jsonrpc":"2.0","id":2,"result":{"":"empty","a.b":"dot","0":"numeric","a/b":"slash","m~n":"tilde","a b":"space","日本語":"unicode","~1":"escape-order","adj~/":"adjacent"}}"#;
    let first = r#"{"jsonrpc":"2.0","method":"note","params":{"value":"first"}}"#;
    let second = r#"{"jsonrpc":"2.0","method":"note","params":{"value":"second"}}"#;
    let stdout = format!(
        "{}{}{}",
        lsp_frame(response),
        lsp_frame(first),
        lsp_frame(second)
    );
    let source = r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 2
path = "/result/"
equals = "empty"
[[lsp_assert]]
id = 2
path = "/result/a.b"
equals = "dot"
[[lsp_assert]]
id = 2
path = "/result/0"
equals = "numeric"
[[lsp_assert]]
id = 2
path = "/result/a~1b"
equals = "slash"
[[lsp_assert]]
id = 2
path = "/result/m~0n"
equals = "tilde"
[[lsp_assert]]
id = 2
path = "/result/a b"
equals = "space"
[[lsp_assert]]
id = 2
path = "/result/日本語"
equals = "unicode"
[[lsp_assert]]
id = 2
path = "/result/~01"
equals = "escape-order"
[[lsp_assert]]
id = 2
path = "/result/adj~0~1"
equals = "adjacent"
[[lsp_assert]]
method = "note"
occurrence = 1
path = "/params/value"
equals = "second"
[[lsp_assert]]
method = "note"
path = "/params/value"
equals = "first"
[[lsp_assert]]
id = 2
path = ""
equals = {"jsonrpc":"2.0","id":2,"result":{"":"empty","a.b":"dot","0":"numeric","a/b":"slash","m~n":"tilde","a b":"space","日本語":"unicode","~1":"escape-order","adj~/":"adjacent"}}
"#;
    let assertions = parsed_lsp_assertions(source);
    let context = CaseRunContext {
        case_dir: Path::new("decoded-lsp"),
        run_number: 1,
    };
    assert_lsp_assertions(&context, &stdout, &assertions);
}
