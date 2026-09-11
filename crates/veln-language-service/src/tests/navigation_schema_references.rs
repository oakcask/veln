mod navigation_schema_references_tests {
    use super::*;

    #[test]
    fn workspace_schema_references_cover_local_imported_and_qualified_operations() {
        let result = query(
            vec![
                source(
                    "main.veln",
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
                        "use main\n\n",
                        "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let qualified = decode main::Packet from view at byte_offset(0)?\n",
                        "  let bare = encode Packet from packet\n",
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
        assert_location(&result.definition, "main.veln", 1, 12);
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 7, 24),
                ("main.veln", 8, 24),
                ("other.veln", 4, 32),
                ("other.veln", 5, 21),
            ]
        );
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
                source("main.veln", "schema PrivatePacket\n  value: Int\nend\n"),
                source(
                    "main.test.veln",
                    concat!(
                        "use main\n\n",
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
                ("main.test.veln", 4, 30),
                ("main.test.veln", 5, 30),
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
        assert_eq!(locations(&result.references), [("main.veln", 18, 24)]);
        assert!(query(sources, "main.veln", 18, 18).is_none());
    }
}
