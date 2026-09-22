mod navigation_handler_references_tests {
    use super::*;

    fn shared_sources() -> Vec<SourceFile> {
        vec![
            source(
                "declaration.veln",
                "mod shared\n\nhandler run(value: Int) handles Work\n  go() => 1\nend\n",
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
                    "handler run() handles Work\n",
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
                    "handler run() handles Work\n",
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
                    "handler wrapper(run: fn(Int) -> Int) handles Work\n",
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
        let duplicate = vec![
            source(
                "first.veln",
                "mod shared\n\nhandler run() handles Work\n  go() => 1\nend\n",
            ),
            source(
                "second.veln",
                concat!(
                    "mod shared\n\n",
                    "handler run() handles Work\n  go() => 2\nend\n\n",
                    "handler keep() handles Work\n  go() => 3\nend\n\n",
                    "fn use() -> Int\n  handle 1 with run()\nend\n\n",
                    "fn valid() -> Int\n  handle 2 with keep()\nend\n",
                ),
            ),
        ];
        let declaration = query(duplicate.clone(), "first.veln", 3, 9).unwrap();
        assert!(!declaration.reference_eligible);
        assert!(declaration.references.is_empty());
        assert!(query(duplicate.clone(), "second.veln", 12, 17).is_none());
        let valid = query(duplicate, "second.veln", 16, 17).unwrap();
        assert!(valid.reference_eligible);
        assert_eq!(locations(&valid.references), [("second.veln", 16, 17)]);

        let recovered = vec![source(
            "main.veln",
            concat!(
                "handler run() handles Work @\n  go() => 1\nend\n\n",
                "handler keep() handles Work\n  go() => 2\nend\n\n",
                "fn use() -> Int\n  handle 1 with run()\nend\n\n",
                "fn valid() -> Int\n  handle 2 with keep()\nend\n",
            ),
        )];
        let recovered_declaration = query(recovered.clone(), "main.veln", 1, 9).unwrap();
        assert!(!recovered_declaration.reference_eligible);
        assert!(recovered_declaration.references.is_empty());
        assert!(query(recovered.clone(), "main.veln", 10, 17).is_none());
        let valid = query(recovered, "main.veln", 14, 17).unwrap();
        assert_eq!(locations(&valid.references), [("main.veln", 14, 17)]);
    }

    #[test]
    fn workspace_handler_references_reject_declaration_with_non_handler_diagnostic_context() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "handler run() handles Work\n  go() => 0x1_0\nend\n\n",
                "handler keep() handles Work\n  go() => 2\nend\n\n",
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
                "handler run() handles Work\n  go() => 1\nend\n\n",
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
    fn workspace_handler_references_reject_resolved_imported_workspace_paths() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "use other\n\n",
                    "handler stable() handles Work\n  go() => 1\nend\n\n",
                    "fn imported() -> Int\n  handle 1 with other::run()\nend\n\n",
                    "fn valid() -> Int\n  handle 2 with stable()\nend\n",
                ),
            ),
            source(
                "other.veln",
                "pub handler run() handles Work\n  go() => 2\nend\n",
            ),
        ];

        assert!(query(sources.clone(), "main.veln", 8, 23).is_none());
        let valid = query(sources, "main.veln", 12, 17).unwrap();
        assert_eq!(locations(&valid.references), [("main.veln", 12, 17)]);
    }

    #[test]
    fn workspace_handler_references_reject_ambiguous_paths_without_losing_valid_selection() {
        let sources = vec![
            source(
                "first.veln",
                "mod shared\n\nhandler run() handles Work\n  go() => 1\nend\n",
            ),
            source(
                "second.veln",
                concat!(
                    "mod shared\n\n",
                    "handler run() handles Work\n  go() => 2\nend\n\n",
                    "handler stable() handles Work\n  go() => 3\nend\n\n",
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
    fn workspace_handler_references_reject_package_backed_paths() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "handler run() handles Work\n  go() => 1\nend\n\n",
                    "fn local() -> Int\n  handle 1 with run()\nend\n\n",
                    "fn imported() -> Int\n  handle 2 with dep::run()\nend\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[(
                    "dep.veln",
                    "pub handler run() handles Work\n  go() => 2\nend\n",
                )],
                ["dep.veln"],
            )],
        );
        let local = query_snapshot(&snapshot, "main.veln", 8, 17).unwrap();
        assert_eq!(locations(&local.references), [("main.veln", 8, 17)]);
        assert!(query_snapshot(&snapshot, "main.veln", 12, 22).is_none());
    }

    #[test]
    fn workspace_handler_references_keep_valid_selection_beside_unrelated_parse_errors() {
        let sources = vec![
            source(
                "declaration.veln",
                "mod shared\n\nhandler run() handles Work\n  go() => 1\nend\n",
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

    #[test]
    fn workspace_handler_reference_indexing_work_grows_linearly() {
        fn measured_work(count: usize) -> (usize, usize, usize, std::time::Duration) {
            let mut body = String::from(
                "handler run() handles Work\n  go() => 1\nend\n\nfn consume() -> Int\n",
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

        let (small_tokens, small_diagnostics, small_queries, _) = measured_work(500);
        let (large_tokens, large_diagnostics, large_queries, _) = measured_work(1_000);
        assert!(large_tokens <= small_tokens * 2 + 16);
        assert_eq!(small_diagnostics, 0);
        assert_eq!(large_diagnostics, 0);
        assert!(small_queries <= 501, "{small_queries}");
        assert!(large_queries <= 1_001, "{large_queries}");
        assert!(large_queries <= small_queries * 2 + 1);

        fn measured_nested_work(depth: usize) -> (usize, std::time::Duration) {
            let mut body = String::from(
                "handler run(value: Int) handles Work\n  go() => 1\nend\n\nfn consume() -> Int\n  ",
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

        let (small_nested_tokens, _) = measured_nested_work(50);
        let (large_nested_tokens, _) = measured_nested_work(100);
        assert!(large_nested_tokens <= small_nested_tokens * 2 + 16);

        fn measured_recovery_work(count: usize) -> (usize, usize) {
            let mut body = String::from(
                "handler run() handles Work\n  go() => 1\nend\n\nfn consume() -> Int\n",
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

        let (small_diagnostics, small_queries) = measured_recovery_work(250);
        let (large_diagnostics, large_queries) = measured_recovery_work(500);
        assert_eq!(small_diagnostics, 250);
        assert_eq!(large_diagnostics, 500);
        assert!(large_queries <= small_queries * 2 + 1);
    }
}
