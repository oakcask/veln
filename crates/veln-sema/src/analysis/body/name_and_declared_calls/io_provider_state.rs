use super::*;

impl FunctionChecker<'_> {
    pub(super) fn infer_io_provider_state_actuals(
        &mut self,
        operation: &str,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Vec<(Type, bool)> {
        let mut actuals = Vec::with_capacity(args.len());
        for (index, arg) in args.iter().enumerate() {
            if operation == "update"
                && index == 0
                && matches!(arg.kind, ExprKind::NamePath { .. })
                && let Some(actual) = self.shallow_expr_type(arg)
            {
                actuals.push((actual, true));
                continue;
            }
            let initial_expected = (operation == "state" && index == 0)
                .then(|| expected.and_then(io_provider_state_initial_expected))
                .flatten();
            let diagnostics_before = self.diagnostics.len();
            let actual = self.infer_expr(arg, initial_expected.as_ref());
            actuals.push((actual, self.diagnostics.len() == diagnostics_before));
        }
        actuals
    }

    pub(super) fn reinfer_io_provider_state_cell(
        &mut self,
        operation: &str,
        args: &[Expr],
        params: &[Type],
        origin: &CallOrigin,
        actuals: &mut [(Type, bool)],
    ) {
        if operation != "update" {
            return;
        }
        let (Some(arg), Some(param_type), Some(actual)) =
            (args.first(), params.first(), actuals.first_mut())
        else {
            return;
        };
        if !matches!(arg.kind, ExprKind::NamePath { .. }) {
            return;
        }
        let expected = call_argument_expected(param_type, origin, None);
        let diagnostics_before = self.diagnostics.len();
        actual.0 = self.infer_expr(arg, Some(&expected));
        actual.1 = self.diagnostics.len() == diagnostics_before;
    }
}

fn io_provider_state_initial_expected(expected: &ExpectedType) -> Option<ExpectedType> {
    let Type::Named {
        name,
        identity,
        args,
    } = &expected.ty
    else {
        return None;
    };
    let [item] = args.as_slice() else {
        return None;
    };
    if name != "State" || identity != "std::io_provider::State" || type_contains_unknown(item) {
        return None;
    }
    Some(ExpectedType {
        ty: item.clone(),
        ..expected.clone()
    })
}

pub(super) fn call_arity_is_valid(
    argument_count: usize,
    parameter_count: usize,
    variadic: Option<&Type>,
) -> bool {
    variadic.is_some_and(|_| argument_count >= parameter_count)
        || (variadic.is_none() && argument_count == parameter_count)
}

pub(super) fn io_provider_argument_evidence(
    actuals: &[(Type, bool)],
) -> crate::call_resolution::CallArgumentEvidence<'_, Type> {
    crate::call_resolution::CallArgumentEvidence {
        first: actuals.first().map(|(ty, _)| ty),
        second: actuals.get(1).map(|(ty, _)| ty),
        third: actuals.get(2).map(|(ty, _)| ty),
    }
}

pub(super) fn inferred_io_provider_arguments<'a>(
    args: &'a [Expr],
    actuals: &[(Type, bool)],
    params: &[Type],
    variadic: Option<&Type>,
    origin: &CallOrigin,
) -> Vec<InferredCallArgument<'a>> {
    args.iter()
        .zip(actuals)
        .enumerate()
        .filter_map(|(index, (arg, (actual, valid)))| {
            let param_type = params.get(index).or(variadic)?;
            Some((
                arg,
                call_argument_expected(param_type, origin, None),
                actual.clone(),
                *valid,
            ))
        })
        .collect()
}
