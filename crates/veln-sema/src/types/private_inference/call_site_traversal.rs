use super::*;
use crate::types::TypeParameterIdentityContext;

pub(crate) struct PrivateCallSiteConstraintContext<'a> {
    pub(crate) uses: &'a [UseDecl],
    pub(crate) function_by_path: &'a FunctionAstMap<'a>,
    pub(crate) omitted_private_slots: &'a PrivateSlotMap,
    pub(crate) signatures_by_path: &'a FunctionSignatureMap,
    pub(crate) returns_by_path: &'a FunctionReturnMap,
    pub(crate) functions: &'a mut [FunctionSignature],
    pub(crate) adts: &'a AdtRegistry,
    pub(crate) changed: &'a mut bool,
}

pub(crate) struct PrivateCallSiteExprContext<'a, 'b> {
    pub(crate) current_module: Option<&'b str>,
    pub(crate) current_package: Option<&'b str>,
    pub(crate) caller_key: Option<&'b FunctionKey>,
    pub(crate) type_parameters: &'b TypeParameterIdentityContext<'b>,
    pub(crate) bindings: &'b mut PrivateBindings,
    pub(crate) constraints: &'b mut PrivateCallSiteConstraintContext<'a>,
}

pub(crate) struct PrivateCallbackBindingContext<'a, 'function> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) current_package: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) function_by_path: &'a FunctionAstMap<'function>,
    pub(crate) returns_by_path: &'a FunctionReturnMap,
    pub(crate) adts: &'a AdtRegistry,
}

pub(crate) fn collect_private_call_site_constraints(
    function: &Function,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    #[cfg(test)]
    private_inference_counters::record_call_site_scan();

    let current_module = function.module_name.as_deref();
    let current_package = function.package_name.as_deref();
    let caller_key = function.name.as_ref().map(|name| {
        (
            function.package_name.clone(),
            function.module_name.clone(),
            name.clone(),
        )
    });
    let type_parameters = TypeParameterIdentityContext::for_function(function);
    let mut bindings = private_function_body_bindings(function, context.signatures_by_path);
    let declared_return = private_call_site_declared_return(
        function,
        caller_key.as_ref(),
        context.signatures_by_path,
    );

    collect_private_call_site_body_constraints(
        &function.body,
        declared_return.as_ref(),
        current_module,
        current_package,
        caller_key.as_ref(),
        &type_parameters,
        &mut bindings,
        context,
    );
}

fn private_call_site_declared_return(
    function: &Function,
    caller_key: Option<&FunctionKey>,
    signatures_by_path: &FunctionSignatureMap,
) -> Option<Type> {
    if function.type_binders.is_empty()
        && let Some(return_type) = function.return_type.as_deref()
    {
        return Some(parse_type_or_unknown(Some(return_type)));
    }
    caller_key
        .and_then(|key| signatures_by_path.get(key))
        .map(|signature| signature.return_type.clone())
        .filter(|ty| !type_has_unknown(ty))
}

#[allow(clippy::too_many_arguments)]
fn collect_private_call_site_body_constraints(
    body: &[BodyLine],
    tail_expected: Option<&Type>,
    current_module: Option<&str>,
    current_package: Option<&str>,
    caller_key: Option<&FunctionKey>,
    type_parameters: &TypeParameterIdentityContext<'_>,
    bindings: &mut PrivateBindings,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    for (index, line) in body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
                ..
            } => {
                let annotation_type = annotation.as_deref().map(|annotation| {
                    let ty = parse_type_or_unknown(Some(annotation));
                    if type_parameters.name_count() == 0 {
                        ty
                    } else {
                        canonicalize_private_call_site_annotation(
                            ty,
                            current_module,
                            current_package,
                            context.uses,
                            context.adts,
                            type_parameters,
                        )
                    }
                });
                collect_private_call_site_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        current_package,
                        caller_key,
                        type_parameters,
                        bindings,
                        constraints: context,
                    },
                );
                collect_private_callback_let_bindings(
                    pattern,
                    expr,
                    annotation_type,
                    &PrivateCallbackBindingContext {
                        current_module,
                        current_package,
                        uses: context.uses,
                        function_by_path: context.function_by_path,
                        returns_by_path: context.returns_by_path,
                        adts: context.adts,
                    },
                    bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == body.len()).then_some(tail_expected).flatten();
                collect_private_call_site_expr_constraints(
                    expr,
                    expected,
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        current_package,
                        caller_key,
                        type_parameters,
                        bindings,
                        constraints: context,
                    },
                );
            }
            BodyLineKind::Defer { body, .. } => {
                let binding_count = bindings.len();
                record_scoped_binding_count(bindings);
                collect_private_call_site_body_constraints(
                    body,
                    Some(&Type::unit()),
                    current_module,
                    current_package,
                    caller_key,
                    type_parameters,
                    bindings,
                    context,
                );
                bindings.truncate(binding_count);
            }
        }
    }
}

pub(crate) fn collect_private_callback_let_bindings(
    pattern: &Pattern,
    expression: &Expr,
    annotation_type: Option<Type>,
    context: &PrivateCallbackBindingContext<'_, '_>,
    bindings: &mut PrivateBindings,
) {
    let initializer_private_function = annotation_type
        .is_none()
        .then(|| {
            private_same_module_call_target(
                expression,
                context.current_module,
                context.current_package,
                context.function_by_path,
            )
        })
        .flatten();
    let ty = annotation_type.unwrap_or_else(|| {
        infer_private_signature_expr_type(
            expression,
            None,
            context.current_module,
            context.current_package,
            context.uses,
            bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    collect_let_pattern_bindings(pattern, &ty, initializer_private_function, bindings);
}

pub(crate) fn collect_private_call_site_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(_) | ExprKind::Dict(_) | ExprKind::Record(_) => {
            collect_private_call_site_collection_constraints(expr, expected, context);
        }
        ExprKind::Call { callee, args } => {
            collect_private_call_site_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaValidate { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try { .. }
        | ExprKind::Prefix { .. } => {
            collect_private_call_site_wrapped_expr_constraints(expr, expected, context);
        }
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Binary { .. } => {
            collect_private_call_site_control_flow_constraints(expr, expected, context);
        }
        ExprKind::Begin { body, .. } => {
            let binding_count = context.bindings.len();
            record_scoped_binding_count(context.bindings);
            collect_private_call_site_body_constraints(
                body,
                expected,
                context.current_module,
                context.current_package,
                context.caller_key,
                context.type_parameters,
                context.bindings,
                context.constraints,
            );
            context.bindings.truncate(binding_count);
        }
        ExprKind::NamePath { segments, .. } => {
            collect_private_parameter_constraints(segments, expected, context);
            collect_private_function_value_constraints(segments, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn canonicalize_private_call_site_annotation(
    ty: Type,
    current_module: Option<&str>,
    current_package: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
    type_parameters: &TypeParameterIdentityContext<'_>,
) -> Type {
    crate::types::signature_collection::canonicalize_type_effects_with_context(
        ty,
        uses,
        &[],
        current_module,
        current_package,
        &[],
        adts,
        &BTreeMap::new(),
        Some(type_parameters),
    )
}

pub(crate) fn collect_private_call_site_collection_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_call_site_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_call_site_expr_constraints(&entry.key, key_expected, context);
                collect_private_call_site_expr_constraints(&entry.value, value_expected, context);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_call_site_expr_constraints(&field.expr, field_expected, context);
            }
        }
        _ => {}
    }
}

pub(crate) fn private_prelude_callback_collection_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            let item_expected = expected.and_then(Type::vec_part);
            private_prelude_callback_expr_references_slot(item, item_expected, context)
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            private_prelude_callback_expr_references_slot(&entry.key, key_expected, context)
                || private_prelude_callback_expr_references_slot(
                    &entry.value,
                    value_expected,
                    context,
                )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            let field_expected = expected.and_then(|expected| expected.record_field(&field.name));
            private_prelude_callback_expr_references_slot(&field.expr, field_expected, context)
        }),
        _ => false,
    }
}

pub(crate) fn collect_private_call_site_wrapped_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, entries } => {
            collect_private_call_site_expr_constraints(body, expected, context);
            for entry in entries {
                for arg in &entry.args {
                    collect_private_call_site_expr_constraints(arg, None, context);
                }
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_call_site_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_call_site_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_call_site_expr_constraints(value, None, context);
        }
        ExprKind::SchemaValidate { value, base, .. } => {
            collect_private_call_site_expr_constraints(value, None, context);
            collect_private_call_site_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try { expr: base, .. }
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_call_site_expr_constraints(base, None, context);
        }
        _ => {}
    }
}

pub(crate) fn collect_private_call_site_control_flow_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                context.current_module,
                context.constraints.uses,
                context.constraints.adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            collect_private_call_site_expr_constraints(
                scrutinee,
                scrutinee_expected.as_ref(),
                context,
            );
            for arm in arms {
                collect_private_call_site_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_call_site_expr_constraints(condition, Some(&Type::bool()), context);
            collect_private_call_site_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_call_site_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_call_site_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_call_site_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_call_site_expr_constraints(left, expected, context);
            collect_private_call_site_expr_constraints(right, expected, context);
        }
        _ => {}
    }
}

pub(crate) fn collect_private_parameter_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(expected) = expected.filter(|ty| !type_has_unknown(ty)) else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let Some(caller_key) = context.caller_key else {
        return;
    };
    let Some((omitted_params, _)) = context.constraints.omitted_private_slots.get(caller_key)
    else {
        return;
    };
    let Some(function) = context.constraints.function_by_path.get(caller_key) else {
        return;
    };
    let Some(index) = function
        .params
        .iter()
        .position(|param| param.name == *name && parameter_annotation_is_omitted(param))
    else {
        return;
    };
    if !omitted_params.get(index).copied().unwrap_or(false) {
        return;
    }
    if function.params[index].is_variadic {
        let Some(item_type) = expected.vec_part().filter(|ty| !type_has_unknown(ty)) else {
            return;
        };
        update_private_signature_variadic(
            context.constraints.functions,
            caller_key,
            item_type.clone(),
            context.constraints.changed,
        );
    } else {
        update_private_signature_param(
            context.constraints.functions,
            caller_key,
            index,
            expected.clone(),
            context.constraints.changed,
        );
    }
}

pub(crate) fn collect_private_call_site_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(target_key) = private_same_module_call_target(
        callee,
        context.current_module,
        context.current_package,
        context.constraints.function_by_path,
    ) else {
        collect_private_call_site_non_target_call_args(callee, args, expected, context);
        return;
    };

    let is_recursive_edge = context.caller_key == Some(&target_key);
    if !is_recursive_edge
        && let Some((omitted_params, omitted_return)) =
            context.constraints.omitted_private_slots.get(&target_key)
    {
        if let Some(target_params) = context
            .constraints
            .signatures_by_path
            .get(&target_key)
            .map(|signature| signature.params.to_vec())
        {
            for (index, arg) in args.iter().enumerate() {
                if omitted_params.get(index).copied().unwrap_or(false) {
                    let actual = infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.current_package,
                        context.constraints.uses,
                        context.bindings,
                        context.constraints.returns_by_path,
                        context.constraints.adts,
                    );
                    if !type_has_unknown(&actual) {
                        update_private_signature_param(
                            context.constraints.functions,
                            &target_key,
                            index,
                            actual,
                            context.constraints.changed,
                        );
                    }
                }
                let arg_expected = target_params
                    .get(index)
                    .filter(|ty| private_expected_can_constrain(ty));
                collect_private_call_site_expr_constraints(arg, arg_expected, context);
            }
        }

        if *omitted_return
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            update_private_signature_return(
                context.constraints.functions,
                &target_key,
                expected.clone(),
                context.constraints.changed,
            );
        }
    }

    if context
        .constraints
        .omitted_private_slots
        .contains_key(&target_key)
    {
        return;
    }
    let Some(target_params) = context
        .constraints
        .signatures_by_path
        .get(&target_key)
        .map(|signature| signature.params.to_vec())
    else {
        return;
    };
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = target_params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

pub(crate) fn collect_private_call_site_non_target_call_args(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        for arg in args {
            collect_private_call_site_expr_constraints(arg, None, context);
        }
        return;
    };
    let params = private_call_site_non_target_params(segments, args, expected, context);
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}
