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
    Diagnostic::new(
        diagnostic.id,
        Severity::Error,
        DiagnosticKind::Parse,
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
        if let Some(span) = &diagnostic.span {
            println!(
                "{}:{}:{}: {}[{}]: {}",
                span.file.as_str(),
                span.start.line,
                span.start.column,
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            );
        } else {
            println!(
                "{}[{}]: {}",
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            );
        }
    }
}

pub(crate) fn print_human_stderr(envelope: &DiagnosticEnvelope) -> Result<(), String> {
    let mut stderr = io::stderr();
    for diagnostic in &envelope.diagnostics {
        if let Some(span) = &diagnostic.span {
            writeln!(
                stderr,
                "{}:{}:{}: {}[{}]: {}",
                span.file.as_str(),
                span.start.line,
                span.start.column,
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            )
            .map_err(|error| error.to_string())?;
        } else {
            writeln!(
                stderr,
                "{}[{}]: {}",
                diagnostic.severity.as_str(),
                diagnostic.id,
                diagnostic.message
            )
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(crate) fn tool_info() -> ToolInfo {
    ToolInfo::new("veln", env!("CARGO_PKG_VERSION"))
}
