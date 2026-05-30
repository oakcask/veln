use veln_ast::{Contract, Expr, SatisfyClause};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::contracts::{ContractValidation, contract_kind_text, referenced_names};
use crate::diagnostics::span_json;
use crate::prelude::prelude_signature;
use crate::types::{Binding, ExpectedType, ExpectedTypeSource, Type};

use super::FunctionChecker;

fn satisfy_shadow_related_note(origin_span: Option<&SourceSpan>) -> JsonValue {
    if let Some(origin_span) = origin_span {
        return JsonValue::object([
            ("kind", JsonValue::string("shadow_origin")),
            (
                "message",
                JsonValue::string("Visible binding with this name is here."),
            ),
            ("span", span_json(origin_span)),
        ]);
    }
    JsonValue::object([
        ("kind", JsonValue::string("shadow_origin")),
        (
            "message",
            JsonValue::string("Prelude helper names cannot be reused as satisfy candidates."),
        ),
    ])
}

fn hole_message(expected_type: &str) -> String {
    if expected_type == "unknown" {
        "hole requires a value of unknown type".to_string()
    } else {
        format!("hole requires a `{expected_type}` value")
    }
}

fn hole_details(
    expr: &Expr,
    name: Option<&str>,
    expected_type: &str,
    expected_source: ExpectedTypeSource,
    constraints: Vec<JsonValue>,
    bindings: &[Binding],
    candidate_queries: Vec<JsonValue>,
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("hole")),
        ("node_id", JsonValue::string(expr.node_id.display("hole"))),
        (
            "label",
            name.map_or(JsonValue::Null, |name| {
                JsonValue::string(format!("_{name}"))
            }),
        ),
        ("expected_type", JsonValue::string(expected_type)),
        (
            "expected_type_source",
            JsonValue::string(expected_source.as_hole_source()),
        ),
        ("constraints", JsonValue::array(constraints)),
        ("local_bindings", JsonValue::array(local_bindings(bindings))),
        ("candidate_queries", JsonValue::array(candidate_queries)),
    ])
}

fn local_bindings(bindings: &[Binding]) -> Vec<JsonValue> {
    bindings
        .iter()
        .map(|binding| {
            JsonValue::object([
                ("name", JsonValue::string(binding.name.clone())),
                ("type", JsonValue::string(binding.ty.render())),
            ])
        })
        .collect()
}

fn satisfy_details(
    expr: &Expr,
    candidate: &str,
    predicate: &str,
    extra: Vec<(&'static str, JsonValue)>,
) -> JsonValue {
    let mut details = vec![
        ("phase", JsonValue::string("hole")),
        ("node_id", JsonValue::string(expr.node_id.display("hole"))),
        ("candidate_binding", JsonValue::string(candidate)),
        ("predicate_text", JsonValue::string(predicate.to_string())),
    ];
    details.extend(extra);
    JsonValue::object(details)
}

fn non_boolean_satisfy_diagnostic(
    expr: &Expr,
    satisfy: &SatisfyClause,
    candidate: &str,
    actual_type: String,
) -> Diagnostic {
    Diagnostic::new(
        "hole.satisfy_type_mismatch",
        Severity::Error,
        DiagnosticKind::Hole,
        "satisfy predicate is not `Bool`",
        Some(satisfy.span.clone()),
        satisfy_details(
            expr,
            candidate,
            &satisfy.predicate,
            vec![
                ("expected_type", JsonValue::string("Bool")),
                ("actual_type", JsonValue::string(actual_type)),
            ],
        ),
    )
}

fn unsupported_satisfy_diagnostic(
    expr: &Expr,
    satisfy: &SatisfyClause,
    candidate: &str,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "hole.satisfy_unsupported_construct",
        Severity::Error,
        DiagnosticKind::Hole,
        "satisfy predicate contains an unsupported construct",
        Some(satisfy.span.clone()),
        satisfy_details(
            expr,
            candidate,
            &satisfy.predicate,
            vec![("reason", JsonValue::string(reason))],
        ),
    )
}

fn missing_field_satisfy_diagnostic(
    expr: &Expr,
    satisfy: &SatisfyClause,
    candidate: &str,
    base_type: String,
    field: String,
) -> Diagnostic {
    Diagnostic::new(
        "hole.satisfy_field_missing",
        Severity::Error,
        DiagnosticKind::Hole,
        format!("satisfy field `{field}` is not present on `{base_type}`"),
        Some(satisfy.span.clone()),
        satisfy_details(
            expr,
            candidate,
            &satisfy.predicate,
            vec![
                ("base_type", JsonValue::string(base_type)),
                ("field", JsonValue::string(field)),
            ],
        ),
    )
}

fn hole_related_notes(
    contracts: &[Contract],
    satisfy: Option<&SatisfyClause>,
    expected: Option<&ExpectedType>,
) -> Vec<JsonValue> {
    let mut notes = Vec::new();
    notes.extend(expected_type_related_note(expected));
    notes.extend(contract_related_notes(contracts));
    notes.extend(satisfy_related_note(satisfy));
    notes
}

fn expected_type_related_note(expected: Option<&ExpectedType>) -> Option<JsonValue> {
    let expected = expected?;
    let span = expected.origin_span.as_ref()?;
    Some(JsonValue::object([
        ("kind", JsonValue::string("expected_type_origin")),
        ("message", JsonValue::string(expected.origin_message)),
        ("span", span_json(span)),
    ]))
}

fn contract_related_notes(contracts: &[Contract]) -> Vec<JsonValue> {
    contracts
        .iter()
        .map(|contract| {
            JsonValue::object([
                ("kind", JsonValue::string("constraint_origin")),
                (
                    "message",
                    JsonValue::string(format!(
                        "{} contract contributes a repair constraint.",
                        contract_kind_text(contract.kind)
                    )),
                ),
                ("span", span_json(&contract.span)),
            ])
        })
        .collect()
}

fn satisfy_related_note(satisfy: Option<&SatisfyClause>) -> Option<JsonValue> {
    let satisfy = satisfy?;
    Some(JsonValue::object([
        ("kind", JsonValue::string("constraint_origin")),
        (
            "message",
            JsonValue::string("Satisfy predicate contributes a repair constraint."),
        ),
        ("span", span_json(&satisfy.span)),
    ]))
}

impl<'a> FunctionChecker<'a> {
    pub(super) fn check_satisfy_clause(
        &mut self,
        expr: &Expr,
        satisfy: &SatisfyClause,
        expected: Option<&ExpectedType>,
    ) {
        let Some(candidate) = satisfy.candidate.as_deref() else {
            return;
        };
        let candidate_span = satisfy
            .candidate_span
            .clone()
            .unwrap_or_else(|| satisfy.span.clone());

        self.check_satisfy_candidate_shadow(expr, candidate, &candidate_span);
        self.check_satisfy_candidate_used(expr, satisfy, candidate, candidate_span);

        let mut predicate_bindings = self.bindings.clone();
        predicate_bindings.push(Binding {
            name: candidate.to_string(),
            ty: expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown),
        });
        let validation =
            self.validate_predicate_with_bindings(&satisfy.predicate, &predicate_bindings);
        self.push_satisfy_validation_diagnostic(expr, satisfy, candidate, validation);
    }

    fn check_satisfy_candidate_shadow(
        &mut self,
        expr: &Expr,
        candidate: &str,
        candidate_span: &SourceSpan,
    ) {
        let Some((origin_kind, origin_span)) = self.satisfy_shadow_origin(candidate) else {
            return;
        };
        let mut diagnostic = Diagnostic::new(
            "hole.satisfy_candidate_shadow",
            Severity::Error,
            DiagnosticKind::Hole,
            format!("satisfy candidate `{candidate}` shadows a visible binding"),
            Some(candidate_span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("hole")),
                ("node_id", JsonValue::string(expr.node_id.display("hole"))),
                ("candidate_binding", JsonValue::string(candidate)),
                ("shadowed_binding_kind", JsonValue::string(origin_kind)),
            ]),
        );
        diagnostic
            .related
            .push(satisfy_shadow_related_note(origin_span.as_ref()));
        self.diagnostics.push(diagnostic);
    }

    fn check_satisfy_candidate_used(
        &mut self,
        expr: &Expr,
        satisfy: &SatisfyClause,
        candidate: &str,
        candidate_span: SourceSpan,
    ) {
        if referenced_names(&satisfy.predicate)
            .iter()
            .any(|name| name == candidate)
        {
            return;
        }
        let mut diagnostic = Diagnostic::new(
            "hole.satisfy_candidate_unused",
            Severity::Error,
            DiagnosticKind::Hole,
            format!("satisfy predicate does not reference candidate `{candidate}`"),
            Some(candidate_span),
            JsonValue::object([
                ("phase", JsonValue::string("hole")),
                ("node_id", JsonValue::string(expr.node_id.display("hole"))),
                ("candidate_binding", JsonValue::string(candidate)),
                (
                    "predicate_text",
                    JsonValue::string(satisfy.predicate.clone()),
                ),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("constraint_origin")),
            (
                "message",
                JsonValue::string("The predicate for this satisfy clause is here."),
            ),
            ("span", span_json(&satisfy.span)),
        ]));
        self.diagnostics.push(diagnostic);
    }

    fn push_satisfy_validation_diagnostic(
        &mut self,
        expr: &Expr,
        satisfy: &SatisfyClause,
        candidate: &str,
        validation: ContractValidation,
    ) {
        match validation {
            ContractValidation::Valid => {}
            ContractValidation::NonBoolean { actual_type } => {
                self.diagnostics.push(non_boolean_satisfy_diagnostic(
                    expr,
                    satisfy,
                    candidate,
                    actual_type,
                ));
            }
            ContractValidation::UnsupportedConstruct { reason } => {
                self.diagnostics.push(unsupported_satisfy_diagnostic(
                    expr, satisfy, candidate, reason,
                ));
            }
            ContractValidation::MissingField { base_type, field } => {
                self.diagnostics.push(missing_field_satisfy_diagnostic(
                    expr, satisfy, candidate, base_type, field,
                ));
            }
            ContractValidation::UnresolvedName { name } => {
                self.push_unresolved_name(
                    expr.node_id,
                    satisfy.span.clone(),
                    &name,
                    "satisfy_predicate",
                );
            }
        }
    }

    fn satisfy_shadow_origin(&self, candidate: &str) -> Option<(&'static str, Option<SourceSpan>)> {
        if let Some((_, span)) = self.local_names.get(candidate) {
            return Some(("local", Some(span.clone())));
        }
        if let Some(result_binding) = &self.function.return_binding
            && result_binding.name == candidate
        {
            return Some(("result", Some(result_binding.span.clone())));
        }
        if prelude_signature(candidate, None).is_some() {
            return Some(("prelude", None));
        }
        None
    }

    pub(super) fn push_hole_diagnostic(
        &mut self,
        expr: &Expr,
        name: Option<&str>,
        satisfy: Option<&SatisfyClause>,
        expected: Option<&ExpectedType>,
    ) {
        let expected_type = expected
            .map(|expected| expected.ty.render())
            .unwrap_or_else(|| "unknown".to_string());
        let expected_source =
            expected.map_or(ExpectedTypeSource::Unknown, |expected| expected.source);
        let candidate_queries =
            self.candidate_queries(expected.map(|expected| &expected.ty), expr, satisfy);
        let constraints = self.hole_constraints(satisfy, expected.map(|expected| &expected.ty));
        let details = hole_details(
            expr,
            name,
            &expected_type,
            expected_source,
            constraints,
            self.bindings.as_slice(),
            candidate_queries,
        );
        let mut diagnostic = Diagnostic::new(
            "hole.unfilled",
            Severity::Hint,
            DiagnosticKind::Hole,
            hole_message(&expected_type),
            Some(expr.span.clone()),
            details,
        );
        diagnostic.related.extend(hole_related_notes(
            &self.function.contracts,
            satisfy,
            expected,
        ));
        self.diagnostics.push(diagnostic);
    }
}
