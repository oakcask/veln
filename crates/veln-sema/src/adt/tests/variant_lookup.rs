use super::*;

#[test]
fn exact_variant_lookup_requires_a_unique_declaration() {
    let mut descriptor = lookup_descriptor("sample", 0);
    let unique = descriptor.variants[0].clone();
    descriptor.variants.push(unique.clone());
    let registry = AdtRegistry::from_parts(vec![descriptor], std::collections::BTreeMap::new());
    let descriptor = &registry.descriptors()[0];

    assert_eq!(
        registry.variant_for_descriptor(descriptor, "Hidden"),
        VariantLookup::Ambiguous
    );
    assert_eq!(
        registry.variant_for_descriptor(descriptor, "Missing"),
        VariantLookup::Missing
    );

    let registry = AdtRegistry::from_parts(
        vec![lookup_descriptor("sample", 0)],
        std::collections::BTreeMap::new(),
    );
    let descriptor = &registry.descriptors()[0];
    assert_eq!(
        registry.variant_for_descriptor(descriptor, "Hidden"),
        VariantLookup::Found(&unique)
    );
}
