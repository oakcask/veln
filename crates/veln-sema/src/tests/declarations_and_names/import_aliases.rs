use super::*;

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
        "mod app\nuse alpha as first\nuse alpha as second\n",
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
