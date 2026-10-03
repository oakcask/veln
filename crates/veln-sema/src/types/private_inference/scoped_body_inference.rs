use super::*;

pub(crate) fn private_tail_can_use_expected(
    function: &Function,
    expected: &Type,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let Some(BodyLineKind::Expr { expr }) = function.body.last().map(|line| &line.kind) else {
        return false;
    };
    tail_expr_can_use_expected(expr, expected, function.module_name.as_deref(), uses, adts)
}

pub(crate) fn tail_expr_can_use_expected(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::List(_) => expected.vec_part().is_some(),
        ExprKind::Dict(_) => expected.dict_parts().is_some(),
        ExprKind::Record(fields) => record_tail_can_use_expected(fields, expected),
        ExprKind::NamePath { segments, .. } => {
            match adts.nullary_constructor(segments, current_module, uses) {
                ConstructorLookup::Found(constructor) => {
                    unification::adt_args(expected, constructor.descriptor).is_some()
                }
                ConstructorLookup::Ambiguous => adts
                    .descriptor_for_type_prefer_module(expected, current_module)
                    .and_then(|descriptor| {
                        adts.constructor_for_descriptor(segments, descriptor, current_module, uses)
                    })
                    .is_some_and(|constructor| constructor.variant.payload_fields.is_empty()),
                ConstructorLookup::Missing => false,
            }
        }
        ExprKind::Call { callee, .. } => {
            constructor_tail_can_use_expected(callee, expected, current_module, uses, adts)
        }
        ExprKind::Match { arms, .. } => arms
            .iter()
            .all(|arm| tail_expr_can_use_expected(&arm.expr, expected, current_module, uses, adts)),
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => std::iter::once(then_branch.as_ref())
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch.as_ref()))
            .all(|branch| tail_expr_can_use_expected(branch, expected, current_module, uses, adts)),
        ExprKind::Begin { body, .. } => matches!(
            body.last().map(|line| &line.kind),
            Some(BodyLineKind::Expr { expr })
                if tail_expr_can_use_expected(expr, expected, current_module, uses, adts)
        ),
        _ => false,
    }
}

fn record_tail_can_use_expected(fields: &[RecordField], expected: &Type) -> bool {
    if fields.is_empty() {
        return expected.dict_parts().is_some();
    }
    fields
        .iter()
        .all(|field| expected.record_field(&field.name).is_some())
}

fn constructor_tail_can_use_expected(
    callee: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return false;
    };
    matches!(
        adts.constructor(segments, current_module, uses),
        ConstructorLookup::Found(constructor)
            if unification::adt_args(expected, constructor.descriptor).is_some()
    )
}

pub(crate) fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    #[cfg(test)]
    private_inference_counters::record_body_return_scan();

    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    infer_private_body_type(
        &function.body,
        None,
        function.module_name.as_deref(),
        uses,
        &mut bindings,
        returns_by_path,
        adts,
    )
}

pub(super) fn infer_private_body_type(
    body: &[BodyLine],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let mut tail = Type::unit();
    for (index, line) in body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
                ..
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        current_module,
                        uses,
                        bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, bindings);
                tail = Type::unit();
            }
            BodyLineKind::Expr { expr } => {
                tail = infer_private_signature_expr_type(
                    expr,
                    (index + 1 == body.len()).then_some(expected).flatten(),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
            }
            BodyLineKind::Defer { body, .. } => {
                let binding_count = bindings.len();
                record_scoped_binding_count(bindings);
                infer_private_body_type(
                    body,
                    Some(&Type::unit()),
                    current_module,
                    uses,
                    bindings,
                    returns_by_path,
                    adts,
                );
                bindings.truncate(binding_count);
                tail = Type::unit();
            }
        }
    }
    tail
}

pub(crate) fn private_function_body_bindings(
    function: &veln_ast::Function,
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
) -> PrivateBindings {
    let signature = function
        .name
        .as_ref()
        .and_then(|name| signatures_by_path.get(&(function.module_name.clone(), name.clone())));
    let mut bindings = PrivateBindings::for_function(function);
    if function.callsite.is_some() {
        bindings.push(Binding::builtin_callsite());
    }
    bindings.extend(
        function
            .params
            .iter()
            .enumerate()
            .filter(|(_, param)| {
                valid_value_binding_name(&param.name)
                    && !(function.callsite.is_some() && param.name == "callsite")
            })
            .map(|(index, param)| {
                let ty = if param.is_variadic {
                    signature
                        .and_then(|signature| signature.variadic.clone())
                        .map(|ty| Type::named("List", vec![ty]))
                        .unwrap_or_else(|| function_body_param_type(param))
                } else {
                    signature
                        .and_then(|signature| signature.params.get(index).cloned())
                        .unwrap_or_else(|| function_body_param_type(param))
                };
                Binding::new(param.name.clone(), ty)
            }),
    );
    bindings
}
