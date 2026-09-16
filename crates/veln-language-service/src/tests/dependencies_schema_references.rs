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
        let local = query_snapshot(&snapshot, "main.veln", 16, 22).unwrap();
        assert_eq!(locations(&local.references), [("main.veln", 16, 22)]);
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
        assert!(query_snapshot(&snapshot, "main.veln", 12, 18).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 13, 18).is_none());
        assert!(query_snapshot(&snapshot, "mismatch.veln", 4, 18).is_none());
        assert!(query_snapshot(&snapshot, "transitive.veln", 4, 18).is_none());
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
