use super::*;

#[test]
fn schema_alias_target_import_resolution_is_indexed_once() {
    for count in [100usize, 200, 400] {
        let mut facade_text = String::from("mod facade\n");
        for index in 0..count.saturating_sub(1) {
            facade_text.push_str(&format!("use unused{index}\n"));
        }
        facade_text.push_str("use core\n\n");
        let mut core_text = String::from("mod core\n\n");
        for index in 0..count {
            facade_text.push_str(&format!("pub schema Alias{index} = core::Packet{index}\n"));
            core_text.push_str(&format!("pub schema Packet{index}\n  value: Int\nend\n\n"));
        }
        let facade_source = SourceFile::new("facade.veln", &facade_text);
        let facade = parse(&facade_source);
        assert!(facade.diagnostics.is_empty(), "{:#?}", facade.diagnostics);
        let core_source = SourceFile::new("core.veln", &core_text);
        let core = parse(&core_source);
        assert!(core.diagnostics.is_empty(), "{:#?}", core.diagnostics);
        let mut module = lower_surface_ast(&facade.tree);
        module.schemas.extend(lower_surface_ast(&core.tree).schemas);

        reset_schema_alias_target_import_work();
        assert_eq!(resolved_schema_aliases(&module).len(), count);
        assert_eq!(
            schema_alias_target_import_work(),
            (count, count),
            "imports must be indexed once and each qualified alias must use one route lookup"
        );
    }
}

#[test]
fn schema_alias_chain_resolution_work_is_linear() {
    for count in [64usize, 128, 256] {
        let mut source = String::from("mod facade\n\npub schema Packet\n  value: Int\nend\n\n");
        source.push_str("pub schema Alias0 = Packet\n");
        for index in 1..count {
            source.push_str(&format!("pub schema Alias{index} = Alias{}\n", index - 1));
        }
        let parsed = parse(&SourceFile::new("facade.veln", &source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        reset_schema_alias_chain_resolution_work();
        assert_eq!(resolved_schema_alias_chains(&module).len(), count);
        assert!(
            schema_alias_chain_resolution_work() <= count * 2,
            "chain resolution must visit each alias a bounded number of times"
        );
    }
}

#[test]
fn schema_alias_chain_shared_suffix_and_disconnected_cycle_stay_bounded() {
    for count in [64usize, 128, 256] {
        let mut source = String::from("mod facade\n\npub schema Packet\n  value: Int\nend\n\n");
        source.push_str("pub schema Shared0 = Packet\n");
        for index in 1..count {
            source.push_str(&format!("pub schema Shared{index} = Shared{}\n", index - 1));
        }
        for index in 0..count {
            source.push_str(&format!("pub schema Left{index} = Shared{}\n", count - 1));
            source.push_str(&format!("pub schema Right{index} = Shared{}\n", count - 1));
        }
        source.push_str("pub schema CycleA = CycleB\npub schema CycleB = CycleA\n");

        let parsed = parse(&SourceFile::new("facade.veln", &source));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        reset_schema_alias_chain_resolution_work();
        let resolved = resolved_schema_alias_chains(&module);
        assert_eq!(resolved.len(), count * 3);
        let work = schema_alias_chain_resolution_work();
        assert!(
            work <= count * 8 + 8,
            "shared suffixes and disconnected cycles must not cause repeated walks: work={work} count={count}"
        );
    }
}

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
        "duplicate import path name `platform.io`"
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
fn unaliased_imports_with_colliding_leaf_names_remain_valid() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use transport::net\n",
            "use net\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_written_import_path_precedes_an_explicit_qualifier_in_both_orders() {
    for imports in [
        "use first as second\nuse second as exact\n",
        "use second as exact\nuse first as second\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app\n{imports}fn main() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let path = vec!["second".to_string()];

        let selected =
            crate::name_recovery::normal_imported_use_for_path(&module, &path, Some("app"))
                .expect("the exact written path must remain selectable");
        assert_eq!(selected.written_name, "second");
        assert_eq!(selected.alias, "exact");

        let schema_selected = crate::name_recovery::schema_composition_imported_use_for_path(
            &module,
            &path,
            Some("app"),
        )
        .expect("schema lookup must share exact-path precedence");
        assert_eq!(schema_selected.written_name, "second");
        assert_eq!(schema_selected.alias, "exact");
    }
}

#[test]
fn distinct_package_origins_make_an_exact_written_path_ambiguous_in_both_orders() {
    for imports in [
        "use shared as first from \"pkg-a\"\nuse shared as second from \"pkg-b\"\n",
        "use shared as second from \"pkg-b\"\nuse shared as first from \"pkg-a\"\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app\n{imports}fn main() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let path = vec!["shared".to_string()];

        assert!(
            crate::name_recovery::normal_imported_use_for_path(&module, &path, Some("app"))
                .is_none()
        );
        assert!(
            crate::name_recovery::schema_composition_imported_use_for_path(
                &module,
                &path,
                Some("app")
            )
            .is_none()
        );
    }
}

#[test]
fn repeated_target_with_distinct_explicit_aliases_is_quarantined_in_both_orders() {
    for imports in [
        "use shared as first\nuse shared as second\n",
        "use shared as second\nuse shared as first\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app\n{imports}fn main() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| (diagnostic.id.as_str(), diagnostic.message.as_str()))
                .collect::<Vec<_>>(),
            [("name.duplicate", "duplicate import path name `shared`")],
            "{diagnostics:#?}"
        );
        for qualifier in ["first", "second", "shared"] {
            assert!(
                crate::name_recovery::normal_imported_use_for_path(
                    &module,
                    &[qualifier.to_string()],
                    Some("app")
                )
                .is_none(),
                "duplicate target must quarantine {qualifier:?}"
            );
        }
    }
}

#[test]
fn target_and_qualifier_overlap_reports_one_or_two_proven_conflicts() {
    let same_origin = lower_surface_ast(
        &parse(&SourceFile::new(
            "same.veln",
            "mod app\nuse alpha as shared\nuse alpha as shared\n",
        ))
        .tree,
    );
    let same_diagnostics = analyze_surface_module(&same_origin);
    assert_eq!(same_diagnostics.len(), 1, "{same_diagnostics:#?}");
    assert_eq!(
        same_diagnostics[0].message,
        "duplicate import path name `alpha`"
    );

    let different_origins = lower_surface_ast(
        &parse(&SourceFile::new(
            "different.veln",
            concat!(
                "mod app\n",
                "use alpha as first\n",
                "use beta as shared\n",
                "use alpha as shared\n",
            ),
        ))
        .tree,
    );
    let different_diagnostics = analyze_surface_module(&different_origins);
    assert_eq!(
        different_diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "duplicate import path name `alpha`",
            "duplicate import alias name `shared`",
        ],
        "{different_diagnostics:#?}"
    );
    assert_eq!(different_diagnostics[0].related.len(), 1);
    assert_eq!(different_diagnostics[1].related.len(), 1);
}

#[test]
fn explicit_qualifier_collisions_never_select_by_member_availability() {
    for imports in [
        "use first as shared\nuse second as shared\n",
        "use second as shared\nuse first as shared\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app\n{imports}fn main() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            ["duplicate import alias name `shared`"],
            "{diagnostics:#?}"
        );
        assert!(
            crate::name_recovery::normal_imported_use_for_path(
                &module,
                &["shared".to_string()],
                Some("app")
            )
            .is_none()
        );
    }
}

#[test]
fn explicit_prelude_rules_do_not_cross_declaration_namespaces() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use target as prelude\n",
            "use prelude as other\n",
            "use target::shared as imported\n",
            "fn imported() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.reserved")
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["import alias `prelude` conflicts with the standard prelude"],
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.duplicate"
                || !diagnostic.message.contains("import alias name `imported`")
        }),
        "a function and import qualifier use separate namespaces: {diagnostics:#?}"
    );
    let other = crate::name_recovery::normal_imported_use_for_path(
        &module,
        &["other".to_string()],
        Some("app"),
    )
    .expect("an explicitly renamed target leaf named prelude remains eligible");
    assert_eq!(other.written_name, "prelude");
}

#[test]
fn eligible_implicit_package_qualifiers_collide_and_remain_unresolved() {
    for imports in [
        "use first::net from \"pkg-a\"\nuse second::net from \"pkg-b\"\n",
        "use second::net from \"pkg-b\"\nuse first::net from \"pkg-a\"\n",
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!("mod app\n{imports}fn main() -> ()\n  ()\nend\n"),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let diagnostics = analyze_surface_module(&module);
        let duplicates = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.duplicate")
            .collect::<Vec<_>>();

        assert_eq!(duplicates.len(), 1, "{diagnostics:#?}");
        assert_eq!(duplicates[0].message, "duplicate import alias name `net`");
        assert_eq!(duplicates[0].related.len(), 1);
        assert_eq!(
            duplicates[0]
                .span
                .as_ref()
                .expect("primary span")
                .start
                .line,
            3
        );
        assert!(
            duplicates[0].related[0]
                .to_json()
                .contains("\"start\":{\"line\":2,")
        );

        let qualifier = vec!["net".to_string()];
        assert!(
            crate::name_recovery::normal_imported_use_for_path(&module, &qualifier, Some("app"))
                .is_none()
        );
    }
}

#[test]
fn qualifier_collisions_do_not_select_by_a_unique_member_leaf() {
    for imports in [
        "use first::net from \"pkg-a\"\nuse second::net from \"pkg-b\"\n",
        "use second::net from \"pkg-b\"\nuse first::net from \"pkg-a\"\n",
    ] {
        let consumer = SourceFile::new(
            "main.veln",
            format!("{imports}fn main() -> Int\n  only_first()\nend\n"),
        );
        let module = merged_modules_with_identities(vec![
            (
                "first::net",
                SourceFile::new("first-net.veln", "pub fn only_first() -> Int\n  1\nend\n"),
            ),
            (
                "second::net",
                SourceFile::new("second-net.veln", "pub fn other() -> Int\n  2\nend\n"),
            ),
            ("main", consumer),
        ]);
        let diagnostics = analyze_surface_module(&module);

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved call_target `only_first`"
            }),
            "{diagnostics:#?}"
        );
        let (_, lowered) = check_project_surface_module(&module);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn combined_source_qualifier_diagnostics_use_lexical_source_order() {
    let mut stable_diagnostics = None;
    for reversed in [false, true] {
        let first = SourceFile::new("a.veln", "mod app\nuse first as shared\n");
        let second = SourceFile::new("b.veln", "mod app\nuse second as shared\n");
        let sources = if reversed {
            vec![("app", second), ("app", first)]
        } else {
            vec![("app", first), ("app", second)]
        };
        let module = merged_modules_with_identities(sources);
        let diagnostics = analyze_surface_module(&module);
        let rendered = format!("{diagnostics:#?}");
        if let Some(expected) = &stable_diagnostics {
            assert_eq!(
                &rendered, expected,
                "source discovery order changed diagnostics"
            );
        } else {
            stable_diagnostics = Some(rendered);
        }
        let duplicate = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "name.duplicate")
            .expect("qualifier collision");

        assert_eq!(
            duplicate.span.as_ref().expect("primary span").file.as_str(),
            "b.veln"
        );
        assert_eq!(duplicate.related.len(), 1);
        assert!(
            duplicate.related[0]
                .to_json()
                .contains("\"file\":\"a.veln\"")
        );
    }
}

#[test]
fn combined_source_alias_selection_is_independent_of_discovery_order() {
    for reversed in [false, true] {
        let first = SourceFile::new("a.veln", "mod app\nuse first as selected\n");
        let second = SourceFile::new("b.veln", "mod app\nuse second as other\n");
        let sources = if reversed {
            vec![("app", second), ("app", first)]
        } else {
            vec![("app", first), ("app", second)]
        };
        let module = merged_modules_with_identities(sources);

        for (qualifier, target) in [("selected", "first"), ("other", "second")] {
            let selected = crate::name_recovery::normal_imported_use_for_path(
                &module,
                &[qualifier.to_string()],
                Some("app"),
            )
            .expect("combined-module alias must remain selectable");
            assert_eq!(selected.name, target);
        }
    }
}

#[test]
fn syntax_recovered_alias_does_not_report_reserved_or_qualifier_collision() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use prelude as\n",
            "use target as prelude\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(!parsed.diagnostics.is_empty());
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.reserved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
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

#[test]
fn public_function_alias_rejects_type_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub fn parse = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `Document` is a type, not a function"
    }));
}

#[test]
fn public_type_alias_rejects_function_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub type Document = parse\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `parse` is a function, not a type"
    }));
}

#[test]
fn public_type_alias_resolves_transitive_type_target() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = First\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    assert!(
        TypeEnvironment::from_module(&module)
            .adts
            .descriptors()
            .iter()
            .any(|descriptor| descriptor.type_name == "Second")
    );
}

#[test]
fn duplicate_type_alias_keeps_unresolved_target_diagnostic_in_both_orders() {
    for invalid_first in [false, true] {
        assert_duplicate_type_alias_target_diagnostic(
            invalid_first,
            "pub type Choice = Missing\n",
            "Missing",
            "name.unresolved",
            "unresolved type alias target `Missing`",
            &["\"expected_kind\":\"type\"", "\"target\":\"Missing\""],
        );
    }
}

#[test]
fn duplicate_type_alias_keeps_wrong_kind_target_diagnostic_in_both_orders() {
    for invalid_first in [false, true] {
        assert_duplicate_type_alias_target_diagnostic(
            invalid_first,
            "pub type Choice = action\n",
            "action",
            "name.kind_mismatch",
            "public alias target `action` is a function, not a type",
            &[
                "\"expected_kind\":\"type\"",
                "\"actual_kind\":\"function\"",
                "\"target\":\"action\"",
            ],
        );
    }
}

#[test]
fn duplicate_type_alias_resolution_uses_source_identity_in_combined_modules() {
    let valid_source = SourceFile::new(
        "valid.veln",
        concat!(
            "mod spec.api\n",
            "type State\n",
            "  Ready\n",
            "end\n",
            "pub type Choice = State\n",
        ),
    );
    let invalid_source =
        SourceFile::new("invalid.veln", "mod spec.api\npub type Choice = Missing\n");
    let valid = parse(&valid_source);
    let invalid = parse(&invalid_source);
    assert!(valid.diagnostics.is_empty(), "{:#?}", valid.diagnostics);
    assert!(invalid.diagnostics.is_empty(), "{:#?}", invalid.diagnostics);
    let mut module = lower_surface_ast(&valid.tree);
    let invalid_module = lower_surface_ast(&invalid.tree);
    let invalid_alias = invalid_module.aliases[0].clone();
    module.aliases.extend(invalid_module.aliases);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type alias name `Choice`"
    }));
    let unresolved = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved type alias target `Missing`"
        })
        .expect("invalid alias target diagnostic");
    assert_eq!(unresolved.span.as_ref(), Some(&invalid_alias.span));
}

fn assert_duplicate_type_alias_target_diagnostic(
    invalid_first: bool,
    invalid_declaration: &str,
    invalid_target: &str,
    target_diagnostic_id: &str,
    target_message: &str,
    expected_detail_fragments: &[&str],
) {
    let valid_declaration = "pub type Choice = State\n";
    let declarations = if invalid_first {
        format!("{invalid_declaration}{valid_declaration}")
    } else {
        format!("{valid_declaration}{invalid_declaration}")
    };
    let source = SourceFile::new(
        "api.veln",
        format!("type State\n  Ready\nend\nfn action() -> ()\n  ()\nend\n{declarations}"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let invalid_alias = module
        .aliases
        .iter()
        .find(|alias| {
            alias
                .target
                .last()
                .is_some_and(|target| target == invalid_target)
        })
        .expect("invalid duplicate alias");

    let diagnostics = analyze_surface_module(&module);
    let duplicate = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate type alias name `Choice`"
        })
        .expect("duplicate alias diagnostic");
    let target_diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == target_diagnostic_id && diagnostic.message == target_message
        })
        .expect("invalid alias target diagnostic");

    assert!(duplicate.span.is_some());
    assert_eq!(target_diagnostic.span.as_ref(), Some(&invalid_alias.span));
    let details = target_diagnostic.details.to_json();
    assert!(
        details.contains(&format!(
            "\"node_id\":\"{}\"",
            invalid_alias.node_id.display("alias")
        )),
        "details={details}"
    );
    for fragment in expected_detail_fragments {
        assert!(details.contains(fragment), "details={details}");
    }
}

#[test]
fn public_alias_rejects_unresolved_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub fn parse = impl::parse\n",
            "pub type Document = impl::Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `impl::parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `impl::Document`"
    }));
}

#[test]
fn public_type_alias_resolves_nested_import_path_target() {
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "use http2::core::detail\n",
            "pub type DocumentAlias = http2::core::detail::Document\n",
        ),
    );
    let detail_source = SourceFile::new(
        "detail.veln",
        concat!("pub type Document\n", "  pub Text(String)\n", "end\n",),
    );
    let api = lower_surface_ast_with_module_identity(
        &parse(&api_source).tree,
        "std::http2::core".to_string(),
        api_source.span(TextRange::new(0, 0)),
    );
    let detail = lower_surface_ast_with_module_identity(
        &parse(&detail_source).tree,
        "std::http2::core::detail".to_string(),
        detail_source.span(TextRange::new(0, 0)),
    );
    let module = SurfaceModule {
        module: api.module,
        uses: [api.uses, detail.uses].concat(),
        aliases: [api.aliases, detail.aliases].concat(),
        effects: [api.effects, detail.effects].concat(),
        handlers: [api.handlers, detail.handlers].concat(),
        types: [api.types, detail.types].concat(),
        schemas: [api.schemas, detail.schemas].concat(),
        functions: [api.functions, detail.functions].concat(),
        invalid_names: [api.invalid_names, detail.invalid_names].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || diagnostic.message
                    != "unresolved type alias target `http2::core::detail::Document`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .adts
            .descriptors()
            .iter()
            .any(|descriptor| {
                descriptor.module_name.as_deref() == Some("std::http2::core")
                    && descriptor.type_name == "DocumentAlias"
            })
    );
}

#[test]
fn public_type_alias_preserves_target_nominal_identity_across_module_calls() {
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "use http2::core::detail\n",
            "pub type DocumentAlias = http2::core::detail::Document\n",
            "pub fn forward() -> DocumentAlias\n",
            "  http2::core::detail::make()\n",
            "end\n",
        ),
    );
    let detail_source = SourceFile::new(
        "detail.veln",
        concat!(
            "pub type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub fn make() -> Document\n",
            "  Text(\"ok\")\n",
            "end\n",
        ),
    );
    let api = lower_surface_ast_with_module_identity(
        &parse(&api_source).tree,
        "std::http2::core".to_string(),
        api_source.span(TextRange::new(0, 0)),
    );
    let detail = lower_surface_ast_with_module_identity(
        &parse(&detail_source).tree,
        "std::http2::core::detail".to_string(),
        detail_source.span(TextRange::new(0, 0)),
    );
    let module = SurfaceModule {
        module: api.module,
        uses: [api.uses, detail.uses].concat(),
        aliases: [api.aliases, detail.aliases].concat(),
        effects: [api.effects, detail.effects].concat(),
        handlers: [api.handlers, detail.handlers].concat(),
        types: [api.types, detail.types].concat(),
        schemas: [api.schemas, detail.schemas].concat(),
        functions: [api.functions, detail.functions].concat(),
        invalid_names: [api.invalid_names, detail.invalid_names].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn public_member_alias_resolves_but_its_import_qualifier_does_not_leak() {
    let module = merged_modules_with_identities(vec![
        (
            "helpers",
            SourceFile::new("helpers.veln", "pub fn answer() -> Int\n  42\nend\n"),
        ),
        (
            "api",
            SourceFile::new(
                "api.veln",
                "use helpers as tools\npub fn answer = tools::answer\n",
            ),
        ),
        (
            "consumer",
            SourceFile::new(
                "consumer.veln",
                concat!(
                    "use api\n",
                    "fn published() -> Int\n",
                    "  api::answer()\n",
                    "end\n",
                    "fn leaked() -> Int\n",
                    "  tools::answer()\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["unresolved call_target `tools::answer`"],
        "{diagnostics:#?}"
    );
}

#[test]
fn public_alias_target_leaf_casing_reports_before_independent_target_failures() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn wrongKind = Document\n",
            "pub type WrongKind = parse\n",
            "pub fn missing = Missing\n",
            "pub type MissingType = missing_type\n",
            "pub schema Packet = schema_impl::packet\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        alias_target_observations(&diagnostics),
        vec![
            (
                "name.invalid_case",
                "function alias target `Document` must start with an ASCII lowercase letter"
            ),
            (
                "name.invalid_case",
                "type alias target `parse` must start with an ASCII uppercase letter"
            ),
            (
                "name.invalid_case",
                "function alias target `Missing` must start with an ASCII lowercase letter"
            ),
            (
                "name.invalid_case",
                "type alias target `missing_type` must start with an ASCII uppercase letter"
            ),
            (
                "name.kind_mismatch",
                "public alias target `Document` is a type, not a function"
            ),
            (
                "name.kind_mismatch",
                "public alias target `parse` is a function, not a type"
            ),
            (
                "name.unresolved",
                "unresolved function alias target `Missing`"
            ),
            (
                "name.unresolved",
                "unresolved type alias target `missing_type`"
            ),
            (
                "name.unresolved",
                "unresolved schema alias target `schema_impl::packet`"
            ),
        ],
        "{diagnostics:#?}"
    );
    assert_first_alias_target_invalid_case(&diagnostics);
    assert_invalid_alias_targets_are_quarantined(&module, &diagnostics);
}

fn alias_target_observations(diagnostics: &[Diagnostic]) -> Vec<(&str, &str)> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                || diagnostic.id == "name.kind_mismatch"
                || diagnostic.id == "name.unresolved"
        })
        .map(|diagnostic| (diagnostic.id.as_str(), diagnostic.message.as_str()))
        .collect()
}

fn assert_first_alias_target_invalid_case(diagnostics: &[Diagnostic]) {
    let invalid = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("invalid target diagnostic");
    let span = invalid.span.as_ref().expect("invalid target span");
    assert_eq!(
        (span.start.line, span.start.column, span.end.column),
        (8, 20, 28)
    );
    let details = invalid.details.to_json();
    assert!(details.contains("\"occurrence\":\"alias_target\""));
    assert!(details.contains("\"name\":\"Document\""));
    assert!(details.contains("\"name_class\":\"function\""));
    assert!(details.contains("\"required_initial\":\"ascii_lowercase\""));
    assert!(details.contains("\"observed_initial\":\"ascii_uppercase\""));
}

fn assert_invalid_alias_targets_are_quarantined(
    surface: &SurfaceModule,
    diagnostics: &[Diagnostic],
) {
    assert!(
        !diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message.contains("schema alias target")
        }),
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(surface);
    assert!(environment.function("wrongKind").is_none());
    assert_eq!(
        environment
            .schema_reference_error(&["WrongKind".to_string()], Some("spec.api"))
            .kind,
        SchemaReferenceErrorKind::Unresolved
    );
    assert!(
        environment
            .adts
            .descriptors()
            .iter()
            .all(|descriptor| descriptor.type_name != "WrongKind")
    );
}

#[test]
fn public_schema_alias_with_invalid_target_leaf_does_not_enter_schema_namespace() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "schema Packet\n",
            "  format binary\n",
            "  byte: UInt8\n",
            "end\n",
            "pub schema Alias = Packet\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let target_span = module.aliases[0].target_spans[0].clone();
    module.invalid_names.push(InvalidName {
        name: "Packet".to_string(),
        class: NameClass::Function,
        occurrence: NameOccurrence::AliasTarget,
        span: target_span,
        enclosing_function_span: None,
        segment_index: None,
    });

    let environment = TypeEnvironment::from_module(&module);

    assert!(
        environment
            .schema_decode_step_signature(&["Packet".to_string()], Some("spec.api"))
            .is_some()
    );
    assert!(
        environment
            .schema_decode_step_signature(&["Alias".to_string()], Some("spec.api"))
            .is_none()
    );
}

#[test]
fn public_schema_alias_with_invalid_target_leaf_has_no_navigation_identity() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  byte: UInt8\n",
            "end\n",
            "pub schema Alias = Packet\n",
            "schema Frame\n",
            "  format binary\n",
            "  nested: Alias\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let target_span = module.aliases[0].target_spans[0].clone();
    module.invalid_names.push(InvalidName {
        name: "Packet".to_string(),
        class: NameClass::Function,
        occurrence: NameOccurrence::AliasTarget,
        span: target_span,
        enclosing_function_span: None,
        segment_index: None,
    });

    assert!(resolved_schema_aliases(&module).is_empty());
    assert!(resolved_schema_composition_references(&module).is_empty());
}

#[test]
fn public_alias_names_share_member_namespaces() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn parse = parse\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub type Document = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function alias name `parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type alias name `Document`"
    }));
}

#[test]
fn public_schema_alias_names_share_schema_namespace() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "pub schema Packet = Packet\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate schema alias name `Packet`"
    }));
}
