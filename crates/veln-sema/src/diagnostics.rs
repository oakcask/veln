use veln_ast::ContractKind;
use veln_diagnostics::JsonValue;
use veln_source::SourceSpan;

use crate::contracts::contract_kind_text;
use crate::types::EffectUse;

pub(crate) fn span_json(span: &SourceSpan) -> JsonValue {
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

pub(crate) fn type_details(
    node_id: String,
    expected_type: impl Into<String>,
    actual_type: impl Into<String>,
    expected_type_source: &'static str,
    actual_type_source: &'static str,
    constraint: &'static str,
    origin_node_ids: impl IntoIterator<Item = String>,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("type")),
        ("node_id", JsonValue::string(node_id)),
        ("expected_type", JsonValue::string(expected_type)),
        ("actual_type", JsonValue::string(actual_type)),
        (
            "expected_type_source",
            JsonValue::string(expected_type_source),
        ),
        ("actual_type_source", JsonValue::string(actual_type_source)),
        ("constraint", JsonValue::string(constraint)),
        (
            "origin_node_ids",
            JsonValue::array(origin_node_ids.into_iter().map(JsonValue::string)),
        ),
    ])
}

pub(crate) fn effect_details(node_id: String, boundary: &'static str) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("effect")),
        ("node_id", JsonValue::string(node_id)),
        ("effect", JsonValue::string("unknown")),
        ("boundary", JsonValue::string(boundary)),
        ("declared_effects", JsonValue::array([])),
        ("inferred_effects", JsonValue::array([])),
        ("provenance", JsonValue::array([])),
        ("provenance_truncated", JsonValue::Bool(false)),
    ])
}

pub(crate) fn module_details(
    node_id: String,
    field: &'static str,
    expected_owner: &'static str,
    observed_owner: &'static str,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("module")),
        ("node_id", JsonValue::string(node_id)),
        ("field", JsonValue::string(field)),
        ("expected_owner", JsonValue::string(expected_owner)),
        ("observed_owner", JsonValue::string(observed_owner)),
    ])
}

pub(crate) fn effect_missing_public_details(
    node_id: String,
    effect: &str,
    boundary: &'static str,
    declared_effects: &[String],
    inferred_effects: &[String],
    provenance: &[EffectUse],
    provenance_truncated: bool,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("effect")),
        ("node_id", JsonValue::string(node_id)),
        ("effect", JsonValue::string(effect)),
        ("boundary", JsonValue::string(boundary)),
        (
            "declared_effects",
            JsonValue::array(declared_effects.iter().cloned().map(JsonValue::string)),
        ),
        (
            "inferred_effects",
            JsonValue::array(inferred_effects.iter().cloned().map(JsonValue::string)),
        ),
        (
            "provenance",
            JsonValue::array(provenance.iter().map(|effect_use| {
                JsonValue::object([
                    (
                        "node_id",
                        JsonValue::string(effect_use.node_id.display("call")),
                    ),
                    ("kind", JsonValue::string(effect_use.kind)),
                    ("symbol", JsonValue::string(effect_use.symbol.clone())),
                ])
            })),
        ),
        (
            "provenance_truncated",
            JsonValue::Bool(provenance_truncated),
        ),
    ])
}

pub(crate) fn contract_details(
    node_id: String,
    kind: ContractKind,
    predicate_text: &str,
    validation_status: &'static str,
    obligation_status: &'static str,
    reason: &'static str,
    runtime_required: bool,
    referenced_bindings: Vec<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("contract")),
        ("node_id", JsonValue::string(node_id)),
        ("clause", JsonValue::string(contract_kind_text(kind))),
        ("predicate_text", JsonValue::string(predicate_text)),
        ("validation_status", JsonValue::string(validation_status)),
        ("obligation_status", JsonValue::string(obligation_status)),
        ("reason", JsonValue::string(reason)),
        (
            "blame",
            JsonValue::string(match kind {
                ContractKind::Require => "caller",
                ContractKind::Ensure => "implementation",
            }),
        ),
        ("runtime_required", JsonValue::Bool(runtime_required)),
        ("referenced_bindings", JsonValue::array(referenced_bindings)),
    ])
}
