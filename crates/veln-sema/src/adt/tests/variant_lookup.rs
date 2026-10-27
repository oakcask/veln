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

#[test]
fn multi_segment_exact_import_is_selected_before_constructor_qualifier() {
    let base = AdtRegistry::from_parts(
        vec![lookup_descriptor("a::b", 0), lookup_descriptor("c", 0)],
        std::collections::BTreeMap::new(),
    );

    for source in [
        "mod consumer\nuse a::b as wire\nuse c as a\n",
        "mod consumer\nuse c as a\nuse a::b as wire\n",
        "mod consumer\nuse a::b as wire\nuse c as other\n",
    ] {
        let parsed = veln_syntax::parse(&veln_source::SourceFile::new("consumer.veln", source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = veln_ast::lower_surface_ast(&parsed.tree);
        let uses = crate::name_recovery::normal_use_decls(&module);
        let indexed = AdtRegistry::from_module_with_base(&module, &base);
        let constructor_path = path(&["a", "b", "State", "Hidden"]);

        for registry in [&indexed, &base] {
            let constructor = registry.constructor(&constructor_path, Some("consumer"), &uses);
            assert!(
                matches!(
                    constructor,
                    ConstructorLookup::Found(constructor)
                        if constructor.descriptor.module_name.as_deref() == Some("a::b")
                ),
                "source={source:?}, constructor={constructor:?}"
            );
            let candidates =
                registry.constructor_candidates(&constructor_path, Some("consumer"), &uses);
            assert_eq!(candidates.len(), 1, "source={source:?}");
            assert_eq!(
                candidates[0].descriptor.module_name.as_deref(),
                Some("a::b"),
                "source={source:?}"
            );

            let descriptor = registry
                .unique_descriptor_for_type_path("a::b::State", 0, Some("consumer"), &uses)
                .expect("the exact multi-segment import selects its source type");
            assert!(
                registry
                    .constructor_for_descriptor(
                        &constructor_path,
                        descriptor,
                        Some("consumer"),
                        &uses,
                    )
                    .is_some(),
                "source={source:?}"
            );
        }
    }
}

#[test]
fn same_written_path_from_distinct_origins_is_ambiguous_before_alias_lookup() {
    for source in [
        "mod consumer\nuse foo as local\nuse foo as external from \"pkg\"\n",
        "mod consumer\nuse foo as external from \"pkg\"\nuse foo as local\n",
    ] {
        let parsed = veln_syntax::parse(&veln_source::SourceFile::new("consumer.veln", source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let mut module = veln_ast::lower_surface_ast(&parsed.tree);
        for use_decl in &mut module.uses {
            if use_decl.package.as_deref() == Some("pkg") {
                use_decl.name = "pkg::foo".to_string();
            }
        }
        let mut external = lookup_descriptor("pkg::foo", 0);
        external.package_name = Some("pkg".to_string());
        let base = AdtRegistry::from_parts(
            vec![lookup_descriptor("foo", 0), external],
            std::collections::BTreeMap::new(),
        );
        let registry = AdtRegistry::from_module_with_base(&module, &base);
        let uses = crate::name_recovery::normal_use_decls(&module);

        assert!(
            registry
                .unique_descriptor_for_type_path("foo::State", 0, Some("consumer"), &uses)
                .is_none(),
            "source={source:?}"
        );
        assert_eq!(
            registry.constructor(&path(&["foo", "State", "Hidden"]), Some("consumer"), &uses,),
            ConstructorLookup::Ambiguous,
            "source={source:?}"
        );
        for (alias, expected_module) in [("local", "foo"), ("external", "pkg::foo")] {
            let descriptor = registry
                .unique_descriptor_for_type_path(
                    &format!("{alias}::State"),
                    0,
                    Some("consumer"),
                    &uses,
                )
                .expect("distinct aliases remain valid");
            assert_eq!(descriptor.module_name.as_deref(), Some(expected_module));
            assert!(matches!(
                registry.constructor(
                    &path(&[alias, "State", "Hidden"]),
                    Some("consumer"),
                    &uses,
                ),
                ConstructorLookup::Found(constructor)
                    if constructor.descriptor.module_name.as_deref() == Some(expected_module)
            ));
        }
    }
}

#[test]
fn canonical_standard_path_remains_an_exact_import_spelling() {
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new(
        "consumer.veln",
        "mod consumer\nuse transport\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = veln_ast::lower_surface_ast(&parsed.tree);
    module.uses[0].name = "std::transport".to_string();
    module.uses[0].package = Some(veln_stdlib::PACKAGE_NAME.to_string());
    let mut standard = lookup_descriptor("std::transport", 0);
    standard.package_name = Some(veln_stdlib::PACKAGE_NAME.to_string());
    let base = AdtRegistry::from_parts(vec![standard], std::collections::BTreeMap::new());
    let registry = AdtRegistry::from_module_with_base(&module, &base);
    let uses = crate::name_recovery::normal_use_decls(&module);

    for qualifier in ["transport", "std::transport"] {
        let descriptor = registry
            .unique_descriptor_for_type_path(
                &format!("{qualifier}::State"),
                0,
                Some("consumer"),
                &uses,
            )
            .expect("both relative and canonical standard spellings resolve");
        assert_eq!(descriptor.module_name.as_deref(), Some("std::transport"));
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
