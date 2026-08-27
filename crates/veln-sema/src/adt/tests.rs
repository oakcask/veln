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
