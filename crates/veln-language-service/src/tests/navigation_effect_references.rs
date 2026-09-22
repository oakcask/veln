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
            assert_eq!(locations(&result.references), expected, "{path}:{line}:{column}");
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
    fn workspace_effect_references_reject_unsupported_origins_ambiguity_and_recovery() {
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
            source("foreign.veln", "mod foreign\n\neffect E\n  run() -> Int\nend\n"),
        ];
        let local_result = query(
            qualified_workspace_sources.clone(),
            "main.veln",
            1,
            8,
        )
        .unwrap();
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

        let ambiguous = vec![
            source("first.veln", "mod shared\n\neffect Choose\n  first() -> Int\nend\n"),
            source(
                "second.veln",
                "mod shared\n\neffect Choose\n  second() -> Int\nend\n\nfn use() -> Int effects [Choose]\n  1\nend\n",
            ),
        ];
        let ambiguous_declaration = query(ambiguous.clone(), "first.veln", 3, 8).unwrap();
        assert!(ambiguous_declaration.references.is_empty());
        assert!(!ambiguous_declaration.reference_eligible);
        assert!(query(ambiguous, "second.veln", 7, 27).is_none());

        let recovered = "effect Choose\n  pick() -> Int\nend\n\nfn broken() -> Int effects [Choose\n  1\nend\n";
        assert!(query(vec![source("main.veln", recovered)], "main.veln", 5, 29).is_none());

        let recovered_declaration = query(
            vec![source(
                "main.veln",
                "effect Choose\n  pick() -> Int\n",
            )],
            "main.veln",
            1,
            8,
        )
        .unwrap();
        assert!(!recovered_declaration.reference_eligible);
        assert!(recovered_declaration.references.is_empty());

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
        assert!(query(
            vec![source("invalid.veln", invalid_casing)],
            "invalid.veln",
            5,
            29,
        )
        .is_none());
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
    fn workspace_effect_reference_collection_handles_many_declarations_and_occurrences() {
        let mut declarations =
            String::from("mod shared\n\neffect Choose\n  pick() -> Int\nend\n\n");
        let mut uses = String::from("mod shared\n\n");
        for index in 0..256 {
            declarations.push_str(&format!(
                "effect Noise{index}\n  ignore() -> Int\nend\n\n"
            ));
            uses.push_str(&format!(
                "fn use_{index}() -> Int effects [Choose]\n  {index}\nend\n\n"
            ));
        }
        let result = query(
            vec![source("declarations.veln", &declarations), source("uses.veln", &uses)],
            "declarations.veln",
            3,
            8,
        )
        .unwrap();
        assert_eq!(result.references.len(), 256);
    }

    #[test]
    fn workspace_effect_reference_collection_keeps_long_rows_adjacent_linear() {
        for count in [1_000, 2_000, 4_000] {
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
            let source_scalar_visits =
                crate::navigation::effect_reference_source_scalar_visits();
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
}
