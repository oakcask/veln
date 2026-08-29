use super::*;

#[test]
pub(super) fn decoded_lsp_transport_failure_matrix_rejects_invalid_complete_streams() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
    let valid = lsp_frame(response);
    for (stdout, expected) in [
        ("garbage".to_string(), "malformed or partial framing"),
        (
            "Content-Length: 10\r\n\r\n{}".to_string(),
            "partial frame body",
        ),
        (format!("{valid}garbage"), "trailing bytes"),
        (lsp_frame("{"), "invalid JSON"),
        (
            format!("{valid}{}", lsp_frame(response)),
            "duplicate response identifier 1",
        ),
    ] {
        let error = decode_lsp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
pub(super) fn decoded_lsp_transport_preserves_header_and_body_failure_boundaries() {
    for (stdout, expected) in [
        (
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}".to_string(),
            "missing Content-Length header",
        ),
        (
            "Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_string(),
            "duplicate Content-Length header",
        ),
        (
            "Content-Length: many\r\n\r\n{}".to_string(),
            "invalid Content-Length `many`",
        ),
        (lsp_frame("[]"), "is not a JSON-RPC object"),
        (
            "Content-Length: 1\r\n\r\né".to_string(),
            "frame body at byte offset 21 is not UTF-8",
        ),
    ] {
        let error = decode_lsp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
pub(super) fn decoded_lsp_duplicate_ids_are_rejected_only_for_responses() {
    let requests = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"method":"first"}"#),
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"method":"second"}"#),
    );
    assert_eq!(
        decode_lsp_stdout(&requests)
            .expect("request identifiers may repeat in decoded server output")
            .len(),
        2
    );
}

#[test]
pub(super) fn decoded_lsp_array_pointer_boundary_matrix_distinguishes_missing_and_invalid() {
    let value = parse_json(r#"["first","last"]"#).expect("array should parse");
    for (token, expected) in [("0", "first"), ("1", "last")] {
        match json_pointer(&value, &[token.to_string()]) {
            JsonPointerResult::Found(JsonValue::String(actual)) => assert_eq!(actual, expected),
            _ => panic!("array token {token:?} should resolve"),
        }
    }
    assert!(matches!(
        json_pointer(&value, &["2".to_string()]),
        JsonPointerResult::Missing
    ));
    for token in [
        "184467440737095516160",
        "01",
        "-1",
        "+1",
        " 1",
        "١",
        "-",
        "",
    ] {
        assert!(
            matches!(
                json_pointer(&value, &[token.to_string()]),
                JsonPointerResult::Invalid(_)
            ),
            "array token {token:?} should be invalid"
        );
    }
}

#[test]
pub(super) fn decoded_lsp_operations_cover_string_kinds_missing_paths_and_selectors() {
    let stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":2}}"#);
    let passing = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "beta"
[[lsp_assert]]
id = 1
path = "/result/absent"
missing = true
"#,
    );
    let messages = decode_lsp_stdout(&stdout).expect("stream should decode");
    for assertion in &passing {
        evaluate_lsp_assertion(&messages, assertion).expect("assertion should pass");
    }

    for (fields, expected) in [
        (
            "id = 1\npath = \"/result/number\"\ncontains = \"2\"",
            "requires a selected JSON string",
        ),
        (
            "id = 1\npath = \"/result/text/value\"\nmissing = true",
            "invalid traversal",
        ),
        (
            "id = 1\npath = \"/result/text\"\nmissing = true",
            "exists but should be missing",
        ),
        (
            "id = 9\npath = \"/result\"\nmissing = true",
            "selected response was not found",
        ),
        (
            "method = \"absent\"\npath = \"/params\"\nmissing = true",
            "selected notification was not found",
        ),
    ] {
        let assertion = parsed_lsp_assertions(&format!(
            "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{fields}\n"
        ))
        .remove(0);
        let error = evaluate_lsp_assertion(&messages, &assertion)
            .expect_err("assertion should report its failure");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
pub(super) fn decoded_lsp_equals_file_uses_an_immutable_string_operand() {
    let root = test_temp_root("lsp-equals-file");
    let manifest_path = root.join("case.toml");
    fs::write(root.join("expected.txt"), "alpha\r\nbeta\n")
        .expect("expected text should be written");
    let manifest = parse_manifest(
        &manifest_path,
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result/text\"\nequals_file = \"expected.txt\"\n",
    );
    fs::write(root.join("expected.txt"), "changed")
        .expect("expected text should be changed after loading");
    let messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha\r\nbeta\n"}}"#,
    ))
    .expect("stream should decode");
    evaluate_lsp_assertion(&messages, &manifest.expectations.lsp_assertions[0])
        .expect("snapshot should retain exact original text");

    let wrong_kind = parse_manifest(
        &manifest_path,
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result\"\nequals_file = \"expected.txt\"\n",
    );
    let error = evaluate_lsp_assertion(&messages, &wrong_kind.expectations.lsp_assertions[0])
        .expect_err("equals_file should reject a non-string value");
    assert!(error.contains("requires a selected JSON string"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn raw_stdout_and_decoded_lsp_failures_are_reported_independently() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[stdout]
contains = ["raw marker"]
[[lsp_assert]]
id = 1
path = "/result"
equals = "expected"
"#,
    );
    let context = CaseRunContext {
        case_dir: Path::new("independence"),
        run_number: 1,
    };
    let output = CapturedOutput {
        exit: Some(0),
        stdout: lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":"actual"}"#),
        stderr: String::new(),
    };
    let root = test_temp_root("lsp-independent-assertions");
    let panic = std::panic::catch_unwind(|| {
        manifest
            .expectations
            .assert_matches(&context, &output, &root)
    })
    .expect_err("both assertions should fail");
    let message = panic_message(panic);
    assert!(message.contains("raw marker"));
    assert!(message.contains("value mismatch"));

    let transport_output = CapturedOutput {
        exit: Some(0),
        stdout: "trailing transport bytes".to_string(),
        stderr: String::new(),
    };
    let panic = std::panic::catch_unwind(|| {
        manifest
            .expectations
            .assert_matches(&context, &transport_output, &root)
    })
    .expect_err("raw and transport assertions should fail");
    let message = panic_message(panic);
    assert!(message.contains("raw marker"));
    assert!(message.contains("malformed or partial framing"));
    assert!(!message.contains("value mismatch"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn repeated_run_failures_are_grouped_by_run_and_manifest_assertion_order() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[stdout]
contains = ["required raw marker"]
[stderr]
format = "empty"
[[lsp_assert]]
id = 1
path = "/result/value"
equals = "first expected"
[[lsp_assert]]
id = 1
path = "/result/other"
equals = "second expected"
"#,
    );
    let outputs = [
        CapturedOutput {
            exit: Some(0),
            stdout: lsp_frame(
                r#"{"jsonrpc":"2.0","id":1,"result":{"value":"run one","other":"one"}}"#,
            ),
            stderr: "run one stderr".to_string(),
        },
        CapturedOutput {
            exit: Some(7),
            stdout: format!(
                "{}trailing",
                lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"value":"run two"}}"#)
            ),
            stderr: "run two stderr".to_string(),
        },
    ];
    let mut failures = Vec::new();
    let root = test_temp_root("lsp-repeated-failures");
    for (index, output) in outputs.iter().enumerate() {
        let context = CaseRunContext {
            case_dir: Path::new("repeat-lsp"),
            run_number: index + 1,
        };
        collect_panic_failure(&mut failures, || {
            manifest
                .expectations
                .assert_matches(&context, output, &root)
        });
    }
    assert_eq!(failures.len(), 2);
    assert!(failures[0].contains("repeat-lsp run 1"));
    assert_fragments_in_order(&failures[0], &["/result/value", "/result/other"]);
    assert!(failures[1].contains("repeat-lsp run 2"));
    assert!(failures[1].contains("expected exit 0, got Some(7)"));
    assert!(failures[1].contains("expected stdout to contain `required raw marker`"));
    assert!(failures[1].contains("expected stderr to be empty"));
    assert!(failures[1].contains("trailing bytes"));
    assert_fragments_in_order(
        &failures[1],
        &[
            "expected exit 0",
            "expected stdout to contain",
            "expected stderr to be empty",
            "trailing bytes",
        ],
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

pub(super) fn assert_fragments_in_order(text: &str, fragments: &[&str]) {
    let mut remainder = text;
    for fragment in fragments {
        let position = remainder
            .find(fragment)
            .unwrap_or_else(|| panic!("expected `{fragment}` after prior fragments in:\n{text}"));
        remainder = &remainder[position + fragment.len()..];
    }
}

pub(super) fn assert_manifest_parse_error(source: &str, expected: &str) {
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("incomplete manifest section should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
}

pub(super) fn assert_manifest_parse_error_without(source: &str, expected: &str, forbidden: &str) {
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("manifest should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
    assert!(
        !message.contains(forbidden),
        "expected panic to avoid `{forbidden}`, got `{message}`"
    );
}
