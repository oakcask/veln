use super::control_flow_results::{ControlFlowResultJoin, contribute_control_flow_result};
use super::*;
use crate::adt::descriptors::AdtPayloadField;
use crate::adt::registry::{AdtRegistry, VariantDeclarationOrder};
use std::sync::Arc;

#[cfg(test)]
fn record_refined_match_coverage_work(units: usize) {
    refined_match_coverage_work::record(units);
}

#[cfg(not(test))]
fn record_refined_match_coverage_work(_units: usize) {}

#[cfg(test)]
fn record_refined_match_diagnostic_render() {
    refined_match_diagnostic_work::record_render();
}

#[cfg(not(test))]
fn record_refined_match_diagnostic_render() {}

#[cfg(test)]
fn record_refined_match_diagnostic_retained_bytes(bytes: usize) {
    refined_match_diagnostic_work::record_retained_bytes(bytes);
}

#[cfg(not(test))]
fn record_refined_match_diagnostic_retained_bytes(_bytes: usize) {}

#[cfg(test)]
fn record_refined_match_refinement_variants(variants: usize) {
    refined_match_diagnostic_work::record_refinement_variants(variants);
}

#[cfg(not(test))]
fn record_refined_match_refinement_variants(_variants: usize) {}

#[cfg(test)]
pub(crate) fn reset_refined_match_coverage_work() {
    refined_match_coverage_work::reset();
}

#[cfg(test)]
pub(crate) fn take_refined_match_coverage_work() -> usize {
    refined_match_coverage_work::take()
}

#[cfg(test)]
pub(crate) fn reset_refined_match_diagnostic_work() {
    refined_match_diagnostic_work::reset();
}

#[cfg(test)]
pub(crate) fn take_refined_match_diagnostic_work() -> RefinedMatchDiagnosticWork {
    refined_match_diagnostic_work::take()
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct RefinedMatchDiagnosticWork {
    pub(crate) renders: usize,
    pub(crate) retained_bytes: usize,
    pub(crate) refinement_variants: usize,
}

struct RefinedMatchCoverage {
    declaration_order: Arc<VariantDeclarationOrder>,
    coverage_cases: Vec<String>,
    domain_ranks: Vec<usize>,
    slots: Vec<RefinedMatchCoverageSlot>,
    covered_order: Vec<usize>,
    remaining_count: usize,
    preceding_catch_all: Option<SourceSpan>,
    diagnostic_facts: RefinedMatchDiagnosticFacts,
}

struct RefinedMatchDiagnosticFacts {
    scrutinee_type: JsonValue,
    refinement_source_message: DiagnosticText,
    scrutinee_span: SourceSpan,
    adt_declaration_message: DiagnosticText,
    adt_declaration_span: Option<SourceSpan>,
}

enum RefinedMatchCoverageSlot {
    Outside,
    Remaining,
    Covered(SourceSpan),
}

struct RefinedMatchArmPlan {
    arm_type: Type,
    coverage: RefinedMatchArmCoverage,
}

enum RefinedMatchArmCoverage {
    Constructor {
        variant: String,
        variant_rank: usize,
        variant_span: SourceSpan,
        classification: RefinedConstructorClassification,
    },
    CatchAll {
        classification: RefinedCatchAllClassification,
        remaining_ranks: Vec<usize>,
    },
}

enum RefinedConstructorClassification {
    Covering,
    Impossible,
    Redundant,
}

enum RefinedCatchAllClassification {
    Covering,
    Redundant,
}

impl RefinedMatchCoverage {
    fn new(
        scrutinee: &Expr,
        scrutinee_type: &Type,
        adts: &AdtRegistry,
        current_module: Option<&str>,
    ) -> Option<Self> {
        let Type::VariantRefinement { variants, .. } = scrutinee_type else {
            return None;
        };
        let declaration_order = adts.variant_declaration_order_for_type(scrutinee_type)?;
        let descriptor = adts.descriptor_for_type_prefer_module(scrutinee_type, current_module)?;
        let coverage_cases = descriptor
            .variants
            .iter()
            .map(|variant| variant.coverage_case.clone())
            .collect::<Vec<_>>();
        let mut slots = (0..declaration_order.len())
            .map(|_| RefinedMatchCoverageSlot::Outside)
            .collect::<Vec<_>>();
        let mut domain_ranks = Vec::with_capacity(variants.len());
        for variant in variants.iter() {
            record_refined_match_coverage_work(1);
            let rank = declaration_order.rank(variant)?;
            slots[rank] = RefinedMatchCoverageSlot::Remaining;
            domain_ranks.push(rank);
        }
        Some(Self {
            declaration_order,
            coverage_cases,
            domain_ranks,
            slots,
            covered_order: Vec::new(),
            remaining_count: variants.len(),
            preceding_catch_all: None,
            diagnostic_facts: RefinedMatchDiagnosticFacts::new(scrutinee, scrutinee_type, adts),
        })
    }

    fn rank(&self, variant: &str) -> Option<usize> {
        record_refined_match_coverage_work(1);
        self.declaration_order.rank(variant)
    }

    fn slot(&self, rank: usize) -> &RefinedMatchCoverageSlot {
        record_refined_match_coverage_work(1);
        &self.slots[rank]
    }

    fn remaining_ranks(&self) -> Vec<usize> {
        self.domain_ranks
            .iter()
            .copied()
            .filter(|rank| {
                record_refined_match_coverage_work(1);
                matches!(self.slots[*rank], RefinedMatchCoverageSlot::Remaining)
            })
            .collect()
    }

    fn variants_for_ranks(&self, ranks: &[usize]) -> Vec<String> {
        ranks
            .iter()
            .map(|rank| {
                record_refined_match_coverage_work(1);
                self.declaration_order
                    .name(*rank)
                    .expect("refined match rank belongs to its ADT")
                    .to_string()
            })
            .collect()
    }

    fn first_missing_case(&self) -> Option<String> {
        self.domain_ranks.iter().find_map(|rank| {
            record_refined_match_coverage_work(1);
            matches!(self.slots[*rank], RefinedMatchCoverageSlot::Remaining)
                .then(|| self.coverage_cases[*rank].clone())
        })
    }

    fn proving_arms(&self) -> Vec<(String, SourceSpan)> {
        self.covered_order
            .iter()
            .map(|rank| {
                record_refined_match_coverage_work(1);
                let RefinedMatchCoverageSlot::Covered(span) = &self.slots[*rank] else {
                    unreachable!("covered order contains only covered refined variants")
                };
                (self.coverage_cases[*rank].clone(), span.clone())
            })
            .collect()
    }
}

impl RefinedMatchDiagnosticFacts {
    fn new(scrutinee: &Expr, scrutinee_type: &Type, adts: &AdtRegistry) -> Self {
        record_refined_match_diagnostic_render();
        let rendered_type = DiagnosticText::from(scrutinee_type.render());
        record_refined_match_diagnostic_retained_bytes(rendered_type.as_str().len());
        let base_name = refined_base_name(scrutinee_type).to_string();
        let declaration_span = adts.declaration_span_for_type(scrutinee_type).cloned();
        let scrutinee_type = JsonValue::shared(JsonValue::text(rendered_type.clone()));
        Self {
            scrutinee_type,
            refinement_source_message: DiagnosticText::parts([
                DiagnosticText::from("This match scrutinee has refined type `"),
                rendered_type,
                DiagnosticText::from("`."),
            ]),
            scrutinee_span: scrutinee.span.clone(),
            adt_declaration_message: DiagnosticText::from(format!(
                "The selected ADT is `{base_name}`."
            )),
            adt_declaration_span: declaration_span,
        }
    }
}

impl<'a> FunctionChecker<'a> {
    pub(super) fn infer_adt_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        constructor: AdtConstructor,
    ) -> Type {
        let diagnostic_count = self.diagnostics.len();
        let expected = expected
            .filter(|expected| adt::type_matches_descriptor(&expected.ty, constructor.descriptor));
        let mut inferred_type_args =
            self.infer_constructor_type_args(expr, args, expected, constructor);
        fill_unknown_constructor_type_args(&mut inferred_type_args, expected, constructor);
        let expected_type_args = expected
            .and_then(|expected| unification::adt_args(&expected.ty, constructor.descriptor));
        let type_args = if self.diagnostics.len() != diagnostic_count {
            expected_type_args.unwrap_or(&inferred_type_args)
        } else {
            &inferred_type_args
        };
        let inferred_base = adt::constructed_type_from_args(constructor, type_args);
        if type_contains_unknown(&inferred_base) && expected.is_none() {
            self.push_ambiguous_constructor_type(
                expr.node_id,
                expr.span.clone(),
                &constructor.variant.name,
                &inferred_base,
            );
        }
        adt::refined_constructed_type_from_args(constructor, type_args)
    }

    fn infer_constructor_type_args(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        constructor: AdtConstructor,
    ) -> Vec<Type> {
        let mut type_args = ConstructorTypeArgInference::new(constructor);
        for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
            let Some(arg) = args.get(index) else {
                continue;
            };
            self.infer_constructor_payload(
                expr,
                arg,
                field,
                index,
                expected,
                constructor,
                &mut type_args,
            );
        }
        for arg in args.iter().skip(constructor.variant.payload_fields.len()) {
            self.infer_expr(arg, None);
        }
        type_args.finish()
    }

    #[allow(clippy::too_many_arguments)]
    fn infer_constructor_payload(
        &mut self,
        call: &Expr,
        arg: &Expr,
        field: &AdtPayloadField,
        index: usize,
        expected: Option<&ExpectedType>,
        constructor: AdtConstructor,
        type_args: &mut ConstructorTypeArgInference,
    ) {
        let mut arg_expected = expected.cloned().unwrap_or_else(|| ExpectedType {
            ty: Type::Unknown,
            source: ExpectedTypeSource::Inferred,
            origin_node_id: call.node_id,
            origin_span: Some(call.span.clone()),
            origin_message: "Constructor payload inferred here.",
        });
        arg_expected.ty = type_args.payload_expected(expected, constructor, index, field);
        let diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(arg, Some(&arg_expected));
        if self.diagnostics.len() != diagnostic_count {
            return;
        }
        let has_context = expected.is_some()
            || (!matches!(field.ty, AdtPayloadType::TypeParameter(_))
                && !type_contains_unknown(&arg_expected.ty));
        let inferred = if has_context {
            inferred_aggregate_member_type_with_expected(actual, &arg_expected.ty)
        } else {
            actual
        };
        let joined = type_args.try_join(field, &inferred, &self.environment.adts);
        self.check_constructor_payload(arg, field, &arg_expected, &inferred, joined, type_args);
        if self.diagnostics.len() == diagnostic_count
            && let Err(conflict) = type_args.commit(
                field,
                constructor,
                index,
                &inferred,
                joined,
                &self.environment.adts,
            )
        {
            self.check_assignable_nested(
                arg,
                &conflict.expected,
                &conflict.actual,
                &arg_expected,
                "call_argument",
            );
        }
    }

    fn check_constructor_payload(
        &mut self,
        arg: &Expr,
        field: &AdtPayloadField,
        expected: &ExpectedType,
        actual: &Type,
        joined: bool,
        type_args: &ConstructorTypeArgInference,
    ) {
        if matches!(
            field.ty,
            AdtPayloadType::SelfType | AdtPayloadType::Concrete(_)
        ) {
            if matches!(field.ty, AdtPayloadType::Concrete(_))
                && unification::is_direct_variant_carrier(&expected.ty, actual)
            {
                return;
            }
            self.check_assignable_nested(arg, &expected.ty, actual, expected, "call_argument");
            return;
        }
        if joined || is_assignable_nested(&expected.ty, actual) {
            return;
        }
        let mismatch_context = ExpectedType {
            ty: type_args.mismatch_expected(field, expected),
            ..expected.clone()
        };
        self.check_assignable_nested(
            arg,
            &mismatch_context.ty,
            actual,
            &mismatch_context,
            "call_argument",
        );
    }

    pub(super) fn infer_list(
        &mut self,
        expr: &Expr,
        items: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let diagnostic_count = self.diagnostics.len();
        if items.is_empty()
            && let Some(expected) = expected
            && expected.ty.vec_part().is_some()
            && type_contains_unknown(&expected.ty)
        {
            self.push_ambiguous_empty_collection_type(
                expr.node_id,
                expr.span.clone(),
                "Vec",
                &expected.ty,
            );
        }
        let expected_item = expected
            .and_then(|expected| expected.ty.vec_part())
            .cloned()
            .unwrap_or(Type::Unknown);
        let contextual_item = expected_item != Type::Unknown;
        let mut item_type = expected_item.clone();
        let mut joined_items = None;
        for item in items {
            self.infer_list_item(
                expr,
                item,
                expected,
                &expected_item,
                contextual_item,
                &mut item_type,
                &mut joined_items,
            );
        }
        if let Some(joined) = joined_items {
            item_type = joined.result_type();
        }
        let actual = Type::vec(item_type);
        if let Some(expected) = expected
            && expected.ty.vec_part().is_some()
            && (self.diagnostics.len() != diagnostic_count
                || !type_contains_variant_refinement(&actual))
        {
            expected.ty.clone()
        } else {
            actual
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn infer_list_item(
        &mut self,
        list: &Expr,
        item: &Expr,
        expected: Option<&ExpectedType>,
        expected_item: &Type,
        contextual_item: bool,
        item_type: &mut Type,
        joined_items: &mut Option<AggregateTypeJoin>,
    ) {
        let inferred_context = joined_items
            .as_ref()
            .map(AggregateTypeJoin::inference_type)
            .unwrap_or_else(|| item_type.clone());
        let item_expected = collection_item_expected(
            if inferred_context == Type::Unknown {
                expected_item.clone()
            } else {
                inferred_context
            },
            expected,
            list.node_id,
            list.span.clone(),
            "Vec element type inferred here.",
        );
        let diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(item, Some(&item_expected));
        if self.diagnostics.len() != diagnostic_count {
            return;
        }
        let aggregate_actual = if contextual_item {
            inferred_aggregate_member_type_with_expected(actual.clone(), expected_item)
        } else {
            actual.clone()
        };
        let joined =
            self.join_list_item(contextual_item, item_type, joined_items, &aggregate_actual);
        if !is_assignable_nested(&item_expected.ty, &aggregate_actual)
            && (contextual_item || !joined)
        {
            self.push_list_item_mismatch(item, &item_expected, &actual, joined_items);
        }
        if *item_type == Type::Unknown {
            *item_type = aggregate_actual;
            *joined_items = (!contextual_item)
                .then(|| AggregateTypeJoin::new(&self.environment.adts, item_type))
                .flatten();
        }
    }

    fn join_list_item(
        &self,
        contextual_item: bool,
        item_type: &Type,
        joined_items: &mut Option<AggregateTypeJoin>,
        actual: &Type,
    ) -> bool {
        if !contextual_item && joined_items.is_none() && item_type != &Type::Unknown {
            *joined_items = AggregateTypeJoin::new(&self.environment.adts, item_type);
        }
        !contextual_item
            && joined_items
                .as_mut()
                .is_some_and(|joined| joined.try_join(actual))
    }

    fn push_list_item_mismatch(
        &mut self,
        item: &Expr,
        item_expected: &ExpectedType,
        actual: &Type,
        joined_items: &Option<AggregateTypeJoin>,
    ) {
        let mismatch_expected = joined_items
            .as_ref()
            .map(AggregateTypeJoin::result_type)
            .unwrap_or_else(|| item_expected.ty.clone());
        let mismatch_context = ExpectedType {
            ty: mismatch_expected,
            ..item_expected.clone()
        };
        self.check_assignable_nested(
            item,
            &mismatch_context.ty,
            actual,
            &mismatch_context,
            "list_element",
        );
    }

    pub(super) fn infer_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let scrutinee_type = self.infer_match_scrutinee(expr, scrutinee, arms);
        let mut refined_coverage = self
            .is_direct_match_binding(scrutinee)
            .then(|| {
                RefinedMatchCoverage::new(
                    scrutinee,
                    &scrutinee_type,
                    &self.environment.adts,
                    self.function.module_name.as_deref(),
                )
            })
            .flatten();
        if arms.is_empty() {
            if let Some(coverage) = &refined_coverage {
                if let Some(missing_case) = coverage.first_missing_case() {
                    self.report_refined_match_non_exhaustive(
                        expr,
                        scrutinee,
                        &scrutinee_type,
                        missing_case,
                        Vec::new(),
                    );
                }
            } else {
                self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
            }
            return expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown);
        }

        let mut result = ControlFlowResultJoin::new(
            expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown),
        );
        let domain = RefinedMatchDomain {
            scrutinee,
            ty: &scrutinee_type,
        };
        for arm in arms {
            self.infer_match_arm(
                expr,
                &domain,
                arm,
                expected,
                &mut result,
                refined_coverage.as_mut(),
            );
        }

        if let Some(coverage) = refined_coverage {
            if let Some(missing_case) = coverage.first_missing_case() {
                let proving_arms = coverage.proving_arms();
                self.report_refined_match_non_exhaustive(
                    expr,
                    scrutinee,
                    &scrutinee_type,
                    missing_case,
                    proving_arms,
                );
            }
        } else {
            self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
        }
        result.materialize()
    }

    fn is_direct_match_binding(&self, scrutinee: &Expr) -> bool {
        let ExprKind::NamePath {
            segments,
            segment_spans,
        } = &scrutinee.kind
        else {
            return false;
        };
        let ([binding_name], [binding_span]) = (segments.as_slice(), segment_spans.as_slice())
        else {
            return false;
        };
        binding_span == &scrutinee.span
            && self
                .bindings
                .iter()
                .any(|binding| binding.name == *binding_name)
    }

    fn report_refined_match_non_exhaustive(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        scrutinee_type: &Type,
        missing_case: String,
        proving_arms: Vec<(String, SourceSpan)>,
    ) {
        self.report_match_non_exhaustive(
            expr,
            scrutinee,
            scrutinee_type,
            missing_case,
            proving_arms,
        );
    }

    pub(super) fn infer_match_scrutinee(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
    ) -> Type {
        let mut prechecked_scrutinee_type = None;
        let pattern_scrutinee_type = match infer_match_scrutinee_type_from_constructor_patterns(
            arms,
            self.function.module_name.as_deref(),
            &self.environment.uses,
            &self.environment.adts,
        ) {
            MatchScrutineePatternInference::Inferred(ty) => Some(ty),
            MatchScrutineePatternInference::Ambiguous(candidates) => {
                let scrutinee_type = self.infer_expr(scrutinee, None);
                if type_contains_unknown(&scrutinee_type) {
                    self.push_ambiguous_match_scrutinee_type(
                        scrutinee.node_id,
                        scrutinee.span.clone(),
                        candidates,
                    );
                } else {
                    prechecked_scrutinee_type = Some(scrutinee_type);
                }
                None
            }
            MatchScrutineePatternInference::Uninferred => None,
        };
        let scrutinee_expected = pattern_scrutinee_type.as_ref().map(|ty| ExpectedType {
            ty: ty.clone(),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Match constructor patterns inferred the scrutinee type here.",
        });
        prechecked_scrutinee_type
            .unwrap_or_else(|| self.infer_expr(scrutinee, scrutinee_expected.as_ref()))
    }

    fn refined_match_arm_plan(
        &self,
        pattern: &Pattern,
        scrutinee_type: &Type,
        coverage: &RefinedMatchCoverage,
    ) -> Option<RefinedMatchArmPlan> {
        let Type::VariantRefinement {
            name,
            identity,
            args: type_args,
            ..
        } = scrutinee_type
        else {
            return None;
        };
        match &pattern.kind {
            PatternKind::Constructor {
                name: constructor_name,
                name_spans,
                ..
            } => {
                if !self.match_pattern_is_valid_for_type(pattern, scrutinee_type) {
                    return None;
                }
                let descriptor = self.environment.adts.descriptor_for_type_prefer_module(
                    scrutinee_type,
                    self.function.module_name.as_deref(),
                )?;
                let constructor = self.environment.adts.constructor_for_descriptor(
                    constructor_name,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                )?;
                let variant = constructor.variant.name.clone();
                let variant_rank = coverage.rank(&variant)?;
                let classification = match coverage.slot(variant_rank) {
                    RefinedMatchCoverageSlot::Outside => {
                        RefinedConstructorClassification::Impossible
                    }
                    RefinedMatchCoverageSlot::Remaining => {
                        RefinedConstructorClassification::Covering
                    }
                    RefinedMatchCoverageSlot::Covered(_) => {
                        RefinedConstructorClassification::Redundant
                    }
                };
                let arm_type =
                    if matches!(classification, RefinedConstructorClassification::Impossible) {
                        scrutinee_type.clone()
                    } else {
                        record_refined_match_refinement_variants(1);
                        Type::resolved_variant_refinement(
                            name,
                            identity,
                            type_args.clone(),
                            vec![variant.clone()],
                        )
                    };
                Some(RefinedMatchArmPlan {
                    arm_type,
                    coverage: RefinedMatchArmCoverage::Constructor {
                        variant,
                        variant_rank,
                        variant_span: name_spans.last()?.clone(),
                        classification,
                    },
                })
            }
            PatternKind::Wildcard | PatternKind::Binding(_) => {
                let classification = if coverage.remaining_count == 0 {
                    RefinedCatchAllClassification::Redundant
                } else {
                    RefinedCatchAllClassification::Covering
                };
                let remaining_ranks = if coverage.remaining_count == 0 {
                    Vec::new()
                } else {
                    coverage.remaining_ranks()
                };
                let arm_type = if remaining_ranks.is_empty() {
                    scrutinee_type.clone()
                } else {
                    record_refined_match_refinement_variants(remaining_ranks.len());
                    Type::resolved_variant_refinement(
                        name,
                        identity,
                        type_args.clone(),
                        coverage.variants_for_ranks(&remaining_ranks),
                    )
                };
                Some(RefinedMatchArmPlan {
                    arm_type,
                    coverage: RefinedMatchArmCoverage::CatchAll {
                        classification,
                        remaining_ranks,
                    },
                })
            }
            _ => None,
        }
    }

    fn commit_refined_match_arm(
        &mut self,
        pattern: &Pattern,
        coverage: &mut RefinedMatchCoverage,
        plan: &RefinedMatchArmPlan,
    ) {
        match &plan.coverage {
            RefinedMatchArmCoverage::Constructor {
                variant_rank,
                classification: RefinedConstructorClassification::Covering,
                ..
            } => {
                record_refined_match_coverage_work(1);
                coverage.slots[*variant_rank] =
                    RefinedMatchCoverageSlot::Covered(pattern.span.clone());
                coverage.covered_order.push(*variant_rank);
                coverage.remaining_count -= 1;
            }
            RefinedMatchArmCoverage::Constructor {
                variant,
                variant_span,
                classification: RefinedConstructorClassification::Impossible,
                ..
            } => self.push_match_impossible_variant(coverage, variant, variant_span),
            RefinedMatchArmCoverage::Constructor {
                variant,
                variant_rank,
                classification: RefinedConstructorClassification::Redundant,
                ..
            } => {
                let (reason, related_span, related_message) = if let Some(span) =
                    &coverage.preceding_catch_all
                {
                    (
                        "preceding_catch_all",
                        span,
                        "This preceding valid catch-all arm takes precedence.".to_string(),
                    )
                } else {
                    record_refined_match_coverage_work(1);
                    let RefinedMatchCoverageSlot::Covered(span) = &coverage.slots[*variant_rank]
                    else {
                        unreachable!("redundant variant must have a covering arm")
                    };
                    (
                        "duplicate_variant",
                        span,
                        format!("This preceding arm first covered `{variant}`."),
                    )
                };
                self.push_match_redundant_arm(
                    pattern,
                    coverage,
                    Some(variant),
                    reason,
                    [(related_span, related_message)],
                );
            }
            RefinedMatchArmCoverage::CatchAll {
                classification: RefinedCatchAllClassification::Covering,
                remaining_ranks,
            } => {
                for rank in remaining_ranks {
                    record_refined_match_coverage_work(1);
                    coverage.slots[*rank] = RefinedMatchCoverageSlot::Covered(pattern.span.clone());
                    coverage.covered_order.push(*rank);
                }
                coverage.remaining_count = 0;
                coverage.preceding_catch_all = Some(pattern.span.clone());
            }
            RefinedMatchArmCoverage::CatchAll {
                classification: RefinedCatchAllClassification::Redundant,
                ..
            } => {
                if let Some(span) = &coverage.preceding_catch_all {
                    self.push_match_redundant_arm(
                        pattern,
                        coverage,
                        None,
                        "preceding_catch_all",
                        [(
                            span,
                            "This preceding valid catch-all arm takes precedence.".to_string(),
                        )],
                    );
                } else {
                    let related = coverage
                        .domain_ranks
                        .iter()
                        .filter_map(|rank| {
                            record_refined_match_coverage_work(1);
                            let RefinedMatchCoverageSlot::Covered(span) = &coverage.slots[*rank]
                            else {
                                return None;
                            };
                            let variant = coverage
                                .declaration_order
                                .name(*rank)
                                .expect("refined match rank belongs to its ADT");
                            Some((span, format!("This preceding arm covered `{variant}`.")))
                        })
                        .collect::<Vec<_>>();
                    self.push_match_redundant_arm(
                        pattern,
                        coverage,
                        None,
                        "complete_prior_coverage",
                        related,
                    );
                }
                if coverage.preceding_catch_all.is_none() {
                    coverage.preceding_catch_all = Some(pattern.span.clone());
                }
            }
        }
    }

    fn push_match_impossible_variant(
        &mut self,
        coverage: &RefinedMatchCoverage,
        variant: &str,
        variant_span: &SourceSpan,
    ) {
        record_refined_match_diagnostic_retained_bytes(variant.len());
        let mut diagnostic = Diagnostic::new(
            "type.match_impossible_variant",
            Severity::Error,
            DiagnosticKind::Type,
            format!("variant `{variant}` is excluded by the refined match domain"),
            Some(variant_span.clone()),
            JsonValue::object([
                (
                    "scrutinee_type",
                    coverage.diagnostic_facts.scrutinee_type.clone(),
                ),
                ("arm_variant", JsonValue::string(variant)),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("refinement_source")),
            (
                "message",
                JsonValue::text(coverage.diagnostic_facts.refinement_source_message.clone()),
            ),
            ("span", span_json(&coverage.diagnostic_facts.scrutinee_span)),
        ]));
        let mut adt_declaration = vec![
            ("kind", JsonValue::string("adt_declaration")),
            (
                "message",
                JsonValue::text(coverage.diagnostic_facts.adt_declaration_message.clone()),
            ),
        ];
        if let Some(span) = &coverage.diagnostic_facts.adt_declaration_span {
            adt_declaration.push(("span", span_json(span)));
        }
        diagnostic.related.push(JsonValue::object(adt_declaration));
        self.diagnostics.push(diagnostic);
    }

    fn push_match_redundant_arm<'b>(
        &mut self,
        pattern: &Pattern,
        coverage: &RefinedMatchCoverage,
        variant: Option<&str>,
        reason: &'static str,
        related: impl IntoIterator<Item = (&'b SourceSpan, String)>,
    ) {
        let arm_pattern = render_match_pattern(pattern);
        record_refined_match_diagnostic_retained_bytes(
            arm_pattern.len() + variant.map_or(0, str::len),
        );
        let mut diagnostic = Diagnostic::new(
            "type.match_redundant_arm",
            Severity::Error,
            DiagnosticKind::Type,
            "match arm has no remaining variant to cover",
            Some(pattern.span.clone()),
            JsonValue::object([
                (
                    "scrutinee_type",
                    coverage.diagnostic_facts.scrutinee_type.clone(),
                ),
                ("arm_pattern", JsonValue::string(arm_pattern)),
                (
                    "arm_variant",
                    variant.map_or(JsonValue::Null, JsonValue::string),
                ),
                ("reason", JsonValue::string(reason)),
            ]),
        );
        for (span, message) in related {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("preceding_arm")),
                ("message", JsonValue::string(message)),
                ("span", span_json(span)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }

    fn infer_match_arm(
        &mut self,
        match_expr: &Expr,
        domain: &RefinedMatchDomain<'_>,
        arm: &MatchArm,
        expected: Option<&ExpectedType>,
        result: &mut ControlFlowResultJoin,
        refined_coverage: Option<&mut RefinedMatchCoverage>,
    ) {
        let saved_bindings = self.bindings.len();
        let saved_invalid_binding_recoveries = self.invalid_binding_recoveries.len();
        self.local_name_scopes.push(Vec::new());

        let arm_plan = refined_coverage
            .as_deref()
            .and_then(|coverage| self.refined_match_arm_plan(&arm.pattern, domain.ty, coverage));
        let pattern_type = arm_plan.as_ref().map_or(domain.ty, |plan| &plan.arm_type);
        let pattern_bindings_admitted =
            self.declare_match_pattern_bindings(&arm.pattern, pattern_type);
        if pattern_bindings_admitted
            && let (Some(coverage), Some(plan)) = (refined_coverage, arm_plan.as_ref())
        {
            self.commit_refined_match_arm(&arm.pattern, coverage, plan);
        }
        let direct_binding_refinement = self.direct_match_binding_refinement(
            domain.scrutinee,
            &arm.pattern,
            domain.ty,
            pattern_bindings_admitted
                .then_some(arm_plan.as_ref())
                .flatten()
                .map(|plan| &plan.arm_type),
        );
        if pattern_bindings_admitted && let Some(binding) = direct_binding_refinement {
            self.bindings.push(binding);
        }
        self.infer_match_arm_result(match_expr, arm, expected, result);

        self.bindings.truncate(saved_bindings);
        self.invalid_binding_recoveries
            .truncate(saved_invalid_binding_recoveries);
        for (name, previous) in self.local_name_scopes.pop().expect("match arm name frame") {
            if let Some(previous) = previous {
                self.local_names.insert(name, previous);
            } else {
                self.local_names.remove(&name);
            }
        }
    }

    fn direct_match_binding_refinement(
        &self,
        scrutinee: &Expr,
        pattern: &Pattern,
        scrutinee_type: &Type,
        planned_refinement: Option<&Type>,
    ) -> Option<Binding> {
        let ExprKind::NamePath {
            segments,
            segment_spans,
        } = &scrutinee.kind
        else {
            return None;
        };
        let [binding_name] = segments.as_slice() else {
            return None;
        };
        let [binding_span] = segment_spans.as_slice() else {
            return None;
        };
        if binding_span != &scrutinee.span {
            return None;
        }
        self.bindings
            .iter()
            .rfind(|binding| binding.name == *binding_name)?;

        if let Some(refinement) = planned_refinement {
            return Some(Binding::new(binding_name.clone(), refinement.clone()));
        }

        let Type::Named {
            name: type_name,
            identity,
            args: type_args,
        } = scrutinee_type
        else {
            return None;
        };

        let PatternKind::Constructor { name, args, .. } = &pattern.kind else {
            return None;
        };
        if invalid_qualified_constructor_pattern(name) {
            return None;
        }
        let descriptor = self.environment.adts.descriptor_for_type_prefer_module(
            scrutinee_type,
            self.function.module_name.as_deref(),
        )?;
        let constructor = self.environment.adts.constructor_for_descriptor(
            name,
            descriptor,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        )?;
        if args.len() != constructor.variant.payload_fields.len() {
            return None;
        }
        if !args.iter().enumerate().all(|(index, pattern)| {
            adt::payload_type(scrutinee_type, constructor, index)
                .is_some_and(|ty| self.match_pattern_is_valid_for_type(pattern, &ty))
        }) {
            return None;
        }
        Some(Binding::new(
            binding_name.clone(),
            Type::resolved_variant_refinement(
                type_name,
                identity,
                type_args.clone(),
                vec![constructor.variant.name.clone()],
            ),
        ))
    }

    fn match_pattern_is_valid_for_type(&self, pattern: &Pattern, expected: &Type) -> bool {
        match &pattern.kind {
            PatternKind::Constructor { name, args, .. } => {
                if invalid_qualified_constructor_pattern(name) {
                    return false;
                }
                let Some(descriptor) = self.environment.adts.descriptor_for_type_prefer_module(
                    expected,
                    self.function.module_name.as_deref(),
                ) else {
                    return false;
                };
                let Some(constructor) = self.environment.adts.constructor_for_descriptor(
                    name,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                ) else {
                    return false;
                };
                args.len() == constructor.variant.payload_fields.len()
                    && args.iter().enumerate().all(|(index, pattern)| {
                        adt::payload_type(expected, constructor, index)
                            .is_some_and(|ty| self.match_pattern_is_valid_for_type(pattern, &ty))
                    })
            }
            PatternKind::Record(fields) => {
                let Type::Record(expected_fields) = expected else {
                    return false;
                };
                let mut expected_by_name = HashMap::with_capacity(expected_fields.len());
                for (name, ty) in expected_fields {
                    expected_by_name.entry(name.as_str()).or_insert(ty);
                }
                let mut seen_fields = BTreeSet::new();
                fields.iter().all(|field| {
                    seen_fields.insert(field.name.as_str())
                        && expected_by_name.get(field.name.as_str()).is_some_and(|ty| {
                            self.match_pattern_is_valid_for_type(&field.pattern, ty)
                        })
                })
            }
            PatternKind::Wildcard | PatternKind::Binding(_) => true,
            PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {
                literal_or_unit_pattern_is_compatible(pattern, expected).unwrap_or(false)
            }
        }
    }

    pub(super) fn declare_match_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
    ) -> bool {
        let mut all_admitted = true;
        for binding in self.pattern_bindings(pattern, scrutinee_type) {
            if !valid_value_binding_name(&binding.name) {
                self.push_invalid_binding_recovery(binding);
                all_admitted = false;
                continue;
            }
            if !self.declare_local_name(
                &binding.name,
                binding.node_id.display("pattern"),
                binding.span,
                "pattern binding",
                false,
            ) {
                all_admitted = false;
                continue;
            }
            self.bindings.push(Binding::new(binding.name, binding.ty));
        }
        all_admitted
    }

    fn infer_match_arm_result(
        &mut self,
        match_expr: &Expr,
        arm: &MatchArm,
        expected: Option<&ExpectedType>,
        result: &mut ControlFlowResultJoin,
    ) {
        let recovery_expected = if expected.is_none()
            && result.joined.is_none()
            && result.recovery_type != Type::Unknown
        {
            Some(ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: match_expr.node_id,
                origin_span: Some(match_expr.span.clone()),
                origin_message: "Match result type inferred here.",
            })
        } else {
            None
        };
        let actual = self.infer_expr(&arm.expr, expected.or(recovery_expected.as_ref()));
        if let Some(expected) = expected {
            self.check_assignable(&arm.expr, &expected.ty, &actual, expected, "match_arm");
            return;
        }
        if !contribute_control_flow_result(&self.environment.adts, result, &actual) {
            let mismatch_expected = ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: match_expr.node_id,
                origin_span: Some(match_expr.span.clone()),
                origin_message: "Match result type inferred here.",
            };
            self.check_assignable(
                &arm.expr,
                &mismatch_expected.ty,
                &actual,
                &mismatch_expected,
                "match_arm",
            );
        }
    }

    pub(super) fn infer_if(
        &mut self,
        expr: &Expr,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        self.check_if_condition(expr, condition);

        let mut result = ControlFlowResultJoin::new(
            expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown),
        );
        self.infer_if_branch(expr, then_branch, expected, &mut result);
        for branch in else_if_branches {
            self.check_if_condition(expr, &branch.condition);
            self.infer_if_branch(expr, &branch.expr, expected, &mut result);
        }
        self.infer_if_branch(expr, else_branch, expected, &mut result);
        result.materialize()
    }
}

struct RefinedMatchDomain<'a> {
    scrutinee: &'a Expr,
    ty: &'a Type,
}

fn refined_base_name(ty: &Type) -> &str {
    match ty {
        Type::VariantRefinement { name, .. } => name,
        _ => "unknown",
    }
}

fn render_match_pattern(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Wildcard => "_".to_string(),
        PatternKind::Binding(name) => name.clone(),
        PatternKind::StringLiteral(value) => format!("{value:?}"),
        PatternKind::IntLiteral(value) | PatternKind::FloatLiteral(value) => value.clone(),
        PatternKind::BoolLiteral(value) => value.to_string(),
        PatternKind::Unit => "()".to_string(),
        PatternKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| format!("{}: {}", field.name, render_match_pattern(&field.pattern)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        PatternKind::Constructor { name, args, .. } => {
            let name = name.join("::");
            if args.is_empty() {
                name
            } else {
                let args = args
                    .iter()
                    .map(render_match_pattern)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}({args})")
            }
        }
    }
}

struct ConstructorTypeArgInference {
    inferred: Vec<Type>,
    joined: Vec<Option<AggregateTypeJoin>>,
    invariant: Vec<bool>,
}

impl ConstructorTypeArgInference {
    fn new(constructor: AdtConstructor<'_>) -> Self {
        let parameter_count = constructor.descriptor.type_parameters.len();
        crate::aggregate_type_join::record_work(parameter_count);
        Self {
            inferred: vec![Type::Unknown; parameter_count],
            joined: (0..parameter_count).map(|_| None).collect(),
            invariant: vec![false; parameter_count],
        }
    }

    fn payload_expected(
        &self,
        expected: Option<&ExpectedType>,
        constructor: AdtConstructor<'_>,
        index: usize,
        field: &AdtPayloadField,
    ) -> Type {
        expected
            .and_then(|expected| {
                let expected_args = unification::adt_args(&expected.ty, constructor.descriptor)?;
                adt::payload_type_with_resolved_args(constructor, index, |type_index| {
                    crate::aggregate_type_join::record_work(1);
                    expected_args[type_index].clone()
                })
            })
            .or_else(|| self.direct_payload_context(field))
            .or_else(|| {
                adt::payload_type_with_resolved_args(constructor, index, |type_index| {
                    crate::aggregate_type_join::record_work(1);
                    self.joined[type_index]
                        .as_ref()
                        .map(AggregateTypeJoin::result_type)
                        .unwrap_or_else(|| self.inferred[type_index].clone())
                })
            })
            .unwrap_or(Type::Unknown)
    }

    fn direct_payload_context(&self, field: &AdtPayloadField) -> Option<Type> {
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return None;
        };
        (!self.invariant[type_index])
            .then(|| self.joined[type_index].as_ref())
            .flatten()
            .map(AggregateTypeJoin::inference_type)
    }

    fn try_join(&mut self, field: &AdtPayloadField, actual: &Type, adts: &AdtRegistry) -> bool {
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return false;
        };
        if self.invariant[type_index] {
            return false;
        }
        if self.joined[type_index].is_none() && self.inferred[type_index] != Type::Unknown {
            self.joined[type_index] = AggregateTypeJoin::new(adts, &self.inferred[type_index]);
        }
        self.joined[type_index]
            .as_mut()
            .is_some_and(|joined| joined.try_join(actual))
    }

    fn mismatch_expected(&self, field: &AdtPayloadField, fallback: &ExpectedType) -> Type {
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return fallback.ty.clone();
        };
        self.joined[type_index]
            .as_ref()
            .map(AggregateTypeJoin::result_type)
            .unwrap_or_else(|| fallback.ty.clone())
    }

    fn commit(
        &mut self,
        field: &AdtPayloadField,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &Type,
        joined: bool,
        adts: &AdtRegistry,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        if let AdtPayloadType::TypeParameter(type_index) = field.ty
            && joined
        {
            self.inferred[type_index] = self.joined[type_index]
                .as_ref()
                .expect("joined aggregate type argument")
                .inference_type();
            return Ok(());
        }
        if !matches!(field.ty, AdtPayloadType::TypeParameter(_)) {
            return crate::aggregate_type_join::merge_invariant_payload_type_args(
                &mut self.inferred,
                &mut self.joined,
                &mut self.invariant,
                constructor,
                index,
                actual,
            );
        }
        crate::aggregate_type_join::record_work(1);
        adt::merge_type_args_from_payload(&mut self.inferred, constructor, index, actual);
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return Ok(());
        };
        if self.joined[type_index].is_none() {
            self.joined[type_index] = AggregateTypeJoin::new(adts, &self.inferred[type_index]);
            if let Some(joined) = &self.joined[type_index] {
                self.inferred[type_index] = joined.inference_type();
            }
        }
        Ok(())
    }

    fn finish(mut self) -> Vec<Type> {
        crate::aggregate_type_join::record_work(self.inferred.len());
        for (inferred, joined) in self.inferred.iter_mut().zip(self.joined) {
            if let Some(joined) = joined {
                *inferred = joined.result_type();
            }
        }
        self.inferred
    }
}

fn fill_unknown_constructor_type_args(
    inferred_type_args: &mut [Type],
    expected: Option<&ExpectedType>,
    constructor: AdtConstructor<'_>,
) {
    let Some(expected_args) =
        expected.and_then(|expected| unification::adt_args(&expected.ty, constructor.descriptor))
    else {
        return;
    };
    for (inferred, expected) in inferred_type_args.iter_mut().zip(expected_args) {
        if *inferred == Type::Unknown {
            *inferred = expected.clone();
        }
    }
}

#[cfg(test)]
mod refined_match_coverage_work {
    use std::cell::Cell;

    thread_local! {
        static WORK: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn record(units: usize) {
        WORK.set(WORK.get() + units);
    }

    pub(super) fn reset() {
        WORK.set(0);
    }

    pub(super) fn take() -> usize {
        WORK.replace(0)
    }
}

#[cfg(test)]
mod refined_match_diagnostic_work {
    use std::cell::Cell;

    use super::RefinedMatchDiagnosticWork;

    thread_local! {
        static RENDERS: Cell<usize> = const { Cell::new(0) };
        static RETAINED_BYTES: Cell<usize> = const { Cell::new(0) };
        static REFINEMENT_VARIANTS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn record_render() {
        RENDERS.set(RENDERS.get() + 1);
    }

    pub(super) fn record_retained_bytes(bytes: usize) {
        RETAINED_BYTES.set(RETAINED_BYTES.get() + bytes);
    }

    pub(super) fn record_refinement_variants(variants: usize) {
        REFINEMENT_VARIANTS.set(REFINEMENT_VARIANTS.get() + variants);
    }

    pub(super) fn reset() {
        RENDERS.set(0);
        RETAINED_BYTES.set(0);
        REFINEMENT_VARIANTS.set(0);
    }

    pub(super) fn take() -> RefinedMatchDiagnosticWork {
        RefinedMatchDiagnosticWork {
            renders: RENDERS.replace(0),
            retained_bytes: RETAINED_BYTES.replace(0),
            refinement_variants: REFINEMENT_VARIANTS.replace(0),
        }
    }
}
