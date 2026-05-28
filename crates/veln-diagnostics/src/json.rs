#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

pub fn parse_json_value(source: &str) -> Result<JsonValue, String> {
    let mut parser = JsonParser::new(source);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_at_end() {
        Ok(value)
    } else {
        Err("unexpected trailing JSON input".to_string())
    }
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

struct JsonParser<'a> {
    source: &'a str,
    offset: usize,
}

impl<'a> JsonParser<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, offset: 0 }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        match self.peek_byte() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            Some(_) => Err("unexpected JSON token".to_string()),
            None => Err("expected JSON value".to_string()),
        }
    }

    fn parse_literal(&mut self, literal: &[u8], value: JsonValue) -> Result<JsonValue, String> {
        if self.source.as_bytes()[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(value)
        } else {
            Err("invalid JSON literal".to_string())
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        let mut value = String::new();
        while let Some(byte) = self.next_byte() {
            match byte {
                b'"' => return Ok(value),
                b'\\' => value.push(self.parse_escape()?),
                0x00..=0x1f => return Err("unescaped control character in JSON string".to_string()),
                _ => {
                    let start = self.offset - 1;
                    let ch = self.source[start..]
                        .chars()
                        .next()
                        .ok_or_else(|| "invalid JSON string character".to_string())?;
                    self.offset = start + ch.len_utf8();
                    value.push(ch);
                }
            }
        }
        Err("unterminated JSON string".to_string())
    }

    fn parse_escape(&mut self) -> Result<char, String> {
        match self.next_byte() {
            Some(b'"') => Ok('"'),
            Some(b'\\') => Ok('\\'),
            Some(b'/') => Ok('/'),
            Some(b'b') => Ok('\u{08}'),
            Some(b'f') => Ok('\u{0c}'),
            Some(b'n') => Ok('\n'),
            Some(b'r') => Ok('\r'),
            Some(b't') => Ok('\t'),
            Some(b'u') => self.parse_unicode_escape(),
            Some(_) => Err("invalid JSON string escape".to_string()),
            None => Err("unterminated JSON string escape".to_string()),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, String> {
        let mut value = 0;
        for _ in 0..4 {
            let digit = self
                .next_byte()
                .and_then(|byte| (byte as char).to_digit(16))
                .ok_or_else(|| "invalid JSON unicode escape".to_string())?;
            value = (value << 4) | digit;
        }
        char::from_u32(value).ok_or_else(|| "invalid JSON unicode scalar".to_string())
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'[')?;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            values.push(self.parse_value()?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.expect_byte(b'{')?;
        let mut entries = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect_byte(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                break;
            }
            self.expect_byte(b',')?;
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_number(&mut self) -> Result<i64, String> {
        let start = self.offset;
        self.consume_byte(b'-');
        match self.peek_byte() {
            Some(b'0') => {
                self.offset += 1;
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek_byte(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => return Err("invalid JSON number".to_string()),
        }
        if matches!(self.peek_byte(), Some(b'.' | b'e' | b'E')) {
            return Err("JSON numbers must be integers".to_string());
        }
        self.source[start..self.offset]
            .parse()
            .map_err(|_| "JSON number is outside the supported range".to_string())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.offset >= self.source.len()
    }

    fn peek_byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.offset).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.offset += 1;
        Some(byte)
    }

    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        if self.consume_byte(expected) {
            Ok(())
        } else {
            Err(format!("expected JSON `{}`", expected as char))
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

    #[test]
    fn parses_json_values() {
        let value = parse_json_value(
            "{\"name\":\"main\\nvalue\",\"ok\":true,\"items\":[1,null,{\"path\":\"a\\\\b\"}]}",
        )
        .expect("JSON should parse");

        assert_eq!(
            value,
            JsonValue::object([
                ("name", JsonValue::string("main\nvalue")),
                ("ok", JsonValue::Bool(true)),
                (
                    "items",
                    JsonValue::array([
                        JsonValue::Number(1),
                        JsonValue::Null,
                        JsonValue::object([("path", JsonValue::string("a\\b"))]),
                    ]),
                ),
            ])
        );
    }

    #[test]
    fn parse_json_value_rejects_trailing_input() {
        let error = parse_json_value("{} []").expect_err("trailing input should fail");

        assert_eq!(error, "unexpected trailing JSON input");
    }
}
