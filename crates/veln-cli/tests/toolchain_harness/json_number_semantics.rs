use super::*;

#[test]
pub(super) fn manifest_mcp_assertions_reject_link_like_workspace_uris() {
    let root = test_temp_root("mcp-link-uri-manifest");
    let manifest_path = root.join("case.toml");

    #[cfg(unix)]
    {
        fs::create_dir(root.join("target-dir")).expect("target dir should be created");
        fs::write(root.join("target-dir").join("linked.veln"), "")
            .expect("target file should be written");
        std::os::unix::fs::symlink(
            root.join("target-dir").join("linked.veln"),
            root.join("link.veln"),
        )
        .expect("symlink file should be created");
        std::os::unix::fs::symlink(root.join("target-dir"), root.join("link-dir"))
            .expect("symlink directory should be created");

        for relative in ["link.veln", "link-dir/linked.veln"] {
            let error = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = {relative:?}\n"
                    ),
                );
            })
            .expect_err("link-like workspace_file_uri traversal should fail");
            assert!(
                panic_message(error).contains("link-like"),
                "unexpected error for {relative:?}"
            );
        }
    }
    #[cfg(windows)]
    {
        fs::write(root.join("target-file.veln"), "").expect("target file should be written");
        match std::os::windows::fs::symlink_file(
            root.join("target-file.veln"),
            root.join("link.veln"),
        ) {
            Ok(()) => {
                let error = std::panic::catch_unwind(|| {
                    parse_manifest(
                        &manifest_path,
                        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"link.veln\"\n",
                    );
                })
                .expect_err("Windows link-like workspace_file_uri traversal should fail");
                assert!(panic_message(error).contains("link-like"));
            }
            Err(error) => {
                eprintln!("skipping Windows link-like workspace_file_uri evidence: {error}");
            }
        }
    }
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[cfg(not(unix))]
#[test]
pub(super) fn workspace_file_uri_percent_encodes_native_non_unix_separators() {
    assert_eq!(
        path_to_file_uri(Path::new("workspace\\main.veln")),
        "file://workspace%5Cmain.veln"
    );
}

#[test]
pub(super) fn manifest_json_assertions_preserve_scalar_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals = 1.0
[[result_value_assert]]
value_path = "value"
path = "value"
equals = 1e0
[[lsp_assert]]
id = 1
path = "/value"
equals = 1.0
"#,
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "1e0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
}

#[test]
pub(super) fn manifest_non_mcp_assertions_preserve_container_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals = [1.0, {"nested": 1e0}]
[[result_value_assert]]
value_path = "value"
path = "value"
equals = {"nested": [1.0]}
[[lsp_assert]]
id = 1
path = "/value"
equals = {"nested": 1e0}
"#,
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Array(vec![
            JsonValue::Decimal("1.0".to_string()),
            JsonValue::Object(vec![(
                "nested".to_string(),
                JsonValue::Decimal("1e0".to_string())
            )])
        ])))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Array(vec![JsonValue::Decimal("1.0".to_string())])
        )])))
    );
    assert_eq!(
        manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Decimal("1e0".to_string())
        )])))
    );
}

#[test]
pub(super) fn manifest_mcp_assertions_preserve_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/decimal"
equals = 1.0
[[mcp_assert]]
id = 1
path = "/result/exponent"
equals = 1e0
[[mcp_assert]]
id = 1
path = "/result/nested"
equals = {"nested": [1.0, 1e0]}
"#,
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[1].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1e0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[2].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Array(vec![
                JsonValue::Decimal("1.0".to_string()),
                JsonValue::Decimal("1e0".to_string())
            ])
        )])))
    );
}

#[test]
pub(super) fn json_equality_preserves_object_array_kind_and_number_boundaries() {
    for (left, right, equal) in [
        (
            r#"{"outer":{"a":1,"b":[true,null]}}"#,
            r#"{"outer":{"b":[true,null],"a":1}}"#,
            true,
        ),
        (r#"[1,2]"#, r#"[1,2]"#, true),
        (r#"[1,2]"#, r#"[2,1]"#, false),
        (r#"[1,2]"#, r#"[1,2,3]"#, false),
        ("1", "1.0", false),
        ("1", "1e0", false),
        ("0", "-0", false),
        ("1.0", "1e0", false),
        (r#"{"nested":1}"#, r#"{"nested":2}"#, false),
        ("true", r#""true""#, false),
    ] {
        let left = parse_json(left).expect("left matrix value should parse");
        let right = parse_json(right).expect("right matrix value should parse");
        assert_eq!(json_values_equal(&left, &right), equal);
    }
}

pub(super) fn assert_number_spelling_adapter_result(
    result: Result<(), Box<dyn std::any::Any + Send>>,
    should_match: bool,
    adapter: &str,
    mismatch_context: &str,
) {
    if should_match {
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
        return;
    }

    let panic = match result {
        Ok(()) => panic!("{adapter} adapter should preserve number spelling"),
        Err(panic) => panic,
    };
    let message = panic_message(panic);
    assert!(message.contains(mismatch_context));
    assert!(message.contains("value mismatch"));
}

pub(super) fn assert_json_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let actual = parse_json(&format!(r#"{{"selected":{actual}}}"#))
        .expect("JSON adapter input should parse");
    let assertion = JsonAssertion {
        path: "selected".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json(expected).expect("JSON expectation should parse"),
        )),
    };
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_json_path(context, &actual, &assertion)),
        should_match,
        "JSON",
        "JSON path `selected` mismatch",
    );
}

pub(super) fn assert_lsp_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let stdout = lsp_frame(&format!(r#"{{"jsonrpc":"2.0","id":1,"result":{actual}}}"#));
    let assertions = parsed_lsp_assertions(&format!(
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result\"\nequals = {expected}\n"
    ));
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_lsp_assertions(context, &stdout, &assertions)),
        should_match,
        "LSP",
        "response id 1 path \"/result\"",
    );
}

pub(super) fn assert_mcp_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let stdout = format!(r#"{{"jsonrpc":"2.0","id":1,"result":{actual}}}"#) + "\n";
    let assertions = parsed_mcp_assertions(&format!(
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals = {expected}\n"
    ));
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| {
            assert_mcp_assertions(context, &stdout, &assertions, Path::new("."))
        }),
        should_match,
        "MCP",
        "response id 1 path \"/result\"",
    );
}

pub(super) fn assert_result_value_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let rendered = parse_json(&format!(r#"{{"rendered":"{actual}"}}"#))
        .expect("result-value adapter input should parse");
    let assertion = ResultValueAssertion {
        value_path: "rendered".to_string(),
        path: "value".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json(expected).expect("result-value expectation should parse"),
        )),
    };
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_result_value_path(context, &rendered, &assertion)),
        should_match,
        "result-value",
        "result value path `value` mismatch",
    );
}

pub(super) fn assert_number_spelling_through_every_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    assert_json_number_spelling_adapter(context, actual, expected, should_match);
    assert_lsp_number_spelling_adapter(context, actual, expected, should_match);
    assert_mcp_number_spelling_adapter(context, actual, expected, should_match);
    assert_result_value_number_spelling_adapter(context, actual, expected, should_match);
}

#[test]
pub(super) fn json_number_spelling_matrix_runs_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-number-spelling-adapters"),
        run_number: 1,
    };

    for (actual, expected, should_match) in [
        ("1", "1", true),
        ("1.0", "1.0", true),
        ("1e0", "1e0", true),
        ("1", "1.0", false),
        ("1", "1e0", false),
        ("1.0", "1", false),
        ("1.0", "1e0", false),
        ("1e0", "1", false),
        ("1e0", "1.0", false),
    ] {
        assert_number_spelling_through_every_adapter(&context, actual, expected, should_match);
    }
}

#[test]
pub(super) fn diagnostic_assertions_use_common_json_equality_for_integer_tokens() {
    let context = CaseRunContext {
        case_dir: Path::new("diagnostic-json-equality"),
        run_number: 1,
    };
    let json = parse_json(
        r#"{"diagnostics":[{"id":"type.mismatch","severity":"error","kind":"type","message":"expected `Int`, but found `String`","span":{"file":"main.veln","start":{"line":2,"column":3,"offset":23},"end":{"line":2,"column":7,"offset":27}}}]}"#,
    )
    .expect("diagnostic JSON should parse");

    assert_diagnostic(
        &context,
        &json,
        &DiagnosticExpectation {
            id: "type.mismatch".to_string(),
            severity: Some("error".to_string()),
            kind: Some("type".to_string()),
            message: Some("expected `Int`, but found `String`".to_string()),
            span: Some(SpanExpectation {
                file: Some("main.veln".to_string()),
                line: Some(2),
                column: Some(3),
            }),
        },
    );
}

#[test]
pub(super) fn reordered_json_objects_compare_equal_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-equality-adapters"),
        run_number: 1,
    };
    let actual = parse_json(r#"{"selected":{"outer":{"a":1,"b":2}}}"#)
        .expect("JSON adapter input should parse");
    assert_json_path(
        &context,
        &actual,
        &JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(r#"{"outer":{"b":2,"a":1}}"#)
                    .expect("JSON adapter expectation should parse"),
            )),
        },
    );

    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    assert_result_value_path(
        &context,
        &rendered,
        &ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(r#"{"value":2,"constructor":"ByteOffset"}"#)
                    .expect("result-value adapter expectation should parse"),
            )),
        },
    );

    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"a":1,"b":2}}}"#,
    ))
    .expect("LSP adapter input should decode");
    let lsp_assertion = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = {"outer":{"b":2,"a":1}}
"#,
    )
    .remove(0);
    evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
        .expect("LSP adapter should ignore object member order");

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"a":1,"b":2}}}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertion = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = {"outer":{"b":2,"a":1}}
"#,
    )
    .remove(0);
    evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
        .expect("MCP adapter should ignore object member order");
}

#[test]
pub(super) fn reordered_json_arrays_fail_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-array-order-adapters"),
        run_number: 1,
    };
    let actual = parse_json(r#"{"selected":[1,2]}"#).expect("JSON adapter input should parse");
    let json_assertion = JsonAssertion {
        path: "selected".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json("[2,1]").expect("JSON adapter expectation should parse"),
        )),
    };
    let panic = std::panic::catch_unwind(|| assert_json_path(&context, &actual, &json_assertion))
        .expect_err("JSON adapter should retain array order");
    assert!(panic_message(panic).contains("JSON path `selected` mismatch"));

    let rendered = parse_json(r#"{"rendered":"Cons(1, Cons(2, Nil))"}"#)
        .expect("result-value adapter input should parse");
    let result_assertion = ResultValueAssertion {
        value_path: "rendered".to_string(),
        path: "value".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json("[2,1]").expect("result-value expectation should parse"),
        )),
    };
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path(&context, &rendered, &result_assertion)
    })
    .expect_err("result-value adapter should retain array order");
    assert!(panic_message(panic).contains("result value path `value` mismatch"));

    let lsp_messages = decode_lsp_stdout(&lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}"#))
        .expect("LSP adapter input should decode");
    let lsp_assertion = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [2,1]
"#,
    )
    .remove(0);
    let error = evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
        .expect_err("LSP adapter should retain array order");
    assert!(error.contains("value mismatch"));

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertion = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [2,1]
"#,
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
        .expect_err("MCP adapter should retain array order");
    assert!(error.contains("value mismatch"));
}

pub(super) fn ordered_array_adapter_context() -> CaseRunContext<'static> {
    CaseRunContext {
        case_dir: Path::new("json-array-length-adapters"),
        run_number: 1,
    }
}
