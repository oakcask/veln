use super::*;

pub(super) fn runtime_base_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        AdtVariantDescriptor {
            name: "RuntimeByteDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "byte_offset".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("ByteOffset", Vec::new())),
                },
                AdtPayloadField {
                    name: "field_path".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named(
                        "List",
                        vec![Type::named("RuntimeDiagnosticFieldPathSegment", Vec::new())],
                    )),
                },
                AdtPayloadField {
                    name: "facts".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named(
                        "RuntimeByteDiagnosticFacts",
                        Vec::new(),
                    )),
                },
                AdtPayloadField {
                    name: "preview".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named("RuntimeBytePreview", Vec::new())),
                },
            ],
            coverage_case: "RuntimeByteDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeValueDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "field_path".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named(
                        "List",
                        vec![Type::named("RuntimeDiagnosticFieldPathSegment", Vec::new())],
                    )),
                },
                AdtPayloadField {
                    name: "reason".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
            ],
            coverage_case: "RuntimeValueDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]
}
