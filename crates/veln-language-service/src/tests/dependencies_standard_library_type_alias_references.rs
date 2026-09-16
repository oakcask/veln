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
