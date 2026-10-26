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

#[test]
fn unqualified_descriptor_lookup_uses_only_the_current_modules_imports() {
    let (registry, uses) = unqualified_lookup_fixture();

    let exact = registry
        .descriptor_for_type_path("State", 0, Some("consumer"), &uses)
        .expect("the current module's local declaration wins");
    assert_eq!(exact.module_name.as_deref(), Some("consumer"));
    let imported_only = registry_without_consumer(&registry);
    let exact = imported_only
        .descriptor_for_type_path("State", 1, Some("consumer"), &uses)
        .expect("a use with an implicit alias still imports public names unqualified");
    assert_eq!(exact.module_name.as_deref(), Some("dep_private"));
    assert!(
        imported_only
            .descriptor_for_type_path("State", 0, Some("consumer"), &uses)
            .is_none(),
        "private imports and unrelated-module imports are not eligible"
    );
}

#[test]
fn unqualified_any_arity_lookup_uses_only_the_current_modules_imports() {
    let (registry, uses) = unqualified_lookup_fixture();
    let imported_only = registry_without_consumer(&registry);
    let consumer = imported_only
        .unique_descriptor_for_type_path_any_arity("State", Some("consumer"), &uses)
        .expect("an unrelated module's import must not make any-arity lookup ambiguous");
    assert_eq!(consumer.module_name.as_deref(), Some("dep_private"));

    let unrelated = imported_only
        .unique_descriptor_for_type_path_any_arity("State", Some("unrelated"), &uses)
        .expect("the unrelated module keeps its own import");
    assert_eq!(unrelated.module_name.as_deref(), Some("dep_public"));
}

#[test]
fn explicit_import_alias_contributes_no_bare_constructor_candidate() {
    let registry = AdtRegistry::from_parts(
        vec![lookup_descriptor("dep", 0)],
        std::collections::BTreeMap::new(),
    );
    let uses = lookup_uses("mod consumer\nuse dep as wire\n");

    assert_eq!(
        registry.constructor(&path(&["Hidden"]), Some("consumer"), &uses),
        ConstructorLookup::Missing
    );
    assert!(matches!(
        registry.constructor(
            &path(&["wire", "State", "Hidden"]),
            Some("consumer"),
            &uses,
        ),
        ConstructorLookup::Found(constructor)
            if constructor.descriptor.module_name.as_deref() == Some("dep")
    ));
    let full_path =
        registry.constructor(&path(&["dep", "State", "Hidden"]), Some("consumer"), &uses);
    assert!(
        matches!(
            full_path,
            ConstructorLookup::Found(constructor)
                if constructor.descriptor.module_name.as_deref() == Some("dep")
        ),
        "{full_path:?}"
    );
}

#[test]
fn exact_import_module_is_selected_before_constructor_members() {
    let mut unrelated = lookup_descriptor("b", 0);
    unrelated.type_name = "Other".to_string();
    unrelated.variants[0].name = "Ready".to_string();
    let registry = AdtRegistry::from_parts(
        vec![lookup_descriptor("a", 0), unrelated],
        std::collections::BTreeMap::new(),
    );

    for source in [
        "mod consumer\nuse a as b\nuse b as other\n",
        "mod consumer\nuse b as other\nuse a as b\n",
    ] {
        let uses = lookup_uses(source);
        assert_eq!(
            registry.constructor(&path(&["b", "Hidden"]), Some("consumer"), &uses),
            ConstructorLookup::Missing,
            "source={source:?}"
        );
        assert!(matches!(
            registry.constructor(
                &path(&["a", "State", "Hidden"]),
                Some("consumer"),
                &uses,
            ),
            ConstructorLookup::Found(constructor)
                if constructor.descriptor.module_name.as_deref() == Some("a")
        ));
    }
}

fn unqualified_lookup_fixture() -> (AdtRegistry, Vec<veln_ast::UseDecl>) {
    let private_dependency = lookup_descriptor("dep_private", 1);
    let public_dependency = lookup_descriptor("dep_public", 0);
    let mut inaccessible_dependency = lookup_descriptor("dep_inaccessible", 0);
    inaccessible_dependency.visibility = Visibility::Private;
    let local = lookup_descriptor("consumer", 0);
    let registry = AdtRegistry::from_parts(
        vec![
            private_dependency,
            public_dependency,
            inaccessible_dependency,
            local,
        ],
        std::collections::BTreeMap::new(),
    );
    let mut uses = lookup_uses("mod consumer\nuse dep_private\nuse dep_inaccessible\n");
    uses.extend(lookup_uses("mod unrelated\nuse dep_public\n"));
    (registry, uses)
}

fn registry_without_consumer(registry: &AdtRegistry) -> AdtRegistry {
    AdtRegistry::from_parts(
        registry
            .descriptors()
            .iter()
            .filter(|descriptor| descriptor.module_name.as_deref() != Some("consumer"))
            .cloned()
            .collect(),
        std::collections::BTreeMap::new(),
    )
}
