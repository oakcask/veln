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
                    "handler chooser() for Choose\n",
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
    fn imported_workspace_effect_operation_references_share_one_identity() {
        let sources = vec![
            source(
                "library/fx.veln",
                concat!(
                    "pub effect Remote\n",
                    "  run() -> Int\n",
                    "end\n\n",
                    "fn local() -> Int effects [Remote]\n",
                    "  perform Remote::run()\n",
                    "end\n\n",
                    "handler local_handler() for Remote\n",
                    "  run() => 1\n",
                    "end\n",
                ),
            ),
            source(
                "consumer.veln",
                concat!(
                    "use library::fx\n\n",
                    "fn alias() -> Int effects [fx::Remote]\n",
                    "  perform fx::Remote::run()\n",
                    "end\n\n",
                    "fn full() -> Int effects [library::fx::Remote]\n",
                    "  perform library::fx::Remote::run()\n",
                    "end\n\n",
                    "handler imported_handler() for fx::Remote\n",
                    "  run() => 2\n",
                    "end\n",
                ),
            ),
        ];
        let expected = [
            ("consumer.veln", 4, 23, 4, 26),
            ("consumer.veln", 8, 32, 8, 35),
            ("library/fx.veln", 6, 19, 6, 22),
            ("library/fx.veln", 10, 3, 10, 6),
        ];

        for (path, line, column) in [
            ("library/fx.veln", 2, 3),
            ("library/fx.veln", 6, 19),
            ("library/fx.veln", 10, 3),
            ("consumer.veln", 4, 23),
            ("consumer.veln", 8, 32),
        ] {
            let result = query(sources.clone(), path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::EffectOperation);
            assert_eq!(result.selected_symbol.name, "run");
            assert!(result.reference_eligible);
            assert_location(&result.definition, "library/fx.veln", 2, 3);
            assert_eq!(exact_locations(&result.references), expected);
        }

        assert!(query(sources, "consumer.veln", 12, 3).is_none());
    }

    #[test]
    fn imported_workspace_effect_operation_resolution_obeys_import_identity() {
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
                    "fn exact() -> Int\n",
                    "  perform fx::Remote::run()\n",
                    "end\n\n",
                    "fn full() -> Int\n",
                    "  perform library::fx::Remote::run()\n",
                    "end\n",
                ),
            ),
        ];
        let exact = query(precedence.clone(), "consumer.veln", 5, 23).unwrap();
        assert_location(&exact.definition, "fx.veln", 2, 3);
        let full = query(precedence, "consumer.veln", 9, 32).unwrap();
        assert_location(&full.definition, "library/fx.veln", 2, 3);

        for (label, imports) in [
            ("ambiguous implicit leaf", "use first::fx\nuse second::fx\n"),
            ("duplicate import", "use first::fx\nuse first::fx\n"),
            ("recovered import", "use first::fx unexpected\n"),
        ] {
            let invalid_line = imports.lines().count() + 4;
            let valid_line = imports.lines().count() + 8;
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
                        "use stable\n{imports}\nfn invalid() -> Int\n  perform fx::Remote::run()\nend\n\nfn valid() -> Int\n  perform stable::Stable::run()\nend\n"
                    ),
                ),
            ];
            assert!(
                query(sources.clone(), "consumer.veln", invalid_line, 23).is_none(),
                "{label}"
            );
            let valid = query(sources, "consumer.veln", valid_line, 27).unwrap();
            assert_location(&valid.definition, "stable.veln", 2, 3);
        }
    }

    #[test]
    fn imported_workspace_effect_operation_references_reject_ineligible_owners_and_operations() {
        for (label, declaration, effect, operation, unsupported_selection) in [
            (
                "private effect",
                "effect Remote\n  run() -> Int\nend\n",
                "Remote",
                "run",
                false,
            ),
            (
                "invalid effect casing",
                "pub effect remote\n  run() -> Int\nend\n",
                "remote",
                "run",
                true,
            ),
            (
                "invalid operation casing",
                "pub effect Remote\n  Run() -> Int\nend\n",
                "Remote",
                "Run",
                true,
            ),
            (
                "unknown operation",
                "pub effect Remote\n  run() -> Int\nend\n",
                "Remote",
                "missing",
                false,
            ),
        ] {
            let sources = vec![
                source("library/fx.veln", declaration),
                source(
                    "consumer.veln",
                    &format!(
                        "use library::fx\n\nfn use() -> Int\n  perform fx::{effect}::{operation}()\nend\n"
                    ),
                ),
            ];
            let selected = query(sources, "consumer.veln", 4, 23);
            assert_eq!(selected.is_some(), unsupported_selection, "{label}");
            if let Some(result) = selected {
                assert!(!result.reference_eligible, "{label}");
                assert!(result.references.is_empty(), "{label}");
            }
        }

        for declaration in [
            "pub effect Remote\n  run() -> Int\n  run(value: Int) -> Int\nend\n",
            "pub effect Remote\n  run() Int\nend\n",
            "pub effect Remote\n  run() -> Int\n  broken()\nend\n",
        ] {
            let sources = vec![
                source("library/fx.veln", declaration),
                source(
                    "consumer.veln",
                    "use library::fx\n\nfn use() -> Int\n  perform fx::Remote::run()\nend\n",
                ),
            ];
            assert!(query(sources, "consumer.veln", 4, 23).is_none());
        }

        let duplicate_owner = vec![
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
                "use library::fx\n\nfn use() -> Int\n  perform fx::Remote::run()\nend\n",
            ),
        ];
        assert!(query(duplicate_owner, "consumer.veln", 4, 23).is_none());
    }

    #[test]
    fn imported_workspace_effect_operation_references_obey_shape_boundaries() {
        for expression in [
            "perform fx::::run()",
            "perform fx::Remote::()",
            "perform fx::Remote::run",
            "perform fx::Remote::run(",
            "perform fx::Remote::extra::run()",
            "perform fx::Remote::run(1 2)",
        ] {
            let source_text = format!(
                "use library::fx\n\nfn broken() -> Int\n  {expression}\nend\n"
            );
            let sources = vec![
                source(
                    "library/fx.veln",
                    "pub effect Remote\n  run(value: Int) -> Int\nend\n",
                ),
                source("consumer.veln", &source_text),
            ];
            if let Some(operation_offset) = expression.rfind("run") {
                let operation_column = 3 + operation_offset;
                let selected = query(
                    sources.clone(),
                    "consumer.veln",
                    4,
                    operation_column,
                );
                assert!(
                    selected.is_none_or(|result| {
                        result.selected_symbol.kind != SymbolKind::EffectOperation
                    }),
                    "{expression}"
                );
            }
            if let Some(effect_offset) = expression.find("Remote") {
                let effect_column = 3 + effect_offset;
                if let Some(result) = query(
                    sources.clone(),
                    "consumer.veln",
                    4,
                    effect_column,
                ) {
                    assert_eq!(result.selected_symbol.kind, SymbolKind::Effect, "{expression}");
                }
            }
            assert!(query(sources.clone(), "consumer.veln", 1, 5).is_none());
            assert!(query(sources, "consumer.veln", 4, 11).is_none());
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
                "handler choose_handler() for Choose\n",
                "  pick() => 1\n",
                "end\n\n",
                "handler other_handler() for Other\n",
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
                "handler duplicate() for Choose\n",
                "  pick() => 1\n",
                "  pick() => 2\n",
                "  keep() => 3\n",
                "end\n\n",
                "handler unknown() for Choose\n",
                "  missing() => 0\n",
                "end\n\n",
                "handler broken() for Choose\n",
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
                    "mod shared\n\neffect Choose\n  pick() -> Int\nend\n\nhandler choose() for Choose\n  pick() => 1\nend\n",
                ),
                source(
                    "second.veln",
                    "mod shared\n\nhandler choose() for Choose\n  pick() => 2\nend\n",
                ),
                ],
                4,
                8,
            ),
            (
                vec![source(
                    "first.veln",
                    "effect Choose\n  pick() -> Int\nend\n\nhandler choose() for Choose\n  pick() => 1\n",
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
    fn same_named_handlers_in_distinct_modules_keep_clause_headings() {
        let sources = vec![
            source(
                "first.veln",
                concat!(
                    "mod first\n\n",
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "handler choose() for Choose\n",
                    "  pick() => 1\n",
                    "end\n",
                ),
            ),
            source(
                "second.veln",
                concat!(
                    "mod second\n\n",
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "handler choose() for Choose\n",
                    "  pick() => 2\n",
                    "end\n",
                ),
            ),
        ];

        let first = query(sources.clone(), "first.veln", 4, 3).unwrap();
        assert_eq!(locations(&first.references), [("first.veln", 8, 3)]);

        let second = query(sources, "second.veln", 4, 3).unwrap();
        assert_eq!(locations(&second.references), [("second.veln", 8, 3)]);
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
                    "handler qualified() for foreign::Choose\n",
                    "  pick() => 1\n",
                    "end\n\n",
                    "handler unresolved() for Missing\n",
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
                "handler direct() for tasks::Task\n",
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
                "handler standard() for tasks::Task\n",
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
    fn workspace_effect_operation_clause_references_reject_duplicate_and_recovered_owners() {
        for (text, heading_line) in [
            (
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "  pick(value: Int) -> Int\n",
                    "end\n\n",
                    "handler chooser() for Choose\n",
                    "  pick() => 1\n",
                    "end\n",
                ),
                7,
            ),
            (
                concat!(
                    "effect Choose\n",
                    "  pick() Int\n",
                    "end\n\n",
                    "handler chooser() for Choose\n",
                    "  pick() => 1\n",
                    "end\n",
                ),
                6,
            ),
            (
                concat!(
                    "effect Choose\n",
                    "  pick() -> Int\n",
                    "end\n\n",
                    "handler chooser() for Choose @\n",
                    "  pick() => 1\n",
                    "end\n",
                ),
                6,
            ),
        ] {
            let sources = vec![source("main.veln", text)];
            let operation = query(sources.clone(), "main.veln", 2, 3).unwrap();
            assert!(operation.references.is_empty());
            assert!(query(sources, "main.veln", heading_line, 3).is_none());
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

        let transitive = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use upstream from \"up/pkg\"\n\n",
                    "fn use() -> Int effects [upstream::Task]\n",
                    "  perform upstream::Task::run()\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/bridge",
                &[(
                    "bridge.veln",
                    concat!(
                        "use upstream from \"up/pkg\"\n\n",
                        "pub fn bridge() -> Int effects [upstream::Task]\n",
                        "  perform upstream::Task::run()\n",
                        "end\n",
                    ),
                )],
                ["bridge.veln"],
            )],
        );
        assert!(query_snapshot(&transitive, "main.veln", 4, 27).is_none());

        let standard = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use tasks from \"std\"\n\n",
                "fn use() -> Int effects [tasks::Task]\n",
                "  perform tasks::Task::run()\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("tasks.veln", "pub effect Task\n  run() -> Int\nend\n")],
            ["tasks.veln"],
        ));
        assert!(query_snapshot(&standard, "main.veln", 4, 24).is_none());
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
                    "handler ambiguous() for Choose\n",
                    "  pick() => 1\n",
                    "end\n\n",
                    "handler other() for Other\n",
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

    #[test]
    fn same_named_workspace_effect_operation_lookup_work_is_adjacent_linear() {
        use std::fmt::Write as _;

        fn measured_identity_work(count: usize) -> ((usize, usize), std::time::Duration) {
            let mut sources = Vec::with_capacity(count + 1);
            let mut consumer = String::new();
            for index in 0..count {
                let module = format!("module_{index}");
                sources.push(SourceFile::new(
                    format!("{module}.veln"),
                    "pub effect Remote\n  run() -> Int\nend\n",
                ));
                writeln!(consumer, "use {module}").unwrap();
            }
            consumer.push_str("\nfn use_all() -> Int\n");
            for index in 0..count {
                writeln!(consumer, "  perform module_{index}::Remote::run()").unwrap();
            }
            consumer.push_str("end\n");
            sources.push(source("consumer.veln", &consumer));
            let snapshot = EffectiveProjectSnapshot::new(sources);
            let position = || SourcePosition {
                source: SourcePath::new("module_0.veln"),
                line: 2,
                column: 3,
            };
            navigate(&snapshot, position()).unwrap();
            crate::navigation::reset_effect_operation_identity_work();
            let started = std::time::Instant::now();
            let result = navigate(&snapshot, position()).unwrap();
            assert_eq!(result.references.len(), 1);
            (
                crate::navigation::effect_operation_identity_work(),
                started.elapsed(),
            )
        }

        let (smaller_work, smaller_elapsed) = measured_identity_work(100);
        let (larger_work, larger_elapsed) = measured_identity_work(200);
        eprintln!(
            "same-named effect-operation lookup: 100={smaller_elapsed:?}/{smaller_work:?}, 200={larger_elapsed:?}/{larger_work:?}"
        );
        assert!(smaller_work.0 <= 100 * 3 + 10, "{smaller_work:?}");
        assert!(smaller_work.1 <= 100 * 2 + 10, "{smaller_work:?}");
        assert!(larger_work.0 <= 200 * 3 + 10, "{larger_work:?}");
        assert!(larger_work.1 <= 200 * 2 + 10, "{larger_work:?}");
        assert!(larger_work.0 <= smaller_work.0 * 2 + 10);
        assert!(larger_work.1 <= smaller_work.1 * 2 + 10);
    }
}
