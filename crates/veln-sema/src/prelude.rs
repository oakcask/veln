use veln_ast::{BinaryOp, PrefixOp};
use veln_core::{CoreCallTarget, CoreType};

use crate::adt;
use crate::standard_symbols::prelude_symbol;
use crate::types::Type;

pub(crate) const PRELUDE_MODULE: &str = "prelude";
const PRELUDE_BUILTIN_MODULE: &str = "prelude_builtin";

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected(expected);
    prelude_float_signature(descriptor.name)
        .or_else(|| prelude_byte_signature(descriptor.name))
        .or_else(|| prelude_string_signature(descriptor.name))
        .or_else(|| prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| prelude_list_signature(descriptor.name, &expected))
        .or_else(|| prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| prelude_option_signature(descriptor.name, &expected))
        .or_else(|| prelude_result_signature(descriptor.name, &expected))
}

pub(crate) fn qualified_prelude_builtin_signature(
    segments: &[String],
    expected: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_BUILTIN_MODULE {
        return None;
    }
    let (params, return_type) = prelude_signature(name, expected)?;
    Some((name.clone(), params, return_type))
}

pub(crate) fn qualified_prelude_signature(
    segments: &[String],
    expected: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_MODULE {
        return None;
    }
    let (params, return_type) = prelude_signature(name, expected)?;
    Some((name.clone(), params, return_type))
}

struct ExpectedPreludeParts {
    direct: Type,
    vec_item: Type,
    list_item: Type,
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
            list_item: expected
                .and_then(adt::list_part)
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

fn prelude_byte_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    let byte = Type::named("Byte", Vec::new());
    let flag8 = Type::named("Flag8", Vec::new());
    let flag16be = Type::named("Flag16be", Vec::new());
    let flag16le = Type::named("Flag16le", Vec::new());
    let flag32be = Type::named("Flag32be", Vec::new());
    let flag32le = Type::named("Flag32le", Vec::new());
    let flag64be = Type::named("Flag64be", Vec::new());
    let flag64le = Type::named("Flag64le", Vec::new());
    let byte_chunk = Type::named("ByteChunk", Vec::new());
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_count = Type::named("ByteCount", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    match name {
        "byte" => Some((
            vec![Type::int()],
            adt::result_type(byte.clone(), Type::string()),
        )),
        "byte_to_int" => Some((vec![byte.clone()], Type::int())),
        "flag8_is_set" => Some((
            vec![flag8.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag8_set" => Some((
            vec![flag8.clone(), Type::int()],
            adt::result_type(flag8.clone(), Type::string()),
        )),
        "flag8_bits" => Some((vec![flag8.clone()], Type::int())),
        "flag8_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag8.clone(), Type::string()),
        )),
        "flag16be_is_set" => Some((
            vec![flag16be.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag16be_set" => Some((
            vec![flag16be.clone(), Type::int()],
            adt::result_type(flag16be.clone(), Type::string()),
        )),
        "flag16be_bits" => Some((vec![flag16be.clone()], Type::int())),
        "flag16be_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag16be.clone(), Type::string()),
        )),
        "flag16le_is_set" => Some((
            vec![flag16le.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag16le_set" => Some((
            vec![flag16le.clone(), Type::int()],
            adt::result_type(flag16le.clone(), Type::string()),
        )),
        "flag16le_bits" => Some((vec![flag16le.clone()], Type::int())),
        "flag16le_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag16le.clone(), Type::string()),
        )),
        "flag32be_is_set" => Some((
            vec![flag32be.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag32be_set" => Some((
            vec![flag32be.clone(), Type::int()],
            adt::result_type(flag32be.clone(), Type::string()),
        )),
        "flag32be_bits" => Some((vec![flag32be.clone()], Type::int())),
        "flag32be_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag32be.clone(), Type::string()),
        )),
        "flag32le_is_set" => Some((
            vec![flag32le.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag32le_set" => Some((
            vec![flag32le.clone(), Type::int()],
            adt::result_type(flag32le.clone(), Type::string()),
        )),
        "flag32le_bits" => Some((vec![flag32le.clone()], Type::int())),
        "flag32le_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag32le.clone(), Type::string()),
        )),
        "flag64be_is_set" => Some((
            vec![flag64be.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag64be_set" => Some((
            vec![flag64be.clone(), Type::int()],
            adt::result_type(flag64be.clone(), Type::string()),
        )),
        "flag64be_bits" => Some((vec![flag64be.clone()], Type::int())),
        "flag64be_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag64be.clone(), Type::string()),
        )),
        "flag64le_is_set" => Some((
            vec![flag64le.clone(), Type::int()],
            adt::result_type(Type::bool(), Type::string()),
        )),
        "flag64le_set" => Some((
            vec![flag64le.clone(), Type::int()],
            adt::result_type(flag64le.clone(), Type::string()),
        )),
        "flag64le_bits" => Some((vec![flag64le.clone()], Type::int())),
        "flag64le_from_bits" => Some((
            vec![Type::int()],
            adt::result_type(flag64le.clone(), Type::string()),
        )),
        "byte_chunk" => Some((vec![Type::vec(byte.clone())], byte_chunk.clone())),
        "byte_chunk_count" => Some((vec![byte_chunk.clone()], byte_count.clone())),
        "byte_append" => Some((
            vec![byte_chunk.clone(), byte_chunk.clone()],
            byte_chunk.clone(),
        )),
        "byte_chunk_from_hex" => Some((
            vec![Type::string()],
            adt::result_type(byte_chunk.clone(), Type::string()),
        )),
        "byte_chunk_to_visible_ascii_string" => Some((
            vec![byte_chunk.clone()],
            adt::result_type(Type::string(), Type::string()),
        )),
        "byte_take" | "byte_drop" => Some((
            vec![byte_chunk.clone(), byte_count.clone()],
            adt::result_type(byte_chunk.clone(), Type::string()),
        )),
        "byte_view" => Some((
            vec![byte_chunk.clone(), byte_offset.clone(), byte_count.clone()],
            adt::result_type(byte_view.clone(), Type::string()),
        )),
        "byte_view_to_chunk" => Some((vec![byte_view.clone()], byte_chunk.clone())),
        "byte_view_count" => Some((vec![byte_view.clone()], byte_count.clone())),
        "byte_view_take" | "byte_view_drop" => Some((
            vec![byte_view.clone(), byte_count.clone()],
            adt::result_type(byte_view.clone(), Type::string()),
        )),
        "byte_view_slice" => Some((
            vec![byte_view.clone(), byte_count.clone(), byte_count.clone()],
            adt::result_type(byte_view.clone(), Type::string()),
        )),
        "byte_chunks_empty" => Some((Vec::new(), adt::list_type(byte_chunk.clone()))),
        "byte_chunks_one" => Some((vec![byte_chunk.clone()], adt::list_type(byte_chunk.clone()))),
        "byte_chunks_append" => Some((
            vec![
                adt::list_type(byte_chunk.clone()),
                adt::list_type(byte_chunk.clone()),
            ],
            adt::list_type(byte_chunk.clone()),
        )),
        "byte_expect_fixed_u8_be" => Some((
            vec![
                byte_view.clone(),
                Type::int(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::int(), Type::string()),
        )),
        "byte_decode_http2_frame" => Some((
            vec![byte_view.clone()],
            adt::result_type(http2_frame_type(), Type::string()),
        )),
        "byte_decode_schema_width_sample" => Some((
            vec![byte_view.clone()],
            adt::result_type(schema_width_sample_type(), Type::string()),
        )),
        "byte_decode_schema_validation_sample" => Some((
            vec![byte_view.clone()],
            adt::result_type(schema_validation_sample_type(), Type::string()),
        )),
        "http2_protocol_closed_with_pending" => Some((
            vec![Type::int(), Type::int(), Type::string()],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_partial_preface" => Some((
            vec![Type::int(), Type::int(), byte_view.clone()],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_preface" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                byte_view.clone(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_continuation_expected" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_frame_kind" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                byte_view.clone(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_stream_id" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_payload_length" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                byte_view.clone(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_data_padding" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                byte_view.clone(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_unexpected_settings_ack" => Some((
            vec![Type::int(), Type::string(), Type::string()],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_invalid_priority_dependency" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                byte_view.clone(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_protocol_stream_after_goaway" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_peer_limit_frame_size_exceeded" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_peer_limit_header_list_size_exceeded" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_peer_limit_flow_control_window_exceeded" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_peer_limit_concurrent_streams_exceeded" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "http2_peer_limit_settings_value_out_of_range" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::string(),
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "hpack_fixture_unsupported_header_block" => Some((
            vec![
                Type::int(),
                Type::int(),
                Type::int(),
                Type::string(),
                Type::string(),
            ],
            adt::result_type(Type::unit(), Type::string()),
        )),
        "byte_read_u8_be" | "byte_read_u16_be" | "byte_read_u24_be" | "byte_read_u31_be"
        | "byte_read_u32_be" | "byte_read_u40_be" | "byte_read_u48_be" | "byte_read_u64_be"
        | "byte_read_u16_le" | "byte_read_u24_le" | "byte_read_u31_le" | "byte_read_u32_le"
        | "byte_read_u40_le" | "byte_read_u48_le" | "byte_read_u64_le" => Some((
            vec![byte_view.clone()],
            adt::result_type(Type::int(), Type::string()),
        )),
        "byte_write_u8_be" | "byte_write_u16_be" | "byte_write_u24_be" | "byte_write_u31_be"
        | "byte_write_u32_be" | "byte_write_u40_be" | "byte_write_u48_be" | "byte_write_u64_be"
        | "byte_write_u16_le" | "byte_write_u24_le" | "byte_write_u31_le" | "byte_write_u32_le"
        | "byte_write_u40_le" | "byte_write_u48_le" | "byte_write_u64_le" => Some((
            vec![Type::int()],
            adt::result_type(byte_chunk.clone(), Type::string()),
        )),
        "byte_count" => Some((
            vec![Type::int()],
            adt::result_type(byte_count.clone(), Type::string()),
        )),
        "byte_count_to_int" => Some((vec![byte_count.clone()], Type::int())),
        "byte_offset" => Some((
            vec![Type::int()],
            adt::result_type(byte_offset.clone(), Type::string()),
        )),
        "byte_offset_to_int" => Some((vec![byte_offset], Type::int())),
        _ => None,
    }
}

fn http2_frame_type() -> Type {
    Type::Record(vec![
        ("length".to_string(), Type::int()),
        ("kind".to_string(), Type::int()),
        ("flags".to_string(), Type::int()),
        ("stream_id".to_string(), Type::int()),
        ("payload".to_string(), Type::named("ByteView", Vec::new())),
    ])
}

fn schema_width_sample_type() -> Type {
    Type::Record(vec![
        ("short_value".to_string(), Type::int()),
        ("wide_value".to_string(), Type::int()),
    ])
}

fn schema_validation_sample_type() -> Type {
    Type::Record(vec![
        ("length".to_string(), Type::int()),
        ("padding_length".to_string(), Type::int()),
    ])
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
                    variadic: None,
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
                    variadic: None,
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
    match name {
        "list_map" => Some((
            vec![
                adt::list_type(Type::Unknown),
                Type::Function {
                    params: vec![Type::Unknown],
                    variadic: None,
                    return_type: Box::new(list_item.clone()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(list_item.clone()),
        )),
        "list_filter" => Some((
            vec![
                adt::list_type(list_item.clone()),
                Type::Function {
                    params: vec![list_item.clone()],
                    variadic: None,
                    return_type: Box::new(Type::bool()),
                    effects: Vec::new(),
                },
            ],
            adt::list_type(list_item.clone()),
        )),
        "list_fold" => Some((
            vec![
                adt::list_type(Type::Unknown),
                expected.direct.clone(),
                Type::Function {
                    params: vec![expected.direct.clone(), Type::Unknown],
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
                    variadic: None,
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
    match name {
        "result_map" => Some((
            vec![
                adt::result_type(Type::Unknown, result_error.clone()),
                Type::Function {
                    params: vec![Type::Unknown],
                    variadic: None,
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
                    variadic: None,
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
                    variadic: None,
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
        variadic: None,
        return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
        effects: Vec::new(),
    });

    (
        params,
        adt::result_type(Type::vec(mapped_item), result_error),
    )
}

fn list_try_map_signature(result_value: &Type, result_error: Type) -> (Vec<Type>, Type) {
    let mapped_item = adt::list_part(result_value)
        .cloned()
        .unwrap_or(Type::Unknown);
    (
        vec![
            adt::list_type(Type::Unknown),
            Type::Function {
                params: vec![Type::Unknown],
                variadic: None,
                return_type: Box::new(adt::result_type(mapped_item.clone(), result_error.clone())),
                effects: Vec::new(),
            },
        ],
        adt::result_type(adt::list_type(mapped_item), result_error),
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
    let expected = ExpectedCorePreludeParts::from_expected(expected);
    let signature = core_prelude_float_signature(descriptor.name)
        .or_else(|| core_prelude_byte_signature(descriptor.name))
        .or_else(|| core_prelude_string_signature(descriptor.name))
        .or_else(|| core_prelude_vec_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_list_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_dict_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_option_signature(descriptor.name, &expected))
        .or_else(|| core_prelude_result_signature(descriptor.name, &expected))?;
    Some((
        CoreCallTarget::PreludeBuiltin(descriptor.name.to_string()),
        signature.0,
        signature.1,
    ))
}

pub(crate) fn qualified_core_prelude_builtin_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_BUILTIN_MODULE {
        return None;
    }
    core_prelude_signature(name, expected)
}

pub(crate) fn qualified_core_prelude_signature(
    segments: &[String],
    expected: Option<&CoreType>,
) -> Option<(CoreCallTarget, Vec<CoreType>, CoreType)> {
    let [module, name] = segments else {
        return None;
    };
    if module != PRELUDE_MODULE {
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
    let byte = CoreType::named("Byte", Vec::new());
    let flag8 = CoreType::named("Flag8", Vec::new());
    let flag16be = CoreType::named("Flag16be", Vec::new());
    let flag16le = CoreType::named("Flag16le", Vec::new());
    let flag32be = CoreType::named("Flag32be", Vec::new());
    let flag32le = CoreType::named("Flag32le", Vec::new());
    let flag64be = CoreType::named("Flag64be", Vec::new());
    let flag64le = CoreType::named("Flag64le", Vec::new());
    let byte_chunk = CoreType::named("ByteChunk", Vec::new());
    let byte_view = CoreType::named("ByteView", Vec::new());
    let byte_count = CoreType::named("ByteCount", Vec::new());
    let byte_offset = CoreType::named("ByteOffset", Vec::new());
    match name {
        "byte" => Some((
            vec![CoreType::int()],
            adt::core_result_type(byte.clone(), CoreType::string()),
        )),
        "byte_to_int" => Some((vec![byte.clone()], CoreType::int())),
        "flag8_is_set" => Some((
            vec![flag8.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag8_set" => Some((
            vec![flag8.clone(), CoreType::int()],
            adt::core_result_type(flag8.clone(), CoreType::string()),
        )),
        "flag8_bits" => Some((vec![flag8.clone()], CoreType::int())),
        "flag8_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag8.clone(), CoreType::string()),
        )),
        "flag16be_is_set" => Some((
            vec![flag16be.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag16be_set" => Some((
            vec![flag16be.clone(), CoreType::int()],
            adt::core_result_type(flag16be.clone(), CoreType::string()),
        )),
        "flag16be_bits" => Some((vec![flag16be.clone()], CoreType::int())),
        "flag16be_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag16be.clone(), CoreType::string()),
        )),
        "flag16le_is_set" => Some((
            vec![flag16le.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag16le_set" => Some((
            vec![flag16le.clone(), CoreType::int()],
            adt::core_result_type(flag16le.clone(), CoreType::string()),
        )),
        "flag16le_bits" => Some((vec![flag16le.clone()], CoreType::int())),
        "flag16le_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag16le.clone(), CoreType::string()),
        )),
        "flag32be_is_set" => Some((
            vec![flag32be.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag32be_set" => Some((
            vec![flag32be.clone(), CoreType::int()],
            adt::core_result_type(flag32be.clone(), CoreType::string()),
        )),
        "flag32be_bits" => Some((vec![flag32be.clone()], CoreType::int())),
        "flag32be_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag32be.clone(), CoreType::string()),
        )),
        "flag32le_is_set" => Some((
            vec![flag32le.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag32le_set" => Some((
            vec![flag32le.clone(), CoreType::int()],
            adt::core_result_type(flag32le.clone(), CoreType::string()),
        )),
        "flag32le_bits" => Some((vec![flag32le.clone()], CoreType::int())),
        "flag32le_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag32le.clone(), CoreType::string()),
        )),
        "flag64be_is_set" => Some((
            vec![flag64be.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag64be_set" => Some((
            vec![flag64be.clone(), CoreType::int()],
            adt::core_result_type(flag64be.clone(), CoreType::string()),
        )),
        "flag64be_bits" => Some((vec![flag64be.clone()], CoreType::int())),
        "flag64be_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag64be.clone(), CoreType::string()),
        )),
        "flag64le_is_set" => Some((
            vec![flag64le.clone(), CoreType::int()],
            adt::core_result_type(CoreType::bool(), CoreType::string()),
        )),
        "flag64le_set" => Some((
            vec![flag64le.clone(), CoreType::int()],
            adt::core_result_type(flag64le.clone(), CoreType::string()),
        )),
        "flag64le_bits" => Some((vec![flag64le.clone()], CoreType::int())),
        "flag64le_from_bits" => Some((
            vec![CoreType::int()],
            adt::core_result_type(flag64le.clone(), CoreType::string()),
        )),
        "byte_chunk" => Some((vec![CoreType::vec(byte.clone())], byte_chunk.clone())),
        "byte_chunk_count" => Some((vec![byte_chunk.clone()], byte_count.clone())),
        "byte_append" => Some((
            vec![byte_chunk.clone(), byte_chunk.clone()],
            byte_chunk.clone(),
        )),
        "byte_chunk_from_hex" => Some((
            vec![CoreType::string()],
            adt::core_result_type(byte_chunk.clone(), CoreType::string()),
        )),
        "byte_chunk_to_visible_ascii_string" => Some((
            vec![byte_chunk.clone()],
            adt::core_result_type(CoreType::string(), CoreType::string()),
        )),
        "byte_take" | "byte_drop" => Some((
            vec![byte_chunk.clone(), byte_count.clone()],
            adt::core_result_type(byte_chunk.clone(), CoreType::string()),
        )),
        "byte_view" => Some((
            vec![byte_chunk.clone(), byte_offset.clone(), byte_count.clone()],
            adt::core_result_type(byte_view.clone(), CoreType::string()),
        )),
        "byte_view_to_chunk" => Some((vec![byte_view.clone()], byte_chunk.clone())),
        "byte_view_count" => Some((vec![byte_view.clone()], byte_count.clone())),
        "byte_view_take" | "byte_view_drop" => Some((
            vec![byte_view.clone(), byte_count.clone()],
            adt::core_result_type(byte_view.clone(), CoreType::string()),
        )),
        "byte_view_slice" => Some((
            vec![byte_view.clone(), byte_count.clone(), byte_count.clone()],
            adt::core_result_type(byte_view.clone(), CoreType::string()),
        )),
        "byte_chunks_empty" => Some((Vec::new(), adt::core_list_type(byte_chunk.clone()))),
        "byte_chunks_one" => Some((
            vec![byte_chunk.clone()],
            adt::core_list_type(byte_chunk.clone()),
        )),
        "byte_chunks_append" => Some((
            vec![
                adt::core_list_type(byte_chunk.clone()),
                adt::core_list_type(byte_chunk.clone()),
            ],
            adt::core_list_type(byte_chunk.clone()),
        )),
        "byte_expect_fixed_u8_be" => Some((
            vec![
                byte_view.clone(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::int(), CoreType::string()),
        )),
        "byte_decode_http2_frame" => Some((
            vec![byte_view.clone()],
            adt::core_result_type(core_http2_frame_type(), CoreType::string()),
        )),
        "byte_decode_schema_width_sample" => Some((
            vec![byte_view.clone()],
            adt::core_result_type(core_schema_width_sample_type(), CoreType::string()),
        )),
        "byte_decode_schema_validation_sample" => Some((
            vec![byte_view.clone()],
            adt::core_result_type(core_schema_validation_sample_type(), CoreType::string()),
        )),
        "http2_protocol_closed_with_pending" => Some((
            vec![CoreType::int(), CoreType::int(), CoreType::string()],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_partial_preface" => Some((
            vec![CoreType::int(), CoreType::int(), byte_view.clone()],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_preface" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                byte_view.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_continuation_expected" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_frame_kind" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                byte_view.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_stream_id" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_payload_length" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                byte_view.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_data_padding" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                byte_view.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_unexpected_settings_ack" => Some((
            vec![CoreType::int(), CoreType::string(), CoreType::string()],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_invalid_priority_dependency" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                byte_view.clone(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_protocol_stream_after_goaway" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_peer_limit_frame_size_exceeded" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_peer_limit_header_list_size_exceeded" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_peer_limit_flow_control_window_exceeded" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_peer_limit_concurrent_streams_exceeded" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "http2_peer_limit_settings_value_out_of_range" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "hpack_fixture_unsupported_header_block" => Some((
            vec![
                CoreType::int(),
                CoreType::int(),
                CoreType::int(),
                CoreType::string(),
                CoreType::string(),
            ],
            adt::core_result_type(CoreType::unit(), CoreType::string()),
        )),
        "byte_read_u8_be" | "byte_read_u16_be" | "byte_read_u24_be" | "byte_read_u31_be"
        | "byte_read_u32_be" | "byte_read_u40_be" | "byte_read_u48_be" | "byte_read_u64_be"
        | "byte_read_u16_le" | "byte_read_u24_le" | "byte_read_u31_le" | "byte_read_u32_le"
        | "byte_read_u40_le" | "byte_read_u48_le" | "byte_read_u64_le" => Some((
            vec![byte_view.clone()],
            adt::core_result_type(CoreType::int(), CoreType::string()),
        )),
        "byte_write_u8_be" | "byte_write_u16_be" | "byte_write_u24_be" | "byte_write_u31_be"
        | "byte_write_u32_be" | "byte_write_u40_be" | "byte_write_u48_be" | "byte_write_u64_be"
        | "byte_write_u16_le" | "byte_write_u24_le" | "byte_write_u31_le" | "byte_write_u32_le"
        | "byte_write_u40_le" | "byte_write_u48_le" | "byte_write_u64_le" => Some((
            vec![CoreType::int()],
            adt::core_result_type(byte_chunk.clone(), CoreType::string()),
        )),
        "byte_count" => Some((
            vec![CoreType::int()],
            adt::core_result_type(byte_count.clone(), CoreType::string()),
        )),
        "byte_count_to_int" => Some((vec![byte_count.clone()], CoreType::int())),
        "byte_offset" => Some((
            vec![CoreType::int()],
            adt::core_result_type(byte_offset.clone(), CoreType::string()),
        )),
        "byte_offset_to_int" => Some((vec![byte_offset], CoreType::int())),
        _ => None,
    }
}

fn core_http2_frame_type() -> CoreType {
    CoreType::Record(vec![
        ("length".to_string(), CoreType::int()),
        ("kind".to_string(), CoreType::int()),
        ("flags".to_string(), CoreType::int()),
        ("stream_id".to_string(), CoreType::int()),
        (
            "payload".to_string(),
            CoreType::named("ByteView", Vec::new()),
        ),
    ])
}

fn core_schema_width_sample_type() -> CoreType {
    CoreType::Record(vec![
        ("short_value".to_string(), CoreType::int()),
        ("wide_value".to_string(), CoreType::int()),
    ])
}

fn core_schema_validation_sample_type() -> CoreType {
    CoreType::Record(vec![
        ("length".to_string(), CoreType::int()),
        ("padding_length".to_string(), CoreType::int()),
    ])
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
