    #[test]
    fn ineligible_dependency_schema_alias_composition_leaves_stay_empty() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "lib/wire.veln",
                    concat!(
                        "mod lib::wire\n",
                        "use private\n\n",
                        "pub schema PrivateTarget = private::Packet\n",
                    ),
                ),
                (
                    "private.veln",
                    "mod private\n\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use lib::wire from \"example/dep\"\n\n",
                    "schema Frame\n",
                    "  count: UInt8\n",
                    "  direct: lib::wire::PrivateTarget\n",
                    "  repeated: Repeat(count, lib::wire::PrivateTarget)\n",
                    "  array: [lib::wire::PrivateTarget; count]\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(5, 24), (6, 40), (7, 23)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "ineligible alias composition leaf at {line}:{column} must not be selectable"
            );
        }
    }

    #[test]
    fn recovered_dependency_schema_alias_blocks_same_module_schema_fallback() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                ("alias.veln", "mod dep\n\npub schema Alias =\n"),
                (
                    "schema.veln",
                    "mod dep\n\npub schema Alias\n  value: Int\nend\n",
                ),
            ],
            ["alias.veln", "schema.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
    }

    #[test]
    fn recovered_dependency_schema_alias_blocks_same_named_valid_alias() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "valid.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                    ),
                ),
                ("recovered.veln", "mod dep\n\npub schema Alias =\n"),
            ],
            ["valid.veln", "recovered.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
    }

    #[test]
    fn recovered_dependency_schema_declarations_block_alias_eligibility() {
        for (name, valid_source, recovered_source) in [
            (
                "duplicate target",
                concat!(
                    "mod dep\n\n",
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
                "mod dep\n\npub schema Packet\n  recovered: Int\n",
            ),
            (
                "alias-name collision",
                concat!(
                    "mod dep\n\n",
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
                "mod dep\n\npub schema Alias\n  recovered: Int\n",
            ),
            (
                "sole recovered target",
                "mod dep\n\npub schema Alias = Packet\n",
                "mod dep\n\npub schema Packet\n  recovered: Int\n",
            ),
        ] {
            let dependency = dependency_snapshot(
                "example/dep",
                &[("valid.veln", valid_source), ("recovered.veln", recovered_source)],
                ["valid.veln", "recovered.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode dep::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );

            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 16).is_none(),
                "{name} must block dependency alias eligibility"
            );
        }
    }

    #[test]
    fn hidden_same_module_dependency_schema_aliases_block_alias_eligibility() {
        for (name, hidden_source) in [
            ("duplicate alias", "mod dep\n\npub schema Alias = Packet\n"),
            (
                "target-name alias",
                concat!(
                    "mod dep\n\n",
                    "pub schema Other\n  value: Int\nend\n\n",
                    "pub schema Packet = Other\n",
                ),
            ),
        ] {
            let dependency = dependency_snapshot(
                "example/dep",
                &[
                    (
                        "valid.veln",
                        concat!(
                            "mod dep\n\n",
                            "pub schema Packet\n  value: Int\nend\n\n",
                            "pub schema Alias = Packet\n",
                        ),
                    ),
                    ("hidden.veln", hidden_source),
                ],
                ["valid.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode dep::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );

            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 16).is_none(),
                "{name} must block dependency alias eligibility"
            );
        }
    }

    #[test]
    fn non_exported_dependency_schema_alias_blocks_exported_schema_fallback() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "exported.veln",
                    "mod dep\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "hidden.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Alias = Packet\n",
                    ),
                ),
            ],
            ["exported.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
    }

    #[test]
    fn dependency_schema_alias_references_require_valid_imports() {
        for (name, imports) in [
            (
                "duplicate import",
                concat!(
                    "use dep from \"example/dep\"\n",
                    "use dep from \"example/dep\"\n",
                ),
            ),
            (
                "recovered import",
                "use dep from \"example/dep\" unexpected\n",
            ),
        ] {
            let dependency = dependency_snapshot(
                "example/dep",
                &[
                    (
                        "alias.veln",
                        concat!(
                            "mod dep\n\n",
                            "pub schema Packet\n  value: Int\nend\n\n",
                            "pub schema Alias = Packet\n",
                        ),
                    ),
                    (
                        "schema.veln",
                        "mod dep\n\npub schema Alias\n  value: Int\nend\n",
                    ),
                ],
                ["alias.veln", "schema.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    &format!(
                        "{imports}\nfn read(view: ByteView) -> ()\n  decode dep::Alias from view at byte_offset(0)?\nend\n"
                    ),
                )],
                vec![dependency],
            );
            let operation_line = imports.lines().count() + 3;

            assert!(
                query_snapshot(&snapshot, "main.veln", operation_line, 16).is_none(),
                "{name} must preserve the alias blocker instead of falling back to the same-named schema"
            );
        }
    }

    #[test]
    fn direct_dependency_schema_alias_target_resolves_across_modules() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "core.veln",
                    "mod core\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "facade.veln",
                    concat!(
                        "mod facade\n",
                        "use core\n\n",
                        "pub schema Alias = core::Packet\n",
                    ),
                ),
            ],
            ["core.veln", "facade.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/dep\"\n",
                    "use unrelated::core\n\n",
                    "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode facade::Alias from view at byte_offset(0)?\n",
                    "  encode facade::Alias from packet\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (path, line, column) in [
            ("main.veln", 5, 19),
            ("main.veln", 6, 19),
        ] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("main.veln", 5, 18), ("main.veln", 6, 18)]
            );
        }
    }

    #[test]
    fn direct_dependency_schema_alias_target_imports_are_visible_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "core.veln",
                    "mod core\n\npub schema Packet\n  value: Int\nend\n",
                ),
                ("facade/imports.veln", "mod facade\nuse core\n"),
                (
                    "facade/alias.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Intermediate = core::Packet\n",
                        "pub schema Alias = Intermediate\n",
                    ),
                ),
            ],
            ["core.veln", "facade/alias.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/dep\"\n\n",
                    "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode facade::Alias from view at byte_offset(0)?\n",
                    "  encode facade::Alias from packet\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(4, 19), (5, 19)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("main.veln", 4, 18), ("main.veln", 5, 18)]
            );
        }
    }
