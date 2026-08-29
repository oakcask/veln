use super::*;

fn sample_inventory() -> Inventory {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\", \"main.veln\"]\nstdin = \"a\\nb\"\nexit = 0\n[env]\nB = \"2\"\nA = \"1\"\n[[json_assert]]\npath = \"value\"\nequals = {\"b\": [1, 2], \"a\": true}\n[[json_assert]]\npath = \"other\"\nmissing = true\n[stdout]\ncontains = [\"first\", \"second\"]\n",
    );
    Inventory {
        schema: SCHEMA.to_string(),
        roots: ROOTS.iter().map(|(root, _)| (*root).to_string()).collect(),
        source_git_tree: "sample".to_string(),
        cases: BTreeMap::from([(
            "tests/toolchain_cases/sample".to_string(),
            describe(&manifest),
        )]),
    }
}

#[test]
fn semantic_export_is_deterministic_and_round_trips() {
    let inventory = sample_inventory();
    let first = inventory.render();
    let second = inventory.render();
    assert_eq!(first, second);
    assert_eq!(Inventory::parse(&first).unwrap(), inventory);
}

#[test]
fn semantic_export_records_structured_jsonrpc_source_and_framed_stdin() {
    let root = test_temp_root("semantic-jsonrpc-input");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","id":1,"method":"shutdown"}]"#,
    )
    .expect("JSON-RPC fixture should be written");
    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let fields = describe(&manifest);
    assert_eq!(
        fields["invocation.stdin_jsonrpc_file"],
        json_string("requests.json")
    );
    let body = r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#;
    assert_eq!(
        fields["invocation.stdin"],
        json_string(&format!("Content-Length: {}\r\n\r\n{body}", body.len()))
    );
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn semantic_export_normalizes_object_order_and_hashes_large_exact_text() {
    let left = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nstdin = {:?}\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"b\": 2, \"a\": 1}}\n",
            "x".repeat(LARGE_TEXT_BYTES)
        ),
    );
    let right = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nstdin = {:?}\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"a\": 1, \"b\": 2}}\n",
            "x".repeat(LARGE_TEXT_BYTES)
        ),
    );
    let left = describe(&left);
    let right = describe(&right);
    assert_eq!(left, right);
    assert_eq!(
        left["invocation.stdin"],
        format!(
            "{{\"logical_field\":\"invocation.stdin\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{}}}",
            json_string(&sha256("x".repeat(LARGE_TEXT_BYTES).as_bytes()))
        )
    );
}

#[test]
fn semantic_export_hashes_large_typed_json_strings_with_logical_fields() {
    let large = "x".repeat(LARGE_TEXT_BYTES);
    let manifest = parse_manifest(
        Path::new("case.toml"),
        &format!(
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"value\"\nequals = {{\"outer\": [{{\"inner/key\": {:?}}}]}}\n[[json_assert]]\npath = \"top\"\nequals = {:?}\n",
            large, large
        ),
    );
    let fields = describe(&manifest);
    let digest = json_string(&sha256(large.as_bytes()));
    assert_eq!(
        fields["expectations.json_assertions[0].equals"],
        format!(
            "{{\"outer\":[{{\"inner/key\":{{\"logical_field\":\"expectations.json_assertions[0].equals/outer/0/inner~1key\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{digest}}}}}]}}"
        )
    );
    assert_eq!(
        fields["expectations.json_assertions[1].equals"],
        format!(
            "{{\"logical_field\":\"expectations.json_assertions[1].equals\",\"byte_length\":{LARGE_TEXT_BYTES},\"sha256\":{digest}}}"
        )
    );
}

#[test]
fn semantic_export_records_common_json_assertion_equality_boundaries() {
    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals = {"b": [1.0, 1e0], "a": 1}
[[result_value_assert]]
value_path = "value"
path = "value"
equals = {"b": [1.0, 1e0], "a": 1}
[[lsp_assert]]
id = 1
path = "/result"
equals = {"b": [1.0, 1e0], "a": 1}
"#,
    );
    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = {"b": [1.0, 1e0], "a": 1}
"#,
    );
    let lsp_fields = describe(&lsp_manifest);
    let mcp_fields = describe(&mcp_manifest);
    let expected = r#"{"a":1,"b":[1.0,1e0]}"#;
    assert_eq!(
        lsp_fields["expectations.json_assertions[0].equals"],
        expected
    );
    assert_eq!(
        lsp_fields["expectations.result_value_assertions[0].equals"],
        expected
    );
    assert_eq!(
        lsp_fields["expectations.lsp_assertions[0].equals"],
        expected
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[0].equals"],
        expected
    );
    assert_eq!(
        lsp_fields["expectations.json_assertions[0].operation"],
        json_string("equals")
    );
    assert_eq!(
        lsp_fields["expectations.result_value_assertions[0].operation"],
        json_string("equals")
    );
    assert_eq!(
        lsp_fields["expectations.lsp_assertions[0].operation"],
        json_string("equals")
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[0].operation"],
        json_string("equals")
    );
}

#[test]
fn semantic_export_records_contains_operands_for_every_assertion_adapter() {
    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
contains = "json needle"
[[result_value_assert]]
value_path = "rendered"
path = "value"
contains = "result needle"
[[lsp_assert]]
id = 1
path = "/result"
contains = "lsp needle"
"#,
    );
    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
contains = "mcp needle"
"#,
    );
    let lsp_fields = describe(&lsp_manifest);
    let mcp_fields = describe(&mcp_manifest);

    for (fields, base, operand) in [
        (
            &lsp_fields,
            "expectations.json_assertions[0]",
            "json needle",
        ),
        (
            &lsp_fields,
            "expectations.result_value_assertions[0]",
            "result needle",
        ),
        (&lsp_fields, "expectations.lsp_assertions[0]", "lsp needle"),
        (&mcp_fields, "expectations.mcp_assertions[0]", "mcp needle"),
    ] {
        assert_eq!(
            fields[&format!("{base}.operation")],
            json_string("contains")
        );
        assert_eq!(fields[&format!("{base}.contains")], json_string(operand));
    }
}

#[test]
fn semantic_export_distinguishes_equals_json_file_operands() {
    let root = test_temp_root("semantic-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("expected.json"), r#"{"b":1,"a":-0}"#)
        .expect("expected JSON sidecar should be written");
    fs::write(text_dir.join("expected.txt"), "expected text\n")
        .expect("expected text sidecar should be written");
    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals_json_file = "case-text/expected.json"
[[result_value_assert]]
value_path = "value"
path = "value"
equals_json_file = "case-text/expected.json"
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
    let fields = describe(&manifest);
    let mcp_fields = describe(&mcp_manifest);
    let expected = r#"{"a":-0,"b":1}"#;
    assert_eq!(
        fields["expectations.json_assertions[0].operation"],
        json_string("equals_json_file")
    );
    assert_eq!(
        fields["expectations.json_assertions[0].equals_json_file"],
        expected
    );
    assert_eq!(
        fields["expectations.result_value_assertions[0].operation"],
        json_string("equals_json_file")
    );
    assert_eq!(
        fields["expectations.result_value_assertions[0].equals_json_file"],
        expected
    );
    assert_eq!(
        fields["expectations.lsp_assertions[0].operation"],
        json_string("equals_json_file")
    );
    assert_eq!(
        fields["expectations.lsp_assertions[0].equals_json_file"],
        expected
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[0].operation"],
        json_string("equals_file")
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[0].equals_file"],
        json_string("expected text\n")
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[1].operation"],
        json_string("equals_json_file")
    );
    assert_eq!(
        mcp_fields["expectations.mcp_assertions[1].equals_json_file"],
        expected
    );
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn semantic_export_preserves_value_assertion_workspace_uri_operands() {
    let root = test_temp_root("semantic-value-workspace-uri");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");

    let json_manifest = parse_manifest(
        &root.join("json-case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "uri"
workspace_file_uri = "main.veln"
"#,
    );
    let json_fields = describe(&json_manifest);
    assert_eq!(
        json_fields["expectations.json_assertions[0].operation"],
        json_string("workspace_file_uri")
    );
    assert_eq!(
        json_fields["expectations.json_assertions[0].workspace_file_uri"],
        json_string("main.veln")
    );
    assert!(
        !json_fields["expectations.json_assertions[0].workspace_file_uri"].contains("file://")
    );

    let result_manifest = parse_manifest(
        &root.join("result-case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value.uri"
workspace_file_uri = "main.veln"
"#,
    );
    let result_fields = describe(&result_manifest);
    assert_eq!(
        result_fields["expectations.result_value_assertions[0].operation"],
        json_string("workspace_file_uri")
    );
    assert_eq!(
        result_fields["expectations.result_value_assertions[0].workspace_file_uri"],
        json_string("main.veln")
    );
    assert!(
        !result_fields["expectations.result_value_assertions[0].workspace_file_uri"]
            .contains("file://")
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn checked_in_semantic_baseline_matches_authoritative_cases() {
    let expected =
        Inventory::parse(BASELINE).expect("checked-in semantic baseline should be valid");
    let actual = Inventory::current(&expected.source_git_tree);
    compare(&expected, &actual).unwrap_or_else(|difference| panic!("toolchain case semantic baseline changed:\n{difference}\nGenerate a candidate only for a deliberate contract review."));
}

#[test]
fn comparator_rejects_case_membership_changes() {
    let expected = sample_inventory();
    let mut actual = expected.clone();
    actual
        .cases
        .insert("examples/specification/added".to_string(), BTreeMap::new());
    let error = compare(&expected, &actual).unwrap_err();
    assert!(error.contains("case set changed"));
    assert!(error.contains("examples/specification/added"));
}

#[test]
fn comparator_rejects_invocation_and_order_changes_with_field_context() {
    let expected = sample_inventory();
    for field in ["invocation.command[0]", "invocation.env[0].name"] {
        let mut actual = expected.clone();
        actual
            .cases
            .values_mut()
            .next()
            .unwrap()
            .insert(field.to_string(), json_string("changed"));
        let error = compare(&expected, &actual).unwrap_err();
        assert!(error.contains(field), "{error}");
    }

    let mut reordered = expected.clone();
    let fields = reordered.cases.values_mut().next().unwrap();
    fields.insert(
        "expectations.stdout.contains[0]".to_string(),
        json_string("second"),
    );
    fields.insert(
        "expectations.stdout.contains[1]".to_string(),
        json_string("first"),
    );
    let error = compare(&expected, &reordered).unwrap_err();
    assert!(error.contains("expectations.stdout.contains[0]"), "{error}");
}

#[test]
fn comparator_rejects_assertion_operation_typed_value_and_exact_bytes() {
    let expected = sample_inventory();
    for (field, value) in [
        (
            "expectations.json_assertions[0].operation",
            json_string("missing"),
        ),
        (
            "expectations.json_assertions[0].equals",
            "{\"a\":true,\"b\":[1,\"2\"]}".to_string(),
        ),
        ("invocation.stdin", json_string("a\r\nb")),
    ] {
        let mut actual = expected.clone();
        actual
            .cases
            .values_mut()
            .next()
            .unwrap()
            .insert(field.to_string(), value);
        let error = compare(&expected, &actual).unwrap_err();
        assert!(error.contains(field), "{error}");
    }
}

#[test]
#[ignore = "writes a deliberate baseline candidate"]
fn generate_toolchain_semantic_baseline_candidate() {
    let destination = std::env::var_os("VELN_TOOLCHAIN_BASELINE_CANDIDATE")
        .expect("set VELN_TOOLCHAIN_BASELINE_CANDIDATE to a candidate output path");
    let source_git_tree = std::env::var("VELN_TOOLCHAIN_SOURCE_GIT_TREE")
        .expect("set VELN_TOOLCHAIN_SOURCE_GIT_TREE to the reviewed source tree identifier");
    fs::write(destination, Inventory::current(&source_git_tree).render())
        .expect("semantic baseline candidate should be written");
}
