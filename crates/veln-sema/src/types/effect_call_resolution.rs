use super::*;
use crate::effect_rows::{collect_effect_row_substitution, instantiate_effect_rows};

pub(super) fn instantiate_call_effect_rows(
    signature: &FunctionSignature,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Vec<String> {
    let mut row_substitutions = Vec::<(String, Vec<String>)>::new();
    for (param, arg) in signature.params.iter().zip(args) {
        let Some(actual) = function_type_for_expr(arg, context) else {
            continue;
        };
        collect_effect_row_substitution(param, &actual, &mut row_substitutions);
    }
    instantiate_effect_rows(&signature.effects, &row_substitutions)
}

pub(super) fn function_type_for_expr(expr: &Expr, context: &ExprEffectContext<'_>) -> Option<Type> {
    let segments = callee_name_path(expr)?;
    match segments.as_slice() {
        [name] => context
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                function_signature_path(
                    segments,
                    context.uses,
                    context.functions,
                    context.current_module,
                    context.companion_access_targets,
                )
                .map(FunctionSignature::ty)
            }),
        _ => {
            let public_or_same_module_access = BTreeMap::new();
            function_signature_path(
                segments,
                context.uses,
                context.functions,
                context.current_module,
                &public_or_same_module_access,
            )
            .map(FunctionSignature::ty)
        }
    }
}

pub(super) fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
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
        && let Some(job_effects) = args
            .first()
            .and_then(callee_name_path)
            .and_then(|segments| {
                effects_for_callee_path(
                    segments,
                    context.uses,
                    context.current_module,
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

pub(super) fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
) -> Option<&'a [String]> {
    match segments {
        [name] => effects_for_bare_callee(name, current_module, bindings, effects_by_function),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            effects_by_module_path
                .get(&(use_decl.name.clone(), name.clone()))
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
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
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
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
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
                    (current_module.starts_with("std::") && target_module.starts_with("std::"))
                        || companion_access_targets
                            .get(current_module)
                            .is_some_and(|access| access.target_module == target_module)
                })
            }))
}

pub(super) fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .chain(module.schemas.iter().filter_map(|schema| {
            companion_access_target(schema.span.file.as_str(), schema.module_name.as_deref())
        }))
        .collect()
}

pub(super) fn companion_access_target(
    path: &str,
    module_name: Option<&str>,
) -> Option<(String, String)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((companion_module, target_module))
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
    module
        .functions
        .iter()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

pub(super) fn companion_access_targets_for_signatures(
    functions: &[FunctionSignature],
) -> BTreeMap<String, String> {
    functions
        .iter()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

pub(super) fn effects_for_bare_callee<'a>(
    name: &str,
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
) -> Option<&'a [String]> {
    if let Some(effects) = lexical_effects_for_bare_callee(name, bindings, effects_by_function) {
        return Some(effects);
    }
    if let Some(current_module) = current_module {
        return effects_by_function
            .get(&(Some(current_module.to_string()), name.to_string()))
            .map(Vec::as_slice);
    }
    effects_by_function
        .get(&(None, name.to_string()))
        .map(Vec::as_slice)
}

pub(super) fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}
