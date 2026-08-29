use super::*;

pub(super) fn runtime_support_descriptors() -> Vec<AdtDescriptor> {
    vec![
        AdtDescriptor {
            type_name: "RuntimeDiagnosticFieldPathSegment".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![AdtVariantDescriptor {
                name: "RuntimeDiagnosticFieldPathSegment".to_string(),
                name_class: SourceLessNameClass::Constructor,
                kind: AdtVariantKind::Source,
                payload_fields: vec![
                    AdtPayloadField {
                        name: "kind".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "name".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                ],
                coverage_case: "RuntimeDiagnosticFieldPathSegment(_)".to_string(),
                visibility: Visibility::Public,
            }],
            diagnostic_name: "runtimediagnosticfieldpathsegment".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "RuntimeByteDiagnosticFacts".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "RuntimeByteCountFacts".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "expected_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "available_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "readiness".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                    ],
                    coverage_case: "RuntimeByteCountFacts(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "RuntimeByteRangeFacts".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "requested_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "available_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                    ],
                    coverage_case: "RuntimeByteRangeFacts(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "RuntimeByteFixedValueFacts".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "expected_value".to_string(),
                            ty: AdtPayloadType::Concrete(Type::int()),
                        },
                        AdtPayloadField {
                            name: "actual_value".to_string(),
                            ty: AdtPayloadType::Concrete(Type::int()),
                        },
                    ],
                    coverage_case: "RuntimeByteFixedValueFacts(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "RuntimeByteReasonFacts".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "reason".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    }],
                    coverage_case: "RuntimeByteReasonFacts(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "runtimebytediagnosticfacts".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "RuntimeBytePreview".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "RuntimeBytePreview".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "data".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                        AdtPayloadField {
                            name: "preview_byte_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "total_byte_count".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "truncated".to_string(),
                            ty: AdtPayloadType::Concrete(Type::bool()),
                        },
                    ],
                    coverage_case: "RuntimeBytePreview(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "NoRuntimeBytePreview".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: Vec::new(),
                    coverage_case: "NoRuntimeBytePreview".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "runtimebytepreview".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "EncodeStep".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: vec!["TState".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Encoded".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "chunks".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named(
                            "List",
                            vec![Type::named("ByteChunk", Vec::new())],
                        )),
                    }],
                    coverage_case: "Encoded(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Partial".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "chunks".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named(
                                "List",
                                vec![Type::named("ByteChunk", Vec::new())],
                            )),
                        },
                        AdtPayloadField {
                            name: "produced".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "state".to_string(),
                            ty: AdtPayloadType::TypeParameter(0),
                        },
                    ],
                    coverage_case: "Partial(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Invalid".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "error".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("EncodeError", Vec::new())),
                    }],
                    coverage_case: "Invalid(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "encodestep".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
    ]
}
