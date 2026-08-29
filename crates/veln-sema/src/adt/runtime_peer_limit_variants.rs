use super::*;

pub(super) fn runtime_peer_limit_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_list_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "allowed_header_list_size".to_string(),
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
                    name: "receive_limit_provenance".to_string(),
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
            coverage_case: "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_header_table_size".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "allowed_header_table_size".to_string(),
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
                    name: "receive_limit_provenance".to_string(),
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
            coverage_case: "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic".to_string(),
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
                    name: "attempted_concurrent_stream_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "allowed_concurrent_stream_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "endpoint_role".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "active_state".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "receive_limit_provenance".to_string(),
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
            coverage_case: "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitSettingsValueDiagnostic".to_string(),
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
                    name: "observed_value".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "accepted_min_value".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "accepted_max_value".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "peer_limit_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2PeerLimitSettingsValueDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic".to_string(),
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
                    name: "observed_payload_length".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_payload_length".to_string(),
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
            coverage_case: "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic".to_string(),
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
                    name: "pad_length".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "remaining_payload_length".to_string(),
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
            coverage_case: "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "observed_payload_length".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "allowed_window_credit".to_string(),
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
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]
}
