//! Test discovery, test JSON, and captured events.
//!
//! ```rust
//! use veln_test as _;
//! ```

use std::collections::BTreeSet;
use std::process::Output;

use veln_ast::SurfaceModule;
use veln_diagnostics::{Diagnostic, JsonValue, Severity, diagnostic_to_json};
use veln_project::Project;
use veln_source::{LineCol, SourceSpan};

pub fn selected_test_files(project: &Project, explicit: bool) -> BTreeSet<String> {
    project
        .files
        .iter()
        .filter(|source| explicit || source.path().as_str().ends_with("_test.veln"))
        .map(|source| source.path().as_str().to_string())
        .collect()
}

pub fn selection_targets(project: &Project, test_files: &BTreeSet<String>) -> Vec<String> {
    project
        .files
        .iter()
        .filter_map(|source| {
            let path = source.path().as_str();
            test_files.contains(path).then(|| path.to_string())
        })
        .collect()
}

pub fn discover_test_cases(module: &SurfaceModule, test_files: &BTreeSet<String>) -> Vec<TestCase> {
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

pub fn stdio_events_from_output(output: &Output, source: &TestCaseSource) -> Vec<JsonValue> {
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

fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
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
pub enum TestRunStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestRunStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TestCaseStatus {
    Passed,
    Failed,
    Blocked,
    Error,
}

impl TestCaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Error => "error",
        }
    }
}

pub struct TestSelection {
    pub mode_name: String,
    pub targets: Vec<String>,
    pub confidence: String,
    pub reason: String,
}

impl TestSelection {
    pub fn new(project: &Project, test_files: &BTreeSet<String>, explicit: bool) -> Self {
        Self {
            mode_name: if explicit { "explicit" } else { "discovered" }.to_string(),
            targets: selection_targets(project, test_files),
            confidence: "complete".to_string(),
            reason: if explicit {
                "user_selected".to_string()
            } else {
                "pattern_discovery".to_string()
            },
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("mode", JsonValue::string(self.mode_name.clone())),
            (
                "targets",
                JsonValue::array(self.targets.iter().map(JsonValue::string)),
            ),
            ("confidence", JsonValue::string(self.confidence.clone())),
            ("reason", JsonValue::string(self.reason.clone())),
        ])
    }
}

pub struct TestReport {
    pub status: TestRunStatus,
    pub selection: TestSelection,
    pub diagnostics: Vec<Diagnostic>,
    pub suite_errors: Vec<SuiteError>,
    pub cases: Vec<TestCase>,
}

impl TestReport {
    pub fn new(
        selection: TestSelection,
        diagnostics: Vec<Diagnostic>,
        suite_errors: Vec<SuiteError>,
        cases: Vec<TestCase>,
    ) -> Self {
        let status = test_run_status(&cases, &diagnostics, &suite_errors);
        Self {
            status,
            selection,
            diagnostics,
            suite_errors,
            cases,
        }
    }

    pub fn to_json(&self) -> String {
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

pub struct SuiteError {
    pub kind: String,
    pub message: String,
}

impl SuiteError {
    pub fn discovery(message: impl Into<String>) -> Self {
        Self {
            kind: "discovery".to_string(),
            message: message.into(),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
        ])
    }
}

pub struct TestCase {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub status: TestCaseStatus,
    pub source: TestCaseSource,
    pub reason: Option<String>,
    pub failure: Option<TestFailure>,
    pub events: Vec<JsonValue>,
    pub diagnostics: Vec<Diagnostic>,
}

impl TestCase {
    pub fn to_json(&self) -> JsonValue {
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

pub struct TestCaseSource {
    pub file: String,
    pub node_id: String,
    pub span: SourceSpan,
}

pub struct TestFailure {
    pub kind: String,
    pub message: String,
}

impl TestFailure {
    pub fn to_json(&self) -> JsonValue {
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

fn source_span_to_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn source_span_range_to_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

fn line_col_to_json(line_col: LineCol) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line_col.line as i64)),
        ("column", JsonValue::Number(line_col.column as i64)),
        ("offset", JsonValue::Number(line_col.offset as i64)),
    ])
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::process::{ExitStatus, Output};

    use veln_ast::lower_surface_ast;
    use veln_project::Project;
    use veln_source::{SourceFile, TextRange};
    use veln_syntax::parse;

    use super::*;

    fn module(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main_test.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        lower_surface_ast(&parsed.tree)
    }

    #[test]
    fn discovered_selection_uses_test_file_pattern() {
        let project = Project {
            root: PathBuf::new(),
            files: vec![
                SourceFile::new("main.veln", ""),
                SourceFile::new("main_test.veln", ""),
            ],
        };

        let test_files = selected_test_files(&project, false);
        let selection = TestSelection::new(&project, &test_files, false);

        assert_eq!(selection.mode_name, "discovered");
        assert_eq!(selection.targets, vec!["main_test.veln"]);
        assert_eq!(selection.reason, "pattern_discovery");
    }

    #[test]
    fn discovers_zero_argument_functions_in_selected_files() {
        let module = module(concat!(
            "fn first()\n",
            "  ()\n",
            "end\n",
            "fn helper(value)\n",
            "  value\n",
            "end\n",
        ));
        let test_files = BTreeSet::from(["main_test.veln".to_string()]);

        let cases = discover_test_cases(&module, &test_files);

        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].id, "case-1");
        assert_eq!(cases[0].name, "first");
        assert_eq!(cases[0].source.file, "main_test.veln");
        assert_eq!(cases[0].source.node_id, "fn-1");
    }

    #[test]
    fn report_json_contains_summary_suite_errors_and_cases() {
        let module = module("fn first()\n  ()\nend\n");
        let test_files = BTreeSet::from(["main_test.veln".to_string()]);
        let cases = discover_test_cases(&module, &test_files);
        let report = TestReport::new(
            TestSelection {
                mode_name: "explicit".to_string(),
                targets: vec!["main_test.veln".to_string()],
                confidence: "complete".to_string(),
                reason: "user_selected".to_string(),
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
                "\"source\":{\"file\":\"main_test.veln\",\"node_id\":\"fn-1\",",
                "\"span\":{\"start\":{\"line\":1,\"column\":1,\"offset\":0},",
                "\"end\":{\"line\":4,\"column\":1,\"offset\":20}}},",
                "\"reason\":null,\"failure\":null,\"events\":[],",
                "\"diagnostics\":[]}]}"
            )
        );
    }

    #[test]
    fn stdio_events_preserve_stream_sequence_and_source() {
        let source_file = SourceFile::new("main_test.veln", "fn first()\n  ()\nend\n");
        let source = TestCaseSource {
            file: "main_test.veln".to_string(),
            node_id: "fn-1".to_string(),
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
                "\"node_id\":\"fn-1\",\"span\":{\"file\":\"main_test.veln\",",
                "\"start\":{\"line\":1,\"column\":1,\"offset\":0},",
                "\"end\":{\"line\":4,\"column\":1,\"offset\":20}}}"
            )
        );
        assert!(events[1].to_json().contains("\"sequence\":2"));
        assert!(events[1].to_json().contains("\"stream\":\"stderr\""));
    }

    #[cfg(unix)]
    fn exit_status(code: i32) -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;

        ExitStatus::from_raw(code)
    }

    #[cfg(windows)]
    fn exit_status(code: u32) -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;

        ExitStatus::from_raw(code)
    }
}
