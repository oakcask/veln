use std::{collections::BTreeSet, sync::OnceLock};

use veln_ast::{BinaryOp, PrefixOp, SurfaceModule, lower_surface_ast};
use veln_core::{CoreCallTarget, CoreType};
use veln_source::SourceFile;
use veln_syntax::parse;

use crate::adt;
use crate::semantic_model::Type;
use crate::source_less_lookup::{
    compiler_adapter_symbol, prelude_builtin_module, prelude_symbol, standard_module,
};
use crate::standard_symbols::StandardSymbolDescriptor;
use crate::type_lowering::core_type;
use crate::type_syntax::parse_type_or_unknown;

mod byte_signatures;
mod core_signatures;
mod expected_types;
mod source_signatures;

use byte_signatures::{core_byte_prelude_signature, prelude_byte_signature};
pub(crate) use core_signatures::{
    core_prelude_signature, qualified_core_prelude_builtin_signature,
    qualified_core_prelude_signature,
};
use expected_types::ExpectedPreludeParts;
use source_signatures::compiler_adapter_callback_signature;
#[cfg(test)]
use source_signatures::source_prelude_callback_signatures_from_text;

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    prelude_signature_with_input(name, expected, None)
}

pub(crate) fn prelude_signature_with_input(
    name: &str,
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected_and_input(expected, input);
    prelude_float_signature(descriptor.name)
        .or_else(|| prelude_byte_signature(descriptor.name))
        .or_else(|| prelude_string_signature(descriptor.name))
        .or_else(|| prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| prelude_list_signature(descriptor.name, &expected))
        .or_else(|| prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| prelude_option_signature(descriptor.name, &expected))
        .or_else(|| prelude_result_signature(descriptor.name, &expected))
        .or_else(|| compiler_adapter_callback_signature(descriptor))
}

pub(crate) fn qualified_prelude_builtin_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != prelude_builtin_module() {
        return None;
    }
    let descriptor = compiler_adapter_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected_and_input(expected, input);
    let (params, return_type) = prelude_float_signature(descriptor.name)
        .or_else(|| prelude_byte_signature(descriptor.name))
        .or_else(|| prelude_string_signature(descriptor.name))
        .or_else(|| prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| prelude_list_signature(descriptor.name, &expected))
        .or_else(|| prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| prelude_option_signature(descriptor.name, &expected))
        .or_else(|| prelude_result_signature(descriptor.name, &expected))
        .or_else(|| compiler_adapter_callback_signature(descriptor))?;
    Some((name.clone(), params, return_type))
}

pub(crate) fn qualified_prelude_signature(
    segments: &[String],
    expected: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    qualified_prelude_signature_with_input(segments, expected, None)
}

pub(crate) fn qualified_prelude_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != standard_module() {
        return None;
    }
    let (params, return_type) = prelude_signature_with_input(name, expected, input)?;
    Some((name.clone(), params, return_type))
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
    let input_vec_item = &expected.input_vec_item;
    let same_vec_item = prefer_known(input_vec_item, vec_item);
    match name {
        "vec_map" => Some((
            vec![
                Type::vec(input_vec_item.to_owned()),
                Type::Function {
                    params: vec![input_vec_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(vec_item.clone()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(vec_item.clone()),
        )),
        "vec_filter" => Some((
            vec![
                Type::vec(same_vec_item.clone()),
                Type::Function {
                    params: vec![same_vec_item.clone()],
                    variadic: None,
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            Type::vec(same_vec_item),
        )),
        "vec_fold" => Some((
            vec![
                Type::vec(input_vec_item.to_owned()),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), input_vec_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "vec_try_map" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_vec_item,
            false,
        )),
        "vec_try_map_with" => Some(vec_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_vec_item,
            true,
        )),
        _ => None,
    }
}

fn prelude_list_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    prelude_list_basic_signature(name, &expected.list_item)
        .or_else(|| prelude_list_callback_signature(name, expected))
}

fn prelude_list_basic_signature(name: &str, list_item: &Type) -> Option<(Vec<Type>, Type)> {
    match name {
        "list_nil" => Some((Vec::new(), adt::list_type(list_item.clone()))),
        "list_cons" => Some((
            vec![list_item.clone(), adt::list_type(list_item.clone())],
            adt::list_type(list_item.clone()),
        )),
        "list_is_empty" => Some((vec![adt::list_type(Type::Unknown)], Type::bool())),
        "list_reverse" => Some((
            vec![adt::list_type(list_item.clone())],
            adt::list_type(list_item.clone()),
        )),
        _ => None,
    }
}

fn prelude_list_callback_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let list_item = &expected.list_item;
    let input_list_item = &expected.input_list_item;
    let same_list_item = prefer_known(input_list_item, list_item);
    match name {
        "list_map" => Some((
            vec![
                adt::list_type(input_list_item.to_owned()),
                Type::Function {
                    params: vec![input_list_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(list_item.clone()),
        )),
        "list_filter" => Some((
            vec![
                adt::list_type(same_list_item.clone()),
                Type::Function {
                    params: vec![same_list_item.clone()],
                    variadic: None,
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(same_list_item),
        )),
        "list_fold" => Some((
            vec![
                adt::list_type(input_list_item.to_owned()),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), input_list_item.to_owned()],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "list_try_map" => Some(list_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
            input_list_item,
        )),
        _ => None,
    }
}

fn prelude_dict_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    prelude_dict_basic_signature(name, expected)
        .or_else(|| prelude_dict_callback_signature(name, expected))
}

fn prelude_dict_basic_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let dict_key = prefer_known(&expected.dict_key, &expected.input_dict_key);
    let dict_value = prefer_known(&expected.dict_value, &expected.input_dict_value);
    let option_item = prefer_known(&expected.option_item, &expected.input_dict_value);
    match name {
        "dict_get" => Some((
            vec![Type::dict(dict_key.clone(), option_item.clone()), dict_key],
            adt::option_type(option_item.clone()),
        )),
        "dict_contains" => Some((
            vec![Type::dict(dict_key.clone(), dict_value.clone()), dict_key],
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

fn prelude_dict_callback_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    match name {
        "dict_map" => Some(prelude_dict_map_signature(false, expected)),
        "dict_map_with" => Some(prelude_dict_map_signature(true, expected)),
        "dict_filter" => Some(prelude_dict_filter_signature(false, expected)),
        "dict_filter_with" => Some(prelude_dict_filter_signature(true, expected)),
        "dict_fold" => Some(prelude_dict_fold_signature(false, expected)),
        "dict_fold_with" => Some(prelude_dict_fold_signature(true, expected)),
        "dict_try_map" => Some(prelude_dict_try_map_signature(false, expected)),
        "dict_try_map_with" => Some(prelude_dict_try_map_signature(true, expected)),
        _ => None,
    }
}

fn prelude_dict_map_signature(
    with_context: bool,
    expected: &ExpectedPreludeParts,
) -> (Vec<Type>, Type) {
    let result_key = prefer_known(&expected.input_dict_key, &expected.dict_key);
    let result_value = expected.dict_value.clone();
    let callback = Type::Function {
        params: prelude_callback_args_with_context(
            with_context,
            vec![result_key.clone(), expected.input_dict_value.clone()],
        ),
        variadic: None,
        return_type: Box::new(result_value.clone()),
        effects: Vec::new(),
    };
    (
        prelude_callback_params_with_context(
            with_context,
            Type::dict(result_key.clone(), expected.input_dict_value.clone()),
            callback,
        ),
        Type::dict(result_key, result_value),
    )
}

fn prelude_dict_filter_signature(
    with_context: bool,
    expected: &ExpectedPreludeParts,
) -> (Vec<Type>, Type) {
    let key = prefer_known(&expected.input_dict_key, &expected.dict_key);
    let value = prefer_known(&expected.input_dict_value, &expected.dict_value);
    let callback = Type::Function {
        params: prelude_callback_args_with_context(with_context, vec![key.clone(), value.clone()]),
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: Vec::new(),
    };
    (
        prelude_callback_params_with_context(
            with_context,
            Type::dict(key.clone(), value.clone()),
            callback,
        ),
        Type::dict(key, value),
    )
}

fn prelude_dict_fold_signature(
    with_context: bool,
    expected: &ExpectedPreludeParts,
) -> (Vec<Type>, Type) {
    let callback = Type::Function {
        params: prelude_callback_args_with_context(
            with_context,
            vec![
                expected.direct.clone(),
                expected.input_dict_key.clone(),
                expected.input_dict_value.clone(),
            ],
        ),
        variadic: None,
        return_type: Box::new(expected.direct.clone()),
        effects: Vec::new(),
    };
    (
        prelude_fold_params_with_context(
            with_context,
            Type::dict(
                expected.input_dict_key.clone(),
                expected.input_dict_value.clone(),
            ),
            expected.direct.clone(),
            callback,
        ),
        expected.direct.clone(),
    )
}

fn prelude_dict_try_map_signature(
    with_context: bool,
    expected: &ExpectedPreludeParts,
) -> (Vec<Type>, Type) {
    let (result_key, result_value) = expected
        .result_value
        .dict_parts()
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    let output_key = prefer_known(&expected.input_dict_key, &result_key);
    let callback = Type::Function {
        params: prelude_callback_args_with_context(
            with_context,
            vec![output_key.clone(), expected.input_dict_value.clone()],
        ),
        variadic: None,
        return_type: Box::new(adt::result_type(
            result_value.clone(),
            expected.result_error.clone(),
        )),
        effects: Vec::new(),
    };
    (
        prelude_callback_params_with_context(
            with_context,
            Type::dict(output_key.clone(), expected.input_dict_value.clone()),
            callback,
        ),
        adt::result_type(
            Type::dict(output_key, result_value),
            expected.result_error.clone(),
        ),
    )
}

fn prelude_callback_params_with_context(
    with_context: bool,
    input: Type,
    callback: Type,
) -> Vec<Type> {
    let mut params = Vec::new();
    if with_context {
        params.push(Type::Unknown);
    }
    params.push(input);
    params.push(callback);
    params
}

fn prelude_fold_params_with_context(
    with_context: bool,
    input: Type,
    initial: Type,
    callback: Type,
) -> Vec<Type> {
    let mut params = Vec::new();
    if with_context {
        params.push(Type::Unknown);
    }
    params.push(input);
    params.push(initial);
    params.push(callback);
    params
}

fn prelude_callback_args_with_context(with_context: bool, args: Vec<Type>) -> Vec<Type> {
    if with_context {
        std::iter::once(Type::Unknown).chain(args).collect()
    } else {
        args
    }
}

fn prelude_option_signature(
    name: &str,
    expected: &ExpectedPreludeParts,
) -> Option<(Vec<Type>, Type)> {
    let option_item = &expected.option_item;
    let input_option_item = &expected.input_option_item;
    match name {
        "option_map" => Some((
            vec![
                adt::option_type(input_option_item.clone()),
                Type::Function {
                    params: vec![input_option_item.clone()],
                    variadic: None,
                    return_type: Box::new(option_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::option_type(option_item.clone()),
        )),
        "option_and_then" => Some((
            vec![
                adt::option_type(input_option_item.clone()),
                Type::Function {
                    params: vec![input_option_item.clone()],
                    variadic: None,
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
    let input_result_value = &expected.input_result_value;
    let input_result_error = &expected.input_result_error;
    let carried_result_value = prefer_known(result_value, &expected.input_result_value);
    let carried_result_error = prefer_known(result_error, &expected.input_result_error);
    match name {
        "result_map" => Some((
            vec![
                adt::result_type(input_result_value.clone(), carried_result_error.clone()),
                Type::Function {
                    params: vec![input_result_value.clone()],
                    variadic: None,
                    return_type: Box::new(result_value.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), carried_result_error),
        )),
        "result_map_err" => Some((
            vec![
                adt::result_type(carried_result_value.clone(), input_result_error.clone()),
                Type::Function {
                    params: vec![input_result_error.clone()],
                    variadic: None,
                    return_type: Box::new(result_error.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(carried_result_value, result_error.clone()),
        )),
        "result_and_then" => Some((
            vec![
                adt::result_type(input_result_value.clone(), carried_result_error.clone()),
                Type::Function {
                    params: vec![input_result_value.clone()],
                    variadic: None,
                    return_type: Box::new(adt::result_type(
                        result_value.clone(),
                        carried_result_error.clone(),
                    )),
                    effects: Vec::new(),
                },
            ],
            adt::result_type(result_value.clone(), carried_result_error),
        )),
        _ => None,
    }
}

fn vec_try_map_signature(
    result_value: &Type,
    result_error: Type,
    input_vec_item: &Type,
    with_context: bool,
) -> (Vec<Type>, Type) {
    let mapped_item = result_value.vec_part().cloned().unwrap_or(Type::Unknown);
    let input_item = input_vec_item.clone();
    let mut params = Vec::new();
    let mut callback_params = Vec::new();

    if with_context {
        params.push(Type::Unknown);
        callback_params.push(Type::Unknown);
    }

    params.push(Type::vec(input_item.clone()));
    callback_params.push(input_item);
    params.push(Type::Function {
        params: callback_params,
        variadic: None,
        return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
        effects: Vec::new(),
    });

    (
        params,
        adt::result_type(Type::vec(mapped_item), result_error),
    )
}

fn list_try_map_signature(
    result_value: &Type,
    result_error: Type,
    input_list_item: &Type,
) -> (Vec<Type>, Type) {
    let mapped_item = adt::list_part(result_value)
        .cloned()
        .unwrap_or(Type::Unknown);
    let input_item = input_list_item.clone();
    (
        vec![
            adt::list_type(input_item.clone()),
            Type::Function {
                params: vec![input_item],
                variadic: None,
                return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
                effects: Vec::new(),
            },
        ],
        adt::result_type(adt::list_type(mapped_item), result_error),
    )
}

fn prefer_known(primary: &Type, fallback: &Type) -> Type {
    if primary == &Type::Unknown {
        fallback.clone()
    } else {
        primary.clone()
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

#[cfg(test)]
#[path = "prelude/tests.rs"]
mod tests;
