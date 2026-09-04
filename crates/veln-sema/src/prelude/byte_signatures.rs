use veln_core::CoreType;

use crate::adt::type_operations as adt;
use crate::semantic_model::Type;

pub(super) fn prelude_byte_signature(name: &str) -> Option<(Vec<Type>, Type)> {
    byte_prelude_signature::<Type>(name)
}

pub(super) fn core_byte_prelude_signature(name: &str) -> Option<(Vec<CoreType>, CoreType)> {
    byte_prelude_signature::<CoreType>(name)
}

type ByteSignature<T> = (Vec<T>, T);

trait BytePreludeType: Clone {
    fn named(name: &'static str) -> Self;
    fn record(fields: Vec<(&'static str, Self)>) -> Self;
    fn int() -> Self;
    fn string() -> Self;
    fn unit() -> Self;
    fn vec(item: Self) -> Self;
    fn list(item: Self) -> Self;
    fn result(value: Self, error: Self) -> Self;
}

impl BytePreludeType for Type {
    fn named(name: &'static str) -> Self {
        Type::named(name, Vec::new())
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn int() -> Self {
        Type::int()
    }

    fn string() -> Self {
        Type::string()
    }

    fn unit() -> Self {
        Type::unit()
    }

    fn vec(item: Self) -> Self {
        Type::vec(item)
    }

    fn list(item: Self) -> Self {
        adt::list_type(item)
    }

    fn result(value: Self, error: Self) -> Self {
        adt::result_type(value, error)
    }
}

impl BytePreludeType for CoreType {
    fn named(name: &'static str) -> Self {
        CoreType::named(name, Vec::new())
    }

    fn record(fields: Vec<(&'static str, Self)>) -> Self {
        CoreType::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        )
    }

    fn int() -> Self {
        CoreType::int()
    }

    fn string() -> Self {
        CoreType::string()
    }

    fn unit() -> Self {
        CoreType::unit()
    }

    fn vec(item: Self) -> Self {
        CoreType::vec(item)
    }

    fn list(item: Self) -> Self {
        adt::core_list_type(item)
    }

    fn result(value: Self, error: Self) -> Self {
        adt::core_result_type(value, error)
    }
}

struct BytePreludeTypes<T> {
    byte: T,
    byte_chunk: T,
    byte_view: T,
    byte_count: T,
    byte_offset: T,
}

impl<T: BytePreludeType> BytePreludeTypes<T> {
    fn new() -> Self {
        Self {
            byte: T::named("Byte"),
            byte_chunk: T::named("ByteChunk"),
            byte_view: T::named("ByteView"),
            byte_count: T::named("ByteCount"),
            byte_offset: T::named("ByteOffset"),
        }
    }
}

fn byte_prelude_signature<T: BytePreludeType>(name: &str) -> Option<ByteSignature<T>> {
    let types = BytePreludeTypes::new();
    byte_constructor_signature(name, &types)
        .or_else(|| byte_chunk_signature(name, &types))
        .or_else(|| byte_view_signature(name, &types))
        .or_else(|| byte_chunk_list_signature(name, &types))
        .or_else(|| byte_decode_sample_signature(name, &types))
        .or_else(|| http2_protocol_preface_signature(name, &types))
        .or_else(|| http2_protocol_frame_signature(name, &types))
        .or_else(|| http2_peer_limit_signature(name, &types))
        .or_else(|| hpack_fixture_signature(name, &types))
        .or_else(|| byte_numeric_signature(name, &types))
}

fn result_string<T: BytePreludeType>(value: T) -> T {
    T::result(value, T::string())
}

fn unit_runtime_diagnostic_result<T: BytePreludeType>() -> T {
    T::result(T::unit(), T::named("RuntimeDiagnostic"))
}

fn byte_constructor_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte" => Some((vec![T::int()], result_string(types.byte.clone()))),
        "byte_to_int" => Some((vec![types.byte.clone()], T::int())),
        _ => None,
    }
}

fn byte_chunk_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_chunk" => Some((vec![T::vec(types.byte.clone())], types.byte_chunk.clone())),
        "byte_chunk_count" => Some((vec![types.byte_chunk.clone()], types.byte_count.clone())),
        "byte_append" => Some((
            vec![types.byte_chunk.clone(), types.byte_chunk.clone()],
            types.byte_chunk.clone(),
        )),
        "byte_chunk_from_hex" => Some((vec![T::string()], result_string(types.byte_chunk.clone()))),
        "byte_chunk_to_visible_ascii_string" => {
            Some((vec![types.byte_chunk.clone()], result_string(T::string())))
        }
        "byte_chunk_from_visible_ascii_string" => {
            Some((vec![T::string()], result_string(types.byte_chunk.clone())))
        }
        "byte_take" | "byte_drop" => Some((
            vec![types.byte_chunk.clone(), types.byte_count.clone()],
            result_string(types.byte_chunk.clone()),
        )),
        _ => None,
    }
}

fn byte_view_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_view" => Some((
            vec![
                types.byte_chunk.clone(),
                types.byte_offset.clone(),
                types.byte_count.clone(),
            ],
            result_string(types.byte_view.clone()),
        )),
        "byte_view_to_chunk" => Some((vec![types.byte_view.clone()], types.byte_chunk.clone())),
        "byte_view_count" => Some((vec![types.byte_view.clone()], types.byte_count.clone())),
        "byte_view_take" | "byte_view_drop" => Some((
            vec![types.byte_view.clone(), types.byte_count.clone()],
            result_string(types.byte_view.clone()),
        )),
        "byte_view_slice" => Some((
            vec![
                types.byte_view.clone(),
                types.byte_count.clone(),
                types.byte_count.clone(),
            ],
            result_string(types.byte_view.clone()),
        )),
        _ => None,
    }
}

fn byte_chunk_list_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_chunks_empty" => Some((Vec::new(), T::list(types.byte_chunk.clone()))),
        "byte_chunks_one" => Some((
            vec![types.byte_chunk.clone()],
            T::list(types.byte_chunk.clone()),
        )),
        "byte_chunks_append" => Some((
            vec![
                T::list(types.byte_chunk.clone()),
                T::list(types.byte_chunk.clone()),
            ],
            T::list(types.byte_chunk.clone()),
        )),
        "byte_chunks_produce" => {
            let chunk_list = T::list(types.byte_chunk.clone());
            Some((
                vec![chunk_list.clone(), types.byte_count.clone()],
                T::record(vec![
                    ("chunks", chunk_list.clone()),
                    ("produced", types.byte_count.clone()),
                    ("remaining", chunk_list),
                ]),
            ))
        }
        _ => None,
    }
}

fn byte_decode_sample_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_expect_fixed_u8_be" => Some((
            vec![types.byte_view.clone(), T::int(), T::string(), T::string()],
            result_string(T::int()),
        )),
        "byte_decode_http2_frame" => Some((
            vec![types.byte_view.clone()],
            result_string(http2_frame_type()),
        )),
        "byte_decode_schema_width_sample" => Some((
            vec![types.byte_view.clone()],
            result_string(schema_width_sample_type()),
        )),
        "byte_decode_schema_validation_sample" => Some((
            vec![types.byte_view.clone()],
            result_string(schema_validation_sample_type()),
        )),
        _ => None,
    }
}

fn http2_protocol_preface_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_protocol_closed_with_pending" => Some((
            vec![
                T::int(),
                T::int(),
                T::string(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_partial_preface" => Some((
            vec![T::int(), T::int(), types.byte_view.clone()],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_preface" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_initial_peer_settings_required" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_continuation_expected" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn http2_protocol_frame_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_protocol_invalid_frame_kind" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_stream_id" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_payload_length" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_payload_length_chunk" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_chunk.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_window_update_increment" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_data_padding" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_content_length_mismatch" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_request_header_list"
        | "http2_protocol_invalid_response_header_list" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_unexpected_settings_ack" => Some((
            vec![T::int(), T::string(), T::string(), types.byte_view.clone()],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_settings_not_allowed_for_endpoint" => Some((
            vec![
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_invalid_priority_dependency" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_protocol_stream_after_goaway" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn http2_peer_limit_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "http2_peer_limit_frame_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_header_list_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_header_table_size_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_flow_control_window_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_concurrent_streams_exceeded" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "http2_peer_limit_settings_value_out_of_range" => Some((
            vec![
                T::int(),
                T::int(),
                T::string(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn hpack_fixture_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "hpack_fixture_unsupported_header_block"
        | "hpack_fixture_unsupported_static_index"
        | "hpack_fixture_malformed_string_length"
        | "hpack_fixture_malformed_raw_string_value"
        | "hpack_fixture_malformed_huffman_padding"
        | "hpack_fixture_huffman_eos_symbol"
        | "hpack_fixture_huffman_non_visible_value"
        | "hpack_fixture_table_size_update_malformed" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "hpack_fixture_dynamic_index_out_of_range"
        | "hpack_fixture_dynamic_name_continuation_missing"
        | "hpack_fixture_dynamic_name_continuation_malformed"
        | "hpack_fixture_dynamic_name_continuation_out_of_range" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        "hpack_fixture_table_size_update_not_at_start"
        | "hpack_fixture_table_size_update_trailing_bytes" => Some((
            vec![
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::int(),
                T::string(),
                T::string(),
                T::string(),
                types.byte_view.clone(),
            ],
            unit_runtime_diagnostic_result(),
        )),
        _ => None,
    }
}

fn byte_numeric_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
        "byte_read_u8_be" | "byte_read_u16_be" | "byte_read_u24_be" | "byte_read_u31_be"
        | "byte_read_u32_be" | "byte_read_u40_be" | "byte_read_u48_be" | "byte_read_u56_be"
        | "byte_read_u64_be" | "byte_read_u16_le" | "byte_read_u24_le" | "byte_read_u31_le"
        | "byte_read_u32_le" | "byte_read_u40_le" | "byte_read_u48_le" | "byte_read_u56_le"
        | "byte_read_u64_le" => Some((vec![types.byte_view.clone()], result_string(T::int()))),
        "byte_write_u8_be" | "byte_write_u16_be" | "byte_write_u24_be" | "byte_write_u31_be"
        | "byte_write_u32_be" | "byte_write_u40_be" | "byte_write_u48_be" | "byte_write_u56_be"
        | "byte_write_u64_be" | "byte_write_u16_le" | "byte_write_u24_le" | "byte_write_u31_le"
        | "byte_write_u32_le" | "byte_write_u40_le" | "byte_write_u48_le" | "byte_write_u56_le"
        | "byte_write_u64_le" => Some((vec![T::int()], result_string(types.byte_chunk.clone()))),
        "byte_count" => Some((vec![T::int()], result_string(types.byte_count.clone()))),
        "byte_count_to_int" => Some((vec![types.byte_count.clone()], T::int())),
        "byte_offset" => Some((vec![T::int()], result_string(types.byte_offset.clone()))),
        "byte_offset_to_int" => Some((vec![types.byte_offset.clone()], T::int())),
        _ => None,
    }
}

fn http2_frame_type<T: BytePreludeType>() -> T {
    T::record(vec![
        ("length", T::int()),
        ("kind", T::int()),
        ("flags", T::int()),
        ("stream_id", T::int()),
        ("payload", T::named("ByteView")),
    ])
}

fn schema_width_sample_type<T: BytePreludeType>() -> T {
    T::record(vec![("short_value", T::int()), ("wide_value", T::int())])
}

fn schema_validation_sample_type<T: BytePreludeType>() -> T {
    T::record(vec![("length", T::int()), ("padding_length", T::int())])
}
