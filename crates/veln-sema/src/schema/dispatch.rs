use veln_literals::parse_integer_literal;

use super::primitives::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchSpec {
    pub(crate) tag_field: String,
    pub(crate) length_field: Option<String>,
    pub(crate) preserves_unknown: bool,
    pub(crate) cases: Vec<SchemaDispatchCase>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchCase {
    pub(crate) tag: i64,
    pub(crate) payload: SchemaDispatchCasePayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaDispatchCasePayload {
    Primitive { width: u8, little_endian: bool },
    ReservedBits { bit_width: u8, expected_value: i64 },
    Schema { schema_name: String },
}

pub(crate) fn schema_dispatch_has_schema_payload_where(
    dispatch: &SchemaDispatchSpec,
    mut predicate: impl FnMut(&str) -> bool,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name } if predicate(schema_name)
        )
    })
}

pub(crate) fn closed_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "Dispatch")?;
    let mut args = split_top_level_args(inner).into_iter().peekable();
    let tag_field = args.next()?.to_string();
    if !is_simple_schema_field_reference(&tag_field) {
        return None;
    }
    let length_field = args
        .peek()
        .filter(|arg| !arg.contains("=>"))
        .map(|arg| (*arg).to_string());
    if length_field
        .as_deref()
        .is_some_and(|length_field| !is_simple_schema_field_reference(length_field))
    {
        return None;
    }
    if length_field.is_some() {
        args.next();
    }
    let cases = schema_dispatch_cases(args)?;
    Some(SchemaDispatchSpec {
        tag_field,
        length_field,
        preserves_unknown: false,
        cases,
    })
}

pub(crate) fn extension_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "ExtensionDispatch")?;
    let mut args = split_top_level_args(inner).into_iter();
    let tag_field = args.next()?.to_string();
    let length_field = args.next()?.to_string();
    if !is_simple_schema_field_reference(&tag_field)
        || !is_simple_schema_field_reference(&length_field)
    {
        return None;
    }
    let cases = schema_dispatch_cases(args)?;
    Some(SchemaDispatchSpec {
        tag_field,
        length_field: Some(length_field),
        preserves_unknown: true,
        cases,
    })
}

fn schema_dispatch_cases<'a>(
    args: impl IntoIterator<Item = &'a str>,
) -> Option<Vec<SchemaDispatchCase>> {
    let cases = args
        .into_iter()
        .map(|arg| {
            let (tag, primitive) = arg.split_once("=>")?;
            let tag = parse_schema_tag(tag.trim())?;
            let payload = schema_dispatch_case_payload(primitive.trim())?;
            Some(SchemaDispatchCase { tag, payload })
        })
        .collect::<Option<Vec<_>>>()?;
    if cases.is_empty() {
        return None;
    }
    Some(cases)
}

fn schema_dispatch_case_payload(text: &str) -> Option<SchemaDispatchCasePayload> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(text) {
        let bit_width = dispatch_reserved_bits_width(bit_width, expected_value)?;
        return Some(SchemaDispatchCasePayload::ReservedBits {
            bit_width,
            expected_value,
        });
    }
    if let Some(width) = exact_width_schema_primitive(text) {
        if exact_width_schema_primitive_bit_width(text)? < 8 {
            return None;
        }
        return Some(SchemaDispatchCasePayload::Primitive {
            width,
            little_endian: exact_width_schema_primitive_little_endian(text),
        });
    }
    schema_payload_name_is_path(text).then(|| SchemaDispatchCasePayload::Schema {
        schema_name: text.to_string(),
    })
}

pub(crate) fn schema_dispatch_payload_accepts_lowercase_primitive(text: &str) -> bool {
    (lowercase_schema_primitive(text).is_some()
        || lowercase_reserved_bits_schema_primitive(text).is_some())
        && schema_dispatch_case_payload(text).is_some()
}

pub(crate) fn lowercase_schema_primitive_nested_payloads(ty: &str) -> Vec<(&str, &'static str)> {
    let mut payloads = Vec::new();
    if let Some(inner) = schema_call_inner(ty, "Repeat") {
        let args = inner
            .split(',')
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .collect::<Vec<_>>();
        if let [_, payload] = args.as_slice()
            && (lowercase_schema_primitive(payload).is_some()
                || lowercase_reserved_bits_schema_primitive(payload).is_some())
        {
            payloads.push((*payload, "repeat_payload"));
        }
    }
    for call_name in ["Dispatch", "ExtensionDispatch"] {
        if let Some(inner) = schema_call_inner(ty, call_name) {
            for arg in split_top_level_args(inner) {
                let Some((_, payload)) = arg.split_once("=>") else {
                    continue;
                };
                let payload = payload.trim();
                if lowercase_schema_primitive(payload).is_some()
                    || lowercase_reserved_bits_schema_primitive(payload).is_some()
                {
                    payloads.push((payload, "dispatch_payload"));
                }
            }
        }
    }
    payloads
}

fn split_top_level_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

fn parse_schema_tag(text: &str) -> Option<i64> {
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}
