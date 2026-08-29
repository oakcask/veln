use super::*;

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
        variadic: None,
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
    core_compiler_adapter_signature(descriptor, expected)
}

fn core_compiler_adapter_signature(
    descriptor: &StandardSymbolDescriptor,
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let expected = ExpectedCorePreludeParts::from_expected(expected);
    let signature = core_prelude_float_signature(descriptor.name)
        .or_else(|| core_prelude_byte_signature(descriptor.name))
        .or_else(|| core_prelude_string_signature(descriptor.name))
        .or_else(|| core_prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_list_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_option_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_result_signature(descriptor.name, &expected))
        .or_else(|| compiler_adapter_core_callback_signature(descriptor))?;
    Some((
        CoreCallTarget::PreludeBuiltin(descriptor.name.to_string()),
        signature.0,
        signature.1,
    ))
}

fn compiler_adapter_core_callback_signature(
    descriptor: &StandardSymbolDescriptor,
) -> Option<(Vec<CoreType>, CoreType)> {
    let (params, return_type) = compiler_adapter_callback_signature(descriptor)?;
    Some((
        params.iter().map(core_type).collect(),
        core_type(&return_type),
    ))
}

pub(crate) fn qualified_core_prelude_builtin_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != prelude_builtin_module() {
        return None;
    }
    core_compiler_adapter_signature(compiler_adapter_symbol(name)?, expected)
}

pub(crate) fn qualified_core_prelude_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != standard_module() {
        return None;
    }
    core_prelude_signature(name, expected)
}

struct ExpectedCorePreludeParts {
    direct: CoreType,
    vec_item: CoreType,
    list_item: CoreType,
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
            list_item: expected
                .and_then(adt::core_list_part)
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

fn core_prelude_byte_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    core_byte_prelude_signature(name)
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
                    variadic: None,
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
                    variadic: None,
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
                    variadic: None,
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

fn core_prelude_list_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    core_prelude_list_basic_signature(name, &expected.list_item)
        .or_else(|| core_prelude_list_callback_signature(name, expected))
}

fn core_prelude_list_basic_signature(
    name: &str,
    list_item: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "list_nil" => Some((Vec::new(), adt::core_list_type(list_item.clone()))),
        "list_cons" => Some((
            vec![list_item.clone(), adt::core_list_type(list_item.clone())],
            adt::core_list_type(list_item.clone()),
        )),
        "list_is_empty" => Some((
            vec![adt::core_list_type(CoreType::Unknown)],
            CoreType::bool(),
        )),
        "list_reverse" => Some((
            vec![adt::core_list_type(list_item.clone())],
            adt::core_list_type(list_item.clone()),
        )),
        _ => None,
    }
}

fn core_prelude_list_callback_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    let list_item = &expected.list_item;
    match name {
        "list_map" => Some((
            vec![
                adt::core_list_type(CoreType::Unknown),
                CoreType::Function {
                    params: vec![CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::core_list_type(list_item.clone()),
        )),
        "list_filter" => Some((
            vec![
                adt::core_list_type(list_item.clone()),
                CoreType::Function {
                    params: vec![list_item.clone()],
                    variadic: None,
                    return_type: Box::new(CoreType::bool()),
                    effects: Vec::new(),
                },
            ],
            adt::core_list_type(list_item.clone()),
        )),
        "list_fold" => Some((
            vec![
                adt::core_list_type(CoreType::Unknown),
                expected.direct.clone(),
                CoreType::Function {
                    params: vec![expected.direct.clone(), CoreType::Unknown],
                    variadic: None,
                    return_type: Box::new(expected.direct.clone()),
                    effects: Vec::new(),
                },
            ],
            expected.direct.clone(),
        )),
        "list_try_map" => Some(core_list_try_map_signature(
            &expected.result_value,
            expected.result_error.clone(),
        )),
        _ => None,
    }
}

fn core_prelude_dict_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    core_prelude_dict_basic_signature(name, expected)
        .or_else(|| core_prelude_dict_callback_signature(name, expected))
}

fn core_prelude_dict_basic_signature(
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

fn core_prelude_dict_callback_signature(
    name: &str,
    expected: &ExpectedCorePreludeParts,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "dict_map" => Some(core_prelude_dict_map_signature(false, expected)),
        "dict_map_with" => Some(core_prelude_dict_map_signature(true, expected)),
        "dict_filter" => Some(core_prelude_dict_filter_signature(false, expected)),
        "dict_filter_with" => Some(core_prelude_dict_filter_signature(true, expected)),
        "dict_fold" => Some(core_prelude_dict_fold_signature(false, expected)),
        "dict_fold_with" => Some(core_prelude_dict_fold_signature(true, expected)),
        "dict_try_map" => Some(core_prelude_dict_try_map_signature(false, expected)),
        "dict_try_map_with" => Some(core_prelude_dict_try_map_signature(true, expected)),
        _ => None,
    }
}

fn core_prelude_dict_map_signature(
    with_context: bool,
    expected: &ExpectedCorePreludeParts,
) -> (Vec<CoreType>, CoreType) {
    let callback = CoreType::Function {
        params: core_prelude_callback_args_with_context(
            with_context,
            vec![expected.dict_key.clone(), CoreType::Unknown],
        ),
        variadic: None,
        return_type: Box::new(expected.dict_value.clone()),
        effects: Vec::new(),
    };
    (
        core_prelude_callback_params_with_context(
            with_context,
            CoreType::dict(expected.dict_key.clone(), CoreType::Unknown),
            callback,
        ),
        CoreType::dict(expected.dict_key.clone(), expected.dict_value.clone()),
    )
}

fn core_prelude_dict_filter_signature(
    with_context: bool,
    expected: &ExpectedCorePreludeParts,
) -> (Vec<CoreType>, CoreType) {
    let callback = CoreType::Function {
        params: core_prelude_callback_args_with_context(
            with_context,
            vec![expected.dict_key.clone(), expected.dict_value.clone()],
        ),
        variadic: None,
        return_type: Box::new(CoreType::bool()),
        effects: Vec::new(),
    };
    (
        core_prelude_callback_params_with_context(
            with_context,
            CoreType::dict(expected.dict_key.clone(), expected.dict_value.clone()),
            callback,
        ),
        CoreType::dict(expected.dict_key.clone(), expected.dict_value.clone()),
    )
}

fn core_prelude_dict_fold_signature(
    with_context: bool,
    expected: &ExpectedCorePreludeParts,
) -> (Vec<CoreType>, CoreType) {
    let callback = CoreType::Function {
        params: core_prelude_callback_args_with_context(
            with_context,
            vec![
                expected.direct.clone(),
                CoreType::Unknown,
                CoreType::Unknown,
            ],
        ),
        variadic: None,
        return_type: Box::new(expected.direct.clone()),
        effects: Vec::new(),
    };
    (
        core_prelude_fold_params_with_context(
            with_context,
            CoreType::dict(CoreType::Unknown, CoreType::Unknown),
            expected.direct.clone(),
            callback,
        ),
        expected.direct.clone(),
    )
}

fn core_prelude_dict_try_map_signature(
    with_context: bool,
    expected: &ExpectedCorePreludeParts,
) -> (Vec<CoreType>, CoreType) {
    let (result_key, result_value) = expected
        .result_value
        .dict_parts()
        .map_or((CoreType::Unknown, CoreType::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    let callback = CoreType::Function {
        params: core_prelude_callback_args_with_context(
            with_context,
            vec![result_key.clone(), CoreType::Unknown],
        ),
        variadic: None,
        return_type: Box::new(adt::core_result_type(
            result_value.clone(),
            expected.result_error.clone(),
        )),
        effects: Vec::new(),
    };
    (
        core_prelude_callback_params_with_context(
            with_context,
            CoreType::dict(result_key.clone(), CoreType::Unknown),
            callback,
        ),
        adt::core_result_type(
            CoreType::dict(result_key, result_value),
            expected.result_error.clone(),
        ),
    )
}

fn core_prelude_callback_params_with_context(
    with_context: bool,
    input: CoreType,
    callback: CoreType,
) -> Vec<CoreType> {
    let mut params = Vec::new();
    if with_context {
        params.push(CoreType::Unknown);
    }
    params.push(input);
    params.push(callback);
    params
}

fn core_prelude_fold_params_with_context(
    with_context: bool,
    input: CoreType,
    initial: CoreType,
    callback: CoreType,
) -> Vec<CoreType> {
    let mut params = Vec::new();
    if with_context {
        params.push(CoreType::Unknown);
    }
    params.push(input);
    params.push(initial);
    params.push(callback);
    params
}

fn core_prelude_callback_args_with_context(
    with_context: bool,
    args: Vec<CoreType>,
) -> Vec<CoreType> {
    if with_context {
        std::iter::once(CoreType::Unknown).chain(args).collect()
    } else {
        args
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
                    variadic: None,
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
                    variadic: None,
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
                    variadic: None,
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
                    variadic: None,
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
                    variadic: None,
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

fn core_list_try_map_signature(
    result_value: &CoreType,
    result_error: CoreType,
) -> (Vec<CoreType>, CoreType) {
    let mapped_item = adt::core_list_part(result_value)
        .cloned()
        .unwrap_or(CoreType::Unknown);
    (
        vec![
            adt::core_list_type(CoreType::Unknown),
            CoreType::Function {
                params: vec![CoreType::Unknown],
                variadic: None,
                return_type: Box::new(adt::core_result_type(
                    mapped_item.clone(),
                    result_error.clone(),
                )),
                effects: Vec::new(),
            },
        ],
        adt::core_result_type(adt::core_list_type(mapped_item), result_error),
    )
}
