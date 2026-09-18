use super::*;
use std::ops::ControlFlow;

pub(crate) fn private_function_mentions_candidate(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
) -> bool {
    visit_private_function_references(function, function_by_path, &mut |key| {
        if key.0.as_deref() == function.module_name.as_deref() && candidates.contains(&key.1) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    })
    .is_break()
}

pub(crate) fn private_reference_initial_bindings(function: &Function) -> Vec<Binding> {
    function_parameter_bindings(function)
}

pub(crate) fn collect_private_reference_pattern_bindings(
    pattern: &Pattern,
    bindings: &mut Vec<Binding>,
) {
    pattern.for_each_binding(&mut |name| {
        if valid_value_binding_name(name) {
            bindings.push(Binding::new(name.to_string(), Type::Unknown));
        }
    });
}

pub(crate) fn collect_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
) {
    let _ = visit_private_function_references(function, function_by_path, &mut |key| {
        references.insert(key);
        ControlFlow::Continue(())
    });
}

fn visit_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    visitor: &mut impl FnMut(FunctionKey) -> ControlFlow<()>,
) -> ControlFlow<()> {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        visit_private_line_references(
            line,
            current_module,
            function_by_path,
            &mut bindings,
            visitor,
        )?;
    }
    ControlFlow::Continue(())
}

fn visit_private_line_references(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &mut Vec<Binding>,
    visitor: &mut impl FnMut(FunctionKey) -> ControlFlow<()>,
) -> ControlFlow<()> {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            visit_private_expr_references(
                expr,
                current_module,
                function_by_path,
                bindings,
                visitor,
            )?;
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
            ControlFlow::Continue(())
        }
        BodyLineKind::Expr { expr } => {
            visit_private_expr_references(expr, current_module, function_by_path, bindings, visitor)
        }
    }
}

fn visit_private_expr_references(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
    visitor: &mut impl FnMut(FunctionKey) -> ControlFlow<()>,
) -> ControlFlow<()> {
    if let Some(key) =
        private_expr_reference_target(expr, current_module, function_by_path, bindings)
    {
        visitor(key)?;
    }
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            visit_private_expr_references(
                scrutinee,
                current_module,
                function_by_path,
                bindings,
                visitor,
            )?;
            for arm in arms {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                visit_private_expr_references(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    &arm_bindings,
                    visitor,
                )?;
            }
            ControlFlow::Continue(())
        }
        // Explicitly instantiated callees do not contribute private inference constraints.
        ExprKind::TypeApply { .. } => ControlFlow::Continue(()),
        _ => expr.try_for_each_child(&mut |child| {
            visit_private_expr_references(
                child,
                current_module,
                function_by_path,
                bindings,
                visitor,
            )
        }),
    }
}

pub(crate) fn private_expr_reference_target(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let ExprKind::NamePath { segments, .. } = &expr.kind else {
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

#[cfg(test)]
#[path = "reference_discovery_tests.rs"]
mod tests;
