use super::{CallArgumentEvidence, CoreType, Type};

pub(super) fn io_provider_state_core_signature(
    name: &str,
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "state" => Some(core_state_signature(expected, evidence.first)),
        "update" => Some(core_update_signature(expected, evidence)),
        _ => None,
    }
}

fn core_state_signature(
    expected: Option<&CoreType>,
    first: Option<&CoreType>,
) -> (Vec<CoreType>, CoreType) {
    let item = expected
        .and_then(core_state_item)
        .or_else(|| first.cloned())
        .unwrap_or(CoreType::Unknown);
    let state = expected
        .filter(|expected| matches!(expected, CoreType::Named { name, .. } if name == "State"))
        .cloned()
        .unwrap_or_else(|| CoreType::named("State", vec![item.clone()]));
    (vec![item], state)
}

fn core_update_signature(
    expected: Option<&CoreType>,
    evidence: CallArgumentEvidence<'_, CoreType>,
) -> (Vec<CoreType>, CoreType) {
    let original_cell = evidence.first.cloned().unwrap_or(CoreType::Unknown);
    let mut item = core_state_item(&original_cell).unwrap_or(CoreType::Unknown);
    if item.contains_unknown()
        && let Some(callback_item) = evidence.third.and_then(core_callback_item_type)
    {
        item = callback_item;
    }
    let cell = if matches!(original_cell, CoreType::Named { ref name, .. } if name == "State") {
        CoreType::named("State", vec![item.clone()])
    } else {
        original_cell
    };
    let context = evidence.second.cloned().unwrap_or(CoreType::Unknown);
    let result = evidence
        .third
        .and_then(core_callback_result_type)
        .or_else(|| expected.cloned())
        .unwrap_or(CoreType::Unknown);
    let callback = CoreType::Function {
        params: vec![item.clone(), context.clone()],
        variadic: None,
        return_type: Box::new(CoreType::Record(
            [
                ("state".to_string(), item),
                ("result".to_string(), result.clone()),
            ]
            .into_iter()
            .collect(),
        )),
        effects: Vec::new(),
    };
    (vec![cell, context, callback], result)
}

fn core_state_item(ty: &CoreType) -> Option<CoreType> {
    match ty {
        CoreType::Named { name, args } if name == "State" && args.len() == 1 => {
            Some(args[0].clone())
        }
        _ => None,
    }
}

fn core_callback_result_type(callback: &CoreType) -> Option<CoreType> {
    let CoreType::Function { return_type, .. } = callback else {
        return None;
    };
    let CoreType::Record(fields) = return_type.as_ref() else {
        return None;
    };
    fields
        .iter()
        .find(|(name, _)| name == "result")
        .map(|(_, ty)| ty.clone())
}

fn core_callback_item_type(callback: &CoreType) -> Option<CoreType> {
    let CoreType::Function { params, .. } = callback else {
        return None;
    };
    params.first().cloned()
}

pub(super) fn io_provider_state_surface_signature(
    name: &str,
    expected: Option<&Type>,
    evidence: CallArgumentEvidence<'_, Type>,
) -> Option<(Vec<Type>, Type)> {
    match name {
        "state" => Some(surface_state_signature(expected, evidence.first)),
        "update" => Some(surface_update_signature(expected, evidence)),
        _ => None,
    }
}

fn surface_state_signature(expected: Option<&Type>, first: Option<&Type>) -> (Vec<Type>, Type) {
    let item = expected
        .and_then(surface_state_item)
        .or_else(|| first.map(state_item_type))
        .unwrap_or(Type::Unknown);
    let state = expected
        .filter(|expected| is_surface_state(expected))
        .cloned()
        .unwrap_or_else(|| surface_state(item.clone()));
    (vec![item], state)
}

fn surface_update_signature(
    expected: Option<&Type>,
    evidence: CallArgumentEvidence<'_, Type>,
) -> (Vec<Type>, Type) {
    let original_cell = evidence.first.cloned().unwrap_or(Type::Unknown);
    let mut item = surface_state_item(&original_cell).unwrap_or(Type::Unknown);
    if item.contains_unknown()
        && let Some(callback_item) = evidence.third.and_then(callback_item_type)
    {
        item = callback_item;
    }
    let cell = surface_state(item.clone());
    let context = evidence.second.cloned().unwrap_or(Type::Unknown);
    let result = evidence
        .third
        .and_then(callback_result_type)
        .or_else(|| expected.cloned())
        .unwrap_or(Type::Unknown);
    let callback = Type::Function {
        params: vec![item.clone(), context.clone()].into(),
        variadic: None,
        return_type: Box::new(Type::Record(
            [
                ("state".to_string(), item),
                ("result".to_string(), result.clone()),
            ]
            .into_iter()
            .collect(),
        )),
        effects: Vec::new(),
    };
    (vec![cell, context, callback], result)
}

fn surface_state(item: Type) -> Type {
    Type::resolved_named("State", "std::io_provider::State", vec![item])
}

fn is_surface_state(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, identity, .. }
        if name == "State" && identity == "std::io_provider::State")
}

fn surface_state_item(ty: &Type) -> Option<Type> {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        } if name == "State" && identity == "std::io_provider::State" && args.len() == 1 => {
            Some(args[0].clone())
        }
        _ => None,
    }
}

fn callback_result_type(callback: &Type) -> Option<Type> {
    let Type::Function { return_type, .. } = callback else {
        return None;
    };
    let Type::Record(fields) = return_type.as_ref() else {
        return None;
    };
    fields
        .iter()
        .find(|(name, _)| name == "result")
        .map(|(_, ty)| ty.clone())
}

fn callback_item_type(callback: &Type) -> Option<Type> {
    let Type::Function { params, .. } = callback else {
        return None;
    };
    params.first().cloned()
}

fn state_item_type(ty: &Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name.to_string(), identity.to_string(), args.to_vec()),
        _ => ty.clone(),
    }
}
