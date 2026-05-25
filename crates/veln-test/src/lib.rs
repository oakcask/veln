//! Test discovery, test JSON, and captured events.
//!
//! ```rust
//! use veln_test as _;
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Output;

use veln_ast::{BodyLineKind, Expr, ExprKind, FunctionKind, SurfaceModule};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity, diagnostic_to_json};
use veln_project::Project;
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan, TextRange};

pub struct TestTargetExpansion {
    pub targets: Vec<PathBuf>,
    pub source_to_test_added_count: usize,
}

pub fn expand_test_targets(root: &Path, targets: &[PathBuf]) -> TestTargetExpansion {
    if targets.is_empty() {
        return TestTargetExpansion {
            targets: Vec::new(),
            source_to_test_added_count: 0,
        };
    }

    let mut original_targets = targets.to_vec();
    original_targets.sort();
    original_targets.dedup();
    let original_count = original_targets.len();
    let mut expanded = targets.to_vec();
    for target in targets {
        if let Some(test_target) = paired_test_target(root, target) {
            expanded.push(test_target);
        }
    }
    expanded.sort();
    expanded.dedup();
    let source_to_test_added_count = expanded.len().saturating_sub(original_count);
    TestTargetExpansion {
        targets: expanded,
        source_to_test_added_count,
    }
}

fn paired_test_target(root: &Path, target: &Path) -> Option<PathBuf> {
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    if absolute.is_dir()
        || absolute
            .extension()
            .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    let file_name = absolute.file_name()?.to_str()?;
    if file_name.ends_with("_test.veln") {
        return None;
    }
    let stem = absolute.file_stem()?.to_str()?;
    let candidate = absolute.with_file_name(format!("{stem}_test.veln"));
    if !candidate.is_file() {
        return None;
    }
    if target.is_absolute() {
        Some(candidate)
    } else {
        candidate.strip_prefix(root).map_or_else(
            |_| Some(candidate.clone()),
            |relative| Some(relative.to_path_buf()),
        )
    }
}

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
    let mut targets = BTreeSet::new();
    project
        .files
        .iter()
        .filter(|source| {
            let path = source.path().as_str();
            test_files.contains(path)
        })
        .for_each(|source| {
            targets.insert(selection_target_path(source.path().as_str()).to_string());
        });
    targets.into_iter().collect()
}

fn selection_target_path(path: &str) -> &str {
    path.split_once("#doctest-")
        .map_or(path, |(origin, _)| origin)
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
            expected_output: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        })
        .collect()
}

pub fn attach_expected_outputs(
    cases: &mut [TestCase],
    expected_outputs: &BTreeMap<String, ExpectedOutput>,
) {
    for case in cases {
        if let Some(expected_output) = expected_outputs.get(&case.name) {
            case.kind = "doctest".to_string();
            case.expected_output = Some(expected_output.clone());
        }
    }
}

pub fn doctest_sources(sources: &[SourceFile]) -> DoctestSources {
    let mut generated_sources = Vec::new();
    let mut expected_outputs = BTreeMap::new();
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
                expected_outputs.insert(name, doctest.expected_output);
            }
            next_index += 1;
        }
    }

    DoctestSources {
        sources: generated_sources,
        expected_outputs,
        expected_failures,
        diagnostics,
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
        if let Some(span) = &diagnostic.span {
            if expected_failures.contains_key(span.file.as_str()) {
                matched.insert(span.file.as_str().to_string());
                continue;
            }
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
            "negative doctest produced no diagnostics",
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
        ExprKind::TypeApply { callee, .. } => collect_stdio_call_spans(callee, spans),
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
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn is_stdio_callee(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::NamePath(segments) => matches!(
            segments.as_slice(),
            [module, name]
                if module == "stdio"
                    && matches!(name.as_str(), "print" | "println" | "eprint" | "eprintln")
        ),
        ExprKind::TypeApply { callee, .. } => is_stdio_callee(callee),
        _ => false,
    }
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
    pub expected_output: Option<ExpectedOutput>,
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
    pub expected_outputs: BTreeMap<String, ExpectedOutput>,
    pub expected_failures: BTreeMap<String, SourceSpan>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, Default)]
pub struct ExpectedOutput {
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_span: Option<SourceSpan>,
    pub stderr_span: Option<SourceSpan>,
}

pub struct OutputDifference {
    pub line: usize,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

impl OutputDifference {
    fn to_json(&self) -> JsonValue {
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
struct ExtractedDoctest {
    code: Vec<String>,
    error_type: Option<String>,
    expected_output: ExpectedOutput,
    should_fail: bool,
    fail_span: Option<SourceSpan>,
}

enum Fence {
    Veln {
        lines: Vec<String>,
        error_type: Option<String>,
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
struct ExtractedDoctests {
    doctests: Vec<ExtractedDoctest>,
    diagnostics: Vec<Diagnostic>,
}

fn extract_doctests(
    source: &SourceFile,
    signatures: &BTreeMap<String, Option<String>>,
) -> ExtractedDoctests {
    let mut doctests = Vec::new();
    let mut diagnostics = Vec::new();
    let mut pending: Option<ExtractedDoctest> = None;
    let mut fence: Option<Fence> = None;
    let mut offset = 0;

    for raw_line in source.text().split_inclusive('\n') {
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));
        let line_range = TextRange::new(offset, offset + line.len());
        offset += raw_line.len();
        let Some(content) = doc_comment_content(line) else {
            if let Some(doctest) = pending.take() {
                doctests.push(with_error_type_context(doctest, line, signatures));
            }
            continue;
        };
        let content = content.strip_prefix(' ').unwrap_or(content);
        if let Some(active) = &mut fence {
            if content.trim_start().starts_with("```") {
                match fence.take().expect("active fence should exist") {
                    Fence::Veln {
                        lines,
                        error_type,
                        ignored,
                        should_fail,
                        fail_span,
                    } => {
                        if let Some(doctest) = pending.take() {
                            doctests.push(doctest);
                        }
                        if !ignored {
                            pending = Some(ExtractedDoctest {
                                code: lines,
                                error_type,
                                expected_output: ExpectedOutput::default(),
                                should_fail,
                                fail_span,
                            });
                        }
                    }
                    Fence::Output {
                        stream,
                        lines,
                        span,
                    } => {
                        if let Some(doctest) = &mut pending {
                            let output = lines.join("\n");
                            if stream == "stdout" {
                                if let Some(first_span) = &doctest.expected_output.stdout_span {
                                    diagnostics.push(duplicate_output_diagnostic(
                                        &stream, &span, first_span,
                                    ));
                                } else {
                                    doctest.expected_output.stdout = Some(output);
                                    doctest.expected_output.stdout_span = Some(span);
                                }
                            } else if stream == "stderr" {
                                if let Some(first_span) = &doctest.expected_output.stderr_span {
                                    diagnostics.push(duplicate_output_diagnostic(
                                        &stream, &span, first_span,
                                    ));
                                } else {
                                    doctest.expected_output.stderr = Some(output);
                                    doctest.expected_output.stderr_span = Some(span);
                                }
                            }
                        }
                    }
                    Fence::Ignored => {}
                }
                continue;
            }
            match active {
                Fence::Veln { lines, .. } => lines.push(doctest_code_line(content)),
                Fence::Output { lines, .. } => lines.push(content.to_string()),
                Fence::Ignored => {}
            }
            continue;
        }

        let trimmed = content.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            let info = info.trim();
            if veln_fence_info(info) {
                diagnostics.extend(veln_metadata_diagnostics(info, source.span(line_range)));
                fence = Some(Fence::Veln {
                    lines: Vec::new(),
                    error_type: doctest_error_type(info).map(ToString::to_string),
                    ignored: doctest_ignored(info),
                    should_fail: doctest_should_fail(info),
                    fail_span: doctest_should_fail(info).then(|| source.span(line_range)),
                });
            } else if output_fence_info(info) {
                let span = source.span(line_range);
                diagnostics.extend(output_metadata_diagnostics(info, span.clone()));
                if let Some(stream) = output_fence_stream(info) {
                    fence = Some(Fence::Output {
                        stream: stream.to_string(),
                        lines: Vec::new(),
                        span,
                    });
                } else {
                    fence = Some(Fence::Ignored);
                }
            } else if let Some(doctest) = pending.take() {
                doctests.push(doctest);
            }
        } else if !trimmed.is_empty() {
            if let Some(doctest) = pending.take() {
                doctests.push(doctest);
            }
        }
    }

    if let Some(doctest) = pending {
        doctests.push(doctest);
    }
    ExtractedDoctests {
        doctests,
        diagnostics,
    }
}

fn duplicate_output_diagnostic(
    stream: &str,
    duplicate_span: &SourceSpan,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "doctest.duplicate_output",
        Severity::Error,
        DiagnosticKind::Doc,
        format!("duplicate expected {stream} output fence"),
        Some(duplicate_span.clone()),
        JsonValue::object([
            ("kind", JsonValue::string("doctest_metadata")),
            ("stream", JsonValue::string(stream)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!("First expected {stream} output fence is here.")),
        ),
        ("span", source_span_to_json(first_span)),
    ]));
    diagnostic
}

fn doc_comment_content(line: &str) -> Option<&str> {
    line.trim_start().strip_prefix("///")
}

fn doctest_code_line(content: &str) -> String {
    content.strip_prefix("# ").unwrap_or(content).to_string()
}

fn veln_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln")
}

fn doctest_error_type(info: &str) -> Option<&str> {
    info.split_whitespace()
        .skip(1)
        .find_map(|field| field.strip_prefix("error="))
        .filter(|value| !value.is_empty())
}

fn doctest_ignored(info: &str) -> bool {
    info.split_whitespace()
        .skip(1)
        .any(|field| field == "ignore")
}

fn doctest_should_fail(info: &str) -> bool {
    info.split_whitespace().skip(1).any(|field| field == "fail")
}

fn output_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln-output")
}

fn output_fence_stream(info: &str) -> Option<&str> {
    let mut fields = info.split_whitespace();
    if fields.next()? != "veln-output" {
        return None;
    }
    let stream = fields.find_map(|field| field.strip_prefix("stream="))?;
    matches!(stream, "stdout" | "stderr").then_some(stream)
}

fn veln_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
    info.split_whitespace()
        .skip(1)
        .filter_map(|field| {
            if field.starts_with("error=") {
                field
                    .strip_prefix("error=")
                    .is_some_and(|value| value.is_empty())
                    .then(|| {
                        doctest_metadata_diagnostic(
                            "doctest.invalid_metadata",
                            "empty doctest error type",
                            span.clone(),
                            JsonValue::object([
                                ("kind", JsonValue::string("doctest_metadata")),
                                ("attribute", JsonValue::string("error")),
                            ]),
                        )
                    })
            } else if matches!(field, "ignore" | "fail") {
                None
            } else {
                Some(doctest_metadata_diagnostic(
                    "doctest.unknown_metadata",
                    format!(
                        "unknown doctest attribute `{}`",
                        metadata_attribute_name(field)
                    ),
                    span.clone(),
                    JsonValue::object([
                        ("kind", JsonValue::string("doctest_metadata")),
                        (
                            "attribute",
                            JsonValue::string(metadata_attribute_name(field)),
                        ),
                        ("fence", JsonValue::string("veln")),
                    ]),
                ))
            }
        })
        .collect()
}

fn output_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut has_stream = false;
    for field in info.split_whitespace().skip(1) {
        if let Some(stream) = field.strip_prefix("stream=") {
            has_stream = true;
            if !matches!(stream, "stdout" | "stderr") {
                diagnostics.push(doctest_metadata_diagnostic(
                    "doctest.invalid_metadata",
                    format!("unknown doctest output stream `{stream}`"),
                    span.clone(),
                    JsonValue::object([
                        ("kind", JsonValue::string("doctest_metadata")),
                        ("attribute", JsonValue::string("stream")),
                        ("stream", JsonValue::string(stream)),
                    ]),
                ));
            }
        } else {
            diagnostics.push(doctest_metadata_diagnostic(
                "doctest.unknown_metadata",
                format!(
                    "unknown doctest output attribute `{}`",
                    metadata_attribute_name(field)
                ),
                span.clone(),
                JsonValue::object([
                    ("kind", JsonValue::string("doctest_metadata")),
                    (
                        "attribute",
                        JsonValue::string(metadata_attribute_name(field)),
                    ),
                    ("fence", JsonValue::string("veln-output")),
                ]),
            ));
        }
    }
    if !has_stream {
        diagnostics.push(doctest_metadata_diagnostic(
            "doctest.invalid_metadata",
            "missing doctest output stream",
            span,
            JsonValue::object([
                ("kind", JsonValue::string("doctest_metadata")),
                ("attribute", JsonValue::string("stream")),
            ]),
        ));
    }
    diagnostics
}

fn doctest_metadata_diagnostic(
    id: &str,
    message: impl Into<String>,
    span: SourceSpan,
    details: JsonValue,
) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Doc,
        message,
        Some(span),
        details,
    )
}

fn metadata_attribute_name(field: &str) -> &str {
    field.split_once('=').map_or(field, |(name, _)| name)
}

fn with_error_type_context(
    mut doctest: ExtractedDoctest,
    line: &str,
    signatures: &BTreeMap<String, Option<String>>,
) -> ExtractedDoctest {
    if doctest.error_type.is_none() && doctest.code.iter().any(|line| line.contains('?')) {
        doctest.error_type = documented_result_error_type(line)
            .or_else(|| inferred_doctest_error_type(&doctest.code, signatures));
    }
    doctest
}

fn result_error_signatures(sources: &[SourceFile]) -> BTreeMap<String, Option<String>> {
    let mut signatures = BTreeMap::<String, Option<String>>::new();
    for source in sources {
        for line in source.text().lines() {
            let line = line.trim_start();
            let Some(name) = function_name(line) else {
                continue;
            };
            let Some(error_type) = function_result_error_type(line) else {
                continue;
            };
            signatures
                .entry(name.to_string())
                .and_modify(|existing| {
                    if existing.as_deref() != Some(error_type) {
                        *existing = None;
                    }
                })
                .or_insert_with(|| Some(error_type.to_string()));
        }
    }
    signatures
}

fn function_name(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("fn "))?;
    let open = rest.find('(')?;
    let name = rest[..open].trim();
    (!name.is_empty()).then_some(name)
}

fn function_result_error_type(line: &str) -> Option<&str> {
    let return_text = line.split_once("->")?.1;
    let return_text = return_text
        .split_once(" effects ")
        .map_or(return_text, |(return_text, _)| return_text)
        .trim();
    let return_text = strip_result_binding(return_text);
    result_error_type(return_text)
}

fn inferred_doctest_error_type(
    code: &[String],
    signatures: &BTreeMap<String, Option<String>>,
) -> Option<String> {
    let mut inferred = None::<String>;
    let mut found_try = false;
    for line in code {
        for callee in propagated_call_names(line) {
            found_try = true;
            let error_type = signatures.get(callee).and_then(|value| value.as_deref())?;
            if inferred
                .as_deref()
                .is_some_and(|existing| existing != error_type)
            {
                return None;
            }
            inferred.get_or_insert_with(|| error_type.to_string());
        }
    }
    found_try.then_some(inferred).flatten()
}

fn propagated_call_names(line: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for (index, ch) in line.char_indices() {
        if ch != '?' {
            continue;
        }
        let Some(name) = propagated_call_name(&line[..index]) else {
            names.push("");
            continue;
        };
        names.push(name);
    }
    names
}

fn propagated_call_name(text: &str) -> Option<&str> {
    let text = text.trim_end();
    if !text.ends_with(')') {
        return None;
    }
    let open = matching_open_paren(text)?;
    let before_open = text[..open].trim_end();
    let start = before_open
        .rfind(|ch: char| !(ch == '_' || ch == ':' || ch.is_ascii_alphanumeric()))
        .map_or(0, |index| index + 1);
    let name = &before_open[start..];
    (!name.is_empty()).then_some(name)
}

fn matching_open_paren(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn documented_result_error_type(line: &str) -> Option<String> {
    let line = line.trim_start();
    if !line.starts_with("pub fn ") {
        return None;
    }
    function_result_error_type(line).map(ToString::to_string)
}

fn strip_result_binding(return_text: &str) -> &str {
    let Some((binding, ty)) = return_text.split_once(':') else {
        return return_text;
    };
    if binding
        .trim()
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        ty.trim_start()
    } else {
        return_text
    }
}

fn result_error_type(ty: &str) -> Option<&str> {
    let ty = ty.trim();
    let args = ty.strip_prefix("Result(")?.strip_suffix(')')?;
    let comma = top_level_comma(args)?;
    let error = args[comma + 1..].trim();
    (!error.is_empty()).then_some(error)
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn generated_doctest_source(name: &str, doctest: &ExtractedDoctest) -> String {
    let return_type = doctest.error_type.as_ref().map_or_else(
        || "()".to_string(),
        |error_type| format!("Result((), {error_type})"),
    );
    let item_kind = if doctest.should_fail { "fn" } else { "test" };
    let mut text = format!("{item_kind} {name}() -> {return_type} effects [stdio]\n");
    for line in &doctest.code {
        if line.is_empty() {
            text.push('\n');
        } else {
            text.push_str("  ");
            text.push_str(line);
            text.push('\n');
        }
    }
    if doctest.error_type.is_some() {
        text.push_str("  Ok(())\nend\n");
    } else {
        text.push_str("  ()\nend\n");
    }
    text
}

fn reconstructed_stream(events: &[JsonValue], stream: &str) -> String {
    let mut text = String::new();
    for event in events {
        let JsonValue::Object(fields) = event else {
            continue;
        };
        if json_field(fields, "kind") != Some("stdio")
            || json_field(fields, "stream") != Some(stream)
        {
            continue;
        }
        if let Some(value) = json_field(fields, "text") {
            text.push_str(value);
        }
        if json_field(fields, "terminator") == Some("newline") {
            text.push('\n');
        }
    }
    text
}

fn json_field<'a>(fields: &'a [(String, JsonValue)], key: &str) -> Option<&'a str> {
    fields.iter().find_map(|(field, value)| {
        if field == key {
            if let JsonValue::String(value) = value {
                return Some(value.as_str());
            }
        }
        None
    })
}

fn normalize_lines(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    normalized
        .strip_suffix('\n')
        .unwrap_or(&normalized)
        .to_string()
}

fn first_differing_line(expected: &str, actual: &str) -> OutputDifference {
    let expected = normalize_lines(expected);
    let actual = normalize_lines(actual);
    let expected_lines = expected.split('\n').collect::<Vec<_>>();
    let actual_lines = actual.split('\n').collect::<Vec<_>>();
    let max_len = expected_lines.len().max(actual_lines.len());
    for index in 0..max_len {
        let expected_line = expected_lines.get(index).copied();
        let actual_line = actual_lines.get(index).copied();
        if expected_line != actual_line {
            return OutputDifference {
                line: index + 1,
                expected: expected_line.map(ToString::to_string),
                actual: actual_line.map(ToString::to_string),
            };
        }
    }
    OutputDifference {
        line: 1,
        expected: None,
        actual: None,
    }
}

fn output_events_for_stream(events: &[JsonValue], stream: &str) -> Vec<JsonValue> {
    events
        .iter()
        .filter_map(|event| {
            let JsonValue::Object(fields) = event else {
                return None;
            };
            (json_field(fields, "kind") == Some("stdio")
                && json_field(fields, "stream") == Some(stream))
            .then(|| event.clone())
        })
        .take(4)
        .collect()
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
    use std::fs;
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
    fn expands_explicit_source_target_to_paired_test_file() {
        let root = test_root("paired-source");
        fs::create_dir_all(&root).expect("create test root");
        fs::write(root.join("app.veln"), "").expect("write source file");
        fs::write(root.join("app_test.veln"), "").expect("write test file");

        let expansion = expand_test_targets(&root, &[PathBuf::from("app.veln")]);

        assert_eq!(
            expansion.targets,
            vec![PathBuf::from("app.veln"), PathBuf::from("app_test.veln")]
        );
        assert_eq!(expansion.source_to_test_added_count, 1);
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn does_not_expand_directory_or_test_file_targets() {
        let root = test_root("direct-target");
        fs::create_dir_all(root.join("cases")).expect("create test root");
        fs::write(root.join("app_test.veln"), "").expect("write test file");

        let expansion = expand_test_targets(
            &root,
            &[PathBuf::from("cases"), PathBuf::from("app_test.veln")],
        );

        assert_eq!(
            expansion.targets,
            vec![PathBuf::from("app_test.veln"), PathBuf::from("cases")]
        );
        assert_eq!(expansion.source_to_test_added_count, 0);
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn selection_targets_report_doctest_origin_source() {
        let project = Project {
            root: PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new("main.veln", ""),
                SourceFile::new("main.veln#doctest-1_test.veln", ""),
            ],
        };
        let test_files = BTreeSet::from(["main.veln#doctest-1_test.veln".to_string()]);

        assert_eq!(selection_targets(&project, &test_files), vec!["main.veln"]);
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
    fn extracts_doc_comment_veln_fences_with_expected_output() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "/// ```veln-output stream=stdout\n",
                "/// ready\n",
                "/// ```\n",
                "pub fn main() -> () effects []\n",
                "  ()\n",
                "end\n",
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
            .expected_outputs
            .get("doctest_1")
            .expect("expected output should be recorded");
        assert_eq!(expected.stdout.as_deref(), Some("ready"));
        assert_eq!(expected.stderr, None);
    }

    #[test]
    fn duplicate_doctest_output_stream_reports_diagnostic() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "/// ```veln-output stream=stdout\n",
                "/// ready\n",
                "/// ```\n",
                "/// ```veln-output stream=stdout\n",
                "/// duplicate\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        let expected = doctests
            .expected_outputs
            .get("doctest_1")
            .expect("first expected output should be kept");
        assert_eq!(expected.stdout.as_deref(), Some("ready"));
        assert_eq!(doctests.diagnostics.len(), 1);
        assert_eq!(doctests.diagnostics[0].id, "doctest.duplicate_output");
        assert_eq!(
            doctests.diagnostics[0].message,
            "duplicate expected stdout output fence"
        );
        assert_eq!(doctests.diagnostics[0].related.len(), 1);
    }

    #[test]
    fn unknown_doctest_output_attribute_reports_diagnostic() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "/// ```veln-output stream=stdout trim=true\n",
                "/// ready\n",
                "/// ```\n",
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
                "/// ```veln\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "/// ```veln-output stream=combined\n",
                "/// ready\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        let expected = doctests
            .expected_outputs
            .get("doctest_1")
            .expect("doctest should keep an empty expectation record");
        assert_eq!(expected.stdout, None);
        assert_eq!(expected.stderr, None);
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
                "/// ```veln\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "/// ```veln-output\n",
                "/// ready\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        let expected = doctests
            .expected_outputs
            .get("doctest_1")
            .expect("doctest should keep an empty expectation record");
        assert_eq!(expected.stdout, None);
        assert_eq!(expected.stderr, None);
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
    fn ignores_non_runnable_doctest_fences() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln ignore\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert!(doctests.sources.is_empty());
        assert!(doctests.expected_outputs.is_empty());
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
                "/// ```veln fail\n",
                "/// let value: Int = \"no\"\n",
                "/// ```\n",
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
        assert!(doctests.expected_outputs.is_empty());
        assert_eq!(doctests.expected_failures.len(), 1);
        assert!(
            doctests
                .expected_failures
                .contains_key("main.veln#doctest-1_test.veln")
        );
    }

    #[test]
    fn negative_doctest_failure_reconciliation_consumes_matching_diagnostics() {
        let source = SourceFile::new("main.veln", "/// ```veln fail\n");
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
        let source = SourceFile::new("main.veln", "/// ```veln fail\n");
        let fail_span = source.span(TextRange::new(0, 16));
        let expected_failures =
            BTreeMap::from([("main.veln#doctest-1_test.veln".to_string(), fail_span)]);

        let reconciled = reconcile_expected_doctest_failures(Vec::new(), &expected_failures);

        assert_eq!(reconciled.len(), 1);
        assert_eq!(reconciled[0].id, "doctest.expected_failure_missing");
        assert_eq!(
            reconciled[0].message,
            "negative doctest produced no diagnostics"
        );
    }

    #[test]
    fn extracts_hidden_doctest_setup_lines() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln\n",
                "/// # let greeting = \"ready\"\n",
                "/// stdio::println(greeting)\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> () effects [stdio]\n",
                "  let greeting = \"ready\"\n",
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
    fn unknown_doctest_fence_attribute_reports_diagnostic() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln skip=true\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(doctests.diagnostics.len(), 1);
        assert_eq!(doctests.diagnostics[0].id, "doctest.unknown_metadata");
        assert_eq!(
            doctests.diagnostics[0].message,
            "unknown doctest attribute `skip`"
        );
        assert_eq!(
            doctests.diagnostics[0].details.to_json(),
            "{\"kind\":\"doctest_metadata\",\"attribute\":\"skip\",\"fence\":\"veln\"}"
        );
    }

    #[test]
    fn empty_doctest_error_type_reports_diagnostic() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln error=\n",
                "/// let value = parse(\"1\")?\n",
                "/// ```\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> () effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  ()\n",
                "end\n",
            )
        );
        assert_eq!(doctests.diagnostics.len(), 1);
        assert_eq!(doctests.diagnostics[0].id, "doctest.invalid_metadata");
        assert_eq!(doctests.diagnostics[0].message, "empty doctest error type");
        assert_eq!(
            doctests.diagnostics[0].details.to_json(),
            "{\"kind\":\"doctest_metadata\",\"attribute\":\"error\"}"
        );
    }

    #[test]
    fn extracts_doctest_error_type_fence_attribute() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln error=AppError\n",
                "/// let value = parse(\"1\")?\n",
                "/// stdio::println(\"ready\")\n",
                "/// ```\n",
                "pub fn main() -> () effects []\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result((), AppError) effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  stdio::println(\"ready\")\n",
                "  Ok(())\n",
                "end\n",
            )
        );
    }

    #[test]
    fn infers_doctest_error_type_from_documented_public_result() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "/// ```veln\n",
                "/// let value: Int = Ok(1)?\n",
                "/// ```\n",
                "pub fn parse(raw: String) -> Result(Int, AppError) effects []\n",
                "  Ok(1)\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result((), AppError) effects [stdio]\n",
                "  let value: Int = Ok(1)?\n",
                "  Ok(())\n",
                "end\n",
            )
        );
    }

    #[test]
    fn infers_doctest_error_type_from_single_result_operation() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
                "  Ok(1)\n",
                "end\n",
                "/// ```veln\n",
                "/// let value = parse(\"1\")?\n",
                "/// ```\n",
                "pub fn main() -> () effects []\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result((), AppError) effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  Ok(())\n",
                "end\n",
            )
        );
    }

    #[test]
    fn does_not_infer_doctest_error_type_from_mixed_result_operations() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
                "  Ok(1)\n",
                "end\n",
                "fn read(raw: String) -> Result(String, IoError) effects []\n",
                "  Ok(raw)\n",
                "end\n",
                "/// ```veln\n",
                "/// let value = parse(\"1\")?\n",
                "/// let text = read(\"x\")?\n",
                "/// ```\n",
                "pub fn main() -> () effects []\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> () effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  let text = read(\"x\")?\n",
                "  ()\n",
                "end\n",
            )
        );
    }

    #[test]
    fn expected_output_mismatch_marks_case_failed() {
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
                stderr: None,
                ..ExpectedOutput::default()
            }),
            events: vec![stdio_event(
                "stdout",
                "println",
                "waiting",
                "newline",
                1,
                "call-1",
                &source_file.span(TextRange::new(0, source_file.len())),
            )],
            diagnostics: Vec::new(),
        };

        compare_expected_output(&mut case);

        assert_eq!(case.status, TestCaseStatus::Failed);
        assert_eq!(case.reason.as_deref(), Some("expected_output"));
        let failure = case.failure.expect("mismatch should create failure");
        assert_eq!(failure.kind, "output");
        assert_eq!(failure.message, "expected stdout output did not match");
        let failure_json = failure.to_json().to_json();
        assert!(failure_json.contains("\"actual\":\"waiting\\n\""));
        assert!(failure_json.contains(
            "\"first_difference\":{\"line\":1,\"expected\":\"ready\",\"actual\":\"waiting\"}"
        ));
        assert!(failure_json.contains("\"actual_events\":[{\"kind\":\"stdio\""));
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
    fn stdio_trace_skips_malformed_lines() {
        let source_file = SourceFile::new(
            "main_test.veln",
            "test first() -> () effects []\n  ()\nend\n",
        );
        let source = TestCaseSource {
            file: "main_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        };

        let events = stdio_events_from_trace(
            concat!(
                "not-a-sequence\tstdout\tprint\tnone\t\t\t7265616479\n",
                "1\tstdout\tprint\tnone\t\t\tinvalid-hex\n",
                "2\tstdout\tprint\tnone\t\t\t6f6b\n",
            ),
            &BTreeMap::new(),
            &source,
        );

        assert_eq!(events.len(), 1);
        assert!(events[0].to_json().contains("\"sequence\":2"));
        assert!(events[0].to_json().contains("\"text\":\"ok\""));
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

        let expansion = expand_test_targets(&root, &[source.clone()]);

        assert_eq!(expansion.targets, vec![source, test]);
        assert_eq!(expansion.source_to_test_added_count, 1);
        fs::remove_dir_all(root).expect("remove test root");
    }

    #[test]
    fn negative_doctest_failure_reconciliation_keeps_unrelated_diagnostics() {
        let source = SourceFile::new("main.veln", "/// ```veln fail\n");
        let generated = SourceFile::new("main.veln#doctest-1_test.veln", "fn doctest_1()\nend\n");
        let other = SourceFile::new("other.veln", "fn helper()\nend\n");
        let fail_span = source.span(TextRange::new(0, 16));
        let generated_span = generated.span(TextRange::new(0, generated.len()));
        let other_span = other.span(TextRange::new(0, other.len()));
        let diagnostics = vec![
            Diagnostic::new(
                "type.mismatch",
                Severity::Error,
                DiagnosticKind::Type,
                "expected `Int`, but found `String`",
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
        let source_file = SourceFile::new(
            "main_test.veln",
            "test first() -> () effects []\n  ()\nend\n",
        );
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
                "\"end\":{\"line\":4,\"column\":1,\"offset\":39}}}"
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
    fn source_to_test_convention_records_plural_note() {
        let selection = TestSelection {
            mode_name: "explicit".to_string(),
            targets: vec!["app.veln".to_string(), "app_test.veln".to_string()],
            confidence: "complete".to_string(),
            reason: "user_selected".to_string(),
            notes: Vec::new(),
        }
        .source_to_test_convention(2);

        assert_eq!(selection.confidence, "partial");
        assert_eq!(selection.reason, "source_to_test_convention");
        assert_eq!(
            selection.notes,
            vec!["added 2 test files by source-to-test convention"]
        );
        assert!(
            selection
                .to_json()
                .to_json()
                .contains("\"notes\":[\"added 2 test files by source-to-test convention\"]")
        );
    }

    #[test]
    fn test_run_status_precedence_handles_errors_blockers_and_failures() {
        let source_file = SourceFile::new("main_test.veln", "test first() -> () effects []\nend\n");
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

    fn test_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("veln-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        root
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
