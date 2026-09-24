use super::descriptors::AdtVariantDescriptor;
use super::runtime_variant_builders::{
    int_field, named_field, runtime_diagnostic_variant, string_field,
};

pub(super) fn runtime_connection_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolClosedWithPendingDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("pending_count"),
                string_field("active_continuation"),
                int_field("expected_stream_id"),
                int_field("started_frame_kind"),
                int_field("started_byte_offset"),
                int_field("accumulated_header_block_bytes"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolPartialPrefaceDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("pending_count"),
                int_field("expected_count"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("expected_byte"),
                int_field("actual_byte"),
                int_field("matched_prefix_count"),
                int_field("expected_count"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("actual_frame_kind"),
                int_field("actual_flags"),
                int_field("stream_id"),
                string_field("endpoint_role"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolContinuationExpectedDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("actual_frame_kind"),
                int_field("actual_stream_id"),
                int_field("expected_stream_id"),
                int_field("started_frame_kind"),
                int_field("started_byte_offset"),
                string_field("active_continuation"),
                int_field("accumulated_header_block_bytes"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("actual_frame_kind"),
                int_field("stream_id"),
                int_field("expected_frame_kind"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("required_stream_id_domain"),
                string_field("endpoint_role"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("previous_peer_stream_id"),
                string_field("endpoint_role"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2PeerLimitFrameSizeDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_payload_length"),
                int_field("allowed_max_frame_size"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("receive_limit_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
    ]
}
