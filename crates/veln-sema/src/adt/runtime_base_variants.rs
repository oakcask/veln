use super::descriptors::AdtVariantDescriptor;
use super::runtime_variant_builders::{
    diagnostic_path_field, named_field, runtime_diagnostic_variant, string_field,
};

pub(super) fn runtime_base_variants() -> Vec<AdtVariantDescriptor> {
    vec![
        runtime_diagnostic_variant(
            "RuntimeByteDiagnostic",
            vec![
                named_field("byte_offset", "ByteOffset"),
                diagnostic_path_field(),
                named_field("facts", "RuntimeByteDiagnosticFacts"),
                named_field("preview", "RuntimeBytePreview"),
            ],
        ),
        runtime_diagnostic_variant(
            "RuntimeValueDiagnostic",
            vec![diagnostic_path_field(), string_field("reason")],
        ),
    ]
}
