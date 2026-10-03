use super::*;
use crate::adt::registry::AdtRegistry;

impl<'a> FunctionChecker<'a> {
    pub(in crate::analysis) fn check_assignable(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
    ) {
        self.check_assignable_with_widening(
            expr,
            expected,
            actual,
            expected_source,
            constraint,
            true,
        );
    }

    pub(in crate::analysis) fn check_assignable_nested(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
    ) {
        self.check_assignable_with_widening(
            expr,
            expected,
            actual,
            expected_source,
            constraint,
            false,
        );
    }

    fn check_assignable_with_widening(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
        direct: bool,
    ) {
        if if direct {
            is_assignable(expected, actual)
        } else {
            is_assignable_nested(expected, actual)
        } {
            return;
        }
        if !type_contains_unknown(expected)
            && !type_contains_unknown(actual)
            && let Some(mismatch) = variant_mismatch_sets(expected, actual, &self.environment.adts)
        {
            let mut diagnostic = Diagnostic::new(
                "type.variant_mismatch",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "value of type `{}` is not assignable to variant type `{}`",
                    actual.render(),
                    expected.render()
                ),
                Some(expr.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type_check")),
                    ("node_id", JsonValue::string(expr.node_id.display("expr"))),
                    ("actual_type", JsonValue::string(actual.render())),
                    ("expected_type", JsonValue::string(expected.render())),
                    (
                        "expected_variants",
                        JsonValue::array(
                            mismatch
                                .expected_variants
                                .iter()
                                .cloned()
                                .map(JsonValue::string),
                        ),
                    ),
                    ("excluded_variants", mismatch.exclusion.json()),
                    ("constraint", JsonValue::string(constraint)),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("variant_exclusion")),
                (
                    "message",
                    JsonValue::string(
                        mismatch
                            .exclusion
                            .message(&actual.render(), &mismatch.expected_variants),
                    ),
                ),
                ("span", span_json(&expr.span)),
            ]));
            if let Some(origin_span) = &expected_source.origin_span {
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("expected_type_origin")),
                    ("message", JsonValue::string(expected_source.origin_message)),
                    ("span", span_json(origin_span)),
                ]));
            }
            self.diagnostics.push(diagnostic);
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                expected.render(),
                actual.render()
            ),
            Some(expr.span.clone()),
            type_details(
                expr.node_id.display("expr"),
                expected.render(),
                actual.render(),
                expected_source.source.as_type_source(),
                "inferred_expression",
                constraint,
                [
                    self.function.node_id.display("fn"),
                    expected_source.origin_node_id.display("expr"),
                    expr.node_id.display("expr"),
                ],
            ),
        ));
    }

    pub(super) fn push_invalid_type_annotation(
        &mut self,
        annotation: &str,
        error: &str,
        origin_node_id: NodeId,
        span: SourceSpan,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.invalid_annotation",
            Severity::Error,
            DiagnosticKind::Type,
            format!("invalid type annotation `{annotation}`: {error}"),
            Some(span),
            type_details(
                origin_node_id.display("expr"),
                "valid_type",
                annotation,
                "source",
                "source",
                "assignable",
                [
                    self.function.node_id.display("fn"),
                    origin_node_id.display("expr"),
                ],
            ),
        ));
    }

    pub(in crate::analysis) fn push_unresolved_name(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) {
        if matches!(namespace, "value" | "contract_predicate")
            && symbol == "callsite"
            && self.function.callsite.is_none()
            && self.supports_callsite_modifier
        {
            let mut diagnostic = Diagnostic::new(
                "name.callsite_requires_modifier",
                Severity::Error,
                DiagnosticKind::Name,
                "unresolved `callsite`; the function is missing the `callsite` modifier",
                Some(span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("name")),
                    ("node_id", JsonValue::string(node_id.display("name"))),
                    ("symbol", JsonValue::string(symbol)),
                    ("namespace", JsonValue::string(namespace)),
                    ("resolution_status", JsonValue::string("missing_modifier")),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("repair_hint")),
                (
                    "message",
                    JsonValue::string(
                        "Add `callsite` after the function's optional effects clause.",
                    ),
                ),
                ("span", span_json(&self.function.span)),
            ]));
            self.diagnostics.push(diagnostic);
            return;
        }
        if namespace == "value"
            && let Some(primitive) = exact_width_binary_primitive_name(symbol)
        {
            self.diagnostics
                .push(exact_width_schema_primitive_diagnostic(
                    primitive,
                    None,
                    None,
                    node_id.display("name"),
                    span,
                    "value_position",
                ));
            return;
        }
        if namespace == "value" && lowercase_schema_primitive(symbol).is_some() {
            self.diagnostics
                .push(lowercase_schema_primitive_position_diagnostic(
                    symbol,
                    None,
                    None,
                    node_id.display("name"),
                    span,
                    "value_position",
                ));
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "name.unresolved",
            Severity::Error,
            DiagnosticKind::Name,
            format!("unresolved {namespace} `{symbol}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("unresolved")),
                ("candidates", JsonValue::array([])),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_name(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "name.ambiguous",
            Severity::Error,
            DiagnosticKind::Name,
            format!("ambiguous {namespace} `{symbol}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("ambiguous")),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_constructor_type(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        symbol: &str,
        ty: &Type,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.inference_ambiguous",
            Severity::Error,
            DiagnosticKind::Type,
            format!("constructor `{symbol}` needs type context"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("slot_kind", JsonValue::string("constructor_type")),
                ("constructor", JsonValue::string(symbol)),
                ("inferred_type", JsonValue::string(ty.render())),
                ("constraint", JsonValue::string("constructor_type_context")),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_match_scrutinee_type(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        candidates: Vec<String>,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.inference_ambiguous",
            Severity::Error,
            DiagnosticKind::Type,
            "match scrutinee type is ambiguous",
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("slot_kind", JsonValue::string("match_scrutinee")),
                (
                    "candidates",
                    JsonValue::array(candidates.into_iter().map(JsonValue::string)),
                ),
                (
                    "constraint",
                    JsonValue::string("match_constructor_pattern_domain"),
                ),
            ]),
        ));
    }

    pub(super) fn push_ambiguous_empty_collection_type(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        collection: &str,
        ty: &Type,
    ) {
        self.diagnostics.push(Diagnostic::new(
            "type.inference_ambiguous",
            Severity::Error,
            DiagnosticKind::Type,
            format!("empty {collection} literal needs concrete type context"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("type")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("slot_kind", JsonValue::string("empty_collection")),
                ("collection", JsonValue::string(collection)),
                ("inferred_type", JsonValue::string(ty.render())),
                (
                    "constraint",
                    JsonValue::string("empty_collection_type_context"),
                ),
            ]),
        ));
    }

    pub(in crate::analysis) fn hole_constraints(
        &self,
        satisfy: Option<&SatisfyClause>,
        expected: Option<&Type>,
    ) -> Vec<JsonValue> {
        let mut constraints = self
            .function
            .contracts
            .iter()
            .map(|contract| {
                JsonValue::object([
                    ("kind", JsonValue::string("contract")),
                    (
                        "clause",
                        JsonValue::string(contract_kind_text(contract.kind)),
                    ),
                    ("text", JsonValue::string(contract.text.clone())),
                    ("validation_status", JsonValue::string("valid_unknown")),
                    (
                        "source_node_id",
                        JsonValue::string(contract.node_id.display("contract")),
                    ),
                ])
            })
            .collect::<Vec<_>>();
        if let Some(satisfy) = satisfy {
            let repair_status = expected
                .and_then(|expected| self.satisfy_repair_constraint(satisfy, expected))
                .filter(|constraint| self.constraint_has_assignable_candidate(expected, constraint))
                .map_or(SATISFY_STATUS_BLOCKED_UNTIL_DISCHARGED, |_| {
                    SATISFY_STATUS_STATICALLY_SATISFIED
                });
            constraints.push(JsonValue::object([
                ("kind", JsonValue::string("satisfy")),
                ("text", JsonValue::string(satisfy.predicate.clone())),
                (
                    "candidate_binding",
                    satisfy
                        .candidate
                        .as_ref()
                        .map_or(JsonValue::Null, JsonValue::string),
                ),
                ("validation_status", JsonValue::string("valid_unknown")),
                ("repair_status", JsonValue::string(repair_status)),
            ]));
        }
        constraints
    }

    pub(super) fn constraint_has_assignable_candidate(
        &self,
        expected: Option<&Type>,
        constraint: &SatisfyRepairConstraint,
    ) -> bool {
        let Some(expected) = expected.filter(|expected| **expected != Type::Unknown) else {
            return false;
        };
        self.bindings.iter().any(|binding| {
            is_assignable(expected, &binding.ty)
                && constraint.reason_for(binding.name.as_str()).is_some()
        })
    }

    pub(in crate::analysis) fn candidate_queries(
        &self,
        expected: Option<&Type>,
        hole: &Expr,
        satisfy: Option<&SatisfyClause>,
    ) -> Vec<JsonValue> {
        let Some(expected) = expected.filter(|expected| **expected != Type::Unknown) else {
            return Vec::new();
        };
        let argument_types = self
            .bindings
            .iter()
            .map(|binding| binding.ty.render())
            .collect::<Vec<_>>()
            .join(", ");
        let repair_constraint =
            satisfy.and_then(|satisfy| self.satisfy_repair_constraint(satisfy, expected));
        let ranked_candidates =
            self.ranked_symbol_candidates(expected, hole, repair_constraint.as_ref());
        let mut query = vec![
            ("kind", JsonValue::string("symbol")),
            (
                "candidate_status",
                JsonValue::string(CANDIDATE_STATUS_QUERY_ONLY),
            ),
            (
                "application_policy",
                JsonValue::string(APPLICATION_POLICY_MANUAL_REVIEW_REQUIRED),
            ),
            (
                "query",
                JsonValue::string(format!("fn({argument_types}) -> {}", expected.render())),
            ),
        ];
        if let Some(satisfy) = satisfy {
            query.push((
                "satisfy_predicate",
                JsonValue::string(satisfy.predicate.clone()),
            ));
            query.push((
                "satisfy_candidate_binding",
                satisfy
                    .candidate
                    .as_ref()
                    .map_or(JsonValue::Null, JsonValue::string),
            ));
        }
        if !ranked_candidates.is_empty() {
            query.push(("candidates", JsonValue::array(ranked_candidates)));
        }
        vec![JsonValue::object(query)]
    }

    pub(super) fn ranked_symbol_candidates(
        &self,
        expected: &Type,
        hole: &Expr,
        satisfy: Option<&SatisfyRepairConstraint>,
    ) -> Vec<JsonValue> {
        let mut candidates = self
            .bindings
            .iter()
            .rev()
            .enumerate()
            .filter(|(_, binding)| is_assignable(expected, &binding.ty))
            .map(|(distance, binding)| {
                let score = if binding.ty == *expected { 0 } else { 1 };
                (score, distance, binding)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.cmp(&right.1))
                .then(left.2.name.cmp(&right.2.name))
        });
        candidates
            .into_iter()
            .enumerate()
            .filter_map(|(sorted_index, candidate)| {
                let static_satisfy =
                    satisfy.and_then(|satisfy| satisfy.reason_for(candidate.2.name.as_str()));
                (sorted_index < 5 || static_satisfy.is_some())
                    .then_some((candidate, static_satisfy))
            })
            .enumerate()
            .map(|(index, ((score, _, binding), static_satisfy))| {
                let rank = index + 1;
                let reason = if let Some(reason) = static_satisfy {
                    reason
                } else if score == 0 {
                    "exact_type_match"
                } else {
                    "assignable_type_match"
                };
                let policy = application_policy(static_satisfy.is_some());
                let satisfy_status =
                    candidate_satisfy_status(satisfy.is_some(), static_satisfy.is_some());
                let mut candidate = vec![
                    ("candidate_id", JsonValue::string(format!("symbol-{rank}"))),
                    ("name", JsonValue::string(binding.name.clone())),
                    ("type", JsonValue::string(binding.ty.render())),
                    ("rank", JsonValue::Number(rank as i64)),
                    ("reason", JsonValue::string(reason)),
                    ("application_policy", JsonValue::string(policy)),
                    (
                        "edits",
                        JsonValue::array([JsonValue::object([
                            ("kind", JsonValue::string("replace")),
                            ("span", span_json(&hole.span)),
                            ("replacement", JsonValue::string(binding.name.clone())),
                        ])]),
                    ),
                    (
                        "target",
                        JsonValue::object([
                            ("node_id", JsonValue::string(hole.node_id.display("hole"))),
                            ("span", span_json(&hole.span)),
                        ]),
                    ),
                    (
                        "edit_summary",
                        JsonValue::string(format!("Replace hole with `{}`", binding.name)),
                    ),
                    (
                        "evidence",
                        candidate_evidence(expected, &binding.ty, rank, reason, satisfy_status),
                    ),
                    ("known_limits", candidate_known_limits(satisfy_status)),
                    (
                        "blocking_obligations",
                        candidate_blocking_obligations(policy, satisfy_status),
                    ),
                    (
                        "verification_hint",
                        JsonValue::object([
                            (
                                "command",
                                JsonValue::string(format!(
                                    "veln check --json {}",
                                    hole.span.file.as_str()
                                )),
                            ),
                            ("scope", JsonValue::string("after_applying_candidate_edit")),
                        ]),
                    ),
                    (
                        "application_status",
                        JsonValue::string(APPLICATION_STATUS_UNAPPLIED),
                    ),
                ];
                if let Some(satisfy_status) = satisfy_status {
                    candidate.push(("satisfy_status", JsonValue::string(satisfy_status)));
                }
                JsonValue::object(candidate)
            })
            .collect()
    }

    pub(super) fn satisfy_repair_constraint(
        &self,
        satisfy: &SatisfyClause,
        expected: &Type,
    ) -> Option<SatisfyRepairConstraint> {
        let allow_static_truth = self.valid_static_satisfy_predicate(satisfy, expected);
        let mut direct_constraint =
            SatisfyRepairConstraint::from_satisfy(satisfy, allow_static_truth);
        if direct_constraint
            .as_ref()
            .is_some_and(SatisfyRepairConstraint::allows_any_binding)
        {
            return direct_constraint;
        }
        let candidate = satisfy.candidate.as_ref()?;
        let required_predicates = self
            .function
            .contracts
            .iter()
            .filter(|contract| contract.kind != ContractKind::Ensure)
            .filter(|contract| {
                matches!(
                    self.validate_contract_predicate(contract.kind, &contract.text),
                    ContractValidation::Valid
                )
            })
            .map(|contract| contract.text.clone())
            .collect::<Vec<_>>();
        let proof_context = (!required_predicates.is_empty())
            .then(|| RequiredPredicateProofContext::new(&required_predicates));
        let allowed_bindings = self.satisfy_allowed_bindings(
            satisfy,
            candidate,
            allow_static_truth,
            proof_context.as_ref(),
        );
        if let Some(constraint) = &mut direct_constraint {
            constraint.extend_allowed_bindings(allowed_bindings);
            return direct_constraint;
        }
        (!allowed_bindings.is_empty()).then_some(SatisfyRepairConstraint {
            allowed_bindings: Some(allowed_bindings),
            reason: if proof_context.is_some() {
                "satisfy_require_match"
            } else {
                "satisfy_tautology"
            },
        })
    }

    fn satisfy_allowed_bindings(
        &self,
        satisfy: &SatisfyClause,
        candidate: &str,
        allow_static_truth: bool,
        proof_context: Option<&RequiredPredicateProofContext<'_>>,
    ) -> Vec<SatisfyAllowedBinding> {
        self.bindings
            .iter()
            .filter_map(|binding| {
                let replaced = replace_identifier(&satisfy.predicate, candidate, &binding.name);
                let reason = if allow_static_truth
                    && predicate_is_statically_true_with_literal_bounds(&replaced)
                {
                    "satisfy_tautology"
                } else if proof_context.is_some_and(|proof| {
                    proof.guarantees(&replaced)
                        || (binding.ty == Type::int() && proof.guarantees_int_successor(&replaced))
                }) {
                    "satisfy_require_match"
                } else {
                    return None;
                };
                Some(SatisfyAllowedBinding {
                    name: binding.name.clone(),
                    reason,
                })
            })
            .collect()
    }

    pub(super) fn valid_static_satisfy_predicate(
        &self,
        satisfy: &SatisfyClause,
        expected: &Type,
    ) -> bool {
        let Some(candidate) = satisfy.candidate.as_ref() else {
            return false;
        };
        let mut predicate_bindings = self.bindings.clone();
        predicate_bindings.push(Binding::new(candidate.clone(), expected.clone()));
        matches!(
            self.validate_predicate_with_bindings(&satisfy.predicate, &predicate_bindings),
            ContractValidation::Valid
        )
    }
}

fn variant_mismatch_sets(
    expected: &Type,
    actual: &Type,
    adts: &AdtRegistry,
) -> Option<VariantMismatchFacts> {
    match (expected, actual) {
        (
            Type::VariantRefinement {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
                variants: expected_variants,
                ..
            },
            Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                variants: actual_variants,
                ..
            },
        ) => {
            if expected_identity != actual_identity && expected_name != actual_name {
                return None;
            }
            let excluded = if expected_identity == actual_identity && expected_args == actual_args {
                actual_variants
                    .iter()
                    .filter(|variant| !expected_variants.contains(variant))
                    .cloned()
                    .collect::<Vec<_>>()
            } else {
                actual_variants.clone()
            };
            (!excluded.is_empty()).then(|| VariantMismatchFacts {
                expected_variants: expected_variants.clone(),
                exclusion: VariantExclusion::Listed(excluded),
            })
        }
        (
            Type::VariantRefinement {
                name: expected_name,
                variants,
                ..
            },
            Type::Named {
                name: actual_name, ..
            },
        ) if expected_name == actual_name => {
            adts.descriptor_for_type(actual)?;
            Some(VariantMismatchFacts {
                expected_variants: variants.clone(),
                exclusion: VariantExclusion::AllExceptExpected,
            })
        }
        (
            Type::Named {
                name: expected_name,
                ..
            },
            Type::VariantRefinement {
                name: actual_name,
                variants,
                ..
            },
        ) if expected_name == actual_name => {
            let expected_variants = adts.variant_names_for_type(expected)?;
            Some(VariantMismatchFacts {
                expected_variants,
                exclusion: VariantExclusion::Listed(variants.clone()),
            })
        }
        (Type::Named { args: expected, .. }, Type::Named { args: actual, .. }) => expected
            .iter()
            .zip(actual)
            .find_map(|(expected, actual)| variant_mismatch_sets(expected, actual, adts)),
        (Type::Record(expected), Type::Record(actual)) => {
            expected.iter().find_map(|(name, expected)| {
                actual
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                    .and_then(|(_, actual)| variant_mismatch_sets(expected, actual, adts))
            })
        }
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                ..
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                ..
            },
        ) => expected_params
            .iter()
            .zip(actual_params)
            .find_map(|(expected, actual)| variant_mismatch_sets(expected, actual, adts))
            .or_else(|| match (expected_variadic, actual_variadic) {
                (Some(expected), Some(actual)) => variant_mismatch_sets(expected, actual, adts),
                _ => None,
            })
            .or_else(|| variant_mismatch_sets(expected_return, actual_return, adts)),
        _ => None,
    }
}

struct VariantMismatchFacts {
    expected_variants: Vec<String>,
    exclusion: VariantExclusion,
}

enum VariantExclusion {
    Listed(Vec<String>),
    AllExceptExpected,
}

impl VariantExclusion {
    fn json(&self) -> JsonValue {
        match self {
            Self::Listed(variants) => JsonValue::object([
                ("form", JsonValue::string("listed")),
                (
                    "variants",
                    JsonValue::array(variants.iter().cloned().map(JsonValue::string)),
                ),
            ]),
            Self::AllExceptExpected => JsonValue::object([
                ("form", JsonValue::string("all_except_expected")),
                ("variants", JsonValue::array(std::iter::empty())),
            ]),
        }
    }

    fn message(&self, actual: &str, expected: &[String]) -> String {
        match self {
            Self::Listed(variants) => format!("Excluded variants: {}.", variants.join(", ")),
            Self::AllExceptExpected => format!(
                "Excluded variants: every `{actual}` variant except {}.",
                expected.join(", ")
            ),
        }
    }
}
