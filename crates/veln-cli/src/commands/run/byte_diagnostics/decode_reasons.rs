use super::*;

fn finish_decode_reason_diagnostic_without_byte_context(
    mut diagnostic: Diagnostic,
    failure: &TestFailure,
    byte_entries: &[(String, JsonValue)],
    reason_label: &str,
) -> Diagnostic {
    push_decode_reason_note(&mut diagnostic, byte_entries, reason_label);
    push_decode_error_value_note(&mut diagnostic, failure);
    diagnostic
}

fn decode_reason_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    summary: String,
    reason_label: &str,
    related_note: Option<String>,
    include_byte_context: bool,
) -> Diagnostic {
    let mut diagnostic = start_decode_reason_diagnostic(byte_diagnostic, byte_entries, id, summary);
    if let Some(related_note) = related_note {
        diagnostic.related.push(note_json(related_note));
    }
    if include_byte_context {
        finish_decode_reason_diagnostic(diagnostic, failure, byte_entries, reason_label)
    } else {
        finish_decode_reason_diagnostic_without_byte_context(
            diagnostic,
            failure,
            byte_entries,
            reason_label,
        )
    }
}

pub(super) fn checksum_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_string(byte_entries, "expected_checksum"),
        json_string(byte_entries, "actual_checksum"),
    ) {
        (Some(expected_checksum), Some(actual_checksum)) => Some(format!(
            "Expected checksum `{expected_checksum}`; actual checksum was `{actual_checksum}`."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("checksum mismatch at byte offset {byte_offset}"),
        "Checksum failure reason",
        related_note,
        true,
    )
}

pub(super) fn length_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "expected_length"),
        json_number(byte_entries, "actual_length"),
    ) {
        (Some(expected_length), Some(actual_length)) => Some(format!(
            "Expected length {expected_length}; actual length was {actual_length}."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("length mismatch at byte offset {byte_offset}"),
        "Length mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn payload_length_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "expected_payload_length"),
        json_number(byte_entries, "actual_payload_length"),
    ) {
        (Some(expected_payload_length), Some(actual_payload_length)) => Some(format!(
            "Expected payload length {expected_payload_length}; actual payload length was {actual_payload_length}."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("payload length mismatch at byte offset {byte_offset}"),
        "Payload length mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn padding_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "expected_padding_length"),
        json_number(byte_entries, "actual_padding_length"),
    ) {
        (Some(expected_padding_length), Some(actual_padding_length)) => Some(format!(
            "Expected padding length {expected_padding_length}; actual padding length was {actual_padding_length}."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("padding mismatch at byte offset {byte_offset}"),
        "Padding mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn integer_out_of_range_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "byte_width"),
        json_number(byte_entries, "min_value"),
        json_number(byte_entries, "max_value"),
        json_number(byte_entries, "actual_value"),
    ) {
        (Some(byte_width), Some(min_value), Some(max_value), Some(actual_value)) => Some(format!(
            "{byte_width}-byte integer expected value between {min_value} and {max_value}; actual value was {actual_value}."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("integer out of range at byte offset {byte_offset}"),
        "Integer conversion reason",
        related_note,
        true,
    )
}

pub(super) fn sequence_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_string(byte_entries, "expected_sequence"),
        json_string(byte_entries, "actual_sequence"),
    ) {
        (Some(expected_sequence), Some(actual_sequence)) => Some(format!(
            "Expected sequence `{expected_sequence}`; actual sequence was `{actual_sequence}`."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("sequence mismatch at byte offset {byte_offset}"),
        "Sequence mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn tag_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_string(byte_entries, "expected_tag"),
        json_string(byte_entries, "actual_tag"),
    ) {
        (Some(expected_tag), Some(actual_tag)) => Some(format!(
            "Expected tag `{expected_tag}`; actual tag was `{actual_tag}`."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("tag mismatch at byte offset {byte_offset}"),
        "Tag mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn magic_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_string(byte_entries, "expected_magic"),
        json_string(byte_entries, "actual_magic"),
    ) {
        (Some(expected_magic), Some(actual_magic)) => Some(format!(
            "Expected magic `{expected_magic}`; actual magic was `{actual_magic}`."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("magic mismatch at byte offset {byte_offset}"),
        "Magic mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn version_mismatch_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_string(byte_entries, "expected_version"),
        json_string(byte_entries, "actual_version"),
    ) {
        (Some(expected_version), Some(actual_version)) => Some(format!(
            "Expected version `{expected_version}`; actual version was `{actual_version}`."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("version mismatch at byte offset {byte_offset}"),
        "Version mismatch reason",
        related_note,
        true,
    )
}

pub(super) fn unsupported_feature_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = json_string(byte_entries, "unsupported_feature")
        .map(|unsupported_feature| format!("Unsupported feature: `{unsupported_feature}`."));
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("unsupported feature failed at byte offset {byte_offset}"),
        "Unsupported feature reason",
        related_note,
        true,
    )
}

pub(super) fn trailing_input_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "consumed_count"),
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "remaining_count"),
    ) {
        (Some(consumed_count), Some(available_count), Some(remaining_count)) => Some(format!(
            "Consumed {consumed_count} of {available_count} available bytes; {remaining_count} bytes remain."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("trailing input at byte offset {byte_offset}"),
        "Trailing input reason",
        related_note,
        true,
    )
}

pub(super) fn consumed_count_invalid_result_failure_diagnostic(
    failure: &TestFailure,
    byte_diagnostic: &JsonValue,
    byte_entries: &[(String, JsonValue)],
    id: &str,
    byte_offset: i64,
) -> Diagnostic {
    let related_note = match (
        json_number(byte_entries, "available_count"),
        json_number(byte_entries, "actual_consumed_count"),
    ) {
        (Some(available_count), Some(actual_consumed_count)) => Some(format!(
            "Decoder consumed {actual_consumed_count} byte(s); supplied view length was {available_count} byte(s)."
        )),
        _ => None,
    };
    decode_reason_result_failure_diagnostic(
        failure,
        byte_diagnostic,
        byte_entries,
        id,
        format!("invalid decoded consumed count at byte offset {byte_offset}"),
        "Consumed count reason",
        related_note,
        false,
    )
}
