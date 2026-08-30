use super::*;

pub(crate) fn private_expected_can_constrain(ty: &Type) -> bool {
    if !type_has_unknown(ty) {
        return true;
    }
    matches!(
        ty,
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } if !variadic.as_deref().is_some_and(type_has_unknown)
            && (params.iter().any(|param| !type_has_unknown(param))
            || !type_has_unknown(return_type)
            || variadic.as_deref().is_some_and(|ty| !type_has_unknown(ty)))
    )
}

pub(crate) fn private_call_site_non_target_params(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Vec<Type> {
    if let crate::adt::ConstructorLookup::Found(constructor) = context.constraints.adts.constructor(
        segments,
        context.current_module,
        context.constraints.uses,
    ) {
        return expected
            .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
            .map(|_| {
                constructor
                    .variant
                    .payload_fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .filter(|ty| !type_has_unknown(ty))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    if let Some(signature) = private_call_site_declared_signature(
        segments,
        context.current_module,
        context.constraints.uses,
        context.constraints.signatures_by_path,
    )
    .filter(|signature| {
        context.current_module == Some("std::prelude")
            || signature.module_name.as_deref() != Some("std::prelude")
    }) {
        return signature.params.clone();
    }

    private_prelude_constraint_name(
        segments,
        context.current_module,
        context.constraints.function_by_path,
    )
    .and_then(|name| {
        let input_type = private_prelude_input_arg(args, name).map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.constraints.uses,
                context.bindings,
                context.constraints.returns_by_path,
                context.constraints.adts,
            )
        });
        let mut params =
            crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
                .map(|(params, _)| params)?;
        if name == "vec_try_map_with" {
            let context_type = args.first().map(|arg| {
                infer_private_signature_expr_type(
                    arg,
                    None,
                    context.current_module,
                    context.constraints.uses,
                    context.bindings,
                    context.constraints.returns_by_path,
                    context.constraints.adts,
                )
            });
            apply_vec_try_map_with_context_param(&mut params, context_type);
        }
        Some(params)
    })
    .unwrap_or_default()
}

pub(crate) fn private_prelude_input_arg<'a>(
    args: &'a [Expr],
    helper_name: &str,
) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

pub(crate) fn collect_private_function_value_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let expected = expected.filter(|ty| private_expected_can_constrain(ty));
    let Some(Type::Function {
        params,
        variadic,
        return_type,
        ..
    }) = expected
    else {
        return;
    };
    let Some(target_key) = private_function_value_target(segments, context) else {
        return;
    };
    if context.caller_key == Some(&target_key) {
        return;
    }
    let Some(target_function) = context.constraints.function_by_path.get(&target_key) else {
        return;
    };
    let Some((omitted_params, omitted_return)) =
        context.constraints.omitted_private_slots.get(&target_key)
    else {
        return;
    };
    for (index, param) in params.iter().enumerate() {
        if omitted_params.get(index).copied().unwrap_or(false) && !type_has_unknown(param) {
            update_private_signature_param(
                context.constraints.functions,
                &target_key,
                index,
                param.clone(),
                context.constraints.changed,
            );
        }
    }
    if let Some(variadic) = variadic.as_deref().filter(|ty| !type_has_unknown(ty))
        && let Some(index) = target_function
            .params
            .iter()
            .position(|param| param.is_variadic && parameter_annotation_is_omitted(param))
        && omitted_params.get(index).copied().unwrap_or(false)
    {
        update_private_signature_variadic(
            context.constraints.functions,
            &target_key,
            variadic.clone(),
            context.constraints.changed,
        );
    }
    if *omitted_return && !type_has_unknown(return_type) {
        update_private_signature_return(
            context.constraints.functions,
            &target_key,
            return_type.as_ref().clone(),
            context.constraints.changed,
        );
    }
}

pub(crate) fn private_function_value_target(
    segments: &[String],
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
    {
        return binding.private_function_value.clone();
    }
    Some((context.current_module.map(str::to_string), name.clone()))
}

pub(crate) fn private_call_site_declared_signature<'a>(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    signatures_by_path: &'a FunctionSignatureMap,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => signatures_by_path.get(&(current_module.map(str::to_string), name.clone())),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    signatures_by_path.get(&(Some(use_decl.name.clone()), name.clone()))
                })
                .filter(|signature| signature.visibility == Visibility::Public)
        }
        _ => None,
    }
}

pub(crate) fn parameter_annotation_is_omitted(param: &veln_ast::Param) -> bool {
    param
        .ty
        .as_deref()
        .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
}

pub(crate) fn private_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    let key = (current_module.map(str::to_string), name.clone());
    let function = function_by_path.get(&key)?;
    (function.kind == FunctionKind::Function && function.visibility == Visibility::Private)
        .then_some(key)
}

pub(crate) fn private_same_module_call_target(
    callee: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    private_name_path_target(segments, current_module, function_by_path)
}

pub(crate) fn update_private_signature_param(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    index: usize,
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = private_signature_mut(functions, key) else {
        return;
    };
    let Some(current) = signature.params.get_mut(index) else {
        return;
    };
    update_unknown_private_signature_type(current, inferred, changed);
}

pub(crate) fn update_private_signature_variadic(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = private_signature_mut(functions, key) else {
        return;
    };
    let Some(current) = signature.variadic.as_mut() else {
        return;
    };
    update_unknown_private_signature_type(current, inferred, changed);
}

pub(crate) fn update_private_signature_return(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = private_signature_mut(functions, key) else {
        return;
    };
    update_unknown_private_signature_type(&mut signature.return_type, inferred, changed);
}

fn private_signature_mut<'a>(
    functions: &'a mut [FunctionSignature],
    key: &FunctionKey,
) -> Option<&'a mut FunctionSignature> {
    functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
}

fn update_unknown_private_signature_type(current: &mut Type, inferred: Type, changed: &mut bool) {
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}
