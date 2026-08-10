use super::support::*;

#[test]
fn check_json_typechecks_executable_doctest_fences() {
    let project = TestProject::new("check-json-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = \"no\"\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"main.veln#doctest-1_test.veln\"",
        ],
    );
}

#[test]
fn check_json_uses_doctest_error_type_fence_attribute() {
    let project = TestProject::new("check-json-doctest-error-type");
    project.write(
        "main.veln",
        concat!(
            "## ```veln error=AppError\n",
            "## let value: Int = Ok(1)?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"ok\"",
            "\"diagnostics\":[]",
            "\"summary\":{\"diagnostic_count\":0",
        ],
    );
}

#[test]
fn check_json_infers_doctest_error_type_from_public_result() {
    let project = TestProject::new("check-json-doctest-public-result-error-type");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = Ok(1)?\n",
            "## ```\n",
            "pub fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"ok\"",
            "\"diagnostics\":[]",
            "\"summary\":{\"diagnostic_count\":0",
        ],
    );
}

#[test]
fn check_reports_duplicate_doctest_output_stream() {
    let project = TestProject::new("check-duplicate-doctest-output");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## duplicate\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.duplicate_output\"",
            "\"kind\":\"doc\"",
            "\"message\":\"duplicate expected stdout output fence\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"stream\":\"stdout\"}",
            "\"related\":[{\"kind\":\"duplicate_origin\",\"message\":\"First expected stdout output fence is here.\"",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.duplicate_output]: duplicate expected stdout output fence",
            "note: main.veln:4:1: First expected stdout output fence is here.",
        ],
    );
}

#[test]
fn check_reports_unknown_doctest_metadata() {
    let project = TestProject::new("check-unknown-doctest-metadata");
    project.write(
        "main.veln",
        concat!(
            "## ```veln skip=true\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout trim=true\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.unknown_metadata\"",
            "\"message\":\"unknown doctest attribute `skip`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"skip\",\"fence\":\"veln\"}",
            "\"message\":\"unknown doctest output attribute `trim`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"trim\",\"fence\":\"veln-output\"}",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.unknown_metadata]: unknown doctest attribute `skip`",
            "error[doctest.unknown_metadata]: unknown doctest output attribute `trim`",
        ],
    );
}

#[test]
fn check_reports_invalid_doctest_metadata() {
    let project = TestProject::new("check-invalid-doctest-metadata");
    project.write(
        "main.veln",
        concat!(
            "## ```veln error=\n",
            "## let value = Ok(1)?\n",
            "## ```\n",
            "## ```veln-output\n",
            "## ready\n",
            "## ```\n",
            "## ```veln-output stream=combined\n",
            "## mixed\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.invalid_metadata\"",
            "\"message\":\"empty doctest error type\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"error\"}",
            "\"message\":\"missing doctest output stream\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\"}",
            "\"message\":\"unknown doctest output stream `combined`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\",\"stream\":\"combined\"}",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.invalid_metadata]: empty doctest error type",
            "error[doctest.invalid_metadata]: missing doctest output stream",
            "error[doctest.invalid_metadata]: unknown doctest output stream `combined`",
        ],
    );
}

#[test]
fn check_ignores_non_runnable_doctest_fences() {
    let project = TestProject::new("check-ignore-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln ignore\n",
            "## missing_function()\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_reports_negative_doctest_with_semantic_only_diagnostic() {
    let project = TestProject::new("check-negative-doctest-semantic-only");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = \"no\"\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.expected_failure_missing\"",
            "\"message\":\"negative doctest produced no error diagnostics\"",
            "\"details\":{\"kind\":\"doctest_metadata\"}",
        ],
    );
}

#[test]
fn check_accepts_negative_doctest_with_parse_diagnostic() {
    let project = TestProject::new("check-negative-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## @\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_reports_negative_doctest_that_does_not_fail() {
    let project = TestProject::new("check-negative-doctest-missing-failure");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = 1\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.expected_failure_missing\"",
            "\"message\":\"negative doctest produced no error diagnostics\"",
            "\"details\":{\"kind\":\"doctest_metadata\"}",
        ],
    );
}

#[test]
fn check_reports_negative_doctest_with_only_hole_hint() {
    let project = TestProject::new("check-negative-doctest-hole-hint");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = _\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"hole.unfilled\"",
            "\"severity\":\"hint\"",
            "\"span\":{\"file\":\"main.veln#doctest-1_test.veln\"",
            "\"id\":\"doctest.expected_failure_missing\"",
            "\"message\":\"negative doctest produced no error diagnostics\"",
            "\"summary\":{\"diagnostic_count\":2",
        ],
    );
}

#[test]
fn check_json_typechecks_hidden_doctest_setup_lines() {
    let project = TestProject::new("check-json-hidden-doctest-setup");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_json_typechecks_hash_doctest_setup_with_visible_comment() {
    let project = TestProject::new("check-json-hash-doctest-setup");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## # visible example comment\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}
#[test]
fn test_json_runs_doctest_and_compares_expected_output_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-doctest-output");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"passed\"",
            "\"summary\":{\"total\":1,\"passed\":1,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"doctest_1\",\"kind\":\"doctest\",\"status\":\"passed\"",
            "\"events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"ready\",\"terminator\":\"newline\"",
        ],
    );
}

#[test]
fn test_json_reports_doctest_expected_output_mismatch_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-doctest-output-mismatch");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"waiting\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"failed\"",
            "\"name\":\"doctest_1\",\"kind\":\"doctest\",\"status\":\"failed\"",
            "\"reason\":\"expected_output\"",
            "\"failure\":{\"kind\":\"output\",\"message\":\"expected stdout output did not match\"",
            "\"details\":{\"kind\":\"output\",\"stream\":\"stdout\",\"expected\":\"ready\",\"actual\":\"waiting\\n\",\"first_difference\":{\"line\":1,\"expected\":\"ready\",\"actual\":\"waiting\"}",
            "\"actual_events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"waiting\"",
            "\"expected_span\":{\"file\":\"main.veln\"",
        ],
    );
}
