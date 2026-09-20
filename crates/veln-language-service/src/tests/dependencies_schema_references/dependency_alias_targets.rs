    #[test]
    fn direct_dependency_schema_alias_qualified_target_resolution_matrix() {
        let cases = [
            (
                "implicit leaf",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                true,
            ),
            (
                "full written path",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = nested::core::Packet\n",
                true,
            ),
            (
                "qualified import back into alias module",
                "mod facade\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                concat!(
                    "mod facade\n",
                    "use facade\n\n",
                    "pub schema Alias = facade::Packet\n",
                ),
                true,
            ),
            (
                "private target",
                "mod nested::core\n\nschema Packet\n  value: Int\nend\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                false,
            ),
            (
                "missing target",
                "mod nested::core\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                false,
            ),
            (
                "wrong kind",
                "mod nested::core\n\npub type Packet\n  pub Ready(Int)\nend\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                false,
            ),
            (
                "invalid-cased target",
                "mod nested::core\n\npub schema badPacket\n  value: Int\nend\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::badPacket\n",
                false,
            ),
            (
                "recovered target",
                "mod nested::core\n\npub schema Packet\n  value: Int\n",
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                false,
            ),
            (
                "ambiguous target",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod nested::core\n\npub schema Packet\n  other: Int\nend\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Packet\n",
                false,
            ),
            (
                "alias chain",
                concat!(
                    "mod nested::core\n\n",
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Chained = Packet\n",
                ),
                "mod spare\n",
                "mod facade\nuse nested::core\n\npub schema Alias = core::Chained\n",
                true,
            ),
            (
                "ambiguous implicit import",
                "mod alpha::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod beta::core\n\npub schema Packet\n  value: Int\nend\n",
                concat!(
                    "mod facade\n",
                    "use alpha::core\n",
                    "use beta::core\n\n",
                    "pub schema Alias = core::Packet\n",
                ),
                false,
            ),
            (
                "duplicate target import",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                concat!(
                    "mod facade\n",
                    "use nested::core\n",
                    "use nested::core\n\n",
                    "pub schema Alias = core::Packet\n",
                ),
                false,
            ),
            (
                "invalid-cased target import",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                concat!(
                    "mod facade\n",
                    "use nested::Core\n\n",
                    "pub schema Alias = Core::Packet\n",
                ),
                false,
            ),
            (
                "recovered target import",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                concat!(
                    "mod facade\n",
                    "use nested::core unexpected\n\n",
                    "pub schema Alias = core::Packet\n",
                ),
                false,
            ),
            (
                "other package import",
                "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                "mod spare\n",
                concat!(
                    "mod facade\n",
                    "use nested::core from \"other/dep\"\n\n",
                    "pub schema Alias = core::Packet\n",
                ),
                false,
            ),
        ];

        for (name, core, other, facade, eligible) in cases {
            let dependency = dependency_snapshot(
                "example/dep",
                &[
                    ("core.veln", core),
                    ("other.veln", other),
                    ("facade.veln", facade),
                ],
                ["core.veln", "other.veln", "facade.veln"],
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

            let selected = query_snapshot(&snapshot, "main.veln", 5, 19);
            if eligible {
                let selected = selected.unwrap();
                assert_eq!(
                    selected.selected_symbol.declaration_kind,
                    SymbolDeclarationKind::PublicAlias,
                    "{name}",
                );
                assert_eq!(
                    locations(&selected.references),
                    [("main.veln", 5, 18), ("main.veln", 6, 18)],
                    "{name}",
                );
            } else {
                assert!(selected.is_none(), "{name}");
            }
        }
    }

    #[test]
    fn dependency_schema_alias_target_import_blockers_cross_source_boundaries() {
        for (name, import_sources) in [
            (
                "duplicate target import",
                vec![
                    ("import_a.veln", "mod facade\n\nuse nested::core\n"),
                    ("import_b.veln", "mod facade\n\nuse nested::core\n"),
                ],
            ),
            (
                "recovered target import",
                vec![
                    ("import_a.veln", "mod facade\n\nuse nested::core\n"),
                    (
                        "import_b.veln",
                        "mod facade\n\nuse nested::core unexpected\n",
                    ),
                ],
            ),
        ] {
            let mut dependency_sources = vec![
                (
                    "core.veln",
                    "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "facade.veln",
                    "mod facade\n\npub schema Mid = nested::core::Packet\npub schema Alias = Mid\n",
                ),
            ];
            dependency_sources.extend(import_sources);
            let dependency = dependency_snapshot(
                "example/dep",
                &dependency_sources,
                dependency_sources.iter().map(|(path, _)| *path),
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use facade from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode facade::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );

            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 19).is_none(),
                "{name} must block cross-module alias target resolution"
            );
        }
    }

    #[test]
    fn dependency_schema_alias_requires_an_exported_cross_module_target_source() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "core.veln",
                    "mod core\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "facade.veln",
                    "mod facade\nuse core\n\npub schema Alias = core::Packet\n",
                ),
            ],
            ["facade.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode facade::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 19).is_none());
    }

    #[test]
    fn cross_module_dependency_schema_alias_keeps_its_exact_identity() {
        let selected = dependency_snapshot(
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
                        "pub schema Sibling = core::Packet\n",
                    ),
                ),
            ],
            ["core.veln", "facade.veln"],
        );
        let other = dependency_snapshot(
            "other/dep",
            &[(
                "other.veln",
                "mod other\n\npub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
            )],
            ["other.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use facade from \"example/dep\"\n",
                        "use core from \"example/dep\"\n",
                        "use other from \"other/dep\"\n\n",
                        "schema Alias\n  value: Int\nend\n\n",
                        "fn operations(view: ByteView) -> ()\n",
                        "  decode facade::Alias from view at byte_offset(0)?\n",
                        "  decode facade::Sibling from view at byte_offset(0)?\n",
                        "  decode core::Packet from view at byte_offset(0)?\n",
                        "  decode other::Alias from view at byte_offset(0)?\n",
                        "  decode Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use facade from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode facade::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, other],
        );

        let result = query_snapshot(&snapshot, "main.veln", 10, 19).unwrap();
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            locations(&result.references),
            [("main.veln", 10, 18), ("other.veln", 4, 18)]
        );
    }

    #[test]
    fn dependency_schema_references_exclude_recovered_operation_leaves() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[("dep.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  decode dep::Packet from view at byte_offset(0)?\n",
                        "  encode dep::Packet from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "broken_decode.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode dep::Packet from view byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "broken_encode.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn write(packet: {value: Int}) -> ()\n",
                        "  encode dep::Packet junk from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "recovery.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(Packet: Int, view: ByteView) -> ()\n",
                        "  decode Packet from view at byte_offset(0)?\n",
                        "  decode dep::Packet junk from view at byte_offset(0)?\n",
                        "  decode dep::Packet from view byte_offset(0)?\n",
                        "  encode dep::Packet junk from {value: 1}\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        let valid = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
        assert_eq!(
            locations(&valid.references),
            [("main.veln", 4, 15), ("main.veln", 5, 15)]
        );
        for (path, line, column) in [
            ("broken_decode.veln", 4, 16),
            ("broken_encode.veln", 4, 16),
            ("recovery.veln", 5, 16),
            ("recovery.veln", 6, 16),
            ("recovery.veln", 7, 16),
        ] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert!(result.references.is_empty(), "{path}:{line}: {result:#?}");
            assert_eq!(
                definition_at(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new(path),
                        line,
                        column,
                    },
                ),
                Some(valid.definition.clone()),
                "{path}:{line}",
            );
        }
    }
