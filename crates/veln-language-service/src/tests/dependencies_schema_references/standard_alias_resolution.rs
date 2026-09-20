    #[test]
    fn standard_library_schema_alias_import_paths_precedence_and_collisions_are_explicit() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                ),
                (
                    "alpha/wire.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                ),
            ],
            ["wire.veln", "alpha/wire.veln"],
        );

        let implicit = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use alpha::wire from \"std\"\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode alpha::wire::AliasPacket from view at byte_offset(0)?\n",
                "  decode wire::AliasPacket from view at byte_offset(0)?\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library.clone());
        for (line, column) in [(4, 25), (5, 18)] {
            let result = query_snapshot(&implicit, "main.veln", line, column).unwrap();
            assert_eq!(result.selected_symbol.name, "AliasPacket");
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(locations(&result.references), [("main.veln", 4, 23), ("main.veln", 5, 16)]);
        }

        for imports in [
            "mod app\nuse workspace::wire\nuse wire from \"std\"\n\n",
            "mod app\nuse wire from \"std\"\nuse workspace::wire\n\n",
        ] {
            let snapshot = EffectiveProjectSnapshot::new(vec![
                source(
                    "workspace/wire.veln",
                    "pub schema Packet\n  value: Bool\nend\n\npub schema AliasPacket = Packet\n",
                ),
                source("imports.veln", imports),
                source(
                    "uses.veln",
                    concat!(
                        "mod app\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::AliasPacket from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ])
            .with_standard_library(standard_library.clone());
            let result = query_snapshot(&snapshot, "uses.veln", 4, 18).unwrap();
            assert_eq!(result.selected_symbol.name, "AliasPacket");
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(locations(&result.references), [("uses.veln", 4, 16)]);
        }

        let collision = EffectiveProjectSnapshot::new(vec![
            source(
                "wire.veln",
                "pub schema Packet\n  value: Bool\nend\n\npub schema AliasPacket = Packet\n",
            ),
            source(
                "main.veln",
                "use wire\nuse wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::AliasPacket from view at byte_offset(0)?\nend\n",
            ),
        ])
        .with_standard_library(standard_library);
        assert!(query_snapshot(&collision, "main.veln", 5, 18).is_none());
    }

    #[test]
    fn standard_library_schema_alias_qualified_target_resolution_matrix() {
        let cases = [
            (
                "full written target path",
                vec![
                    (
                        "nested/core.veln",
                        "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse nested::core\n\npub schema Mid = nested::core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                true,
            ),
            (
                "unique implicit target leaf",
                vec![
                    (
                        "nested/core.veln",
                        "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse nested::core\n\npub schema Mid = core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                true,
            ),
            (
                "exact path precedes ambiguous implicit leaf",
                vec![
                    (
                        "alpha/core.veln",
                        "mod alpha::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "beta/core.veln",
                        "mod beta::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse alpha::core\nuse beta::core\n\npub schema Mid = alpha::core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                true,
            ),
            (
                "ambiguous target import",
                vec![
                    (
                        "alpha/core.veln",
                        "mod alpha::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "beta/core.veln",
                        "mod beta::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse alpha::core\nuse beta::core\n\npub schema Mid = core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                false,
            ),
            (
                "recovered target import",
                vec![
                    (
                        "nested/core.veln",
                        "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse nested::core unexpected\n\npub schema Mid = core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                false,
            ),
            (
                "invalid-cased target import",
                vec![
                    (
                        "nested/core.veln",
                        "mod nested::core\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "facade.veln",
                        "mod facade\nuse nested::Core\n\npub schema Mid = Core::Packet\npub schema Alias = Mid\n",
                    ),
                ],
                false,
            ),
        ];

        for (name, standard_sources, eligible) in cases {
            let exports = standard_sources.iter().map(|(path, _)| *path);
            let snapshot = EffectiveProjectSnapshot::new(vec![source(
                "main.veln",
                concat!(
                    "use facade from \"std\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode facade::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )])
            .with_standard_library(standard_library_snapshot(&standard_sources, exports));

            let selected = query_snapshot(&snapshot, "main.veln", 4, 19);
            if eligible {
                let selected = selected.unwrap_or_else(|| panic!("{name}"));
                assert_eq!(
                    selected.selected_symbol.declaration_kind,
                    SymbolDeclarationKind::PublicAlias,
                    "{name}"
                );
                assert_eq!(
                    locations(&selected.references),
                    [("main.veln", 4, 18)],
                    "{name}"
                );
            } else {
                assert!(selected.is_none(), "{name}");
            }
        }
    }

    #[test]
    fn standard_library_schema_alias_import_failures_cover_composition_and_operation_leaves() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "alpha/wire.veln",
                    "pub schema Packet\n  value: Int\nend\npub schema AliasPacket = Packet\n",
                ),
                (
                    "beta/wire.veln",
                    "pub schema Packet\n  value: Int\nend\npub schema AliasPacket = Packet\n",
                ),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        );
        let cases = [
            (
                "duplicate",
                "use alpha::wire from \"std\"\nuse alpha::wire from \"std\"\n\n",
                "wire::AliasPacket",
            ),
            (
                "conflicting",
                "use alpha::wire from \"std\"\nuse beta::wire from \"std\"\n\n",
                "wire::AliasPacket",
            ),
            ("recovered", "use alpha::wire from \"std\" broken\n\n", "wire::AliasPacket"),
            (
                "mismatched",
                "use alpha::wire from \"std\"\n\n",
                "beta::wire::AliasPacket",
            ),
        ];
        for (name, imports, selected) in cases {
            for reverse in [false, true] {
                let imports = if reverse && name == "conflicting" {
                    "use beta::wire from \"std\"\nuse alpha::wire from \"std\"\n\n"
                } else {
                    imports
                };
                let main = format!(
                    "{imports}schema Host\n  count: UInt8\n  nested: {selected}\n  repeated: Repeat(count, {selected})\n  array: [{selected}; count]\nend\n\nfn read(view: ByteView, packet: {{value: Int}}) -> ()\n  decode {selected} from view at byte_offset(0)?\n  encode {selected} from packet\nend\n"
                );
                let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &main)])
                    .with_standard_library(standard_library.clone());
                let leaf_positions: Vec<_> = main
                    .lines()
                    .enumerate()
                    .filter_map(|(line, text)| {
                        text.find(selected).map(|column| {
                            let alias_column = selected
                                .rfind("::")
                                .map_or(0, |separator| separator + 2);
                            (line + 1, column + alias_column + 1)
                        })
                    })
                    .collect();
                assert_eq!(leaf_positions.len(), 5, "{name} {reverse}");
                for (line, column) in leaf_positions {
                    let result = query_snapshot(&snapshot, "main.veln", line, column);
                    assert!(
                        result.as_ref().is_none_or(|selection| selection.references.is_empty()),
                        "{name} {reverse} leaf must be empty at {line}:{column}: {result:#?}"
                    );
                }
            }
        }
    }

    #[test]
    fn standard_library_schema_exact_import_precedes_workspace_alias_in_either_source_order() {
        for workspace_import_first in [true, false] {
            let workspace_import = source(
                "workspace_import.veln",
                "mod app\nuse workspace::wire\n",
            );
            let standard_import = source("standard_import.veln", "mod app\nuse wire from \"std\"\n");
            let imports = if workspace_import_first {
                vec![workspace_import, standard_import]
            } else {
                vec![standard_import, workspace_import]
            };
            let mut sources = vec![source(
                "workspace/wire.veln",
                "pub schema Packet\n  value: Bool\nend\n",
            )];
            sources.extend(imports);
            sources.push(source(
                "uses.veln",
                concat!(
                    "mod app\n\n",
                    "schema Host\n",
                    "  nested: wire::Packet\n",
                    "end\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ));
            let snapshot = EffectiveProjectSnapshot::new(sources).with_standard_library(
                standard_library_snapshot(
                    &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
                    ["wire.veln"],
                ),
            );

            for (line, column) in [(4, 19), (8, 16)] {
                let selected = query_snapshot(&snapshot, "uses.veln", line, column)
                    .unwrap_or_else(|| panic!("missing standard schema selection at {line}:{column}"));
                assert_eq!(
                    locations(&selected.references),
                    [("uses.veln", 4, 17), ("uses.veln", 8, 16)]
                );
                assert_eq!(
                    selected.selected_symbol.package_origin,
                    Some(PackageOrigin::StandardLibrary)
                );
            }
        }
    }

    #[test]
    fn standard_library_schema_exact_import_conflicts_with_exact_workspace_import() {
        for workspace_import_first in [true, false] {
            let imports = if workspace_import_first {
                "use wire\nuse wire from \"std\"\n\n"
            } else {
                "use wire from \"std\"\nuse wire\n\n"
            };
            let snapshot = EffectiveProjectSnapshot::new(vec![
                source("wire.veln", "pub schema Packet\n  value: Bool\nend\n"),
                source(
                    "main.veln",
                    &format!(
                        "{imports}schema Host\n  nested: wire::Packet\nend\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n"
                    ),
                ),
            ])
            .with_standard_library(standard_library_snapshot(
                &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
                ["wire.veln"],
            ));

            for (line, column) in [(3, 19), (7, 16)] {
                let selected = query_snapshot(&snapshot, "main.veln", line, column);
                assert!(
                    selected.is_none() || selected.unwrap().references.is_empty(),
                    "exact workspace and std imports must be ambiguous: {line}:{column}"
                );
            }
        }
    }

    #[test]
    fn standard_library_schema_references_unify_supported_leaf_roles_and_isolate_origins() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "workspace/wire.veln",
                    "pub schema Packet\n  value: Bool\nend\n",
                ),
                source("workspace_import.veln", "mod app\nuse workspace::wire\n"),
                source(
                    "imports.veln",
                    "mod app\nuse wire from \"std\"\n",
                ),
                source(
                    "dependency.veln",
                    "mod dependency\nuse lib::wire from \"example/dep\"\n\nschema Host\n  nested: lib::wire::Packet\nend\n",
                ),
                source(
                    "lib/wire.veln",
                    "pub schema Packet\n  value: String\nend\n",
                ),
                source(
                    "uses.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: wire::Packet\n",
                        "  repeated: Repeat(count, wire::Packet)\n",
                        "  array: [wire::Packet; count]\n",
                        "end\n\n",
                        "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "  encode wire::Packet from packet\n",
                        "end\n\n",
                        "fn noise() -> String\n",
                        "  // wire::Packet\n",
                        "  \"wire::Packet\"\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    "mod other\n\nschema Host\n  nested: wire::Packet\nend\n",
                ),
            ],
            vec![dependency_snapshot(
                "example/dep",
                &[("lib/wire.veln", "pub schema Packet\n  value: Int\nend\n")],
                ["lib/wire.veln"],
            )],
        )
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));

        let expected = [
            ("uses.veln", 5, 17),
            ("uses.veln", 6, 33),
            ("uses.veln", 7, 17),
            ("uses.veln", 11, 16),
            ("uses.veln", 12, 16),
        ];
        for (line, column) in [(5, 20), (6, 34), (7, 18), (11, 16), (12, 19)] {
            let result = query_snapshot(&snapshot, "uses.veln", line, column)
                .unwrap_or_else(|| panic!("missing standard schema selection at {line}:{column}"));
            assert_eq!(locations(&result.references), expected);
            assert!(matches!(result.definition.source, NavigationSource::Package { .. }));
        }

        // Import tokens and module qualifiers are lexical selections rather
        // than references to the schema identity.
        for (path, line, column) in [
            ("imports.veln", 1, 8),
            ("uses.veln", 5, 12),
            ("uses.veln", 5, 16),
        ] {
            assert!(
                query_snapshot(&snapshot, path, line, column).is_none(),
                "standard-library schema exclusion at {path}:{line}:{column}"
            );
        }
        assert!(query_snapshot(&snapshot, "uses.veln", 16, 8).is_none());
        assert!(query_snapshot(&snapshot, "uses.veln", 17, 4).is_none());
        assert!(query_snapshot(&snapshot, "other.veln", 4, 20).is_none());
    }

    #[test]
    fn standard_library_schema_reference_exclusions_cover_import_module_and_alias_target_tokens() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "schema Host\n",
                "  Packet: Int\n",
                "end\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  # wire::Packet\n",
                "  \"wire::Packet\"\n",
                "  decode wire::Packet from view at byte_offset(0)?\n",
                "end\n",
            ),
        ), source(
            "noise.veln",
            "type Packet\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "wire.veln",
                "pub schema Base\n  value: Int\nend\n\npub schema Packet = Base\n",
            )],
            ["wire.veln"],
        ));

        for (line, column) in [(1, 5), (10, 10)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "standard-library schema exclusion at main.veln:{line}:{column}"
            );
        }
        for (line, column) in [(8, 6), (9, 6)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "standard-library alias lexical exclusion at main.veln:{line}:{column}"
            );
        }
        let field = query_snapshot(&snapshot, "main.veln", 4, 4);
        assert!(
            field.is_none(),
            "standard-library alias field name must not select the alias: {field:#?}"
        );
        let unrelated = query_snapshot(&snapshot, "noise.veln", 1, 6);
        assert!(
            unrelated.as_ref().is_none_or(|result| {
                result.selected_symbol.package_origin != Some(PackageOrigin::StandardLibrary)
            }),
            "same-spelled unrelated declaration must not select the standard-library alias: {unrelated:#?}"
        );
        let alias_target = query_snapshot(&snapshot, "wire.veln", 4, 20);
        assert!(
            alias_target.is_none(),
            "standard-library alias target must be an unsupported selection: {alias_target:#?}"
        );
        let alias_declaration = query_snapshot(&snapshot, "wire.veln", 4, 12);
        assert!(
            alias_declaration.is_none(),
            "standard-library alias declaration must be an unsupported selection: {alias_declaration:#?}"
        );
    }

    #[test]
    fn standard_library_schema_alias_origin_isolation_is_symmetric() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "model.veln",
                    "mod model\npub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n",
                ),
                source(
                    "workspace-use.veln",
                    "mod app\n\nuse model\n\nschema Host\n  nested: model::Alias\nend\n",
                ),
                source(
                    "dependency-use.veln",
                    "use model from \"example/dep\"\n\nschema Host\n  nested: model::Alias\nend\n",
                ),
                source(
                    "standard-use.veln",
                    "use model from \"std\"\n\nschema Host\n  nested: model::Alias\nend\n",
                ),
            ],
            vec![dependency_snapshot(
                "example/dep",
                &[("model.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n")],
                ["model.veln"],
            )],
        )
        .with_standard_library(standard_library_snapshot(
            &[("model.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n")],
            ["model.veln"],
        ));

        let cases = [
            (
                "workspace-use.veln",
                6,
                18,
                None,
                "workspace-use.veln",
                6,
                18,
            ),
            (
                "dependency-use.veln",
                4,
                18,
                Some(PackageOrigin::DirectDependency),
                "dependency-use.veln",
                4,
                18,
            ),
            (
                "standard-use.veln",
                4,
                18,
                Some(PackageOrigin::StandardLibrary),
                "standard-use.veln",
                4,
                18,
            ),
        ];
        for (path, line, column, origin, reference_path, reference_line, reference_column) in cases {
            let selected = query_snapshot(&snapshot, path, line, column)
                .unwrap_or_else(|| panic!("missing {origin:?} selection in {path}"));
            assert_eq!(selected.selected_symbol.package_origin, origin);
            assert_eq!(
                locations(&selected.references),
                [(reference_path, reference_line, reference_column)]
            );
        }
    }
