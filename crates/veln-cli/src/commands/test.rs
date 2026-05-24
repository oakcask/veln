use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::{ExitCode, Output};

use veln_ast::SurfaceModule;
use veln_backend_jvm::generate_java_with_entry;
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, JsonValue, diagnostic_to_json};
use veln_project::Project;
use veln_sema::{analyze_surface_module, lower_checked_surface_module};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{JavaRunResult, compile_and_run_java_capture, create_build_dir};
use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) fn test(json: bool, targets: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let project = Project::discover(root, &targets).map_err(|error| error.to_string())?;
    let (module, mut diagnostics) = load_surface_module(&project);
    let test_files = selected_test_files(&project, explicit);
    let mut cases = discover_test_cases(&module, &test_files);
    let mut suite_errors = Vec::new();

    if !has_error(&diagnostics) {
        diagnostics.extend(analyze_surface_module(&module));
    }

    if cases.is_empty() && !has_error(&diagnostics) {
        suite_errors.push(SuiteError {
            kind: "discovery".to_string(),
            message: "no zero-argument test functions were discovered".to_string(),
        });
    }

    if has_error(&diagnostics) {
        for case in &mut cases {
            case.status = TestCaseStatus::Blocked;
            case.reason = Some("static_gate".to_string());
        }
    } else if suite_errors.is_empty() {
        for case in &mut cases {
            run_test_case(&module, case)?;
        }
    }

    let report = TestReport {
        status: test_run_status(&cases, &diagnostics, &suite_errors),
        selection: TestSelection {
            mode_name: if explicit { "explicit" } else { "discovered" }.to_string(),
            targets: selection_targets(&project, &test_files),
            confidence: "complete".to_string(),
            reason: if explicit {
                "user_selected".to_string()
            } else {
                "pattern_discovery".to_string()
            },
        },
        diagnostics,
        suite_errors,
        cases,
    };

    if json {
        println!("{}", report.to_json());
    } else {
        print_test_human(&report)?;
    }

    Ok(if report.status == TestRunStatus::Passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn selected_test_files(project: &Project, explicit: bool) -> BTreeSet<String> {
    project
        .files
        .iter()
        .filter(|source| explicit || source.path().as_str().ends_with("_test.veln"))
        .map(|source| source.path().as_str().to_string())
        .collect()
}

fn discover_test_cases(module: &SurfaceModule, test_files: &BTreeSet<String>) -> Vec<TestCase> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.params.is_empty() && test_files.contains(function.span.file.as_str())
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
                node_id: function.node_id.display("fn"),
                span: function.span.clone(),
            },
            reason: None,
            failure: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        })
        .collect()
}

fn run_test_case(module: &SurfaceModule, case: &mut TestCase) -> Result<(), String> {
    let reachable_module = reachable_entry_module(module, &case.name);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        case.status = TestCaseStatus::Blocked;
        case.reason = Some("static_gate".to_string());
        case.diagnostics = lowered.diagnostics;
        return Ok(());
    };

    let java = generate_java_with_entry(&ir, &case.name);
    let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
    let result = compile_and_run_java_capture(&build_dir, &java, "veln test");
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }

    let output = match result? {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            case.status = TestCaseStatus::Error;
            case.reason = Some("runner_error".to_string());
            case.failure = Some(TestFailure {
                kind: "runtime".to_string(),
                message,
            });
            return Ok(());
        }
    };

    case.events = stdio_events_from_output(&output, &case.source);
    if output.status.success() {
        case.status = TestCaseStatus::Passed;
    } else {
        case.status = TestCaseStatus::Failed;
        case.failure = Some(TestFailure {
            kind: "runtime".to_string(),
            message: format!("test process exited with status {}", output.status),
        });
    }
    Ok(())
}

fn stdio_events_from_output(output: &Output, source: &TestCaseSource) -> Vec<JsonValue> {
    let mut events = Vec::new();
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        events.push(stdio_event(
            "stdout",
            stdout.as_ref(),
            events.len() + 1,
            source,
        ));
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        events.push(stdio_event(
            "stderr",
            stderr.as_ref(),
            events.len() + 1,
            source,
        ));
    }
    events
}

fn stdio_event(stream: &str, text: &str, sequence: usize, source: &TestCaseSource) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("stdio")),
        ("stream", JsonValue::string(stream)),
        ("operation", JsonValue::string("print")),
        ("text", JsonValue::string(text)),
        ("terminator", JsonValue::string("none")),
        ("sequence", JsonValue::Number(sequence as i64)),
        ("node_id", JsonValue::string(source.node_id.clone())),
        ("span", source_span_to_json(&source.span)),
    ])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestRunStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestRunStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestCaseStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestCaseStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

struct TestSelection {
    mode_name: String,
    targets: Vec<String>,
    confidence: String,
    reason: String,
}

struct TestReport {
    status: TestRunStatus,
    selection: TestSelection,
    diagnostics: Vec<Diagnostic>,
    suite_errors: Vec<SuiteError>,
    cases: Vec<TestCase>,
}

impl TestReport {
    fn to_json(&self) -> String {
        JsonValue::object([
            ("schema_version", JsonValue::string("veln-test-json/v0")),
            ("command", JsonValue::string("test")),
            ("status", JsonValue::string(self.status.as_str())),
            ("selection", self.selection.to_json()),
            (
                "summary",
                test_summary_to_json(&self.cases, &self.suite_errors),
            ),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
            (
                "suite_errors",
                JsonValue::array(self.suite_errors.iter().map(SuiteError::to_json)),
            ),
            (
                "cases",
                JsonValue::array(self.cases.iter().map(TestCase::to_json)),
            ),
        ])
        .to_json()
    }
}

impl TestSelection {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("mode", JsonValue::string(self.mode_name.clone())),
            (
                "targets",
                JsonValue::array(self.targets.iter().map(|target| JsonValue::string(target))),
            ),
            ("confidence", JsonValue::string(self.confidence.clone())),
            ("reason", JsonValue::string(self.reason.clone())),
        ])
    }
}

struct SuiteError {
    kind: String,
    message: String,
}

impl SuiteError {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
        ])
    }
}

struct TestCase {
    id: String,
    name: String,
    kind: String,
    status: TestCaseStatus,
    source: TestCaseSource,
    reason: Option<String>,
    failure: Option<TestFailure>,
    events: Vec<JsonValue>,
    diagnostics: Vec<Diagnostic>,
}

impl TestCase {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("id", JsonValue::string(self.id.clone())),
            ("name", JsonValue::string(self.name.clone())),
            ("kind", JsonValue::string(self.kind.clone())),
            ("status", JsonValue::string(self.status.as_str())),
            (
                "source",
                JsonValue::object([
                    ("file", JsonValue::string(self.source.file.clone())),
                    ("node_id", JsonValue::string(self.source.node_id.clone())),
                    ("span", source_span_range_to_json(&self.source.span)),
                ]),
            ),
            (
                "reason",
                self.reason
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ),
            (
                "failure",
                self.failure
                    .as_ref()
                    .map_or(JsonValue::Null, TestFailure::to_json),
            ),
            ("events", JsonValue::Array(self.events.clone())),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
        ])
    }
}

struct TestCaseSource {
    file: String,
    node_id: String,
    span: veln_source::SourceSpan,
}

struct TestFailure {
    kind: String,
    message: String,
}

impl TestFailure {
    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
            ("expected", JsonValue::Null),
            ("actual", JsonValue::Null),
            ("span", JsonValue::Null),
            (
                "details",
                JsonValue::object(Vec::<(String, JsonValue)>::new()),
            ),
        ])
    }
}

fn test_run_status(
    cases: &[TestCase],
    diagnostics: &[Diagnostic],
    suite_errors: &[SuiteError],
) -> TestRunStatus {
    if cases
        .iter()
        .any(|case| case.status == TestCaseStatus::Error)
    {
        TestRunStatus::Error
    } else if !suite_errors.is_empty() && cases.is_empty() {
        TestRunStatus::Blocked
    } else if has_error(diagnostics)
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

fn test_summary_to_json(cases: &[TestCase], suite_errors: &[SuiteError]) -> JsonValue {
    let count = |status| cases.iter().filter(|case| case.status == status).count() as i64;
    JsonValue::object([
        ("total", JsonValue::Number(cases.len() as i64)),
        ("passed", JsonValue::Number(count(TestCaseStatus::Passed))),
        ("failed", JsonValue::Number(count(TestCaseStatus::Failed))),
        ("skipped", JsonValue::Number(0)),
        ("todo", JsonValue::Number(0)),
        ("blocked", JsonValue::Number(count(TestCaseStatus::Blocked))),
        (
            "errors",
            JsonValue::Number(count(TestCaseStatus::Error) + suite_errors.len() as i64),
        ),
    ])
}

fn source_span_to_json(span: &veln_source::SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn source_span_range_to_json(span: &veln_source::SourceSpan) -> JsonValue {
    JsonValue::object([
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn line_col_to_json(line_col: veln_source::LineCol) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line_col.line as i64)),
        ("column", JsonValue::Number(line_col.column as i64)),
        ("offset", JsonValue::Number(line_col.offset as i64)),
    ])
}

fn selection_targets(project: &Project, test_files: &BTreeSet<String>) -> Vec<String> {
    project
        .files
        .iter()
        .filter_map(|source| {
            let path = source.path().as_str();
            test_files.contains(path).then(|| path.to_string())
        })
        .collect()
}

fn print_test_human(report: &TestReport) -> Result<(), String> {
    if !report.diagnostics.is_empty() {
        print_human_stderr(&DiagnosticEnvelope::new(
            tool_info(),
            report.diagnostics.clone(),
        ))?;
    }
    for suite_error in &report.suite_errors {
        eprintln!("veln: test {}: {}", suite_error.kind, suite_error.message);
    }
    for case in &report.cases {
        match case.status {
            TestCaseStatus::Passed => println!("ok {}", case.name),
            TestCaseStatus::Failed => println!("not ok {}", case.name),
            TestCaseStatus::Blocked => println!("blocked {}", case.name),
            TestCaseStatus::Error => println!("error {}", case.name),
        }
        for diagnostic in &case.diagnostics {
            print_human_stderr(&DiagnosticEnvelope::new(
                tool_info(),
                vec![diagnostic.clone()],
            ))?;
        }
        if let Some(failure) = &case.failure {
            eprintln!("veln: test `{}` failed: {}", case.name, failure.message);
        }
    }
    Ok(())
}
