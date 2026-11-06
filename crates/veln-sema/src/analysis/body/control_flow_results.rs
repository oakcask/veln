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
        let recovery_type = match result.joined.as_ref() {
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
        let recovery_expected = if expected.is_none()
            && let Some(recovery_type) = recovery_type
        {
            Some(ExpectedType {
                ty: recovery_type,
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            })
        } else {
            None
        };
        let diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(branch_expr, expected.or(recovery_expected.as_ref()));
        if self.diagnostics.len() != diagnostic_count
            && expected
                .is_none_or(|expected| matches!(expected.source, ExpectedTypeSource::Inferred))
        {
            return;
        }
        if let Some(expected) = expected {
            self.check_assignable(branch_expr, &expected.ty, &actual, expected, "if_branch");
            return;
        }
        let recovering_from_failed_join = result.failed();
        if !contribute_control_flow_result(&self.environment.adts, result, &actual) {
            let mismatch_expected = ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            };
            if recovering_from_failed_join || type_contains_unknown(&actual) {
                self.check_assignable(
                    branch_expr,
                    &mismatch_expected.ty,
                    &actual,
                    &mismatch_expected,
                    "if_branch",
                );
            } else {
                self.push_inferred_type_mismatch(
                    branch_expr,
                    &mismatch_expected.ty,
                    &actual,
                    &mismatch_expected,
                    "if_branch",
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
        if result
            .joined
            .as_ref()
            .is_some_and(|joined| joined.has_same_root(actual))
        {
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
