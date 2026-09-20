    #[test]
    fn dependency_schema_composition_imports_are_shared_by_explicit_module_identity() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "import.veln",
                    "mod app\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "host.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Host\n",
                        "  nested: wire::Packet\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency_snapshot(
                "example/dep",
                &[("lib/wire.veln", "pub schema Packet\n  value: Int\nend\n")],
                ["lib/wire.veln"],
            )],
        );

        let result = query_snapshot(&snapshot, "host.veln", 4, 19).unwrap();
        assert_eq!(locations(&result.references), [("host.veln", 4, 17)]);
        assert!(matches!(result.definition.source, NavigationSource::Package { .. }));
    }

    #[test]
    fn dependency_schema_imports_unify_all_leaf_roles_across_explicit_module_sources() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "import.veln",
                    "mod app\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "composition.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: lib::wire::Packet\n",
                        "  repeated: Repeat(count, wire::Packet)\n",
                        "  canonical: [wire::Packet; count]\n",
                        "end\n",
                    ),
                ),
                source(
                    "decode.veln",
                    concat!(
                        "mod app\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "encode.veln",
                    concat!(
                        "mod app\n\n",
                        "fn write(packet: {value: Int}) -> ()\n",
                        "  encode lib::wire::Packet from packet\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency_snapshot(
                "example/dep",
                &[("lib/wire.veln", "pub schema Packet\n  value: Int\nend\n")],
                ["lib/wire.veln"],
            )],
        );
        let expected = [
            ("composition.veln", 5, 22),
            ("composition.veln", 6, 33),
            ("composition.veln", 7, 21),
            ("decode.veln", 4, 16),
            ("encode.veln", 4, 21),
        ];

        for (path, line, column) in expected {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(locations(&result.references), expected, "{path}:{line}");
        }
    }

    #[test]
    fn invalid_module_wide_dependency_schema_imports_block_composition_and_operations() {
        let cases = [
            (
                "duplicate import",
                vec![
                    ("a.veln", "mod app\nuse dep from \"first/dep\"\n"),
                    ("b.veln", "mod app\nuse dep from \"first/dep\"\n"),
                ],
                "dep",
                "Alias",
                16,
                15,
            ),
            (
                "recovered import",
                vec![(
                    "a.veln",
                    "mod app\nuse dep from \"first/dep\" unexpected\n",
                )],
                "dep",
                "Alias",
                16,
                15,
            ),
            (
                "conflicting exact imports",
                vec![
                    ("a.veln", "mod app\nuse shared from \"first/dep\"\n"),
                    ("b.veln", "mod app\nuse shared from \"second/dep\"\n"),
                    (
                        "c.veln",
                        "mod app\nuse fallback::shared from \"first/dep\"\n",
                    ),
                ],
                "shared",
                "Packet",
                19,
                18,
            ),
            (
                "ambiguous implicit alias",
                vec![
                    (
                        "a.veln",
                        "mod app\nuse alpha::wire from \"first/dep\"\n",
                    ),
                    (
                        "b.veln",
                        "mod app\nuse beta::wire from \"second/dep\"\n",
                    ),
                ],
                "wire",
                "Alias",
                17,
                16,
            ),
        ];

        for (name, imports, qualifier, target, composition_column, operation_column) in cases {
            for reverse in [false, true] {
                let mut sources = imports
                    .iter()
                    .map(|(path, text)| source(path, text))
                    .collect::<Vec<_>>();
                sources.push(source(
                    "selection.veln",
                    &format!(
                        concat!(
                            "mod app\n",
                            "schema Host\n",
                            "  nested: {0}::{1}\n",
                            "end\n",
                            "fn read(view: ByteView) -> ()\n",
                            "  decode {0}::{1} from view at byte_offset(0)?\n",
                            "end\n",
                        ),
                        qualifier,
                        target,
                    ),
                ));
                if reverse {
                    sources.reverse();
                }
                let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                    sources,
                    vec![
                        dependency_snapshot(
                            "first/dep",
                            &[
                                (
                                    "dep.veln",
                                    "mod dep\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                                (
                                    "shared.veln",
                                    "mod shared\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                                (
                                    "fallback/shared.veln",
                                    "mod fallback::shared\npub schema Packet\n  value: Int\nend\n",
                                ),
                                (
                                    "alpha/wire.veln",
                                    "mod alpha::wire\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                            ],
                            [
                                "dep.veln",
                                "shared.veln",
                                "fallback/shared.veln",
                                "alpha/wire.veln",
                            ],
                        ),
                        dependency_snapshot(
                            "second/dep",
                            &[
                                (
                                    "shared.veln",
                                    "mod shared\npub schema Packet\n  value: Int\nend\n",
                                ),
                                (
                                    "beta/wire.veln",
                                    "mod beta::wire\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                            ],
                            ["shared.veln", "beta/wire.veln"],
                        ),
                    ],
                );

                assert!(
                    query_snapshot(
                        &snapshot,
                        "selection.veln",
                        3,
                        composition_column,
                    )
                    .is_none(),
                    "{name}, reverse={reverse}: composition",
                );
                assert!(
                    query_snapshot(
                        &snapshot,
                        "selection.veln",
                        6,
                        operation_column,
                    )
                    .is_none(),
                    "{name}, reverse={reverse}: operation",
                );
            }
        }
    }

    #[test]
    fn exact_workspace_qualifier_precedes_dependency_schema_alias_implicit_alias() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "other/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            )],
            ["other/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source("wire.veln", "pub schema Alias\n  value: Int\nend\n"),
                source(
                    "main.veln",
                    concat!(
                        "use wire\n",
                        "use other::wire from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        let selected = query_snapshot(&snapshot, "main.veln", 5, 16).unwrap();
        assert_eq!(
            selected.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert!(matches!(
            selected.definition.source,
            NavigationSource::Workspace
        ));
        assert_eq!(selected.definition.span.file.as_str(), "wire.veln");
        assert_eq!(locations(&selected.references), [("main.veln", 5, 16)]);
    }

    #[test]
    fn exact_dependency_schema_alias_import_precedes_colliding_implicit_alias() {
        let selected = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema WirePacket = Packet\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let collision = dependency_snapshot(
            "other/dep",
            &[("other/lib.veln", "pub schema Other\n  value: Int\nend\n")],
            ["other/lib.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use lib::wire from \"example/dep\"\n",
                    "use other::lib from \"other/dep\"\n\n",
                    "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode lib::wire::WirePacket from view at byte_offset(0)?\n",
                    "  encode wire::WirePacket from packet\n",
                    "end\n",
                ),
            )],
            vec![selected, collision],
        );

        let selected = query_snapshot(&snapshot, "main.veln", 5, 27).unwrap();
        assert_eq!(
            selected.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            locations(&selected.references),
            [("main.veln", 5, 21), ("main.veln", 6, 16)]
        );
    }

    #[test]
    fn direct_dependency_schema_alias_references_keep_alias_identity() {
        let selected = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  format binary\n  value: UInt8\nend\n\n",
                    "pub type Packet\n  pub Ready(Int)\nend\n\n",
                    "pub schema WirePacket = Packet\n",
                    "pub schema OtherPacket = Packet\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let collision = dependency_snapshot(
            "other/dep",
            &[(
                "other/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema WirePacket = Packet\n",
                ),
            )],
            ["other/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  decode lib::wire::WirePacket from view at byte_offset(0)?\n",
                        "  encode wire::WirePacket from packet\n",
                        "  encode wire::OtherPacket from packet\n",
                        "  encode wire::Packet from packet\n",
                        "  encode WirePacket from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::WirePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "collision.veln",
                    concat!(
                        "use other::wire from \"other/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::WirePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "workspace.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema WirePacket = Packet\n",
                    ),
                ),
                source(
                    "boundaries.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "type WirePacket\n  Local(Int)\nend\n\n",
                        "schema Frame\n",
                        "  nested: wire::WirePacket\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        for (path, line, column) in [
            ("main.veln", 4, 22),
            ("main.veln", 5, 16),
            ("other.veln", 4, 16),
        ] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(result.selected_symbol.declaration_kind, SymbolDeclarationKind::PublicAlias);
            assert_eq!(
                locations(&result.references),
                [
                    ("boundaries.veln", 8, 17),
                    ("main.veln", 4, 21),
                    ("main.veln", 5, 16),
                    ("other.veln", 4, 16),
                ]
            );
        }

        let sibling_alias = query_snapshot(&snapshot, "main.veln", 6, 16).unwrap();
        assert_eq!(locations(&sibling_alias.references), [("main.veln", 6, 16)]);
        let target = query_snapshot(&snapshot, "main.veln", 7, 16).unwrap();
        assert_eq!(locations(&target.references), [("main.veln", 7, 16)]);
        assert!(query_snapshot(&snapshot, "main.veln", 8, 10).is_none());
        let collision = query_snapshot(&snapshot, "collision.veln", 4, 16).unwrap();
        assert_eq!(locations(&collision.references), [("collision.veln", 4, 16)]);
        let workspace = query_snapshot(&snapshot, "workspace.veln", 5, 12).unwrap();
        assert!(workspace.references.is_empty());
        let boundary = query_snapshot(&snapshot, "boundaries.veln", 8, 18).unwrap();
        assert_eq!(
            locations(&boundary.references),
            [
                ("boundaries.veln", 8, 17),
                ("main.veln", 4, 21),
                ("main.veln", 5, 16),
                ("other.veln", 4, 16),
            ]
        );
    }

    #[test]
    fn direct_dependency_schema_alias_target_resolves_across_same_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "lib/packet.veln",
                    concat!(
                        "mod lib::wire\n\n",
                        "pub schema Packet\n  value: Int\nend\n",
                    ),
                ),
                (
                    "lib/alias.veln",
                    "mod lib::wire\n\npub schema WirePacket = Packet\n",
                ),
            ],
            ["lib/packet.veln", "lib/alias.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use lib::wire from \"example/dep\"\n\n",
                    "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode wire::WirePacket from view at byte_offset(0)?\n",
                    "  encode wire::WirePacket from packet\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(4, 16), (5, 16)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("main.veln", 4, 16), ("main.veln", 5, 16)]
            );
        }
    }

    #[test]
    fn direct_dependency_schema_alias_references_keep_declaration_identity_across_modules() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "alpha.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                    ),
                ),
                (
                    "beta.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                    ),
                ),
            ],
            ["alpha.veln", "beta.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "read.veln",
                    concat!(
                        "use alpha from \"example/dep\"\n",
                        "use beta from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode alpha::Alias from view at byte_offset(0)?\n",
                        "  decode beta::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "write.veln",
                    concat!(
                        "use alpha from \"example/dep\"\n",
                        "use beta from \"example/dep\"\n\n",
                        "fn write(packet: {value: Int}) -> ()\n",
                        "  encode alpha::Alias from packet\n",
                        "  encode beta::Alias from packet\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (path, line, column) in [("read.veln", 5, 17), ("write.veln", 5, 17)] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("read.veln", 5, 17), ("write.veln", 5, 17)]
            );
        }

        for (path, line, column) in [("read.veln", 6, 16), ("write.veln", 6, 16)] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("read.veln", 6, 16), ("write.veln", 6, 16)]
            );
        }
    }

    #[test]
    fn direct_dependency_schema_alias_imports_are_visible_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Mid = Packet\n",
                    "pub schema Alias = Mid\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "imports.veln",
                    "mod app\n\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "read.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "write.veln",
                    concat!(
                        "mod app\n\n",
                        "fn write(packet: {value: Int}) -> ()\n",
                        "  encode wire::Alias from packet\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (path, column) in [("read.veln", 16), ("write.veln", 16)] {
            let result = query_snapshot(&snapshot, path, 4, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [
                    ("read.veln", 4, 16),
                    ("read.veln", 8, 16),
                    ("write.veln", 4, 16),
                ]
            );
        }
    }
