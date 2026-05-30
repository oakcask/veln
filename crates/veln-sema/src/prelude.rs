use veln_ast::{BinaryOp, PrefixOp};
use veln_core::{CoreCallTarget, CoreType};

use crate::adt;
use crate::standard_symbols::prelude_symbol;
use crate::types::Type;

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected(expected);
    prelude_float_signature(descriptor.name)
        .or_else(|| prelude_string_signature(descriptor.name))
        .or_else(|| prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| prelude_option_signature(descriptor.name, &expected))
        .or_else(|| prelude_result_signature(descriptor.name, &expected))
}

struct ExpectedPreludeParts {
    direct: Type,
    vec_item: Type,
    option_item: Type,
    result_value: Type,
    result_error: Type,
    dict_key: Type,
    dict_value: Type,
}

impl ExpectedPreludeParts {
    fn from_expected(expected: Option<&Type>) -> Self {
        let (result_value, result_error) = expected
            .and_then(adt::result_parts)
            .map_or((Type::Unknown, Type::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let (dict_key, dict_value) = expected
            .and_then(Type::dict_parts)
            .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        Self {
            direct: expected.cloned().unwrap_or(Type::Unknown),
            vec_item: expected
                .and_then(Type::vec_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            option_item: expected
                .and_then(adt::option_part)
                .cloned()
                .unwrap_or(Type::Unknown),
            result_value,
            result_error,
            dict_key,
            dict_value,
        }
    }
}

fn prelude_float_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    match name {
        "float_negate" => Some((vec![Type::float()], Type::float())),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => {
            Some((vec![Type::float(), Type::float()], Type::float()))
        }
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            Some((vec![Type::float(), Type::float()], Type::bool()))
        }
        _ => None,
    }
}

fn prelude_string_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    match name {
        "string_split_once" => Some((
            vec![Type::string(), Type::string()],
            adt::option_type(Type::Record(vec![
                ("left".to_string(), Type::string()),
                ("right".to_string(), Type::string()),
            ])),
        )),
        "string_parse_int" => Some((
            vec![Type::string()],
            adt::result_type(Type::int(), Type::string()),
        )),
        "int_to_string" => Some((vec![Type::int()], Type::string())),
        _ => None,
    }
}

fn prelude_vec_signature(name: &str, expected: &ExpectedPreludeParts) -> Option<(Vec<Type>, Type)> {
    prelude_vec_basic_signature(name, &expected.vec_item)
        .or_else(|| prelude_vec_callback_signature(name, expected))
}

fn prelude_vec_basic_signature(name: &str, vec_item: &Type) -> Option<(Vec<Type>, Type)> {
    match name {
        "vec_len" => Some((vec![Type::vec(Type::Unknown)], Type::int())),
        "vec_is_empty" => Some((vec![Type::vec(Type::Unknown)], Type::bool())),
        "vec_push" => Some((
            vec![Type::vec(vec_item.clone()), vec_item.clone()],
            Type::vec(vec_item.clone()),
        )),
        "vec_concat" => Some((
            vec![Type::vec(vec_item.clone()), Type::vec(vec_item.clone())],
            Type::vec(vec_item.clone()),
        )),
        _ => None,
    }
}

fn prelude_vec_callback_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let vec_item = &expected.vec_item;
    match name {
        "vec_map" => Some((
            vec![
                Type::vec(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(vec_item.clone()),
        )),
        "vec_filter" => Some((
            vec![
                Type::vec(vec_item.clone()),
                Type::Function {
                    params: vec![vec_item.clone()],
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(vec_item.clone()),
        )),
        "vec_fold" => Some((
            vec![
                Type::vec(Type::Unknown),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), Type::Unknown],
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "vec_try_map" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            false,
        )),
        "vec_try_map_with" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            true,
        )),
        _ => None,
    }
}

fn prelude_dict_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let dict_key = &expected.dict_key;
    let dict_value = &expected.dict_value;
    let option_item = &expected.option_item;
    match name {
        "dict_get" => Some((
            vec![
                Type::dict(dict_key.clone(), option_item.clone()),
                dict_key.clone(),
            ],
            adt::option_type(option_item.clone()),
        )),
        "dict_contains" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            Type::bool(),
        )),
        "dict_insert" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            Type::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_remove" => Some((
            vec![
                Type::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            Type::dict(dict_key.clone(), dict_value.clone()),
        )),
        _ => None,
    }
}

fn prelude_option_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let option_item = &expected.option_item;
    match name {
        "option_map" => Some((
            vec![
                adt::option_type(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::option_type(option_item.clone()),
        )),
        "option_and_then" => Some((
            vec![
                adt::option_type(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(adt::option_type(option_item.clone())),
                    effects: Vec::new(),
                },
            ],
            adt::option_type(option_item.clone()),
        )),
        "option_unwrap_or" => Some((
            vec![
                adt::option_type(expected.direct.clone()),
                expected.direct.clone(),
            ],
            expected.direct.clone(),
        )),
        _ => None,
    }
}

fn prelude_result_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let result_value = &expected.result_value;
    let result_error = &expected.result_error;
    match name {
        "result_map" => Some((
            vec![
                adt::result_type(Type::Unknown, result_error.clone()),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), result_error.clone()),
        )),
        "result_map_err" => Some((
            vec![
                adt::result_type(result_value.clone(), Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), result_error.clone()),
        )),
        "result_and_then" => Some((
            vec![
                adt::result_type(Type::Unknown, result_error.clone()),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(adt::result_type(
                        result_value.clone(),
                        result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), result_error.clone()),
        )),
        _ => None,
    }
}

fn vec_try_map_signature(
    result_value: &Type,
    result_error: Type,
    with_context: bool,
) -> (Vec<Type>, Type) {
    let mapped_item = result_value.vec_part().cloned().unwrap_or(Type::Unknown);
    let mut params = Vec::new();
    let mut callback_params = Vec::new();

    if with_context {
        params.push(Type::Unknown);
        callback_params.push(Type::Unknown);
    }

    params.push(Type::vec(Type::Unknown));
    callback_params.push(Type::Unknown);
    params.push(Type::Function {
        params: callback_params,
        return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
        effects: Vec::new(),
    });

    (
        params,
        adt::result_type(Type::vec(mapped_item), result_error),
    )
}

pub(crate) fn float_prefix_prelude_name(op: PrefixOp) -> Option<&'static str> {
    match op {
        PrefixOp::Negate => Some("float_negate"),
        _ => None,
    }
}

pub(crate) fn float_arithmetic_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("float_add"),
        BinaryOp::Subtract => Some("float_subtract"),
        BinaryOp::Multiply => Some("float_multiply"),
        BinaryOp::Divide => Some("float_divide"),
        _ => None,
    }
}

pub(crate) fn float_comparison_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Less => Some("float_less"),
        BinaryOp::LessEqual => Some("float_less_equal"),
        BinaryOp::Greater => Some("float_greater"),
        BinaryOp::GreaterEqual => Some("float_greater_equal"),
        _ => None,
    }
}

fn core_vec_try_map_signature(
    result_value: &CoreType,
    result_error: CoreType,
    with_context: bool,
) -> (Vec<CoreType>, CoreType) {
    let mapped_item = result_value
        .vec_part()
        .cloned()
        .unwrap_or(CoreType::Unknown);
    let mut params = Vec::new();
    let mut callback_params = Vec::new();

    if with_context {
        params.push(CoreType::Unknown);
        callback_params.push(CoreType::Unknown);
    }

    params.push(CoreType::vec(CoreType::Unknown));
    callback_params.push(CoreType::Unknown);
    params.push(CoreType::Function {
        params: callback_params,
        return_type: Box::new(adt::core_result_type(
            mapped_item.clone(),
            result_error.clone(),
        )),
        effects: Vec::new(),
    });

    (
        params,
        adt::core_result_type(CoreType::vec(mapped_item), result_error),
    )
}

pub(crate) fn core_prelude_signature(
    name: &str,
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedCorePreludeParts::from_expected(expected);
    let signature = core_prelude_float_signature(descriptor.name)
        .or_else(|| core_prelude_string_signature(descriptor.name))
        .or_else(|| core_prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_option_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_result_signature(descriptor.name, &expected))?;
    Some((
        CoreCallTarget::PreludeBuiltin(descriptor.name.to_string()),
        signature.0,
        signature.1,
    ))
}

struct ExpectedCorePreludeParts {
    direct: CoreType,
    vec_item: CoreType,
    option_item: CoreType,
    result_value: CoreType,
    result_error: CoreType,
    dict_key: CoreType,
    dict_value: CoreType,
}

impl ExpectedCorePreludeParts {
    fn from_expected(expected: Option<&CoreType>) -> Self {
        let (result_value, result_error) = expected
            .and_then(adt::core_result_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let (dict_key, dict_value) = expected
            .and_then(CoreType::dict_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(key, value)| {
                (key.clone(), value.clone())
            });
        Self {
            direct: expected.cloned().unwrap_or(CoreType::Unknown),
            vec_item: expected
                .and_then(CoreType::vec_part)
                .cloned()
                .unwrap_or(CoreType::Unknown),
            option_item: expected
                .and_then(adt::core_option_part)
                .cloned()
                .unwrap_or(CoreType::Unknown),
            result_value,
            result_error,
            dict_key,
            dict_value,
        }
    }
}

fn core_prelude_float_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "float_negate" => Some((vec![CoreType::float()], CoreType::float())),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => Some((
            vec![CoreType::float(), CoreType::float()],
            CoreType::float(),
        )),
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            Some((vec![CoreType::float(), CoreType::float()], CoreType::bool()))
        }
        _ => None,
    }
}

fn core_prelude_string_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "string_split_once" => Some((
            vec![CoreType::string(), CoreType::string()],
            adt::core_option_type(CoreType::Record(vec![
                ("left".to_string(), CoreType::string()),
                ("right".to_string(), CoreType::string()),
            ])),
        )),
        "string_parse_int" => Some((
            vec![CoreType::string()],
            adt::core_result_type(CoreType::int(), CoreType::string()),
        )),
        "int_to_string" => Some((vec![CoreType::int()], CoreType::string())),
        _ => None,
    }
}

fn core_prelude_vec_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    core_prelude_vec_basic_signature(name, &expected.vec_item)
        .or_else(|| core_prelude_vec_callback_signature(name, expected))
}

fn core_prelude_vec_basic_signature(
    name: &str,
    vec_item: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "vec_len" => Some((vec![CoreType::vec(CoreType::Unknown)], CoreType::int())),
        "vec_is_empty" => Some((vec![CoreType::vec(CoreType::Unknown)], CoreType::bool())),
        "vec_push" => Some((
            vec![CoreType::vec(vec_item.clone()), vec_item.clone()],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_concat" => Some((
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::vec(vec_item.clone()),
            ],
            CoreType::vec(vec_item.clone()),
        )),
        _ => None,
    }
}

fn core_prelude_vec_callback_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let vec_item = &expected.vec_item;
    match name {
        "vec_map" => Some((
            vec![
                CoreType::vec(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_filter" => Some((
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::Function {
                    params: vec![vec_item.clone()],
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item.clone()),
        )),
        "vec_fold" => Some((
            vec![
                CoreType::vec(CoreType::Unknown),
                expected.direct.clone(),
                CoreType::Function {
                    params: vec![expected.direct.clone(), CoreType::Unknown],
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "vec_try_map" => Some(core_vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            false,
        )),
        "vec_try_map_with" => Some(core_vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            true,
        )),
        _ => None,
    }
}

fn core_prelude_dict_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let dict_key = &expected.dict_key;
    let dict_value = &expected.dict_value;
    let option_item = &expected.option_item;
    match name {
        "dict_get" => Some((
            vec![
                CoreType::dict(dict_key.clone(), option_item.clone()),
                dict_key.clone(),
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "dict_contains" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            CoreType::bool(),
        )),
        "dict_insert" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
                dict_value.clone(),
            ],
            CoreType::dict(dict_key.clone(), dict_value.clone()),
        )),
        "dict_remove" => Some((
            vec![
                CoreType::dict(dict_key.clone(), dict_value.clone()),
                dict_key.clone(),
            ],
            CoreType::dict(dict_key.clone(), dict_value.clone()),
        )),
        _ => None,
    }
}

fn core_prelude_option_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let option_item = &expected.option_item;
    match name {
        "option_map" => Some((
            vec![
                adt::core_option_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "option_and_then" => Some((
            vec![
                adt::core_option_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(adt::core_option_type(option_item.clone())),
                    effects: Vec::new(),
                },
            ],
            adt::core_option_type(option_item.clone()),
        )),
        "option_unwrap_or" => Some((
            vec![
                adt::core_option_type(expected.direct.clone()),
                expected.direct.clone(),
            ],
            expected.direct.clone(),
        )),
        _ => None,
    }
}

fn core_prelude_result_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let result_value = &expected.result_value;
    let result_error = &expected.result_error;
    match name {
        "result_map" => Some((
            vec![
                adt::core_result_type(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        "result_map_err" => Some((
            vec![
                adt::core_result_type(result_value.clone(), CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        "result_and_then" => Some((
            vec![
                adt::core_result_type(CoreType::Unknown, result_error.clone()),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(adt::core_result_type(
                        result_value.clone(),
                        result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            adt::core_result_type(result_value.clone(), result_error.clone()),
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prelude_signature_is_gated_by_standard_symbol_descriptor() {
        let (params, return_type) =
            prelude_signature("vec_len", None).expect("standard helper signature");

        assert_eq!(params, vec![Type::vec(Type::Unknown)]);
        assert_eq!(return_type, Type::int());
        assert!(prelude_signature("unknown_helper", None).is_none());
    }
}
