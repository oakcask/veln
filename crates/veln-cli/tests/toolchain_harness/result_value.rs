use super::assertion_json::{JsonValue, parse_json};

pub(super) fn parse_result_value(rendered_value: &str) -> Result<JsonValue, String> {
    let trimmed = rendered_value.trim();
    if let Some(inner) = constructor_arg(trimmed, "Err") {
        return parse_veln_value(trimmed).or_else(|_| {
            Ok(result_value_object(
                "Err",
                vec![("value", parse_veln_value(inner)?)],
            ))
        });
    }
    Ok(result_value_object(
        "Err",
        vec![("value", parse_veln_value(trimmed)?)],
    ))
}

#[derive(Clone, Copy)]
enum VelnFieldKind {
    List,
    Text,
    Value,
}

type VelnConstructorField = (&'static str, VelnFieldKind);
type VelnConstructorSchema = (&'static str, &'static [VelnConstructorField]);

use VelnFieldKind::{List, Text, Value};

const VELN_CONSTRUCTOR_SCHEMAS: &[VelnConstructorSchema] = &[
    ("Err", &[("value", Value)]),
    (
        "RuntimeDiagnostic",
        &[("id", Text), ("message", Text), ("detail", Value)],
    ),
    (
        "RuntimeByteDiagnostic",
        &[
            ("byte_offset", Value),
            ("field_path", List),
            ("facts", Value),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeValueDiagnostic",
        &[("field_path", List), ("reason", Text)],
    ),
    ("RuntimeHttp2Diagnostic", &[("detail", Value)]),
    ("RuntimeHttp2HpackDiagnostic", &[("detail", Value)]),
    (
        "RuntimeHpackFixtureDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureDynamicIndexDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("requested_dynamic_index", Value),
            ("dynamic_table_entry_count", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureDynamicNameDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("requested_dynamic_index", Value),
            ("dynamic_table_entry_count", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureTableSizeUpdateDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("observed_header_table_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("active_state", Text),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPartialPrefaceDiagnostic",
        &[
            ("byte_offset", Value),
            ("pending_count", Value),
            ("expected_count", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic",
        &[
            ("byte_offset", Value),
            ("expected_byte", Value),
            ("actual_byte", Value),
            ("matched_prefix_count", Value),
            ("expected_count", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolClosedWithPendingDiagnostic",
        &[
            ("byte_offset", Value),
            ("pending_count", Value),
            ("active_continuation", Text),
            ("expected_stream_id", Value),
            ("started_frame_kind", Value),
            ("started_byte_offset", Value),
            ("accumulated_header_block_bytes", Value),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolContinuationExpectedDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("actual_stream_id", Value),
            ("expected_stream_id", Value),
            ("started_frame_kind", Value),
            ("started_byte_offset", Value),
            ("active_continuation", Text),
            ("accumulated_header_block_bytes", Value),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("stream_id", Value),
            ("expected_frame_kind", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("required_stream_id_domain", Text),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("previous_peer_stream_id", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitFrameSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_payload_length", Value),
            ("allowed_max_frame_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_list_size", Value),
            ("allowed_header_list_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitSettingsValueDiagnostic",
        &[
            ("byte_offset", Value),
            ("setting_identifier", Value),
            ("setting_name", Text),
            ("observed_value", Value),
            ("accepted_min_value", Value),
            ("accepted_max_value", Value),
            ("peer_limit_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("observed_payload_length", Value),
            ("expected_payload_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("pad_length", Value),
            ("remaining_payload_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_payload_length", Value),
            ("allowed_window_credit", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("expected_content_length", Value),
            ("observed_body_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_table_size", Value),
            ("allowed_header_table_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("attempted_concurrent_stream_count", Value),
            ("allowed_concurrent_stream_count", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("failed_header_fact", Text),
            ("header_name", Text),
            ("decoded_header_names", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("failed_header_fact", Text),
            ("header_name", Text),
            ("decoded_header_names", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("observed_window_increment", Value),
            ("accepted_min_window_increment", Value),
            ("accepted_max_window_increment", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic",
        &[
            ("byte_offset", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("actual_flags", Value),
            ("stream_id", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic",
        &[
            ("byte_offset", Value),
            ("setting_identifier", Value),
            ("setting_name", Text),
            ("endpoint_role", Text),
            ("frame_kind", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPriorityDependencyDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("dependency_stream_id", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("last_stream_id", Value),
            ("shutdown_state", Text),
            ("endpoint_role", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeDiagnosticFieldPathSegment",
        &[("kind", Text), ("name", Text)],
    ),
    (
        "RuntimeByteCountFacts",
        &[
            ("expected_count", Value),
            ("available_count", Value),
            ("readiness", Text),
        ],
    ),
    (
        "RuntimeByteRangeFacts",
        &[("requested_count", Value), ("available_count", Value)],
    ),
    (
        "RuntimeByteFixedValueFacts",
        &[("expected_value", Value), ("actual_value", Value)],
    ),
    ("RuntimeByteReasonFacts", &[("reason", Text)]),
];

pub(super) fn parse_veln_value(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(JsonValue::Array(Vec::new()));
    }
    if text == "NoRuntimeBytePreview" {
        return Ok(result_value_object("NoRuntimeBytePreview", Vec::new()));
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Ok(parse_veln_atom(text));
    };

    if let Some(schema) = VELN_CONSTRUCTOR_SCHEMAS
        .iter()
        .find(|(constructor, _)| *constructor == name)
    {
        return parse_veln_constructor(name, args, schema.1);
    }

    match name {
        "RuntimeBytePreview" => parse_runtime_byte_preview(name, args),
        "ByteChunk" => parse_byte_chunk(name, args),
        "Byte" | "ByteOffset" | "ByteCount" => parse_byte_measure(name, args),
        "Cons" => Ok(JsonValue::Array(parse_veln_list_items(text)?)),
        _ => parse_unknown_veln_constructor(name, args),
    }
}

fn parse_veln_constructor(
    name: &str,
    args: Vec<&str>,
    fields: &[VelnConstructorField],
) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, fields.len())?;
    let fields = fields
        .iter()
        .zip(args)
        .map(|((field, kind), value)| Ok((*field, parse_veln_field(*kind, value)?)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(result_value_object(name, fields))
}

fn parse_veln_field(kind: VelnFieldKind, text: &str) -> Result<JsonValue, String> {
    match kind {
        List => parse_veln_list(text),
        Text => Ok(JsonValue::String(text.trim().to_string())),
        Value => parse_veln_value(text),
    }
}

fn parse_runtime_byte_preview(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 4)?;
    Ok(result_value_object(
        name,
        vec![
            ("encoding", JsonValue::String("hex".to_string())),
            ("data", JsonValue::String(args[0].trim().to_string())),
            ("preview_byte_count", parse_veln_value(args[1])?),
            ("total_byte_count", parse_veln_value(args[2])?),
            ("truncated", parse_veln_value(args[3])?),
        ],
    ))
}

fn parse_byte_chunk(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 1)?;
    Ok(result_value_object(
        name,
        vec![("bytes", parse_veln_bracketed_list(args[0])?)],
    ))
}

fn parse_byte_measure(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 1)?;
    Ok(result_value_object(
        name,
        vec![("value", parse_veln_nonnegative_integer(name, args[0])?)],
    ))
}

fn parse_unknown_veln_constructor(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    Ok(result_value_object(
        name,
        vec![(
            "fields",
            JsonValue::Array(
                args.into_iter()
                    .map(parse_veln_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        )],
    ))
}

fn parse_veln_list(text: &str) -> Result<JsonValue, String> {
    Ok(JsonValue::Array(parse_veln_list_items(text)?))
}

fn parse_veln_list_items(text: &str) -> Result<Vec<JsonValue>, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(Vec::new());
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Err(format!("expected list value, got `{text}`"));
    };
    if name != "Cons" {
        return Err(format!("expected `Cons` or `Nil`, got `{name}`"));
    }
    let args = expect_arity(name, args, 2)?;
    let mut values = vec![parse_veln_value(args[0])?];
    values.extend(parse_veln_list_items(args[1])?);
    Ok(values)
}

fn parse_veln_bracketed_list(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    let inner = text
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("expected bracketed list, got `{text}`"))?;
    if inner.trim().is_empty() {
        return Ok(JsonValue::Array(Vec::new()));
    }
    Ok(JsonValue::Array(
        split_top_level_args(inner)
            .into_iter()
            .map(parse_veln_value)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_veln_atom(text: &str) -> JsonValue {
    match text {
        "true" => JsonValue::Bool(true),
        "false" => JsonValue::Bool(false),
        _ => parse_veln_number_atom(text).unwrap_or_else(|| JsonValue::String(text.to_string())),
    }
}

fn parse_veln_number_atom(text: &str) -> Option<JsonValue> {
    let JsonValue::Decimal(raw) = parse_json(text).ok()? else {
        return None;
    };
    if let Ok(value) = raw.parse::<i64>()
        && value.to_string() == raw
    {
        return Some(JsonValue::Number(value));
    }
    Some(JsonValue::Decimal(raw))
}

fn parse_veln_nonnegative_integer(name: &str, text: &str) -> Result<JsonValue, String> {
    let value = text
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("`{name}` expects an integer payload, got `{}`", text.trim()))?;
    Ok(JsonValue::Number(value))
}

fn split_constructor_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let name = text[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    Some((name, split_top_level_args(inner)))
}

fn constructor_arg<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    text.strip_prefix(&prefix)?.strip_suffix(')')
}

fn split_top_level_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                args.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        args.push(tail);
    }
    args
}

fn expect_arity<'a>(
    name: &str,
    args: Vec<&'a str>,
    expected: usize,
) -> Result<Vec<&'a str>, String> {
    if args.len() == expected {
        Ok(args)
    } else {
        Err(format!(
            "`{name}` expects {expected} argument(s), got {}",
            args.len()
        ))
    }
}

fn result_value_object(constructor: &str, fields: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut entries = vec![(
        "constructor".to_string(),
        JsonValue::String(constructor.to_string()),
    )];
    entries.extend(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    JsonValue::Object(entries)
}
