use super::*;

fn registry() -> AdtRegistry {
    AdtRegistry::from_parts(builtin_descriptors(), std::collections::BTreeMap::new())
}

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_string()).collect()
}

#[test]
fn constructors_match_qualified_and_unqualified_builtin_names() {
    let registry = registry();
    validate_builtin_adt_descriptors().expect("built-in ADT descriptors are valid");

    let ConstructorLookup::Found(some) = registry.constructor(&path(&["Some"]), None, &[]) else {
        panic!("Some should resolve");
    };
    assert_eq!(some.descriptor.type_name, "Option");
    assert_eq!(some.variant.name, "Some");
    assert_eq!(some.variant.coverage_case, "Some(_)");
    assert_eq!(some.variant.payload_fields[0].name, "value");

    let ConstructorLookup::Found(nil) =
        registry.nullary_constructor(&path(&["List", "Nil"]), None, &[])
    else {
        panic!("List::Nil should resolve");
    };
    assert_eq!(nil.descriptor.type_name, "List");
    assert_eq!(nil.variant.name, "Nil");
    assert_eq!(nil.variant.coverage_case, "Nil");
    assert!(nil.variant.payload_fields.is_empty());
}

#[test]
fn source_less_provider_inventory_names_builtin_adt_lookup_routes() {
    let registry = registry();
    let option = Type::named("Option", vec![Type::Unknown]);

    let descriptor = registry
        .descriptor_for_type(&option)
        .expect("built-in ADT type route");
    assert_eq!(descriptor.type_name, "Option");
    assert_eq!(descriptor.module_name, None);

    let ConstructorLookup::Found(constructor) =
        registry.constructor(&path(&["Option", "Some"]), None, &[])
    else {
        panic!("built-in ADT constructor route should resolve");
    };
    assert_eq!(constructor.descriptor.type_name, "Option");
    assert_eq!(constructor.variant.name, "Some");
}

#[test]
fn invalid_builtin_adt_type_name_reports_descriptor_details() {
    let mut descriptors = builtin_descriptors();
    descriptors[0].type_name = "option".to_string();

    let failure =
        validate_adt_lookup_descriptors("adt", &descriptors).expect_err("invalid ADT type name");
    let diagnostic = failure.diagnostic();

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "option");
    assert_eq!(failure.name_class, SourceLessNameClass::Type);
    assert_eq!(failure.required_initial(), "ascii_uppercase");
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
    assert_eq!(diagnostic.span, None);
    assert_eq!(
        diagnostic.details.to_json(),
        "{\"provider\":\"adt\",\"name\":\"option\",\"name_class\":\"type\",\"required_initial\":\"ascii_uppercase\"}"
    );
}

#[test]
fn invalid_builtin_adt_constructor_name_prevents_registry_publication() {
    let mut descriptors = builtin_descriptors();
    descriptors[0].variants[0].name = "some".to_string();

    let failure = validate_adt_lookup_descriptors("adt", &descriptors)
        .expect_err("invalid ADT constructor name");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "some");
    assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
    assert_eq!(failure.required_initial(), "ascii_uppercase");
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
}

#[test]
fn duplicate_builtin_adt_type_key_reports_lookup_key_failure() {
    let mut descriptors = builtin_descriptors();
    let duplicate = descriptors[0].clone();
    descriptors.push(duplicate);

    let failure =
        validate_adt_lookup_descriptors("adt", &descriptors).expect_err("duplicate ADT type key");
    let diagnostic = failure.diagnostic();

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "Option");
    assert_eq!(failure.name_class, SourceLessNameClass::Type);
    assert_eq!(failure.required_initial(), "ascii_uppercase");
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::DuplicateLookupKey
    );
    assert_eq!(
        diagnostic.message,
        "compiler-provided type lookup key `Option` from `adt` is duplicated"
    );
    assert_eq!(
        diagnostic.details.to_json(),
        "{\"provider\":\"adt\",\"name\":\"Option\",\"name_class\":\"type\",\"required_initial\":\"ascii_uppercase\"}"
    );
}

#[test]
fn duplicate_builtin_adt_constructor_key_reports_lookup_key_failure() {
    let mut descriptors = builtin_descriptors();
    let duplicate = descriptors[0].variants[0].clone();
    descriptors[0].variants.push(duplicate);

    let failure = validate_adt_lookup_descriptors("adt", &descriptors)
        .expect_err("duplicate ADT constructor key");
    let diagnostic = failure.diagnostic();

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "Option::Some");
    assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
    assert_eq!(failure.required_initial(), "ascii_uppercase");
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::DuplicateLookupKey
    );
    assert_eq!(
        diagnostic.message,
        "compiler-provided constructor lookup key `Option::Some` from `adt` is duplicated"
    );
    assert_eq!(
        diagnostic.details.to_json(),
        "{\"provider\":\"adt\",\"name\":\"Option::Some\",\"name_class\":\"constructor\",\"required_initial\":\"ascii_uppercase\"}"
    );
}

#[test]
fn invalid_injected_adt_descriptor_does_not_publish_lookup_registry() {
    let mut descriptors = builtin_descriptors();
    descriptors[0].variants[0].name = "some".to_string();

    let failure = AdtRegistry::from_validated_parts(descriptors, std::collections::BTreeMap::new())
        .expect_err("invalid injected ADT descriptor");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "some");
    assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
    assert_eq!(failure.reason, InvalidStandardSymbolReason::InvalidCase);
}

#[test]
fn duplicate_injected_adt_constructor_does_not_publish_lookup_registry() {
    let mut descriptors = builtin_descriptors();
    let duplicate = descriptors[0].variants[0].clone();
    descriptors[0].variants.push(duplicate);

    let failure = AdtRegistry::from_validated_parts(descriptors, std::collections::BTreeMap::new())
        .expect_err("duplicate injected ADT constructor");

    assert_eq!(failure.code(), "toolchain.invalid_symbol_case");
    assert_eq!(failure.provider, "adt");
    assert_eq!(failure.name, "Option::Some");
    assert_eq!(failure.name_class, SourceLessNameClass::Constructor);
    assert_eq!(
        failure.reason,
        InvalidStandardSymbolReason::DuplicateLookupKey
    );
}

#[test]
fn constructor_lookup_skips_unrelated_adt_variants() {
    fn candidate_scans(unrelated_count: usize) -> usize {
        let mut descriptors = builtin_descriptors();
        let template = descriptors[0].clone();
        for index in 0..unrelated_count {
            let mut descriptor = template.clone();
            descriptor.type_name = format!("Unrelated{index}");
            for (variant_index, variant) in descriptor.variants.iter_mut().enumerate() {
                variant.name = format!("Unrelated{index}Variant{variant_index}");
            }
            descriptors.push(descriptor);
        }
        let registry = AdtRegistry::from_parts(descriptors, std::collections::BTreeMap::new());
        constructor_lookup_counters::reset();
        assert!(matches!(
            registry.constructor(&path(&["Some"]), None, &[]),
            ConstructorLookup::Found(_)
        ));
        constructor_lookup_counters::candidate_scans()
    }

    let base = candidate_scans(0);
    let expanded = candidate_scans(128);
    assert_eq!(
        expanded, base,
        "unrelated ADTs must not add constructor resolution scans"
    );
}
