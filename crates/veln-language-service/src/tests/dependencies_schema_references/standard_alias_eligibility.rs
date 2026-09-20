    #[test]
    fn standard_library_schema_eligibility_and_import_failures_are_empty() {
        assert_standard_schema_declaration_cases();
        assert_ambiguous_standard_schema_import_is_empty();
        assert_exact_standard_schema_import_beats_implicit_collision();
    }

    fn assert_standard_schema_declaration_cases() {
        let cases: &[(&str, &str, &[&str], bool)] = &[
            (
                "wire.veln",
                "schema Packet\n  value: Int\nend\n",
                &["wire.veln"],
                false,
            ),
            (
                "wire.veln",
                "pub schema Packet\n  value: Int\nend\n",
                &[],
                false,
            ),
            (
                "wire.veln",
                "pub schema packet\n  value: Int\nend\n",
                &["wire.veln"],
                false,
            ),
            (
                "wire.veln",
                "pub schema Base\n  value: Int\nend\n\npub schema Packet = Base\n",
                &["wire.veln"],
                true,
            ),
            (
                "wire.veln",
                "pub schema Packet =\n",
                &["wire.veln"],
                false,
            ),
        ];
        for (path, body, exports, eligible) in cases {
            assert_standard_schema_declaration_case(path, body, exports, *eligible);
        }
    }

    fn assert_standard_schema_declaration_case(
        path: &str,
        body: &str,
        exports: &[&'static str],
        eligible: bool,
    ) {
        let main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Packet\n  repeated: Repeat(count, wire::Packet)\n  array: [wire::Packet; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Packet from view at byte_offset(0)?\n  encode wire::Packet from packet\nend\n";
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
            .with_standard_library(standard_library_snapshot(
                &[(path, body)],
                exports.iter().copied(),
            ));
        let selections = alias_selections_on_matching_lines(
            &snapshot,
            main,
            "wire::Packet",
            "Packet",
        );
        assert_eq!(selections.len(), 5, "{path}");
        for (index, result) in selections.into_iter().enumerate() {
            if eligible {
                assert_eq!(
                    locations(&result.expect("eligible standard-library alias").references),
                    [
                        ("main.veln", 5, 17),
                        ("main.veln", 6, 33),
                        ("main.veln", 7, 17),
                        ("main.veln", 11, 16),
                        ("main.veln", 12, 16),
                    ],
                    "{path} at leaf {index}"
                );
            } else {
                assert!(
                    result.is_none_or(|selection| selection.references.is_empty()),
                    "{path} at leaf {index}"
                );
            }
        }
    }

    fn assert_ambiguous_standard_schema_import_is_empty() {
        let ambiguous = EffectiveProjectSnapshot::new(vec![
            source("workspace_import.veln", "mod app\nuse workspace::wire\n"),
            source(
                "main.veln",
                concat!(
                    "mod app\n",
                    "use alpha::wire from \"std\"\n",
                    "use beta::wire from \"std\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        assert!(query_snapshot(&ambiguous, "main.veln", 5, 16).is_none());
    }

    fn assert_exact_standard_schema_import_beats_implicit_collision() {
        let exact = EffectiveProjectSnapshot::new(vec![
            source(
                "imports.veln",
                "mod app\nuse alpha::wire from \"std\"\nuse beta::wire from \"std\"\n",
            ),
            source(
                "main.veln",
                concat!(
                    "mod app\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode alpha::wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        let selected = query_snapshot(&exact, "main.veln", 4, 23)
            .expect("an exact standard-library import must beat an implicit collision");
        assert_eq!(locations(&selected.references), [("main.veln", 4, 23)]);
    }

    #[test]
    fn standard_library_schema_alias_eligibility_matrix_rejects_target_boundaries() {
        assert_standard_alias_declaration_boundaries();
        assert_ambiguous_standard_alias_is_empty();
        assert_standard_alias_target_boundaries();
    }

    fn assert_standard_alias_declaration_boundaries() {
        let cases = [
            (
                "private alias",
                "pub schema Packet\n  value: Int\nend\nschema Alias = Packet\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "unresolved target",
                "pub schema Alias = Missing\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "wrong-kind target",
                "pub type Wrong\n  Ready(Int)\nend\npub schema Alias = Wrong\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "private target",
                "schema Hidden\n  value: Int\nend\npub schema Alias = Hidden\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "cycle",
                "pub schema Alias = Other\npub schema Other = Alias\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "non-exported source",
                "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n",
                [].as_slice(),
            ),
        ];

        for (name, body, exports) in cases {
            let main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Alias\n  repeated: Repeat(count, wire::Alias)\n  array: [wire::Alias; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Alias from view at byte_offset(0)?\n  encode wire::Alias from packet\nend\n";
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
                .with_standard_library(standard_library_snapshot(
                    &[("wire.veln", body)],
                    exports.iter().copied(),
                ));
            assert_standard_alias_path_empty(&snapshot, main, "wire::Alias", name);
        }
    }

    fn assert_ambiguous_standard_alias_is_empty() {
        let ambiguous_main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Alias\n  repeated: Repeat(count, wire::Alias)\n  array: [wire::Alias; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Alias from view at byte_offset(0)?\n  encode wire::Alias from packet\nend\n";
        let ambiguous = EffectiveProjectSnapshot::new(vec![source("main.veln", ambiguous_main)])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        assert_standard_alias_path_empty(
            &ambiguous,
            ambiguous_main,
            "wire::Alias",
            "ambiguous alias",
        );
    }

    fn assert_standard_alias_target_boundaries() {
        let boundary_cases = [
            (
                "invalid-cased target",
                vec![(
                    "wire.veln",
                    "pub schema packet\n  value: Int\nend\npub schema Alias = packet\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "blocked nonterminal hop",
                vec![(
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\nschema Mid = Packet\npub schema Alias = Mid\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "target in non-exported source",
                vec![
                    (
                        "wire.veln",
                        "use hidden from \"std\"\npub schema Alias = hidden::Packet\n",
                    ),
                    ("hidden.veln", "pub schema Packet\n  value: Int\nend\n"),
                ],
                vec!["wire.veln"],
            ),
            (
                "invalid-cased alias",
                vec![(
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\npub schema alias = Packet\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "external-package target",
                vec![(
                    "wire.veln",
                    "use external from \"example/external\"\npub schema Alias = external::Packet\n",
                )],
                vec!["wire.veln"],
            ),
        ];
        for (name, sources, exports) in boundary_cases {
            let alias_name = if name == "invalid-cased alias" {
                "alias"
            } else {
                "Alias"
            };
            let alias_path = format!("wire::{alias_name}");
            let main = format!(
                "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: {alias_path}\n  repeated: Repeat(count, {alias_path})\n  array: [{alias_path}; count]\nend\n\nfn read(view: ByteView, packet: {{value: Int}}) -> ()\n  decode {alias_path} from view at byte_offset(0)?\n  encode {alias_path} from packet\nend\n"
            );
            let snapshot = standard_alias_boundary_snapshot(name, &main, &sources, exports);
            assert_standard_alias_path_empty(&snapshot, &main, &alias_path, name);
        }
    }

    fn standard_alias_boundary_snapshot(
        name: &str,
        main: &str,
        sources: &[(&str, &str)],
        exports: Vec<&'static str>,
    ) -> EffectiveProjectSnapshot {
        let standard_library = standard_library_snapshot(sources, exports);
        if name != "external-package target" {
            return EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
                .with_standard_library(standard_library);
        }
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", main)],
            vec![dependency_snapshot(
                "example/external",
                &[("external.veln", "pub schema Packet\n  value: Int\nend\n")],
                ["external.veln"],
            )],
        )
        .with_standard_library(standard_library)
    }

    fn assert_standard_alias_path_empty(
        snapshot: &EffectiveProjectSnapshot,
        main: &str,
        alias_path: &str,
        name: &str,
    ) {
        for (line, text) in main.lines().enumerate() {
            let Some(column) = text.find(alias_path) else {
                continue;
            };
            let result = query_snapshot(snapshot, "main.veln", line + 1, column + 7);
            assert!(
                result
                    .as_ref()
                    .is_none_or(|selection| selection.references.is_empty()),
                "{name} must remain a successful empty selection at {}:{}: {result:#?}",
                line + 1,
                column + 7
            );
        }
    }

    #[test]
    fn standard_library_schema_invalid_casing_alias_blockers_and_recovered_leaves_are_empty() {
        let invalid_casing = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode wire::packet from view at byte_offset(0)?\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));
        let invalid = query_snapshot(&invalid_casing, "main.veln", 4, 16)
            .expect("invalid-cased standard schema should retain its definition");
        assert!(matches!(invalid.definition.source, NavigationSource::Package { .. }));
        assert!(invalid.references.is_empty());

        let alias_collision = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "use wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[
                (
                    "alias/wire.veln",
                    "mod wire\npub schema Base\n  value: Int\nend\npub schema Packet = Base\n",
                ),
                (
                    "fallback/wire.veln",
                    "mod wire\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["alias/wire.veln", "fallback/wire.veln"],
        ));
        assert!(query_snapshot(&alias_collision, "main.veln", 4, 16)
            .is_none_or(|result| result.references.is_empty()));

        let recovered_alias_collision = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "use wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[
                (
                    "wire/valid.veln",
                    "mod wire\npub schema Packet\n  value: Int\nend\n",
                ),
                ("wire/recovered.veln", "mod wire\npub schema Packet =\n"),
            ],
            ["wire/valid.veln", "wire/recovered.veln"],
        ));
        assert!(query_snapshot(&recovered_alias_collision, "main.veln", 4, 16)
            .is_none_or(|result| result.references.is_empty()));

        let recovered_leaves = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "schema Host\n",
                "  count: UInt8\n",
                "  direct: wire::Packet\n",
                "  bad_repeat: Repeat(, wire::Packet)\n",
                "  bad_array: [wire::Packet;]\n",
                "end\n\n",
                "fn broken(view: ByteView) -> ()\n",
                "  decode wire::Packet junk from view at byte_offset(0)?\n",
                "  encode wire::Packet junk from {value: 1}\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));
        let valid = query_snapshot(&recovered_leaves, "main.veln", 5, 20)
            .expect("the valid composition leaf should remain navigable");
        assert_eq!(locations(&valid.references), [("main.veln", 5, 17)]);
        for (line, column) in [(6, 31), (7, 25), (11, 16), (12, 16)] {
            if let Some(result) = query_snapshot(&recovered_leaves, "main.veln", line, column)
            {
                assert!(result.references.is_empty(), "recovered leaf at {line}:{column}");
            }
        }
    }
