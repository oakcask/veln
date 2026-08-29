//! Test discovery, test JSON, and captured events.
//!
//! ```rust
//! use veln_test as _;
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::process::Output;

use veln_ast::{BodyLineKind, Expr, ExprKind, FunctionKind, SurfaceModule};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity, diagnostic_to_json};
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan, TextRange};

mod runtime_expectation;
mod selection;

pub use runtime_expectation::{
    ExpectedContractFailure, ExpectedResultFailure, ExpectedRuntimeFailure, apply_runtime_result,
};
pub use selection::{
    TestSelection, TestSelectionConfidence, TestSelectionMetadata, TestSelectionMode,
    TestSelectionPlan, TestSelectionReason, TestTargetExpansion, dependency_aware_selection_plan,
    expand_test_targets, selected_test_files, selection_targets,
};

pub fn discover_test_cases(module: &SurfaceModule, test_files: &BTreeSet<String>) -> Vec<TestCase> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Test && test_files.contains(function.span.file.as_str())
        })
        .enumerate()
        .map(|(index, function)| TestCase {
            id: format!("case-{}", index + 1),
            name: function
                .name
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string()),
            kind: "test".to_string(),
            status: TestCaseStatus::Passed,
            source: TestCaseSource {
                file: function.span.file.as_str().to_string(),
                node_id: function.node_id.display(function.kind.node_prefix()),
                span: function.span.clone(),
            },
            reason: None,
            failure: None,
            expected_output: None,
            expected_runtime_failure: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        })
        .collect()
}

pub fn attach_doctest_expectations(
    cases: &mut [TestCase],
    expectations: &BTreeMap<String, DoctestExpectation>,
) {
    for case in cases {
        if let Some(expectation) = expectations.get(&case.name) {
            case.kind = "doctest".to_string();
            case.expected_output = expectation.expected_output.clone();
            case.expected_runtime_failure = expectation.expected_runtime_failure.clone();
        }
    }
}

pub fn doctest_sources(sources: &[SourceFile]) -> DoctestSources {
    let mut generated_sources = Vec::new();
    let mut expectations = BTreeMap::new();
    let mut expected_failures = BTreeMap::new();
    let mut diagnostics = Vec::new();
    let mut next_index = 1;
    let signatures = result_error_signatures(sources);

    for source in sources {
        let extracted = extract_doctests(source, &signatures);
        diagnostics.extend(extracted.diagnostics);
        for doctest in extracted.doctests {
            let name = format!("doctest_{next_index}");
            let generated_path =
                format!("{}#doctest-{next_index}_test.veln", source.path().as_str());
            let generated = generated_doctest_source(&name, &doctest);
            if let Some(fail_span) = doctest.fail_span {
                expected_failures.insert(generated_path.clone(), fail_span);
            }
            generated_sources.push(SourceFile::new(generated_path, generated));
            if !doctest.should_fail {
                expectations.insert(
                    name,
                    DoctestExpectation {
                        expected_output: doctest.expected_output,
                        expected_runtime_failure: doctest.expected_runtime_failure,
                    },
                );
            }
            next_index += 1;
        }
    }

    DoctestSources {
        sources: generated_sources,
        expectations,
        expected_failures,
        diagnostics,
    }
}

pub fn visible_doctests(source: &SourceFile) -> VisibleDoctests {
    let signatures = result_error_signatures(std::slice::from_ref(source));
    let extracted = extract_doctests(source, &signatures);
    VisibleDoctests {
        doctests: extracted
            .doctests
            .into_iter()
            .map(|doctest| VisibleDoctest {
                code: doctest.visible_code.join("\n"),
                expected_error: doctest.error_type,
                should_fail: doctest.should_fail,
                expected_output: doctest.expected_output,
            })
            .collect(),
        diagnostics: extracted.diagnostics,
    }
}

pub fn reconcile_expected_doctest_failures(
    diagnostics: Vec<Diagnostic>,
    expected_failures: &BTreeMap<String, SourceSpan>,
) -> Vec<Diagnostic> {
    if expected_failures.is_empty() {
        return diagnostics;
    }

    let mut matched = BTreeSet::new();
    let mut kept = Vec::new();
    for diagnostic in diagnostics {
        if let Some(span) = &diagnostic.span
            && diagnostic.severity == Severity::Error
            && expected_failures.contains_key(span.file.as_str())
        {
            matched.insert(span.file.as_str().to_string());
            continue;
        }
        kept.push(diagnostic);
    }

    for (path, span) in expected_failures {
        if matched.contains(path) {
            continue;
        }
        kept.push(Diagnostic::new(
            "doctest.expected_failure_missing",
            Severity::Error,
            DiagnosticKind::Doc,
            "negative doctest produced no error diagnostics",
            Some(span.clone()),
            JsonValue::object([("kind", JsonValue::string("doctest_metadata"))]),
        ));
    }
    kept
}

pub fn compare_expected_output(case: &mut TestCase) {
    let Some(expected) = &case.expected_output else {
        return;
    };
    let actual_stdout = reconstructed_stream(&case.events, "stdout");
    let actual_stderr = reconstructed_stream(&case.events, "stderr");
    let expected_stdout = expected.stdout.clone().unwrap_or_default();
    let expected_stderr = expected.stderr.clone().unwrap_or_default();
    if normalize_lines(&actual_stdout) == normalize_lines(&expected_stdout)
        && normalize_lines(&actual_stderr) == normalize_lines(&expected_stderr)
    {
        return;
    }

    let (stream, expected_text, actual_text, expected_span) =
        if normalize_lines(&actual_stdout) != normalize_lines(&expected_stdout) {
            (
                "stdout",
                expected_stdout,
                actual_stdout,
                expected.stdout_span.clone(),
            )
        } else {
            (
                "stderr",
                expected_stderr,
                actual_stderr,
                expected.stderr_span.clone(),
            )
        };
    let first_difference = first_differing_line(&expected_text, &actual_text);
    let actual_events = output_events_for_stream(&case.events, stream);
    case.status = TestCaseStatus::Failed;
    case.reason = Some("expected_output".to_string());
    case.failure = Some(TestFailure::output_mismatch(
        stream,
        expected_text,
        actual_text,
        first_difference,
        expected_span,
        actual_events,
    ));
}

pub fn stdio_call_spans(module: &SurfaceModule) -> BTreeMap<(String, String), SourceSpan> {
    let mut spans = BTreeMap::new();
    for function in &module.functions {
        for line in &function.body {
            match &line.kind {
                BodyLineKind::Let { expr, .. } | BodyLineKind::Expr { expr } => {
                    collect_stdio_call_spans(expr, &mut spans);
                }
            }
        }
    }
    spans
}

pub fn stdio_events_from_output(output: &Output, source: &TestCaseSource) -> Vec<JsonValue> {
    let mut events = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        events.push(stdio_event(
            "stdout",
            "print",
            stdout.as_ref(),
            "none",
            events.len() + 1,
            &source.node_id,
            &source.span,
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        events.push(stdio_event(
            "stderr",
            "print",
            stderr.as_ref(),
            "none",
            events.len() + 1,
            &source.node_id,
            &source.span,
        ));
    }
    events
}

pub fn stdio_events_from_trace(
    trace: &str,
    call_spans: &BTreeMap<(String, String), SourceSpan>,
    fallback_source: &TestCaseSource,
) -> Vec<JsonValue> {
    trace
        .lines()
        .filter_map(|line| stdio_event_from_trace_line(line, call_spans, fallback_source))
        .collect()
}

pub fn contract_failure_from_trace(trace: &str) -> Option<TestFailure> {
    trace.lines().find_map(contract_failure_from_trace_line)
}

pub fn result_failure_from_trace(trace: &str) -> Option<TestFailure> {
    trace.lines().find_map(result_failure_from_trace_line)
}

pub fn test_run_status(
    cases: &[TestCase],
    diagnostics: &[Diagnostic],
    suite_errors: &[SuiteError],
) -> TestRunStatus {
    if cases
        .iter()
        .any(|case| case.status == TestCaseStatus::Error)
    {
        TestRunStatus::Error
    } else if (!suite_errors.is_empty() && cases.is_empty())
        || has_error(diagnostics)
        || cases
            .iter()
            .any(|case| case.status == TestCaseStatus::Blocked)
    {
        TestRunStatus::Blocked
    } else if cases
        .iter()
        .any(|case| case.status == TestCaseStatus::Failed)
    {
        TestRunStatus::Failed
    } else {
        TestRunStatus::Passed
    }
}

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

mod doctest_extractor;
mod doctest_metadata;
mod error_inference;
mod models;
mod output_and_json;
mod stdio_and_trace;

use doctest_extractor::*;
use doctest_metadata::*;
use error_inference::*;
use output_and_json::*;
use stdio_and_trace::*;

pub use models::*;

#[cfg(test)]
mod tests;
