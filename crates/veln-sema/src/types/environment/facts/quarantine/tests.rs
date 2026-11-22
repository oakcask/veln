use super::*;

fn declaration_snapshots(text: &str) -> (SurfaceModule, DeclarationFacts, TypeEnvironment) {
    let source = veln_source::SourceFile::new("main.veln", text);
    let parsed = veln_syntax::parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let declarations = declaration_facts(&module, None);
    let validation = path_classification_environment(&module, None, declarations.clone());
    (module, declarations, validation)
}

#[test]
fn valid_declaration_validation_keeps_the_registry_snapshot_shared() {
    let (module, mut declarations, validation) =
        declaration_snapshots("pub type State\n  pub Ready\nend\n");
    let original = declarations.adts.clone();
    quarantine_invalid_variant_refinement_declaration_types(
        &module,
        &validation,
        &mut declarations.effects,
        &mut declarations.adts,
    );
    assert!(Arc::ptr_eq(&original, &declarations.adts));
    assert!(
        declarations
            .adts
            .quarantined_type_identities()
            .next()
            .is_none()
    );
}

#[test]
fn invalid_declaration_quarantine_does_not_mutate_the_validation_snapshot() {
    let (module, mut declarations, validation) = declaration_snapshots(concat!(
        "pub type State\n  Hidden\nend\n",
        "pub type Broken\n  pub Wrapped(value: State::Hidden)\nend\n",
        "pub type BrokenAlias = Broken\n",
    ));
    let original = declarations.adts.clone();
    assert!(original.quarantined_type_identities().next().is_none());
    quarantine_invalid_variant_refinement_declaration_types(
        &module,
        &validation,
        &mut declarations.effects,
        &mut declarations.adts,
    );
    assert!(!Arc::ptr_eq(&original, &declarations.adts));
    assert!(
        declarations
            .adts
            .quarantined_type_identities()
            .next()
            .is_some()
    );
    assert!(original.quarantined_type_identities().next().is_none());
    assert!(
        validation
            .adts
            .quarantined_type_identities()
            .next()
            .is_none()
    );
}
