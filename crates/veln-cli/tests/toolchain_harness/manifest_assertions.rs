use super::*;

#[test]
pub(super) fn manifest_json_assertions_support_missing_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "error.details.byte_diagnostic.byte_preview"
missing = true
"#,
    );

    let assertion = &manifest.expectations.json_assertions[0];
    assert_eq!(assertion.path, "error.details.byte_diagnostic.byte_preview");
    assert_eq!(assertion.operation, Some(ValueAssertionOperation::Missing));
}

#[test]
pub(super) fn manifest_value_assertions_keep_operation_and_operand_together() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "message"
contains = "needle"

[[result_value_assert]]
value_path = "error.details.value"
path = "value.count"
equals = 3
"#,
    );

    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "3".to_string()
        )))
    );
}

#[test]
#[should_panic(
    expected = "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
)]
pub(super) fn manifest_json_assertions_reject_mixed_equals_and_missing() {
    parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "status"
equals = "failed"
missing = true
"#,
    );
}

#[test]
pub(super) fn manifest_json_assertions_parse_equals_json_file() {
    let root = test_temp_root("json-assert-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(
        text_dir.join("expected.json"),
        "{\"nested\":[1,true,null]}\n",
    )
    .expect("expected JSON sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::EqualsJsonFile(JsonValue::Object(
            vec![(
                "nested".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Decimal("1".to_string()),
                    JsonValue::Bool(true),
                    JsonValue::Null
                ])
            )]
        )))
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn manifest_result_value_assertions_parse_equals_json_file() {
    let root = test_temp_root("result-value-assert-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("expected.json"), "[\"ok\",2]\n")
        .expect("expected JSON sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::EqualsJsonFile(JsonValue::Array(
            vec![
                JsonValue::String("ok".to_string()),
                JsonValue::Decimal("2".to_string())
            ]
        )))
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn manifest_equals_json_file_rejects_invalid_json() {
    let root = test_temp_root("invalid-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");

    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/invalid.json"
"#,
        )
    })
    .expect_err("invalid equals_json_file JSON should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid json_assert equals_json_file value"),
        "expected invalid JSON error, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn manifest_equals_json_file_cardinality_is_checked_before_file_io() {
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals = "inline"
equals_json_file = "case-text/missing-sidecar.json"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
equals = "inline"
equals_json_file = "case-text/missing-sidecar.json"
"#,
        "result_value_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
}

#[test]
pub(super) fn manifest_assertion_missing_false_is_checked_before_file_io() {
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "status"
missing = false

[[json_assert]]
path = "stdout"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 `missing` must be true when present",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
missing = false

[[result_value_assert]]
value_path = "error.other"
path = "value"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "result_value_assert 0 `missing` must be true when present",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[[file_assert]]
path = "out.txt"
missing = false

[[file_assert]]
path = "other.txt"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "file_assert 0 `missing` must be true when present",
        "missing-sidecar",
    );
}

#[test]
pub(super) fn manifest_assertion_operation_omission_is_checked_before_file_io() {
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "status"

[[json_assert]]
path = "stdout"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"

[[result_value_assert]]
value_path = "error.other"
path = "value"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "result_value_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[[file_assert]]
path = "out.txt"

[[file_assert]]
path = "other.txt"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "file_assert 0 needs exactly one of `equals`, `equals_file`, or `missing = true`",
        "missing-sidecar",
    );
    for (command, section) in [("lsp", "lsp_assert"), ("mcp", "mcp_assert")] {
        assert_manifest_parse_error_without(
            &format!(
                r#"
command = ["{command}"]
exit = 0

[[{section}]]
id = 1
path = "/result"

[[{section}]]
id = 2
path = "/result"
equals_file = "case-text/missing-sidecar.txt"
"#
            ),
            &format!(
                "{section} 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
            ),
            "missing-sidecar",
        );
    }
}

#[test]
pub(super) fn manifest_preflight_uses_later_rpc_context_before_earlier_sidecar_io() {
    for (command, section) in [("lsp", "lsp_assert"), ("mcp", "mcp_assert")] {
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                Path::new("case.toml"),
                &format!(
                    r#"
command = ["{command}"]
exit = 0

[[json_assert]]
path = "status"
equals_file = "case-text/missing-sidecar.txt"

[[{section}]]
length = "invalid"
id = 7
path = "/result/items"
"#
                ),
            )
        })
        .expect_err("invalid later RPC operand should fail preflight");
        let message = panic_message(panic);
        assert!(message.contains(&format!("{section} 0")), "{message}");
        assert!(message.contains("response id 7"), "{message}");
        assert!(message.contains("path `/result/items` length"), "{message}");
        assert!(message.contains("expected integer"), "{message}");
        assert!(!message.contains("missing-sidecar"), "{message}");
    }
}

#[test]
pub(super) fn manifest_equals_json_file_loads_before_skip_evaluation() {
    let root = test_temp_root("equals-json-file-skip-lifecycle");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[skip]
reason = "would skip after manifest loading"

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/invalid.json"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case_with_guard_and_after_invocation(
            &case_dir,
            |_| {},
            |_, _| panic!("command lifecycle should not reach invocation"),
        );
    })
    .expect_err("invalid equals_json_file should be rejected before skip evaluation");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid json_assert equals_json_file value"),
        "expected invalid JSON error, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn manifest_jsonrpc_fixture_frames_envelope_matrix_and_exact_case_text_bytes() {
    let root = test_temp_root("jsonrpc-fixture-framing");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(
        text_dir.join("exact.raw"),
        b"\xef\xbb\xbfalpha\r\n\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\r\n",
    )
    .expect("exact case text should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","id":"request","method":"unknown/string","params":{"nested":[{"$case_text":"case-text/exact.raw"}],"emoji":"\uD83D\uDE42"},"extension":true},
  {"jsonrpc":"2.0","id":1.5,"method":"unknown/number","params":[1,null]},
  {"jsonrpc":"2.0","id":null,"method":"unknown/null","params":null},
  {"jsonrpc":"2.0","method":"unknown/notification"},
  {"jsonrpc":"2.0","id":"request","method":"unknown/duplicate","params":{"methodSpecific":"unchecked"}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let bodies = [
        "{\"jsonrpc\":\"2.0\",\"id\":\"request\",\"method\":\"unknown/string\",\"params\":{\"nested\":[\"\u{feff}alpha\\r\\n日本語\\r\\n\"],\"emoji\":\"🙂\"},\"extension\":true}",
        "{\"jsonrpc\":\"2.0\",\"id\":1.5,\"method\":\"unknown/number\",\"params\":[1,null]}",
        "{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"unknown/null\",\"params\":null}",
        "{\"jsonrpc\":\"2.0\",\"method\":\"unknown/notification\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"request\",\"method\":\"unknown/duplicate\",\"params\":{\"methodSpecific\":\"unchecked\"}}",
    ];
    let expected = bodies
        .iter()
        .map(|body| format!("Content-Length: {}\r\n\r\n{body}", body.len()))
        .collect::<String>();
    assert_eq!(
        manifest.invocation.stdin.as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        manifest.invocation.stdin_jsonrpc_file.as_deref(),
        Some("requests.json")
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_jsonrpc_fixture_rejects_root_message_and_envelope_failures() {
    let root = test_temp_root("jsonrpc-fixture-envelope-failures");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    let cases = [
        ("{", "invalid JSON-RPC fixture"),
        ("{}", "root must be an array"),
        ("[null]", "message 0 at $[0]: message must be an object"),
        ("[{\"method\":\"m\"}]", "message 0 at $[0].jsonrpc"),
        (
            "[{\"jsonrpc\":\"2.0\"}]",
            "message 0 at $[0].method: `method` must be a string",
        ),
        (
            "[{\"jsonrpc\":\"1.0\",\"method\":\"m\"}]",
            "`jsonrpc` must be the string `2.0`",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"jsonrpc\":\"1.0\",\"method\":\"m\"}]",
            "message 0 at $[0].jsonrpc: `jsonrpc` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"1.0\",\"jsonrpc\":\"2.0\",\"method\":\"m\"}]",
            "message 0 at $[0].jsonrpc: `jsonrpc` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"method\":false}]",
            "message 0 at $[0].method: `method` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":false,\"method\":\"m\"}]",
            "message 0 at $[0].method: `method` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":1,\"id\":true}]",
            "message 0 at $[0].id: `id` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":true,\"id\":1}]",
            "message 0 at $[0].id: `id` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":null,\"params\":\"bad\"}]",
            "message 0 at $[0].params: `params` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":\"bad\",\"params\":null}]",
            "message 0 at $[0].params: `params` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":false}]",
            "message 0 at $[0].method: `method` must be a string",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":true}]",
            "message 0 at $[0].id: `id` must be a string, number, or null",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":\"bad\"}]",
            "message 0 at $[0].params: `params` must be an object, array, or null",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"result\":null}]",
            "request or notification must not contain `result` or `error`",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"error\":null}]",
            "request or notification must not contain `result` or `error`",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"emoji":"\uD83D"}}]"#,
            "unpaired high surrogate",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"emoji":"\uDE42"}}]"#,
            "unpaired low surrogate",
        ),
    ];
    for (fixture, expected) in cases {
        fs::write(case_dir.join("requests.json"), fixture)
            .expect("JSON-RPC fixture should be written");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
            );
        })
        .expect_err("invalid JSON-RPC fixture should fail");
        let message = panic_message(panic);
        assert!(
            message.contains(expected),
            "expected `{expected}` in `{message}`"
        );
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_jsonrpc_fixture_reports_malformed_following_message_index() {
    let root = test_temp_root("jsonrpc-fixture-malformed-index");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"first"},{"jsonrpc":"2.0","method":}]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
        );
    })
    .expect_err("malformed following message should fail");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid JSON-RPC fixture `requests.json` message 1"),
        "expected message index in `{message}`"
    );
    assert!(
        message.contains("unexpected byte `}`"),
        "expected parse position detail in `{message}`"
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_jsonrpc_fixture_rejects_reserved_directive_shapes_and_paths() {
    let root = test_temp_root("jsonrpc-fixture-directive-failures");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    let cases = [
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"deep":[{"$case_text":1}]}}]"#,
            "message 0 at $[0].params.deep[0]: `$case_text` directive value must be a string",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"missing.txt","extra":null}}]"#,
            "`$case_text` directive object must contain no other members",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"../escape.txt"}}]"#,
            "must use portable relative components",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"missing.txt"}}]"#,
            "case file `missing.txt` must match fixture entry spelling exactly",
        ),
    ];
    for (fixture, expected) in cases {
        fs::write(case_dir.join("requests.json"), fixture)
            .expect("JSON-RPC fixture should be written");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
            );
        })
        .expect_err("invalid directive should fail");
        let message = panic_message(panic);
        assert!(
            message.contains(expected),
            "expected `{expected}` in `{message}`"
        );
    }
    fs::write(case_dir.join("invalid.raw"), [0xff]).expect("non-UTF-8 sidecar should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"invalid.raw"}}]"#,
    )
    .expect("JSON-RPC fixture should be written");
    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
        );
    })
    .expect_err("non-UTF-8 directive sidecar should fail");
    assert!(panic_message(panic).contains("failed to read case file `invalid.raw` as UTF-8"));
    fs::remove_dir_all(root).expect("case root should be removed");
}
