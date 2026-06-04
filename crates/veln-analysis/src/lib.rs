//! Shared project analysis for Veln tools.

mod analysis;
mod surface;

use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_syntax::{ParseDiagnostic, ParseRepairCandidate, ParseRepairEdit};

pub use analysis::{
    DoctestMode, ProjectAnalysis, ReachableEntryAnalysis, analyze_project,
    checked_project_diagnostics,
};
pub use surface::{
    derive_source_module_path, load_surface_module, validate_manifest_dependencies,
    validate_manifest_exports,
};

fn parse_diagnostic_to_envelope(diagnostic: &ParseDiagnostic) -> Diagnostic {
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
    Diagnostic::new(
        diagnostic.id,
        Severity::Error,
        kind,
        diagnostic.message.clone(),
        diagnostic.span.clone(),
        JsonValue::object(details),
    )
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

fn span_json(span: &veln_source::SourceSpan) -> JsonValue {
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
