use super::*;

pub(crate) fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let expected = expected.filter(|expected| !is_incomplete_compiler_covariant(expected));
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    infer_private_control_flow_results(
        arms.iter().map(|arm| &arm.expr),
        expected,
        context,
        false,
        false,
    )
}

pub(crate) fn infer_private_if_result_type(
    condition: &Expr,
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let expected = expected.filter(|expected| !is_incomplete_compiler_covariant(expected));
    context.infer_expected_outcome(condition, &Type::bool());
    for branch in else_if_branches {
        context.infer_expected_outcome(&branch.condition, &Type::bool());
    }
    infer_private_control_flow_results(
        std::iter::once(then_branch)
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch)),
        expected,
        context,
        true,
        true,
    )
}

fn infer_private_control_flow_results<'a>(
    branches: impl Iterator<Item = &'a Expr>,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
    check_expected: bool,
    report_join_failure: bool,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    let mut joined_result = None;
    let mut join_failed = false;
    for branch in branches {
        let branch_expected = expected.cloned().or_else(|| {
            private_control_flow_branch_context(branch, &result, joined_result.as_ref(), context)
        });
        let outcome = if check_expected {
            match branch_expected.as_ref() {
                Some(expected) => context.infer_expected_outcome(branch, expected),
                None => context.infer_outcome(branch, None),
            }
        } else {
            context.infer_outcome(branch, branch_expected.as_ref())
        };
        if !outcome.may_contribute {
            continue;
        }
        let was_join_failed = join_failed;
        merge_private_control_flow_result(
            &mut result,
            &mut joined_result,
            &mut join_failed,
            outcome.ty,
            context.adts,
            expected.is_none(),
        );
        if report_join_failure && !was_join_failed && join_failed {
            *context.failures += 1;
        }
    }
    materialize_private_control_flow_result(result, joined_result, join_failed)
}

fn private_control_flow_branch_context(
    branch: &Expr,
    result: &Type,
    joined: Option<&crate::aggregate_type_join::AggregateTypeJoin>,
    context: &PrivateSignatureInferContext<'_, '_>,
) -> Option<Type> {
    let inferred = match joined {
        Some(joined) if joined.has_compiler_covariant_root() => {
            crate::aggregate_type_join::expression_admitted_context(
                branch,
                &joined.inference_type(),
                context.adts,
                context.current_module,
                context.uses,
            )
        }
        Some(joined)
            if joined.has_complete_domain()
                && !joined.has_incomplete_nested_refinement_domain() =>
        {
            joined.inference_type()
        }
        Some(_) => return None,
        None => result.clone(),
    };
    item_type_unknown_as_none(&inferred).cloned()
}

fn is_incomplete_compiler_covariant(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { identity, .. } | Type::VariantRefinement { identity, .. }
            if Type::is_compiler_covariant_identity(identity) && type_has_unknown(ty)
    )
}

fn merge_private_control_flow_result(
    result: &mut Type,
    joined_result: &mut Option<crate::aggregate_type_join::AggregateTypeJoin>,
    join_failed: &mut bool,
    actual: Type,
    adts: &AdtRegistry,
    allow_join: bool,
) {
    if *result == Type::Unknown {
        *joined_result =
            crate::aggregate_type_join::AggregateTypeJoin::new_resolved_refinement(adts, &actual);
        *result = private_control_flow_recovery_type(&actual);
        return;
    }
    if actual == Type::Unknown {
        return;
    }
    if !allow_join {
        merge_expected_private_control_flow_result(result, &actual);
        return;
    }
    if *join_failed {
        return;
    }
    if joined_result.is_none() && *result == actual {
        return;
    }
    if let Some(joined) = joined_result.as_mut()
        && joined.try_join_resolved(adts, &actual)
    {
        return;
    }
    if joined_result
        .as_ref()
        .is_some_and(|joined| joined.has_same_root(&actual))
    {
        return;
    }
    *join_failed = true;
    *joined_result = None;
}

fn private_control_flow_recovery_type(ty: &Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name, identity, args.to_vec()),
        ty => ty.clone(),
    }
}

fn materialize_private_control_flow_result(
    recovery_type: Type,
    joined_result: Option<crate::aggregate_type_join::AggregateTypeJoin>,
    join_failed: bool,
) -> Type {
    if join_failed {
        recovery_type
    } else {
        joined_result
            .as_ref()
            .map(crate::aggregate_type_join::AggregateTypeJoin::result_type)
            .unwrap_or(recovery_type)
    }
}

fn merge_expected_private_control_flow_result(result: &mut Type, actual: &Type) {
    let common_base = match (&*result, actual) {
        (
            Type::VariantRefinement {
                name,
                identity,
                args,
                ..
            },
            Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                ..
            }
            | Type::Named {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if identity == actual_identity && args == actual_args => {
            Some(Type::resolved_named(name, identity, args.to_vec()))
        }
        (
            Type::Named {
                name,
                identity,
                args,
            },
            Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if identity == actual_identity && args == actual_args => {
            Some(Type::resolved_named(name, identity, args.to_vec()))
        }
        _ => None,
    };
    if let Some(common_base) = common_base {
        *result = common_base;
    }
}
