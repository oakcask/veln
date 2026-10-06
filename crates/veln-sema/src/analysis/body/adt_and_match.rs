use super::control_flow_results::{ControlFlowResultJoin, contribute_control_flow_result};
use super::*;
use crate::adt::descriptors::AdtPayloadField;
use crate::adt::registry::AdtRegistry;

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
        if arms.is_empty() {
            self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
            return expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown);
        }

        let mut result = ControlFlowResultJoin::new(
            expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown),
        );
        for arm in arms {
            self.infer_match_arm(expr, scrutinee, arm, &scrutinee_type, expected, &mut result);
        }

        self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
        result.materialize()
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

    fn infer_match_arm(
        &mut self,
        match_expr: &Expr,
        scrutinee: &Expr,
        arm: &MatchArm,
        scrutinee_type: &Type,
        expected: Option<&ExpectedType>,
        result: &mut ControlFlowResultJoin,
    ) {
        let saved_bindings = self.bindings.len();
        let saved_invalid_binding_recoveries = self.invalid_binding_recoveries.len();
        self.local_name_scopes.push(Vec::new());

        if let Some(binding) =
            self.direct_match_binding_refinement(scrutinee, &arm.pattern, scrutinee_type)
        {
            self.bindings.push(binding);
        }
        self.declare_match_pattern_bindings(&arm.pattern, scrutinee_type);
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
        let (type_name, identity, type_args) = match scrutinee_type {
            Type::Named {
                name,
                identity,
                args,
            }
            | Type::VariantRefinement {
                name,
                identity,
                args,
                ..
            } => (name, identity, args),
            _ => return None,
        };
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
            PatternKind::Record(fields) => fields.iter().enumerate().all(|(index, field)| {
                !fields[..index]
                    .iter()
                    .any(|previous| previous.name == field.name)
                    && expected
                        .record_field(&field.name)
                        .is_some_and(|ty| self.match_pattern_is_valid_for_type(&field.pattern, ty))
            }),
            PatternKind::Wildcard
            | PatternKind::Binding(_)
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => true,
        }
    }

    pub(super) fn declare_match_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &Type,
    ) {
        for binding in self.pattern_bindings(pattern, scrutinee_type) {
            if !valid_value_binding_name(&binding.name) {
                self.push_invalid_binding_recovery(binding);
                continue;
            }
            if !self.declare_local_name(
                &binding.name,
                binding.node_id.display("pattern"),
                binding.span,
                "pattern binding",
                false,
            ) {
                continue;
            }
            self.bindings.push(Binding::new(binding.name, binding.ty));
        }
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
            return self.commit_invariant_payload(constructor, index, actual);
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

    fn commit_invariant_payload(
        &mut self,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &Type,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        let mut contributions = Vec::new();
        adt::visit_type_arg_contributions_from_payload(
            constructor,
            index,
            actual,
            |type_index, contribution| {
                crate::aggregate_type_join::record_work(1);
                contributions.push((type_index, contribution.clone()));
            },
        );
        let constraints = unification::merge_type_parameter_contributions_transactionally(
            &contributions,
            |type_index| {
                self.inferred.get(type_index).map(|inferred| {
                    self.joined[type_index]
                        .as_ref()
                        .map(AggregateTypeJoin::result_type)
                        .unwrap_or_else(|| inferred.clone())
                })
            },
        )?;
        for (type_index, constraint) in constraints {
            crate::aggregate_type_join::record_work(1);
            self.inferred[type_index] = constraint;
            self.joined[type_index] = None;
            self.invariant[type_index] = true;
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
