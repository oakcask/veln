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
                    ("main.veln", 8, 17),
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
    fn direct_dependency_schema_operations_do_not_index_the_standard_library() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "wire.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            )],
            ["wire.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[("prelude.veln", "pub fn identity(value: Int) -> Int\n  value\nend\n")],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use wire from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        reset_dependency_source_indexes();
        let result = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Schema);
        assert_eq!(locations(&result.references), [("main.veln", 4, 16)]);
        assert_eq!(
            dependency_source_indexes(),
            1,
            "direct dependency schema operations do not need standard-library symbols",
        );
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
    fn direct_dependency_schema_references_unify_composition_and_operation_leaves() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                "pub schema Packet\n  format binary\n  value: UInt8\nend\n",
            )],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Frame\n",
                        "  format binary\n",
                        "  count: UInt8\n",
                        "  direct: lib::wire::Packet\n",
                        "  repeated: Repeat(count, wire::Packet)\n",
                        "  canonical: [wire::Packet; count]\n",
                        "end\n\n",
                        "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  let decoded = decode wire::Packet from view at byte_offset(0)?\n",
                        "  encode lib::wire::Packet from packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Other\n",
                        "  format binary\n",
                        "  nested: wire::Packet\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );
        let expected = [
            ("main.veln", 6, 22),
            ("main.veln", 7, 33),
            ("main.veln", 8, 21),
            ("main.veln", 12, 30),
            ("main.veln", 13, 21),
            ("other.veln", 5, 17),
        ];
        for (path, line, column) in expected {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert!(matches!(result.definition.source, NavigationSource::Package { .. }));
            assert_eq!(locations(&result.references), expected);
        }
    }

    #[test]
    fn malformed_dependency_schema_repeats_do_not_select_or_enter_reference_sets() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "dep.veln",
                "mod dep\n\npub schema Packet\n  value: Int\nend\n",
            )],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "schema Host\n",
                    "  count: UInt8\n",
                    "  direct: dep::Packet\n",
                    "  missing_call_count: Repeat(, dep::Packet)\n",
                    "  missing_array_count: [dep::Packet;]\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let selected = query_snapshot(&snapshot, "main.veln", 5, 18).unwrap();
        assert_eq!(locations(&selected.references), [("main.veln", 5, 16)]);
        assert!(query_snapshot(&snapshot, "main.veln", 6, 43).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 7, 37).is_none());
    }

    #[test]
    fn dependency_schema_composition_respects_import_identity_boundaries() {
        let selected = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "schema Private\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let hidden = dependency_snapshot(
            "example/hidden",
            &[("hidden.veln", "pub schema Packet\n  value: Int\nend\n")],
            [],
        );
        let collision = dependency_snapshot(
            "other/dep",
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "lib/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
                source(
                    "ambiguous_exact.veln",
                    concat!(
                        "use lib::wire\n",
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Host\n  nested: lib::wire::Packet\nend\n",
                    ),
                ),
                source(
                    "ambiguous_alias.veln",
                    concat!(
                        "use app::wire\n",
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Host\n  nested: wire::Packet\nend\n",
                    ),
                ),
                source(
                    "duplicate.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n",
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Host\n  nested: lib::wire::Packet\nend\n",
                    ),
                ),
                source(
                    "boundaries.veln",
                    concat!(
                        "use lib::wire from \"example/dep\"\n",
                        "use hidden from \"example/hidden\"\n\n",
                        "schema Host\n",
                        "  private: wire::Private\n",
                        "  alias: wire::Alias\n",
                        "  hidden: hidden::Packet\n",
                        "  bare: Packet\n",
                        "  unresolved: missing::Packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "exact.veln",
                    concat!(
                        "use wire from \"other/dep\"\n",
                        "use lib::wire from \"example/dep\"\n\n",
                        "schema Host\n  nested: wire::Packet\nend\n",
                    ),
                ),
            ],
            vec![selected, hidden, collision],
        );

        for (path, line, column) in [
            ("ambiguous_exact.veln", 5, 28),
            ("ambiguous_alias.veln", 5, 19),
            ("duplicate.veln", 5, 28),
            ("boundaries.veln", 5, 18),
            ("boundaries.veln", 7, 19),
            ("boundaries.veln", 8, 9),
            ("boundaries.veln", 9, 24),
        ] {
            assert!(query_snapshot(&snapshot, path, line, column).is_none());
        }

        let alias = query_snapshot(&snapshot, "boundaries.veln", 6, 16).unwrap();
        assert_eq!(locations(&alias.references), [("boundaries.veln", 6, 16)]);

        let exact = query_snapshot(&snapshot, "exact.veln", 5, 19).unwrap();
        assert_eq!(exact.selected_symbol.name, "Packet");
        assert_eq!(
            exact.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(locations(&exact.references), [("exact.veln", 5, 17)]);
        let NavigationSource::Package { .. } = exact.definition.source else {
            panic!("exact dependency import must select a package declaration");
        };
        assert_eq!(exact.definition.span.file.as_str(), "wire.veln");
    }

    #[test]
    fn dependency_schema_collisions_block_composition_and_aliases_by_package_identity() {
        let valid = concat!(
            "mod dep\n\n",
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Alias = Packet\n",
        );
        for (name, blocker) in [
            ("private target", "mod dep\n\nschema Packet\n  value: Int\nend\n"),
            ("recovered target", "mod dep\n\npub schema Packet\n  value: Int\n"),
        ] {
            for blocker_first in [false, true] {
                let blocker_path = if blocker_first { "before.veln" } else { "zz_after.veln" };
                let sources = [("valid.veln", valid), (blocker_path, blocker)];
                let blocked = dependency_snapshot(
                    "example/blocked",
                    &sources,
                    ["valid.veln"],
                );
                let clean = dependency_snapshot(
                    "example/clean",
                    &[("valid.veln", valid)],
                    ["valid.veln"],
                );
                let consumer = |package| format!(concat!(
                    "use dep from \"{}\"\n\n",
                    "schema Host\n",
                    "  nested: dep::Packet\n",
                    "end\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ), package);
                let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![
                        source("blocked.veln", &consumer("example/blocked")),
                        source("clean.veln", &consumer("example/clean")),
                    ],
                    vec![blocked, clean],
                );

                for (line, column, symbol) in [(4, 16, "Packet"), (8, 15, "Alias")] {
                    assert!(
                        query_snapshot(&snapshot, "blocked.veln", line, column).is_none(),
                        "{name} must block {symbol}, blocker_first={blocker_first}",
                    );
                    let result = query_snapshot(&snapshot, "clean.veln", line, column)
                        .expect("a collision in another package must not block selection");
                    assert_eq!(result.selected_symbol.name, symbol);
                    assert_eq!(locations(&result.references), [("clean.veln", line, column)]);
                }
            }
        }
    }

    #[test]
    fn dependency_schema_aliases_with_same_module_and_name_keep_package_identity() {
        let alias_source = concat!(
            "mod shared\n\n",
            "pub schema Packet\n  value: Int\nend\n\n",
            "pub schema Mid = Packet\n",
            "pub schema Alias = Mid\n",
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "first-import.veln",
                    concat!(
                        "mod app\n\n",
                        "use shared from \"first/dep\"\n",
                    ),
                ),
                source(
                    "first.veln",
                    concat!(
                        "mod app\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode shared::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "second-import.veln",
                    concat!(
                        "mod other\n\n",
                        "use shared from \"second/dep\"\n",
                    ),
                ),
                source(
                    "second.veln",
                    concat!(
                        "mod other\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode shared::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![
                dependency_snapshot("first/dep", &[("wire.veln", alias_source)], ["wire.veln"]),
                dependency_snapshot(
                    "second/dep",
                    &[("wire.veln", alias_source)],
                    ["wire.veln"],
                ),
            ],
        );

        let first = query_snapshot(&snapshot, "first.veln", 4, 18).unwrap();
        assert_eq!(first.selected_symbol.name, "Alias");
        assert_eq!(locations(&first.references), [("first.veln", 4, 18)]);
        let second = query_snapshot(&snapshot, "second.veln", 4, 18).unwrap();
        assert_eq!(second.selected_symbol.name, "Alias");
        assert_eq!(locations(&second.references), [("second.veln", 4, 18)]);
    }

    #[test]
    fn recovered_dependency_schema_declaration_blocks_composition_identity() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "valid.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Clean\n  value: Int\nend\n",
                    ),
                ),
                (
                    "recovered.veln",
                    "mod dep\n\npub schema Packet\n  recovered: Int\n",
                ),
            ],
            ["valid.veln", "recovered.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "schema Host\n",
                        "  nested: dep::Packet\n",
                        "end\n",
                    ),
                ),
                source(
                    "recovery.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "schema Host\n",
                        "  nested: dep::Clean unexpected\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
        assert!(query_snapshot(&snapshot, "recovery.veln", 4, 16).is_none());
    }

    #[test]
    fn dependency_schema_alias_declarations_block_composition_schema_fallback() {
        for (name, alias_source) in [
            ("clean alias", "mod dep\n\npub schema Alias = Packet\n"),
            ("recovered alias", "mod dep\n\npub schema Alias =\n"),
            (
                "alias chain",
                concat!(
                    "mod dep\n\n",
                    "pub schema Alias = Middle\n",
                    "pub schema Middle = Packet\n",
                ),
            ),
        ] {
            let dependency = dependency_snapshot(
                "example/dep",
                &[
                    (
                        "schemas.veln",
                        concat!(
                            "mod dep\n\n",
                            "pub schema Alias\n  value: Int\nend\n\n",
                            "pub schema Packet\n  value: Int\nend\n",
                        ),
                    ),
                    ("alias.veln", alias_source),
                ],
                ["schemas.veln", "alias.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: dep::Alias\n",
                        "  repeated: Repeat(count, dep::Alias)\n",
                        "  canonical: [dep::Alias; count]\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );

            for (line, column) in [(5, 16), (6, 32), (7, 20)] {
                assert!(
                    query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                    "{name} must block composition schema fallback",
                );
            }
        }
    }

    #[test]
    fn local_schema_alias_blocker_precedes_same_named_schema_for_operations() {
        let main = concat!(
            "schema Alias\n",
            "  value: Int\n",
            "end\n",
            "pub schema Alias = Missing\n",
            "\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Alias from view at byte_offset(0)?\n",
            "  encode Alias from packet\n",
            "end\n",
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", main)]);

        for (line, text) in main.lines().enumerate() {
            if let Some(column) = text.find("Alias")
                && (text.contains("decode Alias") || text.contains("encode Alias"))
            {
                let result = query_snapshot(&snapshot, "main.veln", line + 1, column + 1);
                assert!(
                    result.as_ref().is_none_or(|selection| selection.references.is_empty()),
                    "ineligible local alias must block same-named schema fallback at {}:{}: {result:#?}",
                    line + 1,
                    column + 1
                );
            }
        }
    }
