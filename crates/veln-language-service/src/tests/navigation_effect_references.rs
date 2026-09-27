mod navigation_effect_references_tests {
    use super::*;

    fn shared_sources() -> Vec<SourceFile> {
        vec![
            source(
                "declaration.veln",
                concat!(
                    "mod shared\n\n",
                    "effect Choose\n",
                    "  pick(value: Bool) -> Int\n",
                    "end\n\n",
                    "fn choose() -> Int effects [Choose]\n",
                    "  perform Choose::pick(true)\n",
                    "end\n",
                ),
            ),
            source(
                "uses.veln",
                concat!(
                    "mod shared\n\n",
                    "test choose_test() -> Int effects [Choose]\n",
                    "  perform Choose::pick(false)\n",
                    "end\n\n",
                    "handler choose_handler(callback: fn(Int) -> Int effects [Choose]) handles Choose effects [Choose]\n",
                    "  pick(value) => perform Choose::pick(value)\n",
                    "end\n",
                ),
            ),
        ]
    }

    #[test]
    fn workspace_effect_references_share_one_identity_across_sources_and_selection_forms() {
        let expected = [
            ("declaration.veln", 7, 29),
            ("declaration.veln", 8, 11),
            ("uses.veln", 3, 36),
            ("uses.veln", 4, 11),
            ("uses.veln", 7, 58),
            ("uses.veln", 7, 75),
            ("uses.veln", 7, 91),
            ("uses.veln", 8, 26),
        ];
        for (path, line, column) in [
            ("declaration.veln", 3, 8),
            ("declaration.veln", 7, 29),
            ("uses.veln", 7, 75),
            ("uses.veln", 8, 26),
        ] {
            let result = query(shared_sources(), path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Effect);
            assert!(result.reference_eligible);
            assert_location(&result.definition, "declaration.veln", 3, 8);
            assert_eq!(
                locations(&result.references),
                expected,
                "{path}:{line}:{column}"
            );
        }
    }

    #[test]
    fn imported_workspace_effect_references_share_public_identity_across_qualified_forms() {
        let sources = vec![
            source(
                "library/fx.veln",
                concat!(
                    "pub effect Remote\n",
                    "  run() -> Int\n",
                    "end\n\n",
                    "fn local() -> Int effects [Remote]\n",
                    "  perform Remote::run()\n",
                    "end\n",
                ),
            ),
            source(
                "consumer.veln",
                concat!(
                    "use library::fx\n",
                    "fn qualified(callback: fn() -> Int effects [fx::Remote]) -> Int effects [library::fx::Remote]\n",
                    "  perform fx::Remote::run()\n",
                    "end\n\n",
                    "handler qualified_handler() handles fx::Remote effects [library::fx::Remote]\n",
                    "  run() => perform fx::Remote::run()\n",
                    "end\n\n",
                    "test qualified_test() -> Int effects [fx::Remote]\n",
                    "  1\n",
                    "end\n",
                ),
            ),
        ];
        let expected = [
            ("consumer.veln", 2, 49),
            ("consumer.veln", 2, 87),
            ("consumer.veln", 3, 15),
            ("consumer.veln", 6, 41),
            ("consumer.veln", 6, 70),
            ("consumer.veln", 7, 24),
            ("consumer.veln", 10, 43),
            ("library/fx.veln", 5, 28),
            ("library/fx.veln", 6, 11),
        ];

        for (path, line, column) in [
            ("library/fx.veln", 1, 12),
            ("consumer.veln", 2, 49),
            ("consumer.veln", 2, 87),
            ("consumer.veln", 3, 15),
            ("consumer.veln", 6, 41),
            ("consumer.veln", 6, 70),
            ("consumer.veln", 7, 24),
            ("consumer.veln", 10, 43),
        ] {
            let result = query(sources.clone(), path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Effect);
            assert!(result.reference_eligible);
            assert_location(&result.definition, "library/fx.veln", 1, 12);
            assert_eq!(locations(&result.references), expected, "{path}:{line}:{column}");
        }

        for (line, column) in [(1, 5), (3, 11), (3, 23), (7, 20), (7, 32)] {
            assert!(query(sources.clone(), "consumer.veln", line, column).is_none());
        }
    }

    #[test]
    fn imported_workspace_effect_resolution_obeys_import_identity_and_visibility() {
        let precedence = vec![
            source("fx.veln", "pub effect Remote\n  run() -> Int\nend\n"),
            source(
                "library/fx.veln",
                "pub effect Remote\n  run() -> Int\nend\n",
            ),
            source(
                "consumer.veln",
                concat!(
                    "use library::fx\n",
                    "use fx\n\n",
                    "fn exact() -> Int effects [fx::Remote]\n",
                    "  1\n",
                    "end\n\n",
                    "fn full() -> Int effects [library::fx::Remote]\n",
                    "  1\n",
                    "end\n",
                ),
            ),
        ];
        let exact = query(precedence.clone(), "consumer.veln", 4, 32).unwrap();
        assert_location(&exact.definition, "fx.veln", 1, 12);
        let full = query(precedence, "consumer.veln", 8, 40).unwrap();
        assert_location(&full.definition, "library/fx.veln", 1, 12);

        for (label, imports) in [
            (
                "ambiguous implicit leaf",
                "use first::fx\nuse second::fx\n",
            ),
            (
                "duplicate import",
                "use first::fx\nuse first::fx\n",
            ),
            (
                "recovered import",
                "use first::fx unexpected\n",
            ),
        ] {
            let sources = vec![
                source(
                    "first/fx.veln",
                    "pub effect Remote\n  run() -> Int\nend\n",
                ),
                source(
                    "second/fx.veln",
                    "pub effect Remote\n  run() -> Int\nend\n",
                ),
                source(
                    "stable.veln",
                    "pub effect Stable\n  run() -> Int\nend\n",
                ),
                source(
                    "consumer.veln",
                    &format!(
                        "use stable\n{imports}\nfn invalid() -> Int effects [fx::Remote]\n  1\nend\n\nfn valid() -> Int effects [stable::Stable]\n  1\nend\n"
                    ),
                ),
            ];
            assert!(
                query(sources.clone(), "consumer.veln", imports.lines().count() + 3, 34)
                    .is_none(),
                "{label}"
            );
            let valid = query(
                sources,
                "consumer.veln",
                imports.lines().count() + 7,
                36,
            )
            .unwrap();
            assert_location(&valid.definition, "stable.veln", 1, 12);
        }

        let private = vec![
            source("hidden.veln", "effect Hidden\n  run() -> Int\nend\n"),
            source(
                "consumer.veln",
                "use hidden\n\nfn invalid() -> Int effects [hidden::Hidden]\n  1\nend\n",
            ),
        ];
        assert!(query(private, "consumer.veln", 3, 38).is_none());
    }

    #[test]
    fn imported_workspace_effect_references_reject_ineligible_declarations() {
        let duplicate = vec![
            source(
                "first.veln",
                "mod library::fx\n\npub effect Remote\n  run() -> Int\nend\n",
            ),
            source(
                "second.veln",
                "mod library::fx\n\npub effect Remote\n  run() -> Int\nend\n",
            ),
            source(
                "consumer.veln",
                "use library::fx\n\nfn use() -> Int effects [fx::Remote]\n  1\nend\n",
            ),
        ];
        let declaration = query(duplicate.clone(), "first.veln", 3, 12).unwrap();
        assert!(!declaration.reference_eligible);
        assert!(declaration.references.is_empty());
        assert!(query(duplicate, "consumer.veln", 3, 30).is_none());

        for (declaration, effect_name) in [
            ("pub effect Remote\nend\n", "Remote"),
            (
                "pub effect remote\n  run() -> Int\nend\n",
                "remote",
            ),
        ] {
            let sources = vec![
                source("library/fx.veln", declaration),
                source(
                    "consumer.veln",
                    &format!(
                        "use library::fx\n\nfn use() -> Int effects [fx::{effect_name}]\n  1\nend\n"
                    ),
                ),
            ];
            let selected = query(sources.clone(), "library/fx.veln", 1, 12).unwrap();
            assert!(!selected.reference_eligible);
            assert!(selected.references.is_empty());
            assert!(query(sources, "consumer.veln", 3, 30).is_none());
        }
    }

    #[test]
    fn imported_workspace_effect_resolution_rejects_unresolved_and_external_qualifiers() {
        let unresolved = vec![
            source(
                "library/fx.veln",
                "pub effect Remote\n  run() -> Int\nend\n",
            ),
            source(
                "consumer.veln",
                "fn use() -> Int effects [fx::Remote]\n  1\nend\n",
            ),
        ];
        assert!(query(unresolved, "consumer.veln", 1, 30).is_none());

        let collision = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "library/fx.veln",
                    "pub effect Remote\n  run() -> Int\nend\n",
                ),
                source(
                    "stable.veln",
                    "pub effect Stable\n  run() -> Int\nend\n",
                ),
                source(
                    "consumer.veln",
                    concat!(
                        "use library::fx\n",
                        "use fx from \"example/dep\"\n",
                        "use stable\n\n",
                        "fn collision() -> Int effects [fx::Remote]\n  1\nend\n\n",
                        "fn valid() -> Int effects [stable::Stable]\n  1\nend\n",
                    ),
                ),
            ],
            vec![dependency_snapshot(
                "example/dep",
                &[("fx.veln", "pub effect Remote\n  run() -> Int\nend\n")],
                ["fx.veln"],
            )],
        );
        assert!(query_snapshot(&collision, "consumer.veln", 5, 36).is_none());
        let valid = query_snapshot(&collision, "consumer.veln", 9, 36).unwrap();
        assert_location(&valid.definition, "stable.veln", 1, 12);

        let standard = EffectiveProjectSnapshot::new(vec![source(
            "consumer.veln",
            concat!(
                "use tasks from \"std\"\n\n",
                "effect Task\n",
                "  local() -> Int\n",
                "end\n\n",
                "fn standard() -> Int effects [tasks::Task]\n",
                "  1\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("tasks.veln", "pub effect Task\n  run() -> Int\nend\n")],
            ["tasks.veln"],
        ));
        assert!(query_snapshot(&standard, "consumer.veln", 7, 37).is_none());
    }

    #[test]
    fn imported_workspace_effect_references_obey_qualified_perform_recovery_boundaries() {
        for expression in [
            "perform fx::Remote",
            "perform fx::Remote::run",
            "perform fx::Remote::run(",
        ] {
            let sources = vec![
                source(
                    "library/fx.veln",
                    "pub effect Remote\n  run(value: Int) -> Int\nend\n",
                ),
                source(
                    "consumer.veln",
                    &format!("use library::fx\n\nfn broken() -> Int\n  {expression}\nend\n"),
                ),
            ];
            let declaration = query(sources.clone(), "library/fx.veln", 1, 12).unwrap();
            assert!(declaration.references.is_empty(), "{expression}");
            assert!(query(sources, "consumer.veln", 4, 15).is_none(), "{expression}");
        }

        let sources = vec![
            source(
                "library/fx.veln",
                "pub effect Remote\n  run(value: Int) -> Int\nend\n",
            ),
            source(
                "consumer.veln",
                concat!(
                    "use library::fx\n\n",
                    "fn unknown() -> Int\n  perform fx::Remote::missing()\nend\n\n",
                    "fn recovered_arguments() -> Int\n  perform fx::Remote::run(1 2)\nend\n\n",
                    "fn unrelated() -> Int\n  @\nend\n",
                ),
            ),
        ];
        let expected = [("consumer.veln", 4, 15), ("consumer.veln", 8, 15)];
        for (path, line, column) in [
            ("library/fx.veln", 1, 12),
            ("consumer.veln", 4, 15),
            ("consumer.veln", 8, 15),
        ] {
            let result = query(sources.clone(), path, line, column).unwrap();
            assert_eq!(locations(&result.references), expected);
        }
    }

    #[test]
    fn workspace_effect_references_exclude_other_identities_and_lexical_collisions() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "effect Choose\n",
                    "  Choose() -> Int\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "type Choose\n",
                    "  Choose\n",
                    "end\n\n",
                    "handler Choose() handles Choose\n",
                    "  Choose() => perform Choose::Choose()\n",
                    "end\n\n",
                    "fn Choose() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "fn collisions(callback: fn() -> Int effects [Choose], value: Choose) -> String effects [Choose]\n",
                    "  # Choose perform Choose::pick()\n",
                    "  \"Choose\"\n",
                    "  let record = {Choose: 1}\n",
                    "  record.Choose\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "fn other() -> Int effects [Choose]\n",
                    "  perform Choose::pick()\n",
                    "end\n",
                ),
            ),
        ];
        let result = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 10, 26),
                ("main.veln", 11, 23),
                ("main.veln", 18, 46),
                ("main.veln", 18, 89),
            ]
        );

        let type_collision = query(sources.clone(), "main.veln", 18, 62).unwrap();
        assert_eq!(type_collision.selected_symbol.kind, SymbolKind::Type);
        assert_location(&type_collision.definition, "main.veln", 6, 6);

        let handler = query(sources.clone(), "main.veln", 10, 9).unwrap();
        assert_eq!(handler.selected_symbol.kind, SymbolKind::Handler);

        assert!(query(sources.clone(), "main.veln", 21, 17).is_none());
        assert!(query(sources.clone(), "main.veln", 22, 10).is_none());

        let operation = query(sources, "main.veln", 11, 31).unwrap();
        assert_eq!(operation.selected_symbol.kind, SymbolKind::EffectOperation);
        assert!(operation.references.is_empty());
    }

    #[test]
    fn workspace_effect_references_reject_generic_and_qualified_origins() {
        let local = concat!(
            "effect E\n",
            "  run() -> Int\n",
            "end\n\n",
            "fn generic<effect E>() -> Int effects [...E]\n",
            "  1\n",
            "end\n\n",
            "fn qualified() -> Int effects [foreign::E]\n",
            "  perform foreign::E::run()\n",
            "end\n",
        );
        let qualified_workspace_sources = vec![
            source("main.veln", local),
            source(
                "foreign.veln",
                "mod foreign\n\neffect E\n  run() -> Int\nend\n",
            ),
        ];
        let local_result = query(qualified_workspace_sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(locations(&local_result.references), []);
        for (line, column) in [(5, 43), (9, 40), (10, 11), (10, 20)] {
            assert!(
                query(
                    qualified_workspace_sources.clone(),
                    "main.veln",
                    line,
                    column,
                )
                .is_none()
            );
        }
    }

    #[test]
    fn workspace_effect_references_reject_ambiguous_declarations() {
        let ambiguous = vec![
            source(
                "first.veln",
                "mod shared\n\neffect Choose\n  first() -> Int\nend\n",
            ),
            source(
                "second.veln",
                "mod shared\n\neffect Choose\n  second() -> Int\nend\n\nfn use() -> Int effects [Choose]\n  1\nend\n",
            ),
        ];
        let ambiguous_declaration = query(ambiguous.clone(), "first.veln", 3, 8).unwrap();
        assert!(ambiguous_declaration.references.is_empty());
        assert!(!ambiguous_declaration.reference_eligible);
        assert!(query(ambiguous, "second.veln", 7, 27).is_none());
    }

    #[test]
    fn workspace_effect_references_reject_recovered_syntax() {
        let recovered =
            "effect Choose\n  pick() -> Int\nend\n\nfn broken() -> Int effects [Choose\n  1\nend\n";
        assert!(query(vec![source("main.veln", recovered)], "main.veln", 5, 29).is_none());

        let recovered_declaration = query(
            vec![source("main.veln", "effect Choose\n  pick() -> Int\n")],
            "main.veln",
            1,
            8,
        )
        .unwrap();
        assert!(!recovered_declaration.reference_eligible);
        assert!(recovered_declaration.references.is_empty());
    }

    #[test]
    fn workspace_effect_references_reject_imported_effects() {
        let imported = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "effect Task\n",
                    "  local() -> Int\n",
                    "end\n\n",
                    "fn imported() -> Int effects [dep::Task]\n",
                    "  1\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[("dep.veln", "pub effect Task\n  run() -> Int\nend\n")],
                ["dep.veln"],
            )],
        );
        for column in [31, 36] {
            assert!(query_snapshot(&imported, "main.veln", 7, column).is_none());
        }
    }

    #[test]
    fn workspace_effect_references_reject_invalid_casing() {
        let invalid_casing = concat!(
            "effect choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn invalid() -> Int effects [choose]\n",
            "  1\n",
            "end\n",
        );
        let invalid_declaration = query(
            vec![source("invalid.veln", invalid_casing)],
            "invalid.veln",
            1,
            8,
        )
        .unwrap();
        assert_eq!(invalid_declaration.selected_symbol.kind, SymbolKind::Effect);
        assert!(!invalid_declaration.reference_eligible);
        assert!(invalid_declaration.references.is_empty());
        assert!(
            query(
                vec![source("invalid.veln", invalid_casing)],
                "invalid.veln",
                5,
                29,
            )
            .is_none()
        );
    }

    #[test]
    fn workspace_effect_references_keep_valid_occurrences_beside_unrelated_parse_errors() {
        let sources = vec![
            source(
                "declaration.veln",
                "mod shared\n\neffect Choose\n  pick() -> Int\nend\n",
            ),
            source(
                "uses.veln",
                concat!(
                    "mod shared\n\n",
                    "fn choose() -> Int effects [Choose]\n",
                    "  perform Choose::pick()\n",
                    "end\n\n",
                    "fn broken() -> Int\n",
                    "  @\n",
                    "end\n",
                ),
            ),
        ];

        let declaration = query(sources.clone(), "declaration.veln", 3, 8).unwrap();
        assert_eq!(
            locations(&declaration.references),
            [("uses.veln", 3, 29), ("uses.veln", 4, 11)]
        );

        let occurrence = query(sources, "uses.veln", 4, 11).unwrap();
        assert_eq!(occurrence.selected_symbol.kind, SymbolKind::Effect);
        assert_eq!(occurrence.definition.span.file.as_str(), "declaration.veln");
        assert_eq!(
            locations(&occurrence.references),
            [("uses.veln", 3, 29), ("uses.veln", 4, 11)]
        );
    }

    #[test]
    fn workspace_effect_references_exclude_balanced_shapes_from_recovered_body_lines() {
        let text = concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn valid() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  effects [Choose]\n",
            "  handles Choose\n",
            "  value perform Choose::pick()\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(
            locations(&declaration.references),
            [("main.veln", 5, 28), ("main.veln", 6, 11)]
        );
        for (line, column) in [(10, 12), (11, 11), (12, 17)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn workspace_effect_references_exclude_recovered_effect_rows_and_handler_targets() {
        for (text, line, column) in [
            (
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "fn broken() -> Int effects [Choose @]\n",
                    "  1\n",
                    "end\n",
                ),
                5,
                29,
            ),
            (
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "handler broken() handles Choose @\n",
                    "  pick() => 1\n",
                    "end\n",
                ),
                5,
                26,
            ),
        ] {
            let sources = vec![source("main.veln", text)];
            let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();

            assert!(declaration.references.is_empty());
            assert!(query(sources, "main.veln", line, column).is_none());
        }

        let text = concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "handler broken() handles Choose effects [Choose @]\n",
            "  pick() => 1\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];
        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(locations(&declaration.references), [("main.veln", 5, 26)]);
        assert!(query(sources, "main.veln", 5, 42).is_none());
    }

    #[test]
    fn workspace_effect_references_reject_recovered_declaration_closed_by_end() {
        let text = concat!(
            "effect Choose\n",
            "end\n\n",
            "fn use() -> Int effects [Choose]\n",
            "  1\n",
            "end\n",
        );
        let result = query(vec![source("main.veln", text)], "main.veln", 1, 8).unwrap();

        assert!(!result.reference_eligible);
        assert!(result.references.is_empty());
    }

    #[test]
    fn workspace_effect_references_exclude_recovered_perform_qualifiers() {
        let text = concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  perform Choose::pick(\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert!(declaration.references.is_empty());
        assert!(query(sources, "main.veln", 6, 11).is_none());
    }

    #[test]
    fn workspace_effect_references_keep_complete_qualifiers_with_recovered_arguments() {
        let text = concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  perform Choose::pick(1 2)\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(locations(&declaration.references), [("main.veln", 6, 11)]);

        let qualifier = query(sources, "main.veln", 6, 11).unwrap();
        assert_eq!(qualifier.selected_symbol.kind, SymbolKind::Effect);
        assert!(qualifier.reference_eligible);
        assert_location(&qualifier.definition, "main.veln", 1, 8);
        assert_eq!(locations(&qualifier.references), [("main.veln", 6, 11)]);
    }

    #[test]
    fn workspace_effect_references_do_not_resolve_clean_uses_to_recovered_declarations() {
        let text = concat!(
            "effect Choose\n",
            "end\n\n",
            "fn use() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n\n",
            "handler choose_handler() handles Choose\n",
            "  pick() => perform Choose::pick()\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert!(!declaration.reference_eligible);
        assert!(declaration.references.is_empty());
        for (line, column) in [(4, 25), (5, 11), (8, 33), (9, 21)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn recovered_effect_declaration_makes_a_clean_same_module_declaration_ambiguous() {
        let sources = vec![
            source(
                "clean.veln",
                "mod shared\n\neffect Choose\n  pick() -> Int\nend\n",
            ),
            source("recovered.veln", "mod shared\n\neffect Choose\nend\n"),
            source(
                "use.veln",
                concat!(
                    "mod shared\n\n",
                    "fn use() -> Int effects [Choose]\n",
                    "  perform Choose::pick()\n",
                    "end\n\n",
                    "handler use_handler() handles Choose\n",
                    "  pick() => perform Choose::pick()\n",
                    "end\n",
                ),
            ),
        ];

        let clean = query(sources.clone(), "clean.veln", 3, 8).unwrap();
        assert!(!clean.reference_eligible);
        assert!(clean.references.is_empty());
        let recovered = query(sources.clone(), "recovered.veln", 3, 8).unwrap();
        assert!(!recovered.reference_eligible);
        assert!(recovered.references.is_empty());
        for (line, column) in [(3, 25), (4, 11), (7, 30), (8, 20)] {
            assert!(query(sources.clone(), "use.veln", line, column).is_none());
        }
    }

    #[test]
    fn workspace_effect_references_exclude_unresolved_and_invalid_cased_occurrences() {
        let text = concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn boundaries() -> Int effects [Choose, Missing, choose]\n",
            "  perform Choose::pick()\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let valid = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(valid.selected_symbol.kind, SymbolKind::Effect);
        assert_eq!(
            locations(&valid.references),
            [("main.veln", 5, 33), ("main.veln", 6, 11)]
        );

        assert!(query(sources.clone(), "main.veln", 5, 41).is_none());
        assert!(query(sources, "main.veln", 5, 50).is_none());
    }

    #[test]
    fn workspace_effect_references_include_parse_clean_predicate_qualifiers() {
        let text = concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "end\n\n",
            "fn guarded(value: Int) -> Int\n",
            "  require perform Choose::pick(value) > 0\n",
            "  let constrained = _candidate satisfy candidate => perform Choose::pick(candidate) > 0\n",
            "  constrained\n",
            "end\n\n",
            "schema Packet\n",
            "  format binary\n",
            "  value: UInt8 where perform Choose::pick(value) > 0\n",
            "  validate perform Choose::pick(value) > 0\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];
        let expected = [
            ("main.veln", 6, 19),
            ("main.veln", 7, 61),
            ("main.veln", 13, 30),
            ("main.veln", 14, 20),
        ];

        for (line, column) in [(1, 8), (6, 19), (7, 61), (13, 30), (14, 20)] {
            let result = query(sources.clone(), "main.veln", line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Effect);
            assert_eq!(locations(&result.references), expected);
        }

        let operation = query(sources, "main.veln", 6, 27).unwrap();
        assert_eq!(operation.selected_symbol.kind, SymbolKind::EffectOperation);
        assert_eq!(
            locations(&operation.references),
            [
                ("main.veln", 6, 27),
                ("main.veln", 7, 69),
                ("main.veln", 13, 38),
                ("main.veln", 14, 28),
            ]
        );
    }

    #[test]
    fn workspace_effect_references_exclude_recovered_predicate_qualifiers() {
        for (predicate_source, line, column) in [
            (
                "fn guarded() -> Int\n  require perform Choose::pick(\n  1\nend\n",
                6,
                19,
            ),
            (
                "fn guarded() -> Int\n  _value satisfy candidate => perform Choose::pick(\nend\n",
                6,
                39,
            ),
            (
                "schema Packet\n  format binary\n  value: UInt8 where perform Choose::pick(\nend\n",
                7,
                30,
            ),
            (
                "schema Packet\n  format binary\n  validate perform Choose::pick(\nend\n",
                7,
                20,
            ),
        ] {
            let text = format!("effect Choose\n  pick() -> Int\nend\n\n{predicate_source}");
            let sources = vec![source("main.veln", &text)];
            let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();

            assert!(declaration.references.is_empty());
            assert!(query(sources, "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn workspace_effect_references_keep_complete_predicate_qualifiers_beside_recovery() {
        let text = concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "end\n\n",
            "fn guarded() -> Int\n",
            "  require perform Choose::pick(1 2) + perform Choose::pick(\n",
            "  1\n",
            "end\n",
        );
        let sources = vec![source("main.veln", text)];

        let declaration = query(sources.clone(), "main.veln", 1, 8).unwrap();
        assert_eq!(locations(&declaration.references), [("main.veln", 6, 19)]);
        let complete = query(sources.clone(), "main.veln", 6, 19).unwrap();
        assert_eq!(complete.selected_symbol.kind, SymbolKind::Effect);
        assert_eq!(locations(&complete.references), [("main.veln", 6, 19)]);
        assert!(query(sources, "main.veln", 6, 48).is_none());
    }

    #[test]
    fn workspace_effect_reference_collection_handles_many_declarations_and_occurrences() {
        for count in [128, 256, 512] {
            let mut declarations =
                String::from("pub effect Target\n  pick() -> Int\nend\n\n");
            let mut uses = String::from("use library::fx\n\n");
            for index in 0..count {
                declarations
                    .push_str(&format!("effect Noise{index}\n  ignore() -> Int\nend\n\n"));
                uses.push_str(&format!(
                    "fn use_{index}() -> Int effects [fx::Target]\n  {index}\nend\n\n"
                ));
            }
            crate::navigation::reset_effect_identity_index_work();
            let snapshot = EffectiveProjectSnapshot::new(vec![
                source("library/fx.veln", &declarations),
                source("uses.veln", &uses),
            ]);
            let result = query_snapshot(&snapshot, "library/fx.veln", 1, 12).unwrap();
            assert_eq!(result.references.len(), count);
            let (declaration_visits, identity_lookups) =
                crate::navigation::effect_identity_index_work();
            assert_eq!(declaration_visits, count + 1);
            assert_eq!(identity_lookups, count);
        }
    }

    #[test]
    fn workspace_effect_reference_collection_keeps_long_rows_adjacent_linear() {
        for count in [1_000, 2_000, 4_000, 8_000] {
            let mut body = String::from(
                "effect Choose\n  pick() -> Int\nend\n\nfn consume() -> Int effects [",
            );
            for index in 0..count {
                if index > 0 {
                    body.push_str(", ");
                }
                body.push_str("Choose");
            }
            body.push_str("]\n  1\nend\n");

            let input = source("main.veln", &body);
            let token_count = veln_syntax::lex(&input).tokens.len();
            let scalar_count = body.chars().count();
            crate::navigation::reset_effect_list_classification_token_visits();
            crate::navigation::reset_effect_reference_source_scalar_visits();
            let snapshot = EffectiveProjectSnapshot::new(vec![input]);
            let index_started = std::time::Instant::now();
            let _ = snapshot.navigation_index();
            let index_elapsed = index_started.elapsed();
            let started = std::time::Instant::now();
            let result = query_snapshot(&snapshot, "main.veln", 1, 8).unwrap();
            let elapsed = started.elapsed();
            let classification_visits =
                crate::navigation::effect_list_classification_token_visits();
            let source_scalar_visits = crate::navigation::effect_reference_source_scalar_visits();
            eprintln!(
                "long effect row: references={count} collected={} classification_visits={classification_visits} source_scalar_visits={source_scalar_visits} index_elapsed={index_elapsed:?} query_elapsed={elapsed:?}",
                result.references.len(),
            );
            assert_eq!(result.references.len(), count);
            assert_eq!(
                classification_visits, token_count,
                "effect-list classification must visit every token exactly once",
            );
            assert!(
                source_scalar_visits <= scalar_count,
                "effect-reference span conversion must make at most one source pass",
            );
        }
    }

    #[test]
    fn workspace_effect_reference_indexing_keeps_malformed_delimiters_linear() {
        for count in [1_000, 2_000, 4_000] {
            let mut body = String::from("effect Choose\n  pick() -> Int\nend\n");
            body.extend(std::iter::repeat_n('(', count));
            body.extend(std::iter::repeat_n('\n', count));

            let input = source("main.veln", &body);
            let token_count = veln_syntax::lex(&input).tokens.len();
            crate::navigation::reset_effect_list_classification_token_visits();
            let started = std::time::Instant::now();
            let snapshot = EffectiveProjectSnapshot::new(vec![input]);
            let _ = snapshot.navigation_index();
            let elapsed = started.elapsed();
            let token_visits = crate::navigation::effect_list_classification_token_visits();
            let frame_visits = crate::navigation::effect_list_classification_frame_visits();
            eprintln!(
                "malformed delimiters: count={count} tokens={token_count} token_visits={token_visits} frame_visits={frame_visits} elapsed={elapsed:?}",
            );
            assert_eq!(token_visits, token_count);
            assert!(
                frame_visits <= token_count,
                "effect-list classification must inspect at most one delimiter frame per token",
            );
        }
    }
}
