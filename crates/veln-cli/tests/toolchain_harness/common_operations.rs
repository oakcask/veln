use super::*;

#[test]
pub(super) fn json_and_result_value_operation_before_path_errors_keep_resolved_context() {
    let root = test_temp_root("json-result-operation-before-path-error");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, expected_fragments) in [
        (
            "check",
            "json_assert",
            "length = \"2\"\n",
            "path = \"items\"\n",
            vec![
                "json_assert",
                "0",
                "path `items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "check",
            "json_assert",
            "workspace_file_uri = 1\n",
            "path = \"uri\"\n",
            vec![
                "json_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "check",
            "json_assert",
            "workspace_file_uri = \"../main.veln\"\n",
            "path = \"uri\"\n",
            vec![
                "json_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "must not contain",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "length = \"2\"\n",
            "value_path = \"rendered\"\npath = \"items\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "workspace_file_uri = 1\n",
            "value_path = \"rendered\"\npath = \"uri\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "workspace_file_uri = \"../main.veln\"\n",
            "value_path = \"rendered\"\npath = \"uri\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "must not contain",
            ],
        ),
    ] {
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"
                ),
            )
        })
        .expect_err("invalid assertion operand should fail");
        let message = panic_message(panic);
        for expected in expected_fragments {
            assert!(
                message.contains(expected),
                "expected `{expected}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn lsp_and_mcp_length_and_workspace_uri_accept_operation_before_selector_path() {
    let root = test_temp_root("lsp-mcp-operation-before-selector");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, operation_fragment) in [
        (
            "lsp",
            "lsp_assert",
            "length = 2\n",
            "id = 1\npath = \"/result/items\"\n",
            "length",
        ),
        (
            "lsp",
            "lsp_assert",
            "workspace_file_uri = \"main.veln\"\n",
            "method = \"note\"\npath = \"/params/uri\"\n",
            "workspace_file_uri",
        ),
        (
            "mcp",
            "mcp_assert",
            "length = 2\n",
            "id = 1\npath = \"/result/items\"\n",
            "length",
        ),
        (
            "mcp",
            "mcp_assert",
            "workspace_file_uri = \"main.veln\"\n",
            "id = 1\npath = \"/result/uri\"\n",
            "workspace_file_uri",
        ),
    ] {
        let manifest = parse_manifest(
            &manifest_path,
            &format!("command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"),
        );
        let parsed_operation = match section {
            "lsp_assert" => match manifest.expectations.lsp_assertions[0]
                .operation
                .as_ref()
                .expect("operation should parse")
            {
                RpcAssertionOperation::Length(_) => "length",
                RpcAssertionOperation::WorkspaceFileUri(_) => "workspace_file_uri",
                operation => panic!("unexpected lsp operation: {operation:?}"),
            },
            "mcp_assert" => match manifest.expectations.mcp_assertions[0]
                .operation
                .as_ref()
                .expect("operation should parse")
            {
                RpcAssertionOperation::Length(_) => "length",
                RpcAssertionOperation::WorkspaceFileUri(_) => "workspace_file_uri",
                operation => panic!("unexpected mcp operation: {operation:?}"),
            },
            _ => unreachable!("test section should be covered"),
        };
        assert_eq!(parsed_operation, operation_fragment);
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn lsp_and_mcp_operation_before_selector_path_errors_keep_resolved_context() {
    let root = test_temp_root("lsp-mcp-operation-before-selector-error");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, expected_fragments) in [
        (
            "lsp",
            "lsp_assert",
            "length = \"2\"\n",
            "id = 1\npath = \"/result/items\"\n",
            vec![
                "lsp_assert",
                "0",
                "response id 1",
                "path `/result/items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "lsp",
            "lsp_assert",
            "workspace_file_uri = 1\n",
            "method = \"note\"\npath = \"/params/uri\"\n",
            vec![
                "lsp_assert",
                "0",
                "notification method \"note\" occurrence 0",
                "path `/params/uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "mcp",
            "mcp_assert",
            "length = \"2\"\n",
            "id = 1\npath = \"/result/items\"\n",
            vec![
                "mcp_assert",
                "0",
                "response id 1",
                "path `/result/items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "mcp",
            "mcp_assert",
            "workspace_file_uri = 1\n",
            "id = 1\npath = \"/result/uri\"\n",
            vec![
                "mcp_assert",
                "0",
                "response id 1",
                "path `/result/uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
    ] {
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"
                ),
            )
        })
        .expect_err("invalid assertion operand should fail");
        let message = panic_message(panic);
        for expected in expected_fragments {
            assert!(
                message.contains(expected),
                "expected `{expected}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

pub(super) fn common_json_operation_test_root() -> (PathBuf, CaseRunContext<'static>) {
    let root = test_temp_root("common-json-operation-failures");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let context = CaseRunContext {
        case_dir: Path::new("common-json-operation-failures"),
        run_number: 1,
    };
    (root, context)
}

#[test]
pub(super) fn common_length_and_workspace_uri_operations_report_failed_facts() {
    let (root, _) = common_json_operation_test_root();

    for (actual, operation, expected) in [
        (
            JsonValue::String("not-an-array".to_string()),
            ValueAssertionOperation::Length(1),
            "length requires a selected JSON array",
        ),
        (
            JsonValue::Array(vec![]),
            ValueAssertionOperation::Length(1),
            "array length mismatch: expected 1, got 0",
        ),
        (
            JsonValue::Number(1),
            ValueAssertionOperation::WorkspaceFileUri("main.veln".to_string()),
            "workspace_file_uri requires a selected JSON string",
        ),
        (
            JsonValue::String("file:///wrong".to_string()),
            ValueAssertionOperation::WorkspaceFileUri("main.veln".to_string()),
            "workspace URI mismatch",
        ),
    ] {
        let error = expect_value_assertion(&actual, &operation, &root)
            .expect_err("common operation should report its failed fact");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn common_length_failure_keeps_json_section_context() {
    let (root, context) = common_json_operation_test_root();

    let json_manifest = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "missing"
length = 0
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_json_path_in_workspace(
            &context,
            &JsonValue::Object(vec![]),
            &json_manifest.expectations.json_assertions[0],
            0,
            &root,
        )
    })
    .expect_err("missing JSON path should fail");
    let message = panic_message(panic);
    assert!(message.contains("json_assert 0"));
    assert!(message.contains("path `missing`"));
    assert!(message.contains("was not found"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn common_length_failure_keeps_result_value_section_context() {
    let (root, context) = common_json_operation_test_root();

    let result_manifest = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value.missing"
length = 0
"#,
    );
    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path_in_workspace(
            &context,
            &rendered,
            &result_manifest.expectations.result_value_assertions[0],
            0,
            &root,
        )
    })
    .expect_err("missing result-value path should fail");
    let message = panic_message(panic);
    assert!(message.contains("result_value_assert 0"));
    assert!(message.contains("path `value.missing`"));
    assert!(message.contains("was not found"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn unsafe_workspace_uri_operands_keep_adapter_section_context() {
    let (root, _) = common_json_operation_test_root();

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"uri\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value.id\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result/uri\"\n"),
    ] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &root.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}workspace_file_uri = \"../main.veln\"\n"
                ),
            )
        })
        .expect_err("unsafe workspace URI operand should fail");
        let message = panic_message(error);
        assert!(message.contains("workspace_file_uri"));
        assert!(message.contains("must not contain"));
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn common_length_parser_accepts_full_usize_range_without_signed_intermediary() {
    let root = test_temp_root("common-length-usize-range");
    let manifest_path = root.join("case.toml");
    let max = usize::MAX.to_string();
    let over_max = ((usize::MAX as u128) + 1).to_string();

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"items\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"items\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result/items\"\n"),
        ("mcp", "mcp_assert", "id = 1\npath = \"/result/items\"\n"),
    ] {
        for accepted in ["9223372036854775807", max.as_str()] {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = {accepted}\n"
                ),
            );
        }
        if usize::BITS > 63 {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = 9223372036854775808\n"
                ),
            );
        }

        for (rejected, expected) in [
            ("-1", "expected non-negative integer"),
            ("1.0", "expected integer"),
            (
                over_max.as_str(),
                "expected non-negative integer within range",
            ),
        ] {
            let panic = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = {rejected}\n"
                    ),
                )
            })
            .expect_err("invalid length operand should fail");
            let message = panic_message(panic);
            for fragment in [section, "0", "length", expected] {
                assert!(
                    message.contains(fragment),
                    "expected `{fragment}` in `{message}`"
                );
            }
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

pub(super) fn common_operation_wrapper_test_root() -> (PathBuf, CaseRunContext<'static>) {
    let root = test_temp_root("common-operation-wrapper-context");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let context = CaseRunContext {
        case_dir: Path::new("common-operation-wrapper-context"),
        run_number: 1,
    };
    (root, context)
}

#[test]
pub(super) fn json_common_operation_failures_keep_full_adapter_context() {
    let (root, context) = common_operation_wrapper_test_root();
    let wrong_uri = "file:///wrong";

    for (operation, actual, fragments) in [
        (
            "length = 1",
            r#"{"items":"not-array"}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `items`",
                "length requires a selected JSON array",
            ],
        ),
        (
            "length = 2",
            r#"{"items":[1]}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `items`",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            r#"{"uri":1}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `uri`",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            format!(r#"{{"uri":"{wrong_uri}"}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `uri`",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let manifest = parse_manifest(
            &root.join("case.toml"),
            &format!(
                "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"{}\"\n{operation}\n",
                if operation.starts_with("length") {
                    "items"
                } else {
                    "uri"
                }
            ),
        );
        let json = parse_json(&actual).expect("JSON wrapper matrix input should parse");
        let panic = std::panic::catch_unwind(|| {
            assert_json_path_in_workspace(
                &context,
                &json,
                &manifest.expectations.json_assertions[0],
                0,
                &root,
            )
        })
        .expect_err("JSON assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn result_value_common_operation_failures_keep_full_adapter_context() {
    let (root, context) = common_operation_wrapper_test_root();

    for (operation, rendered_value, selected_path, fragments) in [
        (
            "length = 1",
            "RuntimeDiagnostic(id, msg, RuntimeValueDiagnostic(Nil, reason))",
            "value.id",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.id`",
                "length requires a selected JSON array",
            ],
        ),
        (
            "length = 2",
            "RuntimeDiagnostic(id, msg, RuntimeByteDiagnostic(ByteOffset(1), Cons(RuntimeDiagnosticFieldPathSegment(schema, body), Nil), RuntimeByteCountFacts(ByteCount(2), ByteCount(1), partial), NoRuntimeBytePreview))",
            "value.detail.field_path",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.detail.field_path`",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            "ByteOffset(2)",
            "value",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value`",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            "RuntimeDiagnostic(id, msg, RuntimeValueDiagnostic(Nil, reason))",
            "value.id",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.id`",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let manifest = parse_manifest(
            &root.join("case.toml"),
            &format!(
                "command = [\"run\", \"--json\", \"main\", \"main.veln\"]\nexit = 0\n[[result_value_assert]]\nvalue_path = \"rendered\"\npath = \"{selected_path}\"\n{operation}\n"
            ),
        );
        let rendered = JsonValue::Object(vec![(
            "rendered".to_string(),
            JsonValue::String(rendered_value.to_string()),
        )]);
        let panic = std::panic::catch_unwind(|| {
            assert_result_value_path_in_workspace(
                &context,
                &rendered,
                &manifest.expectations.result_value_assertions[0],
                0,
                &root,
            )
        })
        .expect_err("result-value assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}
