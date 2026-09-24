use super::descriptors::AdtVariantDescriptor;
use super::runtime_variant_builders::{
    int_field, named_field, runtime_diagnostic_variant, string_field,
};

pub(super) fn runtime_peer_limit_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_list_size"),
                int_field("allowed_header_list_size"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("receive_limit_provenance"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_table_size"),
                int_field("allowed_header_table_size"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("receive_limit_provenance"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("attempted_concurrent_stream_count"),
                int_field("allowed_concurrent_stream_count"),
                string_field("endpoint_role"),
                string_field("active_state"),
                string_field("receive_limit_provenance"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitSettingsValueDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("setting_identifier"),
                string_field("setting_name"),
                int_field("observed_value"),
                int_field("accepted_min_value"),
                int_field("accepted_max_value"),
                string_field("peer_limit_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("frame_kind"),
                int_field("stream_id"),
                int_field("observed_payload_length"),
                int_field("expected_payload_length"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("pad_length"),
                int_field("remaining_payload_length"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_payload_length"),
                int_field("allowed_window_credit"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
    ]
}
