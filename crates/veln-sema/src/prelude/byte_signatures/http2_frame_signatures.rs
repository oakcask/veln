use super::{BytePreludeType, BytePreludeTypes, ByteSignature, unit_runtime_diagnostic_result};

pub(super) fn http2_protocol_frame_payload_signature<T: BytePreludeType>(
    name: &str,
    types: &BytePreludeTypes<T>,
) -> Option<ByteSignature<T>> {
    match name {
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
        _ => None,
    }
}

pub(super) fn http2_protocol_frame_identity_signature<T: BytePreludeType>(
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
        _ => None,
    }
}
