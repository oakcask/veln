use super::*;

#[derive(Clone, Copy)]
enum ByteContextNotes {
    Include,
    Omit,
}

fn start_decode_reason_diagnostic(
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    message: String,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Runtime,
        message,
        None,
        byte_diagnostic.clone(),
    );
    push_field_path_note(&mut diagnostic, byte_entries);
    diagnostic
}

fn finish_decode_reason_diagnostic(
    mut diagnostic: Diagnostic,
    failure: &TestFailure,
    byte_entries: &[(String, JsonValue)],
    reason_label: &str,
    byte_context_notes: ByteContextNotes,
) -> Diagnostic {
    if let Some(reason) = json_string(byte_entries, "reason") {
        diagnostic
            .related
            .push(note_json(format!("{reason_label}: {reason}.")));
    }
    if matches!(byte_context_notes, ByteContextNotes::Include) {
        push_decode_byte_context_notes(&mut diagnostic, byte_entries);
    }
    if let Some(value) = result_failure_value(failure) {
        diagnostic
            .related
            .push(note_json(format!("DecodeError value: {value}.")));
    }
    diagnostic
}

pub(super) fn checksum_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("checksum mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_checksum), Some(actual_checksum)) = (
        json_string(byte_entries, "expected_checksum"),
        json_string(byte_entries, "actual_checksum"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected checksum `{expected_checksum}`; actual checksum was `{actual_checksum}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Checksum failure reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn length_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("length mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_length), Some(actual_length)) = (
        json_number(byte_entries, "expected_length"),
        json_number(byte_entries, "actual_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected length {expected_length}; actual length was {actual_length}."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Length mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn payload_length_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("payload length mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_payload_length), Some(actual_payload_length)) = (
        json_number(byte_entries, "expected_payload_length"),
        json_number(byte_entries, "actual_payload_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected payload length {expected_payload_length}; actual payload length was {actual_payload_length}."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Payload length mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn padding_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("padding mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_padding_length), Some(actual_padding_length)) = (
        json_number(byte_entries, "expected_padding_length"),
        json_number(byte_entries, "actual_padding_length"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected padding length {expected_padding_length}; actual padding length was {actual_padding_length}."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Padding mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn integer_out_of_range_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("integer out of range at byte offset {byte_offset}"),
    );
    if let (Some(byte_width), Some(min_value), Some(max_value), Some(actual_value)) = (
        json_number(byte_entries, "byte_width"),
        json_number(byte_entries, "min_value"),
        json_number(byte_entries, "max_value"),
        json_number(byte_entries, "actual_value"),
    ) {
        diagnostic.related.push(note_json(format!(
            "{byte_width}-byte integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Integer conversion reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn sequence_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("sequence mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_sequence), Some(actual_sequence)) = (
        json_string(byte_entries, "expected_sequence"),
        json_string(byte_entries, "actual_sequence"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected sequence `{expected_sequence}`; actual sequence was `{actual_sequence}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Sequence mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn tag_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("tag mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_tag), Some(actual_tag)) = (
        json_string(byte_entries, "expected_tag"),
        json_string(byte_entries, "actual_tag"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected tag `{expected_tag}`; actual tag was `{actual_tag}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Tag mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn magic_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("magic mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_magic), Some(actual_magic)) = (
        json_string(byte_entries, "expected_magic"),
        json_string(byte_entries, "actual_magic"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected magic `{expected_magic}`; actual magic was `{actual_magic}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Magic mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn version_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("version mismatch at byte offset {byte_offset}"),
    );
    if let (Some(expected_version), Some(actual_version)) = (
        json_string(byte_entries, "expected_version"),
        json_string(byte_entries, "actual_version"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Expected version `{expected_version}`; actual version was `{actual_version}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Version mismatch reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn unsupported_feature_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("unsupported feature failed at byte offset {byte_offset}"),
    );
    if let Some(unsupported_feature) = json_string(byte_entries, "unsupported_feature") {
        diagnostic.related.push(note_json(format!(
            "Unsupported feature: `{unsupported_feature}`."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Unsupported feature reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn trailing_input_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("trailing input at byte offset {byte_offset}"),
    );
    if let (Some(consumed_count), Some(available_count), Some(remaining_count)) = (
        json_number(byte_entries, "consumed_count"),
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "remaining_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Consumed {consumed_count} of {available_count} available bytes; {remaining_count} bytes remain."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Trailing input reason",
        ByteContextNotes::Include,
    )
}

pub(super) fn consumed_count_invalid_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(
        byte_diagnostic,
        byte_entries,
        id,
        format!("invalid decoded consumed count at byte offset {byte_offset}"),
    );
    if let (Some(available_count), Some(actual_consumed_count)) = (
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "actual_consumed_count"),
    ) {
        diagnostic.related.push(note_json(format!(
            "Decoder consumed {actual_consumed_count} byte(s); supplied view length was {available_count} byte(s)."
        )));
    }
    finish_decode_reason_diagnostic(
        diagnostic,
        failure,
        byte_entries,
        "Consumed count reason",
        ByteContextNotes::Omit,
    )
}
