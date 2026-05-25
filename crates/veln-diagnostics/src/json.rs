#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn array(values: impl IntoIterator<Item = JsonValue>) -> Self {
        Self::Array(values.into_iter().collect())
    }

    pub fn object<K, I>(entries: I) -> Self
    where
        K: Into<String>,
        I: IntoIterator<Item = (K, JsonValue)>,
    {
        Self::Object(
            entries
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => out.push_str(&value.to_string()),
            Self::String(value) => write_json_string(out, value),
            Self::Array(values) => {
                out.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    value.write_json(out);
                }
                out.push(']');
            }
            Self::Object(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_json_string(out, key);
                    out.push(':');
                    value.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch.is_control() => {
                out.push_str("\\u");
                out.push_str(&format!("{:04x}", ch as u32));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_string_escapes_quotes_backslashes_and_control_characters() {
        let value = JsonValue::object([
            (
                "text",
                JsonValue::string("quote \" slash \\ newline\n tab\t backspace\u{08} form\u{0c}"),
            ),
            ("control", JsonValue::string("\u{01}")),
        ]);

        assert_eq!(
            value.to_json(),
            "{\"text\":\"quote \\\" slash \\\\ newline\\n tab\\t backspace\\b form\\f\",\"control\":\"\\u0001\"}"
        );
    }

    #[test]
    fn json_string_escapes_carriage_returns() {
        let value = JsonValue::string("left\rright");

        assert_eq!(value.to_json(), "\"left\\rright\"");
    }

    #[test]
    fn json_string_escapes_nul_as_control_escape() {
        let value = JsonValue::string("left\0right");

        assert_eq!(value.to_json(), "\"left\\u0000right\"");
    }

    #[test]
    fn json_values_render_empty_arrays_and_objects() {
        let value = JsonValue::object([
            ("items", JsonValue::array([])),
            ("metadata", JsonValue::object::<&str, _>([])),
        ]);

        assert_eq!(value.to_json(), "{\"items\":[],\"metadata\":{}}");
    }

    #[test]
    fn json_values_render_nested_arrays_and_objects_in_input_order() {
        let value = JsonValue::object([
            ("ok", JsonValue::Bool(true)),
            (
                "items",
                JsonValue::array([
                    JsonValue::Number(1),
                    JsonValue::Null,
                    JsonValue::object([("name", JsonValue::string("main"))]),
                ]),
            ),
        ]);

        assert_eq!(
            value.to_json(),
            "{\"ok\":true,\"items\":[1,null,{\"name\":\"main\"}]}"
        );
    }
}
