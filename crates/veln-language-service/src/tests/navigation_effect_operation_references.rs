mod navigation_effect_operation_references_tests {
    use super::*;

    fn operation_sources() -> Vec<SourceFile> {
        vec![
            source(
                "declaration.veln",
                concat!(
                    "mod shared\n\n",
                    "effect Choose\n",
                    "  pick(value: Bool) -> Int\n",
                    "  skip() -> Int\n",
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
                    "effect Other\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "fn use() -> Int effects [Choose, Other]\n",
                    "  perform Choose::pick(false) + perform Other::pick()\n",
                    "end\n\n",
                    "handler chooser() handles Choose\n",
                    "  pick(value) => value\n",
                    "  skip() => 0\n",
                    "end\n",
                ),
            ),
        ]
    }

    #[test]
    fn workspace_effect_operation_references_share_identity_across_selection_forms() {
        let expected = [
            ("declaration.veln", 9, 19, 9, 23),
            ("uses.veln", 8, 19, 8, 23),
            ("uses.veln", 12, 3, 12, 7),
        ];
        for (path, line, column) in [
            ("declaration.veln", 4, 3),
            ("declaration.veln", 9, 19),
            ("uses.veln", 8, 19),
            ("uses.veln", 12, 3),
        ] {
            let result = query(operation_sources(), path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::EffectOperation);
            assert_eq!(result.selected_symbol.name, "pick");
            assert!(result.reference_eligible);
            assert_location(&result.definition, "declaration.veln", 4, 3);
            assert_eq!(exact_locations(&result.references), expected);
        }
    }

    #[test]
    fn workspace_effect_operation_references_select_multiline_operation_leaves() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "fn use() -> Int\n",
                "  (perform Choose::\n",
                "    pick())\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        let leaf = query(sources, "main.veln", 7, 5).unwrap();
        assert_eq!(leaf.selected_symbol, declaration.selected_symbol);
        assert_eq!(leaf.definition, declaration.definition);
        assert_eq!(leaf.references, declaration.references);
        assert_eq!(exact_locations(&leaf.references), [("main.veln", 7, 5, 7, 9)]);
    }

    fn exact_locations(spans: &[SourceSpan]) -> Vec<(&str, usize, usize, usize, usize)> {
        spans
            .iter()
            .map(|span| {
                (
                    span.file.as_str(),
                    span.start.line,
                    span.start.column,
                    span.end.line,
                    span.end.column,
                )
            })
            .collect()
    }

    #[test]
    fn workspace_effect_operation_references_exclude_other_identities_and_spelling_collisions() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "effect Other\n",
                "  pick() -> Int\n",
                "end\n\n",
                "type Choice\n",
                "  pick\n",
                "end\n\n",
                "handler choose_handler() handles Choose\n",
                "  pick() => 1\n",
                "end\n\n",
                "handler other_handler() handles Other\n",
                "  pick() => 2\n",
                "end\n\n",
                "fn pick() -> String effects [Choose, Other]\n",
                "  let pick = {pick: \"pick\"}\n",
                "  # pick\n",
                "  perform Choose::pick()\n",
                "  perform Other::pick()\n",
                "end\n",
            ),
        ), source(
            "other.veln",
            "effect Choose\n  pick() -> Int\nend\n\nfn use() -> Int\n  perform Choose::pick()\nend\n",
        )];
        let selected = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert_eq!(
            locations(&selected.references),
            [("main.veln", 14, 3), ("main.veln", 24, 19)]
        );
        let choose_clause = query(sources.clone(), "main.veln", 14, 3).unwrap();
        assert_eq!(choose_clause.selected_symbol, selected.selected_symbol);
        assert_eq!(choose_clause.references, selected.references);
        let other = query(sources.clone(), "main.veln", 25, 18).unwrap();
        assert_eq!(other.selected_symbol.kind, SymbolKind::EffectOperation);
        assert_location(&other.definition, "main.veln", 6, 3);
        assert_eq!(
            locations(&other.references),
            [("main.veln", 18, 3), ("main.veln", 25, 18)]
        );
        let other_clause = query(sources.clone(), "main.veln", 18, 3).unwrap();
        assert_eq!(other_clause.selected_symbol, other.selected_symbol);
        assert_eq!(other_clause.references, other.references);
        let other_module = query(sources, "other.veln", 6, 19).unwrap();
        assert_location(&other_module.definition, "other.veln", 2, 3);
        assert_eq!(locations(&other_module.references), [("other.veln", 6, 19)]);
    }

    #[test]
    fn workspace_effect_operation_clause_references_reject_invalid_handlers_and_headings() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "  keep() -> Int\n",
                "end\n\n",
                "handler duplicate() handles Choose\n",
                "  pick() => 1\n",
                "  pick() => 2\n",
                "  keep() => 3\n",
                "end\n\n",
                "handler unknown() handles Choose\n",
                "  missing() => 0\n",
                "end\n\n",
                "handler broken() handles Choose\n",
                "  pick( => 0\n",
                "end\n\n",
                "fn use() -> Int\n",
                "  perform Choose::pick() + perform Choose::keep()\n",
                "end\n",
            ),
        )];

        let pick = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert_eq!(locations(&pick.references), [("main.veln", 21, 19)]);
        for (line, column) in [(7, 3), (8, 3), (13, 3), (17, 3)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
        let keep = query(sources.clone(), "main.veln", 3, 3).unwrap();
        assert_eq!(
            locations(&keep.references),
            [("main.veln", 9, 3), ("main.veln", 21, 44)]
        );
        let keep_clause = query(sources, "main.veln", 9, 3).unwrap();
        assert_eq!(keep_clause.selected_symbol, keep.selected_symbol);
    }

    #[test]
    fn duplicate_or_recovered_handler_declarations_exclude_clause_headings() {
        for (sources, operation_line, heading_line) in [
            (
                vec![
                source(
                    "first.veln",
                    "mod shared\n\neffect Choose\n  pick() -> Int\nend\n\nhandler choose() handles Choose\n  pick() => 1\nend\n",
                ),
                source(
                    "second.veln",
                    "mod shared\n\nhandler choose() handles Choose\n  pick() => 2\nend\n",
                ),
                ],
                4,
                8,
            ),
            (
                vec![source(
                    "first.veln",
                    "effect Choose\n  pick() -> Int\nend\n\nhandler choose() handles Choose\n  pick() => 1\n",
                )],
                2,
                6,
            ),
        ] {
            let operation = query(sources.clone(), "first.veln", operation_line, 3).unwrap();
            assert!(operation.references.is_empty());
            assert!(query(sources, "first.veln", heading_line, 3).is_none());
        }
    }

    #[test]
    fn handler_clause_headings_require_a_bare_same_module_workspace_effect() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "use foreign\n\n",
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "handler qualified() handles foreign::Choose\n",
                    "  pick() => 1\n",
                    "end\n\n",
                    "handler unresolved() handles Missing\n",
                    "  pick() => 2\n",
                    "end\n",
                ),
            ),
            source(
                "foreign.veln",
                "mod foreign\n\neffect Choose\n  pick() -> Int\nend\n",
            ),
        ];
        let operation = query(sources.clone(), "main.veln", 4, 3).unwrap();
        assert!(operation.references.is_empty());
        assert!(query(sources.clone(), "main.veln", 8, 3).is_none());
        assert!(query(sources, "main.veln", 12, 3).is_none());
    }

    #[test]
    fn handler_clause_headings_reject_package_and_standard_library_effects() {
        let workspace_source = source(
            "main.veln",
            concat!(
                "use tasks from \"example/tasks\"\n\n",
                "handler direct() handles tasks::Task\n",
                "  run() => 1\n",
                "end\n",
            ),
        );
        let direct = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![workspace_source.clone()],
            vec![dependency_snapshot(
                "example/tasks",
                &[("tasks.veln", "pub effect Task\n  run() -> Int\nend\n")],
                ["tasks.veln"],
            )],
        );
        assert!(query_snapshot(&direct, "main.veln", 4, 3).is_none());

        let standard = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use tasks from \"std\"\n\n",
                "handler standard() handles tasks::Task\n",
                "  run() => 1\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
                &[("tasks.veln", "pub effect Task\n  run() -> Int\nend\n")],
                ["tasks.veln"],
            ));
        assert!(query_snapshot(&standard, "main.veln", 4, 3).is_none());
    }

    #[test]
    fn workspace_effect_operation_references_reject_unresolved_paths() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "fn valid() -> Int\n",
                "  perform Choose::pick()\n",
                "end\n\n",
                "fn missing_effect() -> Int\n",
                "  perform Missing::pick()\n",
                "end\n\n",
                "fn missing_operation() -> Int\n",
                "  perform Choose::missing()\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert_eq!(locations(&declaration.references), [("main.veln", 6, 19)]);
        assert!(query(sources.clone(), "main.veln", 10, 20).is_none());
        assert!(query(sources, "main.veln", 14, 19).is_none());
    }

    #[test]
    fn effect_operation_selection_precedes_broad_schema_candidates() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "schema S\n",
                "  format binary\n",
                "end\n\n",
                "schema T\n",
                "  format binary\n",
                "end\n\n",
                "fn use(bytes: Bytes) -> Int\n",
                "  decode S from (perform Choose::pick() + decode T from bytes)\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        let leaf = query(sources, "main.veln", 14, 34).unwrap();
        assert_eq!(leaf.selected_symbol.kind, SymbolKind::EffectOperation);
        assert_eq!(leaf.selected_symbol, declaration.selected_symbol);
        assert_eq!(leaf.references, declaration.references);
        assert_eq!(exact_locations(&leaf.references), [("main.veln", 14, 34, 14, 38)]);
    }

    #[test]
    fn workspace_effect_operation_references_reject_recovered_arguments() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick(value: Int) -> Int\n",
                "end\n\n",
                "fn broken() -> Int\n",
                "  perform Choose::pick(1 2)\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert!(declaration.references.is_empty());
        assert!(query(sources, "main.veln", 6, 19).is_none());
    }

    #[test]
    fn workspace_effect_operation_references_reject_ambiguous_and_recovered_declarations() {
        for text in [
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "  pick(value: Int) -> Int\n",
                "end\n\n",
                "fn use() -> Int\n",
                "  perform Choose::pick()\n",
                "end\n",
            ),
            concat!(
                "effect Choose\n",
                "  pick() Int\n",
                "end\n\n",
                "fn use() -> Int\n",
                "  perform Choose::pick()\n",
                "end\n",
            ),
        ] {
            let sources = vec![source("main.veln", text)];
            let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
            assert!(!declaration.reference_eligible);
            assert!(declaration.references.is_empty());
            assert!(query(sources, "main.veln", 7, 19).is_none());
        }
    }

    #[test]
    fn workspace_effect_operation_references_reject_recovered_owning_effects() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "  broken()\n",
                "end\n\n",
                "effect Other\n",
                "  run() -> Int\n",
                "end\n\n",
                "fn use() -> Int\n",
                "  perform Choose::pick()\n",
                "  perform Other::run()\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert!(!declaration.reference_eligible);
        assert!(declaration.references.is_empty());
        assert!(query(sources.clone(), "main.veln", 11, 19).is_none());

        let unrelated = query(sources, "main.veln", 12, 18).unwrap();
        assert!(unrelated.reference_eligible);
        assert_location(&unrelated.definition, "main.veln", 7, 3);
        assert_eq!(locations(&unrelated.references), [("main.veln", 12, 18)]);
    }

    #[test]
    fn workspace_effect_operation_references_reject_recovered_operation_paths() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "effect Other\n",
                "  run() -> Int\n",
                "end\n\n",
                "fn broken() -> Int\n",
                "  perform Choose::pick::()\n",
                "end\n\n",
                "fn valid() -> Int\n",
                "  perform Other::run()\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert!(declaration.references.is_empty());
        assert!(query(sources.clone(), "main.veln", 10, 19).is_none());

        let unrelated = query(sources, "main.veln", 14, 18).unwrap();
        assert!(unrelated.reference_eligible);
        assert_location(&unrelated.definition, "main.veln", 6, 3);
        assert_eq!(locations(&unrelated.references), [("main.veln", 14, 18)]);
    }

    #[test]
    fn workspace_effect_operation_references_reject_incomplete_and_qualified_paths() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "fn broken() -> Int\n",
                    "  perform Choose::pick(\n",
                    "end\n\n",
                    "fn qualified() -> Int\n",
                    "  perform foreign::Choose::pick()\n",
                    "end\n",
                ),
            ),
            source(
                "foreign.veln",
                "mod foreign\n\neffect Choose\n  pick() -> Int\nend\n",
            ),
        ];
        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert!(declaration.references.is_empty());
        assert!(query(sources.clone(), "main.veln", 6, 19).is_none());
        assert!(query(sources, "main.veln", 9, 28).is_none());
    }

    #[test]
    fn workspace_effect_operation_references_reject_imported_package_operations() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn use() -> Int effects [dep::Task]\n",
                    "  perform dep::Task::run()\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/dep",
                &[("dep.veln", "pub effect Task\n  run() -> Int\nend\n")],
                ["dep.veln"],
            )],
        );
        assert!(query_snapshot(&snapshot, "main.veln", 4, 22).is_none());
    }

    #[test]
    fn ambiguous_owning_effect_does_not_block_an_unrelated_operation() {
        let sources = vec![
            source(
                "first.veln",
                "mod shared\n\neffect Choose\n  pick() -> Int\nend\n",
            ),
            source(
                "second.veln",
                concat!(
                    "mod shared\n\n",
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "effect Other\n",
                    "  run() -> Int\n",
                    "end\n\n",
                    "fn use() -> Int\n",
                    "  perform Choose::pick()\n",
                    "  perform Other::run()\n",
                    "end\n\n",
                    "handler ambiguous() handles Choose\n",
                    "  pick() => 1\n",
                    "end\n\n",
                    "handler other() handles Other\n",
                    "  run() => 2\n",
                    "end\n",
                ),
            ),
        ];
        assert!(query(sources.clone(), "second.veln", 12, 19).is_none());
        assert!(query(sources.clone(), "second.veln", 17, 3).is_none());
        let unrelated = query(sources.clone(), "second.veln", 13, 18).unwrap();
        assert!(unrelated.reference_eligible);
        assert_location(&unrelated.definition, "second.veln", 8, 3);
        assert_eq!(
            locations(&unrelated.references),
            [("second.veln", 13, 18), ("second.veln", 21, 3)]
        );
        let clause = query(sources, "second.veln", 21, 3).unwrap();
        assert_eq!(clause.selected_symbol, unrelated.selected_symbol);
    }

    #[test]
    fn workspace_effect_operation_references_do_not_bind_generic_effect_qualifiers() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect E\n",
                "  run() -> Int\n",
                "end\n\n",
                "fn generic<effect E>() -> Int effects [...E]\n",
                "  perform E::run()\n",
                "end\n",
            ),
        )];
        let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
        assert!(declaration.references.is_empty());
        assert!(query(sources, "main.veln", 6, 14).is_none());
    }

    #[test]
    fn workspace_effect_operation_references_reject_invalid_casing() {
        for text in [
            "effect choose\n  pick() -> Int\nend\n\nfn use() -> Int\n  perform choose::pick()\nend\n",
            "effect Choose\n  Pick() -> Int\nend\n\nfn use() -> Int\n  perform Choose::Pick()\nend\n",
        ] {
            let sources = vec![source("main.veln", text)];
            let declaration = query(sources.clone(), "main.veln", 2, 3).unwrap();
            assert!(!declaration.reference_eligible);
            assert!(declaration.references.is_empty());
            let occurrence = query(sources, "main.veln", 6, 19).unwrap();
            assert_eq!(occurrence.selected_symbol.kind, SymbolKind::EffectOperation);
            assert!(!occurrence.reference_eligible);
            assert!(occurrence.references.is_empty());
        }
    }

    #[test]
    fn workspace_effect_operation_reference_collection_handles_adjacent_input_sizes() {
        for count in [1_000, 2_000, 4_000] {
            let mut text = String::from("effect Choose\n  pick() -> Int\nend\n\n");
            for index in 0..count {
                text.push_str(&format!(
                    "fn use_{index}() -> Int\n  perform Choose::pick()\nend\n\n"
                ));
            }
            let result = query(vec![source("main.veln", &text)], "main.veln", 2, 3).unwrap();
            assert_eq!(result.references.len(), count);
        }
    }
}
