use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

use super::descriptors::{
    AdtDescriptor, AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind,
};

pub(super) fn runtime_root_descriptors() -> Vec<AdtDescriptor> {
    vec![AdtDescriptor {
        type_name: "RuntimeDiagnostic".to_string(),
        name_class: SourceLessNameClass::Type,
        module_name: None,
        type_parameters: Vec::new(),
        variants: vec![AdtVariantDescriptor {
            name: "RuntimeDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![
                AdtPayloadField {
                    name: "id".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "message".to_string(),
                    ty: AdtPayloadType::Concrete(Type::string()),
                },
                AdtPayloadField {
                    name: "detail".to_string(),
                    ty: AdtPayloadType::Concrete(Type::named(
                        "RuntimeDiagnosticDetail",
                        Vec::new(),
                    )),
                },
            ],
            coverage_case: "RuntimeDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        }],
        diagnostic_name: "runtimediagnostic".to_string(),
        propagation: None,
        visibility: Visibility::Public,
    }]
}
