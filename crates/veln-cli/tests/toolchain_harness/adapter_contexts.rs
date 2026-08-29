use super::*;

#[test]
pub(super) fn lsp_common_operation_failures_keep_full_adapter_context() {
    let (root, context) = common_operation_wrapper_test_root();
    let wrong_uri = "file:///wrong";

    for (manifest_fields, lsp_stdout, fragments) in [
        (
            "id = 1\npath = \"/result/items\"\nlength = 1\n",
            lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":"not-array"}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "response id 1",
                "path \"/result/items\"",
                "length requires a selected JSON array",
            ],
        ),
        (
            "id = 1\npath = \"/result/items\"\nlength = 2\n",
            lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":[1]}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "response id 1",
                "path \"/result/items\"",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "method = \"textDocument/publishDiagnostics\"\npath = \"/params/uri\"\nworkspace_file_uri = \"main.veln\"\n",
            lsp_frame(
                r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":1,"diagnostics":[]}}"#,
            ),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "notification method \"textDocument/publishDiagnostics\" occurrence 0",
                "path \"/params/uri\"",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "method = \"textDocument/publishDiagnostics\"\npath = \"/params/uri\"\nworkspace_file_uri = \"main.veln\"\n",
            lsp_frame(&format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":"{wrong_uri}","diagnostics":[]}}}}"#
            )),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "notification method \"textDocument/publishDiagnostics\" occurrence 0",
                "path \"/params/uri\"",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let lsp_failures = parse_manifest(
            &root.join("case.toml"),
            &format!("command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{manifest_fields}"),
        )
        .expectations
        .lsp_assertions;
        let panic = std::panic::catch_unwind(|| {
            assert_lsp_assertions_in_workspace(&context, &lsp_stdout, &lsp_failures, &root)
        })
        .expect_err("LSP assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    let lsp_failures = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/items"
length = 1
[[lsp_assert]]
id = 2
path = "/result/items"
length = 2
"#,
    )
    .expectations
    .lsp_assertions;
    let lsp_stdout = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":"not-array"}}"#),
        lsp_frame(r#"{"jsonrpc":"2.0","id":2,"result":{"items":[1]}}"#)
    );
    let panic = std::panic::catch_unwind(|| {
        assert_lsp_assertions_in_workspace(&context, &lsp_stdout, &lsp_failures, &root)
    })
    .expect_err("LSP assertions should aggregate failures");
    let message = panic_message(panic);
    for fragment in [
        "lsp_assert 0",
        "length requires a selected JSON array",
        "lsp_assert 1",
        "array length mismatch: expected 2, got 1",
    ] {
        assert!(
            message.contains(fragment),
            "expected `{fragment}` in `{message}`"
        );
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn mcp_common_operation_failures_keep_full_adapter_context() {
    let root = test_temp_root("mcp-common-operation-wrapper-context");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let context = CaseRunContext {
        case_dir: Path::new("mcp-common-operation-wrapper-context"),
        run_number: 1,
    };
    let assertions = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/items"
length = 2
[[mcp_assert]]
id = 2
path = "/result/uri"
workspace_file_uri = "main.veln"
"#,
    )
    .expectations
    .mcp_assertions;
    let stdout = r#"{"jsonrpc":"2.0","id":1,"result":{"items":[1]}}
{"jsonrpc":"2.0","id":2,"result":{"uri":"file:///wrong"}}
"#;

    let panic =
        std::panic::catch_unwind(|| assert_mcp_assertions(&context, stdout, &assertions, &root))
            .expect_err("MCP common operation assertions should fail");
    let message = panic_message(panic);
    for fragment in [
        "mcp-common-operation-wrapper-context run 1",
        "mcp_assert 0 response id 1 path \"/result/items\"",
        "array length mismatch: expected 2, got 1",
        "mcp_assert 1 response id 2 path \"/result/uri\"",
        "workspace URI mismatch",
        "main.veln",
    ] {
        assert!(
            message.contains(fragment),
            "expected `{fragment}` in `{message}`"
        );
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

pub(super) fn contains_adapter_context() -> CaseRunContext<'static> {
    CaseRunContext {
        case_dir: Path::new("contains-adapters"),
        run_number: 1,
    }
}

#[test]
pub(super) fn json_contains_evaluation_covers_success_and_failure_classes() {
    let context = contains_adapter_context();
    let json =
        parse_json(r#"{"text":"alpha beta","number":1}"#).expect("JSON adapter input should parse");
    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "text"
contains = "ha be"
[[json_assert]]
path = "text"
contains = "missing"
[[json_assert]]
path = "number"
contains = "1"
[[json_assert]]
path = "absent"
contains = "x"
"#,
    );
    assert_json_path(
        &context,
        &json,
        &json_manifest.expectations.json_assertions[0],
    );
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "was not found"),
    ] {
        let panic = std::panic::catch_unwind(|| {
            assert_json_path(
                &context,
                &json,
                &json_manifest.expectations.json_assertions[index],
            )
        })
        .expect_err("JSON contains assertion should fail");
        assert!(panic_message(panic).contains(expected));
    }
}

#[test]
pub(super) fn result_value_contains_evaluation_covers_success_and_failure_classes() {
    let context = contains_adapter_context();
    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "rr"
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "missing"
[[result_value_assert]]
value_path = "rendered"
path = "value"
contains = "2"
[[result_value_assert]]
value_path = "rendered"
path = "absent"
contains = "x"
"#,
    );
    assert_result_value_path(
        &context,
        &rendered,
        &result_manifest.expectations.result_value_assertions[0],
    );
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "was not found"),
    ] {
        let panic = std::panic::catch_unwind(|| {
            assert_result_value_path(
                &context,
                &rendered,
                &result_manifest.expectations.result_value_assertions[index],
            )
        })
        .expect_err("result-value contains assertion should fail");
        assert!(panic_message(panic).contains(expected));
    }
}

#[test]
pub(super) fn lsp_contains_evaluation_covers_success_and_failure_classes() {
    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":1}}"#,
    ))
    .expect("LSP adapter input should decode");
    let lsp_assertions = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "ha be"
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "missing"
[[lsp_assert]]
id = 1
path = "/result/number"
contains = "1"
[[lsp_assert]]
id = 1
path = "/result/absent"
contains = "x"
"#,
    );
    evaluate_lsp_assertion(&lsp_messages, &lsp_assertions[0])
        .expect("LSP contains assertion should pass");
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "selected JSON path was not found"),
    ] {
        let error = evaluate_lsp_assertion(&lsp_messages, &lsp_assertions[index])
            .expect_err("LSP contains assertion should fail");
        assert!(error.contains(expected));
    }
}

#[test]
pub(super) fn mcp_contains_evaluation_covers_success_and_failure_classes() {
    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":1}}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertions = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "ha be"
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "missing"
[[mcp_assert]]
id = 1
path = "/result/number"
contains = "1"
[[mcp_assert]]
id = 1
path = "/result/absent"
contains = "x"
"#,
    );
    evaluate_mcp_assertion(&mcp_messages, &mcp_assertions[0], Path::new("."))
        .expect("MCP contains assertion should pass");
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "selected JSON path was not found"),
    ] {
        let error = evaluate_mcp_assertion(&mcp_messages, &mcp_assertions[index], Path::new("."))
            .expect_err("MCP contains assertion should fail");
        assert!(error.contains(expected));
    }
}

#[test]
pub(super) fn json_contains_failures_retain_wrapper_context() {
    let context = contains_adapter_context();

    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check", "--json", "main.veln"]
exit = 0
[[json_assert]]
path = "text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        json_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"text":"alpha beta"}"#.to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("JSON contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("JSON path `text` mismatch"));
    assert!(message.contains("string does not contain \"missing\""));
}

#[test]
pub(super) fn result_value_contains_failures_retain_wrapper_context() {
    let context = contains_adapter_context();

    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        result_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"rendered":"ByteOffset(2)"}"#.to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("result-value contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("result value path `constructor` mismatch"));
    assert!(message.contains("string does not contain \"missing\""));
}

#[test]
pub(super) fn lsp_contains_failures_retain_wrapper_context() {
    let context = contains_adapter_context();

    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        lsp_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta"}}"#),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("LSP contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("response id 1 path \"/result/text\""));
    assert!(message.contains("string does not contain \"missing\""));
}

#[test]
pub(super) fn mcp_contains_failures_retain_wrapper_context() {
    let context = contains_adapter_context();

    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        mcp_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta"}}
"#
                .to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("MCP contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("response id 1 path \"/result/text\""));
    assert!(message.contains("string does not contain \"missing\""));
}

#[test]
pub(super) fn manifest_lsp_assertions_validate_selector_operation_and_pointer_contracts() {
    for (fields, expected) in [
        (
            "path = \"\"\nequals = null\n",
            "exactly one of `id` or `method`",
        ),
        (
            "id = 1\nmethod = \"note\"\npath = \"\"\nequals = null\n",
            "exactly one of `id` or `method`",
        ),
        (
            "id = 1\noccurrence = 0\npath = \"\"\nequals = null\n",
            "occurrence` is valid only with `method`",
        ),
        ("id = 1\nequals = null\n", "is missing `path`"),
        ("id = 1\npath = \"\"\n", "needs exactly one"),
        (
            "id = 1\npath = \"\"\nequals = null\ncontains = \"x\"\n",
            "needs exactly one",
        ),
        (
            "id = 1\npath = \"\"\nmissing = false\n",
            "missing` must be true",
        ),
    ] {
        assert_manifest_parse_error(
            &format!("command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{fields}"),
            expected,
        );
    }

    for pointer in ["value", "#fragment", "/~", "/~2", "/a~x"] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = {pointer:?}\nequals = null\n"
            ),
            "JSON Pointer",
        );
    }
}

#[test]
pub(super) fn manifest_mcp_assertions_validate_selector_operation_and_pointer_contracts() {
    for (fields, expected) in [
        ("path = \"\"\nequals = null\n", "is missing `id`"),
        (
            "id = null\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = []\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = 1.0\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = 1e0\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        ("id = 1\nequals = null\n", "is missing `path`"),
        ("id = 1\npath = \"\"\n", "needs exactly one"),
        (
            "id = 1\npath = \"\"\nequals = null\nlength = 0\n",
            "needs exactly one",
        ),
        (
            "id = 1\npath = \"\"\nmissing = false\n",
            "missing` must be true",
        ),
    ] {
        assert_manifest_parse_error(
            &format!("command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\n{fields}"),
            expected,
        );
    }

    for pointer in ["value", "#fragment", "/~", "/~2", "/a~x"] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = {pointer:?}\nequals = null\n"
            ),
            "JSON Pointer",
        );
    }
}

#[test]
pub(super) fn manifest_mcp_assertions_validate_portable_workspace_uri_contracts() {
    let root = test_temp_root("mcp-portable-uri-manifest");
    let manifest_path = root.join("case.toml");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    parse_manifest(
        &manifest_path,
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"main.veln\"\n",
    );
    parse_manifest(
        &manifest_path,
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 9223372036854775808\npath = \"/result/uri\"\nworkspace_file_uri = \"main.veln\"\n",
    );
    for relative in [
        "",
        "/abs.veln",
        "nested/../main.veln",
        "./main.veln",
        "nested//main.veln",
        "nested/./main.veln",
        "bad\\path",
        "missing.veln",
    ] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = {relative:?}\n"
                ),
            );
        })
        .expect_err("invalid workspace_file_uri should fail");
        assert!(
            panic_message(error).contains("workspace_file_uri"),
            "unexpected error for {relative:?}"
        );
    }
    fs::create_dir(root.join("directory.veln")).expect("directory should be created");
    let error = std::panic::catch_unwind(|| {
        parse_manifest(
            &manifest_path,
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"directory.veln\"\n",
        );
    })
    .expect_err("directory workspace_file_uri should fail");
    assert!(panic_message(error).contains("existing regular file"));

    fs::remove_dir_all(root).expect("test root should be removed");
}
