use veln_diagnostics::{Diagnostic, JsonValue};
use veln_test::TestFailure;

pub(super) fn byte_offset_value(entries: &[(String, JsonValue)]) -> Option<i64> {
    let offset = json_field(entries, "byte_offset")?;
    let offset_entries = json_object(offset)?;
    json_number(offset_entries, "value")
}

pub(super) fn result_failure_value(failure: &TestFailure) -> Option<String> {
    let details = json_object(&failure.details)?;
    json_string(details, "value")
}

pub(super) fn field_path_text(entries: &[(String, JsonValue)]) -> Option<String> {
    let JsonValue::Array(segments) = json_field(entries, "field_path")? else {
        return None;
    };
    let mut parts = Vec::new();
    for segment in segments {
        let segment_entries = json_object(segment)?;
        let kind = json_string(segment_entries, "kind")?;
        let name = json_string(segment_entries, "name")?;
        parts.push(format!("{kind} `{name}`"));
    }
    (!parts.is_empty()).then(|| parts.join(" / "))
}

pub(super) fn push_byte_preview_note(diagnostic: &mut Diagnostic, entries: &[(String, JsonValue)]) {
    let context = byte_preview_note(entries).or_else(|| json_string(entries, "nearby_context"));
    if let Some(context) = context
        && !context.is_empty()
    {
        diagnostic
            .related
            .push(note_json(format!("Nearby bytes: {context}.")));
    }
}

fn byte_preview_note(entries: &[(String, JsonValue)]) -> Option<String> {
    let preview = json_field(entries, "byte_preview")?;
    let preview_entries = json_object(preview)?;
    let encoding = json_string(preview_entries, "encoding")?;
    if encoding != "hex" {
        return None;
    }
    let data = json_string(preview_entries, "data")?;
    let preview_byte_count = json_number(preview_entries, "preview_byte_count")?;
    let total_byte_count = json_number(preview_entries, "total_byte_count")?;
    let truncated = json_bool(preview_entries, "truncated")?;
    let state = if truncated { "truncated" } else { "complete" };
    let pairs = spaced_hex_pairs(&data)?;
    let preview_text = if pairs.is_empty() {
        "<empty>"
    } else {
        pairs.as_str()
    };
    Some(format!(
        "{preview_text} (showing {preview_byte_count} of {total_byte_count} byte(s), {state})"
    ))
}

fn spaced_hex_pairs(data: &str) -> Option<String> {
    if !data.len().is_multiple_of(2) {
        return None;
    }
    let mut parts = Vec::with_capacity(data.len() / 2);
    for index in (0..data.len()).step_by(2) {
        let pair = data.get(index..index + 2)?;
        if !pair
            .chars()
            .all(|character| character.is_ascii_digit() || ('a'..='f').contains(&character))
        {
            return None;
        }
        parts.push(pair);
    }
    Some(parts.join(" "))
}

pub(super) fn note_json(message: String) -> JsonValue {
    JsonValue::object([("message", JsonValue::string(message))])
}

pub(super) fn json_field<'a>(
    entries: &'a [(String, JsonValue)],
    key: &str,
) -> Option<&'a JsonValue> {
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

pub(super) fn json_object(value: &JsonValue) -> Option<&[(String, JsonValue)]> {
    match value {
        JsonValue::Object(entries) => Some(entries),
        _ => None,
    }
}

pub(super) fn json_string(entries: &[(String, JsonValue)], key: &str) -> Option<String> {
    match json_field(entries, key)? {
        JsonValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

pub(super) fn json_number(entries: &[(String, JsonValue)], key: &str) -> Option<i64> {
    match json_field(entries, key)? {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
}

pub(super) fn json_number_display(entries: &[(String, JsonValue)], key: &str) -> Option<String> {
    json_number(entries, key).map(|value| value.to_string())
}

fn json_bool(entries: &[(String, JsonValue)], key: &str) -> Option<bool> {
    match json_field(entries, key)? {
        JsonValue::Bool(value) => Some(*value),
        _ => None,
    }
}
