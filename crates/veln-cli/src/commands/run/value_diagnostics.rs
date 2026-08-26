use super::*;

pub(super) fn value_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let value_diagnostic = json_field(details, "value_diagnostic")?;
    let value_entries = json_object(value_diagnostic)?;
    let id = json_string(value_entries, "id")?;

    match id.as_str() {
        "schema.validation_failed" => {
            let predicate = json_string(value_entries, "predicate")?;
            let supplied_values = json_string(value_entries, "supplied_values")?;
            let result_value = result_failure_value(failure)?;
            let encode_result = result_value.starts_with("EncodeError(schema.validation_failed,");
            let mut diagnostic = Diagnostic::new(
                id,
                Severity::Error,
                DiagnosticKind::Runtime,
                if encode_result {
                    "schema encode validation failed".to_string()
                } else {
                    result_value.clone()
                },
                None,
                value_diagnostic.clone(),
            );
            if let Some(field_value) = json_number(value_entries, "field_value") {
                diagnostic.related.push(note_json(format!(
                    "Predicate `{predicate}` failed for supplied field value {field_value}."
                )));
            } else {
                diagnostic
                    .related
                    .push(note_json(format!("Schema predicate `{predicate}` failed.")));
            }
            diagnostic
                .related
                .push(note_json(format!("Supplied values: {supplied_values}.")));
            if let Some(field_path) = field_path_text(value_entries) {
                diagnostic
                    .related
                    .push(note_json(format!("Field path: {field_path}.")));
            }
            if encode_result {
                diagnostic
                    .related
                    .push(note_json(format!("Result value: {result_value}.")));
            }
            Some(diagnostic)
        }
        "schema.encode_value_unrepresentable" | "codec.encode_value_unrepresentable" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_unknown_tag" | "codec.dispatch_unknown_tag" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_length_mismatch" | "codec.dispatch_length_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "schema.dispatch_mismatch" | "codec.dispatch_mismatch" => {
            encode_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        "codec.byte_write_value_unrepresentable" => {
            byte_write_result_failure_diagnostic(failure, value_diagnostic, value_entries)
        }
        _ => None,
    }
}

fn byte_write_result_failure_diagnostic(
    failure: &TestFailure,
    value_diagnostic: &JsonValue,
    value_entries: &[(String, JsonValue)],
) -> Option<Diagnostic> {
    let id = json_string(value_entries, "id")?;
    let helper_name = json_string(value_entries, "helper_name")?;
    let supplied_value = json_number(value_entries, "supplied_value")?;
    let min_value = json_number(value_entries, "min_value")?;
    let max_value = json_number(value_entries, "max_value")?;
    let width = json_number(value_entries, "width")?;
    let byte_order = json_string(value_entries, "byte_order")?;
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        "byte write value is unrepresentable",
        None,
        value_diagnostic.clone(),
    );
    diagnostic.related.push(note_json(format!(
        "Byte write helper `{helper_name}` received value {supplied_value}."
    )));
    diagnostic.related.push(note_json(format!(
        "Accepted range is {min_value}..{max_value}."
    )));
    diagnostic.related.push(note_json(format!(
        "Write width is {width} byte(s) with `{byte_order}` byte order."
    )));
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("Result value: {value}.")));
    }
    Some(diagnostic)
}

fn encode_result_failure_diagnostic(
    failure: &TestFailure,
    value_diagnostic: &JsonValue,
    value_entries: &[(String, JsonValue)],
) -> Option<Diagnostic> {
    let id = json_string(value_entries, "id")?;
    let reason = json_string(value_entries, "reason")?;
    let mut diagnostic = Diagnostic::new(
        id.clone(),
        Severity::Error,
        DiagnosticKind::Runtime,
        encode_diagnostic_message(&id),
        None,
        value_diagnostic.clone(),
    );
    if let Some(field_path) = field_path_text(value_entries) {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    } else if let Some(field_path) = json_string(value_entries, "field_path_display") {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
    diagnostic
        .related
        .push(note_json(format!("Encode failure reason: {reason}.")));
    if let (Some(expected_count), Some(actual_count)) = (
        json_number(value_entries, "expected_count"),
        json_number(value_entries, "actual_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected {expected_count} byte(s); supplied ByteView has {actual_count} byte(s)."
        )));
    }
    if let Some(byte_offset) = json_number(value_entries, "byte_offset") {
        diagnostic.related.push(note_json(format!(
            "Supplied ByteView starts at byte offset {byte_offset}."
        )));
    }
    push_byte_preview_note(&mut diagnostic, value_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("Result value: {value}.")));
    }
    Some(diagnostic)
}

fn encode_diagnostic_message(id: &str) -> String {
    match id {
        "schema.encode_value_unrepresentable" | "codec.encode_value_unrepresentable" => {
            "encode value is unrepresentable"
        }
        "schema.dispatch_unknown_tag" | "codec.dispatch_unknown_tag" => {
            "unknown dispatch tag in encode value"
        }
        "schema.dispatch_length_mismatch" | "codec.dispatch_length_mismatch" => {
            "dispatch payload length mismatch"
        }
        "schema.dispatch_mismatch" | "codec.dispatch_mismatch" => {
            "dispatch tag and payload mismatch"
        }
        _ => "encode failed",
    }
    .to_string()
}
