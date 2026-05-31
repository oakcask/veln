use super::support::*;

#[test]
fn test_json_reports_no_discovered_test_declarations() {
    let project = TestProject::new("test-no-declarations");
    project.write(
        "main_test.veln",
        "fn takes_arg(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":0,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":1}",
            "\"suite_errors\":[{\"kind\":\"discovery\",\"message\":\"no test declarations were discovered\"}]",
            "\"cases\":[]",
        ],
    );
}

#[test]
fn test_human_reports_no_discovered_test_declarations() {
    let project = TestProject::new("test-human-no-declarations");
    project.write(
        "main_test.veln",
        "fn takes_arg(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: test discovery: no test declarations were discovered\n"
    );
}

#[test]
fn test_json_blocks_duplicate_function_like_names_with_origin_note() {
    let project = TestProject::new("test-duplicate-function-like-names-json");
    project.write("first_test.veln", "test same() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "fn same() -> ()\n  ()\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"id\":\"name.duplicate\"",
            "\"message\":\"duplicate function declaration name `same`\"",
            "\"details\":{\"phase\":\"name\",\"node_id\":\"fn-1\",\"name\":\"same\",\"namespace\":\"function\",\"first_node_id\":\"test-1\"}",
            "\"related\":[{\"kind\":\"duplicate_origin\",\"message\":\"First function declaration with this name is here.\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_blocks_duplicate_function_like_names_with_origin_note() {
    let project = TestProject::new("test-duplicate-function-like-names-human");
    project.write("first_test.veln", "test same() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "fn same() -> ()\n  ()\nend\n");

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked same\n");
    assert_contains_all(
        stderr(&output),
        &[
            "second_test.veln:1:1: error[name.duplicate]: duplicate function declaration name `same`",
            "  note: first_test.veln:1:1: First function declaration with this name is here.",
        ],
    );
}

#[test]
fn test_json_blocks_static_gate_before_jdk_execution() {
    let project = TestProject::new("test-static-gate");
    project.write(
        "main_test.veln",
        "test blocked() -> Result<(), AppError>\n  _\nend\n",
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-test-json/v0\"",
            "\"command\":\"test\"",
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"id\":\"hole.unfilled\"",
            "\"cases\":[{\"id\":\"case-1\",\"name\":\"blocked\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_json_reports_parse_static_gate_without_jdk_execution() {
    let project = TestProject::new("test-parse-static-gate");
    project.write("broken_test.veln", "test broken() -> ()\n  @\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"broken_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":0,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"diagnostics\":[{\"id\":\"parse.invalid_token\"",
            "\"message\":\"invalid token in expression\"",
            "\"span\":{\"file\":\"broken_test.veln\"",
            "\"cases\":[]",
        ],
    );
}

#[test]
fn test_json_blocks_cases_from_multiple_files_on_semantic_static_gate() {
    let project = TestProject::new("test-multiple-files-static-gate");
    project.write("first_test.veln", "test first() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "test second() -> Int\n  \"no\"\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"first_test.veln\",\"second_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":2,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":2,\"errors\":0}",
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"second_test.veln\"",
            "\"cases\":[{\"id\":\"case-1\",\"name\":\"first\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"source\":{\"file\":\"first_test.veln\"",
            "{\"id\":\"case-2\",\"name\":\"second\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"source\":{\"file\":\"second_test.veln\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_json_auto_discovers_same_file_test_declarations() {
    let project = TestProject::new("test-same-file-discovery");
    project.write(
        "main.veln",
        concat!(
            "fn helper() -> ()\n",
            "  ()\n",
            "end\n",
            "test same_file() -> Result<(), AppError>\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"same_file\"",
            "\"source\":{\"file\":\"main.veln\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}
#[test]
fn test_json_maps_explicit_source_file_to_paired_test_file() {
    let project = TestProject::new("test-source-to-test-convention");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result<(), AppError>\n",
            "  helper()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "app.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"app.veln\",\"app_test.veln\"],\"confidence\":\"unknown\",\"reason\":\"widened_dependency_graph\",\"notes\":[\"added 1 test file by source-to-test convention\",\"dependency graph is missing module identity for selected source `app.veln`\",\"selected all discovered tests because dependency graph evidence is incomplete\"]}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"paired\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_reports_source_to_test_selection_note() {
    let project = TestProject::new("test-human-source-to-test-convention");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result<(), AppError>\n",
            "  helper()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["app.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked paired\n");
    assert_contains_all(
        stderr(&output),
        &[
            "veln: test selection: added 1 test file by source-to-test convention",
            "app_test.veln:3:3: hint[hole.unfilled]: hole requires a `Result<(), AppError>` value",
        ],
    );
}

#[test]
fn test_json_treats_explicit_directory_target_as_user_selected() {
    let project = TestProject::new("test-explicit-directory-target");
    project.write(
        "tests/app_test.veln",
        "test directory_case() -> Result<(), AppError>\n  _\nend\n",
    );
    project.write("tests/helper.veln", "fn helper() -> ()\n  ()\nend\n");

    let output = project.test(&["--json", "tests"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"tests/app_test.veln\",\"tests/helper.veln\"],\"confidence\":\"complete\",\"reason\":\"user_selected\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"directory_case\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_prints_blocked_cases_and_static_gate_diagnostics() {
    let project = TestProject::new("test-human-static-gate");
    project.write(
        "main_test.veln",
        "test blocked() -> Result<(), AppError>\n  _\nend\n",
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked blocked\n");
    assert_contains_all(
        stderr(&output),
        &["main_test.veln:2:3: hint[hole.unfilled]: hole requires a `Result<(), AppError>` value"],
    );
}

#[test]
fn test_human_reports_passed_and_failed_cases_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-human-cases");
    project.write(
        "main_test.veln",
        concat!(
            "test passes() -> ()\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result<(), String>\n",
            "  Err(\"bad\")\n",
            "end\n",
        ),
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok passes\nnot ok fails\n");
    assert_contains_all(
        stderr(&output),
        &["veln: test `fails` failed: runtime result failure: Err(bad)"],
    );
}

#[test]
fn test_json_discovers_runs_and_captures_stdio_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-cases");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "main_test.veln",
        concat!(
            "test passes() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprintln(\"err\")\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result<(), String>\n",
            "  Err(\"bad\")\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"failed\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":2,\"passed\":1,\"failed\":1,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"passes\",\"kind\":\"test\",\"status\":\"passed\"",
            "\"events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"out\",\"terminator\":\"newline\"",
            "{\"kind\":\"stdio\",\"stream\":\"stderr\",\"operation\":\"eprintln\",\"text\":\"err\",\"terminator\":\"newline\"",
            "\"name\":\"fails\",\"kind\":\"test\",\"status\":\"failed\"",
            "\"failure\":{\"kind\":\"result\",\"message\":\"runtime result failure: Err(bad)\"",
            "\"details\":{\"kind\":\"result\",\"phase\":\"runtime\",\"value\":\"bad\"}",
        ],
    );
}

#[test]
fn test_json_embeds_runtime_contract_failures_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-contract-failure");
    project.write(
        "main_test.veln",
        concat!(
            "test rejects() -> ()\n",
            "require false\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"name\":\"rejects\",\"kind\":\"test\",\"status\":\"failed\"",
            "\"failure\":{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `rejects` blame caller\"",
            "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",\"clause\":\"require\",\"predicate\":\"false\"",
            "\"function\":\"rejects\",\"blame\":\"caller\",\"node_id\":\"contract-",
            "\"span\":{\"file\":\"main_test.veln\"",
        ],
    );
}

#[test]
fn test_explicit_target_runs_same_file_test_declaration_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-explicit-same-file");
    project.write("example.veln", "test example() -> ()\n  ()\nend\n");

    let output = project.test(&["--json", "example.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"passed\"",
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"example.veln\"],\"confidence\":\"complete\",\"reason\":\"user_selected\"}",
            "\"summary\":{\"total\":1,\"passed\":1,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"example\"",
        ],
    );
}

#[test]
fn comparison_line_item_order_summary_example_runs_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("comparison-line-item-order-summary");
    let complete = repo_file("examples/comparison/line_item_order_summary.veln");
    let hole = repo_file("examples/comparison/line_item_order_summary_hole.veln");

    let check_output = project.veln(&["check"], &[complete.as_str()]);
    assert!(check_output.status.success(), "{}", stderr(&check_output));
    assert_eq!(stdout(&check_output), "ok\n");
    assert_eq!(stderr(&check_output), "");

    let test_output = project.test(&[complete.as_str()]);
    assert!(test_output.status.success(), "{}", stderr(&test_output));
    assert_contains_all(
        stdout(&test_output),
        &[
            "ok summarizes_success",
            "ok rejects_malformed_input",
            "ok rejects_bad_quantity",
            "ok rejects_unknown_sku",
        ],
    );
    assert_eq!(stderr(&test_output), "");

    let run_output = project.run(&["main", complete.as_str()]);
    assert!(run_output.status.success(), "{}", stderr(&run_output));
    assert_eq!(stdout(&run_output), "900\n");
    assert_eq!(stderr(&run_output), "");

    let hole_output = project.check_json(&[hole.as_str()]);
    assert!(hole_output.status.success(), "{}", stderr(&hole_output));
    assert_contains_all(
        stdout(&hole_output),
        &[
            "\"status\":\"partial\"",
            "\"id\":\"hole.unfilled\"",
            "\"expected_type\":\"Int\"",
            "\"text\":\"candidate > 0\"",
        ],
    );
    assert_eq!(stderr(&hole_output), "");
}
