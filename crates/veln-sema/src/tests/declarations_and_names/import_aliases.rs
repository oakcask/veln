use super::*;
use crate::semantic_model::Type;

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use platform.io\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(
        diagnostics[0].message,
        "duplicate import name `platform.io`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn explicit_import_alias_collisions_and_repeated_targets_are_static_errors() {
    for source in [
        "mod app\nuse alpha as shared\nuse beta as shared\n",
        "mod app\nuse alpha as shared\nuse beta::shared\n",
        "mod app\nuse beta::shared\nuse alpha as shared\n",
        "mod app\nuse alpha as first\nuse alpha as second\n",
        "mod app\nuse alpha as shared\nuse alpha as shared\n",
    ] {
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source)).tree);
        let diagnostics = analyze_surface_module(&module);
        let duplicates = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.duplicate")
            .collect::<Vec<_>>();
        assert_eq!(duplicates.len(), 1, "{diagnostics:#?}");
        assert_eq!(duplicates[0].related.len(), 1);
    }
}

#[test]
fn equivalent_standard_target_spellings_are_quarantined_in_both_orders() {
    for (source, later_spelling) in [
        (
            "mod app\nuse net as first from \"std\"\nuse std::net as second from \"std\"\n",
            "std::net",
        ),
        (
            "mod app\nuse std::net as second from \"std\"\nuse net as first from \"std\"\n",
            "net",
        ),
    ] {
        let mut module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source)).tree);
        for use_decl in &mut module.uses {
            use_decl.name = "std::net".to_string();
        }

        let diagnostics = analyze_surface_module(&module);
        let duplicates = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.duplicate")
            .collect::<Vec<_>>();
        assert_eq!(
            duplicates.len(),
            1,
            "source={source:?}, diagnostics={diagnostics:#?}"
        );
        assert_eq!(
            duplicates[0].message,
            format!("duplicate import name `{later_spelling}`")
        );
        assert_eq!(duplicates[0].related.len(), 1);
        assert!(
            duplicates[0].related[0]
                .to_json()
                .contains("\"start\":{\"line\":2,"),
            "source={source:?}, diagnostics={diagnostics:#?}"
        );

        for path in ["first", "second", "net", "std::net"] {
            let segments = path.split("::").map(str::to_string).collect::<Vec<_>>();
            assert!(
                crate::name_recovery::normal_imported_use_for_path(
                    &module,
                    &segments,
                    Some("app"),
                )
                .is_none(),
                "source={source:?}, path={path:?}"
            );
        }
    }
}

#[test]
fn equal_targets_in_same_spelled_modules_from_distinct_owner_packages_do_not_cross_quarantine() {
    let source = SourceFile::new(
        "owners.veln",
        concat!(
            "mod shared\n",
            "use net as local_wire from \"std\"\n",
            "use std::net as standard_wire from \"std\"\n",
        ),
    );
    let mut module = lower_surface_ast(&parse(&source).tree);
    for use_decl in &mut module.uses {
        use_decl.name = "std::net".to_string();
    }
    module.uses[1].importing_package = Some("std".to_string());

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );

    for (package, path, alias) in [
        (None, "local_wire", "local_wire"),
        (Some("std"), "standard_wire", "standard_wire"),
        (None, "net", "local_wire"),
        (Some("std"), "std::net", "standard_wire"),
    ] {
        let segments = path.split("::").map(str::to_string).collect::<Vec<_>>();
        let selected = crate::name_recovery::normal_imported_use_for_path_in_owner(
            &module,
            &segments,
            Some("shared"),
            package,
        )
        .unwrap_or_else(|| panic!("owner {package:?} should select {path}"));
        assert_eq!(selected.alias, alias);
    }
}

#[test]
fn same_spelled_declarations_from_distinct_owner_packages_do_not_collide() {
    let local_source = SourceFile::new(
        "local.veln",
        concat!(
            "fn probe() -> Int\n  1\nend\n",
            "let marker: Int = 1\n",
            "type HeaderList\n  LocalHeader(code: Int)\nend\n",
            "effect Observe\n  read() -> Int\nend\n",
            "schema Packet\n  code: Int\nend\n",
        ),
    );
    let standard_source = SourceFile::new(
        "standard.veln",
        concat!(
            "fn probe() -> Int\n  2\nend\n",
            "let marker: Int = 2\n",
            "type HeaderList\n  LocalHeader(code: String)\nend\n",
            "effect Observe\n  read() -> String\nend\n",
            "schema Packet\n  code: String\nend\n",
        ),
    );
    let local = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&local_source).tree,
        "std::http2::core".to_string(),
        local_source.span(TextRange::new(0, 0)),
    );
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        "std".to_string(),
        "http2::core".to_string(),
        standard_source.span(TextRange::new(0, 0)),
    );
    let module = SurfaceModule {
        module: local.module,
        uses: [local.uses, standard.uses].concat(),
        aliases: [local.aliases, standard.aliases].concat(),
        effects: [local.effects, standard.effects].concat(),
        handlers: [local.handlers, standard.handlers].concat(),
        types: [local.types, standard.types].concat(),
        schemas: [local.schemas, standard.schemas].concat(),
        module_values: [local.module_values, standard.module_values].concat(),
        functions: [local.functions, standard.functions].concat(),
        invalid_names: [local.invalid_names, standard.invalid_names].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
    let identities = TypeEnvironment::from_module(&module)
        .adts
        .descriptors()
        .iter()
        .filter(|descriptor| descriptor.type_name == "HeaderList")
        .map(|descriptor| descriptor.identity())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(identities.len(), 2, "{identities:#?}");
}

#[test]
fn same_spelled_effects_and_handlers_select_the_import_target_package() {
    let app_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use http2::core as wire from \"std\"\n",
            "use std::http2::core as local_core\n",
        ),
    );
    let local_source = SourceFile::new(
        "local.veln",
        concat!(
            "pub effect Observe\n  read() -> Int\nend\n",
            "pub handler recorder() for Observe\n  read() => 1\nend\n",
        ),
    );
    let standard_source = SourceFile::new(
        "standard.veln",
        concat!(
            "pub effect Observe\n  read() -> String\nend\n",
            "pub handler recorder() for Observe\n  read() => \"std\"\nend\n",
        ),
    );
    let mut app = lower_surface_ast(&parse(&app_source).tree);
    app.uses[0].name = "std::http2::core".to_string();
    let local = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&local_source).tree,
        "std::http2::core".to_string(),
        local_source.span(TextRange::new(0, 0)),
    );
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        "std".to_string(),
        "http2::core".to_string(),
        standard_source.span(TextRange::new(0, 0)),
    );
    app.effects.extend(local.effects);
    app.effects.extend(standard.effects);
    app.handlers.extend(local.handlers);
    app.handlers.extend(standard.handlers);
    let environment = TypeEnvironment::from_module(&app);

    for (qualifier, package, expected) in [
        ("local_core", None, Type::int()),
        ("wire", Some("std"), Type::string()),
    ] {
        let effect = environment
            .resolve_user_effect_path_in_owner(
                &[qualifier.to_string(), "Observe".to_string()],
                Some("app"),
                None,
            )
            .found()
            .unwrap_or_else(|| panic!("{qualifier} effect should resolve"));
        assert_eq!(effect.package_name.as_deref(), package);
        assert_eq!(effect.operations[0].return_type, expected);

        let handler = environment.handler_path_in_owner(
            &[qualifier.to_string(), "recorder".to_string()],
            Some("app"),
            None,
        );
        let HandlerPathResolution::Found(handler) = handler else {
            panic!("{qualifier} handler should resolve");
        };
        assert_eq!(handler.package_name.as_deref(), package);
    }
}

#[test]
fn same_spelled_owner_types_reject_the_other_owners_shape() {
    let app_source = SourceFile::new(
        "main.veln",
        concat!(
            "use http2::core as wire from \"std\"\n",
            "use std::http2::core as local_core\n",
            "fn reject(value: wire::Headers) -> local_core::Headers\n",
            "  value\n",
            "end\n",
            "fn reject_refinement(value: wire::HeaderList::EmptyHeaderList) -> local_core::HeaderList::LocalHeader\n",
            "  value\n",
            "end\n",
            "fn reject_constructor() -> local_core::HeaderList\n",
            "  wire::HeaderList::EmptyHeaderList\n",
            "end\n",
        ),
    );
    let local_source = SourceFile::new(
        "local.veln",
        concat!(
            "pub type HeaderList\n  LocalHeader(code: Int)\nend\n",
            "pub type Headers = HeaderList\n",
        ),
    );
    let standard_source = SourceFile::new(
        "standard.veln",
        concat!(
            "pub type HeaderList\n  EmptyHeaderList\nend\n",
            "pub type Headers = HeaderList\n",
        ),
    );
    let mut app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&app_source).tree,
        "main".to_string(),
        app_source.span(TextRange::new(0, 0)),
    );
    app.uses[0].name = "std::http2::core".to_string();
    let local = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&local_source).tree,
        "std::http2::core".to_string(),
        local_source.span(TextRange::new(0, 0)),
    );
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        "std".to_string(),
        "http2::core".to_string(),
        standard_source.span(TextRange::new(0, 0)),
    );
    let module = SurfaceModule {
        module: app.module,
        uses: [app.uses, local.uses, standard.uses].concat(),
        aliases: [app.aliases, local.aliases, standard.aliases].concat(),
        effects: [app.effects, local.effects, standard.effects].concat(),
        handlers: [app.handlers, local.handlers, standard.handlers].concat(),
        types: [app.types, local.types, standard.types].concat(),
        schemas: [app.schemas, local.schemas, standard.schemas].concat(),
        module_values: [
            app.module_values,
            local.module_values,
            standard.module_values,
        ]
        .concat(),
        functions: [app.functions, local.functions, standard.functions].concat(),
        invalid_names: [
            app.invalid_names,
            local.invalid_names,
            standard.invalid_names,
        ]
        .concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `HeaderList`, but found `HeaderList`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn equal_written_import_paths_from_distinct_origins_remain_distinct_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use net as workspace\n",
            "use net as dependency from \"dep\"\n",
            "use net as standard from \"std\"\n",
        ),
    );
    let mut module = lower_surface_ast(&parse(&source).tree);
    module.uses[0].name = "net".to_string();
    module.uses[1].name = "dep::net".to_string();
    module.uses[2].name = "std::net".to_string();

    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
    for (qualifier, package) in [
        ("workspace", None),
        ("dependency", Some("dep")),
        ("standard", Some("std")),
    ] {
        assert_eq!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &[qualifier.to_string()],
                Some("app"),
            )
            .and_then(|use_decl| use_decl.package.as_deref()),
            package
        );
    }
}

#[test]
fn repeated_identical_imports_report_only_the_first_target_origin() {
    for source in [
        "mod app\nuse alpha as shared\nuse alpha as shared\nuse alpha as shared\n",
        "mod app\nuse beta as other\nuse alpha as shared\nuse alpha as shared\nuse alpha as shared\n",
    ] {
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source)).tree);
        let diagnostics = analyze_surface_module(&module);
        let duplicates = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.duplicate")
            .collect::<Vec<_>>();

        assert_eq!(duplicates.len(), 2, "{diagnostics:#?}");
        let first_origin_line = if source.starts_with("mod app\nuse alpha") {
            2
        } else {
            3
        };
        assert!(duplicates.iter().all(|diagnostic| {
            diagnostic.message == "duplicate import name `alpha`"
                && diagnostic.related.len() == 1
                && diagnostic.related[0]
                    .to_json()
                    .contains(&format!("\"start\":{{\"line\":{first_origin_line},"))
        }));
    }

    let modules = [false, true].map(|reversed| {
        let first = SourceFile::new("a.veln", "mod app\nuse alpha as shared\n");
        let later = SourceFile::new(
            "z.veln",
            "mod app\nuse alpha as shared\nuse alpha as shared\n",
        );
        let sources = if reversed {
            vec![("app", later), ("app", first)]
        } else {
            vec![("app", first), ("app", later)]
        };
        merged_modules_with_identities(sources)
    });
    let diagnostics = modules.map(|module| analyze_surface_module(&module));
    for observed in &diagnostics {
        assert_eq!(observed.len(), 2, "{observed:#?}");
        assert!(observed.iter().all(|diagnostic| {
            diagnostic.message == "duplicate import name `alpha`"
                && diagnostic.related[0]
                    .to_json()
                    .contains("\"file\":\"a.veln\"")
        }));
    }
    assert_eq!(
        diagnostics[0]
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>(),
        diagnostics[1]
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>()
    );
}

#[test]
fn implicit_leaf_collisions_keep_exact_paths_without_duplicate_diagnostics() {
    for source in [
        "mod app\nuse alpha::shared\nuse beta::shared\n",
        "mod app\nuse beta::shared\nuse alpha::shared\n",
    ] {
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", source)).tree);
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.duplicate"),
            "source={source:?}, diagnostics={diagnostics:#?}"
        );
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &["shared".to_string()],
                Some("app"),
            )
            .is_none(),
            "source={source:?}"
        );
        for exact in [["alpha", "shared"], ["beta", "shared"]] {
            let exact = exact.into_iter().map(str::to_string).collect::<Vec<_>>();
            assert!(
                crate::name_recovery::normal_imported_use_for_path(&module, &exact, Some("app"),)
                    .is_some(),
                "source={source:?}, exact={exact:?}"
            );
        }
    }
}

#[test]
fn independent_target_and_qualifier_collisions_report_both_facts() {
    let source = SourceFile::new(
        "main.veln",
        "mod app\nuse alpha as shared\nuse beta as other\nuse alpha as other\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let duplicates = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.duplicate")
        .collect::<Vec<_>>();

    assert_eq!(duplicates.len(), 2, "{diagnostics:#?}");
    assert!(
        duplicates
            .iter()
            .all(|diagnostic| diagnostic.related.len() == 1)
    );
    assert_eq!(duplicates[0].message, "duplicate import name `alpha`");
    assert_eq!(duplicates[1].message, "duplicate import alias name `other`");
    assert!(
        duplicates[0].related[0]
            .to_json()
            .contains("\"start\":{\"line\":2,")
    );
    assert!(
        duplicates[1].related[0]
            .to_json()
            .contains("\"start\":{\"line\":3,")
    );
}

#[test]
fn same_target_qualifier_collision_keeps_its_distinct_origin() {
    let source = SourceFile::new(
        "main.veln",
        "mod app\nuse alpha as one\nuse alpha as shared\nuse alpha as shared\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let duplicates = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.duplicate")
        .collect::<Vec<_>>();

    assert_eq!(duplicates.len(), 3, "{diagnostics:#?}");
    assert_eq!(duplicates[0].message, "duplicate import name `alpha`");
    assert_eq!(duplicates[1].message, "duplicate import name `alpha`");
    assert_eq!(
        duplicates[2].message,
        "duplicate import alias name `shared`"
    );
    assert!(
        duplicates[1].related[0]
            .to_json()
            .contains("\"start\":{\"line\":2,"),
        "{diagnostics:#?}"
    );
    assert!(
        duplicates[2].related[0]
            .to_json()
            .contains("\"start\":{\"line\":3,"),
        "{diagnostics:#?}"
    );
}

#[test]
fn aliased_prelude_target_does_not_reserve_a_different_qualifier() {
    let source = SourceFile::new("main.veln", "mod app\nuse app::prelude as standard\n");
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.reserved"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        crate::name_recovery::normal_imported_use_for_path(
            &module,
            &["standard".to_string()],
            Some("app"),
        )
        .map(|use_decl| use_decl.written_name.as_str()),
        Some("app::prelude")
    );
}

#[test]
fn explicit_import_alias_does_not_collide_with_a_value_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use alpha as shared\n",
            "fn shared() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
}

#[test]
fn explicit_import_alias_does_not_publish_private_companion_targets() {
    let companion = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\nuse math as target\n",
            "pub fn exposed = target::hidden\n",
            "pub schema PublicPacket = target::Packet\n",
            "test cannot_publish() -> ()\n  let observed: Int = exposed()\n  ()\nend\n",
        ),
    );
    let target = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "fn hidden() -> Int\n  1\nend\n",
            "schema Packet\n  value: Int\nend\n",
        ),
    );
    let mut module = lower_surface_ast(&parse(&companion).tree);
    let target = lower_surface_ast(&parse(&target).tree);
    module.functions.extend(target.functions);
    module.schemas.extend(target.schemas);
    let diagnostics = analyze_surface_module(&module);
    for (id, message) in [
        ("name.unresolved", "unresolved call_target `exposed`"),
        (
            "name.visibility",
            "schema alias target `target::Packet` is private",
        ),
    ] {
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == id && diagnostic.message == message),
            "{diagnostics:#?}"
        );
    }
    assert!(resolved_schema_aliases(&module).is_empty());
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn combined_module_import_diagnostics_use_stable_source_order() {
    fn module(reversed: bool) -> SurfaceModule {
        let first = SourceFile::new("a.veln", "mod app\nuse alpha as shared\n");
        let second = SourceFile::new("z.veln", "mod app\nuse beta as shared\n");
        let sources = if reversed {
            vec![("app", second), ("app", first)]
        } else {
            vec![("app", first), ("app", second)]
        };
        merged_modules_with_identities(sources)
    }

    let forward_module = module(false);
    let reversed_module = module(true);
    for combined in [&forward_module, &reversed_module] {
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                combined,
                &["shared".to_string()],
                Some("app")
            )
            .is_none()
        );
        for exact in ["alpha", "beta"] {
            assert_eq!(
                crate::name_recovery::normal_imported_use_for_path(
                    combined,
                    &[exact.to_string()],
                    Some("app")
                )
                .map(|use_decl| use_decl.written_name.as_str()),
                Some(exact)
            );
        }
    }
    let forward = analyze_surface_module(&forward_module);
    let reversed = analyze_surface_module(&reversed_module);
    for observed in [&forward, &reversed] {
        assert_eq!(observed.len(), 1, "{observed:#?}");
        assert_eq!(observed[0].id, "name.duplicate");
        assert_eq!(
            observed[0].span.as_ref().map(|span| span.file.as_str()),
            Some("z.veln")
        );
        assert!(
            observed[0]
                .related
                .first()
                .is_some_and(|related| related.to_json().contains("a.veln")),
            "{observed:#?}"
        );
    }
    assert_eq!(
        forward
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>(),
        reversed
            .iter()
            .map(veln_diagnostics::diagnostic_to_json)
            .map(|json| json.to_json())
            .collect::<Vec<_>>()
    );
}

#[test]
fn duplicate_import_diagnostic_pipeline_classifies_once_and_scales_linearly() {
    fn work(count: usize) -> usize {
        let mut text = String::from("mod app\n");
        for index in 0..count {
            text.push_str(&format!("use dependency{index} as alias{index}\n"));
        }
        let module = lower_surface_ast(&parse(&SourceFile::new("main.veln", text)).tree);
        crate::name_recovery::reset_import_classification_work();
        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.duplicate"),
            "{diagnostics:#?}"
        );
        assert_eq!(crate::name_recovery::import_classification_builds(), 1);
        crate::name_recovery::import_classification_work()
    }

    let small = work(128);
    let large = work(256);
    assert!(
        large <= small * 2 + 16,
        "duplicate diagnostic import classification must remain linear: small={small} large={large}"
    );
}

#[test]
fn invalid_target_recovery_pipeline_classifies_once_and_scales_linearly() {
    fn work(count: usize) -> usize {
        let mut text = String::from("mod app\n");
        for index in 0..count {
            text.push_str(&format!("use Dependency{index} as alias{index}\n"));
        }
        for index in 0..count {
            text.push_str(&format!(
                "fn call{index}() -> Int\n  alias{index}::entry()\nend\n"
            ));
        }
        let mut sources = vec![SourceFile::new("main.veln", text)];
        for index in 0..count {
            sources.push(SourceFile::new(
                format!("dependency{index}.veln"),
                format!("mod dependency{index}\npub fn entry() -> Int\n  {index}\nend\n"),
            ));
        }
        let module = merged_modules(sources);
        crate::name_recovery::reset_import_classification_work();
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.invalid_case")
                .count(),
            count,
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.unresolved"),
            "{diagnostics:#?}"
        );
        assert_eq!(crate::name_recovery::import_classification_builds(), 1);
        crate::name_recovery::import_classification_work()
    }

    let small = work(128);
    let large = work(256);
    assert!(
        large <= small * 2 + 16,
        "invalid-target recovery import classification must remain linear: small={small} large={large}"
    );
}

#[test]
fn explicit_prelude_alias_is_reserved_at_the_alias_token() {
    let source = SourceFile::new("main.veln", "mod app\nuse alpha as prelude\n");
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let reserved = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.reserved")
        .expect("reserved alias diagnostic");
    assert_eq!(reserved.span.as_ref().unwrap().start.column, 14);
}

#[test]
fn duplicate_use_aliases_are_scoped_to_declaring_module() {
    let first_source = SourceFile::new(
        "first.veln",
        concat!(
            "mod first\n",
            "use shared\n",
            "fn first_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let second_source = SourceFile::new(
        "second.veln",
        concat!(
            "mod second\n",
            "use shared\n",
            "fn second_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let first = lower_surface_ast(&parse(&first_source).tree);
    let second = lower_surface_ast(&parse(&second_source).tree);
    let module = SurfaceModule {
        module: first.module,
        uses: [first.uses, second.uses].concat(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        module_values: Vec::new(),
        functions: [first.functions, second.functions].concat(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
}
