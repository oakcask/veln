mod dependencies_schema_references_tests {
    use super::*;

    #[test]
    fn dependency_schema_alias_resolution_avoids_nonlinear_declaration_scans() {
        // Count eligibility traversals instead of repeatedly timing the entire index build.
        // Keep instrumentation inside declaration visits when changing these traversals.
        for count in [100, 200, 400] {
            let snapshot = dependency_schema_alias_resolution_snapshot(count);
            crate::navigation::reset_schema_alias_declaration_visits();
            let _ = snapshot.navigation_index();
            assert_eq!(
                crate::navigation::schema_alias_declaration_visits(),
                count * 4,
                "eligibility must visit each resolved alias, alias declaration, target, and candidate once",
            );
            let result = query_snapshot(&snapshot, "main.veln", 4, 19).unwrap();
            assert_eq!(result.selected_symbol.name, "Alias0");
            assert_eq!(locations(&result.references), [("main.veln", 4, 18)]);
            let last = query_snapshot(&snapshot, "main.veln", 5, 19).unwrap();
            assert_eq!(last.selected_symbol.name, format!("Alias{}", count - 1));
            assert_eq!(locations(&last.references), [("main.veln", 5, 18)]);
        }
    }

    fn dependency_schema_alias_resolution_snapshot(count: usize) -> EffectiveProjectSnapshot {
        let mut targets = String::from("mod core\n\n");
        let mut aliases = String::from("mod facade\nuse core\n\n");
        for index in 0..count {
            targets.push_str(&format!(
                "pub schema Packet{index}\n  value: Int\nend\n\n"
            ));
            aliases.push_str(&format!(
                "pub schema Alias{index} = core::Packet{index}\n"
            ));
        }
        let dependency = dependency_snapshot(
            "example/dep",
            &[("core.veln", &targets), ("facade.veln", &aliases)],
            ["core.veln", "facade.veln"],
        );
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                &format!(concat!(
                    "use facade from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode facade::Alias0 from view at byte_offset(0)?\n",
                    "  decode facade::Alias{} from view at byte_offset(0)?\n",
                    "end\n",
                ), count - 1),
            )],
            vec![dependency],
        )
    }

    #[test]
    fn dependency_schema_alias_reference_lookup_avoids_nonlinear_import_rescans() {
        for count in [100, 200, 400] {
            let (index_entries, route_lookups) = dependency_schema_alias_reference_work(count);
            assert_eq!(
                index_entries,
                count * 3,
                "index construction must make only its three linear import passes",
            );
            assert_eq!(
                route_lookups,
                count + 1,
                "the first query must perform one indexed route lookup per candidate plus selection",
            );
        }
    }

    fn dependency_schema_alias_reference_work(count: usize) -> (usize, usize) {
        let mut consumer = String::from("use schema0 from \"example/dep\"\n");
        for index in 1..count {
            consumer.push_str(&format!("use unused{index} from \"example/dep\"\n"));
        }
        consumer.push_str("\nfn read(view: ByteView) -> ()\n");
        for _ in 0..count {
            consumer.push_str("  decode schema0::Alias from view at byte_offset(0)?\n");
        }
        consumer.push_str("end\n");
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "schema0.veln",
                "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
            )],
            ["schema0.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", &consumer)],
            vec![dependency],
        );
        reset_schema_alias_import_work();
        let _ = snapshot.navigation_index();
        let (index_entries, construction_lookups) = schema_alias_import_work();
        assert_eq!(construction_lookups, 0);

        reset_schema_alias_import_work();
        let result = query_snapshot(&snapshot, "main.veln", count + 3, 20).unwrap();
        assert_eq!(result.references.len(), count);
        let (query_index_entries, route_lookups) = schema_alias_import_work();
        assert_eq!(query_index_entries, 0);
        (index_entries, route_lookups)
    }

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
            ("boundaries.veln", 6, 16),
            ("boundaries.veln", 7, 19),
            ("boundaries.veln", 8, 9),
            ("boundaries.veln", 9, 24),
        ] {
            assert!(query_snapshot(&snapshot, path, line, column).is_none());
        }

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
    fn standard_library_schema_composition_is_not_a_dependency_reference() {
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

        assert!(query_snapshot(&snapshot, "main.veln", 4, 19).is_none());
    }

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
                    "pub schema Alias = Packet\n",
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
                [("read.veln", 4, 16), ("write.veln", 4, 16)]
            );
        }
    }

    #[test]
    fn workspace_and_dependency_schema_alias_imports_collide_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "workspace/wire.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                    ),
                ),
                source("workspace_import.veln", "mod app\n\nuse workspace::wire\n"),
                source(
                    "dependency_import.veln",
                    "mod app\n\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "operation.veln",
                    concat!(
                        "mod app\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        assert!(
            query_snapshot(&snapshot, "operation.veln", 4, 16).is_none(),
            "a workspace import and dependency import with the same implicit qualifier must be ambiguous across module sources"
        );
    }

    #[test]
    fn exact_workspace_and_dependency_schema_alias_imports_are_ambiguous() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "workspace/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                    "pub schema Fallback = Packet\n",
                ),
            )],
            ["workspace/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "workspace/wire.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                        "pub schema Fallback\n  value: Int\nend\n",
                    ),
                ),
                source(
                    "main.veln",
                    concat!(
                        "use workspace::wire\n",
                        "use workspace::wire from \"example/dep\"\n\n",
                        "fn operations(view: ByteView) -> ()\n",
                        "  decode workspace::wire::Alias from view at byte_offset(0)?\n",
                        "  decode workspace::wire::Fallback from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (line, column) in [(5, 27), (6, 27)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "an exact workspace and dependency import collision must block alias selection and schema fallback"
            );
        }
    }

    #[test]
    fn dependency_alias_and_schema_imports_collide_across_module_sources() {
        let alias_dependency = dependency_snapshot(
            "example/alias",
            &[(
                "a/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            )],
            ["a/wire.veln"],
        );
        let schema_dependency = dependency_snapshot(
            "example/schema",
            &[("b/wire.veln", "pub schema Alias\n  value: Int\nend\n")],
            ["b/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "alias_import.veln",
                    "mod app\n\nuse a::wire from \"example/alias\"\n",
                ),
                source(
                    "operation.veln",
                    concat!(
                        "mod app\n\n",
                        "use b::wire from \"example/schema\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![alias_dependency, schema_dependency],
        );

        assert!(
            query_snapshot(&snapshot, "operation.veln", 6, 16).is_none(),
            "module-wide alias ambiguity must block file-local schema fallback"
        );
    }

    #[test]
    fn invalid_dependency_schema_alias_imports_block_across_module_sources() {
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

        for (name, import_sources) in [
            (
                "duplicate import",
                vec![
                    source(
                        "import_a.veln",
                        "mod app\n\nuse dep from \"example/dep\"\n",
                    ),
                    source(
                        "import_b.veln",
                        "mod app\n\nuse dep from \"example/dep\"\n",
                    ),
                ],
            ),
            (
                "recovered import",
                vec![source(
                    "import.veln",
                    "mod app\n\nuse dep from \"example/dep\" unexpected\n",
                )],
            ),
        ] {
            let mut sources = import_sources;
            sources.push(source(
                "operation.veln",
                concat!(
                    "mod app\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ));
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                sources,
                vec![dependency.clone()],
            );

            assert!(
                query_snapshot(&snapshot, "operation.veln", 4, 16).is_none(),
                "{name} must block dependency alias fallback across module sources"
            );
        }
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
    fn direct_dependency_schema_alias_target_resolves_across_modules() {
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

        for (path, line, column) in [
            ("main.veln", 5, 19),
            ("main.veln", 6, 19),
        ] {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("main.veln", 5, 18), ("main.veln", 6, 18)]
            );
        }
    }

    #[test]
    fn direct_dependency_schema_alias_target_imports_are_visible_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "core.veln",
                    "mod core\n\npub schema Packet\n  value: Int\nend\n",
                ),
                ("facade/imports.veln", "mod facade\nuse core\n"),
                (
                    "facade/alias.veln",
                    "mod facade\n\npub schema Alias = core::Packet\n",
                ),
            ],
            ["core.veln", "facade/alias.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/dep\"\n\n",
                    "fn operations(view: ByteView, packet: {value: Int}) -> ()\n",
                    "  decode facade::Alias from view at byte_offset(0)?\n",
                    "  encode facade::Alias from packet\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(4, 19), (5, 19)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                locations(&result.references),
                [("main.veln", 4, 18), ("main.veln", 5, 18)]
            );
        }
    }

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
                false,
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
                    "mod facade\n\npub schema Alias = nested::core::Packet\n",
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
}
