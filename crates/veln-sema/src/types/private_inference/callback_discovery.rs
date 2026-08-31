use super::*;

pub(crate) fn infer_private_prelude_callback_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let uses = normal_use_decls(module);
    let function_by_path = function_ast_map(module);
    let mut returns_by_path = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let initial_omitted_private_returns =
        omitted_private_returns_requiring_prelude_pass(module, functions, &uses, adts);
    if initial_omitted_private_returns.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_return_omissions(&initial_omitted_private_returns),
        &initial_omitted_private_returns,
    );
    let contributors = private_prelude_callback_constraint_contributors(
        module,
        &initial_omitted_private_returns,
        &returns_by_path,
        &function_by_path,
        &private_references,
        &uses,
        adts,
    );
    if contributors.is_empty() {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_returns = initial_omitted_private_returns.clone();
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_prelude_callback_return_constraints(
                function,
                &uses,
                &function_by_path,
                &omitted_private_returns,
                &mut returns_by_path,
                adts,
                &mut changed,
            );
        }
        for function in functions.iter_mut() {
            let key = (function.module_name.clone(), function.name.clone());
            if !omitted_private_returns.contains(&key) {
                continue;
            }
            if let Some(inferred) = returns_by_path.get(&key)
                && inferred != &function.return_type
            {
                function.return_type = inferred.clone();
            }
        }
    }
}

pub(crate) fn collect_private_prelude_callback_return_constraints(
    function: &Function,
    uses: &[UseDecl],
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
    omitted_private_returns: &BTreeSet<(Option<String>, String)>,
    returns_by_path: &mut BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    changed: &mut bool,
) {
    #[cfg(test)]
    private_inference_counters::record_prelude_callback_scan();

    let mut bindings = function_parameter_bindings(function);
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
                collect_private_callback_let_bindings(
                    pattern,
                    expr,
                    annotation_type,
                    &PrivateCallbackBindingContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        function_by_path,
                        returns_by_path,
                        adts,
                    },
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    expected,
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
            }
        }
    }
}

pub(crate) fn private_prelude_callback_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    private_references: &PrivateReferenceMap,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_returns = omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_returns.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            if !omitted_private_returns.contains(&key)
                && !private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_returns.contains(reference))
                })
            {
                return None;
            }
            private_prelude_callback_function_can_constrain(
                function,
                &key,
                omitted_private_returns,
                returns_by_path,
                function_by_path,
                uses,
                adts,
            )
            .then_some(key)
        })
        .collect()
}

pub(crate) fn private_prelude_callback_function_can_constrain(
    function: &Function,
    key: &FunctionKey,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    if omitted_private_returns.contains(key)
        && returns_by_path.get(key).is_some_and(|return_type| {
            private_tail_can_use_expected(function, return_type, uses, adts)
        })
    {
        return true;
    }

    #[cfg(test)]
    private_inference_counters::record_prelude_callback_discovery_scan();

    let mut bindings = function_parameter_bindings(function);
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(
                    expr,
                    annotation_type.as_ref(),
                    &reference_context,
                ) {
                    return true;
                }
                collect_private_callback_let_bindings(
                    pattern,
                    expr,
                    annotation_type,
                    &PrivateCallbackBindingContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        function_by_path,
                        returns_by_path,
                        adts,
                    },
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(expr, expected, &reference_context)
                {
                    return true;
                }
            }
        }
    }
    false
}

struct PrivateCallbackBindingContext<'a, 'function> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    function_by_path: &'a FunctionAstMap<'function>,
    returns_by_path: &'a FunctionReturnMap,
    adts: &'a AdtRegistry,
}

fn collect_private_callback_let_bindings(
    pattern: &Pattern,
    expression: &Expr,
    annotation_type: Option<Type>,
    context: &PrivateCallbackBindingContext<'_, '_>,
    bindings: &mut Vec<Binding>,
) {
    let initializer_private_function = annotation_type
        .is_none()
        .then(|| {
            private_same_module_call_target(
                expression,
                context.current_module,
                context.function_by_path,
            )
        })
        .flatten();
    let ty = annotation_type.unwrap_or_else(|| {
        infer_private_signature_expr_type(
            expression,
            None,
            context.current_module,
            context.uses,
            bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    collect_let_pattern_bindings(pattern, &ty, initializer_private_function, bindings);
}

pub(crate) fn private_prelude_callback_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    if let ExprKind::NamePath { segments, .. } = &expr.kind
        && expected.is_some_and(|expected| {
            private_callback_return_constraint_can_update(segments, expected, context)
        })
    {
        return true;
    }
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            let direct_reference =
                private_prelude_callback_call_references_slot(callee, args, expected, context);
            direct_reference
                || !matches!(callee.kind, ExprKind::NamePath { segments: _, .. })
                    && private_prelude_callback_expr_references_slot(callee, None, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::List(_) | ExprKind::Dict(_) | ExprKind::Record(_) => {
            private_prelude_callback_collection_references_slot(expr, expected, context)
        }
        ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try(_)
        | ExprKind::Prefix { .. } => {
            private_prelude_callback_wrapped_expr_references_slot(expr, expected, context)
        }
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Binary { .. } => {
            private_prelude_callback_control_flow_references_slot(expr, expected, context)
        }
        ExprKind::NamePath { segments: _, .. }
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

pub(crate) fn private_prelude_callback_collection_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
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

pub(crate) fn private_prelude_callback_wrapped_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Perform { args, .. } => args
            .iter()
            .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context)),
        ExprKind::Handle { body, args, .. } => {
            private_prelude_callback_expr_references_slot(body, expected, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_prelude_callback_expr_references_slot(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            ) || private_prelude_callback_expr_references_slot(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            private_prelude_callback_expr_references_slot(value, None, context)
        }
        _ => false,
    }
}

pub(crate) fn private_prelude_callback_control_flow_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            private_prelude_callback_expr_references_slot(scrutinee, None, context)
                || arms.iter().any(|arm| {
                    private_prelude_callback_expr_references_slot(&arm.expr, expected, context)
                })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_prelude_callback_expr_references_slot(condition, Some(&Type::bool()), context)
                || private_prelude_callback_expr_references_slot(then_branch, expected, context)
                || else_if_branches.iter().any(|branch| {
                    private_prelude_callback_expr_references_slot(
                        &branch.condition,
                        Some(&Type::bool()),
                        context,
                    ) || private_prelude_callback_expr_references_slot(
                        &branch.expr,
                        expected,
                        context,
                    )
                })
                || private_prelude_callback_expr_references_slot(else_branch, expected, context)
        }
        ExprKind::Binary { left, right, .. } => {
            private_prelude_callback_expr_references_slot(left, expected, context)
                || private_prelude_callback_expr_references_slot(right, expected, context)
        }
        _ => false,
    }
}

pub(crate) fn private_prelude_callback_call_references_slot(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let Some(params) = private_prelude_callback_call_params(
        callee,
        args,
        expected,
        &PrivateSignatureInferContext {
            current_module: context.current_module,
            uses: context.uses,
            bindings: context.bindings,
            returns_by_path: context.returns_by_path,
            adts: context.adts,
        },
        context.function_by_path,
    ) else {
        return false;
    };
    args.iter()
        .zip(params.iter())
        .any(|(arg, param)| private_prelude_callback_arg_references_slot(arg, param, context))
}

pub(crate) struct PrivatePreludeCallbackReferenceContext<'a> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) bindings: &'a [Binding],
    pub(crate) omitted_private_returns: &'a BTreeSet<FunctionKey>,
    pub(crate) returns_by_path: &'a FunctionReturnMap,
    pub(crate) function_by_path: &'a FunctionAstMap<'a>,
    pub(crate) adts: &'a AdtRegistry,
}

pub(crate) fn private_prelude_callback_arg_references_slot(
    expr: &Expr,
    expected: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::NamePath { segments, .. } => {
            private_callback_return_constraint_can_update(segments, expected, context)
        }
        _ => private_prelude_callback_expr_references_slot(expr, Some(expected), context),
    }
}
