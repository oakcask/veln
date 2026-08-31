use super::*;

pub(super) fn target_visible_from_current_module(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    let target_module = target.module_name.as_deref();
    if current_module.is_none() || target_module == current_module {
        return true;
    }
    target_module.is_some_and(|module_name| {
        uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && use_decl.origin == veln_ast::UseOrigin::Source
                && use_decl.name == module_name
                && imported_target_visible_from_module(
                    target,
                    use_decl,
                    current_module,
                    companion_access_targets,
                )
        })
    })
}

pub(super) fn function_type_shape(annotation: &str) -> Option<FunctionShape> {
    let params = annotation.trim().strip_prefix("fn")?.trim_start();
    let params = params.strip_prefix('(')?;
    let mut depth = 0usize;
    let mut split_at = None;
    for (index, ch) in params.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' if depth == 0 => {
                split_at = Some(index);
                break;
            }
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let params = &params[..split_at?].trim();
    if params.is_empty() {
        return Some(FunctionShape {
            fixed_arity: 0,
            variadic: None,
        });
    }
    let mut parts = split_top_level_commas(params);
    let variadic = parts.last().and_then(|last| {
        last.strip_prefix("...")
            .map(str::trim)
            .filter(|element| !element.is_empty())
            .map(str::to_string)
    });
    if variadic.is_some() {
        parts.pop();
    }
    Some(FunctionShape {
        fixed_arity: parts.len(),
        variadic,
    })
}

pub(super) fn split_top_level_commas(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

pub(super) fn path_has_valid_constructor(
    segments: &[String],
    arg_count: Option<usize>,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    types: &[&veln_ast::TypeDecl],
) -> bool {
    let target = visible_path_target(uses, segments, current_module);
    let leaf = path_leaf(segments);
    types.iter().copied().any(|type_decl| {
        declaration_visible(
            type_decl.module_name.as_deref(),
            type_decl.visibility,
            target.as_deref(),
            current_module,
        ) && type_decl.variants.iter().any(|variant| {
            variant.name.as_deref() == leaf
                && variant
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                && arg_count.is_none_or(|count| variant.fields.len() == count)
        })
    })
}

pub(super) fn collect_function_name_reference(
    segments: &[String],
    context: &FunctionCalleeContext<'_>,
    local_bindings: &[LocalBinding],
    arg_count: Option<usize>,
    callees: &mut Vec<ReachableFunction>,
) {
    let current_module = context.current_module;
    let uses = context.uses;
    let function_targets = context.function_targets;
    let companion_access_targets = context.companion_access_targets;
    let types = context.types;

    if let [name] = segments
        && let Some(binding) = local_bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
    {
        if let Some(shape) = &binding.function_shape {
            collect_opaque_function_value_callees(
                shape,
                arg_count,
                current_module,
                uses,
                function_targets,
                companion_access_targets,
                callees,
            );
        }
        return;
    }
    if path_has_valid_constructor(segments, None, current_module, uses, types) {
        return;
    }
    let public_or_same_module_access;
    let access_targets = if arg_count.is_some() {
        companion_access_targets
    } else {
        public_or_same_module_access = HashMap::new();
        &public_or_same_module_access
    };
    for callee in resolve_function_reference(
        segments,
        current_module,
        uses,
        function_targets,
        access_targets,
        arg_count,
    ) {
        push_reachable(callees, callee);
    }
}

pub(super) fn collect_handler_operation_clause_callees(
    expr: &Expr,
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    handlers: &[&veln_ast::HandlerDecl],
    callees: &mut Vec<ReachableFunction>,
) {
    let ExprKind::Handle { handler, .. } = &expr.kind else {
        return;
    };
    let matching_handlers = handlers.iter().filter(|candidate| {
        let Some(name) = &candidate.name else {
            return false;
        };
        match handler.as_slice() {
            [segment] => name == segment && candidate.module_name.as_deref() == current_module,
            [_, .., segment] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &handler[..handler.len() - 1], current_module)
                else {
                    return false;
                };
                name == segment && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
            }
            _ => false,
        }
    });
    for handler in matching_handlers {
        let context = FunctionCalleeContext {
            current_module,
            uses,
            function_targets,
            companion_access_targets,
            handlers,
            types: &[],
        };
        let mut local_bindings = handler
            .params
            .iter()
            .map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: param.ty.as_deref().and_then(function_type_shape),
            })
            .collect::<Vec<_>>();
        for clause in &handler.operation_clauses {
            let binding_count = local_bindings.len();
            local_bindings.extend(clause.params.iter().map(|param| LocalBinding {
                name: param.name.clone(),
                function_shape: None,
            }));
            collect_function_callees(&clause.body, &context, &local_bindings, callees);
            local_bindings.truncate(binding_count);
        }
    }
}

pub(super) fn resolve_function_reference(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[&UseDecl],
    function_targets: &FunctionTargetIndex,
    companion_access_targets: &HashMap<String, String>,
    arg_count: Option<usize>,
) -> Vec<ReachableFunction> {
    match segments {
        [name] => function_targets
            .named(name)
            .filter(|target| {
                #[cfg(test)]
                reachability_counters::record_target_resolution_scan();
                target.name == *name
                    && bare_target_visible(target, current_module, uses)
                    && recovery_target_accepts_arg_count(target, arg_count)
            })
            .map(reachable_function_from_target)
            .collect(),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return Vec::new();
            };
            let module_name = use_decl.name.as_str();
            function_targets
                .qualified(module_name, name)
                .filter(|target| {
                    #[cfg(test)]
                    reachability_counters::record_target_resolution_scan();
                    imported_target_visible_from_module(
                        target,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ) && recovery_target_accepts_arg_count(target, arg_count)
                })
                .map(reachable_function_from_target)
                .collect()
        }
        _ => Vec::new(),
    }
}

fn reachable_function_from_target(target: &FunctionTarget) -> ReachableFunction {
    ReachableFunction {
        kind: FunctionKind::Function,
        name: target.target_name.clone(),
        module_name: target.target_module_name.clone(),
        node_id: Some(target.target_node_id),
    }
}

pub(super) fn recovery_target_accepts_arg_count(
    target: &FunctionTarget,
    arg_count: Option<usize>,
) -> bool {
    !target.recovery || arg_count.is_none_or(|count| target.shape.accepts_arg_count(count))
}

pub(super) fn imported_use_for_path<'a>(
    uses: &'a [&'a UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().copied().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path
                || simple_import_alias_matches(use_decl, &module_path)
                || standard_package_relative_import_matches(use_decl, &module_path, current_module))
    })
}

fn simple_import_alias_matches(use_decl: &UseDecl, module_path: &str) -> bool {
    use_decl.alias == module_path
        && (use_decl.package.is_some()
            || !use_decl.name.contains("::")
            || use_decl.origin == veln_ast::UseOrigin::ImplicitStandardPrelude)
}

fn standard_package_relative_import_matches(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    (use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || (use_decl.package.is_none()
            && current_module.is_some_and(|module| module.starts_with("std::"))))
        && use_decl
            .name
            .strip_prefix("std::")
            .is_some_and(|package_relative| package_relative == module_path)
}

pub(super) fn imported_target_is_visible(target: &FunctionTarget, use_decl: &UseDecl) -> bool {
    if target.requires_public_import {
        return target.visibility == Visibility::Public;
    }
    use_decl.package.is_none() || target.visibility == Visibility::Public
}

pub(super) fn imported_target_visible_from_module(
    target: &FunctionTarget,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    if target.recovery {
        return false;
    }
    if target.visibility == Visibility::Public {
        return true;
    }
    if target.requires_public_import || use_decl.package.is_some() {
        return false;
    }
    if current_module.is_some_and(|module| module.starts_with("std::"))
        && target
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    current_module.is_some_and(|current_module| {
        target.module_name.as_ref().is_some_and(|target_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed_target| allowed_target == target_module)
        })
    })
}

pub(super) fn companion_function_access_targets(
    inputs: &ReachabilityInputs<'_>,
) -> HashMap<String, String> {
    inputs
        .functions()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .collect()
}

pub(super) fn bare_target_visible(
    target: &FunctionTarget,
    current_module: Option<&str>,
    uses: &[&UseDecl],
) -> bool {
    let Some(current_module) = current_module else {
        return true;
    };
    if target.module_name.as_deref() == Some(current_module) {
        return true;
    }
    if target.recovery {
        return false;
    }
    target.bare_importable
        && target.module_name.as_deref().is_some_and(|module_name| {
            uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == Some(current_module)
                    && use_decl.name == module_name
                    && imported_target_is_visible(target, use_decl)
            })
        })
}

pub(super) fn push_reachable(callees: &mut Vec<ReachableFunction>, callee: ReachableFunction) {
    if !callees.iter().any(|known| known == &callee) {
        callees.push(callee);
    }
}
