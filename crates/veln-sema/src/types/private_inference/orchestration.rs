use super::*;

pub(crate) fn infer_private_function_body_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let uses = normal_use_decls(module);
    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let omitted_private_returns = omitted_private_returns_that_can_change(module, functions);
        if omitted_private_returns.is_empty() {
            return;
        }
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && private_function_key(function)
                    .is_some_and(|key| omitted_private_returns.contains(&key))
        }) {
            let Some(name) = &function.name else {
                continue;
            };
            let key = (function.module_name.clone(), name.clone());
            let inferred = infer_private_function_tail_type(
                function,
                &uses,
                &signatures_by_path,
                &returns_by_path,
                adts,
            );
            if inferred == Type::Unknown {
                continue;
            }
            let Some(signature) = functions
                .iter_mut()
                .find(|signature| signature.module_name == key.0 && signature.name == key.1)
            else {
                continue;
            };
            if signature.return_type == inferred {
                continue;
            }
            if !type_has_unknown(&signature.return_type) {
                continue;
            }
            signature.return_type = inferred;
            changed = true;
        }
    }
}

pub(crate) fn infer_private_function_call_site_signature_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let uses = normal_use_decls(module);
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let initial_omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
    if initial_omitted_private_slots.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_slot_omissions(&initial_omitted_private_slots),
        &initial_omitted_private_slots
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
    );
    let contributors = private_call_site_constraint_contributors(
        module,
        &initial_omitted_private_slots,
        &private_references,
    );
    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
        if omitted_private_slots.is_empty() {
            return;
        }
        let signatures_by_path = signatures_by_path_with_aliases(module, functions);
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_call_site_constraints(
                function,
                &mut PrivateCallSiteConstraintContext {
                    uses: &uses,
                    function_by_path: &function_by_path,
                    omitted_private_slots: &omitted_private_slots,
                    signatures_by_path: &signatures_by_path,
                    returns_by_path: &returns_by_path,
                    functions,
                    adts,
                    changed: &mut changed,
                },
            );
        }
    }
}

pub(crate) fn function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

pub(crate) fn private_function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

pub(crate) fn signature_for_key<'a>(
    functions: &'a [FunctionSignature],
    key: &FunctionKey,
) -> Option<&'a FunctionSignature> {
    functions
        .iter()
        .find(|signature| signature.module_name == key.0 && signature.name == key.1)
}

pub(crate) fn omitted_private_returns_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let can_change = signature_for_key(functions, &key)
                .is_some_and(|signature| type_has_unknown(&signature.return_type));
            can_change.then_some(key)
        })
        .collect()
}

pub(crate) fn omitted_private_slots_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> PrivateSlotMap {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.name.is_some()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            let omitted_params = function
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    if !parameter_annotation_is_omitted(param) {
                        return false;
                    }
                    if param.is_variadic {
                        signature.variadic.as_ref().is_some_and(type_has_unknown)
                    } else {
                        signature.params.get(index).is_some_and(type_has_unknown)
                    }
                })
                .collect::<Vec<_>>();
            let omitted_return =
                function.return_type.is_none() && type_has_unknown(&signature.return_type);
            (omitted_params.iter().any(|omitted| *omitted) || omitted_return)
                .then_some((key, (omitted_params, omitted_return)))
        })
        .collect()
}

pub(crate) fn modules_with_private_slot_omissions(
    omitted_private_slots: &PrivateSlotMap,
) -> BTreeSet<Option<String>> {
    omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect()
}

pub(crate) fn modules_with_private_return_omissions(
    omitted_private_returns: &BTreeSet<FunctionKey>,
) -> BTreeSet<Option<String>> {
    omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect()
}

pub(crate) fn omitted_private_returns_requiring_prelude_pass(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            (type_has_unknown(&signature.return_type)
                || private_tail_can_use_expected(function, &signature.return_type, uses, adts))
            .then_some(key)
        })
        .collect()
}

pub(crate) fn private_reference_map(
    module: &SurfaceModule,
    function_by_path: &FunctionAstMap<'_>,
    modules_with_omitted_slots: &BTreeSet<Option<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> PrivateReferenceMap {
    let candidates_by_module = private_reference_candidates_by_module(omitted_private_keys);
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter(|function| {
            private_function_needs_reference_index(
                function,
                function_by_path,
                &candidates_by_module,
                omitted_private_keys,
            )
        })
        .filter_map(|function| {
            let key = function_key(function)?;
            let mut references = BTreeSet::new();
            #[cfg(test)]
            private_inference_counters::record_private_reference_index_scan();
            collect_private_function_references(function, function_by_path, &mut references);
            Some((key, references))
        })
        .collect()
}

pub(crate) fn private_reference_candidates_by_module(
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> BTreeMap<Option<String>, BTreeSet<String>> {
    let mut candidates: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();
    for (module_name, name) in omitted_private_keys {
        candidates
            .entry(module_name.clone())
            .or_default()
            .insert(name.clone());
    }
    candidates
}

pub(crate) fn private_function_needs_reference_index(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates_by_module: &BTreeMap<Option<String>, BTreeSet<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> bool {
    let Some(key) = function_key(function) else {
        return false;
    };
    if omitted_private_keys.contains(&key) {
        return true;
    }
    let Some(candidates) = candidates_by_module.get(&function.module_name) else {
        return false;
    };
    #[cfg(test)]
    private_inference_counters::record_private_reference_candidate_scan();
    private_function_mentions_candidate(function, function_by_path, candidates)
}
