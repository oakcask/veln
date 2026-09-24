use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

use super::descriptors::{AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind};

pub(super) fn runtime_diagnostic_variant(
    name: &str,
    payload_fields: Vec<AdtPayloadField>,
) -> AdtVariantDescriptor {
    AdtVariantDescriptor {
        name: name.to_string(),
        name_class: SourceLessNameClass::Constructor,
        kind: AdtVariantKind::Source,
        payload_fields,
        coverage_case: format!("{name}(_)"),
        visibility: Visibility::Public,
    }
}

pub(super) fn field(name: &str, ty: Type) -> AdtPayloadField {
    AdtPayloadField {
        name: name.to_string(),
        ty: AdtPayloadType::Concrete(ty),
    }
}

pub(super) fn int_field(name: &str) -> AdtPayloadField {
    field(name, Type::int())
}

pub(super) fn string_field(name: &str) -> AdtPayloadField {
    field(name, Type::string())
}

pub(super) fn named_field(name: &str, type_name: &str) -> AdtPayloadField {
    field(name, Type::named(type_name, Vec::new()))
}

pub(super) fn diagnostic_path_field() -> AdtPayloadField {
    field(
        "field_path",
        Type::named(
            "List",
            vec![Type::named("RuntimeDiagnosticFieldPathSegment", Vec::new())],
        ),
    )
}
