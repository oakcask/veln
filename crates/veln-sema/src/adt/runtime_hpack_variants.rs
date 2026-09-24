use super::descriptors::AdtVariantDescriptor;
use super::runtime_variant_builders::{
    int_field, named_field, runtime_diagnostic_variant, string_field,
};

pub(super) fn runtime_hpack_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        runtime_diagnostic_variant(
            "RuntimeHpackFixtureDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_block_size"),
                int_field("observed_first_byte"),
                string_field("expected_fixture"),
                string_field("codec_module"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHpackFixtureDynamicIndexDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_block_size"),
                int_field("observed_first_byte"),
                int_field("requested_dynamic_index"),
                int_field("dynamic_table_entry_count"),
                string_field("expected_fixture"),
                string_field("codec_module"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHpackFixtureDynamicNameDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_block_size"),
                int_field("observed_first_byte"),
                int_field("requested_dynamic_index"),
                int_field("dynamic_table_entry_count"),
                string_field("expected_fixture"),
                string_field("codec_module"),
                named_field("preview", "ByteChunk"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeHpackFixtureTableSizeUpdateDiagnostic",
            vec![
                int_field("byte_offset"),
                int_field("observed_header_block_size"),
                int_field("observed_first_byte"),
                int_field("observed_header_table_size"),
                int_field("frame_kind"),
                int_field("stream_id"),
                string_field("active_state"),
                string_field("expected_fixture"),
                string_field("codec_module"),
                named_field("preview", "ByteChunk"),
            ],
        ),
    ]
}
