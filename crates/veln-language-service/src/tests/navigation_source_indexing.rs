mod navigation_source_indexing_tests {
    use super::*;

    #[test]
    fn mixed_workspace_indexes_keep_effect_handler_and_dependency_schema_navigation() {
        let dependency = dependency_snapshot(
            "example/wire",
            &[(
                "wire.veln",
                "pub schema Packet\n  value: Int\nend\n",
            )],
            ["wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use wire from \"example/wire\"\n\n",
                    "effect Fetch\n",
                    "  get() -> Int\n",
                    "end\n\n",
                    "handler fetch() for Fetch\n",
                    "  get() => perform Fetch::get()\n",
                    "end\n\n",
                    "schema Frame\n",
                    "  payload: wire::Packet\n",
                    "end\n\n",
                    "fn decode_packet(view: ByteView) -> ()\n",
                    "  decode wire::Packet from view at byte_offset(0)?\n",
                    "end\n\n",
                    "fn handled() -> Int effects [Fetch]\n",
                    "  handle 1 with fetch()\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let effect = query_snapshot(&snapshot, "main.veln", 3, 8).unwrap();
        assert_eq!(effect.selected_symbol.kind, SymbolKind::Effect);
        assert_location(&effect.definition, "main.veln", 3, 8);

        let handler = query_snapshot(&snapshot, "main.veln", 20, 17).unwrap();
        assert_eq!(handler.selected_symbol.kind, SymbolKind::Handler);
        assert_location(&handler.definition, "main.veln", 7, 9);

        for (line, column) in [(12, 18), (16, 16)] {
            let schema = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_eq!(schema.selected_symbol.kind, SymbolKind::Schema);
            assert!(matches!(
                schema.definition.source,
                NavigationSource::Package { .. }
            ));
        }
    }
}
