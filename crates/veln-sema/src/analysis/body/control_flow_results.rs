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
        let recovery_expected = if expected.is_none()
            && result
                .joined
                .as_ref()
                .is_none_or(AggregateTypeJoin::has_complete_domain)
            && result.recovery_type != Type::Unknown
        {
            Some(ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            })
        } else {
            None
        };
        let actual = self.infer_expr(branch_expr, expected.or(recovery_expected.as_ref()));
        if let Some(expected) = expected {
            self.check_assignable(branch_expr, &expected.ty, &actual, expected, "if_branch");
            return;
        }
        if !contribute_control_flow_result(&self.environment.adts, result, &actual) {
            let mismatch_expected = ExpectedType {
                ty: result.recovery_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: if_expr.node_id,
                origin_span: Some(if_expr.span.clone()),
                origin_message: "If result type inferred here.",
            };
            self.check_assignable(
                branch_expr,
                &mismatch_expected.ty,
                &actual,
                &mismatch_expected,
                "if_branch",
            );
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
        .is_some_and(|joined| joined.try_join_resolved(actual));
    if !joined {
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
}
