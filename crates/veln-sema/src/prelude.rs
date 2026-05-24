use veln_core::{CoreCallTarget, CoreType};

use crate::types::Type;

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let unknown = Type::Unknown;
    let direct_expected = expected.cloned().unwrap_or(Type::Unknown);
    let list_item = expected
        .and_then(Type::list_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    let option_item = expected
        .and_then(Type::option_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    let (result_value, result_error) = expected
        .and_then(Type::result_parts)
        .map_or((Type::Unknown, Type::Unknown), |(value, error)| {
            (value.clone(), error.clone())
        });
    let (dict_key, dict_value) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });

    match name {
        "list_len" => Some((vec![Type::list(unknown)], Type::int())),
        "list_is_empty" => Some((vec![Type::list(unknown)], Type::bool())),
        "list_push" => Some((
            vec![Type::list(list_item.clone()), list_item.clone()],
            Type::list(list_item),
        )),
        "list_concat" => Some((
            vec![Type::list(list_item.clone()), Type::list(list_item.clone())],
            Type::list(list_item),
        )),
        "list_map" => Some((
            vec![
                Type::list(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::list(list_item),
        )),
        "list_filter" => Some((
            vec![
                Type::list(list_item.clone()),
                Type::Function {
                    params: vec![list_item.clone()],
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            Type::list(list_item),
        )),
        "list_fold" => Some((
            vec![
                Type::list(Type::Unknown),
                direct_expected.clone(),
                Type::Function {
                    params: vec![direct_expected.clone(), Type::Unknown],
                    return_type: Box::new(direct_expected.clone()),
                    effects: Vec::new(),
                },
            ],
            direct_expected,
        )),
        "list_try_map" => {
            let mapped_item = result_value.list_part().cloned().unwrap_or(Type::Unknown);
            Some((
                vec![
                    Type::list(Type::Unknown),
                    Type::Function {
                        params: vec![Type::Unknown],
                        return_type: Box::new(Type::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                Type::result(Type::list(mapped_item), result_error),
            ))
        }
        "dict_get" => Some((
            vec![Type::dict(dict_key.clone(), option_item.clone()), dict_key],
            Type::named("Option", vec![option_item]),
        )),
        "dict_contains" => Some((
            vec![Type::dict(dict_key.clone(), dict_value), dict_key],
            Type::bool(),
        )),
        "dict_insert" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            Type::dict(dict_key, dict_value),
        )),
        "dict_remove" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            Type::dict(dict_key, dict_value),
        )),
        "option_map" => Some((
            vec![
                Type::named("Option", vec![Type::Unknown]),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::named("Option", vec![option_item]),
        )),
        "option_and_then" => Some((
            vec![
                Type::named("Option", vec![Type::Unknown]),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(Type::named("Option", vec![option_item.clone()])),
                    effects: Vec::new(),
                },
            ],
            Type::named("Option", vec![option_item]),
        )),
        "option_unwrap_or" => Some((
            vec![
                Type::named("Option", vec![direct_expected.clone()]),
                direct_expected.clone(),
            ],
            direct_expected,
        )),
        "result_map" => Some((
            vec![
                Type::result(Type::Unknown, result_error.clone()),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::result(result_value, result_error),
        )),
        "result_map_err" => Some((
            vec![
                Type::result(result_value.clone(), Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::result(result_value, result_error),
        )),
        "result_and_then" => Some((
            vec![
                Type::result(Type::Unknown, result_error.clone()),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(Type::result(result_value.clone(), result_error.clone())),
                    effects: Vec::new(),
                },
            ],
            Type::result(result_value, result_error),
        )),
        _ => None,
    }
}

pub(crate) fn core_prelude_signature(
    name: &str,
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let unknown = CoreType::Unknown;
    let direct_expected = expected.cloned().unwrap_or(CoreType::Unknown);
    let list_item = expected
        .and_then(CoreType::list_part)
        .cloned()
        .unwrap_or(CoreType::Unknown);
    let option_item = expected
        .and_then(CoreType::option_part)
        .cloned()
        .unwrap_or(CoreType::Unknown);
    let (result_value, result_error) = expected
        .and_then(CoreType::result_parts)
        .map_or((CoreType::Unknown, CoreType::Unknown), |(value, error)| {
            (value.clone(), error.clone())
        });
    let (dict_key, dict_value) = expected
        .and_then(CoreType::dict_parts)
        .map_or((CoreType::Unknown, CoreType::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });

    let signature = match name {
        "list_len" => (vec![CoreType::list(unknown)], CoreType::int()),
        "list_is_empty" => (vec![CoreType::list(unknown)], CoreType::bool()),
        "list_push" => (
            vec![CoreType::list(list_item.clone()), list_item.clone()],
            CoreType::list(list_item),
        ),
        "list_concat" => (
            vec![
                CoreType::list(list_item.clone()),
                CoreType::list(list_item.clone()),
            ],
            CoreType::list(list_item),
        ),
        "list_map" => (
            vec![
                CoreType::list(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::list(list_item),
        ),
        "list_filter" => (
            vec![
                CoreType::list(list_item.clone()),
                CoreType::Function {
                    params: vec![list_item.clone()],
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            CoreType::list(list_item),
        ),
        "list_fold" => (
            vec![
                CoreType::list(CoreType::Unknown),
                direct_expected.clone(),
                CoreType::Function {
                    params: vec![direct_expected.clone(), CoreType::Unknown],
                    return_type: Box::new(direct_expected.clone()),
                    effects: Vec::new(),
                },
            ],
            direct_expected,
        ),
        "list_try_map" => {
            let mapped_item = result_value
                .list_part()
                .cloned()
                .unwrap_or(CoreType::Unknown);
            (
                vec![
                    CoreType::list(CoreType::Unknown),
                    CoreType::Function {
                        params: vec![CoreType::Unknown],
                        return_type: Box::new(CoreType::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                CoreType::result(CoreType::list(mapped_item), result_error),
            )
        }
        "dict_get" => (
            vec![
                CoreType::dict(dict_key.clone(), option_item.clone()),
                dict_key,
            ],
            CoreType::option(option_item),
        ),
        "dict_contains" => (
            vec![CoreType::dict(dict_key.clone(), dict_value), dict_key],
            CoreType::bool(),
        ),
        "dict_insert" => (
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            CoreType::dict(dict_key, dict_value),
        ),
        "dict_remove" => (
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            CoreType::dict(dict_key, dict_value),
        ),
        "option_map" => (
            vec![
                CoreType::option(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::option(option_item),
        ),
        "option_and_then" => (
            vec![
                CoreType::option(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(CoreType::option(option_item.clone())),
                    effects: Vec::new(),
                },
            ],
            CoreType::option(option_item),
        ),
        "option_unwrap_or" => (
            vec![
                CoreType::option(direct_expected.clone()),
                direct_expected.clone(),
            ],
            direct_expected,
        ),
        "result_map" => (
            vec![
                CoreType::result(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::result(result_value, result_error),
        ),
        "result_map_err" => (
            vec![
                CoreType::result(result_value.clone(), CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::result(result_value, result_error),
        ),
        "result_and_then" => (
            vec![
                CoreType::result(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(CoreType::result(
                        result_value.clone(),
                        result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            CoreType::result(result_value, result_error),
        ),
        _ => return None,
    };
    Some((
        CoreCallTarget::PreludeBuiltin(name.to_string()),
        signature.0,
        signature.1,
    ))
}
