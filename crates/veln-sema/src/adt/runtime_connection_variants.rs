use super::*;

pub(super) fn runtime_connection_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolClosedWithPendingDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "pending_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_continuation".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "expected_stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "started_frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "started_byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "accumulated_header_block_bytes".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
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
            coverage_case: "RuntimeHttp2ProtocolClosedWithPendingDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolPartialPrefaceDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "pending_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_count".to_string(),
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
            coverage_case: "RuntimeHttp2ProtocolPartialPrefaceDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_byte".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "matched_prefix_count".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_count".to_string(),
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
            coverage_case: "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_flags".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
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
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic(_)"
                .to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolContinuationExpectedDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "started_frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "started_byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "active_continuation".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "accumulated_header_block_bytes".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
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
            coverage_case: "RuntimeHttp2ProtocolContinuationExpectedDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "actual_frame_kind".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "stream_id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::int()),
                },
                AdtPayloadField {
                    name: "expected_frame_kind".to_string(),
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
            coverage_case: "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic".to_string(),
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
                    name: "required_stream_id_domain".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
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
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic".to_string(),
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
                    name: "previous_peer_stream_id".to_string(),
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
                    name: "rule_provenance".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2PeerLimitFrameSizeDiagnostic".to_string(),
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
                    name: "allowed_max_frame_size".to_string(),
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
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                },
            ],
            coverage_case: "RuntimeHttp2PeerLimitFrameSizeDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]
}
