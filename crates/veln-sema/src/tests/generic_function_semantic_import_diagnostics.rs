use super::*;

fn package_module(path: &str, source_text: &str, package: &str, module: &str) -> SurfaceModule {
    let source = SourceFile::new(path, source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut lowered = veln_ast::lower_surface_ast_with_package_module_identity(
        &parsed.tree,
        package.to_string(),
        module.to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut lowered.uses {
        use_decl.module_name = Some(module.to_string());
    }
    for alias in &mut lowered.aliases {
        alias.module_name = Some(module.to_string());
    }
    for effect in &mut lowered.effects {
        effect.module_name = Some(module.to_string());
    }
    for handler in &mut lowered.handlers {
        handler.module_name = Some(module.to_string());
    }
    for schema in &mut lowered.schemas {
        schema.module_name = Some(module.to_string());
    }
    for type_decl in &mut lowered.types {
        type_decl.module_name = Some(module.to_string());
    }
    for value in &mut lowered.module_values {
        value.module_name = Some(module.to_string());
    }
    for function in &mut lowered.functions {
        function.module_name = Some(module.to_string());
    }
    lowered
}

fn extend_surface_module(module: &mut SurfaceModule, other: SurfaceModule) {
    module.uses.extend(other.uses);
    module.aliases.extend(other.aliases);
    module.effects.extend(other.effects);
    module.handlers.extend(other.handlers);
    module.schemas.extend(other.schemas);
    module.types.extend(other.types);
    module.module_values.extend(other.module_values);
    module.functions.extend(other.functions);
    module.invalid_names.extend(other.invalid_names);
}

#[test]
fn binder_cannot_shadow_an_imported_type_alias() {
    let module = merged_modules_with_identities(vec![
        (
            "api",
            SourceFile::new(
                "api.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "end\n",
                    "pub type Item = State\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use api\n",
                    "fn shadow<Item>(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    let shadow = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .expect("imported alias shadow diagnostic");
    assert_eq!(
        shadow.span.as_ref().map(|span| span.file.as_str()),
        Some("app.veln")
    );
    assert!(shadow.related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"kind\":\"shadowed_type_declaration\"")
            && json.contains("\"file\":\"api.veln\"")
            && json.contains("\"line\":4")
    }));
}

#[test]
fn binder_shadowing_accepts_consistent_imported_alias_identity_as_visible() {
    let module = merged_modules_with_identities(vec![
        (
            "base",
            SourceFile::new("base.veln", "pub type Root\n  pub Value\nend\n"),
        ),
        (
            "alpha",
            SourceFile::new("alpha.veln", "use base\npub type Shared = base::Root\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "use base\npub type Shared = base::Root\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use alpha\n",
                    "use beta\n",
                    "fn accepts(value: Shared) -> Shared\n",
                    "  value\n",
                    "end\n",
                    "fn shadow<Shared>(value: Shared) -> Shared\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || diagnostic
                    .span
                    .as_ref()
                    .is_none_or(|span| span.file.as_str() != "app.veln")
        }),
        "ordinary annotations must keep resolving the consistent identity: {diagnostics:#?}"
    );
    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        shadows[0]
            .span
            .as_ref()
            .map(|span| (span.file.as_str(), span.start.line)),
        Some(("app.veln", 6))
    );
    assert!(shadows[0].related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"kind\":\"shadowed_type_declaration\"")
            && json.contains("\"file\":\"alpha.veln\"")
            && json.contains("\"line\":2")
    }));
}

#[test]
fn binder_shadowing_keeps_distinct_imported_terminal_identities_ambiguous() {
    let module = merged_modules_with_identities(vec![
        (
            "alpha",
            SourceFile::new("alpha.veln", "pub type Shared\n  pub Alpha\nend\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "pub type Shared\n  pub Beta\nend\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use alpha\n",
                    "use beta\n",
                    "fn shadow<Shared>(value: Shared) -> Shared\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.type_parameter_shadow"),
        "distinct terminal identities must remain ambiguous rather than visible: {diagnostics:#?}"
    );
}

#[test]
fn imported_type_lookup_does_not_cross_same_spelled_modules_in_other_packages() {
    let mut project = package_module(
        "importing.veln",
        concat!(
            "use types from \"types/pkg\"\n",
            "fn imported_shadow<Shared>(value: Shared) -> Shared\n",
            "  value\n",
            "end\n",
        ),
        "importing/pkg",
        "app",
    );
    project.uses[0].name = "types".to_string();
    extend_surface_module(
        &mut project,
        package_module(
            "missing.veln",
            concat!(
                "fn local_binder<Shared>(value: Shared) -> Shared\n",
                "  value\n",
                "end\n",
                "fn unresolved_annotation(value: Shared) -> ()\n",
                "  ()\n",
                "end\n",
            ),
            "missing/pkg",
            "app",
        ),
    );
    extend_surface_module(
        &mut project,
        package_module(
            "types.veln",
            "pub type Shared\n  pub Value\nend\n",
            "types/pkg",
            "types",
        ),
    );

    let diagnostics = analyze_surface_module(&project);
    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        shadows[0].span.as_ref().map(|span| span.file.as_str()),
        Some("importing.veln"),
        "only the package that owns the import can see `Shared`: {diagnostics:#?}"
    );

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.file.as_str() == "missing.veln")
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        unresolved[0].span.as_ref().map(|span| span.start.line),
        Some(4)
    );
}

#[test]
fn qualified_private_named_type_requires_matching_package_and_module_owner() {
    let mut project = package_module(
        "consumer.veln",
        concat!(
            "use shared from \"provider/pkg\"\n",
            "fn annotations(hidden: shared::SecretType, exposed: shared::VisibleType) -> ()\n",
            "  ()\n",
            "end\n",
        ),
        "consumer/pkg",
        "shared",
    );
    project.uses[0].name = "shared".to_string();
    extend_surface_module(
        &mut project,
        package_module(
            "provider.veln",
            concat!(
                "type SecretType\n  SecretValue\nend\n",
                "pub type VisibleType\n  pub VisibleValue\nend\n",
            ),
            "provider/pkg",
            "shared",
        ),
    );

    let diagnostics = analyze_surface_module(&project);
    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        unresolved[0].message,
        "unresolved type `shared::SecretType`"
    );
    assert_eq!(
        unresolved[0]
            .span
            .as_ref()
            .map(|span| (span.file.as_str(), span.start.line)),
        Some(("consumer.veln", 2))
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || !diagnostic.message.contains("shared::VisibleType")
        }),
        "the public counterpart must resolve through the exact import: {diagnostics:#?}"
    );
}
