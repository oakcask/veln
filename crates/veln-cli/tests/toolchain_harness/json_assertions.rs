use super::*;

pub(super) enum JsonPointerResult<'a> {
    Found(&'a JsonValue),
    Missing,
    Invalid(String),
}

pub(super) fn json_pointer<'a>(value: &'a JsonValue, tokens: &[String]) -> JsonPointerResult<'a> {
    let mut current = value;
    for token in tokens {
        match current {
            JsonValue::Object(entries) => {
                let Some((_, child)) = entries.iter().find(|(key, _)| key == token) else {
                    return JsonPointerResult::Missing;
                };
                current = child;
            }
            JsonValue::Array(values) => {
                if token != "0"
                    && (token.starts_with('0')
                        || token.is_empty()
                        || !token.bytes().all(|byte| byte.is_ascii_digit()))
                {
                    return JsonPointerResult::Invalid(format!(
                        "array token {token:?} is not a canonical non-negative index"
                    ));
                }
                let Ok(index) = token.parse::<usize>() else {
                    return JsonPointerResult::Invalid(format!(
                        "array token {token:?} exceeds the supported index range"
                    ));
                };
                let Some(child) = values.get(index) else {
                    return JsonPointerResult::Missing;
                };
                current = child;
            }
            _ => {
                return JsonPointerResult::Invalid(format!(
                    "token {token:?} cannot traverse {}",
                    current.to_compact_string()
                ));
            }
        }
    }
    JsonPointerResult::Found(current)
}

pub(super) fn json_pointer_route_mut<'a>(
    value: &'a mut JsonValue,
    route: &[JsonPointerRouteSegment],
) -> Option<&'a mut JsonValue> {
    let mut current = value;
    for segment in route {
        current = match (current, segment) {
            (JsonValue::Array(values), JsonPointerRouteSegment::ArrayIndex(index)) => {
                values.get_mut(*index)?
            }
            (
                JsonValue::Object(entries),
                JsonPointerRouteSegment::ObjectMember { key, occurrence },
            ) => entries
                .iter_mut()
                .filter(|(candidate, _)| candidate == key)
                .nth(*occurrence)
                .map(|(_, child)| child)?,
            _ => return None,
        };
    }
    Some(current)
}

pub(super) fn assert_json_path(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &JsonAssertion,
) {
    assert_json_path_in_workspace(context, json, assertion, 0, Path::new("."));
}

pub(super) fn assert_json_path_in_workspace(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &JsonAssertion,
    index: usize,
    project_root: &Path,
) {
    let operation = assertion
        .operation
        .as_ref()
        .expect("preflight requires one JSON assertion operation");
    if matches!(operation, ValueAssertionOperation::Missing) {
        assert!(
            json_path(json, &assertion.path).is_none(),
            "{}: json_assert {index}: JSON path `{}` exists but should be missing in {:?}",
            context.label(),
            assertion.path,
            json
        );
        return;
    }

    let actual = json_path(json, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: json_assert {index}: JSON path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            json
        )
    });
    let result = expect_value_assertion(actual, operation, project_root);
    result.unwrap_or_else(|error| {
        panic!(
            "{}: json_assert {index}: JSON path `{}` mismatch: {error}",
            context.label(),
            assertion.path
        )
    });
}

pub(super) fn assert_result_value_path(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &ResultValueAssertion,
) {
    assert_result_value_path_in_workspace(context, json, assertion, 0, Path::new("."));
}

pub(super) fn assert_result_value_path_in_workspace(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &ResultValueAssertion,
    index: usize,
    project_root: &Path,
) {
    let rendered = json_path(json, &assertion.value_path)
        .and_then(JsonValue::as_str)
        .unwrap_or_else(|| {
            panic!(
                "{}: result_value_assert {index}: result value source path `{}` was not found as a string in {:?}",
                context.label(),
                assertion.value_path,
                json
            )
        });
    let parsed = parse_result_value(rendered).unwrap_or_else(|error| {
        panic!(
            "{}: result_value_assert {index}: result value at `{}` could not be parsed: {error}\nvalue: {rendered}",
            context.label(),
            assertion.value_path
        )
    });

    let operation = assertion
        .operation
        .as_ref()
        .expect("preflight requires one result_value assertion operation");
    if matches!(operation, ValueAssertionOperation::Missing) {
        assert!(
            json_path(&parsed, &assertion.path).is_none(),
            "{}: result_value_assert {index}: result value path `{}` exists but should be missing in {:?}",
            context.label(),
            assertion.path,
            parsed
        );
        return;
    }

    let actual = json_path(&parsed, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: result_value_assert {index}: result value path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            parsed
        )
    });
    let result = expect_value_assertion(actual, operation, project_root);
    result.unwrap_or_else(|error| {
        panic!(
            "{}: result_value_assert {index}: result value path `{}` mismatch: {error}",
            context.label(),
            assertion.path
        )
    });
}

pub(super) fn expect_value_assertion(
    actual: &JsonValue,
    operation: &ValueAssertionOperation,
    project_root: &Path,
) -> Result<(), String> {
    match operation {
        ValueAssertionOperation::Equals(expected)
        | ValueAssertionOperation::EqualsFile(expected)
        | ValueAssertionOperation::EqualsJsonFile(expected) => expect_json_value(actual, expected),
        ValueAssertionOperation::Contains(expected) => expect_string_contains(actual, expected),
        ValueAssertionOperation::Length(expected) => expect_array_length(actual, *expected),
        ValueAssertionOperation::Missing => unreachable!("handled missing operation"),
        ValueAssertionOperation::WorkspaceFileUri(relative) => {
            expect_workspace_file_uri(actual, project_root, relative)
        }
    }
}

pub(super) fn assert_diagnostic(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    expected: &DiagnosticExpectation,
) {
    let diagnostics = json_path(json, "diagnostics")
        .and_then(JsonValue::as_array)
        .unwrap_or_else(|| panic!("{}: JSON diagnostics array missing", context.label()));

    let mut matches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_field(diagnostic, "id") == Some(expected.id.as_str()))
        .filter(|diagnostic| {
            expected
                .message
                .as_deref()
                .is_none_or(|message| diagnostic_field(diagnostic, "message") == Some(message))
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.file.as_deref())
                .is_none_or(|file| {
                    json_path(diagnostic, "span.file") == Some(&JsonValue::String(file.to_string()))
                })
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.line)
                .is_none_or(|line| {
                    json_path_equals(diagnostic, "span.start.line", &JsonValue::Number(line))
                })
        });

    let diagnostic = matches.next().unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{}` was not found in {:?}",
            context.label(),
            expected.id,
            diagnostics
        )
    });
    assert!(
        matches.next().is_none(),
        "{}: diagnostic `{}` matched more than one JSON diagnostic",
        context.label(),
        expected.id
    );

    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "severity",
        &expected.severity,
    );
    assert_diagnostic_field(context, diagnostic, &expected.id, "kind", &expected.kind);
    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "message",
        &expected.message,
    );
    if let Some(span) = &expected.span {
        if let Some(file) = &span.file {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.file",
                &JsonValue::String(file.clone()),
            );
        }
        if let Some(line) = span.line {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.line",
                &JsonValue::Number(line),
            );
        }
        if let Some(column) = span.column {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.column",
                &JsonValue::Number(column),
            );
        }
    }
}

pub(super) fn assert_diagnostic_field(
    context: &CaseRunContext<'_>,
    diagnostic: &JsonValue,
    id: &str,
    field: &str,
    expected: &Option<String>,
) {
    if let Some(expected) = expected {
        assert_json_equals(
            context,
            diagnostic,
            id,
            field,
            &JsonValue::String(expected.clone()),
        );
    }
}

pub(super) fn assert_json_equals(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    id: &str,
    path: &str,
    expected: &JsonValue,
) {
    let actual = json_path(json, path).unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{id}` JSON path `{path}` missing in {:?}",
            context.label(),
            json
        )
    });
    expect_json_value(actual, expected).unwrap_or_else(|error| {
        panic!(
            "{}: diagnostic `{id}` JSON path `{path}` mismatch: {error}",
            context.label()
        )
    });
}

pub(super) fn diagnostic_field<'a>(diagnostic: &'a JsonValue, field: &str) -> Option<&'a str> {
    json_path(diagnostic, field).and_then(JsonValue::as_str)
}

pub(super) fn json_path_equals(json: &JsonValue, path: &str, expected: &JsonValue) -> bool {
    json_path(json, path).is_some_and(|actual| json_values_equal(actual, expected))
}

pub(super) fn json_path<'a>(mut value: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    for segment in path.split('.') {
        value = if let Ok(index) = segment.parse::<usize>() {
            value.as_array()?.get(index)?
        } else {
            value.object_field(segment)?
        };
    }
    Some(value)
}
