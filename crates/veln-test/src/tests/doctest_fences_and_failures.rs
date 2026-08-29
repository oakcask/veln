use super::*;

#[test]
fn consecutive_veln_doctest_fences_create_separate_sources() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"first\")\n",
            "## ```\n",
            "## ```veln\n",
            "## stdio::println(\"second\")\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 2);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "test doctest_1() -> () effects [stdio]\n",
            "  stdio::println(\"first\")\n",
            "  ()\n",
            "end\n",
        )
    );
    assert_eq!(
        doctests.sources[1].text(),
        concat!(
            "test doctest_2() -> () effects [stdio]\n",
            "  stdio::println(\"second\")\n",
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
fn doctest_output_fence_without_pending_doctest_is_ignored() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln-output stream=stdout\n",
            "## orphaned\n",
            "## ```\n",
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
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
        .expect("doctest should have an expectation record");
    assert!(expected.expected_output.is_none());
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn doctest_output_fence_after_prose_does_not_attach_to_previous_doctest() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## This prose separates the runnable example from later output.\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("doctest should have an expectation record");
    assert!(expected.expected_output.is_none());
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn unknown_doctest_output_attribute_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout trim=true\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.unknown_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "unknown doctest output attribute `trim`"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"trim\",\"fence\":\"veln-output\"}"
    );
}

#[test]
fn invalid_doctest_output_stream_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=combined\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("doctest should keep an expectation record");
    assert!(expected.expected_output.is_none());
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "unknown doctest output stream `combined`"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\",\"stream\":\"combined\"}"
    );
}

#[test]
fn missing_doctest_output_stream_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    let expected = doctests
        .expectations
        .get("doctest_1")
        .expect("doctest should keep an expectation record");
    assert!(expected.expected_output.is_none());
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "missing doctest output stream"
    );
    assert_eq!(
        doctests.diagnostics[0].details.to_json(),
        "{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\"}"
    );
}

#[test]
fn duplicate_doctest_output_stream_attribute_reports_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout stream=stderr\n",
            "## ready\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(doctests.diagnostics.len(), 1);
    assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
    assert_eq!(
        doctests.diagnostics[0].message,
        "duplicate doctest output stream attribute"
    );
}

#[test]
fn ignores_non_runnable_doctest_fences() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln ignore\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert!(doctests.sources.is_empty());
    assert!(doctests.expectations.is_empty());
    assert!(
        doctests.diagnostics.is_empty(),
        "{:#?}",
        doctests.diagnostics
    );
}

#[test]
fn extracts_negative_doctest_fences_as_check_only_sources() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = \"no\"\n",
            "## ```\n",
        ),
    );

    let doctests = doctest_sources(&[source]);

    assert_eq!(doctests.sources.len(), 1);
    assert_eq!(
        doctests.sources[0].text(),
        concat!(
            "fn doctest_1() -> () effects [stdio]\n",
            "  let value: Int = \"no\"\n",
            "  ()\n",
            "end\n",
        )
    );
    assert!(doctests.expectations.is_empty());
    assert_eq!(doctests.expected_failures.len(), 1);
    assert!(
        doctests
            .expected_failures
            .contains_key("main.veln#doctest-1_test.veln")
    );
}

#[test]
fn negative_doctest_failure_reconciliation_consumes_matching_error_diagnostics() {
    let source = SourceFile::new("main.veln", "## ```veln fail\n");
    let generated = SourceFile::new("main.veln#doctest-1_test.veln", "fn doctest_1()\nend\n");
    let fail_span = source.span(TextRange::new(0, 16));
    let generated_span = generated.span(TextRange::new(0, generated.len()));
    let diagnostics = vec![Diagnostic::new(
        "parse.expected_item",
        Severity::Error,
        DiagnosticKind::Parse,
        "expected item",
        Some(generated_span),
        JsonValue::Null,
    )];
    let expected_failures =
        BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

    let reconciled = reconcile_expected_doctest_failures(diagnostics, &expected_failures);

    assert!(reconciled.is_empty(), "{reconciled:#?}");
}

#[test]
fn negative_doctest_failure_reconciliation_consumes_matching_semantic_diagnostics() {
    let source = SourceFile::new("main.veln", "## ```veln fail\n");
    let generated = SourceFile::new("main.veln#doctest-1_test.veln", "fn doctest_1()\nend\n");
    let fail_span = source.span(TextRange::new(0, 16));
    let generated_span = generated.span(TextRange::new(0, generated.len()));
    let diagnostics = vec![Diagnostic::new(
        "type.mismatch",
        Severity::Error,
        DiagnosticKind::Type,
        "expected `Int`, but found `String`",
        Some(generated_span),
        JsonValue::Null,
    )];
    let expected_failures =
        BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

    let reconciled = reconcile_expected_doctest_failures(diagnostics, &expected_failures);

    assert!(reconciled.is_empty(), "{reconciled:#?}");
}

#[test]
fn negative_doctest_failure_reconciliation_reports_missing_diagnostic() {
    let source = SourceFile::new("main.veln", "## ```veln fail\n");
    let fail_span = source.span(TextRange::new(0, 16));
    let expected_failures =
        BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

    let reconciled = reconcile_expected_doctest_failures(Vec::new(), &expected_failures);

    assert_eq!(reconciled.len(), 1);
    assert_eq!(reconciled[0].id, "doctest.expected_failure_missing");
    assert_eq!(
        reconciled[0].message,
        "negative doctest produced no error diagnostics"
    );
}

#[test]
fn negative_doctest_failure_reconciliation_requires_error_diagnostic() {
    let source = SourceFile::new("main.veln", "## ```veln fail\n");
    let generated = SourceFile::new("main.veln#doctest-1_test.veln", "fn doctest_1()\nend\n");
    let fail_span = source.span(TextRange::new(0, 16));
    let generated_span = generated.span(TextRange::new(0, generated.len()));
    let diagnostics = vec![Diagnostic::new(
        "hole.unfilled",
        Severity::Hint,
        DiagnosticKind::Hole,
        "hole requires a `()` value",
        Some(generated_span),
        JsonValue::Null,
    )];
    let expected_failures =
        BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

    let reconciled = reconcile_expected_doctest_failures(diagnostics, &expected_failures);

    assert_eq!(reconciled.len(), 2);
    assert_eq!(reconciled[0].id, "hole.unfilled");
    assert_eq!(reconciled[1].id, "doctest.expected_failure_missing");
}
