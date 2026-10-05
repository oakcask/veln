use super::*;

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
        let mut arg_expected = expected.cloned().unwrap_or_else(|| ExpectedType {
            ty: Type::Unknown,
            source: ExpectedTypeSource::Inferred,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Constructor payload inferred here.",
        });
        let mut inferred_type_args =
            vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
        let mut joined_type_args = (0..inferred_type_args.len())
            .map(|_| None)
            .collect::<Vec<Option<AggregateTypeJoin>>>();
        for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
            arg_expected.ty = expected
                .and_then(|expected| adt::payload_type(&expected.ty, constructor, index))
                .or_else(|| adt::payload_type_with_args(constructor, &inferred_type_args, index))
                .unwrap_or(Type::Unknown);
            let Some(arg) = args.get(index) else {
                continue;
            };
            let actual_arg = self.infer_expr(arg, Some(&arg_expected));
            let has_context =
                expected.is_some() || !matches!(field.ty, AdtPayloadType::TypeParameter(_));
            let inferred_arg = if has_context {
                inferred_aggregate_member_type_with_expected(actual_arg.clone(), &arg_expected.ty)
            } else {
                actual_arg.clone()
            };
            let joined_arg = match &field.ty {
                AdtPayloadType::TypeParameter(type_index) => {
                    let current = &inferred_type_args[*type_index];
                    if joined_type_args[*type_index].is_none() && current != &Type::Unknown {
                        joined_type_args[*type_index] =
                            AggregateTypeJoin::new(&self.environment.adts, current);
                    }
                    joined_type_args[*type_index]
                        .as_mut()
                        .is_some_and(|joined| joined.try_join(&inferred_arg))
                }
                AdtPayloadType::SelfType | AdtPayloadType::Concrete(_) => false,
            };
            match &field.ty {
                AdtPayloadType::SelfType => self.check_assignable_nested(
                    arg,
                    &arg_expected.ty,
                    &inferred_arg,
                    &arg_expected,
                    "call_argument",
                ),
                AdtPayloadType::Concrete(_) => self.check_assignable_nested(
                    arg,
                    &arg_expected.ty,
                    &inferred_arg,
                    &arg_expected,
                    "call_argument",
                ),
                AdtPayloadType::TypeParameter(_) if joined_arg => {}
                AdtPayloadType::TypeParameter(_)
                    if !is_assignable_nested(&arg_expected.ty, &inferred_arg) =>
                {
                    let mismatch_expected = match field.ty {
                        AdtPayloadType::TypeParameter(type_index) => joined_type_args[type_index]
                            .as_ref()
                            .map(AggregateTypeJoin::result_type)
                            .unwrap_or_else(|| arg_expected.ty.clone()),
                        AdtPayloadType::SelfType | AdtPayloadType::Concrete(_) => {
                            arg_expected.ty.clone()
                        }
                    };
                    let mismatch_context = ExpectedType {
                        ty: mismatch_expected,
                        ..arg_expected.clone()
                    };
                    self.check_assignable_nested(
                        arg,
                        &mismatch_context.ty,
                        &inferred_arg,
                        &mismatch_context,
                        "call_argument",
                    );
                }
                AdtPayloadType::TypeParameter(_) => {}
            }
            if let AdtPayloadType::TypeParameter(type_index) = &field.ty
                && joined_arg
            {
                inferred_type_args[*type_index] = joined_type_args[*type_index]
                    .as_ref()
                    .expect("joined aggregate type argument")
                    .inference_type();
            } else {
                adt::merge_type_args_from_payload(
                    &mut inferred_type_args,
                    constructor,
                    index,
                    &inferred_arg,
                );
                if let AdtPayloadType::TypeParameter(type_index) = &field.ty
                    && joined_type_args[*type_index].is_none()
                {
                    joined_type_args[*type_index] = AggregateTypeJoin::new(
                        &self.environment.adts,
                        &inferred_type_args[*type_index],
                    );
                    if let Some(joined) = &joined_type_args[*type_index] {
                        inferred_type_args[*type_index] = joined.inference_type();
                    }
                }
            }
        }
        for (inferred, joined) in inferred_type_args.iter_mut().zip(joined_type_args) {
            if let Some(joined) = joined {
                *inferred = joined.result_type();
            }
        }
        for arg in args.iter().skip(constructor.variant.payload_fields.len()) {
            self.infer_expr(arg, None);
        }
        inferred_type_args
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
                expr.node_id,
                expr.span.clone(),
                "Vec element type inferred here.",
            );
            let actual = self.infer_expr(item, Some(&item_expected));
            let aggregate_actual = if contextual_item {
                inferred_aggregate_member_type_with_expected(actual.clone(), &expected_item)
            } else {
                actual.clone()
            };
            if !contextual_item && joined_items.is_none() && item_type != Type::Unknown {
                joined_items = AggregateTypeJoin::new(&self.environment.adts, &item_type);
            }
            let joined = !contextual_item
                && joined_items
                    .as_mut()
                    .is_some_and(|joined| joined.try_join(&aggregate_actual));
            if !is_assignable_nested(&item_expected.ty, &aggregate_actual)
                && (contextual_item || !joined)
            {
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
                    &actual,
                    &mismatch_context,
                    "list_element",
                );
            }
            if item_type == Type::Unknown {
                item_type = aggregate_actual;
                joined_items = (!contextual_item)
                    .then(|| AggregateTypeJoin::new(&self.environment.adts, &item_type))
                    .flatten();
            }
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

        let mut result_type = expected
            .map(|expected| expected.ty.clone())
            .unwrap_or(Type::Unknown);
        for arm in arms {
            self.infer_match_arm(expr, arm, &scrutinee_type, expected, &mut result_type);
        }

        self.check_match_exhaustiveness(expr, scrutinee, &scrutinee_type, arms);
        result_type
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

    pub(super) fn infer_match_arm(
        &mut self,
        match_expr: &Expr,
        arm: &MatchArm,
        scrutinee_type: &Type,
        expected: Option<&ExpectedType>,
        result_type: &mut Type,
    ) {
        let saved_bindings = self.bindings.len();
        let saved_invalid_binding_recoveries = self.invalid_binding_recoveries.len();
        self.local_name_scopes.push(Vec::new());

        self.declare_match_pattern_bindings(&arm.pattern, scrutinee_type);
        self.infer_match_arm_result(match_expr, arm, expected, result_type);

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

    pub(super) fn infer_match_arm_result(
        &mut self,
        match_expr: &Expr,
        arm: &MatchArm,
        expected: Option<&ExpectedType>,
        result_type: &mut Type,
    ) {
        let arm_expected = if let Some(expected) = expected {
            Some(expected.clone())
        } else if *result_type != Type::Unknown {
            Some(ExpectedType {
                ty: result_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: match_expr.node_id,
                origin_span: Some(match_expr.span.clone()),
                origin_message: "Match result type inferred here.",
            })
        } else {
            None
        };
        let actual = self.infer_expr(&arm.expr, arm_expected.as_ref());
        if let Some(expected) = &arm_expected {
            self.check_assignable(&arm.expr, &expected.ty, &actual, expected, "match_arm");
        }
        if *result_type == Type::Unknown {
            *result_type = inferred_control_flow_result_type(actual);
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

        let mut result_type = expected
            .map(|expected| expected.ty.clone())
            .unwrap_or(Type::Unknown);
        self.infer_if_branch(expr, then_branch, expected, &mut result_type);
        for branch in else_if_branches {
            self.check_if_condition(expr, &branch.condition);
            self.infer_if_branch(expr, &branch.expr, expected, &mut result_type);
        }
        self.infer_if_branch(expr, else_branch, expected, &mut result_type);
        result_type
    }

    pub(super) fn check_if_condition(&mut self, if_expr: &Expr, condition: &Expr) {
        let expected = ExpectedType {
            ty: Type::bool(),
            source: ExpectedTypeSource::Inferred,
            origin_node_id: if_expr.node_id,
            origin_span: Some(if_expr.span.clone()),
            origin_message: "If condition expected `Bool` here.",
        };
        let actual = self.infer_expr(condition, Some(&expected));
        self.check_assignable(condition, &expected.ty, &actual, &expected, "if_condition");
    }

    pub(super) fn infer_if_branch(
        &mut self,
        if_expr: &Expr,
        branch_expr: &Expr,
        expected: Option<&ExpectedType>,
        result_type: &mut Type,
    ) {
        let branch_expected = if let Some(expected) = expected {
            Some(expected.clone())
        } else if *result_type != Type::Unknown {
            Some(ExpectedType {
                ty: result_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            })
        } else {
            None
        };
        let actual = self.infer_expr(branch_expr, branch_expected.as_ref());
        if let Some(expected) = &branch_expected {
            self.check_assignable(branch_expr, &expected.ty, &actual, expected, "if_branch");
        }
        if *result_type == Type::Unknown {
            *result_type = inferred_control_flow_result_type(actual);
        }
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
