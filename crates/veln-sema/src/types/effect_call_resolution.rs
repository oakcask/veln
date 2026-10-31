use super::effect_inference::{
    EffectsByFunction, EffectsByModulePath, user_effect_signature_for_path,
};
use super::*;

use crate::effect_rows::{
    call_bound_effect_row, effect_row_in_type, instantiate_effect_rows,
    instantiate_effect_rows_in_type, solve_call_bound_effect_row,
};
use crate::effects::concurrency_effects;

pub(super) fn instantiate_call_effect_rows(
    signature: &FunctionSignature,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    instantiate_callable_effect_rows(
        &signature.params,
        signature.variadic.as_ref(),
        &signature.return_type,
        &signature.effects,
        args,
        context,
    )
}

pub(super) fn instantiate_local_call_effect_rows(
    name: &str,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let binding = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == name)?;
    let row = binding.call_bound_effect_row.as_deref()?;
    let Type::Function {
        params,
        variadic,
        effects,
        ..
    } = &binding.ty
    else {
        return None;
    };
    instantiate_callable_effect_rows_with_row(
        params,
        variadic.as_deref(),
        effects,
        Some(row),
        args,
        context,
    )
}

pub(super) fn local_call_has_bound_effect_row(name: &str, context: &ExprEffectContext<'_>) -> bool {
    context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == name)
        .is_some_and(|binding| binding.call_bound_effect_row.is_some())
}

fn instantiate_callable_effect_rows(
    params: &[Type],
    variadic: Option<&Type>,
    return_type: &Type,
    effects: &[String],
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let row = call_bound_effect_row(params, variadic, return_type, effects);
    instantiate_callable_effect_rows_with_row(
        params,
        variadic,
        effects,
        row.as_deref(),
        args,
        context,
    )
}

fn instantiate_callable_effect_rows_with_row(
    params: &[Type],
    variadic: Option<&Type>,
    effects: &[String],
    row: Option<&str>,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let arity_matches =
        args.len() >= params.len() && (variadic.is_some() || args.len() == params.len());
    if !arity_matches {
        return None;
    }
    let mut comparisons = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let param = params.get(index).or(variadic)?;
        let Some(actual) = type_for_effect_call_argument(arg, param, context) else {
            if row.is_some() {
                return None;
            }
            continue;
        };
        comparisons.push((param, actual));
    }
    let row_substitutions = solve_call_bound_effect_row(
        row,
        comparisons
            .iter()
            .map(|(expected, actual)| (*expected, actual)),
    );
    if comparisons.iter().any(|(expected, actual)| {
        !crate::type_relations::is_assignable(
            &instantiate_effect_rows_in_type(expected, &row_substitutions),
            actual,
        )
    }) {
        return None;
    }
    Some(instantiate_effect_rows(effects, &row_substitutions))
}

pub(super) fn function_type_for_expr(expr: &Expr, context: &ExprEffectContext<'_>) -> Option<Type> {
    let segments = expr.callee_name_path()?;
    match segments {
        [name] => context
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                effect_function_signature_path(
                    segments,
                    context.uses,
                    context.functions,
                    context.current_module,
                    context.current_package,
                    context.companion_access_targets,
                )
                .map(FunctionSignature::ty)
            }),
        _ => {
            let public_or_same_module_access = BTreeMap::new();
            effect_function_signature_path(
                segments,
                context.uses,
                context.functions,
                context.current_module,
                context.current_package,
                &public_or_same_module_access,
            )
            .map(FunctionSignature::ty)
        }
    }
}

pub(super) fn call_bound_effect_row_for_expr(
    expr: &Expr,
    context: &ExprEffectContext<'_>,
) -> Option<String> {
    let segments = expr.callee_name_path()?;
    if let [name] = segments
        && let Some(binding) = context
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
    {
        return binding.call_bound_effect_row.clone();
    }
    let signature = effect_function_signature_path(
        segments,
        context.uses,
        context.functions,
        context.current_module,
        context.current_package,
        context.companion_access_targets,
    )?;
    call_bound_effect_row(
        &signature.params,
        signature.variadic.as_ref(),
        &signature.return_type,
        &signature.effects,
    )
}

fn type_for_effect_call_argument(
    expr: &Expr,
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    match &expr.kind {
        ExprKind::SchemaDecode {
            schema,
            input,
            base,
        } => type_for_effect_schema_call(
            schema,
            schema_decode_step_function_name,
            &[input, base],
            context,
        ),
        ExprKind::SchemaEncode { schema, value } => {
            type_for_effect_schema_call(schema, schema_encode_function_name, &[value], context)
        }
        ExprKind::SchemaValidate {
            schema,
            value,
            base,
            ..
        } => type_for_effect_schema_call(
            schema,
            schema_runtime_validation_function_name,
            &[value, base],
            context,
        ),
        ExprKind::StringLiteral(_) => Some(Type::string()),
        ExprKind::IntLiteral(_) => Some(Type::int()),
        ExprKind::FloatLiteral(_) => Some(Type::float()),
        ExprKind::BoolLiteral(_) => Some(Type::bool()),
        ExprKind::Unit => Some(Type::unit()),
        ExprKind::Perform {
            effect,
            operation,
            args,
            ..
        } => {
            let effect = user_effect_signature_for_path(
                effect,
                context.uses,
                context.current_module,
                context.current_package,
                context.user_effects,
                context.companion_effect_access_targets,
            )?;
            let operation = effect
                .operations
                .iter()
                .find(|candidate| candidate.name == *operation)?;
            if args.len() != operation.params.len() {
                return None;
            }
            for (arg, param) in args.iter().zip(&operation.params) {
                let actual = type_for_effect_call_argument(arg, param, context)?;
                if !crate::type_relations::is_assignable(param, &actual) {
                    return None;
                }
            }
            Some(operation.return_type.clone())
        }
        ExprKind::Record(fields) => {
            let Type::Record(expected_fields) = expected else {
                return None;
            };
            fields
                .iter()
                .map(|field| {
                    let expected_field = expected_fields
                        .iter()
                        .find(|(name, _)| name == &field.name)
                        .map(|(_, ty)| ty)?;
                    Some((
                        field.name.clone(),
                        type_for_effect_call_argument(&field.expr, expected_field, context)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()
                .map(Type::Record)
        }
        ExprKind::Call { callee, args } => {
            if let Some(constructed) =
                type_for_effect_constructor_call(callee, args, expected, context)
            {
                return Some(constructed);
            }
            let Type::Function {
                params,
                variadic,
                return_type,
                effects,
            } = function_type_for_expr(callee, context)?
            else {
                return None;
            };
            let arity_matches =
                args.len() >= params.len() && (variadic.is_some() || args.len() == params.len());
            if !arity_matches {
                return None;
            }
            let mut comparisons = Vec::new();
            for (index, arg) in args.iter().enumerate() {
                let expected = params.get(index).or(variadic.as_deref())?;
                comparisons.push((
                    expected,
                    type_for_effect_call_argument(arg, expected, context)?,
                ));
            }
            let row = call_bound_effect_row_for_expr(callee, context).or_else(|| {
                call_bound_effect_row(&params, variadic.as_deref(), &return_type, &effects)
            });
            let substitutions = solve_call_bound_effect_row(
                row.as_deref(),
                comparisons
                    .iter()
                    .map(|(expected, actual)| (*expected, actual)),
            );
            if comparisons.iter().any(|(expected, actual)| {
                !crate::type_relations::is_assignable(
                    &instantiate_effect_rows_in_type(expected, &substitutions),
                    actual,
                )
            }) {
                return None;
            }
            Some(instantiate_effect_rows_in_type(
                &return_type,
                &substitutions,
            ))
        }
        _ => function_type_for_expr(expr, context).or_else(|| {
            infer_checked_effect_argument_type(
                expr,
                expected,
                context.current_module,
                context.current_package,
                context.uses,
                context.bindings,
                context.functions,
                context.adts,
            )
        }),
    }
}

fn type_for_effect_schema_call(
    schema: &[String],
    helper_name_for: fn(&str) -> String,
    args: &[&Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let (schema_name, qualifier) = schema.split_last()?;
    let mut helper_path = qualifier.to_vec();
    helper_path.push(helper_name_for(schema_name));
    let signature = effect_function_signature_path(
        &helper_path,
        context.uses,
        context.functions,
        context.current_module,
        context.current_package,
        context.companion_access_targets,
    )?;
    if args.len() != signature.params.len() {
        return None;
    }
    for (arg, param) in args.iter().zip(&signature.params) {
        let actual = type_for_effect_call_argument(arg, param, context)?;
        if !crate::type_relations::is_assignable(param, &actual) {
            return None;
        }
    }
    Some(signature.return_type.clone())
}

fn type_for_effect_constructor_call(
    callee: &Expr,
    args: &[Expr],
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let segments = callee.callee_name_path()?;
    let descriptor = context
        .adts
        .descriptor_for_type_prefer_module(expected, context.current_module)?;
    let constructor = context.adts.constructor_for_descriptor(
        segments,
        descriptor,
        context.current_module,
        context.uses,
    )?;
    if args.len() != constructor.variant.payload_fields.len() {
        return None;
    }
    let expected_args = crate::adt::unification::adt_args(expected, descriptor)?;
    let call_bound_row = effect_row_in_type(expected);
    let mut actual_args = vec![Type::Unknown; expected_args.len()];
    let mut payload_comparisons = Vec::with_capacity(args.len());
    for (index, arg) in args.iter().enumerate() {
        let payload_expected = crate::adt::type_operations::payload_type_with_resolved_args(
            constructor,
            index,
            |type_index| expected_args[type_index].clone(),
        )?;
        let actual = type_for_effect_call_argument(arg, &payload_expected, context)?;
        crate::adt::type_operations::merge_type_args_from_payload(
            &mut actual_args,
            constructor,
            index,
            &actual,
        );
        payload_comparisons.push((payload_expected, actual));
    }
    let row_substitutions = solve_call_bound_effect_row(
        call_bound_row.as_deref(),
        payload_comparisons
            .iter()
            .map(|(expected, actual)| (expected, actual)),
    );
    if payload_comparisons.iter().any(|(expected, actual)| {
        !crate::type_relations::is_assignable(
            &instantiate_effect_rows_in_type(expected, &row_substitutions),
            actual,
        )
    }) {
        return None;
    }
    for (actual, expected) in actual_args.iter_mut().zip(expected_args) {
        if *actual == Type::Unknown {
            *actual = expected.clone();
        }
    }
    Some(crate::adt::type_operations::refined_constructed_type_from_args(constructor, &actual_args))
}

pub(super) fn concurrency_effects_for_call(
    segments: &[String],
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let mut effects = concurrency_effects(segments)?
        .iter()
        .map(|effect| (*effect).to_string())
        .collect::<Vec<_>>();
    if matches!(segments, [module, name] if module == "task" && matches!(name.as_str(), "spawn" | "spawn_with"))
        && let Some(job_effects) =
            args.first()
                .and_then(Expr::callee_name_path)
                .and_then(|segments| {
                    effects_for_callee_path(
                        segments,
                        context.uses,
                        context.current_module,
                        context.current_package,
                        context.bindings,
                        context.effects_by_function,
                        context.effects_by_module_path,
                        context.companion_access_targets,
                    )
                })
    {
        for effect in job_effects {
            push_unique_effect(&mut effects, effect);
        }
    }
    Some(effects)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    current_package: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a EffectsByFunction,
    effects_by_module_path: &'a EffectsByModulePath,
    companion_access_targets: &'a BTreeMap<String, String>,
) -> Option<&'a [String]> {
    match segments {
        [name] => effects_for_bare_callee(
            name,
            current_module,
            current_package,
            bindings,
            effects_by_function,
        ),
        [_, .., name] => {
            let use_decl = imported_use_for_path_in_owner(
                uses,
                &segments[..segments.len() - 1],
                current_module,
                current_package,
            )?;
            effects_by_module_path
                .get(&(
                    use_decl.resolved_target().package,
                    Some(use_decl.name.clone()),
                    name.clone(),
                ))
                .filter(|(_, visibility)| {
                    imported_effects_are_visible(
                        use_decl,
                        current_module,
                        use_decl.name.as_str(),
                        *visibility,
                        companion_access_targets,
                    )
                })
                .map(|(effects, _)| effects.as_slice())
        }
        _ => None,
    }
}

pub(super) fn lexical_effects_for_bare_callee<'a>(
    name: &str,
    bindings: &'a [Binding],
    effects_by_function: &'a EffectsByFunction,
    _current_package: Option<&str>,
) -> Option<&'a [String]> {
    let binding = bindings.iter().rev().find(|binding| binding.name == name)?;
    if let Some(target) = &binding.private_function_value {
        return effects_by_function.get(target).map(Vec::as_slice);
    }
    Some(binding.ty.function_effects().unwrap_or(&[]))
}

pub(super) fn imported_effects_are_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (use_decl.importing_package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
                    && target_module.starts_with("std::"))
                    || companion_access_targets
                        .get(current_module)
                        .is_some_and(|allowed| allowed == target_module)
            }))
}

pub(super) fn imported_handler_is_visible(
    handler: &HandlerSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    handler.visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                handler.module_name.as_deref().is_some_and(|target_module| {
                    (use_decl.importing_package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
                        && target_module.starts_with("std::"))
                        || companion_access_targets
                            .get(current_module)
                            .is_some_and(|access| access.target_module == target_module)
                })
            }))
}

pub(crate) fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    collect_companion_access_targets(
        module
            .functions
            .iter()
            .map(|function| (function.span.file.as_str(), function.module_name.as_deref()))
            .chain(
                module
                    .schemas
                    .iter()
                    .map(|schema| (schema.span.file.as_str(), schema.module_name.as_deref())),
            ),
    )
}

pub(super) fn companion_module_value_access_targets(
    module: &SurfaceModule,
) -> HashMap<ResolvedImportOwner, ResolvedImportOwner> {
    module
        .uses
        .iter()
        .filter_map(|use_decl| {
            let (_, target_module) = companion_access_target(
                use_decl.span.file.as_str(),
                use_decl.module_name.as_deref(),
            )?;
            Some((
                use_decl.resolved_owner(),
                ResolvedImportOwner {
                    package: use_decl.importing_package.clone(),
                    module: Some(target_module),
                },
            ))
        })
        .collect()
}

pub(super) fn companion_access_target_infos(
    module: &SurfaceModule,
) -> BTreeMap<String, CompanionAccessTarget> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target_info(
                function.span.file.as_str(),
                function.module_name.as_deref(),
            )
        })
        .chain(module.handlers.iter().filter_map(|handler| {
            companion_access_target_info(handler.span.file.as_str(), handler.module_name.as_deref())
        }))
        .chain(module.effects.iter().filter_map(|effect| {
            companion_access_target_info(effect.span.file.as_str(), effect.module_name.as_deref())
        }))
        .collect()
}

pub(super) fn companion_access_target_info(
    path: &str,
    module_name: Option<&str>,
) -> Option<(String, CompanionAccessTarget)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((
        companion_module,
        CompanionAccessTarget {
            companion_path: companion.companion_path,
            target_module,
        },
    ))
}

pub(super) fn companion_function_access_targets(
    module: &SurfaceModule,
) -> BTreeMap<String, String> {
    collect_companion_access_targets(
        module
            .functions
            .iter()
            .map(|function| (function.span.file.as_str(), function.module_name.as_deref())),
    )
}

pub(super) fn companion_access_targets_for_signatures(
    functions: &[FunctionSignature],
) -> BTreeMap<String, String> {
    collect_companion_access_targets(
        functions
            .iter()
            .map(|function| (function.span.file.as_str(), function.module_name.as_deref())),
    )
}

fn collect_companion_access_targets<'a>(
    declarations: impl IntoIterator<Item = (&'a str, Option<&'a str>)>,
) -> BTreeMap<String, String> {
    declarations
        .into_iter()
        .filter_map(|(path, module_name)| companion_access_target(path, module_name))
        .collect()
}

pub(super) fn effects_for_bare_callee<'a>(
    name: &str,
    current_module: Option<&str>,
    current_package: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a EffectsByFunction,
) -> Option<&'a [String]> {
    if let Some(effects) =
        lexical_effects_for_bare_callee(name, bindings, effects_by_function, current_package)
    {
        return Some(effects);
    }
    if let Some(current_module) = current_module {
        return effects_by_function
            .get(&(
                current_package.map(str::to_string),
                Some(current_module.to_string()),
                name.to_string(),
            ))
            .map(Vec::as_slice);
    }
    effects_by_function
        .get(&(current_package.map(str::to_string), None, name.to_string()))
        .map(Vec::as_slice)
}

pub(super) fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

/// Resolve inference call targets without dropping the importing owner package.
pub(super) fn effect_function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
    current_package: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| {
            function.name == *name
                && function.module_name.as_deref() == current_module
                && function.package_name.as_deref() == current_package
        }),
        [_, .., name] => {
            let use_decl = imported_use_for_path_in_owner(
                uses,
                &segments[..segments.len() - 1],
                current_module,
                current_package,
            )?;
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(use_decl.name.as_str())
                    && function.package_name == use_decl.resolved_target().package
                    && imported_function_is_visible(
                        function,
                        use_decl,
                        current_module,
                        current_package,
                        companion_access_targets,
                    )
            })
        }
        _ => None,
    }
}
