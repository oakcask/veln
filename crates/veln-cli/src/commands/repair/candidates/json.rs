use veln_diagnostics::JsonValue;

pub(super) fn object_value<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(entries) = value else {
        return None;
    };
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

pub(super) fn object_array<'a>(value: &'a JsonValue, key: &str) -> Option<&'a Vec<JsonValue>> {
    match object_value(value, key)? {
        JsonValue::Array(values) => Some(values),
        _ => None,
    }
}

pub(super) fn object_string_array(value: &JsonValue, key: &str) -> Option<Vec<String>> {
    object_array(value, key).map(|values| {
        values
            .iter()
            .filter_map(|value| match value {
                JsonValue::String(value) => Some(value.clone()),
                _ => None,
            })
            .collect()
    })
}

pub(super) fn object_string<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    match object_value(value, key)? {
        JsonValue::String(value) => Some(value),
        _ => None,
    }
}

pub(super) fn object_number(value: &JsonValue, key: &str) -> Option<i64> {
    match object_value(value, key)? {
        JsonValue::Number(value) => Some(*value),
        JsonValue::Decimal(value) => json_integer_token_value(value),
        _ => None,
    }
}

fn json_integer_token_value(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'-')) {
        index = 1;
    }
    let first = bytes.get(index)?;
    match first {
        b'0' => index += 1,
        b'1'..=b'9' => {
            index += 1;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        _ => return None,
    }
    if index != bytes.len() {
        return None;
    }
    value.parse().ok()
}
