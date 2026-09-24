use super::descriptors::AdtVariantDescriptor;
use super::runtime_variant_builders::{
    int_field, named_field, runtime_diagnostic_variant, string_field,
};

pub(super) fn runtime_protocol_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("frame_kind"),
                int_field("stream_id"),
                int_field("expected_content_length"),
                int_field("observed_body_length"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("failed_header_fact"),
                string_field("header_name"),
                string_field("decoded_header_names"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("failed_header_fact"),
                string_field("header_name"),
                string_field("decoded_header_names"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("observed_window_increment"),
                int_field("accepted_min_window_increment"),
                int_field("accepted_max_window_increment"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic",
            vec![
                int_field("byte_offset"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("setting_identifier"),
                string_field("setting_name"),
                string_field("endpoint_role"),
                int_field("frame_kind"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolPriorityDependencyDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("dependency_stream_id"),
                string_field("active_state"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("stream_id"),
                int_field("last_stream_id"),
                string_field("shutdown_state"),
                string_field("endpoint_role"),
                string_field("rule_provenance"),
                named_field("preview", "ByteChunk"),
            ],
        ),
    ]
}
