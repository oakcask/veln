mod navigation_schema_references_tests {
    use super::*;

    #[test]
    fn workspace_schema_references_cover_local_and_qualified_imported_operations() {
        let result = query(
            vec![
                source(
                    "app/wire.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "fn local(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let decoded = decode Packet from view at byte_offset(0)?\n",
                        "  let encoded = encode Packet from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use app::wire\n\n",
                        "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let qualified = decode app::wire::Packet from view at byte_offset(0)?\n",
                        "  let alias_qualified = encode wire::Packet from packet\n",
                        "  let bare_decode = decode Packet from view at byte_offset(0)?\n",
                        "  let bare_encode = encode Packet from packet\n",
                        "end\n",
                    ),
                ),
            ],
            "app/wire.veln",
            1,
            12,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_location(&result.definition, "app/wire.veln", 1, 12);
        assert_eq!(
            locations(&result.references),
            [
                ("app/wire.veln", 7, 24),
                ("app/wire.veln", 8, 24),
                ("other.veln", 4, 37),
                ("other.veln", 5, 38),
            ]
        );

        let bare_decode = query(
            vec![
                source(
                    "main.veln",
                    "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
                ),
                source(
                    "other.veln",
                    "use main\n\nfn read(view: ByteView) -> ()\n  decode Packet from view at byte_offset(0)?\nend\n",
                ),
            ],
            "other.veln",
            4,
            10,
        );
        assert!(bare_decode.is_none());

        let bare_encode = query(
            vec![
                source(
                    "main.veln",
                    "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
                ),
                source(
                    "other.veln",
                    "use main\n\nfn write(packet: {value: Int}) -> ()\n  encode Packet from packet\nend\n",
                ),
            ],
            "other.veln",
            4,
            10,
        );
        assert!(bare_encode.is_none());
    }

    #[test]
    fn workspace_schema_references_cover_direct_and_repeated_composition_targets() {
        let sources = vec![
            source(
                "app/wire.veln",
                concat!(
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n\n",
                    "schema LocalFrame\n",
                    "  format binary\n",
                    "  count: UInt8\n",
                    "  direct: Packet\n",
                    "  repeated: Repeat(count, Packet)\n",
                    "  canonical: [Packet; count]\n",
                    "end\n\n",
                    "pub schema Collision\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n\n",
                    "pub type Collision\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "use app::wire\n\n",
                    "schema ImportedFrame\n",
                    "  format binary\n",
                    "  count: UInt8\n",
                    "  qualified: app::wire::Packet\n",
                    "  alias_qualified: wire::Packet\n",
                    "  repeated: Repeat(count, app::wire::Packet)\n",
                    "  canonical: [wire::Packet; count]\n",
                    "  bare: Packet\n",
                    "  unresolved: missing::Packet\n",
                    "  collision: wire::Collision\n",
                    "end\n\n",
                    "fn lexical_noise() -> String\n",
                    "  # app::wire::Packet wire::Packet Packet\n",
                    "  \"app::wire::Packet wire::Packet Packet\"\n",
                    "end\n",
                ),
            ),
        ];

        let result = query(sources.clone(), "app/wire.veln", 1, 12).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(
            locations(&result.references),
            [
                ("app/wire.veln", 9, 11),
                ("app/wire.veln", 10, 27),
                ("app/wire.veln", 11, 15),
                ("other.veln", 6, 25),
                ("other.veln", 7, 26),
                ("other.veln", 8, 38),
                ("other.veln", 9, 21),
            ]
        );

        let selected_target = query(sources, "other.veln", 8, 38).unwrap();
        assert_eq!(selected_target.definition.span, result.definition.span);
        assert_eq!(selected_target.references, result.references);

        let collision = query(
            vec![
                source(
                    "app/wire.veln",
                    concat!(
                        "pub schema Collision\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "pub type Collision\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use app::wire\n\n",
                        "schema Host\n",
                        "  format binary\n",
                        "  collision: wire::Collision\n",
                        "end\n",
                    ),
                ),
            ],
            "app/wire.veln",
            1,
            12,
        )
        .unwrap();
        assert!(collision.references.is_empty());
    }

    #[test]
    fn workspace_schema_composition_uses_file_scoped_schema_identity() {
        let direct = query(
            vec![
                source(
                    "target.veln",
                    "use helper\n\npub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "host.veln",
                    "use target\n\nschema Host\n  direct: target::Packet\nend\n",
                ),
            ],
            "target.veln",
            3,
            12,
        )
        .unwrap();
        assert_eq!(locations(&direct.references), [("host.veln", 4, 19)]);

        let repeated = query(
            vec![
                source(
                    "noise.veln",
                    concat!(
                        "use first\n",
                        "use second\n",
                        "use third\n\n",
                        "schema Noise\n",
                        "  value: Int\n",
                        "end\n",
                    ),
                ),
                source(
                    "wire.veln",
                    concat!(
                        "schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "schema Host\n",
                        "  format binary\n",
                        "  count: UInt8\n",
                        "  repeated: [Packet; count]\n",
                        "end\n",
                    ),
                ),
            ],
            "wire.veln",
            1,
            8,
        )
        .unwrap();
        assert_eq!(locations(&repeated.references), [("wire.veln", 9, 14)]);
    }

    #[test]
    fn package_composition_does_not_bind_same_named_workspace_schema() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "model.veln",
                    "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
                ),
                source(
                    "main.veln",
                    concat!(
                        "use model from \"example/pkg\"\n\n",
                        "schema Host\n",
                        "  format binary\n",
                        "  count: UInt8\n",
                        "  nested: model::Packet\n",
                        "  repeated: [model::Packet; count]\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency_snapshot(
                "example/pkg",
                &[(
                    "model.veln",
                    "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
                )],
                ["model.veln"],
            )],
        );

        let result = query_snapshot(&snapshot, "model.veln", 1, 12).unwrap();
        assert!(result.references.is_empty());
        assert!(query_snapshot(&snapshot, "main.veln", 5, 18).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 6, 22).is_none());
    }

    #[test]
    fn dependency_composition_with_matching_source_identity_stays_isolated() {
        let workspace_source = concat!(
            "# pad\n",
            "pub schema Packet\n",
            "  parent: Nois\n",
            "end\n\n",
            "schema Nois\n",
            "  nested: Packet\n",
            "end\n",
        );
        let dependency_source = concat!(
            "use x\n",
            "pub schema Packet\n",
            "  parent: Int \n",
            "end\n\n",
            "schema Host\n",
            "  nested: Packet\n",
            "end\n",
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("model.veln", workspace_source)],
            vec![dependency_snapshot(
                "example/pkg",
                &[("model.veln", dependency_source)],
                ["model.veln"],
            )],
        );

        let result = query_snapshot(&snapshot, "model.veln", 2, 12).unwrap();
        assert!(result.references.is_empty());
        assert!(query_snapshot(&snapshot, "model.veln", 7, 11).is_none());
    }

    #[test]
    fn colliding_implicit_schema_import_aliases_are_order_independent() {
        for imports in [
            "use a::wire\nuse b::wire\n",
            "use b::wire\nuse a::wire\n",
        ] {
            let sources = vec![
                source(
                    "a/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "b/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "main.veln",
                    &format!(
                        "{imports}\nschema Host\n  ambiguous: wire::Packet\n  first: a::wire::Packet\n  second: b::wire::Packet\nend\n"
                    ),
                ),
            ];

            let first = query(sources.clone(), "a/wire.veln", 1, 12).unwrap();
            let second = query(sources.clone(), "b/wire.veln", 1, 12).unwrap();
            assert_eq!(locations(&first.references), [("main.veln", 6, 19)]);
            assert_eq!(locations(&second.references), [("main.veln", 7, 20)]);
            assert!(query(sources, "main.veln", 5, 20).is_none());
        }
    }

    #[test]
    fn exact_schema_import_path_precedes_colliding_implicit_leaf_alias() {
        for imports in [
            "use wire\nuse a::wire\n",
            "use a::wire\nuse wire\n",
        ] {
            let sources = vec![
                source(
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "a/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "main.veln",
                    &format!(
                        "{imports}\nschema Host\n  exact: wire::Packet\n  qualified: a::wire::Packet\nend\n"
                    ),
                ),
            ];

            let exact = query(sources.clone(), "wire.veln", 1, 12).unwrap();
            let qualified = query(sources.clone(), "a/wire.veln", 1, 12).unwrap();
            assert_eq!(locations(&exact.references), [("main.veln", 5, 16)]);
            assert_eq!(locations(&qualified.references), [("main.veln", 6, 23)]);

            let selected_exact = query(sources, "main.veln", 5, 16).unwrap();
            assert_eq!(selected_exact.definition.span.file.as_str(), "wire.veln");
        }
    }

    #[test]
    fn exact_workspace_import_precedes_package_implicit_leaf_alias() {
        for imports in [
            "use wire\nuse a::wire from \"example/pkg\"\n",
            "use a::wire from \"example/pkg\"\nuse wire\n",
        ] {
            let workspace_source = "pub schema Packet\n  value: Int\nend\n";
            let dependency_source = "pub schema Packet\n  value: String\nend\n";
            let consumer_source =
                format!("{imports}\nschema Host\n  nested: wire::Packet\nend\n");
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![
                    source("wire.veln", workspace_source),
                    source("main.veln", &consumer_source),
                ],
                vec![dependency_snapshot(
                    "example/pkg",
                    &[("a/wire.veln", dependency_source)],
                    ["a/wire.veln"],
                )],
            );

            let result = query_snapshot(&snapshot, "wire.veln", 1, 12).unwrap();
            assert_eq!(locations(&result.references), [("main.veln", 5, 17)]);

            let selected = query_snapshot(&snapshot, "main.veln", 5, 17).unwrap();
            assert_eq!(selected.definition.span.file.as_str(), "wire.veln");
        }
    }

    #[test]
    fn workspace_schema_references_preserve_import_visibility_and_shadowing() {
        let result = query(
            vec![
                source(
                    "main.veln",
                    "pub schema Packet\n  value: Int\nend\n\nschema Hidden\n  value: Int\nend\n",
                ),
                source(
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "schema Packet\n",
                        "  local: Int\n",
                        "end\n\n",
                        "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let local = decode Packet from view at byte_offset(0)?\n",
                        "  let selected = encode main::Packet from packet\n",
                        "  let hidden = decode main::Hidden from view at byte_offset(0)?\n",
                        "end\n\n",
                        "schema ShadowFrame\n",
                        "  direct: Packet\n",
                        "end\n",
                    ),
                ),
            ],
            "main.veln",
            1,
            12,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(
            locations(&result.references),
            [("other.veln", 9, 31)]
        );
        assert!(query(
            vec![
                source("main.veln", "schema Hidden\n  value: Int\nend\n"),
                source(
                    "other.veln",
                    "use main\n\nfn read(view: ByteView) -> ()\n  decode main::Hidden from view at byte_offset(0)?\nend\n",
                ),
            ],
            "other.veln",
            4,
            16,
        )
        .is_none());
    }

    #[test]
    fn workspace_schema_references_include_exact_companion_private_qualified_uses() {
        let result = query(
            vec![
                source(
                    "main.veln",
                    "schema PrivatePacket\n  format binary\n  value: UInt8\nend\n",
                ),
                source(
                    "main.test.veln",
                    concat!(
                        "use main\n\n",
                        "schema CompanionFrame\n",
                        "  format binary\n",
                        "  count: UInt8\n",
                        "  direct: main::PrivatePacket\n",
                        "  repeated: [main::PrivatePacket; count]\n",
                        "end\n\n",
                        "test companion(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let decoded = decode main::PrivatePacket from view at byte_offset(0)?\n",
                        "  let encoded = encode main::PrivatePacket from packet\n",
                        "  let bare = decode PrivatePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.test.veln",
                    concat!(
                        "use main\n\n",
                        "schema UnrelatedFrame\n",
                        "  format binary\n",
                        "  direct: main::PrivatePacket\n",
                        "end\n\n",
                        "test unrelated(view: ByteView) -> ()\n",
                        "  decode main::PrivatePacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            "main.veln",
            1,
            8,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(
            locations(&result.references),
            [
                ("main.test.veln", 6, 17),
                ("main.test.veln", 7, 20),
                ("main.test.veln", 11, 30),
                ("main.test.veln", 12, 30),
            ]
        );
        assert!(query(
            vec![
                source("main.veln", "schema PrivatePacket\n  value: Int\nend\n"),
                source(
                    "main.test.veln",
                    "use main\n\ntest companion(view: ByteView) -> ()\n  decode PrivatePacket from view at byte_offset(0)?\nend\n",
                ),
            ],
            "main.test.veln",
            4,
            10,
        )
        .is_none());
    }

    #[test]
    fn workspace_schema_references_exclude_collisions_and_unsupported_selections() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "schema packet\n",
                "  value: Int\n",
                "end\n\n",
                "fn packet() -> Int\n",
                "  1\n",
                "end\n\n",
                "type packet\n",
                "end\n\n",
                "type Holder\n",
                "  packet\n",
                "end\n\n",
                "effect packet\n",
                "  packet() -> Int\n",
                "end\n\n",
                "fn read(packet: {value: Int}) -> () effects [packet]\n",
                "  let packet = packet()\n",
                "  let encoded = encode packet from packet\n",
                "  let field = packet.value\n",
                "  # packet in a comment\n",
                "  \"packet\"\n",
                "end\n",
            ),
        )];
        let result = query(sources.clone(), "main.veln", 1, 8).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(locations(&result.references), [("main.veln", 22, 24)]);
        assert!(query(sources, "main.veln", 22, 18).is_none());
    }

    #[test]
    fn workspace_schema_alias_references_keep_alias_identity_across_operations_and_composition() {
        let sources = vec![
            source(
                "core.veln",
                concat!(
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n",
                ),
            ),
            source(
                "aliases.veln",
                concat!(
                    "use core\n\n",
                    "pub schema WirePacket = core::Packet\n",
                    "pub schema OtherPacket = core::Packet\n\n",
                    "schema Frame\n",
                    "  format binary\n",
                    "  count: UInt8\n",
                    "  direct: WirePacket\n",
                    "  repeated: Repeat(count, WirePacket)\n",
                    "  canonical: [WirePacket; count]\n",
                    "end\n\n",
                    "fn local(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  let decoded = decode WirePacket from view at byte_offset(0)?\n",
                    "  let encoded = encode WirePacket from packet\n",
                    "end\n",
                ),
            ),
            source(
                "main.veln",
                concat!(
                    "use aliases\n\n",
                    "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  let decoded = decode aliases::WirePacket from view at byte_offset(0)?\n",
                    "  let encoded = encode aliases::WirePacket from packet\n",
                    "end\n",
                ),
            ),
            source(
                "other_alias.veln",
                concat!(
                    "use core\n\n",
                    "pub schema WirePacket = core::Packet\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode WirePacket from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ),
        ];

        let alias = query(sources.clone(), "aliases.veln", 3, 12).unwrap();
        assert_eq!(alias.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(
            alias.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            locations(&alias.references),
            [
                ("aliases.veln", 9, 11),
                ("aliases.veln", 10, 27),
                ("aliases.veln", 11, 15),
                ("aliases.veln", 15, 24),
                ("aliases.veln", 16, 24),
                ("main.veln", 4, 33),
                ("main.veln", 5, 33),
            ]
        );

        for (file, line, column) in [
            ("aliases.veln", 9, 11),
            ("aliases.veln", 10, 27),
            ("aliases.veln", 11, 15),
            ("aliases.veln", 15, 24),
            ("aliases.veln", 16, 24),
            ("main.veln", 4, 33),
            ("main.veln", 5, 33),
        ] {
            let selected = query(sources.clone(), file, line, column).unwrap();
            assert_eq!(selected.definition, alias.definition);
            assert_eq!(selected.references, alias.references);
        }

        let other_alias = query(sources.clone(), "aliases.veln", 4, 12).unwrap();
        assert!(other_alias.references.is_empty());

        let same_spelling = query(sources.clone(), "other_alias.veln", 3, 12).unwrap();
        assert_eq!(
            locations(&same_spelling.references),
            [("other_alias.veln", 6, 10)]
        );

        let target = query(sources, "core.veln", 1, 12).unwrap();
        assert!(target.references.is_empty());
    }

    #[test]
    fn workspace_schema_alias_references_reject_alias_chains() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "pub schema Packet\n",
                "  format binary\n",
                "  value: UInt8\n",
                "end\n\n",
                "pub schema First = Packet\n",
                "pub schema Chained = First\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode Chained from view at byte_offset(0)?\n",
                "end\n",
            ),
        )];

        assert!(query(sources.clone(), "main.veln", 7, 12).is_none());
        assert!(query(sources, "main.veln", 10, 10).is_none());
    }

    #[test]
    fn workspace_schema_alias_references_require_a_direct_public_schema_target() {
        let cases = [
            (
                "private",
                "schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
                5,
            ),
            ("missing", "pub schema Alias = Missing\n", 1),
            (
                "wrong kind",
                "pub type Packet\nend\n\npub schema Alias = Packet\n",
                4,
            ),
            (
                "alias chain",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema First = Packet\n",
                    "pub schema Alias = First\n",
                ),
                6,
            ),
            (
                "cycle",
                "pub schema First = Second\npub schema Second = First\n",
                1,
            ),
            (
                "invalid casing",
                "pub schema Packet\n  value: Int\nend\n\npub schema alias = Packet\n",
                5,
            ),
        ];

        for (name, text, line) in cases {
            assert!(
                query(vec![source("main.veln", text)], "main.veln", line, 12).is_none(),
                "{name} alias must remain unsupported"
            );
        }
    }

    #[test]
    fn workspace_schema_alias_references_reject_ambiguous_targets() {
        for imports in [
            "use a::wire\nuse b::wire\n",
            "use b::wire\nuse a::wire\n",
        ] {
            let sources = vec![
                source(
                    "a/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "b/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "main.veln",
                    &format!("{imports}\npub schema Alias = wire::Packet\n"),
                ),
            ];

            assert!(query(sources, "main.veln", 4, 12).is_none());
        }
    }

    #[test]
    fn workspace_schema_alias_references_exclude_same_spelled_non_alias_symbols() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "pub schema Packet\n",
                    "  value: Int\n",
                    "end\n\n",
                    "pub schema WirePacket = Packet\n\n",
                    "schema Frame\n",
                    "  field: WirePacket\n",
                    "end\n\n",
                    "fn field_noise(record: {WirePacket: Int}) -> Int\n",
                    "  record.WirePacket\n",
                    "end\n",
                ),
            ),
            source(
                "noise.veln",
                concat!(
                    "type WirePacket\n",
                    "end\n\n",
                    "fn noise(WirePacket: Int) -> String\n",
                    "  let WirePacket = WirePacket\n",
                    "  # WirePacket in a comment\n",
                    "  \"WirePacket\"\n",
                    "end\n",
                ),
            ),
            source(
                "shadow.veln",
                concat!(
                    "use main\n\n",
                    "schema WirePacket\n",
                    "  value: Int\n",
                    "end\n\n",
                    "schema Frame\n",
                    "  shadowed: WirePacket\n",
                    "  selected: main::WirePacket\n",
                    "end\n",
                ),
            ),
        ];

        let result = query(sources, "main.veln", 5, 12).unwrap();
        assert_eq!(
            locations(&result.references),
            [("main.veln", 8, 10), ("shadow.veln", 9, 19)]
        );
    }
}
