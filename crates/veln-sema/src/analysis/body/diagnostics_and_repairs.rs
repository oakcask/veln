use std::collections::HashSet;
use std::sync::Arc;

use super::*;
use crate::adt::registry::AdtRegistry;

#[cfg(test)]
thread_local! {
    static RETAINED_VARIANT_DIAGNOSTIC_KEY_VARIANTS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_retained_variant_diagnostic_key_variants() {
    RETAINED_VARIANT_DIAGNOSTIC_KEY_VARIANTS.with(|retained| retained.set(0));
}

#[cfg(test)]
pub(crate) fn take_retained_variant_diagnostic_key_variants() -> usize {
    RETAINED_VARIANT_DIAGNOSTIC_KEY_VARIANTS.with(|retained| retained.replace(0))
}

#[cfg(test)]
fn record_retained_variant_diagnostic_key(ty: &Type) {
    let Type::VariantRefinement { variants, .. } = ty else {
        return;
    };
    RETAINED_VARIANT_DIAGNOSTIC_KEY_VARIANTS.with(|retained| {
        retained.set(retained.get() + variants.len());
    });
}

struct RankedSymbolCandidate {
    type_score: u8,
    distance: usize,
    binding_index: usize,
    binding_type: Type,
}

impl<'a> FunctionChecker<'a> {
    pub(in crate::analysis) fn check_schema_encode_assignable(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
    ) {
        if is_assignable(expected, actual) {
            return;
        }
        if !type_contains_unknown(expected)
            && !type_contains_unknown(actual)
            && let Some(mismatch) =
                schema_encode_variant_mismatch(expected, actual, &self.environment.adts)
        {
            self.push_variant_mismatch_diagnostic(
                expr,
                mismatch.expected,
                mismatch.actual,
                expected_source,
                constraint,
                mismatch.facts,
            );
            return;
        }
        self.push_type_mismatch_diagnostic(expr, expected, actual, expected_source, constraint);
    }

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
        if direct
            && !type_contains_unknown(expected)
            && !type_contains_unknown(actual)
            && let Some(mismatch) = variant_mismatch_sets(expected, actual, &self.environment.adts)
        {
            self.push_variant_mismatch_diagnostic(
                expr,
                expected,
                actual,
                expected_source,
                constraint,
                mismatch,
            );
            return;
        }
        self.push_type_mismatch_diagnostic(expr, expected, actual, expected_source, constraint);
    }

    fn push_variant_mismatch_diagnostic(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
        mismatch: VariantMismatchFacts<'_>,
    ) {
        let actual_text = self.variant_diagnostics.rendered_type(actual);
        let expected_facts = self
            .variant_diagnostics
            .expected_facts(expected, mismatch.expected_variants);
        let exclusion_facts = self.variant_diagnostics.exclusion_facts(
            actual,
            &mismatch.exclusion,
            &actual_text,
            &expected_facts,
        );
        let mut diagnostic = variant_mismatch_diagnostic(
            expr,
            constraint,
            &actual_text,
            &expected_facts,
            &exclusion_facts,
        );
        if let Some(origin_span) = &expected_source.origin_span {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("expected_type_origin")),
                ("message", JsonValue::string(expected_source.origin_message)),
                ("span", span_json(origin_span)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    fn push_type_mismatch_diagnostic(
        &mut self,
        expr: &Expr,
        expected: &Type,
        actual: &Type,
        expected_source: &ExpectedType,
        constraint: &'static str,
    ) {
        let diagnostic = Diagnostic::new(
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
        );
        self.diagnostics.push(diagnostic);
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
        if self.push_missing_callsite_modifier(node_id, &span, symbol, namespace) {
            return;
        }
        if self.push_schema_primitive_position(node_id, &span, symbol, namespace) {
            return;
        }
        let mut diagnostic = Diagnostic::new(
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
        );
        self.add_legacy_network_migration_notes(&mut diagnostic, symbol, namespace);
        self.diagnostics.push(diagnostic);
    }

    fn push_missing_callsite_modifier(
        &mut self,
        node_id: NodeId,
        span: &SourceSpan,
        symbol: &str,
        namespace: &'static str,
    ) -> bool {
        if !matches!(namespace, "value" | "contract_predicate")
            || symbol != "callsite"
            || self.function.callsite.is_some()
            || !self.supports_callsite_modifier
        {
            return false;
        }
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
                JsonValue::string("Add `callsite` after the function's optional effects clause."),
            ),
            ("span", span_json(&self.function.span)),
        ]));
        self.diagnostics.push(diagnostic);
        true
    }

    fn push_schema_primitive_position(
        &mut self,
        node_id: NodeId,
        span: &SourceSpan,
        symbol: &str,
        namespace: &str,
    ) -> bool {
        if namespace != "value" {
            return false;
        }
        if let Some(primitive) = exact_width_binary_primitive_name(symbol) {
            self.diagnostics
                .push(exact_width_schema_primitive_diagnostic(
                    primitive,
                    None,
                    None,
                    node_id.display("name"),
                    span.clone(),
                    "value_position",
                ));
            return true;
        }
        if lowercase_schema_primitive(symbol).is_none() {
            return false;
        }
        self.diagnostics
            .push(lowercase_schema_primitive_position_diagnostic(
                symbol,
                None,
                None,
                node_id.display("name"),
                span.clone(),
                "value_position",
            ));
        true
    }

    fn add_legacy_network_migration_notes(
        &self,
        diagnostic: &mut Diagnostic,
        symbol: &str,
        namespace: &str,
    ) {
        if namespace != "call_target" || !removed_legacy_network_call(symbol) {
            return;
        }
        if self.environment.uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == self.function.module_name.as_deref()
                && use_decl.alias == "net"
        }) {
            return;
        }
        for (kind, message) in [
            (
                "network_import_migration",
                "Import the exported network module with `use net from \"std\"`.",
            ),
            (
                "network_effect_migration",
                "Declare the nominal `net::IO` effect for functions that use network operations.",
            ),
            (
                "network_handler_migration",
                "At the application boundary, handle `net::IO` with `net::system()`.",
            ),
        ] {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string(kind)),
                ("message", JsonValue::string(message)),
            ]));
        }
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
        self.effective_visible_bindings()
            .into_iter()
            .any(|(index, ty)| {
                is_assignable(expected, &ty)
                    && constraint
                        .reason_for(self.bindings[index].name.as_str())
                        .is_some()
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
            .effective_visible_bindings()
            .into_iter()
            .map(|(_, ty)| ty.render())
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
            .effective_visible_bindings()
            .into_iter()
            .rev()
            .enumerate()
            .filter(|(_, (_, ty))| is_assignable(expected, ty))
            .map(|(distance, (index, ty))| RankedSymbolCandidate {
                type_score: u8::from(ty != *expected),
                distance,
                binding_index: index,
                binding_type: ty,
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            left.type_score
                .cmp(&right.type_score)
                .then(left.distance.cmp(&right.distance))
        });
        let mut output = Vec::new();
        for (sorted_index, candidate) in candidates.into_iter().enumerate() {
            let binding = &self.bindings[candidate.binding_index];
            let static_satisfy =
                satisfy.and_then(|constraint| constraint.reason_for(binding.name.as_str()));
            if sorted_index >= 5 && static_satisfy.is_none() {
                continue;
            }
            let rank = output.len() + 1;
            output.push(self.symbol_candidate_json(
                expected,
                hole,
                candidate,
                rank,
                static_satisfy,
                satisfy.is_some(),
            ));
        }
        output
    }

    fn symbol_candidate_json(
        &self,
        expected: &Type,
        hole: &Expr,
        candidate: RankedSymbolCandidate,
        rank: usize,
        static_satisfy: Option<&'static str>,
        has_satisfy: bool,
    ) -> JsonValue {
        let binding = &self.bindings[candidate.binding_index];
        let reason = static_satisfy.unwrap_or(if candidate.type_score == 0 {
            "exact_type_match"
        } else {
            "assignable_type_match"
        });
        let policy = application_policy(static_satisfy.is_some());
        let satisfy_status = candidate_satisfy_status(has_satisfy, static_satisfy.is_some());
        let mut fields = vec![
            ("candidate_id", JsonValue::string(format!("symbol-{rank}"))),
            ("name", JsonValue::string(binding.name.clone())),
            ("type", JsonValue::string(candidate.binding_type.render())),
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
                candidate_evidence(
                    expected,
                    &candidate.binding_type,
                    rank,
                    reason,
                    satisfy_status,
                ),
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
                        JsonValue::string(format!("veln check --json {}", hole.span.file.as_str())),
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
            fields.push(("satisfy_status", JsonValue::string(satisfy_status)));
        }
        JsonValue::object(fields)
    }

    pub(super) fn satisfy_repair_constraint(
        &self,
        satisfy: &SatisfyClause,
        expected: &Type,
    ) -> Option<SatisfyRepairConstraint> {
        if predicate_has_nonfoldable_remainder_with_remainders(
            &satisfy.predicate,
            &satisfy.remainder_spans,
        ) {
            return None;
        }
        let mut folded_satisfy = satisfy.clone();
        folded_satisfy.predicate =
            predicate_with_folded_literal_remainders(&satisfy.predicate, &satisfy.remainder_spans)?;
        folded_satisfy.remainder_spans.clear();
        let allow_static_truth = self.valid_static_satisfy_predicate(satisfy, expected);
        let mut direct_constraint =
            SatisfyRepairConstraint::from_satisfy(&folded_satisfy, allow_static_truth);
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
                    self.validate_contract_predicate(contract),
                    ContractValidation::Valid
                )
            })
            .map(|contract| contract.text.clone())
            .collect::<Vec<_>>();
        let proof_context = (!required_predicates.is_empty())
            .then(|| RequiredPredicateProofContext::new(&required_predicates));
        let allowed_bindings = self.satisfy_allowed_bindings(
            &folded_satisfy,
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
        self.effective_visible_bindings()
            .into_iter()
            .filter_map(|(index, ty)| {
                let binding = &self.bindings[index];
                let replaced = replace_identifier(&satisfy.predicate, candidate, &binding.name);
                let reason = if allow_static_truth
                    && predicate_is_statically_true_with_literal_bounds(&replaced)
                {
                    "satisfy_tautology"
                } else if proof_context.is_some_and(|proof| {
                    proof.guarantees(&replaced)
                        || (ty == Type::int() && proof.guarantees_int_successor(&replaced))
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
}

fn removed_legacy_network_call(symbol: &str) -> bool {
    let Some(name) = symbol.strip_prefix("net::") else {
        return false;
    };
    matches!(
        name,
        "receive_chunk"
            | "send_chunk"
            | "listen"
            | "connect"
            | "accept"
            | "accept_or_end"
            | "accept_until"
            | "accept_until_cancellable"
            | "listener_local_addr"
            | "read_chunk"
            | "stream_local_addr"
            | "stream_peer_addr"
            | "stream_can_read"
            | "stream_can_write"
            | "stream_is_closed"
            | "read_chunk_until"
            | "read_chunk_until_cancellable"
            | "read_chunk_or_end"
            | "write_chunk"
            | "write_chunk_until"
            | "write_chunk_until_cancellable"
            | "write_chunks"
            | "write_chunks_until"
            | "write_chunks_until_cancellable"
            | "shutdown_write"
            | "shutdown_read"
            | "close_stream"
            | "close_listener"
    )
}

fn variant_mismatch_sets<'a>(
    expected: &'a Type,
    actual: &'a Type,
    adts: &AdtRegistry,
) -> Option<VariantMismatchFacts<'a>> {
    match (expected, actual) {
        (
            Type::VariantRefinement {
                identity: expected_identity,
                args: expected_args,
                variants: expected_variants,
                ..
            },
            Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                variants: actual_variants,
                ..
            },
        ) => {
            if expected_identity != actual_identity {
                return None;
            }
            if !crate::type_relations::invariant_args_match(expected_args, actual_args) {
                return None;
            }
            let expected_variants_set = expected_variants
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            let excluded = actual_variants
                .iter()
                .filter(|variant| !expected_variants_set.contains(variant.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            (!excluded.is_empty()).then(|| VariantMismatchFacts {
                expected_variants,
                exclusion: VariantExclusion::Listed(excluded),
            })
        }
        (
            Type::VariantRefinement {
                identity: expected_identity,
                args: expected_args,
                variants,
                ..
            },
            Type::Named {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if expected_identity == actual_identity
            && crate::type_relations::invariant_args_match(expected_args, actual_args) =>
        {
            adts.descriptor_for_type(actual)?;
            Some(VariantMismatchFacts {
                expected_variants: variants,
                exclusion: VariantExclusion::AllExceptExpected,
            })
        }
        _ => None,
    }
}

struct NestedVariantMismatch<'a> {
    expected: &'a Type,
    actual: &'a Type,
    facts: VariantMismatchFacts<'a>,
}

enum SchemaEncodeTypeComparison<'a> {
    Assignable,
    VariantMismatch(NestedVariantMismatch<'a>),
    OtherMismatch,
}

fn schema_encode_variant_mismatch<'a>(
    expected: &'a Type,
    actual: &'a Type,
    adts: &AdtRegistry,
) -> Option<NestedVariantMismatch<'a>> {
    match compare_schema_encode_types(expected, actual, adts) {
        SchemaEncodeTypeComparison::VariantMismatch(mismatch) => Some(mismatch),
        SchemaEncodeTypeComparison::Assignable | SchemaEncodeTypeComparison::OtherMismatch => None,
    }
}

fn compare_schema_encode_types<'a>(
    expected: &'a Type,
    actual: &'a Type,
    adts: &AdtRegistry,
) -> SchemaEncodeTypeComparison<'a> {
    if is_assignable(expected, actual) {
        return SchemaEncodeTypeComparison::Assignable;
    }
    if let Some(facts) = variant_mismatch_sets(expected, actual, adts) {
        return SchemaEncodeTypeComparison::VariantMismatch(NestedVariantMismatch {
            expected,
            actual,
            facts,
        });
    }
    match (expected, actual) {
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
        ) if schema_encode_container_arity(expected_name) == Some(expected_args.len())
            && expected_name == actual_name
            && expected_identity == actual_identity
            && expected_args.len() == actual_args.len() =>
        {
            compare_schema_encode_children(expected_args.iter().zip(actual_args), adts)
        }
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            let mut comparisons = Vec::with_capacity(expected_fields.len());
            for (expected_name, expected_ty) in expected_fields {
                let Some((_, actual_ty)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == expected_name)
                else {
                    return SchemaEncodeTypeComparison::OtherMismatch;
                };
                comparisons.push((expected_ty, actual_ty));
            }
            compare_schema_encode_children(comparisons, adts)
        }
        _ => SchemaEncodeTypeComparison::OtherMismatch,
    }
}

fn compare_schema_encode_children<'a>(
    children: impl IntoIterator<Item = (&'a Type, &'a Type)>,
    adts: &AdtRegistry,
) -> SchemaEncodeTypeComparison<'a> {
    let mut nested_mismatch = None;
    for (expected, actual) in children {
        match compare_schema_encode_types(expected, actual, adts) {
            SchemaEncodeTypeComparison::Assignable => {}
            SchemaEncodeTypeComparison::VariantMismatch(mismatch) => {
                nested_mismatch.get_or_insert(mismatch);
            }
            SchemaEncodeTypeComparison::OtherMismatch => {
                return SchemaEncodeTypeComparison::OtherMismatch;
            }
        }
    }
    nested_mismatch.map_or(
        SchemaEncodeTypeComparison::Assignable,
        SchemaEncodeTypeComparison::VariantMismatch,
    )
}

fn schema_encode_container_arity(name: &str) -> Option<usize> {
    match name {
        "Option" | "List" | "Vec" => Some(1),
        "Dict" | "Result" => Some(2),
        _ => None,
    }
}

fn variant_mismatch_diagnostic(
    expr: &Expr,
    constraint: &'static str,
    actual_text: &DiagnosticText,
    expected: &ExpectedVariantDiagnosticFacts,
    exclusion: &VariantExclusionDiagnosticFacts,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new_text(
        "type.variant_mismatch",
        Severity::Error,
        DiagnosticKind::Type,
        exclusion.primary_message.clone(),
        Some(expr.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("type_check")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("actual_type", JsonValue::text(actual_text.clone())),
            (
                "expected_type",
                JsonValue::text(expected.rendered_type.clone()),
            ),
            ("expected_variants", expected.expected_variants.clone()),
            ("excluded_variants", exclusion.excluded_variants.clone()),
            ("constraint", JsonValue::string(constraint)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("variant_exclusion")),
        ("message", JsonValue::text(exclusion.message.clone())),
        ("span", span_json(&expr.span)),
    ]));
    diagnostic
}

struct VariantMismatchFacts<'a> {
    expected_variants: &'a [String],
    exclusion: VariantExclusion,
}

enum VariantExclusion {
    Listed(Vec<String>),
    AllExceptExpected,
}

#[derive(Default)]
pub(crate) struct VariantDiagnosticInterner {
    rendered_types: HashMap<Type, DiagnosticText>,
    expected: HashMap<Type, Arc<ExpectedVariantDiagnosticFacts>>,
    exclusions: HashMap<(usize, Type), Arc<VariantExclusionDiagnosticFacts>>,
}

struct ExpectedVariantDiagnosticFacts {
    cache_id: usize,
    rendered_type: DiagnosticText,
    expected_variants: JsonValue,
    joined_variants: DiagnosticText,
}

struct VariantExclusionDiagnosticFacts {
    excluded_variants: JsonValue,
    primary_message: DiagnosticText,
    message: DiagnosticText,
}

impl VariantDiagnosticInterner {
    fn rendered_type(&mut self, ty: &Type) -> DiagnosticText {
        self.rendered_types
            .entry(ty.clone())
            .or_insert_with(|| DiagnosticText::from(ty.render()))
            .clone()
    }

    fn expected_facts(
        &mut self,
        expected: &Type,
        variants: &[String],
    ) -> Arc<ExpectedVariantDiagnosticFacts> {
        if let Some(facts) = self.expected.get(expected) {
            return facts.clone();
        }
        let facts = Arc::new(ExpectedVariantDiagnosticFacts {
            cache_id: self.expected.len(),
            rendered_type: self.rendered_type(expected),
            expected_variants: JsonValue::shared(JsonValue::array(
                variants.iter().cloned().map(JsonValue::string),
            )),
            joined_variants: DiagnosticText::from(variants.join(", ")),
        });
        self.expected.insert(expected.clone(), facts.clone());
        #[cfg(test)]
        record_retained_variant_diagnostic_key(expected);
        facts
    }

    fn exclusion_facts(
        &mut self,
        actual: &Type,
        exclusion: &VariantExclusion,
        actual_text: &DiagnosticText,
        expected_facts: &ExpectedVariantDiagnosticFacts,
    ) -> Arc<VariantExclusionDiagnosticFacts> {
        let key = (expected_facts.cache_id, actual.clone());
        if let Some(facts) = self.exclusions.get(&key) {
            return facts.clone();
        }
        let facts = Arc::new(match exclusion {
            VariantExclusion::Listed(variants) => {
                let joined = DiagnosticText::from(variants.join(", "));
                VariantExclusionDiagnosticFacts {
                    excluded_variants: JsonValue::shared(JsonValue::object([
                        ("form", JsonValue::string("listed")),
                        (
                            "variants",
                            JsonValue::array(variants.iter().cloned().map(JsonValue::string)),
                        ),
                    ])),
                    primary_message: variant_mismatch_message(actual_text, expected_facts),
                    message: DiagnosticText::parts([
                        DiagnosticText::from("Excluded variants: "),
                        joined,
                        DiagnosticText::from("."),
                    ]),
                }
            }
            VariantExclusion::AllExceptExpected => VariantExclusionDiagnosticFacts {
                excluded_variants: JsonValue::shared(JsonValue::object([
                    ("form", JsonValue::string("all_except_expected")),
                    ("variants", JsonValue::array(std::iter::empty())),
                ])),
                primary_message: variant_mismatch_message(actual_text, expected_facts),
                message: DiagnosticText::parts([
                    DiagnosticText::from("Excluded variants: every `"),
                    actual_text.clone(),
                    DiagnosticText::from("` variant except "),
                    expected_facts.joined_variants.clone(),
                    DiagnosticText::from("."),
                ]),
            },
        });
        self.exclusions.insert(key, facts.clone());
        #[cfg(test)]
        record_retained_variant_diagnostic_key(actual);
        facts
    }
}

fn variant_mismatch_message(
    actual: &DiagnosticText,
    expected: &ExpectedVariantDiagnosticFacts,
) -> DiagnosticText {
    DiagnosticText::parts([
        DiagnosticText::from("value of type `"),
        actual.clone(),
        DiagnosticText::from("` is not assignable to variant type `"),
        expected.rendered_type.clone(),
        DiagnosticText::from("`"),
    ])
}
