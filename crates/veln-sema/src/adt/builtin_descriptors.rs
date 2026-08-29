use super::builtin_codec_descriptors::codec_builtin_descriptors;
use super::builtin_core_descriptors::core_builtin_descriptors;
use super::runtime_base_variants::runtime_base_variants;
use super::runtime_connection_variants::runtime_connection_variants;
use super::runtime_hpack_variants::runtime_hpack_variants;
use super::runtime_peer_limit_variants::runtime_peer_limit_variants;
use super::runtime_protocol_variants::runtime_protocol_variants;
use super::runtime_root_descriptors::runtime_root_descriptors;
use super::runtime_support_descriptors::runtime_support_descriptors;
use super::{
    AdtDescriptor, AdtPayloadField, AdtPayloadType, AdtVariantDescriptor, AdtVariantKind,
    SourceLessNameClass, Type, Visibility,
};

pub(crate) fn build_builtin_descriptors() -> Vec<AdtDescriptor> {
    let mut descriptors = core_builtin_descriptors();
    descriptors.extend(codec_builtin_descriptors());
    descriptors.extend(runtime_root_descriptors());
    descriptors.push(runtime_diagnostic_detail_descriptor());
    descriptors.extend(runtime_support_descriptors());
    publish_runtime_detail_families(descriptors)
}

fn runtime_diagnostic_detail_descriptor() -> AdtDescriptor {
    let mut variants = runtime_base_variants();
    variants.extend(runtime_hpack_variants());
    variants.extend(runtime_connection_variants());
    variants.extend(runtime_peer_limit_variants());
    variants.extend(runtime_protocol_variants());
    AdtDescriptor {
        type_name: "RuntimeDiagnosticDetail".to_string(),
        name_class: SourceLessNameClass::Type,
        module_name: None,
        type_parameters: Vec::new(),
        variants,
        diagnostic_name: "runtimediagnosticdetail".to_string(),
        propagation: None,
        visibility: Visibility::Public,
    }
}

fn publish_runtime_detail_families(mut descriptors: Vec<AdtDescriptor>) -> Vec<AdtDescriptor> {
    let detail_index = descriptors
        .iter()
        .position(|descriptor| descriptor.type_name == "RuntimeDiagnosticDetail")
        .expect("runtime diagnostic detail descriptor");
    let mut detail = descriptors.remove(detail_index);
    let mut hpack_variants = Vec::new();
    let mut http2_variants = Vec::new();
    detail.variants.retain_mut(|variant| {
        if variant.name.starts_with("RuntimeHpack") {
            variant.coverage_case = format!("{}(_)", variant.name);
            hpack_variants.push(variant.clone());
            false
        } else if variant.name.starts_with("RuntimeHttp2") {
            variant.coverage_case = format!("{}(_)", variant.name);
            http2_variants.push(variant.clone());
            false
        } else {
            true
        }
    });
    detail.variants.extend([
        AdtVariantDescriptor {
            name: "RuntimeHttp2Diagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![AdtPayloadField {
                name: "detail".to_string(),
                ty: AdtPayloadType::Concrete(Type::named("Http2DiagnosticDetail", Vec::new())),
            }],
            coverage_case: "RuntimeHttp2Diagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
        AdtVariantDescriptor {
            name: "RuntimeHttp2HpackDiagnostic".to_string(),
            name_class: SourceLessNameClass::Constructor,
            kind: AdtVariantKind::Source,
            payload_fields: vec![AdtPayloadField {
                name: "detail".to_string(),
                ty: AdtPayloadType::Concrete(Type::named("HpackDiagnosticDetail", Vec::new())),
            }],
            coverage_case: "RuntimeHttp2HpackDiagnostic(_)".to_string(),
            visibility: Visibility::Public,
        },
    ]);
    descriptors.insert(detail_index, detail);
    descriptors.push(AdtDescriptor {
        type_name: "Http2DiagnosticDetail".to_string(),
        name_class: SourceLessNameClass::Type,
        module_name: None,
        type_parameters: Vec::new(),
        variants: http2_variants,
        diagnostic_name: "http2diagnosticdetail".to_string(),
        propagation: None,
        visibility: Visibility::Public,
    });
    descriptors.push(AdtDescriptor {
        type_name: "HpackDiagnosticDetail".to_string(),
        name_class: SourceLessNameClass::Type,
        module_name: None,
        type_parameters: Vec::new(),
        variants: hpack_variants,
        diagnostic_name: "hpackdiagnosticdetail".to_string(),
        propagation: None,
        visibility: Visibility::Public,
    });
    descriptors
}
