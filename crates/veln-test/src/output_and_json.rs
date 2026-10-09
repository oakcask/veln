use super::*;

pub(super) struct GeneratedDoctestSource {
    pub(super) text: String,
    pub(super) copied_regions: Vec<(TextRange, LineCol, LineCol)>,
}

pub(super) fn generated_doctest(name: &str, doctest: &ExtractedDoctest) -> GeneratedDoctestSource {
    let declaration_lines = declaration_line_indexes(doctest);
    let mut text = String::new();
    let mut copied_regions = Vec::with_capacity(doctest.code.len());
    for index in &declaration_lines {
        let line = &doctest.code[*index];
        let original = &doctest.source_locations[*index];
        let start = text.len();
        text.push_str(line);
        copied_regions.push((
            TextRange::new(start, text.len()),
            original.start,
            original.end,
        ));
        text.push('\n');
    }
    text.push_str(&generated_doctest_header(name, doctest));
    for (index, (line, original)) in doctest
        .code
        .iter()
        .zip(&doctest.source_locations)
        .enumerate()
    {
        if declaration_lines.contains(&index) {
            continue;
        }
        if !line.is_empty() {
            text.push_str("  ");
        }
        let start = text.len();
        text.push_str(line);
        copied_regions.push((
            TextRange::new(start, text.len()),
            original.start,
            original.end,
        ));
        text.push('\n');
    }
    if doctest.error_type.is_some() {
        text.push_str("  Ok(())\nend\n");
    } else {
        text.push_str("  ()\nend\n");
    }
    GeneratedDoctestSource {
        text,
        copied_regions,
    }
}

fn generated_doctest_header(name: &str, doctest: &ExtractedDoctest) -> String {
    let return_type = doctest.error_type.as_ref().map_or_else(
        || "()".to_string(),
        |error_type| format!("Result<(), {error_type}>"),
    );
    let item_kind = if doctest.should_fail { "fn" } else { "test" };
    format!("{item_kind} {name}() -> {return_type} effects [stdio]\n")
}

fn declaration_line_indexes(doctest: &ExtractedDoctest) -> BTreeSet<usize> {
    if doctest.should_fail {
        return BTreeSet::new();
    }
    let mut text = String::new();
    let mut line_ranges = Vec::with_capacity(doctest.code.len());
    for line in &doctest.code {
        let start = text.len();
        text.push_str(line);
        let end = text.len();
        text.push('\n');
        line_ranges.push(TextRange::new(start, end));
    }
    let parsed = veln_syntax::parse(&SourceFile::new("doctest.veln", text));
    let declaration_ranges = parsed
        .tree
        .items
        .iter()
        .map(|item| {
            let span = match item {
                veln_syntax::SyntaxItem::PublicAlias(alias) => &alias.span,
                veln_syntax::SyntaxItem::Effect(effect) => &effect.span,
                veln_syntax::SyntaxItem::Handler(handler) => &handler.span,
                veln_syntax::SyntaxItem::Type(type_decl) => &type_decl.span,
                veln_syntax::SyntaxItem::Schema(schema) => &schema.span,
                veln_syntax::SyntaxItem::Function(function) => &function.span,
            };
            TextRange::new(span.start.offset, span.end.offset)
        })
        .collect::<Vec<_>>();
    line_ranges
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            declaration_ranges
                .iter()
                .any(|declaration| declaration.start < line.end && line.start < declaration.end)
                .then_some(index)
        })
        .collect()
}

pub(super) fn reconstructed_stream(events: &[JsonValue], stream: &str) -> String {
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

pub(super) fn json_field<'a>(fields: &'a [(String, JsonValue)], key: &str) -> Option<&'a str> {
    fields.iter().find_map(|(field, value)| {
        if field == key {
            return value.as_text();
        }
        None
    })
}

pub(super) fn json_object_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    json_field(fields, key)
}

pub(super) fn normalize_lines(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    normalized
        .strip_suffix('\n')
        .unwrap_or(&normalized)
        .to_string()
}

pub(super) fn first_differing_line(expected: &str, actual: &str) -> OutputDifference {
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

pub(super) fn output_events_for_stream(events: &[JsonValue], stream: &str) -> Vec<JsonValue> {
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

pub(super) fn test_summary_to_json(cases: &[TestCase], suite_errors: &[SuiteError]) -> JsonValue {
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

pub(super) fn source_span_range_to_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("start", line_col_to_json(span.start)),
        ("end", line_col_to_json(span.end)),
    ])
}

pub(super) fn line_col_to_json(line_col: LineCol) -> JsonValue {
    JsonValue::object([
        ("line", JsonValue::Number(line_col.line as i64)),
        ("column", JsonValue::Number(line_col.column as i64)),
        ("offset", JsonValue::Number(line_col.offset as i64)),
    ])
}
