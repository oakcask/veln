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

pub(super) fn collect_effect_row_substitution(
    expected: &Type,
    actual: &Type,
    row_substitutions: &mut Vec<(String, Vec<String>)>,
) {
    let (
        Type::Function {
            params: expected_params,
            variadic: expected_variadic,
            return_type: expected_return,
            effects: expected_effects,
        },
        Type::Function {
            params: actual_params,
            variadic: actual_variadic,
            return_type: actual_return,
            effects: actual_effects,
        },
    ) = (expected, actual)
    else {
        return;
    };

    for effect in expected_effects {
        let Some(row) = effect.strip_prefix("...") else {
            continue;
        };
        let concrete = actual_effects
            .iter()
            .filter(|actual_effect| {
                !expected_effects
                    .iter()
                    .any(|expected_effect| expected_effect == *actual_effect)
            })
            .cloned()
            .collect::<Vec<_>>();
        merge_effect_row_substitution(row_substitutions, row, concrete);
    }

    for (expected_param, actual_param) in expected_params.iter().zip(actual_params) {
        collect_effect_row_substitution(expected_param, actual_param, row_substitutions);
    }
    if let (Some(expected), Some(actual)) =
        (expected_variadic.as_deref(), actual_variadic.as_deref())
    {
        collect_effect_row_substitution(expected, actual, row_substitutions);
    }
    collect_effect_row_substitution(expected_return, actual_return, row_substitutions);
}

pub(super) fn merge_effect_row_substitution(
    row_substitutions: &mut Vec<(String, Vec<String>)>,
    row: &str,
    effects: Vec<String>,
) {
    if let Some((_, existing)) = row_substitutions
        .iter_mut()
        .find(|(existing_row, _)| existing_row == row)
    {
        for effect in effects {
            push_unique_effect(existing, effect);
        }
        return;
    }
    let mut unique = Vec::new();
    for effect in effects {
        push_unique_effect(&mut unique, effect);
    }
    row_substitutions.push((row.to_string(), unique));
}

pub(super) fn instantiate_effects(
    effects: &[String],
    row_substitutions: &[(String, Vec<String>)],
) -> Vec<String> {
    let mut instantiated = Vec::new();
    for effect in effects {
        if let Some(row) = effect.strip_prefix("...") {
            if let Some((_, substitution)) = row_substitutions
                .iter()
                .find(|(candidate, _)| candidate == row)
            {
                for substituted in substitution {
                    push_unique_effect(&mut instantiated, substituted.clone());
                }
            } else {
                push_unique_effect(&mut instantiated, effect.clone());
            }
        } else {
            push_unique_effect(&mut instantiated, effect.clone());
        }
    }
    instantiated
}

pub(super) fn push_unique_effect(effects: &mut Vec<String>, effect: String) {
    if !effects.contains(&effect) {
        effects.push(effect);
    }
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
