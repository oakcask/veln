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
    if !hex.len().is_multiple_of(2) {
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

fn result_failure_from_trace_line(line: &str) -> Option<TestFailure> {
    let mut fields = line.split('\t');
    if fields.next()? != "result" {
        return None;
    }
    let value = decode_hex_text(fields.next()?)?;
    let mut fixture_hex = None;
    let mut byte_diagnostic = None;
    let mut value_diagnostic = None;
    let mut protocol_diagnostic = None;
    while let Some(kind) = fields.next() {
        match kind {
            "fixture_hex" => fixture_hex = Some(fixture_hex_details(&mut fields)?),
            "byte_diagnostic" => byte_diagnostic = Some(byte_diagnostic_details(&mut fields)?),
            "byte_diagnostic_v2" => {
                byte_diagnostic = Some(byte_diagnostic_v2_details(&mut fields)?)
            }
            "value_diagnostic" => value_diagnostic = Some(value_diagnostic_details(&mut fields)?),
            "protocol_diagnostic" => {
                protocol_diagnostic = Some(protocol_diagnostic_details(&mut fields)?)
            }
            _ => return None,
        }
    }
    Some(TestFailure::result_with_extended_details(
        value,
        fixture_hex,
        byte_diagnostic,
        value_diagnostic,
        protocol_diagnostic,
    ))
}

fn fixture_hex_details<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let text_start = fields.next()?.parse::<i64>().ok()?;
    let text_end = fields.next()?.parse::<i64>().ok()?;
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let nibble = fields.next()?.to_string();
    let context_start = fields.next()?.parse::<i64>().ok()?;
    let context_end = fields.next()?.parse::<i64>().ok()?;
    let context = decode_hex_text(fields.next()?)?;
    if fields.next().is_some() {
        return None;
    }
    Some(JsonValue::object([
        ("kind", JsonValue::string("fixture_hex")),
        ("id", JsonValue::string(id)),
        (
            "fixture_text_span",
            JsonValue::object([
                ("start", JsonValue::Number(text_start)),
                ("end", JsonValue::Number(text_end)),
            ]),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(byte_offset)),
            ]),
        ),
        ("nibble_position", JsonValue::string(nibble)),
        (
            "nearby_context",
            JsonValue::object([
                ("start", JsonValue::Number(context_start)),
                ("end", JsonValue::Number(context_end)),
                ("text", JsonValue::string(context)),
            ]),
        ),
    ]))
}

fn byte_diagnostic_details<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let field_path_count = fields.next()?.parse::<usize>().ok()?;
    let mut field_path = Vec::with_capacity(field_path_count);
    for _ in 0..field_path_count {
        let kind = fields.next()?.to_string();
        let name = decode_hex_text(fields.next()?)?;
        field_path.push(JsonValue::object([
            ("kind", JsonValue::string(kind)),
            ("name", JsonValue::string(name)),
        ]));
    }
    let expected_count = fields.next()?.parse::<i64>().ok()?;
    let available_count = fields.next()?.parse::<i64>().ok()?;
    let readiness = fields.next()?.to_string();
    Some(JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string(id)),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(byte_offset)),
            ]),
        ),
        ("field_path", JsonValue::array(field_path)),
        ("expected_count", JsonValue::Number(expected_count)),
        ("available_count", JsonValue::Number(available_count)),
        ("readiness", JsonValue::string(readiness)),
    ]))
}

fn byte_diagnostic_v2_details<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let field_path_count = fields.next()?.parse::<usize>().ok()?;
    let mut field_path = Vec::with_capacity(field_path_count);
    for _ in 0..field_path_count {
        let kind = fields.next()?.to_string();
        let name = decode_hex_text(fields.next()?)?;
        field_path.push(JsonValue::object([
            ("kind", JsonValue::string(kind)),
            ("name", JsonValue::string(name)),
        ]));
    }
    let detail_count = fields.next()?.parse::<usize>().ok()?;
    let mut entries = vec![
        ("kind".to_string(), JsonValue::string("byte_diagnostic")),
        ("id".to_string(), JsonValue::string(id)),
        (
            "byte_offset".to_string(),
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(byte_offset)),
            ]),
        ),
        ("field_path".to_string(), JsonValue::array(field_path)),
    ];
    for _ in 0..detail_count {
        let key = fields.next()?.to_string();
        let value_kind = fields.next()?;
        let value = fields.next()?;
        let json_value = match value_kind {
            "number" => JsonValue::Number(value.parse::<i64>().ok()?),
            "string" => JsonValue::string(decode_hex_text(value)?),
            "byte_preview" => byte_preview_value(value)?,
            "byte_preview_v2" => byte_preview_v2_value(value)?,
            _ => return None,
        };
        entries.push((key, json_value));
    }
    Some(JsonValue::Object(entries))
}

fn byte_preview_value(encoded_hex_text: &str) -> Option<JsonValue> {
    let data = decode_hex_text(encoded_hex_text)?;
    if data.len() % 2 != 0
        || !data
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return None;
    }
    let preview_byte_count = (data.len() / 2) as i64;
    Some(JsonValue::object([
        ("encoding", JsonValue::string("hex")),
        ("data", JsonValue::string(data)),
        ("preview_byte_count", JsonValue::Number(preview_byte_count)),
        ("total_byte_count", JsonValue::Number(preview_byte_count)),
        ("truncated", JsonValue::Bool(false)),
    ]))
}

fn byte_preview_v2_value(encoded_preview: &str) -> Option<JsonValue> {
    let mut fields = encoded_preview.split(':');
    let data = decode_hex_text(fields.next()?)?;
    let preview_byte_count = fields.next()?.parse::<i64>().ok()?;
    let total_byte_count = fields.next()?.parse::<i64>().ok()?;
    let truncated = match fields.next()? {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    if fields.next().is_some()
        || data.len() % 2 != 0
        || preview_byte_count < 0
        || total_byte_count < preview_byte_count
        || preview_byte_count != (data.len() / 2) as i64
        || !data
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
    {
        return None;
    }
    Some(JsonValue::object([
        ("encoding", JsonValue::string("hex")),
        ("data", JsonValue::string(data)),
        ("preview_byte_count", JsonValue::Number(preview_byte_count)),
        ("total_byte_count", JsonValue::Number(total_byte_count)),
        ("truncated", JsonValue::Bool(truncated)),
    ]))
}

fn protocol_diagnostic_details<'a>(
    fields: &mut impl Iterator<Item = &'a str>,
) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let byte_offset = fields.next()?.parse::<i64>().ok()?;
    let detail_count = fields.next()?.parse::<usize>().ok()?;
    let mut entries = vec![
        ("kind".to_string(), JsonValue::string("protocol_diagnostic")),
        ("id".to_string(), JsonValue::string(id)),
        (
            "byte_offset".to_string(),
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(byte_offset)),
            ]),
        ),
    ];
    for _ in 0..detail_count {
        let key = fields.next()?.to_string();
        let value_kind = fields.next()?;
        let value = fields.next()?;
        let json_value = match value_kind {
            "number" => JsonValue::Number(value.parse::<i64>().ok()?),
            "string" => JsonValue::string(decode_hex_text(value)?),
            "byte_preview" => byte_preview_value(value)?,
            "byte_preview_v2" => byte_preview_v2_value(value)?,
            _ => return None,
        };
        entries.push((key, json_value));
    }
    Some(JsonValue::Object(entries))
}

fn value_diagnostic_details<'a>(fields: &mut impl Iterator<Item = &'a str>) -> Option<JsonValue> {
    let id = fields.next()?.to_string();
    let field_path_count = fields.next()?.parse::<usize>().ok()?;
    let mut field_path = Vec::with_capacity(field_path_count);
    for _ in 0..field_path_count {
        let kind = fields.next()?.to_string();
        let name = decode_hex_text(fields.next()?)?;
        field_path.push(JsonValue::object([
            ("kind", JsonValue::string(kind)),
            ("name", JsonValue::string(name)),
        ]));
    }
    let detail_count = fields.next()?.parse::<usize>().ok()?;
    let mut entries = vec![
        ("kind".to_string(), JsonValue::string("value_diagnostic")),
        ("id".to_string(), JsonValue::string(id)),
        ("field_path".to_string(), JsonValue::array(field_path)),
    ];
    for _ in 0..detail_count {
        let key = fields.next()?.to_string();
        let value_kind = fields.next()?;
        let value = fields.next()?;
        let json_value = match value_kind {
            "number" => JsonValue::Number(value.parse::<i64>().ok()?),
            "string" => JsonValue::string(decode_hex_text(value)?),
            "byte_preview" => byte_preview_value(value)?,
            "byte_preview_v2" => byte_preview_v2_value(value)?,
            _ => return None,
        };
        entries.push((key, json_value));
    }
    Some(JsonValue::Object(entries))
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

    fn result_with_extended_details(
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
    expected_output: Option<ExpectedOutput>,
    expected_runtime_failure: Option<ExpectedRuntimeFailure>,
    should_fail: bool,
    fail_span: Option<SourceSpan>,
}

enum Fence {
    Veln {
        lines: Vec<String>,
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
struct ExtractedDoctests {
    doctests: Vec<ExtractedDoctest>,
    diagnostics: Vec<Diagnostic>,
}

struct DoctestExtractor<'a> {
    source: &'a SourceFile,
    signatures: &'a BTreeMap<String, Option<String>>,
    extracted: ExtractedDoctests,
    pending: Option<ExtractedDoctest>,
    fence: Option<Fence>,
    offset: usize,
}

impl<'a> DoctestExtractor<'a> {
    fn new(source: &'a SourceFile, signatures: &'a BTreeMap<String, Option<String>>) -> Self {
        Self {
            source,
            signatures,
            extracted: ExtractedDoctests::default(),
            pending: None,
            fence: None,
            offset: 0,
        }
    }

    fn extract(mut self) -> ExtractedDoctests {
        for raw_line in self.source.text().split_inclusive('\n') {
            self.handle_raw_line(raw_line);
        }
        self.finalize_pending();
        self.extracted
    }

    fn handle_raw_line(&mut self, raw_line: &str) {
        let line = raw_line
            .strip_suffix('\n')
            .unwrap_or(raw_line)
            .strip_suffix('\r')
            .unwrap_or_else(|| raw_line.strip_suffix('\n').unwrap_or(raw_line));
        let line_range = TextRange::new(self.offset, self.offset + line.len());
        self.offset += raw_line.len();

        let Some(content) = doc_comment_content(line) else {
            self.finalize_pending_with_error_context(line);
            return;
        };
        self.handle_doc_line(content.strip_prefix(' ').unwrap_or(content), line_range);
    }

    fn handle_doc_line(&mut self, content: &str, line_range: TextRange) {
        if self.fence.is_some() {
            if content.trim_start().starts_with("```") {
                self.close_fence();
            } else {
                self.append_fence_line(content);
            }
            return;
        }

        let trimmed = content.trim_start();
        if let Some(info) = trimmed.strip_prefix("```") {
            self.open_fence(info.trim(), line_range);
        } else if !trimmed.is_empty() {
            self.finalize_pending();
        }
    }

    fn open_fence(&mut self, info: &str, line_range: TextRange) {
        if veln_fence_info(info) {
            let span = self.source.span(line_range);
            self.extracted
                .diagnostics
                .extend(veln_metadata_diagnostics(info, span.clone()));
            self.fence = Some(Fence::Veln {
                lines: Vec::new(),
                error_type: doctest_error_type(info).map(ToString::to_string),
                expected_runtime_failure: doctest_runtime_failure(info, span).map(Box::new),
                ignored: doctest_ignored(info),
                should_fail: doctest_should_fail(info),
                fail_span: doctest_should_fail(info).then(|| self.source.span(line_range)),
            });
        } else if output_fence_info(info) {
            let span = self.source.span(line_range);
            self.extracted
                .diagnostics
                .extend(output_metadata_diagnostics(info, span.clone()));
            self.fence = output_fence_stream(info).map_or(Some(Fence::Ignored), |stream| {
                Some(Fence::Output {
                    stream: stream.to_string(),
                    lines: Vec::new(),
                    span,
                })
            });
        } else {
            self.finalize_pending();
        }
    }

    fn close_fence(&mut self) {
        match self.fence.take().expect("active fence should exist") {
            Fence::Veln {
                lines,
                error_type,
                expected_runtime_failure,
                ignored,
                should_fail,
                fail_span,
            } => {
                self.finalize_pending();
                if !ignored {
                    self.pending = Some(ExtractedDoctest {
                        code: lines,
                        error_type,
                        expected_output: None,
                        expected_runtime_failure: expected_runtime_failure.map(|failure| *failure),
                        should_fail,
                        fail_span,
                    });
                }
            }
            Fence::Output {
                stream,
                lines,
                span,
            } => self.attach_output(stream, lines, span),
            Fence::Ignored => {}
        }
    }

    fn append_fence_line(&mut self, content: &str) {
        match self.fence.as_mut().expect("active fence should exist") {
            Fence::Veln { lines, .. } => lines.push(doctest_code_line(content)),
            Fence::Output { lines, .. } => lines.push(content.to_string()),
            Fence::Ignored => {}
        }
    }

    fn attach_output(&mut self, stream: String, lines: Vec<String>, span: SourceSpan) {
        let Some(doctest) = &mut self.pending else {
            return;
        };
        let output = lines.join("\n");
        let expected_output = doctest.expected_output.get_or_insert_default();
        match stream.as_str() {
            "stdout" => {
                if let Some(first_span) = &expected_output.stdout_span {
                    self.extracted
                        .diagnostics
                        .push(duplicate_output_diagnostic(&stream, &span, first_span));
                } else {
                    expected_output.stdout = Some(output);
                    expected_output.stdout_span = Some(span);
                }
            }
            "stderr" => {
                if let Some(first_span) = &expected_output.stderr_span {
                    self.extracted
                        .diagnostics
                        .push(duplicate_output_diagnostic(&stream, &span, first_span));
                } else {
                    expected_output.stderr = Some(output);
                    expected_output.stderr_span = Some(span);
                }
            }
            _ => {}
        }
    }

    fn finalize_pending_with_error_context(&mut self, line: &str) {
        if let Some(doctest) = self.pending.take() {
            self.extracted
                .doctests
                .push(with_error_type_context(doctest, line, self.signatures));
        }
    }

    fn finalize_pending(&mut self) {
        if let Some(doctest) = self.pending.take() {
            self.extracted.doctests.push(doctest);
        }
    }
}

const RUNTIME_ATTRIBUTE: &str = "runtime";
const RUNTIME_CONTRACT_KIND: &str = "contract";
const RUNTIME_ENSURE_KIND: &str = "ensure";
const RUNTIME_RESULT_KIND: &str = "result";
const RUNTIME_CONTRACT_ATTRIBUTES: &[&str] = &["clause", "predicate", "function", "blame"];
const RUNTIME_CONTRACT_REQUIRED_ATTRIBUTES: &[&str] = &["clause", "predicate"];
const RUNTIME_ENSURE_ATTRIBUTES: &[&str] = &["predicate", "function", "blame"];
const RUNTIME_ENSURE_REQUIRED_ATTRIBUTES: &[&str] = &["predicate"];
const RUNTIME_RESULT_VALUE_ATTRIBUTE: &str = "value";
const RUNTIME_RESULT_ATTRIBUTES: &[&str] = &[RUNTIME_RESULT_VALUE_ATTRIBUTE];
const RUNTIME_RESULT_REQUIRED_ATTRIBUTES: &[&str] = &[RUNTIME_RESULT_VALUE_ATTRIBUTE];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeExpectationKind {
    Contract,
    Ensure,
    Result,
}

impl RuntimeExpectationKind {
    fn from_value(value: &str) -> Option<Self> {
        match value {
            RUNTIME_CONTRACT_KIND => Some(Self::Contract),
            RUNTIME_ENSURE_KIND => Some(Self::Ensure),
            RUNTIME_RESULT_KIND => Some(Self::Result),
            _ => None,
        }
    }

    fn allows_attribute(self, attribute: &str) -> bool {
        match self {
            Self::Contract => RUNTIME_CONTRACT_ATTRIBUTES.contains(&attribute),
            Self::Ensure => RUNTIME_ENSURE_ATTRIBUTES.contains(&attribute),
            Self::Result => RUNTIME_RESULT_ATTRIBUTES.contains(&attribute),
        }
    }

    fn required_attributes(self) -> &'static [&'static str] {
        match self {
            Self::Contract => RUNTIME_CONTRACT_REQUIRED_ATTRIBUTES,
            Self::Ensure => RUNTIME_ENSURE_REQUIRED_ATTRIBUTES,
            Self::Result => RUNTIME_RESULT_REQUIRED_ATTRIBUTES,
        }
    }

    fn empty_attribute_message(self, attribute: &str) -> String {
        match self {
            Self::Contract => format!("empty doctest runtime contract {attribute}"),
            Self::Ensure => format!("empty doctest runtime ensure {attribute}"),
            Self::Result => "empty doctest runtime result value".to_string(),
        }
    }

    fn missing_attribute_message(self, attribute: &str) -> String {
        match self {
            Self::Contract => format!("missing doctest runtime contract {attribute}"),
            Self::Ensure => format!("missing doctest runtime ensure {attribute}"),
            Self::Result => "missing doctest runtime result value".to_string(),
        }
    }

    fn expected_failure(self, info: &str, span: SourceSpan) -> Option<ExpectedRuntimeFailure> {
        match self {
            Self::Contract => Some(ExpectedRuntimeFailure::Contract(ExpectedContractFailure {
                clause: metadata_value(info, "clause")?.to_string(),
                predicate: metadata_value(info, "predicate")?.to_string(),
                function: metadata_value(info, "function").map(ToString::to_string),
                blame: metadata_value(info, "blame").map(ToString::to_string),
                span,
            })),
            Self::Ensure => Some(ExpectedRuntimeFailure::ContractClause(
                ExpectedContractFailure {
                    clause: RUNTIME_ENSURE_KIND.to_string(),
                    predicate: metadata_value(info, "predicate")?.to_string(),
                    function: metadata_value(info, "function").map(ToString::to_string),
                    blame: metadata_value(info, "blame").map(ToString::to_string),
                    span,
                },
            )),
            Self::Result => Some(ExpectedRuntimeFailure::Result(ExpectedResultFailure {
                value: metadata_value(info, RUNTIME_RESULT_VALUE_ATTRIBUTE)?.to_string(),
                span,
            })),
        }
    }
}

fn extract_doctests(
    source: &SourceFile,
    signatures: &BTreeMap<String, Option<String>>,
) -> ExtractedDoctests {
    DoctestExtractor::new(source, signatures).extract()
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
    let line = line.trim_start();
    line.strip_prefix("##")
}

fn doctest_code_line(content: &str) -> String {
    if let Some(hidden) = content.strip_prefix("> ") {
        return hidden.to_string();
    }
    content.to_string()
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

fn doctest_runtime_failure(info: &str, span: SourceSpan) -> Option<ExpectedRuntimeFailure> {
    RuntimeExpectationKind::from_value(metadata_value(info, RUNTIME_ATTRIBUTE)?)
        .and_then(|kind| kind.expected_failure(info, span))
}

fn metadata_field_value<'a>(field: &'a str, name: &str) -> Option<&'a str> {
    let (attribute, value) = metadata_attribute_value(field)?;
    (attribute == name).then_some(value)
}

fn metadata_value<'a>(info: &'a str, name: &str) -> Option<&'a str> {
    info.split_whitespace()
        .skip(1)
        .find_map(|field| metadata_field_value(field, name))
        .filter(|value| !value.is_empty())
}

fn output_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln-output")
}

fn output_fence_stream(info: &str) -> Option<&str> {
    let mut fields = info.split_whitespace();
    if fields.next()? != "veln-output" {
        return None;
    }
    let stream = fields.find_map(|field| metadata_field_value(field, "stream"))?;
    matches!(stream, "stdout" | "stderr").then_some(stream)
}

fn veln_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
    let runtime = metadata_value(info, RUNTIME_ATTRIBUTE);
    let runtime_kind = runtime.and_then(RuntimeExpectationKind::from_value);
    let mut diagnostics: Vec<Diagnostic> = info
        .split_whitespace()
        .skip(1)
        .filter_map(|field| veln_metadata_field_diagnostic(field, runtime_kind, span.clone()))
        .collect();

    if let Some(kind) = runtime_kind {
        for attribute in kind.required_attributes() {
            if metadata_value(info, attribute).is_none() {
                diagnostics.push(invalid_doctest_metadata_diagnostic(
                    kind.missing_attribute_message(attribute),
                    attribute,
                    span.clone(),
                    Vec::new(),
                ));
            }
        }
    }

    diagnostics
}

fn veln_metadata_field_diagnostic(
    field: &str,
    runtime_kind: Option<RuntimeExpectationKind>,
    span: SourceSpan,
) -> Option<Diagnostic> {
    match classify_veln_metadata_field(field) {
        VelnMetadataField::Error(value) => empty_error_metadata_diagnostic(value, span),
        VelnMetadataField::Runtime(value) => runtime_metadata_diagnostic(value, span),
        VelnMetadataField::RuntimeExpectation { attribute, value } => {
            runtime_expectation_metadata_diagnostic(attribute, value, runtime_kind, span)
        }
        VelnMetadataField::Flag => None,
        VelnMetadataField::Unknown(attribute) => Some(unknown_doctest_metadata_diagnostic(
            format!("unknown doctest attribute `{attribute}`"),
            attribute,
            "veln",
            span,
        )),
    }
}

enum VelnMetadataField<'a> {
    Error(&'a str),
    Runtime(&'a str),
    RuntimeExpectation { attribute: &'a str, value: &'a str },
    Flag,
    Unknown(&'a str),
}

fn classify_veln_metadata_field(field: &str) -> VelnMetadataField<'_> {
    if let Some(value) = metadata_field_value(field, "error") {
        VelnMetadataField::Error(value)
    } else if let Some(value) = metadata_field_value(field, RUNTIME_ATTRIBUTE) {
        VelnMetadataField::Runtime(value)
    } else if let Some((attribute, value)) = runtime_expectation_metadata_field(field) {
        VelnMetadataField::RuntimeExpectation { attribute, value }
    } else if matches!(field, "ignore" | "fail") {
        VelnMetadataField::Flag
    } else {
        VelnMetadataField::Unknown(metadata_attribute_name(field))
    }
}

fn empty_error_metadata_diagnostic(value: &str, span: SourceSpan) -> Option<Diagnostic> {
    value.is_empty().then(|| {
        invalid_doctest_metadata_diagnostic("empty doctest error type", "error", span, Vec::new())
    })
}

fn runtime_metadata_diagnostic(value: &str, span: SourceSpan) -> Option<Diagnostic> {
    if value.is_empty() {
        Some(invalid_doctest_metadata_diagnostic(
            "empty doctest runtime failure kind",
            RUNTIME_ATTRIBUTE,
            span,
            Vec::new(),
        ))
    } else if !matches!(
        value,
        RUNTIME_CONTRACT_KIND | RUNTIME_ENSURE_KIND | RUNTIME_RESULT_KIND
    ) {
        Some(invalid_doctest_metadata_diagnostic(
            format!("unknown doctest runtime failure kind `{value}`"),
            RUNTIME_ATTRIBUTE,
            span,
            vec![("runtime", JsonValue::string(value))],
        ))
    } else {
        None
    }
}

fn runtime_expectation_metadata_diagnostic(
    attribute: &str,
    value: &str,
    runtime_kind: Option<RuntimeExpectationKind>,
    span: SourceSpan,
) -> Option<Diagnostic> {
    let Some(kind) = runtime_kind.filter(|kind| kind.allows_attribute(attribute)) else {
        return Some(unknown_doctest_metadata_diagnostic(
            format!("unknown doctest attribute `{attribute}`"),
            attribute,
            "veln",
            span,
        ));
    };
    value.is_empty().then(|| {
        invalid_doctest_metadata_diagnostic(
            kind.empty_attribute_message(attribute),
            attribute,
            span,
            Vec::new(),
        )
    })
}

fn runtime_expectation_metadata_field(field: &str) -> Option<(&str, &str)> {
    let (attribute, value) = metadata_attribute_value(field)?;
    RUNTIME_CONTRACT_ATTRIBUTES
        .iter()
        .chain(RUNTIME_RESULT_ATTRIBUTES.iter())
        .any(|expected| *expected == attribute)
        .then_some((attribute, value))
}

fn metadata_attribute_value(field: &str) -> Option<(&str, &str)> {
    let (attribute, value) = field.split_once('=')?;
    Some((attribute, value))
}

fn output_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut has_stream = false;
    for field in info.split_whitespace().skip(1) {
        if let Some(stream) = field.strip_prefix("stream=") {
            has_stream = true;
            if !matches!(stream, "stdout" | "stderr") {
                diagnostics.push(invalid_doctest_metadata_diagnostic(
                    format!("unknown doctest output stream `{stream}`"),
                    "stream",
                    span.clone(),
                    vec![("stream", JsonValue::string(stream))],
                ));
            }
        } else {
            diagnostics.push(unknown_doctest_metadata_diagnostic(
                format!(
                    "unknown doctest output attribute `{}`",
                    metadata_attribute_name(field)
                ),
                metadata_attribute_name(field),
                "veln-output",
                span.clone(),
            ));
        }
    }
    if !has_stream {
        diagnostics.push(invalid_doctest_metadata_diagnostic(
            "missing doctest output stream",
            "stream",
            span,
            Vec::new(),
        ));
    }
    diagnostics
}

fn invalid_doctest_metadata_diagnostic(
    message: impl Into<String>,
    attribute: &str,
    span: SourceSpan,
    extra_details: Vec<(&'static str, JsonValue)>,
) -> Diagnostic {
    let mut details = vec![
        ("kind", JsonValue::string("doctest_metadata")),
        ("attribute", JsonValue::string(attribute)),
    ];
    details.extend(extra_details);
    doctest_metadata_diagnostic(
        "doctest.invalid_metadata",
        message,
        span,
        JsonValue::object(details),
    )
}

fn unknown_doctest_metadata_diagnostic(
    message: impl Into<String>,
    attribute: &str,
    fence: &str,
    span: SourceSpan,
) -> Diagnostic {
    doctest_metadata_diagnostic(
        "doctest.unknown_metadata",
        message,
        span,
        JsonValue::object([
            ("kind", JsonValue::string("doctest_metadata")),
            ("attribute", JsonValue::string(attribute)),
            ("fence", JsonValue::string(fence)),
        ]),
    )
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
    let args = ty
        .strip_prefix("Result<")
        .and_then(|ty| ty.strip_suffix('>'))
        .or_else(|| {
            ty.strip_prefix("Result(")
                .and_then(|ty| ty.strip_suffix(')'))
        })?;
    let comma = top_level_comma(args)?;
    let error = args[comma + 1..].trim();
    (!error.is_empty()).then_some(error)
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn generated_doctest_source(name: &str, doctest: &ExtractedDoctest) -> String {
    let return_type = doctest.error_type.as_ref().map_or_else(
        || "()".to_string(),
        |error_type| format!("Result<(), {error_type}>"),
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
        if field == key
            && let JsonValue::String(value) = value
        {
            return Some(value.as_str());
        }
        None
    })
}

fn json_object_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    json_field(fields, key)
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
    fn negative_doctest_failure_reconciliation_consumes_matching_diagnostics() {
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

    #[test]
    fn extracts_hidden_doctest_setup_lines() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## > let greeting = \"ready\"\n",
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
    fn extracts_hash_doc_comment_doctests_with_visible_hash_comments() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "## ```veln\n",
                "## > let greeting = \"ready\"\n",
                "## # visible example comment\n",
                "## stdio::println(greeting)\n",
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
                "  let greeting = \"ready\"\n",
                "  # visible example comment\n",
                "  stdio::println(greeting)\n",
                "  ()\n",
                "end\n",
            )
        );
        let expected = doctests
            .expectations
            .get("doctest_1")
            .expect("expected output should be recorded");
        assert_eq!(
            expected
                .expected_output
                .as_ref()
                .and_then(|output| output.stdout.as_deref()),
            Some("ready")
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
                "## ```veln skip=true\n",
                "## stdio::println(\"ready\")\n",
                "## ```\n",
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
                "## ```veln error=\n",
                "## let value = parse(\"1\")?\n",
                "## ```\n",
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
                "## ```veln error=AppError\n",
                "## let value = parse(\"1\")?\n",
                "## stdio::println(\"ready\")\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
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
                "## ```veln\n",
                "## let value: Int = Ok(1)?\n",
                "## ```\n",
                "pub fn parse(raw: String) -> Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
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
                "fn parse(raw: String) -> Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
                "## ```veln\n",
                "## let value = parse(\"1\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  Ok(())\n",
                "end\n",
            )
        );
    }

    #[test]
    fn infers_doctest_error_type_from_result_binding_return_type() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> result: Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
                "## ```veln\n",
                "## let value = parse(\"1\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  Ok(())\n",
                "end\n",
            )
        );
    }

    #[test]
    fn infers_doctest_error_type_after_nested_result_success_type() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result<Vec<Result<Int, ParseError>>, AppError>\n",
                "  Ok([])\n",
                "end\n",
                "## ```veln\n",
                "## let value = parse(\"1\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), AppError> effects [stdio]\n",
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
                "fn parse(raw: String) -> Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
                "fn read(raw: String) -> Result<String, IoError>\n",
                "  Ok(raw)\n",
                "end\n",
                "## ```veln\n",
                "## let value = parse(\"1\")?\n",
                "## let text = read(\"x\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
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
    fn explicit_doctest_error_type_handles_mixed_result_operations() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
                "fn read(raw: String) -> Result<String, IoError>\n",
                "  Ok(raw)\n",
                "end\n",
                "## ```veln error=ExampleError\n",
                "## let value = parse(\"1\")?\n",
                "## let text = read(\"x\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[source]);

        assert_eq!(doctests.sources.len(), 1);
        assert_eq!(
            doctests.sources[0].text(),
            concat!(
                "test doctest_1() -> Result<(), ExampleError> effects [stdio]\n",
                "  let value = parse(\"1\")?\n",
                "  let text = read(\"x\")?\n",
                "  Ok(())\n",
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
    fn does_not_infer_doctest_error_type_from_ambiguous_function_signatures() {
        let primary = SourceFile::new(
            "main.veln",
            concat!(
                "fn parse(raw: String) -> Result<Int, AppError>\n",
                "  Ok(1)\n",
                "end\n",
                "## ```veln\n",
                "## let value = parse(\"1\")?\n",
                "## ```\n",
                "pub fn main() -> ()\n",
                "  ()\n",
                "end\n",
            ),
        );
        let imported = SourceFile::new(
            "other.veln",
            concat!(
                "fn parse(raw: String) -> Result<Int, ParseError>\n",
                "  Ok(1)\n",
                "end\n",
            ),
        );

        let doctests = doctest_sources(&[primary, imported]);

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
        assert!(
            doctests.diagnostics.is_empty(),
            "{:#?}",
            doctests.diagnostics
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
            expected_runtime_failure: None,
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
    fn expected_output_match_normalizes_line_endings_and_keeps_case_passed() {
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
                stdout: Some("ready\nnext".to_string()),
                stderr: Some("warn".to_string()),
                ..ExpectedOutput::default()
            }),
            expected_runtime_failure: None,
            events: vec![
                stdio_event(
                    "stdout",
                    "print",
                    "ready\r\nnext\n",
                    "none",
                    1,
                    "call-1",
                    &source_file.span(TextRange::new(0, source_file.len())),
                ),
                stdio_event(
                    "stderr",
                    "eprint",
                    "warn\r\n",
                    "none",
                    2,
                    "call-2",
                    &source_file.span(TextRange::new(0, source_file.len())),
                ),
            ],
            diagnostics: Vec::new(),
        };

        compare_expected_output(&mut case);

        assert_eq!(case.status, TestCaseStatus::Passed);
        assert!(case.reason.is_none());
        assert!(case.failure.is_none());
    }

    #[test]
    fn expected_output_mismatch_reports_missing_actual_line() {
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
                stdout: Some("ready\nnext".to_string()),
                stderr: None,
                ..ExpectedOutput::default()
            }),
            expected_runtime_failure: None,
            events: vec![stdio_event(
                "stdout",
                "println",
                "ready",
                "newline",
                1,
                "call-1",
                &source_file.span(TextRange::new(0, source_file.len())),
            )],
            diagnostics: Vec::new(),
        };

        compare_expected_output(&mut case);

        assert_eq!(case.status, TestCaseStatus::Failed);
        let failure = case.failure.expect("mismatch should create failure");
        assert!(
            failure.to_json().to_json().contains(
                "\"first_difference\":{\"line\":2,\"expected\":\"next\",\"actual\":null}"
            )
        );
    }

    #[test]
    fn expected_output_mismatch_reports_extra_actual_line_and_expected_span() {
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
                stdout_span: Some(span.clone()),
                stderr: None,
                ..ExpectedOutput::default()
            }),
            expected_runtime_failure: None,
            events: vec![
                stdio_event("stdout", "println", "ready", "newline", 1, "call-1", &span),
                stdio_event("stdout", "println", "next", "newline", 2, "call-2", &span),
            ],
            diagnostics: Vec::new(),
        };

        compare_expected_output(&mut case);

        assert_eq!(case.status, TestCaseStatus::Failed);
        let failure = case.failure.expect("mismatch should create failure");
        let failure_json = failure.to_json().to_json();
        assert!(
            failure_json.contains(
                "\"first_difference\":{\"line\":2,\"expected\":null,\"actual\":\"next\"}"
            )
        );
        assert!(failure_json.contains("\"expected_span\""));
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
    fn stdio_call_spans_include_type_applied_stdio_calls() {
        let module = module(concat!(
            "test first() -> () effects [stdio]\n",
            "  stdio::println<String>(\"out\")\n",
            "  ()\n",
            "end\n",
        ));

        let call_spans = stdio_call_spans(&module);

        assert_eq!(call_spans.len(), 1);
        let ((file, node_id), span) = call_spans
            .iter()
            .next()
            .expect("typed stdio call span should be recorded");
        assert_eq!(file, "main_test.veln");
        assert!(node_id.starts_with("call-"));
        assert_eq!(span.start.line, 2);
        assert_eq!(span.start.column, 3);
    }

    #[test]
    fn stdio_call_spans_include_nested_aggregate_and_match_calls() {
        let module = module(concat!(
            "test first() -> () effects [stdio]\n",
            "  let record = {out: stdio::println(\"record\")}\n",
            "  let list = [stdio::println(\"list\")]\n",
            "  let dict = {\"out\": stdio::println(\"dict\")}\n",
            "  match true\n",
            "    true => stdio::println(\"match\")\n",
            "    false => ()\n",
            "  end\n",
            "end\n",
        ));

        let call_spans = stdio_call_spans(&module);
        let mut lines = call_spans
            .values()
            .map(|span| span.start.line)
            .collect::<Vec<_>>();
        lines.sort();

        assert_eq!(call_spans.len(), 4);
        assert_eq!(lines, vec![2, 3, 4, 6]);
        assert!(
            call_spans
                .keys()
                .all(|(file, node_id)| file == "main_test.veln" && node_id.starts_with("call-"))
        );
    }

    #[test]
    fn stdio_trace_skips_malformed_lines() {
        let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
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
    fn stdio_trace_decodes_uppercase_hex_text() {
        let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
        let source = TestCaseSource {
            file: "main_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        };

        let events = stdio_events_from_trace(
            "1\tstdout\tprint\tnone\t\t\t4F4B\n",
            &BTreeMap::new(),
            &source,
        );

        assert_eq!(events.len(), 1);
        assert!(events[0].to_json().contains("\"text\":\"OK\""));
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
    fn result_trace_becomes_structured_test_failure() {
        let trace = "result\t626164\n";

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(failure.message, "runtime result failure: Err(bad)");
        assert_eq!(
            failure.to_json().to_json(),
            concat!(
                "{\"kind\":\"result\",\"message\":\"runtime result failure: Err(bad)\",",
                "\"expected\":null,\"actual\":null,\"span\":null,",
                "\"details\":{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"bad\"}}"
            )
        );
    }

    #[test]
    fn fixture_hex_result_trace_keeps_structured_details() {
        let trace = concat!(
            "result\t",
            "666978747572652e6865782e696e76616c69645f6368617261637465723a20",
            "657870656374656420415343494920686578206469676974206174206279746520",
            "6f666673657420312068696768206e6962626c65",
            "\tfixture_hex\tfixture.hex.invalid_character\t2\t3\t1\thigh\t0\t5\t30305f3031\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.message,
            "runtime result failure: Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)"
        );
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble\",",
                "\"fixture_hex\":{\"kind\":\"fixture_hex\",",
                "\"id\":\"fixture.hex.invalid_character\",",
                "\"fixture_text_span\":{\"start\":2,\"end\":3},",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":1},",
                "\"nibble_position\":\"high\",",
                "\"nearby_context\":{\"start\":0,\"end\":5,\"text\":\"00_01\"}}}"
            )
        );
    }

    #[test]
    fn byte_diagnostic_result_trace_keeps_structured_details() {
        let trace = concat!(
            "result\t",
            "6279746520726561642072657175697265732033206279746573206275742076696577206861732032",
            "\tbyte_diagnostic\tcodec.incomplete_input\t2\t0\t3\t2\tneed_bytes\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.message,
            "runtime result failure: Err(byte read requires 3 bytes but view has 2)"
        );
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"byte read requires 3 bytes but view has 2\",",
                "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
                "\"id\":\"codec.incomplete_input\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
                "\"field_path\":[],",
                "\"expected_count\":3,",
                "\"available_count\":2,",
                "\"readiness\":\"need_bytes\"}}"
            )
        );
    }

    #[test]
    fn byte_diagnostic_result_trace_keeps_field_path_segments() {
        let trace = concat!(
            "result\t",
            "6279746520726561642072657175697265732033206279746573206275742076696577206861732032",
            "\tbyte_diagnostic\tcodec.incomplete_input\t2",
            "\t2\tschema\t48747470324672616d65486561646572\tfield\t6c656e677468",
            "\t3\t2\tneed_bytes\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"byte read requires 3 bytes but view has 2\",",
                "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
                "\"id\":\"codec.incomplete_input\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"Http2FrameHeader\"},",
                "{\"kind\":\"field\",\"name\":\"length\"}],",
                "\"expected_count\":3,",
                "\"available_count\":2,",
                "\"readiness\":\"need_bytes\"}}"
            )
        );
    }

    #[test]
    fn byte_diagnostic_v2_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "6669786564206669656c64206d69736d617463682061742062797465206f66667365742030",
            "\tbyte_diagnostic_v2\tschema.fixed_field_mismatch\t0",
            "\t2\tschema\t44656d6f5061636b6574\tfield\t6b696e64",
            "\t3\texpected_value\tnumber\t1",
            "\tactual_value\tnumber\t255",
            "\tbyte_preview\tbyte_preview\t666630303031\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"fixed field mismatch at byte offset 0\",",
                "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
                "\"id\":\"schema.fixed_field_mismatch\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"DemoPacket\"},",
                "{\"kind\":\"field\",\"name\":\"kind\"}],",
                "\"expected_value\":1,",
                "\"actual_value\":255,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"ff0001\",",
                "\"preview_byte_count\":3,",
                "\"total_byte_count\":3,",
                "\"truncated\":false}}}"
            )
        );
    }

    #[test]
    fn byte_diagnostic_v2_result_trace_decodes_preview_counts() {
        let trace = concat!(
            "result\t",
            "6669786564206669656c64206d69736d617463682061742062797465206f66667365742030",
            "\tbyte_diagnostic_v2\tschema.fixed_field_mismatch\t0",
            "\t2\tschema\t44656d6f5061636b6574\tfield\t6b696e64",
            "\t3\texpected_value\tnumber\t1",
            "\tactual_value\tnumber\t255",
            "\tbyte_preview\tbyte_preview_v2\t666630303031:3:7:true\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"fixed field mismatch at byte offset 0\",",
                "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
                "\"id\":\"schema.fixed_field_mismatch\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"DemoPacket\"},",
                "{\"kind\":\"field\",\"name\":\"kind\"}],",
                "\"expected_value\":1,",
                "\"actual_value\":255,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"ff0001\",",
                "\"preview_byte_count\":3,",
                "\"total_byte_count\":7,",
                "\"truncated\":true}}}"
            )
        );
    }

    #[test]
    fn byte_diagnostic_v2_result_trace_keeps_range_details() {
        let trace = concat!(
            "result\t",
            "6279746520766965772072616e67652065786365656473206368756e6b206c656e677468",
            "\tbyte_diagnostic_v2\tcodec.byte_range_out_of_bounds\t2",
            "\t0",
            "\t3\trequested_count\tnumber\t2",
            "\tavailable_count\tnumber\t1",
            "\tbyte_preview\tbyte_preview_v2\t3032:1:1:false\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"byte view range exceeds chunk length\",",
                "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
                "\"id\":\"codec.byte_range_out_of_bounds\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
                "\"field_path\":[],",
                "\"requested_count\":2,",
                "\"available_count\":1,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"02\",",
                "\"preview_byte_count\":1,",
                "\"total_byte_count\":1,",
                "\"truncated\":false}}}"
            )
        );
    }

    #[test]
    fn value_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "736368656d612076616c75652076616c69646174696f6e206661696c656420666f72206669656c64206070616464696e675f6c656e67746860",
            "\tvalue_diagnostic\tschema.validation_failed",
            "\t2\tschema\t4f7264696e6172795061636b6574\tfield\t70616464696e675f6c656e677468",
            "\t5\tpredicate\tstring\t70616464696e675f6c656e677468203c3d206c656e677468",
            "\tfield_value\tnumber\t6",
            "\tsupplied_values\tstring\t6c656e6774683d352c2070616464696e675f6c656e6774683d36",
            "\tlength\tnumber\t5",
            "\tpadding_length\tnumber\t6\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"schema value validation failed for field `padding_length`\",",
                "\"value_diagnostic\":{\"kind\":\"value_diagnostic\",",
                "\"id\":\"schema.validation_failed\",",
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"OrdinaryPacket\"},",
                "{\"kind\":\"field\",\"name\":\"padding_length\"}],",
                "\"predicate\":\"padding_length <= length\",",
                "\"field_value\":6,",
                "\"supplied_values\":\"length=5, padding_length=6\",",
                "\"length\":5,",
                "\"padding_length\":6}}"
            )
        );
    }

    #[test]
    fn value_diagnostic_result_trace_decodes_byte_preview_details() {
        let trace = concat!(
            "result\t",
            "456e636f64654572726f7228636f6465632e656e636f64655f76616c75655f756e726570726573656e7461626c652c205061636b6574576972652e7061796c6f61642c2062797465207669657720636f756e74203320646f6573206e6f74206d61746368206c656e677468206669656c6420606c656e677468602076616c7565203229",
            "\tvalue_diagnostic\tcodec.encode_value_unrepresentable",
            "\t2\tschema\t5061636b657457697265\tfield\t7061796c6f6164",
            "\t7\treason\tstring\t62797465207669657720636f756e74203320646f6573206e6f74206d61746368206c656e677468206669656c6420606c656e677468602076616c75652032",
            "\tfield_path_display\tstring\t5061636b6574576972652e7061796c6f6164",
            "\texpected_count\tnumber\t2",
            "\tactual_count\tnumber\t3",
            "\tlength_expression\tstring\t6c656e677468",
            "\tbyte_offset\tnumber\t0",
            "\tbyte_preview\tbyte_preview_v2\t616162626363:3:3:false\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"EncodeError(codec.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2)\",",
                "\"value_diagnostic\":{\"kind\":\"value_diagnostic\",",
                "\"id\":\"codec.encode_value_unrepresentable\",",
                "\"field_path\":[{\"kind\":\"schema\",\"name\":\"PacketWire\"},",
                "{\"kind\":\"field\",\"name\":\"payload\"}],",
                "\"reason\":\"byte view count 3 does not match length field `length` value 2\",",
                "\"field_path_display\":\"PacketWire.payload\",",
                "\"expected_count\":2,",
                "\"actual_count\":3,",
                "\"length_expression\":\"length\",",
                "\"byte_offset\":0,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"aabbcc\",",
                "\"preview_byte_count\":3,",
                "\"total_byte_count\":3,",
                "\"truncated\":false}}}"
            )
        );
    }

    #[test]
    fn protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220657870656374656420434f4e54494e554154494f4e206672616d652061742062797465206f66667365742039",
            "\tprotocol_diagnostic\thttp2.protocol.continuation_expected\t9",
            "\t6\tactual_frame_kind\tnumber\t0",
            "\tactual_stream_id\tnumber\t1",
            "\texpected_stream_id\tnumber\t1",
            "\tstarted_frame_kind\tnumber\t1",
            "\tstarted_byte_offset\tnumber\t0",
            "\tactive_continuation\tstring\t68656164657273\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 expected CONTINUATION frame at byte offset 9\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.continuation_expected\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
                "\"actual_frame_kind\":0,",
                "\"actual_stream_id\":1,",
                "\"expected_stream_id\":1,",
                "\"started_frame_kind\":1,",
                "\"started_byte_offset\":0,",
                "\"active_continuation\":\"headers\"}}"
            )
        );
    }

    #[test]
    fn protocol_diagnostic_result_trace_decodes_byte_preview_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220696e76616c696420636c69656e7420636f6e6e656374696f6e20707265666163652061742062797465206f66667365742034",
            "\tprotocol_diagnostic\thttp2.protocol.invalid_preface\t4",
            "\t7\texpected_byte\tnumber\t42",
            "\tactual_byte\tnumber\t43",
            "\tmatched_prefix_count\tnumber\t4",
            "\texpected_count\tnumber\t24",
            "\tbyte_preview\tbyte_preview_v2\t35303532343932303262:5:5:false",
            "\tactive_state\tstring\t636f6e6e656374696f6e2d70726566616365",
            "\trule_provenance\tstring\t726663393131335f636c69656e745f636f6e6e656374696f6e5f70726566616365\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 invalid client connection preface at byte offset 4\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.invalid_preface\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":4},",
                "\"expected_byte\":42,",
                "\"actual_byte\":43,",
                "\"matched_prefix_count\":4,",
                "\"expected_count\":24,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"505249202b\",",
                "\"preview_byte_count\":5,",
                "\"total_byte_count\":5,",
                "\"truncated\":false},",
                "\"active_state\":\"connection-preface\",",
                "\"rule_provenance\":\"rfc9113_client_connection_preface\"}}"
            )
        );
    }

    #[test]
    fn peer_limit_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f32206672616d65207061796c6f6164206c656e67746820657863656564732072656365697665206d6178696d756d2061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.peer_limit.frame_size_exceeded\t0",
            "\t6\tobserved_payload_length\tnumber\t16385",
            "\tallowed_max_frame_size\tnumber\t16384",
            "\tframe_kind\tnumber\t0",
            "\tstream_id\tnumber\t3",
            "\tstream_ref\tstring\t73747265616d",
            "\treceive_limit_provenance\tstring\t70726f746f636f6c5f64656661756c74\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 frame payload length exceeds receive maximum at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.peer_limit.frame_size_exceeded\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"observed_payload_length\":16385,",
                "\"allowed_max_frame_size\":16384,",
                "\"frame_kind\":0,",
                "\"stream_id\":3,",
                "\"stream_ref\":\"stream\",",
                "\"receive_limit_provenance\":\"protocol_default\"}}"
            )
        );
    }

    #[test]
    fn header_list_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220686561646572206c6973742073697a6520657863656564732072656365697665206d6178696d756d2061742062797465206f6666736574203132",
            "\tprotocol_diagnostic\thttp2.peer_limit.header_list_size_exceeded\t12",
            "\t7\tobserved_header_list_size\tnumber\t10",
            "\tallowed_header_list_size\tnumber\t9",
            "\tframe_kind\tnumber\t9",
            "\tstream_id\tnumber\t1",
            "\tstream_ref\tstring\t73747265616d",
            "\treceive_limit_provenance\tstring\t6c6f63616c5f636f6e66696775726174696f6e",
            "\trule_provenance\tstring\t6865616465725f6c6973745f726563656976655f6c696d6974\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 header list size exceeds receive maximum at byte offset 12\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.peer_limit.header_list_size_exceeded\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":12},",
                "\"observed_header_list_size\":10,",
                "\"allowed_header_list_size\":9,",
                "\"frame_kind\":9,",
                "\"stream_id\":1,",
                "\"stream_ref\":\"stream\",",
                "\"receive_limit_provenance\":\"local_configuration\",",
                "\"rule_provenance\":\"header_list_receive_limit\"}}"
            )
        );
    }

    #[test]
    fn flow_control_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220666c6f772d636f6e74726f6c2077696e646f772065786365656465642061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.peer_limit.flow_control_window_exceeded\t0",
            "\t7\tobserved_payload_length\tnumber\t4",
            "\tallowed_window_credit\tnumber\t3",
            "\tframe_kind\tnumber\t0",
            "\tstream_id\tnumber\t1",
            "\tstream_ref\tstring\t73747265616d",
            "\tactive_state\tstring\t6f70656e2d73747265616d",
            "\trule_provenance\tstring\t73747265616d5f726563656976655f77696e646f77\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 flow-control window exceeded at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.peer_limit.flow_control_window_exceeded\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"observed_payload_length\":4,",
                "\"allowed_window_credit\":3,",
                "\"frame_kind\":0,",
                "\"stream_id\":1,",
                "\"stream_ref\":\"stream\",",
                "\"active_state\":\"open-stream\",",
                "\"rule_provenance\":\"stream_receive_window\"}}"
            )
        );
    }

    #[test]
    fn concurrent_streams_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220636f6e63757272656e742073747265616d2072656365697665206c696d69742065786365656465642061742062797465206f66667365742039",
            "\tprotocol_diagnostic\thttp2.peer_limit.concurrent_streams_exceeded\t9",
            "\t7\tstream_id\tnumber\t3",
            "\tstream_ref\tstring\t73747265616d",
            "\tattempted_concurrent_stream_count\tnumber\t2",
            "\tallowed_concurrent_stream_count\tnumber\t1",
            "\tactive_state\tstring\t6f70656e2d73747265616d",
            "\treceive_limit_provenance\tstring\t6c6f63616c5f636f6e66696775726174696f6e",
            "\trule_provenance\tstring\t706565725f637265617465645f73747265616d5f726563656976655f6c696d6974\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 concurrent stream receive limit exceeded at byte offset 9\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.peer_limit.concurrent_streams_exceeded\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
                "\"stream_id\":3,",
                "\"stream_ref\":\"stream\",",
                "\"attempted_concurrent_stream_count\":2,",
                "\"allowed_concurrent_stream_count\":1,",
                "\"active_state\":\"open-stream\",",
                "\"receive_limit_provenance\":\"local_configuration\",",
                "\"rule_provenance\":\"peer_created_stream_receive_limit\"}}"
            )
        );
    }

    #[test]
    fn invalid_frame_kind_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220696e76616c6964206672616d65206b696e642061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.protocol.invalid_frame_kind\t0",
            "\t7\tactual_frame_kind\tnumber\t0",
            "\tstream_id\tnumber\t0",
            "\tstream_ref\tstring\t636f6e6e656374696f6e",
            "\texpected_frame_kind\tnumber\t4",
            "\tbyte_preview\tbyte_preview_v2\t30303030303030303030303030303030:8:9:true",
            "\tactive_state\tstring\t636f6e6e656374696f6e2d636f6e74726f6c",
            "\trule_provenance\tstring\t636f6e6e656374696f6e5f6672616d65735f726571756972655f73657474696e6773\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 invalid frame kind at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.invalid_frame_kind\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"actual_frame_kind\":0,",
                "\"stream_id\":0,",
                "\"stream_ref\":\"connection\",",
                "\"expected_frame_kind\":4,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"0000000000000000\",",
                "\"preview_byte_count\":8,",
                "\"total_byte_count\":9,",
                "\"truncated\":true},",
                "\"active_state\":\"connection-control\",",
                "\"rule_provenance\":\"connection_frames_require_settings\"}}"
            )
        );
    }

    #[test]
    fn invalid_stream_id_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220696e76616c69642073747265616d2069642061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.protocol.invalid_stream_id\t0",
            "\t7\tframe_kind\tnumber\t1",
            "\tstream_id\tnumber\t2",
            "\tstream_ref\tstring\t73747265616d",
            "\trequired_stream_id_domain\tstring\t6e6f6e7a65726f20636c69656e742d696e697469617465642073747265616d206964",
            "\tendpoint_role\tstring\t736572766572",
            "\tactive_state\tstring\t73747265616d2d69642d646f6d61696e",
            "\trule_provenance\tstring\t7365727665725f72656365697665735f636c69656e745f696e697469617465645f73747265616d73\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 invalid stream id at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.invalid_stream_id\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"frame_kind\":1,",
                "\"stream_id\":2,",
                "\"stream_ref\":\"stream\",",
                "\"required_stream_id_domain\":\"nonzero client-initiated stream id\",",
                "\"endpoint_role\":\"server\",",
                "\"active_state\":\"stream-id-domain\",",
                "\"rule_provenance\":\"server_receives_client_initiated_streams\"}}"
            )
        );
    }

    #[test]
    fn invalid_payload_length_protocol_diagnostic_result_trace_keeps_byte_preview() {
        let trace = concat!(
            "result\t",
            "485454502f3220696e76616c6964207061796c6f6164206c656e6774682061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.protocol.invalid_payload_length\t0",
            "\t8\tframe_kind\tnumber\t6",
            "\tstream_id\tnumber\t0",
            "\tstream_ref\tstring\t636f6e6e656374696f6e",
            "\tobserved_payload_length\tnumber\t7",
            "\texpected_payload_length\tnumber\t8",
            "\tbyte_preview\tbyte_preview_v2\t3031303230333034303530363037:7:7:false",
            "\tactive_state\tstring\t636f6e6e656374696f6e2d636f6e74726f6c",
            "\trule_provenance\tstring\t726663393131335f70696e675f7061796c6f61645f6c656e677468\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 invalid payload length at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.invalid_payload_length\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"frame_kind\":6,",
                "\"stream_id\":0,",
                "\"stream_ref\":\"connection\",",
                "\"observed_payload_length\":7,",
                "\"expected_payload_length\":8,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"01020304050607\",",
                "\"preview_byte_count\":7,",
                "\"total_byte_count\":7,",
                "\"truncated\":false},",
                "\"active_state\":\"connection-control\",",
                "\"rule_provenance\":\"rfc9113_ping_payload_length\"}}"
            )
        );
    }

    #[test]
    fn stream_after_goaway_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f322073747265616d206f70656e656420616674657220677261636566756c2073687574646f776e2061742062797465206f66667365742039",
            "\tprotocol_diagnostic\thttp2.protocol.stream_after_goaway\t9",
            "\t7\tstream_id\tnumber\t7",
            "\tstream_ref\tstring\t73747265616d",
            "\tlast_stream_id\tnumber\t5",
            "\tshutdown_state\tstring\t677261636566756c5f73687574646f776e",
            "\tendpoint_role\tstring\t736572766572",
            "\tactive_state\tstring\t677261636566756c5f73687574646f776e",
            "\trule_provenance\tstring\t676f617761795f6c6173745f73747265616d5f6964\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 stream opened after graceful shutdown at byte offset 9\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.stream_after_goaway\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
                "\"stream_id\":7,",
                "\"stream_ref\":\"stream\",",
                "\"last_stream_id\":5,",
                "\"shutdown_state\":\"graceful_shutdown\",",
                "\"endpoint_role\":\"server\",",
                "\"active_state\":\"graceful_shutdown\",",
                "\"rule_provenance\":\"goaway_last_stream_id\"}}"
            )
        );
    }

    #[test]
    fn stream_invalid_frame_kind_protocol_diagnostic_result_trace_keeps_value_details() {
        let trace = concat!(
            "result\t",
            "485454502f3220696e76616c6964206672616d65206b696e642061742062797465206f66667365742030",
            "\tprotocol_diagnostic\thttp2.protocol.invalid_frame_kind\t0",
            "\t7\tactual_frame_kind\tnumber\t0",
            "\tstream_id\tnumber\t1",
            "\tstream_ref\tstring\t73747265616d",
            "\texpected_frame_kind\tnumber\t1",
            "\tbyte_preview\tbyte_preview_v2\t30303030303030303030303030303030:8:9:true",
            "\tactive_state\tstring\t69646c652d73747265616d",
            "\trule_provenance\tstring\t69646c655f73747265616d735f726571756972655f68656164657273\n",
        );

        let failure = result_failure_from_trace(trace).expect("trace should decode");

        assert_eq!(failure.kind, "result");
        assert_eq!(
            failure.details.to_json(),
            concat!(
                "{\"kind\":\"result\",\"phase\":\"runtime\",",
                "\"value\":\"HTTP/2 invalid frame kind at byte offset 0\",",
                "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
                "\"id\":\"http2.protocol.invalid_frame_kind\",",
                "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
                "\"actual_frame_kind\":0,",
                "\"stream_id\":1,",
                "\"stream_ref\":\"stream\",",
                "\"expected_frame_kind\":1,",
                "\"byte_preview\":{\"encoding\":\"hex\",",
                "\"data\":\"0000000000000000\",",
                "\"preview_byte_count\":8,",
                "\"total_byte_count\":9,",
                "\"truncated\":true},",
                "\"active_state\":\"idle-stream\",",
                "\"rule_provenance\":\"idle_streams_require_headers\"}}"
            )
        );
    }

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
            expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(
                ExpectedContractFailure {
                    clause: "require".to_string(),
                    predicate: "false".to_string(),
                    function: Some("reject".to_string()),
                    blame: Some("caller".to_string()),
                    span: span.clone(),
                },
            )),
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
            expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(
                ExpectedContractFailure {
                    clause: "require".to_string(),
                    predicate: "false".to_string(),
                    function: Some("reject".to_string()),
                    blame: Some("caller".to_string()),
                    span: span.clone(),
                },
            )),
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
            expected_runtime_failure: Some(ExpectedRuntimeFailure::Contract(
                ExpectedContractFailure {
                    clause: "require".to_string(),
                    predicate: "true".to_string(),
                    function: Some("reject".to_string()),
                    blame: Some("caller".to_string()),
                    span: span.clone(),
                },
            )),
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
