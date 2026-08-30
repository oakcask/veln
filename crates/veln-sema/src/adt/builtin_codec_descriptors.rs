use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

use super::descriptors::{
    AdtDescriptor, AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind,
};

pub(super) fn codec_builtin_descriptors() -> Vec<AdtDescriptor> {
    vec![
        AdtDescriptor {
            type_name: "DecodeError".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "DecodeError".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "id".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                        AdtPayloadField {
                            name: "offset".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteOffset", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "field_path".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                    ],
                    coverage_case: "DecodeError(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "DecodeErrorWithReason".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "id".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                        AdtPayloadField {
                            name: "offset".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteOffset", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "field_path".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                        AdtPayloadField {
                            name: "reason".to_string(),
                            ty: AdtPayloadType::Concrete(Type::string()),
                        },
                    ],
                    coverage_case: "DecodeErrorWithReason(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "decodeerror".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "DecodeReadiness".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "NeedBytes".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "count".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                    }],
                    coverage_case: "NeedBytes(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "NeedEnd".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: Vec::new(),
                    coverage_case: "NeedEnd".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "decodereadiness".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "DecodeStep".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: vec!["T".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Decoded".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "value".to_string(),
                            ty: AdtPayloadType::TypeParameter(0),
                        },
                        AdtPayloadField {
                            name: "consumed".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                    ],
                    coverage_case: "Decoded(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "NeedMore".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "readiness".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("DecodeReadiness", Vec::new())),
                    }],
                    coverage_case: "NeedMore(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Invalid".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "error".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("DecodeError", Vec::new())),
                    }],
                    coverage_case: "Invalid(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "decodestep".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "SchemaDispatchPayload".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: vec!["T".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Known".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "value".to_string(),
                        ty: AdtPayloadType::TypeParameter(0),
                    }],
                    coverage_case: "Known(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Unknown".to_string(),
                    name_class: SourceLessNameClass::Constructor,
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "tag".to_string(),
                            ty: AdtPayloadType::Concrete(Type::int()),
                        },
                        AdtPayloadField {
                            name: "payload".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteView", Vec::new())),
                        },
                    ],
                    coverage_case: "Unknown(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "schemadispatchpayload".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "EncodeError".to_string(),
            name_class: SourceLessNameClass::Type,
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![AdtVariantDescriptor {
                name: "EncodeError".to_string(),
                name_class: SourceLessNameClass::Constructor,
                kind: AdtVariantKind::Source,
                payload_fields: vec![
                    AdtPayloadField {
                        name: "id".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "field_path".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "reason".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                ],
                coverage_case: "EncodeError(_)".to_string(),
                visibility: Visibility::Public,
            }],
            diagnostic_name: "encodeerror".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
    ]
}
