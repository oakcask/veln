use std::io::{self, Write};

use veln_diagnostics::{
    Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity, ToolInfo,
};
use veln_syntax::ParseDiagnostic;

pub(crate) fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

pub(crate) fn parse_diagnostic_to_envelope(diagnostic: &ParseDiagnostic) -> Diagnostic {
    let kind = if diagnostic.parser_context == "contract_predicate" {
        DiagnosticKind::Contract
    } else {
        DiagnosticKind::Parse
    };
    Diagnostic::new(
        diagnostic.id,
        Severity::Error,
        kind,
        diagnostic.message.clone(),
        diagnostic.span.clone(),
        JsonValue::object([
            ("phase", JsonValue::string("parse")),
            ("node_id", JsonValue::Null),
            (
                "parser_context",
                JsonValue::string(diagnostic.parser_context),
            ),
            (
                "unexpected",
                JsonValue::object([
                    (
                        "kind",
                        JsonValue::string(diagnostic.unexpected.kind.clone()),
                    ),
                    (
                        "text",
                        JsonValue::string(diagnostic.unexpected.text.clone()),
                    ),
                ]),
            ),
            (
                "expected",
                JsonValue::array(
                    diagnostic
                        .expected
                        .iter()
                        .map(|expected| JsonValue::string(*expected)),
                ),
            ),
            (
                "recovery",
                JsonValue::object([
                    (
                        "strategy",
                        JsonValue::string(diagnostic.recovery.strategy.as_str()),
                    ),
                    (
                        "anchor",
                        diagnostic
                            .recovery
                            .anchor
                            .as_ref()
                            .map_or(JsonValue::Null, |anchor| JsonValue::string(anchor.clone())),
                    ),
                    (
                        "dropped_token_count",
                        JsonValue::Number(diagnostic.recovery.dropped_token_count as i64),
                    ),
                ]),
            ),
        ]),
    )
}

pub(crate) fn print_parse_diagnostic_human(diagnostic: &ParseDiagnostic) {
    if let Some(span) = &diagnostic.span {
        eprintln!(
            "{}:{}:{}: error[{}]: {}",
            span.file.as_str(),
            span.start.line,
            span.start.column,
            diagnostic.id,
            diagnostic.message
        );
    } else {
        eprintln!("error[{}]: {}", diagnostic.id, diagnostic.message);
    }
}

pub(crate) fn print_human(envelope: &DiagnosticEnvelope) {
    if envelope.diagnostics.is_empty() {
        println!("ok");
        return;
    }

    for diagnostic in &envelope.diagnostics {
        for line in diagnostic_human_lines(diagnostic) {
            println!("{line}");
        }
    }
}

pub(crate) fn print_human_stderr(envelope: &DiagnosticEnvelope) -> Result<(), String> {
    let mut stderr = io::stderr();
    for diagnostic in &envelope.diagnostics {
        for line in diagnostic_human_lines(diagnostic) {
            writeln!(stderr, "{line}").map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn tool_info() -> ToolInfo {
    ToolInfo::new("veln", env!("CARGO_PKG_VERSION"))
}

fn diagnostic_human_lines(diagnostic: &Diagnostic) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(span) = &diagnostic.span {
        lines.push(format!(
            "{}:{}:{}: {}[{}]: {}",
            span.file.as_str(),
            span.start.line,
            span.start.column,
            diagnostic.severity.as_str(),
            diagnostic.id,
            diagnostic.message
        ));
    } else {
        lines.push(format!(
            "{}[{}]: {}",
            diagnostic.severity.as_str(),
            diagnostic.id,
            diagnostic.message
        ));
    }

    lines.extend(diagnostic.related.iter().filter_map(related_human_line));
    lines
}

fn related_human_line(related: &JsonValue) -> Option<String> {
    let JsonValue::Object(entries) = related else {
        return None;
    };
    let message = object_string(entries, "message")?;
    let span = object_value(entries, "span").and_then(span_json_human_prefix);
    Some(match span {
        Some(span) => format!("  note: {span}: {message}"),
        None => format!("  note: {message}"),
    })
}

fn span_json_human_prefix(value: &JsonValue) -> Option<String> {
    let JsonValue::Object(entries) = value else {
        return None;
    };
    let file = object_string(entries, "file")?;
    let start = object_value(entries, "start")?;
    let JsonValue::Object(start_entries) = start else {
        return None;
    };
    let line = object_number(start_entries, "line")?;
    let column = object_number(start_entries, "column")?;
    Some(format!("{file}:{line}:{column}"))
}

fn object_value<'a>(entries: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    entries
        .iter()
        .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
}

fn object_string(entries: &[(String, JsonValue)], key: &str) -> Option<String> {
    match object_value(entries, key)? {
        JsonValue::String(value) => Some(value.clone()),
        _ => None,
    }
}

fn object_number(entries: &[(String, JsonValue)], key: &str) -> Option<i64> {
    match object_value(entries, key)? {
        JsonValue::Number(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use veln_diagnostics::{DiagnosticKind, Severity};
    use veln_source::{LineCol, SourcePath, SourceSpan};
    use veln_syntax::{ParseDiagnostic, Recovery, RecoveryStrategy, UnexpectedToken};

    use super::*;

    #[test]
    fn parse_diagnostic_envelope_preserves_recovery_details() {
        let diagnostic = ParseDiagnostic {
            id: "parse.expected_item",
            message: "expected a function or test declaration".to_string(),
            span: Some(span("main.veln", 2, 3)),
            parser_context: "module",
            unexpected: UnexpectedToken {
                kind: "At".to_string(),
                text: "@".to_string(),
            },
            expected: vec!["fn", "test"],
            recovery: Recovery {
                strategy: RecoveryStrategy::SynchronizeToAnchor,
                anchor: Some("fn".to_string()),
                dropped_token_count: 2,
            },
        };

        let converted = parse_diagnostic_to_envelope(&diagnostic);

        assert_eq!(converted.id, "parse.expected_item");
        assert_eq!(converted.severity, Severity::Error);
        assert_eq!(converted.kind, DiagnosticKind::Parse);
        assert_eq!(converted.message, "expected a function or test declaration");
        assert_eq!(converted.span, Some(span("main.veln", 2, 3)));
        assert_eq!(
            converted.details.to_json(),
            concat!(
                "{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"module\",",
                "\"unexpected\":{\"kind\":\"At\",\"text\":\"@\"},",
                "\"expected\":[\"fn\",\"test\"],",
                "\"recovery\":{\"strategy\":\"synchronize_to_anchor\",",
                "\"anchor\":\"fn\",\"dropped_token_count\":2}}"
            )
        );
        assert!(converted.related.is_empty());
    }

    #[test]
    fn parse_diagnostic_envelope_uses_contract_kind_for_contract_predicates() {
        let diagnostic = ParseDiagnostic {
            id: "parse.contract_predicate",
            message: "unsupported contract predicate syntax".to_string(),
            span: None,
            parser_context: "contract_predicate",
            unexpected: UnexpectedToken {
                kind: "LBracket".to_string(),
                text: "[".to_string(),
            },
            expected: vec!["contract predicate"],
            recovery: Recovery {
                strategy: RecoveryStrategy::None,
                anchor: None,
                dropped_token_count: 0,
            },
        };

        let converted = parse_diagnostic_to_envelope(&diagnostic);

        assert_eq!(converted.kind, DiagnosticKind::Contract);
        assert_eq!(converted.severity, Severity::Error);
        assert_eq!(converted.span, None);
        assert!(
            converted
                .details
                .to_json()
                .contains("\"parser_context\":\"contract_predicate\"")
        );
        assert!(converted.details.to_json().contains(concat!(
            "\"recovery\":{\"strategy\":\"none\",",
            "\"anchor\":null,\"dropped_token_count\":0}"
        )));
    }

    #[test]
    fn human_diagnostic_output_keeps_cause_in_related_note() {
        let mut diagnostic = Diagnostic::new(
            "effect.missing_public",
            Severity::Error,
            DiagnosticKind::Effect,
            "public function uses undeclared effect `stdio`",
            Some(span("main.veln", 1, 1)),
            JsonValue::object(Vec::<(String, JsonValue)>::new()),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("effect_provenance")),
            (
                "message",
                JsonValue::string("Call to `stdio::println` requires this effect."),
            ),
            ("span", span_json("main.veln", 2, 3)),
        ]));

        assert_eq!(
            diagnostic_human_lines(&diagnostic),
            vec![
                "main.veln:1:1: error[effect.missing_public]: public function uses undeclared effect `stdio`",
                "  note: main.veln:2:3: Call to `stdio::println` requires this effect.",
            ]
        );
    }

    #[test]
    fn human_diagnostic_output_handles_missing_spans_and_malformed_related_notes() {
        let mut diagnostic = Diagnostic::new(
            "project.discovery",
            Severity::Error,
            DiagnosticKind::Doc,
            "no input files were discovered",
            None,
            JsonValue::object(Vec::<(String, JsonValue)>::new()),
        );
        diagnostic
            .related
            .push(JsonValue::String("not an object".to_string()));
        diagnostic.related.push(JsonValue::object([(
            "message",
            JsonValue::string("Add a source file or pass an explicit target."),
        )]));
        diagnostic.related.push(JsonValue::object([
            ("message", JsonValue::string("Malformed span is ignored.")),
            ("span", JsonValue::String("not a span".to_string())),
        ]));
        diagnostic.related.push(JsonValue::object([
            ("message", JsonValue::string("Malformed start is ignored.")),
            (
                "span",
                JsonValue::object([
                    ("file", JsonValue::string("main.veln")),
                    ("start", JsonValue::String("not a point".to_string())),
                ]),
            ),
        ]));
        diagnostic.related.push(JsonValue::object([
            ("message", JsonValue::string("Malformed line is ignored.")),
            (
                "span",
                JsonValue::object([
                    ("file", JsonValue::string("main.veln")),
                    (
                        "start",
                        JsonValue::object([
                            ("line", JsonValue::string("one")),
                            ("column", JsonValue::Number(1)),
                        ]),
                    ),
                ]),
            ),
        ]));

        assert_eq!(
            diagnostic_human_lines(&diagnostic),
            vec![
                "error[project.discovery]: no input files were discovered",
                "  note: Add a source file or pass an explicit target.",
                "  note: Malformed span is ignored.",
                "  note: Malformed start is ignored.",
                "  note: Malformed line is ignored.",
            ]
        );
    }

    fn span(file: &str, line: usize, column: usize) -> SourceSpan {
        SourceSpan {
            file: SourcePath::new(file),
            start: LineCol {
                line,
                column,
                offset: 0,
            },
            end: LineCol {
                line,
                column,
                offset: 0,
            },
        }
    }

    fn span_json(file: &str, line: i64, column: i64) -> JsonValue {
        JsonValue::object([
            ("file", JsonValue::string(file)),
            (
                "start",
                JsonValue::object([
                    ("line", JsonValue::Number(line)),
                    ("column", JsonValue::Number(column)),
                    ("offset", JsonValue::Number(0)),
                ]),
            ),
            (
                "end",
                JsonValue::object([
                    ("line", JsonValue::Number(line)),
                    ("column", JsonValue::Number(column)),
                    ("offset", JsonValue::Number(0)),
                ]),
            ),
        ])
    }
}
