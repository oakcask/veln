use super::*;

impl FunctionChecker<'_> {
    pub(super) fn record_call_effects(
        &mut self,
        expr: &Expr,
        origin: &CallOrigin,
        instantiated_effects: &EffectRowSubstitutions,
    ) {
        for effect in &instantiate_effect_rows(&origin.effects, instantiated_effects) {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "direct_call",
                symbol: origin.symbol.clone(),
            });
        }
    }

    pub(in crate::analysis::body) fn check_call_arguments(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
    ) -> Option<EffectRowSubstitutions> {
        let row = call_bound_effect_row(params, variadic, &Type::Unknown, &origin.effects);
        self.check_call_arguments_with_origin(args, params, variadic, origin, None, row.as_deref())
    }

    pub(super) fn check_call_arguments_with_origin(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
        type_origin: Option<&TypeOrigin>,
        call_bound_row: Option<&str>,
    ) -> Option<EffectRowSubstitutions> {
        let valid_arity = variadic.is_some_and(|_| args.len() >= params.len())
            || (variadic.is_none() && args.len() == params.len());
        let inferred =
            self.infer_call_arguments(args, params, variadic, origin, type_origin, call_bound_row);
        let row_substitutions = solve_call_bound_effect_row(
            call_bound_row,
            inferred
                .iter()
                .map(|(_, expected, actual, _)| (&expected.ty, actual)),
        );
        let valid_arguments =
            self.check_inferred_call_arguments(inferred, origin, &row_substitutions);
        (valid_arity && valid_arguments).then_some(row_substitutions)
    }

    fn infer_call_arguments<'b>(
        &mut self,
        args: &'b [Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
        type_origin: Option<&TypeOrigin>,
        call_bound_row: Option<&str>,
    ) -> Vec<InferredCallArgument<'b>> {
        let mut inferred = Vec::new();
        for (index, arg) in args.iter().enumerate() {
            let param_type = params.get(index).or(variadic);
            let Some(param_type) = param_type else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = call_argument_expected(param_type, origin, type_origin);
            let inference_expected = inference_expected_type(&expected, call_bound_row);
            let diagnostics_before = self.diagnostics.len();
            let actual = self.infer_expr(arg, Some(&inference_expected));
            let argument_valid = self.diagnostics.len() == diagnostics_before;
            inferred.push((arg, expected, actual, argument_valid));
        }
        inferred
    }

    pub(super) fn check_inferred_call_arguments(
        &mut self,
        inferred: Vec<InferredCallArgument<'_>>,
        origin: &CallOrigin,
        row_substitutions: &EffectRowSubstitutions,
    ) -> bool {
        let mut valid = true;
        for (arg, mut expected, actual, argument_valid) in inferred {
            expected.ty = instantiate_effect_rows_in_type(&expected.ty, row_substitutions);
            valid &= argument_valid
                && is_assignable_with_registry(&expected.ty, &actual, &self.environment.adts);
            if origin.schema_encode {
                self.check_schema_encode_assignable(
                    arg,
                    &expected.ty,
                    &actual,
                    &expected,
                    "call_argument",
                );
            } else {
                self.check_assignable(arg, &expected.ty, &actual, &expected, "call_argument");
            }
        }
        valid
    }
}

pub(super) fn call_argument_expected(
    param_type: &Type,
    origin: &CallOrigin,
    type_origin: Option<&TypeOrigin>,
) -> ExpectedType {
    type_origin.map_or_else(
        || ExpectedType {
            ty: param_type.clone(),
            source: if origin.schema_encode {
                ExpectedTypeSource::SchemaEncodeParameter
            } else {
                ExpectedTypeSource::DeclaredParameter
            },
            origin_node_id: origin.node_id,
            origin_span: Some(origin.span.clone()),
            declaration_span: (!origin.schema_encode)
                .then(|| origin.declaration_span.clone())
                .flatten(),
            origin_message: "Callee parameter type declared here.",
        },
        |type_origin| ExpectedType {
            ty: param_type.clone(),
            source: type_origin.source,
            origin_node_id: type_origin.node_id,
            origin_span: Some(type_origin.span.clone()),
            declaration_span: type_origin.declaration_span.clone(),
            origin_message: type_origin.message,
        },
    )
}

pub(super) fn inference_expected_type(
    expected: &ExpectedType,
    call_bound_row: Option<&str>,
) -> ExpectedType {
    call_bound_row.map_or_else(
        || expected.clone(),
        |row| ExpectedType {
            ty: mask_call_bound_effect_row(&expected.ty, row),
            ..expected.clone()
        },
    )
}
