    fn assert_standard_library_type_alias(result: &NavigationResult) {
        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
    }

    #[test]
    fn standard_library_type_alias_references_cover_type_roles_and_prelude_forms() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "use bytes\n\npub type ByteCount = bytes::ByteCount\n",
                ),
                (
                    "bytes.veln",
                    "pub type ByteCount\n  pub ByteCount(Int)\nend\n",
                ),
            ],
            ["prelude.veln", "bytes.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "pub type LocalCount = ByteCount\n\n",
                    "fn first(count: ByteCount) -> prelude::ByteCount\n",
                    "  ByteCount::ByteCount(1)\n",
                    "end\n\n",
                    "fn second(counts: Vec<ByteCount>) -> Vec<prelude::ByteCount>\n",
                    "  prelude::ByteCount::ByteCount(2)\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                "fn other(count: ByteCount) -> prelude::ByteCount\n  ByteCount::ByteCount(3)\nend\n",
            ),
        ])
        .with_standard_library(standard_library);

        let expected = [
            ("main.veln", 1, 23),
            ("main.veln", 3, 17),
            ("main.veln", 3, 40),
            ("main.veln", 4, 3),
            ("main.veln", 7, 23),
            ("main.veln", 7, 51),
            ("main.veln", 8, 12),
            ("other.veln", 1, 17),
            ("other.veln", 1, 40),
            ("other.veln", 2, 3),
        ];

        for (source_path, line, column) in [
            ("main.veln", 1, 24),
            ("main.veln", 3, 18),
            ("main.veln", 3, 41),
            ("main.veln", 7, 24),
            ("main.veln", 7, 52),
            ("other.veln", 1, 18),
            ("other.veln", 1, 41),
        ] {
            let result = query_snapshot(&snapshot, source_path, line, column)
                .unwrap_or_else(|| panic!("did not select alias at {source_path}:{line}:{column}"));
            assert_standard_library_type_alias(&result);
            assert_eq!(locations(&result.references), expected);
        }

    }

    #[test]
    fn standard_library_type_alias_import_alias_and_written_module_path_share_identity() {
        let standard_library = standard_library_snapshot(
            &[
                ("facade.veln", "use core\n\npub type Alias = core::Target\n"),
                (
                    "core.veln",
                    "pub type Target\n  pub Ready(Int)\nend\n",
                ),
            ],
            ["facade.veln", "core.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use facade from \"std\"\n\n",
                "fn first(input: facade::Alias) -> facade::Alias\n",
                "  facade::Alias::Ready(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        {
            let (name, column) = ("import alias parameter", 25);
            let result = query_snapshot(&snapshot, "main.veln", 3, column)
                .unwrap_or_else(|| panic!("{name} did not select the standard alias"));
            assert_standard_library_type_alias(&result);
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 3, 25),
                    ("main.veln", 3, 43),
                    ("main.veln", 4, 11),
                ],
                "{name}"
            );
        }
    }

    #[test]
    fn standard_library_type_alias_references_keep_same_spelled_alias_and_target_separate() {
        let standard_library = standard_library_snapshot(
            &[
                ("prelude.veln", "use core\n\npub type Same = core::Same\n"),
                (
                    "core.veln",
                    "pub type Same\n  pub Ready(Int)\nend\n",
                ),
            ],
            ["prelude.veln", "core.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use core from \"std\"\n",
                    "use model from \"example/pkg\"\n\n",
                    "type Same\n",
                    "  Ready(Int)\n",
                    "end\n\n",
                    "pub type LocalAlias = prelude::Same\n\n",
                    "fn alias(input: prelude::Same) -> prelude::Same\n",
                    "  prelude::Same::Ready(1)\n",
                    "end\n\n",
                    "fn target(input: core::Same) -> core::Same\n",
                    "  core::Same::Ready(1)\n",
                    "end\n\n",
                    "fn collisions(input: model::Same, same_value: Int, record: {Same: Int}) -> Same\n",
                    "  \"Same\"\n",
                    "end\n\n",
                    "# Same in a comment is lexical noise.\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/pkg",
                &[("model.veln", "pub type Same\nend\n")],
                ["model.veln"],
            )],
        )
        .with_standard_library(standard_library);

        let alias = query_snapshot(&snapshot, "main.veln", 10, 26).unwrap();
        assert_standard_library_type_alias(&alias);
        assert_eq!(
            locations(&alias.references),
            [
                ("main.veln", 8, 32),
                ("main.veln", 10, 26),
                ("main.veln", 10, 44),
                ("main.veln", 11, 12),
            ]
        );

        let target = query_snapshot(&snapshot, "main.veln", 14, 24).unwrap();
        assert_eq!(target.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(
            target.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(target.definition.span.file.as_str(), "core.veln");
        assert_eq!(
            locations(&target.references),
            [
                ("main.veln", 14, 24),
                ("main.veln", 14, 39),
                ("main.veln", 15, 9),
            ]
        );
    }

    #[test]
    fn standard_library_type_alias_references_keep_different_spelled_alias_and_target_separate() {
        let standard_library = standard_library_snapshot(
            &[
                ("prelude.veln", "use core\n\npub type Count = core::Target\n"),
                (
                    "core.veln",
                    "pub type Target\n  pub Ready(Int)\nend\n",
                ),
            ],
            ["prelude.veln", "core.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use core from \"std\"\n\n",
                "pub type LocalAlias = Count\n\n",
                "fn alias(input: Count) -> prelude::Count\n",
                "  Count::Ready(1)\n",
                "end\n\n",
                "fn target(input: core::Target) -> core::Target\n",
                "  core::Target::Ready(1)\n",
                "end\n\n",
                "fn noise(record: {Count: Int}, count_value: Int, target_value: Int) -> Int\n",
                "  count_value + target_value\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let alias = query_snapshot(&snapshot, "main.veln", 5, 17).unwrap();
        assert_standard_library_type_alias(&alias);
        assert_eq!(
            locations(&alias.references),
            [
                ("main.veln", 3, 23),
                ("main.veln", 5, 17),
                ("main.veln", 5, 36),
                ("main.veln", 6, 3),
            ]
        );

        let target = query_snapshot(&snapshot, "main.veln", 9, 24).unwrap();
        assert_eq!(target.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(
            target.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(target.definition.span.file.as_str(), "core.veln");
        assert_eq!(
            locations(&target.references),
            [
                ("main.veln", 9, 24),
                ("main.veln", 9, 41),
                ("main.veln", 10, 9),
            ]
        );
    }

    #[test]
    fn standard_library_type_alias_references_keep_unsupported_targets_empty() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub type Target\n",
                    "end\n\n",
                    "pub fn value() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub type Good = Target\n",
                    "pub type MissingAlias = Missing\n",
                    "pub type WrongKind = value\n",
                    "pub type Chain = Good\n",
                ),
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "fn read(good: Good, missing: MissingAlias, wrong: WrongKind, chain: Chain) -> Int\n",
                "  0\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let supported = query_snapshot(&snapshot, "main.veln", 1, 15).unwrap();
        assert_standard_library_type_alias(&supported);
        assert_eq!(locations(&supported.references), [("main.veln", 1, 15)]);

        for (name, column) in [
            ("unresolved target", 30),
            ("wrong-kind target", 51),
            ("alias-chain target", 69),
        ] {
            let result = query_snapshot(&snapshot, "main.veln", 1, column)
                .unwrap_or_else(|| panic!("{name} should still select its alias identity"));
            assert_standard_library_type_alias(&result);
            assert!(
                result.references.is_empty(),
                "{name} unexpectedly returned references: {:?}",
                locations(&result.references)
            );
        }

        for (name, column) in [
            ("supported alias definition", 15),
            ("unsupported alias definition", 30),
        ] {
            assert_eq!(
                definition_at(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line: 1,
                        column,
                    },
                ),
                None,
                "{name}"
            );
        }
    }
