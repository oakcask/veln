mod dependencies_schema_references_tests {
    use super::*;

    #[test]
    fn direct_dependency_schema_references_cover_operation_leaves_and_identity() {
        let selected = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n\n",
                    "schema PackageFrame\n",
                    "  nested: Packet\n",
                    "end\n\n",
                    "fn package_operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode Packet from view at byte_offset(0)?\n",
                    "  encode Packet from packet\n",
                    "end\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let collision = dependency_snapshot(
            "other/dep",
            &[("lib/wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n",
                        "\n",
                        "schema Packet\n",
                        "  value: Int\n",
                        "end\n\n",
                        "schema Frame\n",
                        "  nested: wire::Packet\n",
                        "end\n\n",
                        "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  # Packet and wire::Packet are not operation leaves here.\n",
                        "  \"Packet wire::Packet\"\n",
                        "  let full = decode lib::wire::Packet from view at byte_offset(0)?\n",
                        "  let alias = encode wire::Packet from packet\n",
                        "  let local = encode Packet from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "aliases.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "pub schema Packet = wire::Packet\n",
                    ),
                ),
                source("symbols.veln", "type Packet\n  Ready(Int)\nend\n"),
                source(
                    "other.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "collision.veln",
                    concat!(
                        "use lib::wire from \"other/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "ambiguous.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n",
                        "use other::wire from \"other/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::WirePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        for (path, line, column) in [
            ("main.veln", 14, 32),
            ("main.veln", 15, 28),
            ("other.veln", 4, 16),
        ] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
            assert!(matches!(result.definition.source, NavigationSource::Package { .. }));
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 14, 32),
                    ("main.veln", 15, 28),
                    ("other.veln", 4, 16),
                ]
            );
        }

        let collision = query_snapshot(&snapshot, "collision.veln", 4, 16).unwrap();
        assert_eq!(locations(&collision.references), [("collision.veln", 4, 16)]);
        assert!(query_snapshot(&snapshot, "ambiguous.veln", 5, 16).is_none());
        let local = query_snapshot(&snapshot, "main.veln", 16, 22).unwrap();
        assert_eq!(locations(&local.references), [("main.veln", 16, 22)]);
    }

    #[test]
    fn exact_dependency_schema_qualifier_precedes_workspace_implicit_alias() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "a/wire.veln",
                    concat!(
                        "pub schema Local\n  value: Int\nend\n\n",
                        "pub schema Packet = Local\n",
                    ),
                ),
                source(
                    "main.veln",
                    concat!(
                        "use a::wire\n",
                        "use wire from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        let selected = query_snapshot(&snapshot, "main.veln", 5, 16).unwrap();
        assert_eq!(selected.selected_symbol.kind, SymbolKind::Schema);
        assert!(matches!(
            selected.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(locations(&selected.references), [("main.veln", 5, 16)]);
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
        assert!(query_snapshot(&snapshot, "boundaries.veln", 8, 18).is_none());
    }

    #[test]
    fn package_schema_references_require_public_exported_direct_dependencies() {
        let direct = dependency_snapshot(
            "example/dep",
            &[
                (
                    "public.veln",
                    concat!(
                        "pub schema Public\n  value: Int\nend\n\n",
                        "pub schema badSchema\n  value: Int\nend\n\n",
                        "pub schema Alias = Public\n",
                    ),
                ),
                ("private.veln", "schema Private\n  value: Int\nend\n"),
                ("hidden.veln", "pub schema Hidden\n  value: Int\nend\n"),
            ],
            ["public.veln", "private.veln"],
        );
        let standard = standard_library_snapshot(
            &[("wire.veln", "pub schema Standard\n  value: Int\nend\n")],
            ["wire.veln"],
        );
        let mismatched = dependency_snapshot(
            "other/dep",
            &[("other.veln", "pub schema Public\n  value: Int\nend\n")],
            ["other.veln"],
        );
        let bridge = dependency_snapshot(
            "bridge/dep",
            &[(
                "bridge.veln",
                concat!(
                    "use public from \"transitive/dep\"\n\n",
                    "pub fn consume(view: ByteView) -> ()\n",
                    "  decode public::Public from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            ["bridge.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                    "use public from \"example/dep\"\n",
                    "use private from \"example/dep\"\n",
                    "use hidden from \"example/dep\"\n",
                    "use wire from \"std\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode public::Public from view at byte_offset(0)?\n",
                    "  decode private::Private from view at byte_offset(0)?\n",
                    "  decode hidden::Hidden from view at byte_offset(0)?\n",
                    "  decode wire::Standard from view at byte_offset(0)?\n",
                    "  decode public::badSchema from view at byte_offset(0)?\n",
                        "  decode public::Alias from view at byte_offset(0)?\n",
                        "  decode public::Missing from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "mismatch.veln",
                    concat!(
                        "use public from \"other/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode public::Public from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "transitive.veln",
                    concat!(
                        "use public from \"transitive/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode public::Public from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![direct, mismatched, bridge],
        )
        .with_standard_library(standard);

        let public = query_snapshot(&snapshot, "main.veln", 7, 18).unwrap();
        assert_eq!(locations(&public.references), [("main.veln", 7, 18)]);
        assert!(query_snapshot(&snapshot, "main.veln", 8, 19).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 9, 18).is_none());
        let standard = query_snapshot(&snapshot, "main.veln", 10, 16).unwrap();
        assert!(standard.references.is_empty());
        let invalid_casing = query_snapshot(&snapshot, "main.veln", 11, 18);
        assert!(invalid_casing.is_none_or(|result| result.references.is_empty()));
        let alias = query_snapshot(&snapshot, "main.veln", 12, 18).unwrap();
        assert_eq!(locations(&alias.references), [("main.veln", 12, 18)]);
        assert!(query_snapshot(&snapshot, "main.veln", 13, 18).is_none());
        assert!(query_snapshot(&snapshot, "mismatch.veln", 4, 18).is_none());
        assert!(query_snapshot(&snapshot, "transitive.veln", 4, 18).is_none());
    }

    #[test]
    fn direct_dependency_schema_alias_references_require_a_unique_bare_public_target() {
        let cases = [
            (
                "private target",
                concat!(
                    "schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "missing target",
                "pub schema Alias = Missing\n",
            ),
            (
                "invalid-casing target",
                "pub schema Alias = badTarget\n",
            ),
            (
                "wrong kind target",
                "pub type Packet\nend\n\npub schema Alias = Packet\n",
            ),
            (
                "alias chain",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema First = Packet\n",
                    "pub schema Alias = First\n",
                ),
            ),
            (
                "qualified target",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = dep::Packet\n",
                ),
            ),
            (
                "duplicate target",
                concat!(
                    "pub schema Packet\n  left: Int\nend\n\n",
                    "pub schema Packet\n  right: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "duplicate alias",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "private schema collides with public alias",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "schema Alias\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "private schema collides with public target",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "schema Packet\n  hidden: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "alias cycle",
                "pub schema Alias = Other\npub schema Other = Alias\n",
            ),
            (
                "schema collision",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "target alias collision",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Other\n  value: Int\nend\n\n",
                    "pub schema Packet = Other\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "recovered alias declaration",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias =\n",
                ),
            ),
        ];

        for (name, dependency_source) in cases {
            let dependency = dependency_snapshot(
                "example/dep",
                &[("dep.veln", dependency_source)],
                ["dep.veln"],
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
                "{name} must not select an alias or fall back to another schema"
            );
        }

        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "dep.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema badAlias = Packet\n",
                ),
            )],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::badAlias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );
        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
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
                    "mod dep\n\npub schema Alias\n  value: Int\nend\n",
                ),
                (
                    "hidden.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
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
    fn valid_cross_module_dependency_schema_alias_target_stays_outside_reference_slice() {
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
}
