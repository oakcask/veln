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
    fn standard_library_handler_references_share_exact_and_unique_alias_identity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use observe as trace from \"std\"\n\n",
                "fn first() -> Int\n  handle 1 with observe::noop()\nend\n\n",
                "fn second() -> Int\n  handle 2 with trace::noop()\nend\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "observe.veln",
                "pub handler noop() for Observe\n  record() => 1\nend\n",
            )],
            ["observe.veln"],
        ));

        let exact = query_snapshot(&snapshot, "main.veln", 4, 26).unwrap();
        let alias = query_snapshot(&snapshot, "main.veln", 8, 24).unwrap();
        assert_eq!(
            exact.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(
            alias.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(exact.definition.span.file.as_str(), "observe.veln");
        assert_eq!(alias.definition, exact.definition);
        assert_eq!(alias.selected_symbol, exact.selected_symbol);
        assert!(matches!(
            exact.definition.source,
            NavigationSource::Package { .. }
        ));
        let expected = [("main.veln", 4, 26), ("main.veln", 8, 24)];
        assert_eq!(locations(&exact.references), expected);
        assert_eq!(locations(&alias.references), expected);
    }

    #[test]
    fn standard_library_handler_exact_module_path_wins_over_alias_collision() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use observe from \"std\"\n",
                "use telemetry as observe from \"std\"\n\n",
                "fn selected() -> Int\n  handle 1 with observe::noop()\nend\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[
                (
                    "observe.veln",
                    "pub handler noop() for Observe\n  record() => 1\nend\n",
                ),
                (
                    "telemetry.veln",
                    "pub handler noop() for Observe\n  record() => 2\nend\n",
                ),
            ],
            ["observe.veln", "telemetry.veln"],
        ));

        let selected = query_snapshot(&snapshot, "main.veln", 5, 26).unwrap();
        assert_eq!(selected.definition.span.file.as_str(), "observe.veln");
        assert_eq!(locations(&selected.references), [("main.veln", 5, 26)]);
    }

    #[test]
    fn standard_library_handler_references_preserve_declaration_eligibility_guards() {
        let cases = [
            (
                "private",
                "handler noop() for Observe\n  record() => 1\nend\n",
                vec!["observe.veln"],
            ),
            (
                "unexported",
                "pub handler noop() for Observe\n  record() => 1\nend\n",
                Vec::new(),
            ),
            (
                "invalid case",
                "pub handler Noop() for Observe\n  record() => 1\nend\n",
                vec!["observe.veln"],
            ),
            (
                "duplicate",
                concat!(
                    "pub handler noop() for Observe\n  record() => 1\nend\n\n",
                    "pub handler noop() for Observe\n  record() => 2\nend\n",
                ),
                vec!["observe.veln"],
            ),
            (
                "recovered",
                "pub handler noop() for Observe @\n  record() => 1\nend\n",
                vec!["observe.veln"],
            ),
        ];

        for (case, declaration, exports) in cases {
            let name = if case == "invalid case" { "Noop" } else { "noop" };
            let source_text = format!(
                "use observe from \"std\"\n\nfn selected() -> Int\n  handle 1 with observe::{name}()\nend\n"
            );
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)])
                .with_standard_library(standard_library_snapshot(
                    &[("observe.veln", declaration)],
                    exports,
                ));
            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 26).is_none(),
                "{case}"
            );
        }
    }

    #[test]
    fn standard_library_handler_import_guards_keep_recovery_and_ambiguity_separate() {
        let standard_library = || {
            standard_library_snapshot(
                &[(
                    "observe.veln",
                    "pub handler noop() for Observe\n  record() => 1\nend\n",
                )],
                ["observe.veln"],
            )
        };
        let ambiguous = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use observe as first from \"std\"\n",
                "use observe as second from \"std\"\n\n",
                "fn selected() -> Int\n  handle 1 with first::noop()\nend\n",
            ),
        )])
        .with_standard_library(standard_library());
        assert!(query_snapshot(&ambiguous, "main.veln", 5, 24).is_none());

        let recovered = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use observe as bad from \"std\" unexpected\n",
                "use observe as trace from \"std\"\n\n",
                "fn invalid() -> Int\n  handle 0 with bad::noop()\nend\n\n",
                "fn valid() -> Int\n  handle 1 with trace::noop()\nend\n",
            ),
        )])
        .with_standard_library(standard_library());
        assert!(query_snapshot(&recovered, "main.veln", 5, 22).is_none());
        let valid = query_snapshot(&recovered, "main.veln", 9, 24).unwrap();
        assert_eq!(locations(&valid.references), [("main.veln", 9, 24)]);
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
        let (before, other, workspace) = assert_initial_dependency_handler_owners(
            &accepted,
            &first_uri,
            &second_uri,
        );
        let refreshed =
            EffectiveProjectSnapshot::with_direct_dependencies(sources, vec![updated, second]);
        assert_refreshed_dependency_handler_owner(RefreshedHandlerOwner {
            accepted: &accepted,
            refreshed: &refreshed,
            before: &before,
            other: &other,
            workspace: &workspace,
            first_uri: &first_uri,
            updated_uri: &updated_uri,
            original,
            changed,
        });
    }

    fn assert_initial_dependency_handler_owners(
        accepted: &EffectiveProjectSnapshot,
        first_uri: &str,
        second_uri: &str,
    ) -> (NavigationResult, NavigationResult, NavigationResult) {
        let before = query_snapshot(accepted, "main.veln", 5, 24).unwrap();
        let other = query_snapshot(accepted, "main.veln", 9, 25).unwrap();
        let workspace = query_snapshot(accepted, "shared.veln", 6, 17).unwrap();
        assert_eq!(
            before.definition.source,
            NavigationSource::Package {
                uri: first_uri.to_owned()
            }
        );
        assert_eq!(
            other.definition.source,
            NavigationSource::Package {
                uri: second_uri.to_owned()
            }
        );
        assert_location(&workspace.definition, "shared.veln", 1, 9);
        assert_eq!(
            locations(&before.references),
            [("main.veln", 5, 24), ("main.veln", 13, 24)]
        );
        assert_eq!(locations(&other.references), [("main.veln", 9, 25)]);
        assert_eq!(locations(&workspace.references), [("shared.veln", 6, 17)]);
        (before, other, workspace)
    }

    struct RefreshedHandlerOwner<'a> {
        accepted: &'a EffectiveProjectSnapshot,
        refreshed: &'a EffectiveProjectSnapshot,
        before: &'a NavigationResult,
        other: &'a NavigationResult,
        workspace: &'a NavigationResult,
        first_uri: &'a str,
        updated_uri: &'a str,
        original: &'a str,
        changed: &'a str,
    }

    fn assert_refreshed_dependency_handler_owner(case: RefreshedHandlerOwner<'_>) {
        let after = query_snapshot(case.refreshed, "main.veln", 13, 24).unwrap();
        assert_eq!(
            after.definition.source,
            NavigationSource::Package {
                uri: case.updated_uri.to_owned()
            }
        );
        assert_eq!(case.before.definition.span.file.as_str(), "shared.veln");
        assert_eq!(after.definition.span.file, case.before.definition.span.file);
        for (definition, line) in [(&case.before.definition, 1), (&after.definition, 2)] {
            assert_eq!(definition.span.start.line, line);
            assert_eq!(definition.span.start.column, 13);
            assert_eq!(definition.span.end.line, line);
            assert_eq!(definition.span.end.column, 16);
        }
        assert_eq!(after.references, case.before.references);
        assert_eq!(
            query_snapshot(case.refreshed, "main.veln", 9, 25).as_ref(),
            Some(case.other)
        );
        assert_eq!(
            query_snapshot(case.refreshed, "shared.veln", 6, 17).as_ref(),
            Some(case.workspace)
        );
        assert_eq!(
            query_snapshot(case.accepted, "main.veln", 5, 24).as_ref(),
            Some(case.before)
        );
        assert_eq!(
            case.accepted.resolve_virtual_source(case.first_uri),
            Some(case.original.as_bytes())
        );
        assert_eq!(
            case.refreshed.resolve_virtual_source(case.updated_uri),
            Some(case.changed.as_bytes())
        );
        assert!(
            case.accepted
                .resolve_virtual_source(case.updated_uri)
                .is_none()
        );
        assert!(
            case.refreshed
                .resolve_virtual_source(case.first_uri)
                .is_none()
        );
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
    fn direct_dependency_handler_uniqueness_includes_non_exported_sources() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use shared from \"example/dep\"\n\n",
                    "fn duplicate() -> Int\n  handle 1 with shared::run()\nend\n\n",
                    "fn valid() -> Int\n  handle 2 with shared::keep()\nend\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[
                    (
                        "public.veln",
                        concat!(
                            "mod shared\n\n",
                            "pub handler run() for Work\n  go() => 1\nend\n\n",
                            "pub handler keep() for Work\n  go() => 2\nend\n",
                        ),
                    ),
                    (
                        "internal.veln",
                        "mod shared\n\npub handler run() for Work\n  go() => 3\nend\n",
                    ),
                ],
                ["public.veln"],
            )],
        );

        let duplicate_position = || SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 4,
            column: 25,
        };
        assert_eq!(definition_at(&snapshot, duplicate_position()), None);
        assert!(navigate(&snapshot, duplicate_position()).is_none());

        let valid = query_snapshot(&snapshot, "main.veln", 8, 25).unwrap();
        assert_eq!(valid.definition.span.file.as_str(), "public.veln");
        assert_eq!(valid.definition.span.start.line, 7);
        assert_eq!(valid.definition.span.start.column, 13);
        assert!(valid.reference_eligible);
        assert_eq!(locations(&valid.references), [("main.veln", 8, 25)]);
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
    fn recovered_dependency_import_does_not_quarantine_later_valid_handler_import() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep as bad from \"stable/pkg\" unexpected\n",
                    "use dep as wire from \"stable/pkg\"\n\n",
                    "fn invalid() -> Int\n  handle 0 with bad::run()\nend\n\n",
                    "fn first() -> Int\n  handle 1 with wire::run()\nend\n\n",
                    "fn second() -> Int\n  handle 2 with dep::run()\nend\n",
                ),
            )],
            vec![stable_package_handler()],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 5, 22).is_none());
        let aliased = query_snapshot(&snapshot, "main.veln", 9, 23).unwrap();
        assert!(matches!(
            aliased.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(
            locations(&aliased.references),
            [("main.veln", 9, 23), ("main.veln", 13, 22)]
        );
        let exact = query_snapshot(&snapshot, "main.veln", 13, 22).unwrap();
        assert_eq!(exact.selected_symbol, aliased.selected_symbol);
        assert_eq!(exact.definition, aliased.definition);
        assert_eq!(exact.references, aliased.references);
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
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![stable_package_handler_use()],
            vec![stable_package_handler(), other, bridge],
        );
        let expected = query_snapshot(&snapshot, "stable.veln", 4, 22).unwrap();
        assert_eq!(locations(&expected.references), [("stable.veln", 4, 22)]);
        let ordinary_function = query_snapshot(&snapshot, "stable.veln", 8, 8).unwrap();
        assert_eq!(ordinary_function.selected_symbol.kind, SymbolKind::Function);

        let stable_import = "use dep from \"stable/pkg\"\n";
        for (case, imports, application) in [
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
            (
                "incomplete function in handler position",
                stable_import,
                "dep::work(",
            ),
            (
                "recovered function in handler position",
                stable_import,
                "dep::work(1 2)",
            ),
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
