use super::*;

#[test]
fn expected_runtime_contract_failure_marks_matching_case_passed() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  reject()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(ExpectedContractFailure {
            clause: "require".to_string(),
            predicate: "false".to_string(),
            function: Some("reject".to_string()),
            blame: Some("caller".to_string()),
            span: span.clone(),
        })),
        events: Vec::new(),
        diagnostics: Vec::new(),
    };
    let failure = TestFailure::contract(
        "contract failure: require `false` in `reject` blame caller".to_string(),
        "require".to_string(),
        "false".to_string(),
        "reject".to_string(),
        "caller".to_string(),
        "contract-1".to_string(),
        span,
    );

    apply_runtime_result(&mut case, Some(failure));

    assert_eq!(case.status, TestCaseStatus::Passed);
    assert!(case.reason.is_none());
    assert!(case.failure.is_none());
}

#[test]
fn expected_runtime_ensure_failure_marks_matching_case_passed() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  reject()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: Some(ExpectedRuntimeFailure::ContractClause(
            ExpectedContractFailure {
                clause: "ensure".to_string(),
                predicate: "false".to_string(),
                function: Some("reject".to_string()),
                blame: Some("implementation".to_string()),
                span: span.clone(),
            },
        )),
        events: Vec::new(),
        diagnostics: Vec::new(),
    };
    let failure = TestFailure::contract(
        "contract failure: ensure `false` in `reject` blame implementation".to_string(),
        "ensure".to_string(),
        "false".to_string(),
        "reject".to_string(),
        "implementation".to_string(),
        "contract-1".to_string(),
        span,
    );

    apply_runtime_result(&mut case, Some(failure));

    assert_eq!(case.status, TestCaseStatus::Passed);
    assert!(case.reason.is_none());
    assert!(case.failure.is_none());
}

#[test]
fn expected_runtime_result_failure_marks_matching_case_passed() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> Result<(), String> effects [stdio]\n  Err(\"bad\")?\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: Some(ExpectedRuntimeFailure::Result(ExpectedResultFailure {
            value: "bad".to_string(),
            span,
        })),
        events: Vec::new(),
        diagnostics: Vec::new(),
    };

    apply_runtime_result(
        &mut case,
        Some(TestFailure::result("bad".to_string(), None)),
    );

    assert_eq!(case.status, TestCaseStatus::Passed);
    assert!(case.reason.is_none());
    assert!(case.failure.is_none());
}

#[test]
fn expected_output_mismatch_still_reports_after_runtime_expectation_matches() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  reject()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("expected".to_string()),
            stderr: None,
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(ExpectedContractFailure {
            clause: "require".to_string(),
            predicate: "false".to_string(),
            function: Some("reject".to_string()),
            blame: Some("caller".to_string()),
            span: span.clone(),
        })),
        events: vec![stdio_event(
            "stdout", "println", "actual", "newline", 1, "call-1", &span,
        )],
        diagnostics: Vec::new(),
    };
    let failure = TestFailure::contract(
        "contract failure: require `false` in `reject` blame caller".to_string(),
        "require".to_string(),
        "false".to_string(),
        "reject".to_string(),
        "caller".to_string(),
        "contract-1".to_string(),
        span,
    );

    apply_runtime_result(&mut case, Some(failure));
    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Failed);
    assert_eq!(case.reason.as_deref(), Some("expected_output"));
    let failure = case.failure.expect("output mismatch should create failure");
    assert_eq!(failure.kind, "output");
    let failure_json = failure.to_json().to_json();
    assert!(failure_json.contains("\"expected\":\"expected\""));
    assert!(failure_json.contains("\"actual\":\"actual\\n\""));
}

#[test]
fn expected_runtime_contract_failure_reports_mismatch() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  reject()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(ExpectedContractFailure {
            clause: "require".to_string(),
            predicate: "true".to_string(),
            function: Some("reject".to_string()),
            blame: Some("caller".to_string()),
            span: span.clone(),
        })),
        events: Vec::new(),
        diagnostics: Vec::new(),
    };
    let failure = TestFailure::contract(
        "contract failure: require `false` in `reject` blame caller".to_string(),
        "require".to_string(),
        "false".to_string(),
        "reject".to_string(),
        "caller".to_string(),
        "contract-1".to_string(),
        span,
    );

    apply_runtime_result(&mut case, Some(failure));

    assert_eq!(case.status, TestCaseStatus::Failed);
    assert_eq!(case.reason.as_deref(), Some("expected_runtime_failure"));
    let failure_json = case
        .failure
        .expect("mismatch should fail")
        .to_json()
        .to_json();
    assert!(failure_json.contains("\"kind\":\"runtime_expectation\""));
    assert!(failure_json.contains("\"expected\":{\"kind\":\"contract\""));
    assert!(failure_json.contains("\"predicate\":\"true\""));
    assert!(failure_json.contains("\"actual\":{\"kind\":\"contract\""));
    assert!(failure_json.contains("\"predicate\":\"false\""));
}

#[test]
fn contract_trace_skips_non_contract_and_malformed_lines() {
    let trace = concat!(
        "stdio\tstdout\tprint\n",
        "contract\trequire\tinvalid-hex\t72656a65637473\tcaller\t636f6e74726163742d32\t6d61696e5f746573742e76656c6e\t2\t1\t2\t14\n",
    );

    let failure = contract_failure_from_trace(trace);

    assert!(failure.is_none());
}

#[test]
fn expands_absolute_source_target_to_absolute_paired_test_file() {
    let root = test_root("absolute-paired-source");
    fs::create_dir_all(&root).expect("create test root");
    let source = root.join("app.veln");
    let test = root.join("app_test.veln");
    fs::write(&source, "").expect("write source file");
    fs::write(&test, "").expect("write test file");

    let expansion = expand_test_targets(&root, std::slice::from_ref(&source));

    assert_eq!(expansion.targets, vec![source, test]);
    assert_eq!(expansion.source_to_test_added_count, 1);
    fs::remove_dir_all(root).expect("remove test root");
}

#[test]
fn negative_doctest_failure_reconciliation_keeps_unrelated_diagnostics() {
    let source = SourceFile::new("main.veln", "## ```veln fail\n");
    let generated = SourceFile::new("main.veln#doctest-1_test.veln", "fn doctest_1()\nend\n");
    let other = SourceFile::new("other.veln", "fn helper()\nend\n");
    let fail_span = source.span(TextRange::new(0, 16));
    let generated_span = generated.span(TextRange::new(0, generated.len()));
    let other_span = other.span(TextRange::new(0, other.len()));
    let diagnostics = vec![
        Diagnostic::new(
            "parse.expected_item",
            Severity::Error,
            DiagnosticKind::Parse,
            "expected item",
            Some(generated_span),
            JsonValue::Null,
        ),
        Diagnostic::new(
            "parse.expected_end",
            Severity::Error,
            DiagnosticKind::Parse,
            "expected `end`",
            Some(other_span),
            JsonValue::Null,
        ),
    ];
    let expected_failures =
        BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

    let reconciled = reconcile_expected_doctest_failures(diagnostics, &expected_failures);

    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].id, "parse.expected_end");
}

#[test]
fn stdio_trace_falls_back_to_test_source_for_missing_call_identity() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: source_file.span(TextRange::new(0, source_file.len())),
    };

    let events = stdio_events_from_trace(
        "1\tstdout\tprint\tnone\t\t\t7265616479\n",
        &BTreeMap::new(),
        &source,
    );

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].to_json(),
        concat!(
            "{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"print\",",
            "\"text\":\"ready\",\"terminator\":\"none\",\"sequence\":1,",
            "\"node_id\":\"test-1\",\"span\":{\"file\":\"main_test.veln\",",
            "\"start\":{\"line\":1,\"column\":1,\"offset\":0},",
            "\"end\":{\"line\":4,\"column\":1,\"offset\":28}}}"
        )
    );
}

#[test]
fn expected_stderr_mismatch_reports_stderr_failure() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready".to_string()),
            stderr: Some("warn".to_string()),
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: vec![
            stdio_event(
                "stdout",
                "println",
                "ready",
                "newline",
                1,
                "call-1",
                &source_file.span(TextRange::new(0, source_file.len())),
            ),
            stdio_event(
                "stderr",
                "eprintln",
                "error",
                "newline",
                2,
                "call-2",
                &source_file.span(TextRange::new(0, source_file.len())),
            ),
        ],
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Failed);
    assert_eq!(case.reason.as_deref(), Some("expected_output"));
    let failure = case.failure.expect("mismatch should create failure");
    assert_eq!(failure.message, "expected stderr output did not match");
    let failure_json = failure.to_json().to_json();
    assert!(failure_json.contains("\"stream\":\"stderr\""));
    assert!(failure_json.contains("\"expected\":\"warn\""));
    assert!(failure_json.contains("\"actual\":\"error\\n\""));
}

#[test]
fn expected_output_mismatch_limits_actual_events_to_first_four() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready".to_string()),
            stderr: None,
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: (1..=5)
            .map(|sequence| {
                stdio_event(
                    "stdout",
                    "println",
                    &format!("line {sequence}"),
                    "newline",
                    sequence,
                    &format!("call-{sequence}"),
                    &span,
                )
            })
            .collect(),
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    let failure = case.failure.expect("mismatch should create failure");
    let failure_json = failure.to_json().to_json();
    assert!(failure_json.contains("\"sequence\":4"));
    assert!(!failure_json.contains("\"sequence\":5"));
}

#[test]
fn test_run_status_precedence_handles_errors_blockers_and_failures() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\nend\n");
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: source_file.span(TextRange::new(0, source_file.len())),
    };
    let case = |status| TestCase {
        id: "case-1".to_string(),
        name: "first".to_string(),
        kind: "test".to_string(),
        status,
        source: TestCaseSource {
            file: source.file.clone(),
            node_id: source.node_id.clone(),
            span: source.span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: None,
        events: Vec::new(),
        diagnostics: Vec::new(),
    };
    let diagnostic = Diagnostic::new(
        "type.mismatch",
        Severity::Error,
        DiagnosticKind::Type,
        "expected `Int`, but found `String`",
        Some(source.span.clone()),
        JsonValue::Null,
    );

    assert_eq!(
        test_run_status(&[case(TestCaseStatus::Error)], &[], &[]),
        TestRunStatus::Error
    );
    assert_eq!(
        test_run_status(&[], &[], &[SuiteError::discovery("no tests")]),
        TestRunStatus::Blocked
    );
    assert_eq!(
        test_run_status(&[case(TestCaseStatus::Passed)], &[diagnostic], &[]),
        TestRunStatus::Blocked
    );
    assert_eq!(
        test_run_status(&[case(TestCaseStatus::Blocked)], &[], &[]),
        TestRunStatus::Blocked
    );
    assert_eq!(
        test_run_status(&[case(TestCaseStatus::Failed)], &[], &[]),
        TestRunStatus::Failed
    );
}
