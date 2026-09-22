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
                    "  pick() -> Int\n",
                    "end\n\n",
                    "type Choose\n",
                    "  Choose\n",
                    "end\n\n",
                    "handler choose() handles Choose\n",
                    "  pick() => perform Choose::pick()\n",
                    "end\n\n",
                    "fn collisions(value: Choose) -> String effects [Choose]\n",
                    "  # Choose perform Choose::pick()\n",
                    "  \"Choose\"\n",
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
                ("main.veln", 9, 26),
                ("main.veln", 10, 21),
                ("main.veln", 13, 49),
            ]
        );

        let operation = query(sources, "main.veln", 10, 29).unwrap();
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
        let local_result = query(vec![source("main.veln", local)], "main.veln", 1, 8).unwrap();
        assert_eq!(locations(&local_result.references), []);
        for (line, column) in [(5, 43), (9, 40), (10, 11), (10, 20)] {
            assert!(query(vec![source("main.veln", local)], "main.veln", line, column).is_none());
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
    }
}
