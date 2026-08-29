use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn infer_adt_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        constructor: adt::AdtConstructor,
    ) -> Type {
        let mut actual_args = Vec::new();
        let expected_constructor_type = expected
            .and_then(|expected| adt::adt_args(&expected.ty, constructor.descriptor))
            .is_some();
        let mut inferred_type_args =
            vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
        for (index, _) in constructor.variant.payload_fields.iter().enumerate() {
            let expected_payload = expected
                .filter(|_| expected_constructor_type)
                .and_then(|expected| adt::payload_type(&expected.ty, constructor, index))
                .or_else(|| adt::payload_type_with_args(constructor, &inferred_type_args, index))
                .unwrap_or(Type::Unknown);
            let arg_expected = ExpectedType {
                ty: expected_payload,
                source: expected
                    .filter(|_| expected_constructor_type)
                    .map_or(ExpectedTypeSource::Inferred, |expected| expected.source),
                origin_node_id: expected
                    .filter(|_| expected_constructor_type)
                    .map_or(expr.node_id, |expected| expected.origin_node_id),
                origin_span: expected.filter(|_| expected_constructor_type).map_or_else(
                    || Some(expr.span.clone()),
                    |expected| expected.origin_span.clone(),
                ),
                origin_message: expected
                    .filter(|_| expected_constructor_type)
                    .map_or("Constructor payload inferred here.", |expected| {
                        expected.origin_message
                    }),
            };
            let Some(arg) = args.get(index) else {
                continue;
            };
            let actual_arg = self.infer_expr(arg, Some(&arg_expected));
            self.check_assignable(
                arg,
                &arg_expected.ty,
                &actual_arg,
                &arg_expected,
                "call_argument",
            );
            if !expected_constructor_type {
                adt::merge_type_args_from_payload(
                    &mut inferred_type_args,
                    constructor,
                    index,
                    &actual_arg,
                );
            }
            actual_args.push(actual_arg);
        }
        for arg in args.iter().skip(constructor.variant.payload_fields.len()) {
            self.infer_expr(arg, None);
        }

        if expected_constructor_type {
            return expected
                .map(|expected| expected.ty.clone())
                .unwrap_or(Type::Unknown);
        }
        let inferred = adt::constructed_type_from_args(constructor, &inferred_type_args);
        if type_contains_unknown(&inferred) {
            self.push_ambiguous_constructor_type(
                expr.node_id,
                expr.span.clone(),
                &constructor.variant.name,
                &inferred,
            );
            return adt::constructed_type(constructor, &actual_args);
        }
        inferred
    }

    pub(super) fn infer_list(
        &mut self,
        expr: &Expr,
        items: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
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
        let mut item_type = expected_item.clone();
        for item in items {
            let item_expected = collection_item_expected(
                item_type.clone(),
                expected,
                expr.node_id,
                expr.span.clone(),
                "Vec element type inferred here.",
            );
            let actual = self.infer_expr(item, Some(&item_expected));
            self.check_assignable(
                item,
                &item_expected.ty,
                &actual,
                &item_expected,
                "list_element",
            );
            if item_type == Type::Unknown {
                item_type = actual;
            }
        }
        Type::vec(item_type)
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
        let saved_names = self.local_names.clone();

        self.declare_match_pattern_bindings(&arm.pattern, scrutinee_type);
        self.infer_match_arm_result(match_expr, arm, expected, result_type);

        self.bindings.truncate(saved_bindings);
        self.invalid_binding_recoveries
            .truncate(saved_invalid_binding_recoveries);
        self.local_names = saved_names;
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
            *result_type = actual;
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
            *result_type = actual;
        }
    }
}
