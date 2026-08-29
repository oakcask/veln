use super::*;

pub(super) fn invalid_qualified_constructor_recovery_cases(
    pattern: &Pattern,
    domain: &MatchDomain,
    scrutinee_type: &Type,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> Vec<String> {
    let PatternKind::Constructor { name, .. } = &pattern.kind else {
        return Vec::new();
    };
    if !invalid_qualified_constructor_pattern(name) {
        return Vec::new();
    }
    let MatchDomain::Adt = domain else {
        return Vec::new();
    };
    let Some(descriptor) = environment.adts.descriptor_for_type(scrutinee_type) else {
        return Vec::new();
    };
    let Some(recovered) = initial_uppercase_qualified_constructor_name(name) else {
        return Vec::new();
    };
    environment
        .adts
        .constructor_for_descriptor(&recovered, descriptor, current_module, &environment.uses)
        .map(|constructor| vec![constructor.variant.coverage_case.clone()])
        .unwrap_or_default()
}

pub(super) fn invalid_qualified_constructor_pattern(name: &[String]) -> bool {
    name.len() > 1
        && name
            .last()
            .and_then(|name| name.as_bytes().first())
            .is_some_and(u8::is_ascii_lowercase)
}

pub(super) fn initial_uppercase_qualified_constructor_name(name: &[String]) -> Option<Vec<String>> {
    let mut recovered = name.to_vec();
    let leaf = recovered.last_mut()?;
    let first = leaf.as_bytes().first().copied()?;
    if !first.is_ascii_lowercase() {
        return None;
    }
    leaf.replace_range(0..1, &(first as char).to_ascii_uppercase().to_string());
    Some(recovered)
}

pub(super) fn shift_operator_text(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::ShiftLeft => Some("<<"),
        BinaryOp::ShiftRight => Some(">>"),
        BinaryOp::ShiftRightLogical => Some(">>>"),
        _ => None,
    }
}

pub(super) fn invalid_literal_shift_count(op: BinaryOp, expr: &Expr) -> Option<i64> {
    shift_operator_text(op)?;
    let value = match &expr.kind {
        ExprKind::IntLiteral(text) => veln_literals::parse_integer_literal(text).ok()?.value,
        ExprKind::Prefix {
            op: veln_ast::PrefixOp::Negate,
            expr,
        } => match &expr.kind {
            ExprKind::IntLiteral(text) => -veln_literals::parse_integer_literal(text).ok()?.value,
            _ => return None,
        },
        _ => return None,
    };
    (!(0..=63).contains(&value)).then_some(value)
}

pub(super) fn prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

pub(super) fn parameter_annotation_is_omitted(param: &veln_ast::Param) -> bool {
    param
        .ty
        .as_deref()
        .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
}

pub(super) fn collection_item_expected(
    ty: Type,
    expected: Option<&ExpectedType>,
    origin_node_id: NodeId,
    origin_span: SourceSpan,
    inferred_message: &'static str,
) -> ExpectedType {
    ExpectedType {
        ty,
        source: expected.map_or(ExpectedTypeSource::Inferred, |expected| expected.source),
        origin_node_id: expected.map_or(origin_node_id, |expected| expected.origin_node_id),
        origin_span: expected
            .and_then(|expected| expected.origin_span.clone())
            .or(Some(origin_span)),
        origin_message: expected.map_or(inferred_message, |expected| expected.origin_message),
    }
}

pub(super) fn known_concurrency_type_arg_overflow(
    segments: &[String],
    type_args: Option<&[String]>,
) -> bool {
    let Some(type_args) = type_args else {
        return false;
    };
    let limit = match segments {
        [module, name] if module == "task" && name == "spawn_with" => 2,
        [module, _] if module == "channel" || module == "task" => 1,
        _ => return false,
    };
    if type_args.len() <= limit {
        return false;
    }
    qualified_symbol(segments).is_some_and(|symbol| symbol.effects.contains(&"concurrency"))
}
