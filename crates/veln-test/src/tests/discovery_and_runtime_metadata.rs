use super::*;

#[test]
fn discovers_test_declarations_in_selected_files() {
    let module = module(concat!(
        "test first() -> ()\n",
        "  ()\n",
        "end\n",
        "fn helper()\n",
        "  ()\n",
        "end\n",
    ));
    let test_files = BTreeSet::from(["main_test.veln".to_string()]);

    let cases = discover_test_cases(&module, &test_files);

    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].id, "case-1");
    assert_eq!(cases[0].name, "first");
    assert_eq!(cases[0].source.file, "main_test.veln");
    assert_eq!(cases[0].source.node_id, "test-1");
}

#[test]
fn attach_doctest_expectations_marks_matching_cases_as_doctests() {
    let module = module(concat!(
        "test doctest_1() -> ()\n",
        "  ()\n",
        "end\n",
        "test ordinary() -> ()\n",
        "  ()\n",
        "end\n",
    ));
    let test_files = BTreeSet::from(["main_test.veln".to_string()]);
    let mut cases = discover_test_cases(&module, &test_files);
    let expectations = BTreeMap::from([(
        "doctest_1".to_string(),
        DoctestExpectation {
            expected_output: Some(ExpectedOutput {
                stdout: Some("ready".to_string()),
                stderr: Some("warn".to_string()),
                ..ExpectedOutput::default()
            }),
            expected_runtime_failure: None,
        },
    )]);

    attach_doctest_expectations(&mut cases, &expectations);

    assert_eq!(cases[0].kind, "doctest");
    assert_eq!(
        cases[0]
            .expected_output
            .as_ref()
            .and_then(|output| output.stdout.as_deref()),
        Some("ready")
    );
    assert_eq!(
        cases[0]
            .expected_output
            .as_ref()
            .and_then(|output| output.stderr.as_deref()),
        Some("warn")
    );
    assert_eq!(cases[1].kind, "test");
    assert!(cases[1].expected_output.is_none());
}

#[test]
fn ordinary_zero_argument_functions_are_not_test_cases() {
    let module = module("fn helper()\n  ()\nend\n");
    let test_files = BTreeSet::from(["main_test.veln".to_string()]);

    let cases = discover_test_cases(&module, &test_files);

    assert!(cases.is_empty());
}

#[test]
fn report_json_contains_summary_suite_errors_and_cases() {
    let module = module("test first() -> ()\n  ()\nend\n");
    let test_files = BTreeSet::from(["main_test.veln".to_string()]);
    let cases = discover_test_cases(&module, &test_files);
    let report = TestReport::new(
        TestSelection {
            mode: TestSelectionMode::Explicit,
            targets: vec!["main_test.veln".to_string()],
            confidence: TestSelectionConfidence::Complete,
            reason: TestSelectionReason::UserSelected,
            notes: Vec::new(),
        },
        Vec::new(),
        Vec::new(),
        cases,
    );

    assert_eq!(
        report.to_json(),
        concat!(
            "{\"schema_version\":\"veln-test-json/v0\",\"command\":\"test\",",
            "\"status\":\"passed\",\"selection\":{\"mode\":\"explicit\",",
            "\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",",
            "\"reason\":\"user_selected\"},\"summary\":{\"total\":1,",
            "\"passed\":1,\"failed\":0,\"skipped\":0,\"todo\":0,",
            "\"blocked\":0,\"errors\":0},\"diagnostics\":[],",
            "\"suite_errors\":[],\"cases\":[{\"id\":\"case-1\",",
            "\"name\":\"first\",\"kind\":\"test\",\"status\":\"passed\",",
            "\"source\":{\"file\":\"main_test.veln\",\"node_id\":\"test-1\",",
            "\"span\":{\"start\":{\"line\":1,\"column\":1,\"offset\":0},",
            "\"end\":{\"line\":4,\"column\":1,\"offset\":28}}},",
            "\"reason\":null,\"failure\":null,\"events\":[],",
            "\"diagnostics\":[]}]}"
        )
    );
}

#[test]
fn report_json_counts_suite_errors_and_runtime_failures() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\nend\n");
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let report = TestReport::new(
        TestSelection {
            mode: TestSelectionMode::Discovered,
            targets: vec!["main_test.veln".to_string()],
            confidence: TestSelectionConfidence::Complete,
            reason: TestSelectionReason::PatternDiscovery,
            notes: Vec::new(),
        },
        Vec::new(),
        vec![SuiteError::discovery("project discovery failed")],
        vec![TestCase {
            id: "case-1".to_string(),
            name: "first".to_string(),
            kind: "test".to_string(),
            status: TestCaseStatus::Error,
            source: TestCaseSource {
                file: "main_test.veln".to_string(),
                node_id: "test-1".to_string(),
                span,
            },
            reason: Some("runner_error".to_string()),
            failure: Some(TestFailure::runtime("javac not found")),
            expected_output: None,
            expected_runtime_failure: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        }],
    );

    let json = report.to_json();

    assert!(json.contains("\"status\":\"error\""));
    assert!(json.contains("\"summary\":{\"total\":1,\"passed\":0,\"failed\":0"));
    assert!(json.contains("\"errors\":2"));
    assert!(json.contains(
        "\"suite_errors\":[{\"kind\":\"discovery\",\"message\":\"project discovery failed\"}]"
    ));
    assert!(json.contains("\"failure\":{\"kind\":\"runtime\""));
    assert!(json.contains("\"message\":\"javac not found\""));
}

#[test]
fn stdio_events_preserve_stream_sequence_and_source() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: source_file.span(TextRange::new(0, source_file.len())),
    };
    let output = Output {
        status: exit_status(0),
        stdout: b"hello\n".to_vec(),
        stderr: b"warn\n".to_vec(),
    };

    let events = stdio_events_from_output(&output, &source);

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].to_json(),
        concat!(
            "{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"print\",",
            "\"text\":\"hello\\n\",\"terminator\":\"none\",\"sequence\":1,",
            "\"node_id\":\"test-1\",\"span\":{\"file\":\"main_test.veln\",",
            "\"start\":{\"line\":1,\"column\":1,\"offset\":0},",
            "\"end\":{\"line\":4,\"column\":1,\"offset\":28}}}"
        )
    );
    assert!(events[1].to_json().contains("\"sequence\":2"));
    assert!(events[1].to_json().contains("\"stream\":\"stderr\""));
}

#[test]
fn ignores_slash_doc_comment_veln_fences() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "/// ```veln\n",
            "/// stdio::println(\"ready\")\n",
            "/// ```\n",
            "/// ```veln-output stream=stdout\n",
            "/// ready\n",
            "/// ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert!(doctests.sources.is_empty());
    assert!(doctests.expectations.is_empty());
    assert!(doctests.diagnostics.is_empty());
}

#[test]
fn extracts_hash_doc_comments_with_hidden_setup_and_visible_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## # visible example comment\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  let greeting = \"ready\"\n",
            "  # visible example comment\n",
            "  stdio::println(greeting)\n",
            "  ()\n",
            "end\n",
        )
    );
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn extracts_hash_doc_comment_veln_fences_with_expected_output() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  stdio::println(\"ready\")\n",
            "  ()\n",
            "end\n",
        )
    );
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("expected output should be recorded");
    let output = expected
        .expected_output
        .as_ref()
        .expect("expected output should be recorded");
    assert_eq!(output.stdout.as_deref(), Some("ready"));
    assert_eq!(output.stderr, None);
}

#[test]
fn extracts_doctest_runtime_contract_failure_expectation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln runtime=contract clause=require predicate=false function=reject blame=caller\n",
            "## reject()\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  reject()\n",
            "  ()\n",
            "end\n",
        )
    );
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("runtime expectation should be recorded");
    let expected = expected
        .expected_runtime_failure
        .as_ref()
        .expect("runtime expectation should be recorded");
    let ExpectedRuntimeFailure::Contract(expected) = expected else {
        panic!("expected contract runtime failure");
    };
    assert_eq!(expected.clause, "require");
    assert_eq!(expected.predicate, "false");
    assert_eq!(expected.function.as_deref(), Some("reject"));
    assert_eq!(expected.blame.as_deref(), Some("caller"));
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn extracts_doctest_runtime_result_failure_expectation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln error=String runtime=result value=bad\n",
            "## Err(\"bad\")?\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("runtime expectation should be recorded");
    let expected = expected
        .expected_runtime_failure
        .as_ref()
        .expect("runtime expectation should be recorded");
    let ExpectedRuntimeFailure::Result(expected) = expected else {
        panic!("expected result runtime failure");
    };
    assert_eq!(expected.value, "bad");
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn extracts_doctest_runtime_ensure_failure_expectation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln runtime=ensure predicate=false function=reject blame=implementation\n",
            "## reject()\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("runtime expectation should be recorded");
    let expected = expected
        .expected_runtime_failure
        .as_ref()
        .expect("runtime expectation should be recorded");
    let ExpectedRuntimeFailure::ContractClause(expected) = expected else {
        panic!("expected ensure runtime failure");
    };
    assert_eq!(expected.clause, "ensure");
    assert_eq!(expected.predicate, "false");
    assert_eq!(expected.function.as_deref(), Some("reject"));
    assert_eq!(expected.blame.as_deref(), Some("implementation"));
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn runtime_contract_expectation_requires_predicate_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln runtime=contract clause=require\n",
            "## reject()\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "missing doctest runtime contract predicate"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"predicate\"}"
    );
}

#[test]
fn runtime_result_expectation_requires_value_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln error=String runtime=result\n",
            "## Err(\"bad\")?\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "missing doctest runtime result value"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"value\"}"
    );
}

#[test]
fn runtime_ensure_expectation_requires_predicate_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!("## ```veln runtime=ensure\n", "## reject()\n", "## ```\n",),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "missing doctest runtime ensure predicate"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"predicate\"}"
    );
}

#[test]
fn runtime_expectation_rejects_other_kind_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln runtime=contract clause=require predicate=false value=bad\n",
            "## reject()\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.unknown_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "unknown doctest attribute `value`"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"value\",\"fence\":\"veln\"}"
    );
}

#[test]
fn duplicate_doctest_output_stream_reports_diagnostic() {
    let source = SourceFile::new(
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
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("first expected output should be kept");
    let output = expected
        .expected_output
        .as_ref()
        .expect("expected output should be recorded");
    assert_eq!(output.stdout.as_deref(), Some("ready"));
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.duplicate_output");
    assert_eq!(
        doctests.diagnostics[0].message,
        "duplicate expected stdout output fence"
    );
    assert_eq!(doctests.diagnostics[0].related.len(), 1);
}

#[test]
fn duplicate_stderr_doctest_output_stream_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::eprintln(\"warn\")\n",
            "## ```\n",
            "## ```veln-output stream=stderr\n",
            "## warn\n",
            "## ```\n",
            "## ```veln-output stream=stderr\n",
            "## duplicate\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("expected stderr output should be captured");
    let output = expected
        .expected_output
        .as_ref()
        .expect("expected output should be recorded");
    assert_eq!(output.stderr.as_deref(), Some("warn"));
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.duplicate_output");
    assert_eq!(
        doctests.diagnostics[0].message,
        "duplicate expected stderr output fence"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"stream\":\"stderr\"}"
    );
    assert_eq!(doctests.diagnostics[0].related.len(), 1);
}
