mod navigation_handler_references_tests {
    use super::*;

    fn shared_sources() -> Vec<SourceFile> {
        vec![
            source(
                "declaration.veln",
                "mod shared\n\nhandler run(value: Int) for Work\n  go() => 1\nend\n",
            ),
            source(
                "uses.veln",
                concat!(
                    "mod shared\n\n",
                    "fn first() -> Int\n",
                    "  handle 1 with run((1 + 2))\n",
                    "end\n\n",
                    "fn second() -> Int\n",
                    "  handle 2 with run(2)\n",
                    "end\n",
                ),
            ),
        ]
    }

    #[test]
    fn workspace_handler_references_share_one_identity_across_sources_and_selections() {
        let expected = [("uses.veln", 4, 17), ("uses.veln", 8, 17)];
        for (path, line, column) in [
            ("declaration.veln", 3, 9),
            ("uses.veln", 4, 17),
            ("uses.veln", 8, 17),
        ] {
            let result = query(shared_sources(), path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Handler);
            assert!(result.reference_eligible);
            assert_location(&result.definition, "declaration.veln", 3, 9);
            assert_eq!(locations(&result.references), expected, "{path}:{line}:{column}");
            assert!(result.references.iter().all(|span| {
                span.start.line == span.end.line && span.end.column - span.start.column == 3
            }));
        }
    }

    #[test]
    fn workspace_handler_references_exclude_other_modules_and_symbol_classes() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "handler run() for Work\n",
                    "  go() => 1\n",
                    "end\n\n",
                    "fn use() -> Int\n",
                    "  handle 1 with run()\n",
                    "end\n\n",
                    "fn run() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "fn collisions(run: Int) -> String\n",
                    "  # run\n",
                    "  \"run\"\n",
                    "  let record = {run: run}\n",
                    "  record.run\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "mod other\n\n",
                    "handler run() for Work\n",
                    "  go() => 2\n",
                    "end\n\n",
                    "fn use() -> Int\n",
                    "  handle 2 with run()\n",
                    "end\n",
                ),
            ),
            source(
                "collisions.veln",
                concat!(
                    "mod main\n\n",
                    "effect run\n",
                    "  run() -> Int\n",
                    "end\n\n",
                    "type run\n",
                    "  run\n",
                    "end\n\n",
                    "handler wrapper(run: fn(Int) -> Int) for Work\n",
                    "  go(run: Int) => run\n",
                    "  keep(value: Int) => run(value)\n",
                    "end\n",
                ),
            ),
        ];
        let result = query(sources.clone(), "main.veln", 1, 9).unwrap();
        assert_eq!(locations(&result.references), [("main.veln", 6, 17)]);
        let other = query(sources.clone(), "other.veln", 8, 17).unwrap();
        assert_location(&other.definition, "other.veln", 3, 9);
        assert_eq!(locations(&other.references), [("other.veln", 8, 17)]);
        assert_eq!(
            query(sources.clone(), "main.veln", 9, 4)
                .unwrap()
                .selected_symbol
                .kind,
            SymbolKind::Function
        );
        for (line, column) in [(14, 5), (15, 4), (16, 17), (17, 10)] {
            let selected = query(sources.clone(), "main.veln", line, column);
            assert!(selected.is_none_or(|value| value.selected_symbol.kind != SymbolKind::Handler));
        }
        for (line, column, kind) in [
            (3, 8, SymbolKind::Effect),
            (4, 3, SymbolKind::EffectOperation),
            (7, 6, SymbolKind::Type),
            (8, 3, SymbolKind::Constructor),
        ] {
            assert_eq!(
                query(sources.clone(), "collisions.veln", line, column)
                    .unwrap()
                    .selected_symbol
                    .kind,
                kind
            );
        }
        for (line, column) in [(12, 6), (12, 19)] {
            assert_eq!(
                query(sources.clone(), "collisions.veln", line, column)
                    .unwrap()
                    .selected_symbol
                    .kind,
                SymbolKind::HandlerOperationClauseParameter,
            );
        }
        for (line, column) in [(11, 17), (13, 23)] {
            assert_eq!(
                query(sources.clone(), "collisions.veln", line, column)
                    .unwrap()
                    .selected_symbol
                    .kind,
                SymbolKind::HandlerContextParameter,
            );
        }
    }

    #[test]
    fn workspace_handler_references_reject_duplicate_and_recovered_declarations() {
        assert_duplicate_handler_declarations_are_rejected();
        assert_recovered_handler_declarations_are_rejected();
    }

    include!("navigation_handler_declaration_rejection_helpers.rs");

    #[test]
    fn workspace_handler_references_reject_declaration_with_non_handler_diagnostic_context() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "handler run() for Work\n  go() => 0x1_0\nend\n\n",
                "handler keep() for Work\n  go() => 2\nend\n\n",
                "fn use() -> Int\n  handle 1 with run()\nend\n\n",
                "fn valid() -> Int\n  handle 2 with keep()\nend\n",
            ),
        )];
        let parsed = veln_syntax::parse(&sources[0]);
        assert!(parsed.diagnostics.iter().any(|diagnostic| {
            diagnostic.parser_context == "integer_literal"
                && diagnostic.id == "parse.integer_literal"
        }));

        let recovered = query(sources.clone(), "main.veln", 1, 9).unwrap();
        assert!(!recovered.reference_eligible);
        assert!(recovered.references.is_empty());
        assert!(query(sources.clone(), "main.veln", 10, 17).is_none());
        let valid = query(sources, "main.veln", 14, 17).unwrap();
        assert_eq!(locations(&valid.references), [("main.veln", 14, 17)]);
    }

    #[test]
    fn workspace_handler_references_reject_qualified_unresolved_incomplete_and_recovered_paths() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "handler run() for Work\n  go() => 1\nend\n\n",
                "fn valid() -> Int\n  handle 1 with run()\nend\n\n",
                "fn qualified() -> Int\n  handle 1 with other::run()\nend\n\n",
                "fn unresolved() -> Int\n  handle 1 with missing()\nend\n\n",
                "fn incomplete() -> Int\n  handle 1 with run(\nend\n\n",
                "fn recovered() -> Int\n  handle 1 with run(1 2)\nend\n",
            ),
        )];
        let declaration = query(sources.clone(), "main.veln", 1, 9).unwrap();
        assert_eq!(locations(&declaration.references), [("main.veln", 6, 17)]);
        for (line, column) in [(10, 24), (14, 17), (18, 17), (22, 17)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn workspace_handler_references_share_imported_full_and_leaf_alias_paths() {
        let sources = vec![
            source(
                "handler.veln",
                concat!(
                    "mod library::fx\n\n",
                    "pub handler run() for Work\n  go() => 1\nend\n\n",
                    "fn local() -> Int\n  handle 0 with run()\nend\n",
                ),
            ),
            source(
                "consumer.veln",
                concat!(
                    "mod consumer\n\n",
                    "use library::fx\n\n",
                    "fn full() -> Int\n  handle 1 with library::fx::run()\nend\n\n",
                    "fn alias() -> Int\n  handle 2 with fx::run()\nend\n",
                ),
            ),
        ];

        let expected = [
            ("consumer.veln", 6, 30),
            ("consumer.veln", 10, 21),
            ("handler.veln", 8, 17),
        ];
        for (path, line, column) in [
            ("handler.veln", 3, 13),
            ("handler.veln", 8, 17),
            ("consumer.veln", 6, 30),
            ("consumer.veln", 10, 21),
        ] {
            let result = query(sources.clone(), path, line, column).unwrap();
            assert_location(&result.definition, "handler.veln", 3, 13);
            assert_eq!(locations(&result.references), expected, "{path}:{line}:{column}");
            assert!(result.references.iter().all(|span| {
                span.start.line == span.end.line && span.end.column - span.start.column == 3
            }));
        }
    }

    #[test]
    fn workspace_handler_import_resolution_does_not_invent_leaf_for_explicit_alias() {
        let sources = vec![
            source(
                "handler.veln",
                concat!(
                    "mod library::fx\n\n",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
            ),
            source(
                "consumer.veln",
                concat!(
                    "mod consumer\n\n",
                    "use library::fx as other\n\n",
                    "fn full() -> Int\n  handle 1 with library::fx::run()\nend\n\n",
                    "fn nonexistent_leaf() -> Int\n  handle 2 with fx::run()\nend\n",
                ),
            ),
        ];
        let snapshot = EffectiveProjectSnapshot::new(sources);

        let full = query_snapshot(&snapshot, "consumer.veln", 6, 30).unwrap();
        assert_location(&full.definition, "handler.veln", 3, 13);
        assert_eq!(locations(&full.references), [("consumer.veln", 6, 30)]);
        assert_eq!(
            definition_at(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("consumer.veln"),
                    line: 10,
                    column: 21,
                },
            ),
            None,
        );
        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("consumer.veln"),
                    line: 10,
                    column: 21,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn workspace_handler_import_resolution_obeys_import_identity() {
        let precedence = vec![
            source(
                "fx.veln",
                "pub handler run() for Work\n  go() => 1\nend\n",
            ),
            source(
                "library/fx.veln",
                "pub handler run() for Work\n  go() => 2\nend\n",
            ),
            source(
                "consumer.veln",
                concat!(
                    "use library::fx\n",
                    "use fx\n\n",
                    "fn exact() -> Int\n  handle 1 with fx::run()\nend\n\n",
                    "fn full() -> Int\n  handle 2 with library::fx::run()\nend\n",
                ),
            ),
        ];
        let exact = query(precedence.clone(), "consumer.veln", 5, 21).unwrap();
        assert_location(&exact.definition, "fx.veln", 1, 13);
        let full = query(precedence, "consumer.veln", 9, 30).unwrap();
        assert_location(&full.definition, "library/fx.veln", 1, 13);

        for (label, imports) in [
            (
                "ambiguous implicit leaf",
                "use first::fx\nuse second::fx\n",
            ),
            (
                "duplicate import",
                "use first::fx\nuse first::fx\n",
            ),
            ("recovered import", "use first::fx unexpected\n"),
        ] {
            let sources = vec![
                source(
                    "first/fx.veln",
                    "pub handler run() for Work\n  go() => 1\nend\n",
                ),
                source(
                    "second/fx.veln",
                    "pub handler run() for Work\n  go() => 2\nend\n",
                ),
                source(
                    "stable.veln",
                    "pub handler keep() for Work\n  go() => 3\nend\n",
                ),
                source(
                    "consumer.veln",
                    &format!(
                        "use stable\n{imports}\nfn invalid() -> Int\n  handle 1 with fx::run()\nend\n\nfn valid() -> Int\n  handle 2 with stable::keep()\nend\n"
                    ),
                ),
            ];
            let import_lines = imports.lines().count();
            assert!(
                query(sources.clone(), "consumer.veln", import_lines + 4, 21).is_none(),
                "{label}"
            );
            let valid = query(sources, "consumer.veln", import_lines + 8, 25).unwrap();
            assert_location(&valid.definition, "stable.veln", 1, 13);
        }
    }

    #[test]
    fn workspace_handler_import_resolution_rejects_ineligible_paths_without_contamination() {
        let sources = vec![
            source(
                "target.veln",
                concat!(
                    "mod exact::fx\n\n",
                    "pub handler run() for Work\n  go() => 1\nend\n\n",
                    "handler hidden() for Work\n  go() => 2\nend\n\n",
                    "pub fn work() -> Int\n  4\nend\n",
                ),
            ),
            source(
                "collision.veln",
                "mod other::fx\n\npub handler run() for Work\n  go() => 3\nend\n",
            ),
            source(
                "consumer.veln",
                concat!(
                    "mod consumer\n\n",
                    "use exact::fx\n",
                    "use other::fx\n\n",
                    "fn valid_exact() -> Int\n  handle 1 with exact::fx::run()\nend\n\n",
                    "fn ambiguous() -> Int\n  handle 2 with fx::run()\nend\n\n",
                    "fn private() -> Int\n  handle 3 with exact::fx::hidden()\nend\n\n",
                    "fn unresolved() -> Int\n  handle 4 with missing::run()\nend\n\n",
                    "fn wrong_kind() -> Int\n  handle 5 with exact::fx::work()\nend\n",
                ),
            ),
        ];

        let exact = query(sources.clone(), "consumer.veln", 7, 28).unwrap();
        assert_location(&exact.definition, "target.veln", 3, 13);
        assert_eq!(locations(&exact.references), [("consumer.veln", 7, 28)]);
        for (line, column) in [(10, 21), (13, 28), (16, 26)] {
            assert!(query(sources.clone(), "consumer.veln", line, column).is_none());
        }
        let snapshot = EffectiveProjectSnapshot::new(sources.clone());
        let wrong_kind_target = query_snapshot(&snapshot, "target.veln", 11, 8).unwrap();
        assert_eq!(wrong_kind_target.selected_symbol.kind, SymbolKind::Function);
        let wrong_kind_position = || SourcePosition {
            source: SourcePath::new("consumer.veln"),
            line: 19,
            column: 28,
        };
        assert_eq!(definition_at(&snapshot, wrong_kind_position()), None);
        assert!(
            navigate(&snapshot, wrong_kind_position())
                .is_none_or(|result| result.references.is_empty())
        );

        let retained = query_snapshot(&snapshot, "consumer.veln", 7, 28).unwrap();
        assert_location(&retained.definition, "target.veln", 3, 13);
        assert_eq!(locations(&retained.references), [("consumer.veln", 7, 28)]);
    }

    #[test]
    fn workspace_handler_references_reject_ambiguous_paths_without_losing_valid_selection() {
        let sources = vec![
            source(
                "first.veln",
                "mod shared\n\nhandler run() for Work\n  go() => 1\nend\n",
            ),
            source(
                "second.veln",
                concat!(
                    "mod shared\n\n",
                    "handler run() for Work\n  go() => 2\nend\n\n",
                    "handler stable() for Work\n  go() => 3\nend\n\n",
                    "fn ambiguous() -> Int\n  handle 1 with run()\nend\n\n",
                    "fn valid() -> Int\n  handle 2 with stable()\nend\n",
                ),
            ),
        ];

        assert!(query(sources.clone(), "second.veln", 12, 17).is_none());
        let valid = query(sources, "second.veln", 16, 17).unwrap();
        assert_eq!(locations(&valid.references), [("second.veln", 16, 17)]);
    }

    #[test]
    fn direct_dependency_handler_references_keep_package_and_workspace_identities_separate() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "handler run() for Work\n  go() => 1\nend\n\n",
                    "fn local() -> Int\n  handle 1 with run()\nend\n\n",
                    "fn imported() -> Int\n  handle 2 with dep::run()\nend\n\n",
                    "fn imported_again() -> Int\n  handle 3 with dep::run()\nend\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[(
                    "dep.veln",
                    "pub handler run() for Work\n  go() => 2\nend\n",
                )],
                ["dep.veln"],
            )],
        );
        let local = query_snapshot(&snapshot, "main.veln", 8, 17).unwrap();
        assert_eq!(locations(&local.references), [("main.veln", 8, 17)]);
        let imported = query_snapshot(&snapshot, "main.veln", 12, 22).unwrap();
        assert_eq!(imported.selected_symbol.kind, SymbolKind::Handler);
        assert_eq!(imported.definition.span.file.as_str(), "dep.veln");
        assert_eq!(
            imported.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert!(matches!(
            imported.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(
            locations(&imported.references),
            [("main.veln", 12, 22), ("main.veln", 16, 22)]
        );
        let second = query_snapshot(&snapshot, "main.veln", 16, 22).unwrap();
        assert_eq!(second.definition, imported.definition);
        assert_eq!(second.references, imported.references);
    }

    #[test]
    fn direct_dependency_handlers_keep_exact_owners_across_captured_revisions() {
        let original = "pub handler run() for Work\n  go() => 1\nend\n";
        let changed = "\npub handler run() for Work\n  go() => 2\nend\n";
        let first = dependency_snapshot("first/pkg", &[("shared.veln", original)], ["shared.veln"]);
        let second =
            dependency_snapshot("second/pkg", &[("shared.veln", original)], ["shared.veln"]);
        let updated =
            dependency_snapshot("first/pkg", &[("shared.veln", changed)], ["shared.veln"]);
        let first_uri = first.virtual_sources().next().unwrap().0.to_owned();
        let second_uri = second.virtual_sources().next().unwrap().0.to_owned();
        let updated_uri = updated.virtual_sources().next().unwrap().0.to_owned();
        assert_ne!(first_uri, second_uri);
        assert_ne!(first_uri, updated_uri);

        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "use shared as first from \"first/pkg\"\n",
                    "use shared as second from \"second/pkg\"\n\n",
                    "fn left() -> Int\n  handle 1 with first::run()\nend\n\n",
                    "fn right() -> Int\n  handle 2 with second::run()\nend\n\n",
                    "fn again() -> Int\n  handle 3 with first::run()\nend\n",
                ),
            ),
            source(
                "shared.veln",
                "handler run() for Work\n  go() => 3\nend\n\nfn local() -> Int\n  handle 0 with run()\nend\n",
            ),
        ];
        let accepted = EffectiveProjectSnapshot::with_direct_dependencies(
            sources.clone(),
            vec![first, second.clone()],
        );
        let before = query_snapshot(&accepted, "main.veln", 5, 24).unwrap();
        let other = query_snapshot(&accepted, "main.veln", 9, 25).unwrap();
        let workspace = query_snapshot(&accepted, "shared.veln", 6, 17).unwrap();
        assert_eq!(
            before.definition.source,
            NavigationSource::Package {
                uri: first_uri.clone()
            }
        );
        assert_eq!(
            other.definition.source,
            NavigationSource::Package {
                uri: second_uri.clone()
            }
        );
        assert_location(&workspace.definition, "shared.veln", 1, 9);
        assert_eq!(
            locations(&before.references),
            [("main.veln", 5, 24), ("main.veln", 13, 24)]
        );
        assert_eq!(locations(&other.references), [("main.veln", 9, 25)]);
        assert_eq!(locations(&workspace.references), [("shared.veln", 6, 17)]);

        let refreshed =
            EffectiveProjectSnapshot::with_direct_dependencies(sources, vec![updated, second]);
        let after = query_snapshot(&refreshed, "main.veln", 13, 24).unwrap();
        assert_eq!(
            after.definition.source,
            NavigationSource::Package {
                uri: updated_uri.clone()
            }
        );
        assert_eq!(before.definition.span.file.as_str(), "shared.veln");
        assert_eq!(after.definition.span.file, before.definition.span.file);
        for (definition, line) in [(&before.definition, 1), (&after.definition, 2)] {
            assert_eq!(definition.span.start.line, line);
            assert_eq!(definition.span.start.column, 13);
            assert_eq!(definition.span.end.line, line);
            assert_eq!(definition.span.end.column, 16);
        }
        assert_eq!(after.references, before.references);
        assert_eq!(query_snapshot(&refreshed, "main.veln", 9, 25), Some(other));
        assert_eq!(
            query_snapshot(&refreshed, "shared.veln", 6, 17),
            Some(workspace)
        );
        assert_eq!(query_snapshot(&accepted, "main.veln", 5, 24), Some(before));
        assert_eq!(
            accepted.resolve_virtual_source(&first_uri),
            Some(original.as_bytes())
        );
        assert_eq!(
            refreshed.resolve_virtual_source(&updated_uri),
            Some(changed.as_bytes())
        );
        assert!(accepted.resolve_virtual_source(&updated_uri).is_none());
        assert!(refreshed.resolve_virtual_source(&first_uri).is_none());
    }

    #[test]
    fn direct_dependency_handler_references_reject_ineligible_declarations() {
        let cases = [
            (
                "private",
                "handler run() for Work\n  go() => 1\nend\n",
                vec!["dep.veln"],
                "run",
            ),
            (
                "unexported",
                "pub handler run() for Work\n  go() => 1\nend\n",
                Vec::new(),
                "run",
            ),
            (
                "invalid case",
                "pub handler Run() for Work\n  go() => 1\nend\n",
                vec!["dep.veln"],
                "Run",
            ),
            (
                "duplicate",
                concat!(
                    "pub handler run() for Work\n  go() => 1\nend\n\n",
                    "pub handler run() for Work\n  go() => 2\nend\n",
                ),
                vec!["dep.veln"],
                "run",
            ),
            (
                "duplicate with private declaration",
                concat!(
                    "pub handler run() for Work\n  go() => 1\nend\n\n",
                    "handler run() for Work\n  go() => 2\nend\n",
                ),
                vec!["dep.veln"],
                "run",
            ),
            (
                "recovered",
                "pub handler run() for Work @\n  go() => 1\nend\n",
                vec!["dep.veln"],
                "run",
            ),
        ];

        let stable = stable_package_handler();
        let baseline = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![stable_package_handler_use()],
            vec![stable.clone()],
        );
        let expected = query_snapshot(&baseline, "stable.veln", 4, 22).unwrap();
        assert_eq!(locations(&expected.references), [("stable.veln", 4, 22)]);
        for (case, declaration, exports, name) in cases {
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![stable_package_handler_use()],
                vec![
                    stable.clone(),
                    dependency_snapshot("example/dep", &[("dep.veln", declaration)], exports),
                ],
            );
            assert_rejected_dependency_handler(
                &snapshot,
                case,
                "use dep from \"example/dep\"\n",
                &format!("dep::{name}()"),
                &expected,
            );
        }
    }

    #[test]
    fn direct_dependency_handler_survives_unrelated_package_parse_error() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                "use dep from \"example/dep\"\n\nfn valid() -> Int\n  handle 1 with dep::run()\nend\n",
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[(
                    "dep.veln",
                    concat!(
                        "pub handler run() for Work\n  go() => 1\nend\n\n",
                        "fn broken() -> Int\n  @\nend\n",
                    ),
                )],
                ["dep.veln"],
            )],
        );
        let result = query_snapshot(&snapshot, "main.veln", 4, 22).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Handler);
        assert_eq!(locations(&result.references), [("main.veln", 4, 22)]);
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
    }

    #[test]
    fn direct_dependency_handler_references_reject_ineligible_imports_and_applications() {
        let other = dependency_snapshot(
            "other/pkg",
            &[(
                "other.veln",
                "pub handler run() for Work\n  go() => 2\nend\n",
            )],
            ["other.veln"],
        );
        let bridge = dependency_snapshot(
            "bridge/pkg",
            &[(
                "bridge.veln",
                concat!(
                    "use dep from \"transitive/pkg\"\n\n",
                    "pub fn nested() -> Int\n  handle 1 with dep::run()\nend\n",
                ),
            )],
            ["bridge.veln"],
        );
        let standard = standard_library_snapshot(
            &[("dep.veln", "pub handler run() for Work\n  go() => 3\nend\n")],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![stable_package_handler_use()],
            vec![stable_package_handler(), other, bridge],
        )
        .with_standard_library(standard);
        let expected = query_snapshot(&snapshot, "stable.veln", 4, 22).unwrap();
        assert_eq!(locations(&expected.references), [("stable.veln", 4, 22)]);
        let ordinary_function = query_snapshot(&snapshot, "stable.veln", 8, 8).unwrap();
        assert_eq!(ordinary_function.selected_symbol.kind, SymbolKind::Function);

        let stable_import = "use dep from \"stable/pkg\"\n";
        for (case, imports, application) in [
            ("standard library", "use dep from \"std\"\n", "dep::run()"),
            (
                "transitive owner",
                "use dep from \"transitive/pkg\"\n",
                "dep::run()",
            ),
            (
                "ambiguous alias",
                "use dep as target from \"stable/pkg\"\nuse other as target from \"other/pkg\"\n",
                "target::run()",
            ),
            (
                "duplicate import",
                "use dep from \"stable/pkg\"\nuse dep from \"stable/pkg\"\n",
                "dep::run()",
            ),
            (
                "duplicate target through distinct alias",
                "use dep as first from \"stable/pkg\"\nuse dep as second from \"stable/pkg\"\n",
                "first::run()",
            ),
            (
                "duplicate target through full path",
                "use dep as first from \"stable/pkg\"\nuse dep as second from \"stable/pkg\"\n",
                "dep::run()",
            ),
            (
                "recovered import",
                "use dep from \"stable/pkg\" unexpected\n",
                "dep::run()",
            ),
            (
                "mismatched package",
                "use dep from \"other/pkg\"\n",
                "dep::run()",
            ),
            ("unresolved qualifier", stable_import, "missing::run()"),
            ("unresolved handler", stable_import, "dep::missing()"),
            ("incomplete application", stable_import, "dep::run("),
            ("recovered application", stable_import, "dep::run(1 2)"),
            ("additional qualifier", stable_import, "dep::extra::run()"),
            ("function in handler position", stable_import, "dep::work()"),
            (
                "module value in handler position",
                stable_import,
                "dep::value()",
            ),
            ("type in handler position", stable_import, "dep::Item()"),
        ] {
            assert_rejected_dependency_handler(&snapshot, case, imports, application, &expected);
        }
    }

    fn stable_package_handler() -> DirectDependencySnapshot {
        dependency_snapshot(
            "stable/pkg",
            &[
                (
                    "dep.veln",
                    concat!(
                        "pub handler run() for Work\n  go() => 1\nend\n\n",
                        "pub fn work() -> Int\n  1\nend\n",
                        "pub let value: Int = 1\n",
                        "pub type Item\n  Token\nend\n",
                    ),
                ),
                (
                    "dep/extra.veln",
                    "pub handler run() for Work\n  go() => 4\nend\n",
                ),
            ],
            ["dep.veln", "dep/extra.veln"],
        )
    }

    fn stable_package_handler_use() -> SourceFile {
        source(
            "stable.veln",
            concat!(
                "use dep from \"stable/pkg\"\n\n",
                "fn valid() -> Int\n  handle 1 with dep::run()\nend\n\n",
                "fn call() -> Int\n  dep::work()\nend\n",
            ),
        )
    }

    fn assert_rejected_dependency_handler(
        baseline: &EffectiveProjectSnapshot,
        case: &str,
        imports: &str,
        application: &str,
        expected: &NavigationResult,
    ) {
        let prefix = "  handle 1 with ";
        let case_source = format!("{imports}\nfn invalid() -> Int\n{prefix}{application}\nend\n");
        let snapshot = baseline.with_workspace_overlays([source("case.veln", &case_source)]);
        let (qualifier, _) = application.rsplit_once("::").unwrap();
        let position = || SourcePosition {
            source: SourcePath::new("case.veln"),
            line: imports.lines().count() + 3,
            column: prefix.len() + qualifier.len() + "::".len() + 1,
        };
        assert_eq!(definition_at(&snapshot, position()), None, "{case}");
        assert!(navigate(&snapshot, position()).is_none(), "{case}");
        assert_eq!(
            query_snapshot(&snapshot, "stable.veln", 4, 22).as_ref(),
            Some(expected),
            "{case}"
        );
    }

    #[test]
    fn workspace_handler_references_keep_valid_selection_beside_unrelated_parse_errors() {
        let sources = vec![
            source(
                "declaration.veln",
                "mod shared\n\nhandler run() for Work\n  go() => 1\nend\n",
            ),
            source(
                "uses.veln",
                concat!(
                    "mod shared\n\n",
                    "fn valid() -> Int\n  handle 1 with run()\nend\n\n",
                    "fn broken() -> Int\n  @\nend\n",
                ),
            ),
        ];
        let result = query(sources.clone(), "declaration.veln", 3, 9).unwrap();
        assert_eq!(locations(&result.references), [("uses.veln", 4, 17)]);
        let occurrence = query(sources, "uses.veln", 4, 17).unwrap();
        assert_location(&occurrence.definition, "declaration.veln", 3, 9);
    }

    fn measure_handler_reference_index_work(
        count: usize,
    ) -> (usize, usize, usize, std::time::Duration) {
        let mut body = String::from(
            "handler run() for Work\n  go() => 1\nend\n\nfn consume() -> Int\n",
        );
        for index in 0..count {
            body.push_str(&format!("  let value_{index} = handle {index} with run()\n"));
        }
        body.push_str("  0\nend\n");
        crate::navigation::reset_handler_reference_index_work();
        let started = std::time::Instant::now();
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &body)]);
        let _ = snapshot.navigation_index();
        let elapsed = started.elapsed();
        let (token_visits, diagnostic_visits, overlap_queries) =
            crate::navigation::handler_reference_index_work();
        eprintln!(
            "handler reference index: occurrences={count} token_visits={token_visits} diagnostic_visits={diagnostic_visits} overlap_queries={overlap_queries} elapsed={elapsed:?}",
        );
        (token_visits, diagnostic_visits, overlap_queries, elapsed)
    }

    fn measure_nested_handler_reference_index_work(
        depth: usize,
    ) -> (usize, std::time::Duration) {
        let mut body = String::from(
            "handler run(value: Int) for Work\n  go() => 1\nend\n\nfn consume() -> Int\n  ",
        );
        for _ in 0..depth {
            body.push_str("handle 0 with run(");
        }
        body.push('0');
        for _ in 0..depth {
            body.push(')');
        }
        body.push_str("\nend\n");
        crate::navigation::reset_handler_reference_index_work();
        let started = std::time::Instant::now();
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &body)]);
        let _ = snapshot.navigation_index();
        let elapsed = started.elapsed();
        let result = query_snapshot(&snapshot, "main.veln", 6, 17).unwrap();
        assert_eq!(result.references.len(), depth);
        let (token_visits, _, _) = crate::navigation::handler_reference_index_work();
        eprintln!(
            "nested handler reference index: depth={depth} token_visits={token_visits} elapsed={elapsed:?}",
        );
        (token_visits, elapsed)
    }

    fn measure_recovered_handler_reference_index_work(count: usize) -> (usize, usize) {
        let mut body = String::from(
            "handler run() for Work\n  go() => 1\nend\n\nfn consume() -> Int\n",
        );
        for index in 0..count {
            body.push_str(&format!(
                "  let value_{index} = handle {index} with run(0x1_0)\n"
            ));
        }
        body.push_str("  0\nend\n");
        crate::navigation::reset_handler_reference_index_work();
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &body)]);
        let _ = snapshot.navigation_index();
        let (_, diagnostic_visits, overlap_queries) =
            crate::navigation::handler_reference_index_work();
        eprintln!(
            "handler recovery index: occurrences={count} diagnostic_visits={diagnostic_visits} overlap_queries={overlap_queries}",
        );
        (diagnostic_visits, overlap_queries)
    }

    #[test]
    fn workspace_handler_reference_indexing_work_grows_linearly() {
        let (small_tokens, small_diagnostics, small_queries, _) =
            measure_handler_reference_index_work(500);
        let (large_tokens, large_diagnostics, large_queries, _) =
            measure_handler_reference_index_work(1_000);
        assert!(large_tokens <= small_tokens * 2 + 16);
        assert_eq!(small_diagnostics, 0);
        assert_eq!(large_diagnostics, 0);
        assert!(small_queries <= 501, "{small_queries}");
        assert!(large_queries <= 1_001, "{large_queries}");
        assert!(large_queries <= small_queries * 2 + 1);
    }

    #[test]
    fn nested_workspace_handler_reference_indexing_work_grows_linearly() {
        let (small_nested_tokens, _) = measure_nested_handler_reference_index_work(50);
        let (large_nested_tokens, _) = measure_nested_handler_reference_index_work(100);
        assert!(large_nested_tokens <= small_nested_tokens * 2 + 16);
    }

    #[test]
    fn recovered_workspace_handler_reference_indexing_work_grows_linearly() {
        let (small_diagnostics, small_queries) =
            measure_recovered_handler_reference_index_work(250);
        let (large_diagnostics, large_queries) =
            measure_recovered_handler_reference_index_work(500);
        assert_eq!(small_diagnostics, 250);
        assert_eq!(large_diagnostics, 500);
        assert!(large_queries <= small_queries * 2 + 1);
    }

    fn measure_handler_eligibility_index_work(count: usize) -> (usize, usize) {
        let mut workspace_sources = Vec::with_capacity(count);
        let mut dependencies = Vec::with_capacity(count);
        for index in 0..count {
            let workspace_path = if index == 0 {
                "shared.veln".to_string()
            } else {
                format!("workspace_{index}.veln")
            };
            workspace_sources.push(source(
                &workspace_path,
                &format!("handler workspace_{index}() for Work\n  go() => {index}\nend\n"),
            ));
            dependencies.push(dependency_snapshot(
                &format!("example/dependency-{index}"),
                &[(
                    "shared.veln",
                    format!("pub handler package_{index}() for Work\n  go() => {index}\nend\n")
                        .as_str(),
                )],
                ["shared.veln"],
            ));
        }

        crate::navigation::reset_handler_eligibility_index_work();
        let snapshot =
            EffectiveProjectSnapshot::with_direct_dependencies(workspace_sources, dependencies);
        let _ = snapshot.navigation_index();
        let work = crate::navigation::handler_eligibility_index_work();
        eprintln!(
            "handler eligibility index: owners_per_kind={count} source_entries={} candidate_lookups={}",
            work.0, work.1,
        );
        work
    }

    #[test]
    fn handler_eligibility_indexing_uses_one_source_lookup_per_candidate() {
        let small = measure_handler_eligibility_index_work(50);
        let large = measure_handler_eligibility_index_work(100);
        assert_eq!(small, (100, 100));
        assert_eq!(large, (200, 200));
    }
}
