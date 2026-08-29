use super::*;

pub(crate) fn function_body_param_type(param: &veln_ast::Param) -> Type {
    let ty = parse_type_or_unknown(param.ty.as_deref());
    if param.is_variadic {
        Type::named("List", vec![ty])
    } else {
        ty
    }
}

pub(crate) fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
    let companion_access_targets = BTreeMap::new();
    let uses = normal_use_decls(module);
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
                return None;
            }
            if public_alias_has_invalid_target_leaf(
                module,
                alias,
                Some(veln_ast::NameClass::Function),
            ) {
                return None;
            }
            let target = function_signature_path(
                &alias.target,
                &uses,
                functions,
                alias.module_name.as_deref(),
                &companion_access_targets,
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
                variadic: target.variadic.clone(),
                return_type: target.return_type.clone(),
                effects: target.effects.clone(),
                node_id: alias.node_id,
                span: alias.span.clone(),
            })
        })
        .collect()
}

pub(crate) fn function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| function.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(module_name)
                    && imported_function_is_visible(
                        function,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    )
            })
        }
        _ => None,
    }
}

pub(crate) fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    collect_let_pattern_bindings(pattern, ty, None, bindings);
}

pub(crate) fn collect_let_pattern_bindings(
    pattern: &Pattern,
    ty: &Type,
    private_function_value: Option<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            if valid_value_binding_name(name) {
                bindings.push(match private_function_value {
                    Some(target) => {
                        Binding::private_function_value(name.clone(), ty.clone(), target)
                    }
                    None => Binding::new(name.clone(), ty.clone()),
                });
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_let_pattern_bindings(&field.pattern, field_ty, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit
        | PatternKind::Constructor { .. } => {}
    }
}

pub(crate) fn imported_function_is_visible(
    function: &FunctionSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    if function.visibility == Visibility::Public {
        return true;
    }
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                companion_access_targets
                    .get(current_module)
                    .is_some_and(|allowed| allowed == target_module)
            })
        })
}

#[cfg(test)]
pub(crate) mod private_inference_counters {
    use super::Cell;

    thread_local! {
        static BODY_RETURN_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_INDEX_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) body_return_scans: usize,
        pub(crate) call_site_discovery_scans: usize,
        pub(crate) call_site_scans: usize,
        pub(crate) private_reference_candidate_scans: usize,
        pub(crate) private_reference_index_scans: usize,
        pub(crate) prelude_callback_discovery_scans: usize,
        pub(crate) prelude_callback_scans: usize,
    }

    pub(crate) fn reset() {
        BODY_RETURN_SCANS.set(0);
        CALL_SITE_DISCOVERY_SCANS.set(0);
        CALL_SITE_SCANS.set(0);
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(0);
        PRIVATE_REFERENCE_INDEX_SCANS.set(0);
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(0);
        PRELUDE_CALLBACK_SCANS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            body_return_scans: BODY_RETURN_SCANS.get(),
            call_site_discovery_scans: CALL_SITE_DISCOVERY_SCANS.get(),
            call_site_scans: CALL_SITE_SCANS.get(),
            private_reference_candidate_scans: PRIVATE_REFERENCE_CANDIDATE_SCANS.get(),
            private_reference_index_scans: PRIVATE_REFERENCE_INDEX_SCANS.get(),
            prelude_callback_discovery_scans: PRELUDE_CALLBACK_DISCOVERY_SCANS.get(),
            prelude_callback_scans: PRELUDE_CALLBACK_SCANS.get(),
        }
    }

    pub(crate) fn record_body_return_scan() {
        BODY_RETURN_SCANS.set(BODY_RETURN_SCANS.get() + 1);
    }

    pub(crate) fn record_call_site_discovery_scan() {
        CALL_SITE_DISCOVERY_SCANS.set(CALL_SITE_DISCOVERY_SCANS.get() + 1);
    }

    pub(crate) fn record_call_site_scan() {
        CALL_SITE_SCANS.set(CALL_SITE_SCANS.get() + 1);
    }

    pub(crate) fn record_private_reference_candidate_scan() {
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(PRIVATE_REFERENCE_CANDIDATE_SCANS.get() + 1);
    }

    pub(crate) fn record_private_reference_index_scan() {
        PRIVATE_REFERENCE_INDEX_SCANS.set(PRIVATE_REFERENCE_INDEX_SCANS.get() + 1);
    }

    pub(crate) fn record_prelude_callback_discovery_scan() {
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(PRELUDE_CALLBACK_DISCOVERY_SCANS.get() + 1);
    }

    pub(crate) fn record_prelude_callback_scan() {
        PRELUDE_CALLBACK_SCANS.set(PRELUDE_CALLBACK_SCANS.get() + 1);
    }
}
