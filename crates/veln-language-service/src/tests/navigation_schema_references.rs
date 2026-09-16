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
    fn workspace_schema_references_keep_schema_specific_unsupported_selections_empty() {
        let sources = vec![
            source(
                "main.veln",
                concat!(
                    "pub schema Packet\n",
                    "  format binary\n",
                    "  value: UInt8\n",
                    "end\n\n",
                    "pub schema AliasPacket = Packet\n\n",
                    "schema Frame\n",
                    "  format binary\n",
                    "  nested: AliasPacket\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "use main\n\n",
                    "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  let decoded = decode main::Packet from view at byte_offset(0)?\n",
                    "  let encoded = encode main::Packet from packet\n",
                    "end\n",
                ),
            ),
        ];

        assert!(query(sources.clone(), "main.veln", 6, 12).is_none());

        assert!(query(sources.clone(), "main.veln", 10, 11).is_none());
        assert!(query(sources.clone(), "other.veln", 4, 25).is_none());
        assert!(query(sources, "other.veln", 5, 25).is_none());
    }
}
