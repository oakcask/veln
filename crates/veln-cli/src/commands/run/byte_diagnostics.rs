use super::*;

mod decode_reasons;

use decode_reasons::*;

pub(super) fn byte_result_failure_diagnostic(failure: &TestFailure) -> Option<Diagnostic> {
    let details = json_object(&failure.details)?;
    let byte_diagnostic = json_field(details, "byte_diagnostic")?;
    let byte_entries = json_object(byte_diagnostic)?;
    let id = json_string(byte_entries, "id")?;
    let byte_offset = byte_offset_value(byte_entries)?;

    if is_decode_error_result_failure(failure) {
        return Some(decode_error_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        ));
    }
    if id == "codec.incomplete_input" && is_decode_need_more_result_failure(failure) {
        return Some(decode_need_more_result_failure_diagnostic(
            failure,
            byte_diagnostic,
            byte_entries,
            id,
            byte_offset,
        ));
    }

    let mut diagnostic = codec_byte_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset)
        .or_else(|| schema_field_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset))
        .or_else(|| {
            schema_constraint_diagnostic(byte_diagnostic, byte_entries, &id, byte_offset)
        })?;
    push_field_path_note(&mut diagnostic, byte_entries);
    Some(diagnostic)
}

fn codec_byte_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "codec.incomplete_input" => incomplete_input_diagnostic(details, entries, id, byte_offset),
        "codec.byte_range_out_of_bounds" => {
            byte_range_out_of_bounds_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn schema_field_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "schema.fixed_field_mismatch" => {
            fixed_field_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        "schema.truncated_field" => truncated_field_diagnostic(details, entries, id, byte_offset),
        "schema.length_out_of_bounds" => {
            length_out_of_bounds_diagnostic(details, entries, id, byte_offset)
        }
        "schema.integer_out_of_range" => {
            integer_out_of_range_diagnostic(details, entries, id, byte_offset)
        }
        "schema.reserved_bits_mismatch" => {
            reserved_bits_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn schema_constraint_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    match id {
        "schema.validation_failed" => {
            validation_failed_diagnostic(details, entries, id, byte_offset)
        }
        "schema.length_division_by_zero" => {
            length_division_by_zero_diagnostic(details, entries, id, byte_offset)
        }
        "schema.length_multiple_mismatch" => {
            length_multiple_mismatch_diagnostic(details, entries, id, byte_offset)
        }
        "schema.dispatch_unknown_tag" => {
            dispatch_unknown_tag_diagnostic(details, entries, id, byte_offset)
        }
        _ => None,
    }
}

fn incomplete_input_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let readiness = json_string(entries, "readiness")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("missing byte at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "pending readiness is `{readiness}` because input is closed."
    )));
    diagnostic.related.push(note_json(format!(
        "Fixed-width read expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    Some(diagnostic)
}

fn byte_range_out_of_bounds_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let requested_count = json_number(entries, "requested_count")?;
    let available_count = json_number(entries, "available_count")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("byte range out of bounds at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Byte range requested {requested_count} byte(s); {available_count} byte(s) were available from the offset."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn fixed_field_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_value = json_number(entries, "expected_value")?;
    let actual_value = json_number(entries, "actual_value")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("fixed field mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Fixed field expected value {expected_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn truncated_field_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let readiness = json_string(entries, "readiness")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("truncated schema field at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "pending readiness is `{readiness}` because input is closed."
    )));
    diagnostic.related.push(note_json(format!(
        "Schema field expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_out_of_bounds_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let expected_count = json_number(entries, "expected_count")?;
    let available_count = json_number(entries, "available_count")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("payload length out of bounds at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Payload length expected {expected_count} byte(s); {available_count} byte(s) were available."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn integer_out_of_range_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let byte_width = json_number(entries, "byte_width")?;
    let min_value = json_number(entries, "min_value")?;
    let max_value = json_number(entries, "max_value")?;
    let actual_value = json_number_display(entries, "actual_value")
        .or_else(|| json_string(entries, "actual_value_text"))?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema integer out of range at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "{byte_width}-byte schema integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn reserved_bits_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let bit_width = json_number(entries, "bit_width")?;
    let expected_value = json_number(entries, "expected_value")?;
    let actual_value = json_number(entries, "actual_value")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("reserved bits mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "ReservedBits({bit_width}, {expected_value}) expected value {expected_value}; actual value was {actual_value}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn validation_failed_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let predicate = json_string(entries, "predicate")?;
    let decoded_values = json_string(entries, "decoded_values").or_else(|| {
        let length = json_number(entries, "length")?;
        let padding_length = json_number(entries, "padding_length")?;
        Some(format!("length={length}, padding_length={padding_length}"))
    })?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema validation failed at byte offset {byte_offset}"),
        details,
    );
    if let Some(field_value) = json_number(entries, "field_value") {
        diagnostic.related.push(note_json(format!(
            "Predicate `{predicate}` failed for field value {field_value}."
        )));
    } else {
        diagnostic
            .related
            .push(note_json(format!("Schema predicate `{predicate}` failed.")));
    }
    diagnostic
        .related
        .push(note_json(format!("Decoded values: {decoded_values}.")));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_division_by_zero_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let length_expression = json_string(entries, "length_expression")?;
    let divisor_operand = json_string(entries, "divisor_operand")?;
    let operator = json_string(entries, "operator")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("schema length division by zero at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Length expression `{length_expression}` evaluated `{operator}` with divisor operand `{divisor_operand}` equal to 0."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn length_multiple_mismatch_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let observed_count = json_number(entries, "observed_count")?;
    let required_multiple = json_number(entries, "required_multiple")?;
    let multiple_operand = json_string(entries, "multiple_operand")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("payload length multiple mismatch at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Payload count {observed_count} must be a multiple of `{multiple_operand}` value {required_multiple}."
    )));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn dispatch_unknown_tag_diagnostic(
    details: &JsonValue,
    entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Option<Diagnostic> {
    let tag_field = json_string(entries, "tag_field")?;
    let decoded_tag_value = json_number(entries, "decoded_tag_value")?;
    let expected_tags = json_string(entries, "expected_tags")?;
    let mut diagnostic = runtime_byte_diagnostic(
        id,
        format!("unknown dispatch tag at byte offset {byte_offset}"),
        details,
    );
    diagnostic.related.push(note_json(format!(
        "Dispatch tag field `{tag_field}` decoded value {decoded_tag_value}."
    )));
    diagnostic
        .related
        .push(note_json(format!("Expected tag values: {expected_tags}.")));
    push_byte_preview_note(&mut diagnostic, entries);
    Some(diagnostic)
}

fn runtime_byte_diagnostic(id: &str, message: String, details: &JsonValue) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        message,
        None,
        details.clone(),
    )
}

fn is_decode_error_result_failure(failure: &TestFailure) -> bool {
    result_failure_value(failure)
        .as_deref()
        .is_some_and(|value| {
            value.starts_with("DecodeError(") || value.starts_with("DecodeErrorWithReason(")
        })
}

fn is_decode_need_more_result_failure(failure: &TestFailure) -> bool {
    result_failure_value(failure)
        .as_deref()
        .is_some_and(|value| value.starts_with("NeedMore("))
}

fn decode_error_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: String,
    byte_offset: i64,
) -> Diagnostic {
    let reason_diagnostic = match id.as_str() {
        "codec.checksum_mismatch" => checksum_mismatch_result_failure_diagnostic,
        "codec.length_mismatch" => length_mismatch_result_failure_diagnostic,
        "codec.payload_length_mismatch" => payload_length_mismatch_result_failure_diagnostic,
        "codec.padding_mismatch" => padding_mismatch_result_failure_diagnostic,
        "codec.integer_out_of_range" => integer_out_of_range_result_failure_diagnostic,
        "codec.sequence_mismatch" => sequence_mismatch_result_failure_diagnostic,
        "codec.version_mismatch" => version_mismatch_result_failure_diagnostic,
        "codec.tag_mismatch" => tag_mismatch_result_failure_diagnostic,
        "codec.magic_mismatch" => magic_mismatch_result_failure_diagnostic,
        "codec.unsupported_feature" => unsupported_feature_result_failure_diagnostic,
        "codec.trailing_input" => trailing_input_result_failure_diagnostic,
        "codec.consumed_count_invalid" => consumed_count_invalid_result_failure_diagnostic,
        _ => {
            return generic_decode_error_result_failure_diagnostic(
                failure,
                byte_diagnostic,
                byte_entries,
                id,
                byte_offset,
            );
        }
    };
    reason_diagnostic(failure, byte_diagnostic, byte_entries, &id, byte_offset)
}

fn generic_decode_error_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: String,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        format!("decode error at byte offset {byte_offset}"),
        None,
        byte_diagnostic.clone(),
    );
    push_field_path_note(&mut diagnostic, byte_entries);
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("Decode failure reason: {reason}.")));
    }
    push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

fn push_field_path_note(diagnostic: &mut Diagnostic, byte_entries: &[(String, JsonValue)]) {
    if let Some(field_path) = field_path_text(byte_entries) {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    } else if let Some(field_path) = json_string(byte_entries, "field_path_display")
        && !field_path.is_empty()
    {
        diagnostic
            .related
            .push(note_json(format!("Field path: {field_path}.")));
    }
}

fn push_decode_byte_context_notes(diagnostic: &mut Diagnostic, entries: &[(String, JsonValue)]) {
    if let Some(local_byte_offset) = json_number(entries, "local_byte_offset") {
        diagnostic.related.push(note_json(format!(
            "Local byte offset: {local_byte_offset}."
        )));
    }
    if let (Some(expected_count), Some(available_count)) = (
        json_number(entries, "expected_count"),
        json_number(entries, "available_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Decoder expected {expected_count} byte(s); {available_count} byte(s) were available."
        )));
    }
    push_byte_preview_note(diagnostic, entries);
}

fn decode_need_more_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: String,
    byte_offset: i64,
) -> Diagnostic {
    let readiness = json_string(byte_entries, "readiness").unwrap_or_else(|| "unknown".to_string());
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        format!("incomplete input at byte offset {byte_offset}"),
        None,
        byte_diagnostic.clone(),
    );
    diagnostic.related.push(note_json(format!(
        "Decode readiness is `{readiness}` because input is closed."
    )));
    if let Some(needed_count) = json_number(byte_entries, "needed_count") {
        diagnostic.related.push(note_json(format!(
            "Decoder needs at least {needed_count} buffered byte(s) before retrying."
        )));
    }
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeStep value: {value}.")));
    }
    diagnostic
}
