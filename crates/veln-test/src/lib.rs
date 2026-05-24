//! Test discovery, test JSON, and captured events.
//!
//! ```rust
//! use veln_test as _;
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::process::Output;

use veln_ast::{BodyLineKind, Expr, ExprKind, FunctionKind, SurfaceModule};
use veln_diagnostics::{Diagnostic, JsonValue, Severity, diagnostic_to_json};
use veln_project::Project;
use veln_source::{LineCol, SourcePath, SourceSpan};

pub fn selected_test_files(
    project: &Project,
    module: &SurfaceModule,
    explicit: bool,
) -> BTreeSet<String> {
    project
        .files
        .iter()
        .filter(|source| explicit || source.path().as_str().ends_with("_test.veln"))
        .map(|source| source.path().as_str().to_string())
        .chain(
            (!explicit)
                .then(|| same_file_test_files(module))
                .into_iter()
                .flatten(),
        )
        .collect()
}

fn same_file_test_files(module: &SurfaceModule) -> BTreeSet<String> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Test)
        .map(|function| function.span.file.as_str().to_string())
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
            events: Vec::new(),
            diagnostics: Vec::new(),
        })
        .collect()
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

fn collect_stdio_call_spans(expr: &Expr, spans: &mut BTreeMap<(String, String), SourceSpan>) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if is_stdio_callee(callee) {
                spans.insert(
                    (
                        expr.span.file.as_str().to_string(),
                        expr.node_id.display("call"),
                    ),
                    expr.span.clone(),
                );
            }
            collect_stdio_call_spans(callee, spans);
            for arg in args {
                collect_stdio_call_spans(arg, spans);
            }
        }
        ExprKind::FieldAccess { base, .. } => collect_stdio_call_spans(base, spans),
        ExprKind::Try(inner) => collect_stdio_call_spans(inner, spans),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_stdio_call_spans(&field.expr, spans);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_stdio_call_spans(&entry.key, spans);
                collect_stdio_call_spans(&entry.value, spans);
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_stdio_call_spans(item, spans);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_stdio_call_spans(scrutinee, spans);
            for arm in arms {
                collect_stdio_call_spans(&arm.expr, spans);
            }
        }
        ExprKind::Prefix { expr, .. } => collect_stdio_call_spans(expr, spans),
        ExprKind::Binary { left, right, .. } => {
            collect_stdio_call_spans(left, spans);
            collect_stdio_call_spans(right, spans);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath(_)
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn is_stdio_callee(expr: &Expr) -> bool {
    matches!(
        &expr.kind,
        ExprKind::NamePath(segments)
            if matches!(
                segments.as_slice(),
                [module, name]
                    if module == "stdio"
                        && matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln")
            )
    )
}

fn stdio_event_from_trace_line(
    line: &str,
    call_spans: &BTreeMap<(String, String), SourceSpan>,
    fallback_source: &TestCaseSource,
) -> Option<JsonValue> {
    let mut fields = line.splitn(7, '\t');
    let sequence = fields.next()?.parse::<usize>().ok()?;
    let stream = fields.next()?;
    let operation = fields.next()?;
    let terminator = fields.next()?;
    let node_id = fields.next()?;
    let source_file = fields.next()?;
    let text = decode_hex_text(fields.next()?)?;
    let node_id = if node_id.is_empty() {
        fallback_source.node_id.as_str()
    } else {
        node_id
    };
    let source_file = if source_file.is_empty() {
        fallback_source.file.as_str()
    } else {
        source_file
    };
    let span = call_spans
        .get(&(source_file.to_string(), node_id.to_string()))
        .unwrap_or(&fallback_source.span);
    Some(stdio_event(
        stream, operation, &text, terminator, sequence, node_id, span,
    ))
}

fn decode_hex_text(hex: &str) -> Option<String> {
    if hex.len() % 2 != 0 {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    let mut chars = hex.chars();
    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        bytes.push((hex_digit(high)? << 4) | hex_digit(low)?);
    }
    String::from_utf8(bytes).ok()
}

fn contract_failure_from_trace_line(line: &str) -> Option<TestFailure> {
    let mut fields = line.split('\t');
    if fields.next()? != "contract" {
        return None;
    }
    let clause = fields.next()?.to_string();
    let predicate = decode_hex_text(fields.next()?)?;
    let function = decode_hex_text(fields.next()?)?;
    let blame = fields.next()?.to_string();
    let node_id = decode_hex_text(fields.next()?)?;
    let source_file = decode_hex_text(fields.next()?)?;
    let start_line = fields.next()?.parse::<usize>().ok()?;
    let start_column = fields.next()?.parse::<usize>().ok()?;
    let end_line = fields.next()?.parse::<usize>().ok()?;
    let end_column = fields.next()?.parse::<usize>().ok()?;
    let span = SourceSpan {
        file: SourcePath::new(source_file),
        start: LineCol {
            line: start_line,
            column: start_column,
            offset: 0,
        },
        end: LineCol {
            line: end_line,
            column: end_column,
            offset: 0,
        },
    };
    let message = format!("contract failure: {clause} `{predicate}` in `{function}` blame {blame}");
    Some(TestFailure::contract(
        message, clause, predicate, function, blame, node_id, span,
    ))
}

fn hex_digit(character: char) -> Option<u8> {
    match character {
        '0'..='9' => Some(character as u8 - b'0'),
        'a'..='f' => Some(character as u8 - b'a' + 10),
        'A'..='F' => Some(character as u8 - b'A' + 10),
        _ => None,
    }
}

fn stdio_event(
    stream: &str,
    operation: &str,
    text: &str,
    terminator: &str,
    sequence: usize,
    node_id: &str,
    span: &SourceSpan,
) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("stdio")),
        ("stream", JsonValue::string(stream)),
        ("operation", JsonValue::string(operation)),
        ("text", JsonValue::string(text)),
        ("terminator", JsonValue::string(terminator)),
        ("sequence", JsonValue::Number(sequence as i64)),
        ("node_id", JsonValue::string(node_id)),
        ("span", source_span_to_json(span)),
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
    pub notes: Vec<String>,
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
            notes: Vec::new(),
        }
    }

    pub fn source_to_test_convention(mut self, added_count: usize) -> Self {
        if added_count > 0 {
            self.confidence = "partial".to_string();
            self.reason = "source_to_test_convention".to_string();
            let noun = if added_count == 1 { "file" } else { "files" };
            self.notes.push(format!(
                "added {added_count} test {noun} by source-to-test convention"
            ));
        }
        self
    }

    fn to_json(&self) -> JsonValue {
        let mut fields = vec![
            ("mode", JsonValue::string(self.mode_name.clone())),
            (
                "targets",
                JsonValue::array(self.targets.iter().map(JsonValue::string)),
            ),
            ("confidence", JsonValue::string(self.confidence.clone())),
            ("reason", JsonValue::string(self.reason.clone())),
        ];
        if !self.notes.is_empty() {
            fields.push((
                "notes",
                JsonValue::array(self.notes.iter().map(JsonValue::string)),
            ));
        }
        JsonValue::object(fields)
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
        let module = SurfaceModule {
            module: None,
            uses: Vec::new(),
            functions: Vec::new(),
        };
        let project = Project {
            root: PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new("main.veln", ""),
                SourceFile::new("main_test.veln", ""),
            ],
        };

        let test_files = selected_test_files(&project, &module, false);
        let selection = TestSelection::new(&project, &test_files, false);

        assert_eq!(selection.mode_name, "discovered");
        assert_eq!(selection.targets, vec!["main_test.veln"]);
        assert_eq!(selection.reason, "pattern_discovery");
    }

    #[test]
    fn discovered_selection_includes_same_file_test_declarations() {
        let source = SourceFile::new(
            "main.veln",
            "test same_file() -> () effects []\n  ()\nend\n",
        );
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        let module = lower_surface_ast(&parsed.tree);
        let project = Project {
            root: PathBuf::new(),
            manifest: None,
            files: vec![source],
        };

        let test_files = selected_test_files(&project, &module, false);
        let selection = TestSelection::new(&project, &test_files, false);

        assert_eq!(selection.targets, vec!["main.veln"]);
        assert_eq!(selection.reason, "pattern_discovery");
    }

    #[test]
    fn discovers_test_declarations_in_selected_files() {
        let module = module(concat!(
            "test first() -> () effects []\n",
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
    fn ordinary_zero_argument_functions_are_not_test_cases() {
        let module = module("fn helper()\n  ()\nend\n");
        let test_files = BTreeSet::from(["main_test.veln".to_string()]);

        let cases = discover_test_cases(&module, &test_files);

        assert!(cases.is_empty());
    }

    #[test]
    fn report_json_contains_summary_suite_errors_and_cases() {
        let module = module("test first() -> () effects []\n  ()\nend\n");
        let test_files = BTreeSet::from(["main_test.veln".to_string()]);
        let cases = discover_test_cases(&module, &test_files);
        let report = TestReport::new(
            TestSelection {
                mode_name: "explicit".to_string(),
                targets: vec!["main_test.veln".to_string()],
                confidence: "complete".to_string(),
                reason: "user_selected".to_string(),
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
                "\"end\":{\"line\":4,\"column\":1,\"offset\":39}}},",
                "\"reason\":null,\"failure\":null,\"events\":[],",
                "\"diagnostics\":[]}]}"
            )
        );
    }

    #[test]
    fn stdio_events_preserve_stream_sequence_and_source() {
        let source_file = SourceFile::new(
            "main_test.veln",
            "test first() -> () effects []\n  ()\nend\n",
        );
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
                "\"end\":{\"line\":4,\"column\":1,\"offset\":39}}}"
            )
        );
        assert!(events[1].to_json().contains("\"sequence\":2"));
        assert!(events[1].to_json().contains("\"stream\":\"stderr\""));
    }

    #[test]
    fn stdio_trace_events_preserve_operation_terminator_and_call_span() {
        let module = module(concat!(
            "test first() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprint(\"err\")\n",
            "  ()\n",
            "end\n",
        ));
        let call_spans = stdio_call_spans(&module);
        let call_keys = call_spans.keys().cloned().collect::<Vec<_>>();
        let call_ids = call_keys
            .iter()
            .map(|(_, node_id)| node_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(call_ids.len(), 2);
        let source = TestCaseSource {
            file: "main_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: module.functions[0].span.clone(),
        };
        let trace = format!(
            "1\tstdout\tprintln\tnewline\t{}\t{}\t6f7574\n2\tstderr\teprint\tnone\t{}\t{}\t657272\n",
            call_keys[0].1, call_keys[0].0, call_keys[1].1, call_keys[1].0
        );

        let events = stdio_events_from_trace(&trace, &call_spans, &source);

        assert_eq!(events.len(), 2);
        let first_event = events[0].to_json();
        assert!(first_event.contains("\"operation\":\"println\""));
        assert!(first_event.contains("\"text\":\"out\""));
        assert!(first_event.contains("\"terminator\":\"newline\""));
        assert!(first_event.contains(&format!("\"node_id\":\"{}\"", call_ids[0])));
        assert!(first_event.contains("\"file\":\"main_test.veln\""));
        assert!(first_event.contains("\"start\":{\"line\":2,\"column\":3"));
        assert!(events[1].to_json().contains("\"operation\":\"eprint\""));
        assert!(events[1].to_json().contains("\"terminator\":\"none\""));
    }

    #[test]
    fn contract_trace_becomes_structured_test_failure() {
        let trace = "contract\trequire\t66616c7365\t72656a65637473\tcaller\t636f6e74726163742d32\t6d61696e5f746573742e76656c6e\t2\t1\t2\t14\n";

        let failure = contract_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "contract");
        assert_eq!(
            failure.message,
            "contract failure: require `false` in `rejects` blame caller"
        );
        assert_eq!(
            failure.to_json().to_json(),
            concat!(
                "{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `rejects` blame caller\",",
                "\"expected\":null,\"actual\":null,\"span\":null,",
                "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",",
                "\"clause\":\"require\",\"predicate\":\"false\",\"function\":\"rejects\",",
                "\"blame\":\"caller\",\"node_id\":\"contract-2\",",
                "\"span\":{\"file\":\"main_test.veln\",",
                "\"start\":{\"line\":2,\"column\":1,\"offset\":0},",
                "\"end\":{\"line\":2,\"column\":14,\"offset\":0}}}}"
            )
        );
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
