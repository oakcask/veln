use veln_diagnostics::{
    Diagnostic, DiagnosticKind, JsonValue, Severity, source_span_to_json as span_json,
};
use veln_syntax::{ParseDiagnostic, ParseRepairCandidate, ParseRepairEdit};

pub fn parse_diagnostic_to_envelope(diagnostic: &ParseDiagnostic) -> Diagnostic {
    let kind = if diagnostic.parser_context == "contract_predicate" {
        DiagnosticKind::Contract
    } else {
        DiagnosticKind::Parse
    };
    let mut details = vec![
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
    ];
    if !diagnostic.repair_candidates.is_empty() {
        details.push((
            "candidate_queries",
            JsonValue::array([JsonValue::object([
                ("query_id", JsonValue::string(diagnostic.id)),
                (
                    "candidates",
                    JsonValue::array(
                        diagnostic
                            .repair_candidates
                            .iter()
                            .map(parse_repair_candidate_json),
                    ),
                ),
            ])]),
        ));
    }
    let mut envelope = Diagnostic::new(
        diagnostic.id,
        Severity::Error,
        kind,
        diagnostic.message.clone(),
        diagnostic.span.clone(),
        JsonValue::object(details),
    );
    if diagnostic.parser_context == "integer_literal"
        && let Some(expected) = diagnostic.expected.first()
    {
        envelope.related.push(JsonValue::object([(
            "message",
            JsonValue::string(format!("Accepted integer form: {expected}.")),
        )]));
    }
    envelope
}

fn parse_repair_candidate_json(candidate: &ParseRepairCandidate) -> JsonValue {
    JsonValue::object([
        (
            "candidate_id",
            JsonValue::string(candidate.candidate_id.clone()),
        ),
        ("name", JsonValue::string(candidate.name.clone())),
        (
            "application_policy",
            JsonValue::string(candidate.application_policy.clone()),
        ),
        (
            "application_status",
            JsonValue::string(candidate.application_status.clone()),
        ),
        (
            "edit_summary",
            JsonValue::string(candidate.edit_summary.clone()),
        ),
        (
            "edits",
            JsonValue::array(candidate.edits.iter().map(parse_repair_edit_json)),
        ),
    ])
}

fn parse_repair_edit_json(edit: &ParseRepairEdit) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("replace")),
        ("span", span_json(&edit.span)),
        ("replacement", JsonValue::string(edit.replacement.clone())),
    ])
}
