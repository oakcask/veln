use super::*;

fn registry() -> AdtRegistry {
    AdtRegistry {
        descriptors: builtin_descriptors(),
        companion_access_targets: std::collections::BTreeMap::new(),
    }
}

fn path(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| (*part).to_string()).collect()
}

#[test]
fn constructors_match_qualified_and_unqualified_builtin_names() {
    let registry = registry();
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
