use super::*;

#[test]
fn quarantined_imports_do_not_create_qualified_module_value_ambiguity() {
    fn assert_case(
        first_import: &str,
        second_import: &str,
        qualified_value: &str,
        reverse_sources: bool,
    ) {
        let values = (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        );
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let answer: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let answer: Int = 2\n"),
        );
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!(
                    "{first_import}\n{second_import}\nfn read() -> Int\n  {qualified_value}\nend\n"
                ),
            ),
        );
        let sources = if reverse_sources {
            vec![app, beta, alpha, values]
        } else {
            vec![values, alpha, beta, app]
        };
        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            1,
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "{diagnostics:#?}"
        );
    }

    for reverse_sources in [false, true] {
        assert_case(
            "use values",
            "use values as config",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use values as config",
            "use values",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use alpha as shared",
            "use beta as shared",
            "shared::answer",
            reverse_sources,
        );
        assert_case(
            "use beta as shared",
            "use alpha as shared",
            "shared::answer",
            reverse_sources,
        );
    }
}

fn analyze_quarantined_owner_module_value_fallback(
    reverse_sources: bool,
    reverse_imports: bool,
) -> Vec<(String, veln_diagnostics::DiagnosticText)> {
    let imports = if reverse_imports {
        concat!(
            "use values as config from \"dep/pkg\"\n",
            "use values from \"dep/pkg\"\n",
        )
    } else {
        concat!(
            "use values from \"dep/pkg\"\n",
            "use values as config from \"dep/pkg\"\n",
        )
    };
    let owner_source = SourceFile::new(
        "values.veln",
        format!("{imports}let answer: Int = 1\nfn read() -> Int\n  values::answer\nend\n"),
    );
    let owner = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "owner/pkg".to_string(),
        "values".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    let dependency_source = SourceFile::new(
        "vendor/dep/values.veln",
        "pub let answer: String = \"dep\"\n",
    );
    let dependency = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&dependency_source).tree,
        "dep/pkg".to_string(),
        "values".to_string(),
        dependency_source.span(veln_source::TextRange::new(0, 0)),
    );
    let (mut merged, other) = if reverse_sources {
        (dependency, owner)
    } else {
        (owner, dependency)
    };
    merged.uses.extend(other.uses);
    merged.module_values.extend(other.module_values);
    merged.functions.extend(other.functions);

    analyze_surface_module(&merged)
        .into_iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.duplicate" | "name.unresolved" | "name.ambiguous" | "type.mismatch"
            )
        })
        .map(|diagnostic| (diagnostic.id, diagnostic.message))
        .collect::<Vec<_>>()
}

#[test]
fn quarantined_exact_import_blocks_same_owner_module_value_fallback() {
    let expected = analyze_quarantined_owner_module_value_fallback(false, false);
    assert_eq!(
        expected
            .iter()
            .filter(|(id, _)| id == "name.duplicate")
            .count(),
        1,
        "{expected:#?}"
    );
    assert_eq!(
        expected
            .iter()
            .filter(|(id, message)| id == "name.unresolved" && message.contains("values::answer"))
            .count(),
        1,
        "{expected:#?}"
    );
    assert!(
        expected
            .iter()
            .all(|(id, _)| !matches!(id.as_str(), "name.ambiguous" | "type.mismatch")),
        "{expected:#?}"
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(true, false),
        expected
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(false, true),
        expected
    );
    assert_eq!(
        analyze_quarantined_owner_module_value_fallback(true, true),
        expected
    );
}

#[test]
fn reusable_standard_base_preserves_bare_prelude_module_values() {
    let standard_source = SourceFile::new("prelude.veln", "pub let answer: Int = 42\n");
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        veln_stdlib::PACKAGE_NAME.to_string(),
        "prelude".to_string(),
        standard_source.span(veln_source::TextRange::new(0, 0)),
    );
    let base = crate::types::TypeEnvironment::from_module(&standard);

    let app_source = SourceFile::new("app.veln", "use prelude from \"std\"\n");
    let mut app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&app_source).tree,
        "app".to_string(),
        app_source.span(veln_source::TextRange::new(0, 0)),
    );
    app.uses[0].name = "std::prelude".to_string();
    let environment = crate::types::TypeEnvironment::from_module_with_base_for_test(&app, &base);

    assert!(matches!(
        environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Available(_)
    ));
}

#[test]
fn imported_module_value_ambiguity_includes_function_data_collisions() {
    for function in [
        "pub fn shared() -> Int\n  2\nend\n",
        concat!(
            "pub fn original() -> Int\n",
            "  2\n",
            "end\n",
            "pub fn shared = original\n",
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
            ),
            ("beta", SourceFile::new("beta.veln", function)),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    concat!(
                        "use alpha\n",
                        "use beta\n",
                        "fn read() -> Int\n",
                        "  shared\n",
                        "end\n",
                    ),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.ambiguous"),
            "{diagnostics:#?}"
        );
    }
}

pub(super) fn module_with_bare_module_value_ambiguity(
    import_count: usize,
    read_count: usize,
) -> SurfaceModule {
    let mut merged = empty_surface_module();
    for index in 0..import_count {
        let module_name = format!("values_{index}");
        let source = SourceFile::new(format!("values_{index}.veln"), "pub let shared: Int = 1\n");
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }
    let imports = (0..import_count)
        .map(|index| format!("use values_{index}\n"))
        .collect::<String>();
    let reads = (0..read_count)
        .map(|index| format!("fn read_{index}() -> Int\n  shared\nend\n"))
        .collect::<String>();
    let source = SourceFile::new("app.veln", format!("{imports}{reads}"));
    let app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "app".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.module = app.module;
    merged.uses.extend(app.uses);
    merged.functions.extend(app.functions);
    merged
}

#[test]
fn bare_module_value_ambiguity_provenance_work_is_linear_and_shared_across_reads() {
    fn measure(
        import_count: usize,
        read_count: usize,
    ) -> crate::types::ModuleValueAmbiguityProvenanceWork {
        let module = module_with_bare_module_value_ambiguity(import_count, read_count);
        crate::types::reset_module_value_ambiguity_provenance_work();
        let diagnostics = analyze_surface_module(&module);
        let ambiguities = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .collect::<Vec<_>>();
        assert_eq!(ambiguities.len(), read_count, "{diagnostics:#?}");
        assert!(
            ambiguities
                .iter()
                .all(|diagnostic| diagnostic.related.len() == import_count),
            "{diagnostics:#?}"
        );
        crate::types::take_module_value_ambiguity_provenance_work()
    }

    let smaller = measure(64, 1);
    let larger = measure(128, 1);
    let repeated = measure(128, 4);

    assert!(smaller.index_construction_visits >= 64, "{smaller:#?}");
    assert_eq!(
        larger.index_construction_visits,
        smaller.index_construction_visits * 2
    );
    assert_eq!(smaller.candidate_provenance_visits, 64, "{smaller:#?}");
    assert_eq!(larger.candidate_provenance_visits, 128, "{larger:#?}");
    assert_eq!(
        repeated.index_construction_visits, larger.index_construction_visits,
        "index construction must not repeat for each ambiguous read"
    );
    assert_eq!(repeated.candidate_provenance_visits, 128 * 4);
}

#[test]
fn same_owner_function_shadows_bare_imported_module_value_in_value_and_call_positions() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "fn answer() -> Int\n",
                    "  7\n",
                    "end\n",
                    "fn callback() -> fn() -> Int\n",
                    "  answer\n",
                    "end\n",
                    "fn call() -> Int\n",
                    "  answer()\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "name.ambiguous" | "name.unresolved" | "type.not_callable" | "type.mismatch"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn imported_module_value_call_target_does_not_fall_through_to_imported_function() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "functions",
            SourceFile::new("functions.veln", "pub fn answer() -> Int\n  7\nend\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                "use values\nuse functions\nfn call() -> Int\n  values::answer()\nend\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.not_callable")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.ambiguous"),
        "{diagnostics:#?}"
    );
}
