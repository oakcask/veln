use super::*;

pub(crate) fn private_function_mentions_candidate(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
) -> bool {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        if private_line_mentions_candidate(
            line,
            current_module,
            function_by_path,
            candidates,
            &mut bindings,
        ) {
            return true;
        }
    }
    false
}

pub(crate) fn private_line_mentions_candidate(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &mut Vec<Binding>,
) -> bool {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            let mentions = private_expr_mentions_candidate(
                expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
            mentions
        }
        BodyLineKind::Expr { expr } => private_expr_mentions_candidate(
            expr,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
    }
}

pub(crate) fn private_expr_mentions_candidate(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &[Binding],
) -> bool {
    if private_expr_reference_target(expr, current_module, function_by_path, bindings)
        .is_some_and(|key| key.0.as_deref() == current_module && candidates.contains(&key.1))
    {
        return true;
    }
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            private_expr_mentions_candidate(
                item,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            private_expr_mentions_candidate(
                &entry.key,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                &entry.value,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            private_expr_mentions_candidate(
                &field.expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Call { callee, args } => {
            private_expr_mentions_candidate(
                callee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::Perform { args, .. } => args.iter().any(|arg| {
            private_expr_mentions_candidate(
                arg,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Handle { body, args, .. } => {
            private_expr_mentions_candidate(
                body,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_expr_mentions_candidate(
                input,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                base,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => private_expr_mentions_candidate(
            value,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
        ExprKind::Match { scrutinee, arms } => {
            private_expr_mentions_candidate(
                scrutinee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || arms.iter().any(|arm| {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                private_expr_mentions_candidate(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    &arm_bindings,
                )
            })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_expr_mentions_candidate(
                condition,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                then_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || else_if_branches.iter().any(|branch| {
                private_expr_mentions_candidate(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                ) || private_expr_mentions_candidate(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            }) || private_expr_mentions_candidate(
                else_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::Binary { left, right, .. } => {
            private_expr_mentions_candidate(
                left,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                right,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

pub(crate) fn private_reference_initial_bindings(function: &Function) -> Vec<Binding> {
    function
        .params
        .iter()
        .filter(|param| valid_value_binding_name(&param.name))
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect()
}

pub(crate) fn collect_private_reference_pattern_bindings(
    pattern: &Pattern,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            if valid_value_binding_name(name) {
                bindings.push(Binding::new(name.clone(), Type::Unknown));
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                collect_private_reference_pattern_bindings(&field.pattern, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_private_reference_pattern_bindings(arg, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

pub(crate) fn collect_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
) {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        collect_private_line_references(
            line,
            current_module,
            function_by_path,
            references,
            &mut bindings,
        );
    }
}

pub(crate) fn collect_private_line_references(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            collect_private_expr_references(
                expr,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
        }
        BodyLineKind::Expr { expr } => collect_private_expr_references(
            expr,
            current_module,
            function_by_path,
            references,
            bindings,
        ),
    }
}

pub(crate) fn collect_private_expr_references(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &[Binding],
) {
    if let Some(key) =
        private_expr_reference_target(expr, current_module, function_by_path, bindings)
    {
        references.insert(key);
    }
    match &expr.kind {
        ExprKind::List(items) => {
            for item in items {
                collect_private_expr_references(
                    item,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_private_expr_references(
                    &entry.key,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &entry.value,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_private_expr_references(
                    &field.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_expr_references(
                callee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_expr_references(
                body,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_expr_references(
                input,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                base,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            collect_private_expr_references(
                value,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_expr_references(
                scrutinee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arm in arms {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                collect_private_expr_references(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    references,
                    &arm_bindings,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_expr_references(
                condition,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                then_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for branch in else_if_branches {
                collect_private_expr_references(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
            collect_private_expr_references(
                else_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_expr_references(
                left,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                right,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

pub(crate) fn private_expr_reference_target(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &expr.kind else {
        return None;
    };
    private_reference_name_path_target(segments, current_module, function_by_path, bindings)
}

pub(crate) fn private_reference_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == *name) {
        return binding.private_function_value.clone();
    }
    private_name_path_target(segments, current_module, function_by_path)
}

pub(crate) fn signatures_by_path(functions: &[FunctionSignature]) -> FunctionSignatureMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.clone(),
            )
        })
        .collect()
}

pub(crate) fn private_call_site_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_slots: &PrivateSlotMap,
    private_references: &PrivateReferenceMap,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_slots = omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            #[cfg(test)]
            private_inference_counters::record_call_site_discovery_scan();
            (omitted_private_slots.contains_key(&key)
                || private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_slots.contains_key(reference))
                }))
            .then_some(key)
        })
        .collect()
}

pub(crate) fn signatures_by_path_with_aliases(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> FunctionSignatureMap {
    let mut signatures = signatures_by_path(functions);
    for alias in function_alias_signatures(module, functions) {
        let key = (alias.module_name.clone(), alias.name.clone());
        signatures.entry(key).or_insert(alias);
    }
    signatures
}

pub(crate) fn returns_by_path(functions: &[FunctionSignature]) -> FunctionReturnMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect()
}
