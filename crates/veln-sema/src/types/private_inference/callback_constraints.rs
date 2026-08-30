use super::*;

pub(crate) fn private_callback_return_constraint_can_update(
    segments: &[String],
    expected_callback: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let Type::Function { return_type, .. } = expected_callback else {
        return false;
    };
    if type_has_unknown(return_type) {
        return false;
    }
    let [name] = segments else {
        return false;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return false;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return false;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return false;
    }
    context.returns_by_path.get(&key) != Some(return_type)
}

pub(crate) struct PrivatePreludeCallbackConstraintContext<'a> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) bindings: &'a [Binding],
    pub(crate) function_by_path: &'a BTreeMap<(Option<String>, String), &'a Function>,
    pub(crate) omitted_private_returns: &'a BTreeSet<(Option<String>, String)>,
    pub(crate) returns_by_path: &'a mut BTreeMap<(Option<String>, String), Type>,
    pub(crate) adts: &'a AdtRegistry,
    pub(crate) changed: &'a mut bool,
}

pub(crate) fn collect_private_prelude_callback_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_prelude_callback_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_prelude_callback_expr_constraints(
                    &entry.key,
                    key_expected,
                    context,
                );
                collect_private_prelude_callback_expr_constraints(
                    &entry.value,
                    value_expected,
                    context,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_prelude_callback_expr_constraints(
                    &field.expr,
                    field_expected,
                    context,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_prelude_callback_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_prelude_callback_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_prelude_callback_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_prelude_callback_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_prelude_callback_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_prelude_callback_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_prelude_callback_expr_constraints(scrutinee, None, context);
            for arm in arms {
                collect_private_prelude_callback_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_prelude_callback_expr_constraints(
                condition,
                Some(&Type::bool()),
                context,
            );
            collect_private_prelude_callback_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_prelude_callback_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_prelude_callback_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_prelude_callback_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_prelude_callback_expr_constraints(left, expected, context);
            collect_private_prelude_callback_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            if let Some(expected) = expected {
                collect_private_callback_return_constraint_for_segments(
                    segments, expected, context,
                );
            }
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

pub(crate) fn collect_private_prelude_callback_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Some(params) = private_prelude_callback_call_params(
        callee,
        args,
        expected,
        context.current_module,
        context.uses,
        context.bindings,
        context.function_by_path,
        context.returns_by_path,
        context.adts,
    ) else {
        return;
    };
    for (arg, param) in args.iter().zip(params.iter()) {
        collect_private_callback_return_constraint(arg, param, context);
        collect_private_prelude_callback_expr_constraints(arg, Some(param), context);
    }
}

pub(crate) fn private_prelude_callback_call_params(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    function_by_path: &FunctionAstMap<'_>,
    returns_by_path: &FunctionReturnMap,
    adts: &AdtRegistry,
) -> Option<Vec<Type>> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    let name = private_prelude_constraint_name(segments, current_module, function_by_path)?;
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        )
    });
    let (mut params, _) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())?;
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                current_module,
                uses,
                bindings,
                returns_by_path,
                adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    Some(params)
}

pub(crate) fn apply_vec_try_map_with_context_param(
    params: &mut [Type],
    context_type: Option<Type>,
) {
    let Some(context_type) = context_type else {
        return;
    };
    if let Some(param) = params.first_mut() {
        *param = context_type.clone();
    }
    let Some(Type::Function {
        params: callback_params,
        ..
    }) = params.get_mut(2)
    else {
        return;
    };
    if let Some(callback_context) = callback_params.first_mut() {
        *callback_context = context_type;
    }
}

pub(crate) fn private_prelude_constraint_name<'a>(
    segments: &'a [String],
    current_module: Option<&str>,
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
) -> Option<&'a str> {
    match segments {
        [name]
            if !function_by_path
                .contains_key(&(current_module.map(str::to_string), name.clone())) =>
        {
            Some(name)
        }
        [module, name] if crate::source_less_lookup::is_reserved_source_less_module(module) => {
            Some(name)
        }
        _ => None,
    }
}

pub(crate) fn collect_private_callback_return_constraint(
    arg: &Expr,
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let ExprKind::NamePath(segments) = &arg.kind else {
        return;
    };
    collect_private_callback_return_constraint_for_segments(segments, expected_callback, context);
}

pub(crate) fn collect_private_callback_return_constraint_for_segments(
    segments: &[String],
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let [name] = segments else {
        return;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return;
    }
    if context.returns_by_path.get(&key) == Some(return_type) {
        return;
    }
    context
        .returns_by_path
        .insert(key, return_type.as_ref().clone());
    *context.changed = true;
}
