use super::*;

pub(super) fn extract_doctests(
    source: &SourceFile,
    signatures: &BTreeMap<String, Option<String>>,
) -> ExtractedDoctests {
    DoctestExtractor::new(source, signatures).extract()
}

pub(super) fn duplicate_output_diagnostic(
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

pub(super) fn doc_comment_content(line: &str) -> Option<&str> {
    let line = line.trim_start();
    line.strip_prefix("##")
}

pub(super) fn doctest_code_line(content: &str) -> String {
    if let Some(hidden) = content.strip_prefix("> ") {
        return hidden.to_string();
    }
    content.to_string()
}

pub(super) fn veln_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln")
}

pub(super) fn doctest_error_type(info: &str) -> Option<&str> {
    info.split_whitespace()
        .skip(1)
        .find_map(|field| field.strip_prefix("error="))
        .filter(|value| !value.is_empty())
}

pub(super) fn doctest_ignored(info: &str) -> bool {
    info.split_whitespace()
        .skip(1)
        .any(|field| field == "ignore")
}

pub(super) fn doctest_should_fail(info: &str) -> bool {
    info.split_whitespace().skip(1).any(|field| field == "fail")
}

pub(super) fn doctest_runtime_failure(
    info: &str,
    span: SourceSpan,
) -> Option<ExpectedRuntimeFailure> {
    RuntimeExpectationKind::from_value(metadata_value(info, RUNTIME_ATTRIBUTE)?)
        .and_then(|kind| kind.expected_failure(info, span))
}

pub(super) fn metadata_field_value<'a>(field: &'a str, name: &str) -> Option<&'a str> {
    let (attribute, value) = metadata_attribute_value(field)?;
    (attribute == name).then_some(value)
}

pub(super) fn metadata_value<'a>(info: &'a str, name: &str) -> Option<&'a str> {
    info.split_whitespace()
        .skip(1)
        .find_map(|field| metadata_field_value(field, name))
        .filter(|value| !value.is_empty())
}

pub(super) fn output_fence_info(info: &str) -> bool {
    info.split_whitespace().next() == Some("veln-output")
}

pub(super) fn output_fence_stream(info: &str) -> Option<&str> {
    let mut fields = info.split_whitespace();
    if fields.next()? != "veln-output" {
        return None;
    }
    let stream = fields.find_map(|field| metadata_field_value(field, "stream"))?;
    matches!(stream, "stdout" | "stderr").then_some(stream)
}

pub(super) fn veln_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
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

pub(super) fn veln_metadata_field_diagnostic(
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

pub(super) enum VelnMetadataField<'a> {
    Error(&'a str),
    Runtime(&'a str),
    RuntimeExpectation { attribute: &'a str, value: &'a str },
    Flag,
    Unknown(&'a str),
}

pub(super) fn classify_veln_metadata_field(field: &str) -> VelnMetadataField<'_> {
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

pub(super) fn empty_error_metadata_diagnostic(value: &str, span: SourceSpan) -> Option<Diagnostic> {
    value.is_empty().then(|| {
        invalid_doctest_metadata_diagnostic("empty doctest error type", "error", span, Vec::new())
    })
}

pub(super) fn runtime_metadata_diagnostic(value: &str, span: SourceSpan) -> Option<Diagnostic> {
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

pub(super) fn runtime_expectation_metadata_diagnostic(
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

pub(super) fn runtime_expectation_metadata_field(field: &str) -> Option<(&str, &str)> {
    let (attribute, value) = metadata_attribute_value(field)?;
    RUNTIME_CONTRACT_ATTRIBUTES
        .iter()
        .chain(RUNTIME_RESULT_ATTRIBUTES.iter())
        .any(|expected| *expected == attribute)
        .then_some((attribute, value))
}

pub(super) fn metadata_attribute_value(field: &str) -> Option<(&str, &str)> {
    let (attribute, value) = field.split_once('=')?;
    Some((attribute, value))
}

pub(super) fn output_metadata_diagnostics(info: &str, span: SourceSpan) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut has_stream = false;
    for field in info.split_whitespace().skip(1) {
        if let Some(stream) = field.strip_prefix("stream=") {
            if has_stream {
                diagnostics.push(invalid_doctest_metadata_diagnostic(
                    "duplicate doctest output stream attribute",
                    "stream",
                    span.clone(),
                    vec![("stream", JsonValue::string(stream))],
                ));
            }
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

pub(super) fn invalid_doctest_metadata_diagnostic(
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

pub(super) fn unknown_doctest_metadata_diagnostic(
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

pub(super) fn doctest_metadata_diagnostic(
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

pub(super) fn metadata_attribute_name(field: &str) -> &str {
    field.split_once('=').map_or(field, |(name, _)| name)
}

pub(super) fn with_error_type_context(
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
