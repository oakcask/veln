use std::collections::BTreeMap;

use veln_source::SourceSpan;

use crate::{CheckStatus, DiagnosticKind, JsonValue, Severity};

pub const SCHEMA_VERSION: u32 = 1;

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

#[cfg(test)]
mod tests {
    use super::*;
    use veln_source::{SourceFile, TextRange};

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

    #[test]
    fn envelope_status_prefers_errors_over_holes() {
        let envelope = DiagnosticEnvelope::new(
            ToolInfo::new("veln", "0.1.0"),
            vec![
                Diagnostic::new(
                    "hole.unresolved",
                    Severity::Info,
                    DiagnosticKind::Hole,
                    "hole remains",
                    None,
                    JsonValue::Null,
                ),
                Diagnostic::new(
                    "type.mismatch",
                    Severity::Error,
                    DiagnosticKind::Type,
                    "type mismatch",
                    None,
                    JsonValue::Null,
                ),
            ],
        );

        assert_eq!(envelope.status, CheckStatus::Error);
        assert!(envelope.to_json().contains("\"status\":\"error\""));
    }

    #[test]
    fn envelope_status_is_partial_for_non_error_holes() {
        let envelope = DiagnosticEnvelope::new(
            ToolInfo::new("veln", "0.1.0"),
            vec![Diagnostic::new(
                "hole.unresolved",
                Severity::Hint,
                DiagnosticKind::Hole,
                "hole remains",
                None,
                JsonValue::Null,
            )],
        );

        assert_eq!(envelope.status, CheckStatus::Partial);
        assert!(envelope.to_json().contains("\"status\":\"partial\""));
    }

    #[test]
    fn envelope_summary_counts_by_severity_and_kind_in_stable_order() {
        let envelope = DiagnosticEnvelope::new(
            ToolInfo::new("veln", "0.1.0"),
            vec![
                Diagnostic::new(
                    "lint.unused",
                    Severity::Warning,
                    DiagnosticKind::Lint,
                    "unused binding",
                    None,
                    JsonValue::Null,
                ),
                Diagnostic::new(
                    "parse.expected",
                    Severity::Error,
                    DiagnosticKind::Parse,
                    "expected expression",
                    None,
                    JsonValue::Null,
                ),
                Diagnostic::new(
                    "lint.style",
                    Severity::Warning,
                    DiagnosticKind::Lint,
                    "style issue",
                    None,
                    JsonValue::Null,
                ),
                Diagnostic::new(
                    "doc.missing",
                    Severity::Info,
                    DiagnosticKind::Doc,
                    "missing docs",
                    None,
                    JsonValue::Null,
                ),
            ],
        );

        let json = envelope.to_json();
        assert!(json.contains("\"diagnostic_count\":4"));
        assert!(json.contains("\"by_severity\":{\"error\":1,\"info\":1,\"warning\":2}"));
        assert!(json.contains("\"by_kind\":{\"doc\":1,\"lint\":2,\"parse\":1}"));
    }

    #[test]
    fn diagnostic_json_includes_span_details_and_related_payloads() {
        let source = SourceFile::new("src/main.veln", "let x\nnext");
        let span = source.span(TextRange::new(4, 6));
        let mut diagnostic = Diagnostic::new(
            "name.duplicate",
            Severity::Error,
            DiagnosticKind::Name,
            "duplicate name",
            Some(span),
            JsonValue::object([
                ("symbol", JsonValue::string("x")),
                ("previous", JsonValue::Number(1)),
            ]),
        );
        diagnostic.related.push(JsonValue::object([(
            "message",
            JsonValue::string("first binding"),
        )]));

        assert_eq!(
            diagnostic_to_json(&diagnostic).to_json(),
            concat!(
                "{\"id\":\"name.duplicate\",\"severity\":\"error\",\"kind\":\"name\",",
                "\"message\":\"duplicate name\",",
                "\"span\":{\"file\":\"src/main.veln\",",
                "\"start\":{\"line\":1,\"column\":5,\"offset\":4},",
                "\"end\":{\"line\":2,\"column\":1,\"offset\":6}},",
                "\"details\":{\"symbol\":\"x\",\"previous\":1},",
                "\"related\":[{\"message\":\"first binding\"}]}"
            )
        );
    }
}
