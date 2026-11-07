use super::*;

impl<'a> FunctionChecker<'a> {
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
        result: &mut ControlFlowResultJoin,
    ) {
        self.infer_control_flow_result(
            if_expr,
            branch_expr,
            expected,
            result,
            "If result type inferred here.",
            "if_branch",
        );
    }

    pub(super) fn infer_control_flow_result(
        &mut self,
        result_expr: &Expr,
        branch_expr: &Expr,
        expected: Option<&ExpectedType>,
        result: &mut ControlFlowResultJoin,
        origin_message: &'static str,
        constraint: &'static str,
    ) {
        let recovery_expected = self.control_flow_result_expected(
            result_expr,
            branch_expr,
            expected,
            result,
            origin_message,
        );
        let diagnostic_count = self.diagnostics.len();
        self.begin_binding_inference_transaction();
        let actual = self.infer_expr(branch_expr, expected.or(recovery_expected.as_ref()));
        if self.diagnostics.len() != diagnostic_count
            && expected.is_none_or(|expected| {
                matches!(
                    expected.source,
                    ExpectedTypeSource::Inferred | ExpectedTypeSource::AdmittedAggregate
                )
            })
        {
            self.rollback_binding_inference_transaction();
            return;
        }
        self.check_control_flow_result(
            result_expr,
            branch_expr,
            expected,
            result,
            &actual,
            origin_message,
            constraint,
        );
        if self.diagnostics.len() == diagnostic_count {
            self.commit_binding_inference_transaction();
        } else {
            self.rollback_binding_inference_transaction();
        }
    }

    fn control_flow_result_expected(
        &self,
        result_expr: &Expr,
        branch_expr: &Expr,
        expected: Option<&ExpectedType>,
        result: &ControlFlowResultJoin,
        origin_message: &'static str,
    ) -> Option<ExpectedType> {
        let admitted = result
            .joined
            .as_ref()
            .is_some_and(AggregateTypeJoin::has_compiler_covariant_root);
        let recovery_type = match result.joined.as_ref() {
            Some(joined) if joined.has_compiler_covariant_root() => {
                Some(joined.expression_admitted_context(
                    branch_expr,
                    &self.environment.adts,
                    self.function.module_name.as_deref(),
                    &self.resolution_uses,
                    &|name| self.projection_local_context(name),
                ))
            }
            Some(joined)
                if joined.has_complete_domain()
                    && !joined.has_incomplete_nested_refinement_domain() =>
            {
                Some(joined.inference_type())
            }
            Some(_) => None,
            None if result.recovery_type != Type::Unknown => Some(result.recovery_type.clone()),
            None => None,
        };
        let needs_pending_context = recovery_type.is_some()
            || crate::aggregate_type_join::expression_has_compiler_covariant_root(
                branch_expr,
                &self.environment.adts,
                self.function.module_name.as_deref(),
                &self.resolution_uses,
            );
        if expected.is_none() && needs_pending_context {
            Some(ExpectedType {
                ty: recovery_type.unwrap_or(Type::Unknown),
                source: if admitted {
                    ExpectedTypeSource::AdmittedAggregate
                } else {
                    ExpectedTypeSource::Inferred
                },
                origin_node_id: result_expr.node_id,
                origin_span: Some(result_expr.span.clone()),
                origin_message,
            })
        } else {
            None
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_control_flow_result(
        &mut self,
        result_expr: &Expr,
        branch_expr: &Expr,
        expected: Option<&ExpectedType>,
        result: &mut ControlFlowResultJoin,
        actual: &Type,
        origin_message: &'static str,
        constraint: &'static str,
    ) {
        if let Some(expected) = expected {
            self.check_assignable(branch_expr, &expected.ty, actual, expected, constraint);
            return;
        }
        let recovering_from_failed_join = result.failed();
        if !contribute_control_flow_result(&self.environment.adts, result, actual) {
            let mismatch_expected = ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: result_expr.node_id,
                origin_span: Some(result_expr.span.clone()),
                origin_message,
            };
            if recovering_from_failed_join || type_contains_unknown(actual) {
                self.check_assignable(
                    branch_expr,
                    &mismatch_expected.ty,
                    actual,
                    &mismatch_expected,
                    constraint,
                );
            } else {
                self.push_inferred_type_mismatch(
                    branch_expr,
                    &mismatch_expected.ty,
                    actual,
                    &mismatch_expected,
                    constraint,
                );
            }
        }
    }
}

pub(super) fn contribute_control_flow_result(
    adts: &crate::adt::registry::AdtRegistry,
    result: &mut ControlFlowResultJoin,
    actual: &Type,
) -> bool {
    if actual == &Type::Unknown {
        return true;
    }
    if result.recovery_type == Type::Unknown {
        result.recovery_type = inferred_control_flow_recovery_type(actual);
        result.joined = AggregateTypeJoin::new_resolved_refinement(adts, actual);
        return true;
    }
    if result.failed {
        return actual == &result.recovery_type;
    }
    if result.joined.is_none() && actual == &result.recovery_type {
        return true;
    }
    let joined = result
        .joined
        .as_mut()
        .is_some_and(|joined| joined.try_join_resolved(adts, actual));
    if !joined {
        if result.joined.as_ref().is_some_and(|joined| {
            joined.has_compiler_covariant_root() && joined.has_same_root(actual)
        }) {
            return false;
        }
        result.failed = true;
        result.joined = None;
    }
    joined
}

pub(super) struct ControlFlowResultJoin {
    pub(super) recovery_type: Type,
    pub(super) joined: Option<AggregateTypeJoin>,
    failed: bool,
}

impl ControlFlowResultJoin {
    pub(super) fn new(recovery_type: Type) -> Self {
        Self {
            recovery_type,
            joined: None,
            failed: false,
        }
    }

    pub(super) fn materialize(self) -> Type {
        if self.failed {
            self.recovery_type
        } else {
            self.joined
                .as_ref()
                .map(AggregateTypeJoin::result_type)
                .unwrap_or(self.recovery_type)
        }
    }

    pub(super) fn failed(&self) -> bool {
        self.failed
    }
}
