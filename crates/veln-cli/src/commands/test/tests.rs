use super::process_discovered_test_cases;
use super::{
    JvmExecution, SchedulerError, TestCaseJob, TestExecution, TestJvmExecution,
    TestProgramHookGuard, TestRunFiles, execute_test_case_job, execute_test_program,
    preflight_runnable_test_case_jobs, prepare_test_case_job, resolve_test_jobs,
    run_test_case_jobs,
};
use std::collections::BTreeSet;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use veln_analysis::{DoctestMode, ProjectAnalysis, analyze_project};
use veln_backend_jvm::{JvmProgram, generate_classfiles_with_test_entries};
use veln_diagnostics::JsonValue;
use veln_project::Project;
use veln_source::{SourceFile, TextRange};
use veln_test::{
    TestCase, TestCaseSource, TestCaseStatus, TestFailure, attach_doctest_expectations,
    discover_test_cases, selected_test_files,
};

#[path = "tests/execution.rs"]
mod execution;
#[path = "tests/scheduling.rs"]
mod scheduling;

fn assert_stdio_event(case: &TestCase, stream: &str, text: &str) {
    assert!(
        case.events.iter().any(|event| {
            json_field(event, "kind") == Some(&JsonValue::string("stdio"))
                && json_field(event, "stream") == Some(&JsonValue::string(stream))
                && json_field(event, "text") == Some(&JsonValue::string(text))
        }),
        "case `{}` should include {stream} event `{text}` in {:?}",
        case.name,
        case.events
    );
}

fn json_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(field, value)| (field == key).then_some(value))
}

fn assert_stdout_event(case: &TestCase, text: &str) {
    assert_stdio_event(case, "stdout", text);
}

fn assert_unique_paths(paths: &[PathBuf]) {
    let unique = paths.iter().collect::<BTreeSet<_>>();
    assert_eq!(
        unique.len(),
        paths.len(),
        "paths should be unique: {paths:?}"
    );
}

fn named_cases<const N: usize>(names: [&str; N]) -> Vec<TestCase> {
    names
        .into_iter()
        .enumerate()
        .map(|(index, name)| test_case(index + 1, name))
        .collect()
}

fn test_case(index: usize, name: &str) -> TestCase {
    let source = SourceFile::new("main_test.veln", format!("test {name}() -> ()\nend\n"));
    let span = source.span(TextRange::new(0, source.len()));
    TestCase {
        id: format!("case-{index}"),
        name: name.to_string(),
        kind: "test".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main_test.veln".to_string(),
            node_id: format!("test-{index}"),
            span,
        },
        reason: None,
        failure: None,
        expected_output: None,
        expected_runtime_failure: None,
        events: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn case_names<const N: usize>(cases: &[TestCase]) -> [&str; N] {
    case_names_of(cases)
}

fn case_names_of<const N: usize>(cases: &[TestCase]) -> [&str; N] {
    cases
        .iter()
        .map(|case| case.name.as_str())
        .collect::<Vec<_>>()
        .try_into()
        .expect("case count should match expected names")
}
