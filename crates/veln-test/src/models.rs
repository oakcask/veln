use super::*;

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

    pub(super) fn to_json(&self) -> JsonValue {
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
    pub expected_output: Option<ExpectedOutput>,
    pub expected_runtime_failure: Option<ExpectedRuntimeFailure>,
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
    pub details: JsonValue,
}

impl TestFailure {
    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            kind: "runtime".to_string(),
            message: message.into(),
            details: JsonValue::object(Vec::<(String, JsonValue)>::new()),
        }
    }

    pub fn contract(
        message: String,
        clause: String,
        predicate: String,
        function: String,
        blame: String,
        node_id: String,
        span: SourceSpan,
    ) -> Self {
        Self {
            kind: "contract".to_string(),
            message,
            details: JsonValue::object([
                ("kind", JsonValue::string("contract")),
                ("phase", JsonValue::string("runtime")),
                ("clause", JsonValue::string(clause)),
                ("predicate", JsonValue::string(predicate)),
                ("function", JsonValue::string(function)),
                ("blame", JsonValue::string(blame)),
                ("node_id", JsonValue::string(node_id)),
                ("span", source_span_to_json(&span)),
            ]),
        }
    }

    pub fn result(value: String, fixture_hex: Option<JsonValue>) -> Self {
        Self::result_with_details(value, fixture_hex, None, None)
    }

    pub fn result_with_details(
        value: String,
        fixture_hex: Option<JsonValue>,
        byte_diagnostic: Option<JsonValue>,
        protocol_diagnostic: Option<JsonValue>,
    ) -> Self {
        Self::result_with_extended_details(
            value,
            fixture_hex,
            byte_diagnostic,
            None,
            protocol_diagnostic,
        )
    }

    pub(super) fn result_with_extended_details(
        value: String,
        fixture_hex: Option<JsonValue>,
        byte_diagnostic: Option<JsonValue>,
        value_diagnostic: Option<JsonValue>,
        protocol_diagnostic: Option<JsonValue>,
    ) -> Self {
        let mut details = vec![
            ("kind", JsonValue::string("result")),
            ("phase", JsonValue::string("runtime")),
            ("value", JsonValue::string(value.clone())),
        ];
        if let Some(fixture_hex) = fixture_hex {
            details.push(("fixture_hex", fixture_hex));
        }
        if let Some(byte_diagnostic) = byte_diagnostic {
            details.push(("byte_diagnostic", byte_diagnostic));
        }
        if let Some(value_diagnostic) = value_diagnostic {
            details.push(("value_diagnostic", value_diagnostic));
        }
        if let Some(protocol_diagnostic) = protocol_diagnostic {
            details.push(("protocol_diagnostic", protocol_diagnostic));
        }
        Self {
            kind: "result".to_string(),
            message: format!("runtime result failure: Err({value})"),
            details: JsonValue::object(details),
        }
    }

    pub fn output_mismatch(
        stream: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
        first_difference: OutputDifference,
        expected_span: Option<SourceSpan>,
        actual_events: Vec<JsonValue>,
    ) -> Self {
        let stream = stream.into();
        let expected = expected.into();
        let actual = actual.into();
        let mut details = vec![
            ("kind", JsonValue::string("output")),
            ("stream", JsonValue::string(stream.clone())),
            ("expected", JsonValue::string(expected)),
            ("actual", JsonValue::string(actual)),
            ("first_difference", first_difference.to_json()),
            ("actual_events", JsonValue::Array(actual_events)),
        ];
        if let Some(span) = expected_span {
            details.push(("expected_span", source_span_to_json(&span)));
        }
        Self {
            kind: "output".to_string(),
            message: format!("expected {stream} output did not match"),
            details: JsonValue::object(details),
        }
    }

    pub fn runtime_expectation(
        message: impl Into<String>,
        expected: &ExpectedRuntimeFailure,
        actual: Option<TestFailure>,
    ) -> Self {
        Self {
            kind: "runtime_expectation".to_string(),
            message: message.into(),
            details: JsonValue::object([
                ("kind", JsonValue::string("runtime_expectation")),
                ("expected", expected.to_json()),
                (
                    "actual",
                    actual.map_or(JsonValue::Null, |failure| failure.to_json()),
                ),
            ]),
        }
    }

    pub fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("kind", JsonValue::string(self.kind.clone())),
            ("message", JsonValue::string(self.message.clone())),
            ("expected", JsonValue::Null),
            ("actual", JsonValue::Null),
            ("span", JsonValue::Null),
            ("details", self.details.clone()),
        ])
    }
}

pub struct DoctestSources {
    pub sources: Vec<SourceFile>,
    pub expectations: BTreeMap<String, DoctestExpectation>,
    pub expected_failures: BTreeMap<String, SourceSpan>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct DoctestExpectation {
    pub expected_output: Option<ExpectedOutput>,
    pub expected_runtime_failure: Option<ExpectedRuntimeFailure>,
}

#[derive(Clone, Debug, Default)]
pub struct ExpectedOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_span: Option<SourceSpan>,
    pub stderr_span: Option<SourceSpan>,
}

#[derive(Clone, Debug)]
pub struct VisibleDoctests {
    pub doctests: Vec<VisibleDoctest>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug)]
pub struct VisibleDoctest {
    pub code: String,
    pub expected_error: Option<String>,
    pub should_fail: bool,
    pub expected_output: Option<ExpectedOutput>,
}

pub struct OutputDifference {
    pub line: usize,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl OutputDifference {
    pub(super) fn to_json(&self) -> JsonValue {
        JsonValue::object([
            ("line", JsonValue::Number(self.line as i64)),
            (
                "expected",
                self.expected
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ),
            (
                "actual",
                self.actual
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ),
        ])
    }
}

#[derive(Default)]
pub(super) struct ExtractedDoctest {
    pub(super) code: Vec<String>,
    pub(super) visible_code: Vec<String>,
    pub(super) error_type: Option<String>,
    pub(super) expected_output: Option<ExpectedOutput>,
    pub(super) expected_runtime_failure: Option<ExpectedRuntimeFailure>,
    pub(super) should_fail: bool,
    pub(super) fail_span: Option<SourceSpan>,
}

pub(super) enum Fence {
    Veln {
        lines: Vec<String>,
        visible_lines: Vec<String>,
        error_type: Option<String>,
        expected_runtime_failure: Option<Box<ExpectedRuntimeFailure>>,
        ignored: bool,
        should_fail: bool,
        fail_span: Option<SourceSpan>,
    },
    Output {
        stream: String,
        lines: Vec<String>,
        span: SourceSpan,
    },
    Ignored,
}

#[derive(Default)]
pub(super) struct ExtractedDoctests {
    pub(super) doctests: Vec<ExtractedDoctest>,
    pub(super) diagnostics: Vec<Diagnostic>,
}
