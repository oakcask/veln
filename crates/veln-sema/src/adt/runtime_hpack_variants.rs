use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

use super::descriptors::{AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind};

pub(super) fn runtime_hpack_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        AdtVariantDescriptor {
            name: "RuntimeHpackFixtureDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_block_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_first_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_fixture".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "codec_module".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHpackFixtureDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHpackFixtureDynamicIndexDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_block_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_first_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "requested_dynamic_index".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "dynamic_table_entry_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_fixture".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "codec_module".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHpackFixtureDynamicIndexDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHpackFixtureDynamicNameDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_block_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_first_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "requested_dynamic_index".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "dynamic_table_entry_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_fixture".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "codec_module".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHpackFixtureDynamicNameDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHpackFixtureTableSizeUpdateDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_block_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_first_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_table_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "expected_fixture".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "codec_module".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHpackFixtureTableSizeUpdateDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]
}
