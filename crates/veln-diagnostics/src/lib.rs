//! Stable diagnostic envelopes and detail payloads.

use std::collections::BTreeMap;

use veln_source::SourceSpan;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticKind {
    Parse,
    Name,
    Type,
    Contract,
    Effect,
    Lint,
    Hole,
    Doc,
}

impl DiagnosticKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Parse => "parse",
            Self::Name => "name",
            Self::Type => "type",
            Self::Contract => "contract",
            Self::Effect => "effect",
            Self::Lint => "lint",
            Self::Hole => "hole",
            Self::Doc => "doc",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Error,
    Partial,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Partial => "partial",
        }
    }
}

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

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub id: String,
    pub severity: Severity,
    pub kind: DiagnosticKind,
    pub message: String,
    pub span: Option<SourceSpan>,
    pub details: JsonValue,
    pub related: Vec<JsonValue>,
}

impl Diagnostic {
    pub fn new(
        id: impl Into<String>,
        severity: Severity,
        kind: DiagnosticKind,
        message: impl Into<String>,
        span: Option<SourceSpan>,
        details: JsonValue,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            kind,
            message: message.into(),
            span,
            details,
            related: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolInfo {
    pub name: String,
    pub version: String,
}

impl ToolInfo {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DiagnosticEnvelope {
    pub tool: ToolInfo,
    pub status: CheckStatus,
    pub diagnostics: Vec<Diagnostic>,
}

impl DiagnosticEnvelope {
    pub fn new(tool: ToolInfo, diagnostics: Vec<Diagnostic>) -> Self {
        let status = if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
        {
            CheckStatus::Error
        } else if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.kind == DiagnosticKind::Hole)
        {
            CheckStatus::Partial
        } else {
            CheckStatus::Ok
        };
        Self {
            tool,
            status,
            diagnostics,
        }
    }

    pub fn to_json(&self) -> String {
        JsonValue::object([
            ("schema_version", JsonValue::Number(SCHEMA_VERSION.into())),
            (
                "tool",
                JsonValue::object([
                    ("name", JsonValue::string(self.tool.name.clone())),
                    ("version", JsonValue::string(self.tool.version.clone())),
                ]),
            ),
            ("status", JsonValue::string(self.status.as_str())),
            (
                "diagnostics",
                JsonValue::array(self.diagnostics.iter().map(diagnostic_to_json)),
            ),
            ("summary", summary_to_json(&self.diagnostics)),
        ])
        .to_json()
    }
}

pub fn diagnostic_to_json(diagnostic: &Diagnostic) -> JsonValue {
    JsonValue::object([
        ("id", JsonValue::string(diagnostic.id.clone())),
        ("severity", JsonValue::string(diagnostic.severity.as_str())),
        ("kind", JsonValue::string(diagnostic.kind.as_str())),
        ("message", JsonValue::string(diagnostic.message.clone())),
        ("span", span_to_json(diagnostic.span.as_ref())),
        ("details", diagnostic.details.clone()),
        ("related", JsonValue::Array(diagnostic.related.clone())),
    ])
}

fn summary_to_json(diagnostics: &[Diagnostic]) -> JsonValue {
    let mut by_severity = BTreeMap::new();
    let mut by_kind = BTreeMap::new();

    for diagnostic in diagnostics {
        *by_severity
            .entry(diagnostic.severity.as_str().to_string())
            .or_insert(0) += 1;
        *by_kind
            .entry(diagnostic.kind.as_str().to_string())
            .or_insert(0) += 1;
    }

    JsonValue::object([
        (
            "diagnostic_count",
            JsonValue::Number(diagnostics.len() as i64),
        ),
        ("by_severity", map_to_json(by_severity)),
        ("by_kind", map_to_json(by_kind)),
    ])
}

fn map_to_json(map: BTreeMap<String, i64>) -> JsonValue {
    JsonValue::Object(
        map.into_iter()
            .map(|(key, value)| (key, JsonValue::Number(value)))
            .collect(),
    )
}

fn span_to_json(span: Option<&SourceSpan>) -> JsonValue {
    let Some(span) = span else {
        return JsonValue::Null;
    };
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        (
            "start",
            JsonValue::object([
                ("line", JsonValue::Number(span.start.line as i64)),
                ("column", JsonValue::Number(span.start.column as i64)),
                ("offset", JsonValue::Number(span.start.offset as i64)),
            ]),
        ),
        (
            "end",
            JsonValue::object([
                ("line", JsonValue::Number(span.end.line as i64)),
                ("column", JsonValue::Number(span.end.column as i64)),
                ("offset", JsonValue::Number(span.end.offset as i64)),
            ]),
        ),
    ])
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
    fn renders_stable_envelope_fields() {
        let envelope = DiagnosticEnvelope::new(
            ToolInfo::new("veln", "0.1.0"),
            vec![Diagnostic::new(
                "parse.expected",
                Severity::Error,
                DiagnosticKind::Parse,
                "expected function name",
                None,
                JsonValue::object([("phase", JsonValue::string("parse"))]),
            )],
        );

        let json = envelope.to_json();
        assert!(json.contains("\"schema_version\":1"));
        assert!(json.contains("\"status\":\"error\""));
        assert!(json.contains("\"by_kind\":{\"parse\":1}"));
    }
}
