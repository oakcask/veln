use veln_ast::{BinaryOp, PrefixOp};
use veln_core::{CoreCallTarget, CoreType};

use crate::standard_symbols::prelude_symbol;
use crate::types::Type;

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let unknown = Type::Unknown;
    let direct_expected = expected.cloned().unwrap_or(Type::Unknown);
    let vec_item = expected
        .and_then(Type::vec_part)
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

    match descriptor.name {
        "float_negate" => Some((vec![Type::float()], Type::float())),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => {
            Some((vec![Type::float(), Type::float()], Type::float()))
        }
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            Some((vec![Type::float(), Type::float()], Type::bool()))
        }
        "string_split_once" => Some((
            vec![Type::string(), Type::string()],
            Type::named(
                "Option",
                vec![Type::Record(vec![
                    ("left".to_string(), Type::string()),
                    ("right".to_string(), Type::string()),
                ])],
            ),
        )),
        "string_parse_int" => Some((
            vec![Type::string()],
            Type::result(Type::int(), Type::string()),
        )),
        "int_to_string" => Some((vec![Type::int()], Type::string())),
        "vec_len" => Some((vec![Type::vec(unknown)], Type::int())),
        "vec_is_empty" => Some((vec![Type::vec(unknown)], Type::bool())),
        "vec_push" => Some((
            vec![Type::vec(vec_item.clone()), vec_item.clone()],
            Type::vec(vec_item),
        )),
        "vec_concat" => Some((
            vec![Type::vec(vec_item.clone()), Type::vec(vec_item.clone())],
            Type::vec(vec_item),
        )),
        "vec_map" => Some((
            vec![
                Type::vec(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(vec_item),
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
            Type::vec(vec_item),
        )),
        "vec_fold" => Some((
            vec![
                Type::vec(Type::Unknown),
                direct_expected.clone(),
                Type::Function {
                    params: vec![direct_expected.clone(), Type::Unknown],
                    return_type: Box::new(direct_expected.clone()),
                    effects: Vec::new(),
                },
            ],
            direct_expected,
        )),
        "vec_try_map" => {
            let mapped_item = result_value.vec_part().cloned().unwrap_or(Type::Unknown);
            Some((
                vec![
                    Type::vec(Type::Unknown),
                    Type::Function {
                        params: vec![Type::Unknown],
                        return_type: Box::new(Type::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                Type::result(Type::vec(mapped_item), result_error),
            ))
        }
        "vec_try_map_with" => {
            let mapped_item = result_value.vec_part().cloned().unwrap_or(Type::Unknown);
            Some((
                vec![
                    Type::Unknown,
                    Type::vec(Type::Unknown),
                    Type::Function {
                        params: vec![Type::Unknown, Type::Unknown],
                        return_type: Box::new(Type::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                Type::result(Type::vec(mapped_item), result_error),
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

pub(crate) fn core_prelude_signature(
    name: &str,
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let descriptor = prelude_symbol(name)?;
    let unknown = CoreType::Unknown;
    let direct_expected = expected.cloned().unwrap_or(CoreType::Unknown);
    let vec_item = expected
        .and_then(CoreType::vec_part)
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

    let signature = match descriptor.name {
        "float_negate" => (vec![CoreType::float()], CoreType::float()),
        "float_add" | "float_subtract" | "float_multiply" | "float_divide" => (
            vec![CoreType::float(), CoreType::float()],
            CoreType::float(),
        ),
        "float_less" | "float_less_equal" | "float_greater" | "float_greater_equal" => {
            (vec![CoreType::float(), CoreType::float()], CoreType::bool())
        }
        "string_split_once" => (
            vec![CoreType::string(), CoreType::string()],
            CoreType::option(CoreType::Record(vec![
                ("left".to_string(), CoreType::string()),
                ("right".to_string(), CoreType::string()),
            ])),
        ),
        "string_parse_int" => (
            vec![CoreType::string()],
            CoreType::result(CoreType::int(), CoreType::string()),
        ),
        "int_to_string" => (vec![CoreType::int()], CoreType::string()),
        "vec_len" => (vec![CoreType::vec(unknown)], CoreType::int()),
        "vec_is_empty" => (vec![CoreType::vec(unknown)], CoreType::bool()),
        "vec_push" => (
            vec![CoreType::vec(vec_item.clone()), vec_item.clone()],
            CoreType::vec(vec_item),
        ),
        "vec_concat" => (
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::vec(vec_item.clone()),
            ],
            CoreType::vec(vec_item),
        ),
        "vec_map" => (
            vec![
                CoreType::vec(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item),
        ),
        "vec_filter" => (
            vec![
                CoreType::vec(vec_item.clone()),
                CoreType::Function {
                    params: vec![vec_item.clone()],
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            CoreType::vec(vec_item),
        ),
        "vec_fold" => (
            vec![
                CoreType::vec(CoreType::Unknown),
                direct_expected.clone(),
                CoreType::Function {
                    params: vec![direct_expected.clone(), CoreType::Unknown],
                    return_type: Box::new(direct_expected.clone()),
                    effects: Vec::new(),
                },
            ],
            direct_expected,
        ),
        "vec_try_map" => {
            let mapped_item = result_value
                .vec_part()
                .cloned()
                .unwrap_or(CoreType::Unknown);
            (
                vec![
                    CoreType::vec(CoreType::Unknown),
                    CoreType::Function {
                        params: vec![CoreType::Unknown],
                        return_type: Box::new(CoreType::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                CoreType::result(CoreType::vec(mapped_item), result_error),
            )
        }
        "vec_try_map_with" => {
            let mapped_item = result_value
                .vec_part()
                .cloned()
                .unwrap_or(CoreType::Unknown);
            (
                vec![
                    CoreType::Unknown,
                    CoreType::vec(CoreType::Unknown),
                    CoreType::Function {
                        params: vec![CoreType::Unknown, CoreType::Unknown],
                        return_type: Box::new(CoreType::result(
                            mapped_item.clone(),
                            result_error.clone(),
                        )),
                        effects: Vec::new(),
                    },
                ],
                CoreType::result(CoreType::vec(mapped_item), result_error),
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
        CoreCallTarget::PreludeBuiltin(descriptor.name.to_string()),
        signature.0,
        signature.1,
    ))
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
