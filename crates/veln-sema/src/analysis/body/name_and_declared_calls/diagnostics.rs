use super::*;

pub(super) fn module_value_not_callable_diagnostic(
    value: &crate::types::ModuleValueSignature,
    callee: &Expr,
) -> Diagnostic {
    let name = value.declaration.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "type.not_callable",
        Severity::Error,
        DiagnosticKind::Type,
        format!("module value `{name}` is not callable"),
        Some(callee.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("type_check")),
            ("node_id", JsonValue::string(callee.node_id.display("name"))),
            ("constraint", JsonValue::string("call_target")),
            ("actual", JsonValue::string("module_value")),
            ("name", JsonValue::string(name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("module_value_declaration")),
        (
            "message",
            JsonValue::string(format!("Module value `{name}` is declared here.")),
        ),
        ("span", span_json(&value.declaration.span)),
    ]));
    diagnostic
}

pub(super) fn ambiguous_module_value_candidate(
    symbol: &str,
    candidate: &crate::types::ModuleValueAmbiguityCandidate,
) -> JsonValue {
    use crate::types::ModuleValueAmbiguityCandidateKind;

    let package = candidate.package_name.as_deref().unwrap_or("<current>");
    let package_origin = candidate.package_name.as_deref().map_or_else(
        || "the current package".to_string(),
        |package| format!("package `{package}`"),
    );
    let (candidate_kind, message) = match candidate.kind {
        ModuleValueAmbiguityCandidateKind::ModuleValue => (
            "module_value",
            format!(
                "Module `{}` from {package_origin} exports module value `{symbol}`; use `{}::{symbol}` to select it.",
                candidate.module_name, candidate.alias
            ),
        ),
        ModuleValueAmbiguityCandidateKind::Function => (
            "function",
            format!(
                "Module `{}` from {package_origin} exports function `{symbol}`; use `{}::{symbol}` to select it.",
                candidate.module_name, candidate.alias
            ),
        ),
        ModuleValueAmbiguityCandidateKind::ImportTarget => (
            "import_target",
            format!(
                "Import `{}` from package `{package}` can select `{symbol}`; give this import a distinct alias to select that package.",
                candidate.alias
            ),
        ),
    };
    JsonValue::object([
        ("kind", JsonValue::string("import_candidate")),
        ("candidate_kind", JsonValue::string(candidate_kind)),
        ("package", JsonValue::string(package)),
        ("module", JsonValue::string(&candidate.module_name)),
        ("message", JsonValue::string(message)),
        ("span", span_json(&candidate.span)),
    ])
}
