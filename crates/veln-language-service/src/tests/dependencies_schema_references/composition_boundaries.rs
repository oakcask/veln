    #[test]
    fn dependency_schema_alias_composition_rejects_external_targets_and_recovered_leaves() {
        let external_target = dependency_snapshot(
            "example/external-target",
            &[
                (
                    "alias.veln",
                    concat!(
                        "mod dep\n\n",
                        "use other from \"other/dep\"\n\n",
                        "pub schema Alias = other::Packet\n",
                    ),
                ),
                (
                    "other.veln",
                    "mod other\n\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["alias.veln", "other.veln"],
        );
        let valid_alias = dependency_snapshot(
            "example/valid",
            &[(
                "dep.veln",
                "mod dep\n\npub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
            )],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "external.veln",
                    concat!(
                        "use dep from \"example/external-target\"\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: dep::Alias\n",
                        "  repeated: Repeat(count, dep::Alias)\n",
                        "  array: [dep::Alias; count]\n",
                        "end\n",
                    ),
                ),
                source(
                    "recovered-import.veln",
                    concat!(
                        "use dep from \"example/valid\" unexpected\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: dep::Alias\n",
                        "  repeated: Repeat(count, dep::Alias)\n",
                        "  array: [dep::Alias; count]\n",
                        "end\n",
                    ),
                ),
            ],
            vec![external_target, valid_alias],
        );

        for path in ["external.veln", "recovered-import.veln"] {
            for (line, column) in [(5, 16), (6, 32), (7, 20)] {
                assert!(
                    query_snapshot(&snapshot, path, line, column).is_none(),
                    "{path}:{line}:{column} must not select a dependency schema alias",
                );
            }
        }
    }

    #[test]
    fn dependency_schema_composition_rejects_casing_mismatch_and_transitive_inputs() {
        let selected = dependency_snapshot(
            "example/dep",
            &[(
                "public.veln",
                concat!(
                    "mod public\n\n",
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema badPacket\n  value: Int\nend\n",
                ),
            )],
            ["public.veln"],
        );
        let mismatch = dependency_snapshot(
            "other/dep",
            &[("other.veln", "mod other\n\npub schema Packet\n  value: Int\nend\n")],
            ["other.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use public from \"example/dep\"\n",
                    "use mismatch from \"other/dep\"\n",
                    "use transitive from \"transitive/dep\"\n\n",
                    "schema Host\n",
                    "  invalid_casing: public::badPacket\n",
                    "  mismatched_import: mismatch::Packet\n",
                    "  transitive_schema: transitive::Packet\n",
                    "end\n",
                ),
            )],
            vec![selected, mismatch],
        );

        for (line, column) in [(6, 28), (7, 33), (8, 35)] {
            assert!(query_snapshot(&snapshot, "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn dependency_schema_composition_import_collisions_are_order_independent() {
        let first = dependency_snapshot(
            "first/dep",
            &[
                ("shared.veln", "mod shared\n\npub schema Packet\n  value: Int\nend\n"),
                ("alpha/wire.veln", "mod alpha::wire\n\npub schema Packet\n  value: Int\nend\n"),
            ],
            ["shared.veln", "alpha/wire.veln"],
        );
        let second = dependency_snapshot(
            "second/dep",
            &[
                ("shared.veln", "mod shared\n\npub schema Packet\n  value: Int\nend\n"),
                ("beta/wire.veln", "mod beta::wire\n\npub schema Packet\n  value: Int\nend\n"),
            ],
            ["shared.veln", "beta/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "exact_a.veln",
                    concat!(
                        "use shared from \"first/dep\"\n",
                        "use shared from \"second/dep\"\n\n",
                        "schema Host\n  nested: shared::Packet\nend\n",
                    ),
                ),
                source(
                    "exact_b.veln",
                    concat!(
                        "use shared from \"second/dep\"\n",
                        "use shared from \"first/dep\"\n\n",
                        "schema Host\n  nested: shared::Packet\nend\n",
                    ),
                ),
                source(
                    "implicit_a.veln",
                    concat!(
                        "use alpha::wire from \"first/dep\"\n",
                        "use beta::wire from \"second/dep\"\n\n",
                        "schema Host\n  nested: wire::Packet\nend\n",
                    ),
                ),
                source(
                    "implicit_b.veln",
                    concat!(
                        "use beta::wire from \"second/dep\"\n",
                        "use alpha::wire from \"first/dep\"\n\n",
                        "schema Host\n  nested: wire::Packet\nend\n",
                    ),
                ),
            ],
            vec![first, second],
        );

        for path in [
            "exact_a.veln",
            "exact_b.veln",
            "implicit_a.veln",
            "implicit_b.veln",
        ] {
            assert!(query_snapshot(&snapshot, path, 5, 20).is_none(), "{path}");
        }
    }

    #[test]
    fn conflicting_exact_dependency_imports_do_not_fall_back_to_an_implicit_alias() {
        let first = dependency_snapshot(
            "first/dep",
            &[
                (
                    "shared.veln",
                    "mod shared\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "fallback/shared.veln",
                    "mod fallback::shared\n\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["shared.veln", "fallback/shared.veln"],
        );
        let second = dependency_snapshot(
            "second/dep",
            &[(
                "shared.veln",
                "mod shared\n\npub schema Packet\n  value: Int\nend\n",
            )],
            ["shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "first_order.veln",
                    concat!(
                        "use fallback::shared from \"first/dep\"\n",
                        "use shared from \"first/dep\"\n",
                        "use shared from \"second/dep\"\n\n",
                        "schema Host\n  nested: shared::Packet\nend\n",
                    ),
                ),
                source(
                    "second_order.veln",
                    concat!(
                        "use shared from \"second/dep\"\n",
                        "use shared from \"first/dep\"\n",
                        "use fallback::shared from \"first/dep\"\n\n",
                        "schema Host\n  nested: shared::Packet\nend\n",
                    ),
                ),
            ],
            vec![first, second],
        );

        for path in ["first_order.veln", "second_order.veln"] {
            assert!(query_snapshot(&snapshot, path, 6, 20).is_none(), "{path}");
        }
    }

    #[test]
    fn dependency_schema_composition_reference_sets_exclude_lexical_noise() {
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
                    "  nested: dep::Packet\n",
                    "end\n\n",
                    "fn noise() -> String\n",
                    "  // dep::Packet\n",
                    "  \"dep::Packet\"\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let selected = query_snapshot(&snapshot, "main.veln", 4, 17).unwrap();
        assert_eq!(locations(&selected.references), [("main.veln", 4, 16)]);
        for (line, column) in [(1, 5), (4, 11), (7, 10), (8, 9)] {
            assert!(query_snapshot(&snapshot, "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn standard_library_schema_composition_is_a_project_reference() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "schema Host\n  nested: wire::Packet\nend\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));

        let selected = query_snapshot(&snapshot, "main.veln", 4, 19).unwrap();
        assert_eq!(locations(&selected.references), [("main.veln", 4, 17)]);
        assert!(matches!(
            selected.definition.source,
            NavigationSource::Package { .. }
        ));
    }

    #[test]
    fn standard_library_prelude_schema_alias_is_visible_by_bare_name() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "schema Host\n",
                "  nested: AliasPacket\n",
                "end\n\n",
                "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                "  decode AliasPacket from view at byte_offset(0)?\n",
                "  encode AliasPacket from packet\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
            )],
            ["prelude.veln"],
        ));

        for (line, column) in [(2, 12), (6, 10), (7, 10)] {
            let selected = query_snapshot(&snapshot, "main.veln", line, column)
                .unwrap_or_else(|| panic!("missing bare prelude alias at {line}:{column}"));
            assert_eq!(selected.selected_symbol.name, "AliasPacket");
            assert_eq!(selected.selected_symbol.package_origin, Some(PackageOrigin::StandardLibrary));
            assert_eq!(
                locations(&selected.references),
                [("main.veln", 2, 11), ("main.veln", 6, 10), ("main.veln", 7, 10)]
            );
        }
    }

    #[test]
    fn standard_library_prelude_schema_alias_is_visible_by_qualified_name() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "schema Host\n",
                "  nested: prelude::AliasPacket\n",
                "end\n\n",
                "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                "  decode prelude::AliasPacket from view at byte_offset(0)?\n",
                "  encode prelude::AliasPacket from packet\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
            )],
            ["prelude.veln"],
        ));

        for (line, column) in [(2, 20), (6, 19), (7, 19)] {
            let selected = query_snapshot(&snapshot, "main.veln", line, column)
                .unwrap_or_else(|| panic!("missing qualified prelude alias at {line}:{column}"));
            assert_eq!(selected.selected_symbol.name, "AliasPacket");
            assert_eq!(
                locations(&selected.references),
                [("main.veln", 2, 20), ("main.veln", 6, 19), ("main.veln", 7, 19)]
            );
        }
    }
