use super::super::*;

pub(super) fn module_with_values(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Int = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

pub(super) fn module_with_literal_values_and_imports(
    value_count: usize,
    import_count: usize,
) -> SurfaceModule {
    let mut merged = empty_surface_module();
    for owner_index in 0..3 {
        let module = literal_value_owner_module(owner_index, value_count, import_count);
        merged.module = module.module.clone();
        merged.uses.extend(module.uses);
        merged.module_values.extend(module.module_values);
    }
    merged
}

fn literal_value_owner_module(
    owner_index: usize,
    value_count: usize,
    import_count: usize,
) -> SurfaceModule {
    let imports = (0..import_count)
        .filter(|index| index % 3 == owner_index)
        .map(|index| format!("use dependency_{index}\n"))
        .collect::<String>();
    let values = literal_values_for_owner(owner_index, value_count);
    let source = SourceFile::new(
        format!("owner_{owner_index}/shared.veln"),
        format!("{imports}{values}"),
    );
    veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&source).tree,
        format!("owner_{owner_index}/pkg"),
        literal_value_owner_module_name(owner_index).to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    )
}

fn literal_values_for_owner(owner_index: usize, value_count: usize) -> String {
    if owner_index >= 2 {
        return String::new();
    }
    (0..value_count)
        .filter(|index| index % 2 == owner_index)
        .map(|index| format!("let value_{index}: Int = {index}\n"))
        .collect()
}

fn literal_value_owner_module_name(owner_index: usize) -> &'static str {
    if owner_index < 2 {
        "shared"
    } else {
        "unrelated"
    }
}

pub(super) fn module_with_value_chain(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| {
                if index == 0 {
                    "let value_0: Int = 0\n".to_string()
                } else {
                    format!("let value_{index}: Int = value_{}\n", index - 1)
                }
            })
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_value_navigation_binding_work(
    count: usize,
) -> crate::navigation::ModuleValueNavigationWork {
    let locals = (0..count)
        .map(|index| format!("  let local_{index} = answer\n"))
        .collect::<String>();
    let source = SourceFile::new(
        "main.veln",
        format!("pub let answer: Int = 42\nfn read() -> Int\n{locals}  answer\nend\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    crate::navigation::reset_module_value_navigation_work();
    let resolved = resolved_module_value_navigation(&module);
    let work = crate::navigation::module_value_navigation_work();
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].references.len(), count + 1);
    work
}

#[test]
fn module_value_navigation_binding_membership_work_is_adjacent_linear() {
    let smaller = module_value_navigation_binding_work(64);
    let larger = module_value_navigation_binding_work(128);

    assert_eq!(smaller.binding_pushes, 64, "{smaller:#?}");
    assert_eq!(larger.binding_pushes, 128, "{larger:#?}");
    assert_eq!(smaller.binding_lookups, 65, "{smaller:#?}");
    assert_eq!(larger.binding_lookups, 129, "{larger:#?}");
    assert_eq!(smaller.binding_pops, 0, "{smaller:#?}");
    assert_eq!(larger.binding_pops, 0, "{larger:#?}");
}

#[test]
fn module_value_navigation_excludes_the_builtin_callsite_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub let answer: Int = 42\n",
            "fn located() -> Int callsite\n",
            "  callsite.start_line\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    module.module_values[0].name = Some("callsite".to_string());
    let resolved = resolved_module_value_navigation(&module);

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].identity.name, "callsite");
    assert!(resolved[0].references.is_empty());
}

#[test]
fn module_value_navigation_keeps_result_binding_out_of_the_function_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub let answer: Int = 42\n",
            "fn read() -> answer: Int\n",
            "ensure answer == 42\n",
            "  answer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let resolved = resolved_module_value_navigation(&lower_surface_ast(&parsed.tree));

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].identity.name, "answer");
    assert!(resolved[0].declaration.owner_package.is_none());
    assert_eq!(resolved[0].references.len(), 1);
    assert!(resolved[0].references[0].owner_package.is_none());
    assert_eq!(
        (
            resolved[0].references[0].span.start.line,
            resolved[0].references[0].span.start.column,
        ),
        (4, 3),
    );
}

pub(super) fn module_with_value_ring(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "ring.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Int = value_{}\n", (index + 1) % count))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

pub(super) fn module_with_imported_invalidation_chain(count: usize) -> SurfaceModule {
    let mut merged = empty_surface_module();
    for index in 0..count {
        let module_name = format!("chain_{index}");
        let text = if index == 0 {
            "pub let value_0: Missing = 0\n".to_string()
        } else {
            format!(
                "use chain_{}\npub let value_{index}: Int = value_{}\n",
                index - 1,
                index - 1,
            )
        };
        let source = SourceFile::new(format!("chain_{index}.veln"), text);
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module = module.module;
        merged.uses.extend(module.uses);
        merged.module_values.extend(module.module_values);
        merged.invalid_names.extend(module.invalid_names);
    }
    merged
}

pub(super) fn module_with_imported_invalidation_fanout(
    chain_count: usize,
    unrelated_value_count: usize,
) -> SurfaceModule {
    let mut merged = module_with_imported_invalidation_chain(chain_count);
    let imports = (0..chain_count)
        .map(|index| format!("use chain_{index}\n"))
        .collect::<String>();
    let dependent_values = (0..chain_count)
        .map(|index| format!("let observed_{index}: Int = chain_{index}::value_{index}\n"))
        .collect::<String>();
    let unrelated_values = (0..unrelated_value_count)
        .map(|index| format!("let unrelated_{index}: Int = {index}\n"))
        .collect::<String>();
    let source = SourceFile::new(
        "a_fanout.veln",
        format!("{imports}{dependent_values}{unrelated_values}"),
    );
    let fanout = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "a_fanout".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.uses.extend(fanout.uses);
    merged.module_values.extend(fanout.module_values);
    merged.invalid_names.extend(fanout.invalid_names);
    merged
}

pub(super) fn module_with_unresolved_value_annotations(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Missing{index} = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

pub(super) fn module_with_bare_imported_values(count: usize) -> SurfaceModule {
    let mut merged = empty_surface_module();
    for index in 0..count {
        let module_name = format!("values_{index}");
        let source = SourceFile::new(
            format!("values_{index}.veln"),
            format!("pub let value_{index}: Int = {index}\n"),
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }
    let source = SourceFile::new(
        "app.veln",
        (0..count)
            .map(|index| format!("use values_{index}\n"))
            .collect::<String>(),
    );
    let app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "app".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.uses.extend(app.uses);
    merged
}

pub(super) fn module_with_public_exports_and_importers(
    export_count: usize,
    importer_count: usize,
) -> SurfaceModule {
    let export_source = SourceFile::new(
        "values.veln",
        (0..export_count)
            .map(|index| format!("pub let value_{index}: Int = {index}\n"))
            .collect::<String>(),
    );
    let mut merged = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&export_source).tree,
        "values".to_string(),
        export_source.span(veln_source::TextRange::new(0, 0)),
    );
    for index in 0..importer_count {
        let source = SourceFile::new(format!("owner_{index}.veln"), "use values\n");
        let owner = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            format!("owner_{index}"),
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.uses.extend(owner.uses);
    }
    merged
}

pub(super) fn module_with_imported_named_annotations(count: usize) -> SurfaceModule {
    let mut merged = empty_surface_module();
    for index in 0..count {
        let module_name = format!("types_{index}");
        let source = SourceFile::new(
            format!("types_{index}.veln"),
            "pub type Item\n  Only\nend\n",
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.types.extend(module.types);
        let consumer_name = format!("app_{index}");
        let consumer_source = SourceFile::new(
            format!("app_{index}.veln"),
            format!("use types_{index}\nlet value_{index}: types_{index}::Item = 0\n"),
        );
        let consumer = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&consumer_source).tree,
            consumer_name,
            consumer_source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module = consumer.module;
        merged.uses.extend(consumer.uses);
        merged.module_values.extend(consumer.module_values);
    }
    merged
}

fn module_with_colliding_qualified_imports(count: usize) -> SurfaceModule {
    let imports = (0..count)
        .map(|index| format!("use shared from \"package_{index}\"\n"))
        .collect::<String>();
    let owner_source = SourceFile::new("app.veln", imports);
    let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "owner/pkg".to_string(),
        "app".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut merged.uses {
        use_decl.name = "shared".to_string();
    }
    for index in 0..count {
        let declaration_count = if index == 0 { 2 } else { 1 };
        for declaration_index in 0..declaration_count {
            let source = SourceFile::new(
                format!("package_{index}/shared_{declaration_index}.veln"),
                "pub let answer: Int = 1\n",
            );
            let mut module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parse(&source).tree,
                format!("package_{index}"),
                "shared".to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            for value in &mut module.module_values {
                value.module_name = Some("shared".to_string());
            }
            merged.module_values.extend(module.module_values);
        }
    }
    merged
}

pub(super) fn colliding_qualified_import_work(
    count: usize,
    lookup_count: usize,
) -> crate::types::QualifiedModuleValueImportWork {
    let module = module_with_colliding_qualified_imports(count);
    crate::types::reset_qualified_module_value_import_work();
    let environment = crate::types::TypeEnvironment::from_module(&module);
    for _ in 0..lookup_count {
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "answer".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Ambiguous
        ));
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "missing".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Ambiguous
        ));
    }
    crate::types::take_qualified_module_value_import_work()
}

#[test]
fn ambiguous_exact_module_value_import_is_not_selected_by_member_presence() {
    for imports in [
        "use shared from \"one/pkg\"\nuse shared from \"two/pkg\"\n",
        "use shared from \"two/pkg\"\nuse shared from \"one/pkg\"\n",
    ] {
        let owner_source = SourceFile::new("app.veln", imports);
        let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
            &parse(&owner_source).tree,
            "app/pkg".to_string(),
            "app".to_string(),
            owner_source.span(veln_source::TextRange::new(0, 0)),
        );
        for use_decl in &mut merged.uses {
            use_decl.name = "shared".to_string();
        }
        for (package, declaration) in [
            ("one/pkg", "pub let answer: Int = 1\n"),
            ("two/pkg", "pub let other: Int = 2\n"),
        ] {
            let source = SourceFile::new(format!("{package}/shared.veln"), declaration);
            let module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parse(&source).tree,
                package.to_string(),
                "shared".to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            merged.module_values.extend(module.module_values);
        }

        let environment = crate::types::TypeEnvironment::from_module(&merged);
        for member in ["answer", "other", "missing"] {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["shared".to_string(), member.to_string()],
                    Some("app/pkg::app"),
                    Some("app/pkg"),
                ),
                crate::types::ModuleValueLookup::Ambiguous
            ));
        }
    }
}

#[test]
fn ambiguous_qualified_module_value_reports_each_import_target() {
    let owner_source = SourceFile::new(
        "app.veln",
        concat!(
            "use shared from \"one/pkg\"\n",
            "use shared from \"two/pkg\"\n",
            "fn read() -> Int\n",
            "  shared::answer\n",
            "end\n",
        ),
    );
    let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "app/pkg".to_string(),
        "app".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut merged.uses {
        use_decl.name = "shared".to_string();
    }
    for package in ["one/pkg", "two/pkg"] {
        let source = SourceFile::new(
            format!("{package}/shared.veln"),
            "pub let answer: Int = 1\n",
        );
        let module = veln_ast::lower_surface_ast_with_package_module_identity(
            &parse(&source).tree,
            package.to_string(),
            "shared".to_string(),
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }

    let diagnostics = analyze_surface_module(&merged);
    let ambiguity = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.ambiguous")
        .unwrap_or_else(|| panic!("missing ambiguity diagnostic: {diagnostics:#?}"));
    let candidates = ambiguity
        .related
        .iter()
        .map(|related| {
            let entries = related.as_object().expect("related object");
            let field = |name: &str| {
                entries
                    .iter()
                    .find(|(field, _)| field == name)
                    .and_then(|(_, value)| value.as_text())
                    .expect("related field")
            };
            (field("candidate_kind"), field("package"))
        })
        .collect::<Vec<_>>();

    assert_eq!(
        candidates,
        vec![("import_target", "one/pkg"), ("import_target", "two/pkg")]
    );
}
