use super::is_json_integer_token;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    Decimal(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub(super) fn to_compact_string(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Decimal(value) => value.clone(),
            Self::String(value) => format!("\"{}\"", escape_json_string(value)),
            Self::Array(values) => {
                let values = values
                    .iter()
                    .map(JsonValue::to_compact_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{values}]")
            }
            Self::Object(entries) => {
                let entries = entries
                    .iter()
                    .map(|(key, value)| {
                        format!(
                            "\"{}\":{}",
                            escape_json_string(key),
                            value.to_compact_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{entries}}}")
            }
        }
    }

    pub(super) fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    pub(super) fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub(super) fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Decimal(value) if is_json_integer_token(value) => value.parse().ok(),
            _ => None,
        }
    }

    pub(super) fn object_field(&self, name: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

pub(super) fn escape_json_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            ch if ch.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[derive(Debug)]
pub(super) struct JsonParseError {
    message: String,
    pub(super) offset: usize,
    pub(super) missing_closing_delimiter: bool,
}

impl JsonParseError {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset,
            missing_closing_delimiter: false,
        }
    }

    fn missing_closing_delimiter(offset: usize, delimiter: u8) -> Self {
        Self {
            message: format!("expected `{}` at byte {offset}", delimiter as char),
            offset,
            missing_closing_delimiter: true,
        }
    }
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

pub(super) fn parse_json(text: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = JsonParser { text, offset: 0 };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.offset == text.len() {
        Ok(value)
    } else {
        Err(JsonParseError::new(
            parser.offset,
            format!("unexpected trailing input at byte {}", parser.offset),
        ))
    }
}

struct JsonParser<'a> {
    text: &'a str,
    offset: usize,
}

impl JsonParser<'_> {
    fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(byte) => Err(JsonParseError::new(
                self.offset,
                format!("unexpected byte `{}` at byte {}", byte as char, self.offset),
            )),
            None => Err(JsonParseError::new(
                self.offset,
                format!("unexpected end of input at byte {}", self.offset),
            )),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume(b'[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.consume_if(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_ws();
            if self.consume_if(b']') {
                break;
            }
            if self.peek().is_none() {
                return Err(JsonParseError::missing_closing_delimiter(self.offset, b']'));
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.consume_if(b'}') {
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.consume(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.consume_if(b'}') {
                break;
            }
            if self.peek().is_none() {
                return Err(JsonParseError::missing_closing_delimiter(self.offset, b'}'));
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.consume(b'"')?;
        let mut parsed = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(parsed),
                '\\' => parsed.push(self.parse_escape()?),
                ch if ch.is_control() => {
                    return Err(JsonParseError::new(
                        self.offset,
                        format!("control character in string at byte {}", self.offset),
                    ));
                }
                ch => parsed.push(ch),
            }
        }
        Err(JsonParseError::new(
            self.offset,
            format!("unterminated string at byte {}", self.offset),
        ))
    }

    fn parse_escape(&mut self) -> Result<char, JsonParseError> {
        let Some(ch) = self.next_char() else {
            return Err(JsonParseError::new(
                self.offset,
                format!("unterminated escape at byte {}", self.offset),
            ));
        };
        match ch {
            '"' | '\\' | '/' => Ok(ch),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            _ => Err(JsonParseError::new(
                self.offset,
                format!("unsupported escape `{ch}` at byte {}", self.offset),
            )),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonParseError> {
        let start = self.offset;
        let end = start + 4;
        let Some(hex) = self.text.get(start..end) else {
            return Err(JsonParseError::new(
                start,
                format!("short unicode escape at byte {start}"),
            ));
        };
        if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(JsonParseError::new(
                start,
                format!("invalid unicode escape `{hex}` at byte {start}"),
            ));
        }
        self.offset = end;
        let codepoint = u16::from_str_radix(hex, 16).expect("hex was validated");
        if (0xd800..=0xdbff).contains(&codepoint) {
            if !self.text[self.offset..].starts_with("\\u") {
                return Err(JsonParseError::new(
                    start,
                    format!("unpaired high surrogate `{hex}` at byte {start}"),
                ));
            }
            self.offset += 2;
            let (low, low_hex, low_start) = self.parse_unicode_unit()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(JsonParseError::new(
                    low_start,
                    format!("invalid low surrogate `{low_hex}` at byte {low_start}"),
                ));
            }
            let high_value = u32::from(codepoint - 0xd800);
            let low_value = u32::from(low - 0xdc00);
            let scalar = 0x10000 + ((high_value << 10) | low_value);
            return char::from_u32(scalar).ok_or_else(|| {
                JsonParseError::new(start, format!("invalid surrogate pair at byte {start}"))
            });
        }
        if (0xdc00..=0xdfff).contains(&codepoint) {
            return Err(JsonParseError::new(
                start,
                format!("unpaired low surrogate `{hex}` at byte {start}"),
            ));
        }
        char::from_u32(u32::from(codepoint)).ok_or_else(|| {
            JsonParseError::new(
                start,
                format!("invalid unicode codepoint `{hex}` at byte {start}"),
            )
        })
    }

    fn parse_unicode_unit(&mut self) -> Result<(u16, String, usize), JsonParseError> {
        let start = self.offset;
        let end = start + 4;
        let Some(hex) = self.text.get(start..end) else {
            return Err(JsonParseError::new(
                start,
                format!("short unicode escape at byte {start}"),
            ));
        };
        if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(JsonParseError::new(
                start,
                format!("invalid unicode escape `{hex}` at byte {start}"),
            ));
        }
        self.offset = end;
        let codepoint = u16::from_str_radix(hex, 16).expect("hex was validated");
        Ok((codepoint, hex.to_string(), start))
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonParseError> {
        let start = self.offset;
        self.consume_if(b'-');
        match self.peek() {
            Some(b'0') => {
                self.offset += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(JsonParseError::new(
                        self.offset,
                        format!("leading zero in number at byte {}", self.offset),
                    ));
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected digit at byte {}", self.offset),
                ));
            }
        }
        if self.consume_if(b'.') {
            let fraction_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == fraction_start {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected fraction digit at byte {}", self.offset),
                ));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == exponent_start {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected exponent digit at byte {}", self.offset),
                ));
            }
        }
        let raw = &self.text[start..self.offset];
        Ok(JsonValue::Decimal(raw.to_string()))
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), JsonParseError> {
        if self.text[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(JsonParseError::new(
                self.offset,
                format!("expected `{literal}` at byte {}", self.offset),
            ))
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), JsonParseError> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(JsonParseError::new(
                self.offset,
                format!("expected `{}` at byte {}", expected as char, self.offset),
            ))
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.offset).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.text[self.offset..].chars().next()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}
