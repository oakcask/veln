use super::*;

#[test]
fn imported_module_value_ambiguity_is_independent_of_source_and_import_order() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
        );
        let imports = if reverse_imports {
            "use beta\nuse alpha\n"
        } else {
            "use alpha\nuse beta\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn read() -> Int\n  shared\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta, alpha, app]
        } else {
            vec![alpha, beta, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}

#[test]
fn invalid_local_module_value_shadows_an_imported_value() {
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
                    "let answer: Missing = 0\n",
                    "let selected: Int = answer\n",
                    "fn read() -> Int\n",
                    "  selected\n",
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
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.mismatch"
            || !diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.start.line == 3)
    }));
}

#[test]
fn invalid_imported_module_value_does_not_reserve_a_bare_exposure() {
    for reverse_sources in [false, true] {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
        );
        let app = (
            "app",
            SourceFile::new("app.veln", "use alpha\nfn read() -> Int\n  shared\nend\n"),
        );
        let sources = if reverse_sources {
            vec![app, alpha]
        } else {
            vec![alpha, app]
        };

        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved value `shared`"
            }),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
    }
}

#[test]
fn callable_collisions_stay_unavailable_after_imported_availability_rechecks() {
    for callable in [
        "pub fn clash() -> Int\n  1\nend\n",
        concat!(
            "pub fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn clash = original\n",
        ),
    ] {
        let module = merged_modules_with_identities(vec![
            ("b", SourceFile::new("b.veln", "pub let bad: Missing = 0\n")),
            (
                "a",
                SourceFile::new(
                    "a.veln",
                    format!(
                        "use b\n{callable}pub let clash: Int = 2\npub let recheck: Int = b::bad\n"
                    ),
                ),
            ),
            (
                "c",
                SourceFile::new(
                    "c.veln",
                    concat!(
                        "use a\n",
                        "fn bare() -> Int\n",
                        "  clash()\n",
                        "end\n",
                        "fn qualified() -> Int\n",
                        "  a::clash()\n",
                        "end\n",
                    ),
                ),
            ),
        ]);
        let mut environment = crate::types::TypeEnvironment::from_module(&module);
        let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
        let analysis = crate::analysis::check_module_values(
            &module,
            &mut environment,
            &mut variant_diagnostics,
        );

        assert!(
            analysis
                .invalid
                .contains(&(None, Some("a".to_string()), "clash".to_string())),
            "{:#?}",
            analysis.invalid
        );
        assert!(matches!(
            environment.module_value_path_in_owner(&["clash".to_string()], Some("c"), None),
            crate::types::ModuleValueLookup::Missing
        ));
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["a".to_string(), "clash".to_string()],
                Some("c"),
                None,
            ),
            crate::types::ModuleValueLookup::Missing
        ));

        let diagnostics = analyze_surface_module(&module);
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "name.ambiguous"
                    && diagnostic.id != "type.not_callable"
                    && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("clash"))
            }),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn unpublished_functions_and_aliases_do_not_collide_with_imported_values() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let functions = (
                "functions",
                SourceFile::new(
                    "functions.veln",
                    concat!(
                        "pub type State\n",
                        "  Hidden\n",
                        "end\n",
                        "pub fn shared(value: State::Hidden) -> Int\n",
                        "  1\n",
                        "end\n",
                        "pub fn aliased = shared\n",
                    ),
                ),
            );
            let values = (
                "values",
                SourceFile::new(
                    "values.veln",
                    concat!("pub let shared: Int = 2\n", "pub let aliased: Int = 3\n",),
                ),
            );
            let imports = if reverse_imports {
                "use values\nuse functions\n"
            } else {
                "use functions\nuse values\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "{imports}let selected: Int = shared\nlet selected_alias: Int = aliased\n"
                    ),
                ),
            );
            let sources = if reverse_sources {
                vec![app, values, functions]
            } else {
                vec![functions, values, app]
            };
            let module = merged_modules_with_identities(sources);
            let environment = crate::types::TypeEnvironment::from_module(&module);

            for name in ["shared", "aliased"] {
                assert!(matches!(
                    environment.module_value_path_in_owner(&[name.to_string()], Some("app"), None,),
                    crate::types::ModuleValueLookup::Available(_)
                ));
            }

            let diagnostics = analyze_surface_module(&module);
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| { diagnostic.id == "type.variant_refinement_private" }),
                "{diagnostics:#?}"
            );
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && (diagnostic.message.contains("shared")
                                || diagnostic.message.contains("aliased")))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn private_imported_module_value_does_not_collide_with_public_function() {
    for reverse_imports in [false, true] {
        let imports = if reverse_imports {
            "use values\nuse functions\n"
        } else {
            "use functions\nuse values\n"
        };
        let module = merged_modules_with_identities(vec![
            (
                "values",
                SourceFile::new("values.veln", "let shared: Int = 1\n"),
            ),
            (
                "functions",
                SourceFile::new("functions.veln", "pub fn shared() -> Int\n  2\nend\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared()\nend\n"),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    }
}

#[test]
fn valid_imported_value_survives_an_invalid_namesake_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            );
            let beta = (
                "beta",
                SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
            );
            let imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared\nend\n"),
                ),
            );
            let sources = if reverse_sources {
                vec![beta, alpha, app]
            } else {
                vec![alpha, beta, app]
            };

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message == "unresolved value `shared`")
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn module_value_initializers_observe_final_imported_availability_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            );
            let beta = (
                "beta",
                SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
            );
            let imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
            let app = (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "{imports}let selected: Int = shared\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            );
            let sources = if reverse_sources {
                vec![app, beta, alpha]
            } else {
                vec![alpha, beta, app]
            };

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("shared"))
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("selected"))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }

    for reference in ["shared", "alpha::shared"] {
        let module = merged_modules_with_identities(vec![
            (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "use alpha\nlet selected: Int = {reference}\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == format!("unresolved value `{reference}`")
            }),
            "reference={reference}: {diagnostics:#?}"
        );
    }
}

#[test]
fn delayed_bare_import_invalidation_quarantines_dependents_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let owner_imports = if reverse_imports {
                "use beta\nuse alpha\n"
            } else {
                "use alpha\nuse beta\n"
            };
            let mut sources = vec![
                (
                    "bad",
                    SourceFile::new("bad.veln", "pub let broken: Missing = 0\n"),
                ),
                (
                    "alpha",
                    SourceFile::new("alpha.veln", "use bad\npub let shared: Int = bad::broken\n"),
                ),
                (
                    "beta",
                    SourceFile::new(
                        "beta.veln",
                        "pub let shared: Int = 1\npub let shared: Int = 2\n",
                    ),
                ),
                (
                    "owner",
                    SourceFile::new(
                        "owner.veln",
                        format!(
                            "{owner_imports}pub let selected: Int = shared\npub let healthy: Int = 3\n"
                        ),
                    ),
                ),
                (
                    "consumer",
                    SourceFile::new(
                        "consumer.veln",
                        concat!(
                            "use owner\n",
                            "fn read_selected() -> Int\n",
                            "  selected\n",
                            "end\n",
                            "fn read_healthy() -> Int\n",
                            "  healthy\n",
                            "end\n",
                        ),
                    ),
                ),
            ];
            if reverse_sources {
                sources.reverse();
            }
            let module = merged_modules_with_identities(sources);
            let mut environment = crate::types::TypeEnvironment::from_module(&module);
            let mut variant_diagnostics = crate::analysis::VariantDiagnosticInterner::default();
            let analysis = crate::analysis::check_module_values(
                &module,
                &mut environment,
                &mut variant_diagnostics,
            );
            let selected = (None, Some("owner".to_string()), "selected".to_string());

            assert!(
                analysis.invalid.contains(&selected),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {:#?}",
                analysis.invalid
            );
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["selected".to_string()],
                    Some("consumer"),
                    None,
                ),
                crate::types::ModuleValueLookup::Missing
            ));
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &["healthy".to_string()],
                    Some("consumer"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));

            let diagnostics = analyze_surface_module(&module);
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved"
                        && diagnostic.message == "unresolved value `selected`"
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("healthy"))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}
