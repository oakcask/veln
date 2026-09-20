    #[test]
    fn exact_prelude_imports_keep_one_schema_alias_identity_across_leaf_roles() {
        let alias_source = concat!(
            "pub schema Packet\n",
            "  format binary\n",
            "  value: Int\n",
            "end\n\n",
            "pub schema AliasPacket = Packet\n",
        );
        let leaf_source = concat!(
            "schema Host\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  direct: prelude::AliasPacket\n",
            "  repeated: Repeat(count, prelude::AliasPacket)\n",
            "  array: [prelude::AliasPacket; count]\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode prelude::AliasPacket from view at byte_offset(0)?\n",
            "  encode prelude::AliasPacket from packet\n",
            "end\n",
        );
        let dependency = dependency_snapshot(
            "example/dep",
            &[("prelude.veln", alias_source)],
            ["prelude.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[("prelude.veln", alias_source)],
            ["prelude.veln"],
        );
        let cases = [
            ("workspace exact", "use prelude\n", Some(None)),
            (
                "external exact",
                "use prelude from \"example/dep\"\n",
                Some(Some(PackageOrigin::DirectDependency)),
            ),
            (
                "workspace then external collision",
                "use prelude\nuse prelude from \"example/dep\"\n",
                None,
            ),
            (
                "external then workspace collision",
                "use prelude from \"example/dep\"\nuse prelude\n",
                None,
            ),
        ];

        for (name, imports, expected_identity) in cases {
            assert_exact_prelude_import_case(
                name,
                imports,
                expected_identity,
                leaf_source,
                alias_source,
                dependency.clone(),
                standard_library.clone(),
            );
        }
    }

    fn assert_exact_prelude_import_case(
        name: &str,
        imports: &str,
        expected_origin: Option<Option<PackageOrigin>>,
        leaf_source: &str,
        alias_source: &str,
        dependency: DirectDependencySnapshot,
        standard_library: DirectDependencySnapshot,
    ) {
        let main = format!("{imports}\n{leaf_source}");
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source("main.veln", &main),
                source("prelude.veln", alias_source),
            ],
            vec![dependency],
        )
        .with_standard_library(standard_library);
        let selections = alias_selections_on_matching_lines(
            &snapshot,
            &main,
            "prelude::AliasPacket",
            "AliasPacket",
        );
        assert_eq!(selections.len(), 5, "{name}");
        let Some(expected_origin) = expected_origin else {
            assert!(selections.iter().all(Option::is_none), "{name}: {selections:#?}");
            return;
        };
        let selections = selections.into_iter().collect::<Option<Vec<_>>>()
            .unwrap_or_else(|| panic!("{name}: missing alias selection"));
        let expected_locations = selections
            .iter()
            .map(|result| {
                (
                    "main.veln",
                    result.selection.start.line,
                    result.selection.start.column,
                )
            })
            .collect::<Vec<_>>();
        for selected in selections {
            assert_eq!(selected.selected_symbol.package_origin, expected_origin, "{name}");
            assert_eq!(locations(&selected.references), expected_locations, "{name}");
        }
    }

    fn alias_selections_on_matching_lines(
        snapshot: &EffectiveProjectSnapshot,
        source_text: &str,
        written_name: &str,
        selected_name: &str,
    ) -> Vec<Option<NavigationResult>> {
        source_text
            .lines()
            .enumerate()
            .filter(|(_, text)| text.contains(written_name))
            .map(|(line, text)| {
                (0..text.len()).find_map(|column| {
                    query_snapshot(snapshot, "main.veln", line + 1, column)
                        .filter(|result| result.selected_symbol.name == selected_name)
                })
            })
            .collect()
    }

    #[test]
    fn qualified_prelude_schema_alias_ignores_local_bare_name_blockers() {
        let main = concat!(
            "schema Host\n",
            "  count: UInt8\n",
            "  direct: prelude::AliasPacket\n",
            "  repeated: Repeat(count, prelude::AliasPacket)\n",
            "  array: [prelude::AliasPacket; count]\n",
            "end\n\n",
            "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode prelude::AliasPacket from view at byte_offset(0)?\n",
            "  encode prelude::AliasPacket from packet\n",
            "end\n",
        );
        let expected = [
            ("main.veln", 3, 20),
            ("main.veln", 4, 36),
            ("main.veln", 5, 20),
            ("main.veln", 9, 19),
            ("main.veln", 10, 19),
        ];

        for (name, blocker) in [
            (
                "schema",
                "mod main\nschema AliasPacket\n  value: Int\nend\n",
            ),
            (
                "schema alias",
                "mod main\nschema LocalPacket\n  value: Int\nend\nschema AliasPacket = LocalPacket\n",
            ),
            ("type", "mod main\ntype AliasPacket\n  Value\nend\n"),
            (
                "type alias",
                "mod main\ntype LocalPacket\n  Value\nend\npub type AliasPacket = LocalPacket\n",
            ),
        ] {
            let snapshot = EffectiveProjectSnapshot::new(vec![
                source("main.veln", main),
                source("blocker.veln", blocker),
            ])
            .with_standard_library(standard_library_snapshot(
                &[(
                    "prelude.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                )],
                ["prelude.veln"],
            ));

            for (line, column) in [(3, 20), (4, 36), (5, 20), (9, 19), (10, 19)] {
                let selected = query_snapshot(&snapshot, "main.veln", line, column)
                    .unwrap_or_else(|| panic!("{name} blocked qualified alias at {line}:{column}"));
                assert_eq!(selected.selected_symbol.name, "AliasPacket", "{name}");
                assert_eq!(locations(&selected.references), expected, "{name}");
            }
        }
    }

    #[test]
    fn standard_library_schema_alias_package_source_occurrences_are_not_project_references() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "fn read(view: ByteView) -> ()\n  decode AliasPacket from view at byte_offset(0)?\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n\nfn read(view: ByteView) -> ()\n  decode AliasPacket from view at byte_offset(0)?\nend\n",
            )],
            ["prelude.veln"],
        ));

        let selected = query_snapshot(&snapshot, "main.veln", 2, 10).unwrap();
        assert_eq!(selected.selected_symbol.name, "AliasPacket");
        assert_eq!(
            selected.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(locations(&selected.references), [("main.veln", 2, 10)]);
        assert!(query_snapshot(&snapshot, "prelude.veln", 8, 10).is_none());
    }

    #[test]
    fn local_type_names_block_bare_prelude_alias_only_in_composition() {
        for local_declaration in [
            "type AliasPacket\n  Value\nend\n\n",
            "type Packet\n  Value\nend\npub type AliasPacket = Packet\n\n",
        ] {
            let source_text = format!(concat!(
                "{local_declaration}",
                "schema Host\n",
                "  nested: AliasPacket\n",
                "end\n\n",
                "fn operations(view: ByteView, packet: {{value: Int}}) -> ()\n",
                "  decode AliasPacket from view at byte_offset(0)?\n",
                "  encode AliasPacket from packet\n",
                "end\n",
            ), local_declaration = local_declaration);
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)])
                .with_standard_library(standard_library_snapshot(
                    &[(
                        "prelude.veln",
                        "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                    )],
                    ["prelude.veln"],
                ));
            let declaration_lines = local_declaration.lines().count();
            assert!(query_snapshot(&snapshot, "main.veln", declaration_lines + 2, 12).is_none());
            let decode_line = declaration_lines + 6;
            let encode_line = declaration_lines + 7;
            let selected = query_snapshot(&snapshot, "main.veln", decode_line, 10).unwrap();
            assert_eq!(
                selected.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(
                locations(&selected.references),
                [("main.veln", decode_line, 10), ("main.veln", encode_line, 10)]
            );
            assert_eq!(
                query_snapshot(&snapshot, "main.veln", encode_line, 10)
                    .unwrap()
                    .selected_symbol
                    .package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
        }
    }

    #[test]
    fn local_schema_shadows_bare_standard_library_prelude_alias() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "pub schema AliasPacket\n",
                "  value: Int\n",
                "end\n\n",
                "schema Host\n",
                "  nested: AliasPacket\n",
                "end\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode AliasPacket from view at byte_offset(0)?\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub schema Packet\n  value: Bool\nend\n\npub schema AliasPacket = Packet\n",
            )],
            ["prelude.veln"],
        ));

        let selected = query_snapshot(&snapshot, "main.veln", 10, 10).unwrap();
        assert_eq!(selected.selected_symbol.name, "AliasPacket");
        assert_eq!(selected.selected_symbol.package_origin, None);
        assert_eq!(locations(&selected.references), [("main.veln", 6, 11), ("main.veln", 10, 10)]);
    }

    #[test]
    fn local_schema_aliases_shadow_or_block_bare_standard_library_prelude_aliases() {
        let blocked = EffectiveProjectSnapshot::new(vec![source(
            "blocked.veln",
            concat!(
                "pub schema AliasPacket = Missing\n\n",
                "schema Host\n",
                "  nested: AliasPacket\n",
                "end\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode AliasPacket from view at byte_offset(0)?\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub schema Packet\n  value: Bool\nend\n\npub schema AliasPacket = Packet\n",
            )],
            ["prelude.veln"],
        ));
        for (line, column) in [(4, 11), (9, 10)] {
            assert!(query_snapshot(&blocked, "blocked.veln", line, column).is_none_or(|result| {
                result.selected_symbol.package_origin != Some(PackageOrigin::StandardLibrary)
            }));
        }
    }

    #[test]
    fn standard_library_schema_alias_chain_unifies_supported_leaves_and_keeps_target_identity_separate() {
        let main = concat!(
                "use wire from \"std\"\n\n",
                "schema Host\n",
                "  count: UInt8\n",
                "  top: wire::Top\n",
                "  top_repeat: Repeat(count, wire::Top)\n",
                "  top_array: [wire::Top; count]\n",
                "  mid: wire::Mid\n",
                "  mid_repeat: Repeat(count, wire::Mid)\n",
                "  mid_array: [wire::Mid; count]\n",
                "  packet: wire::Packet\n",
                "  packet_repeat: Repeat(count, wire::Packet)\n",
                "  packet_array: [wire::Packet; count]\n",
                "  sibling: wire::Sibling\n",
                "  sibling_repeat: Repeat(count, wire::Sibling)\n",
                "  sibling_array: [wire::Sibling; count]\n",
                "end\n\n",
                "schema Noise\n",
                "  Top: Int\n",
                "end\n\n",
                "type Top\n",
                "  Ready(Int)\n",
                "end\n\n",
                "fn read(view: ByteView, packet: {value: Int}) -> ()\n",
                "  decode wire::Top from view at byte_offset(0)?\n",
                "  decode wire::Mid from view at byte_offset(0)?\n",
                "  decode wire::Packet from view at byte_offset(0)?\n",
                "  decode wire::Sibling from view at byte_offset(0)?\n",
                "  encode wire::Top from packet\n",
                "  encode wire::Mid from packet\n",
                "  encode wire::Packet from packet\n",
                "  encode wire::Sibling from packet\n",
                "end\n",
                "\nfn noise() -> String\n",
                "  // wire::Top\n",
                "  \"wire::Top\"\n",
                "end\n",
            );
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
        .with_standard_library(standard_library_snapshot(
            &[(
                "wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Mid = Packet\npub schema Top = Mid\n",
                    "pub schema Sibling = Packet\n",
                ),
            )],
            ["wire.veln"],
        ));

        for name in ["Top", "Mid", "Packet", "Sibling"] {
            assert_standard_alias_chain_identity(&snapshot, main, name);
        }
    }

    fn assert_standard_alias_chain_identity(
        snapshot: &EffectiveProjectSnapshot,
        source_text: &str,
        name: &str,
    ) {
        let selections = standard_alias_leaf_selections(snapshot, source_text, name);
        assert_eq!(selections.len(), 5, "{name} must have every leaf role");
        let selected = &selections[0];
        assert_eq!(
            selected.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        let expected = selections
            .iter()
            .map(|result| {
                (
                    "main.veln",
                    result.selection.start.line,
                    result.selection.start.column,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            locations(&selected.references),
            expected,
            "{name} must keep each exact leaf role"
        );
        assert_standard_alias_locations_are_isolated(snapshot, source_text, name, selected);
    }

    fn standard_alias_leaf_selections(
        snapshot: &EffectiveProjectSnapshot,
        source_text: &str,
        name: &str,
    ) -> Vec<NavigationResult> {
        source_text
            .lines()
            .enumerate()
            .filter(|(_, text)| {
                text.contains(&format!("wire::{name}"))
                    && !text.trim_start().starts_with("//")
                    && !text.trim_start().starts_with('"')
            })
            .map(|(line, text)| {
                (0..text.len())
                    .find_map(|column| {
                        query_snapshot(snapshot, "main.veln", line + 1, column)
                            .filter(|result| result.selected_symbol.name == name)
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "missing standard-library identity for {name} at line {}",
                            line + 1
                        )
                    })
            })
            .collect()
    }

    fn assert_standard_alias_locations_are_isolated(
        snapshot: &EffectiveProjectSnapshot,
        source_text: &str,
        name: &str,
        selected: &NavigationResult,
    ) {
        let selected_locations = locations(&selected.references);
        for other in ["Top", "Mid", "Packet", "Sibling"]
            .into_iter()
            .filter(|other| *other != name)
        {
            let other_selected = standard_alias_leaf_selections(snapshot, source_text, other)
                .into_iter()
                .next()
                .expect("all chain identities must be selectable");
            let other_locations = locations(&other_selected.references);
            assert!(
                selected_locations
                    .iter()
                    .all(|location| !other_locations.contains(location)),
                "{name} and {other} must not exchange exact occurrence ranges"
            );
        }
    }
