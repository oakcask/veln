use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

use super::descriptors::{AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind};

pub(super) fn runtime_protocol_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
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
                    name: "expected_content_length".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_body_length".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
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
                    name: "failed_header_fact".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "header_name".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "decoded_header_names".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
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
                    name: "failed_header_fact".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "header_name".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "decoded_header_names".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_window_increment".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "accepted_min_window_increment".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "accepted_max_window_increment".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(_)"
                .to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "setting_identifier".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "setting_name".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "endpoint_role".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic(_)"
                .to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolPriorityDependencyDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "dependency_stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolPriorityDependencyDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "last_stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "shutdown_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "endpoint_role".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]
}
