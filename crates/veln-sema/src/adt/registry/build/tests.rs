use super::*;

fn module(source: &str) -> SurfaceModule {
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new("declarations.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    veln_ast::lower_surface_ast(&parsed.tree)
}

#[test]
fn type_free_scopes_share_declarations_and_keep_imports_and_quarantine_isolated() {
    let base = AdtRegistry::from_module(&module(
        "mod library\npub type Packet\n  PacketValue\nend\n",
    ));
    let mut derived = AdtRegistry::from_module_with_base(
        &module("mod consumer\nuse library\nfn entry() -> Int\n  1\nend\n"),
        &base,
    );
    assert!(Arc::ptr_eq(&base.declarations, &derived.declarations));
    let uses = derived.import_scope();
    let descriptor = derived
        .unique_descriptor_for_type_path("Packet", 0, Some("consumer"), &uses)
        .expect("restore inherited type lookup through the derived import scope");
    assert_eq!(descriptor.module_name.as_deref(), Some("library"));
    assert!(
        base.import_scope()
            .iter()
            .all(|use_decl| use_decl.alias != "wire")
    );
    assert!(derived.imported_type_path_has_consistent_identity(
        "Packet",
        0,
        Some("consumer"),
        None,
        &uses,
    ));

    derived.quarantine_public_source_types(&HashSet::from([(
        None,
        Some("library".to_string()),
        "Packet".to_string(),
    )]));
    assert!(!Arc::ptr_eq(&base.declarations, &derived.declarations));
    let uses = derived.import_scope();
    assert!(!derived.imported_type_path_has_consistent_identity(
        "Packet",
        0,
        Some("consumer"),
        None,
        &uses,
    ));
    for (registry, expected) in [(&base, Visibility::Public), (&derived, Visibility::Private)] {
        let descriptor = registry
            .descriptors()
            .iter()
            .find(|descriptor| descriptor.module_name.as_deref() == Some("library"))
            .expect("retain the inherited declaration while isolating quarantine");
        assert_eq!(descriptor.visibility, expected);
    }
}

#[test]
fn inherited_variance_normalization_is_preserved_after_metadata_mutation() {
    let mut base = AdtRegistry::from_module(&module(
        "mod library\npub type Positive<T>\n  PositiveValue(T)\nend\n",
    ));
    let identity = base
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Positive")
        .expect("retain the source type for variance normalization")
        .identity();
    base.clear_parameter_variances_for_identity(&identity);
    assert!(
        base.parameter_variances_for_identity(&identity, 1)
            .is_none()
    );

    let derived = AdtRegistry::from_module_with_base(&module("mod consumer\n"), &base);
    assert_eq!(
        derived.parameter_variances_for_identity(&identity, 1),
        Some([crate::adt::variance::Variance::Invariant].as_slice())
    );
    assert!(
        base.parameter_variances_for_identity(&identity, 1)
            .is_none()
    );
}
