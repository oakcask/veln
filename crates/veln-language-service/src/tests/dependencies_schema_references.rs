mod dependencies_schema_references_tests {
    use super::*;

    #[test]
    fn dependency_schema_alias_eligibility_visits_declarations_once() {
        // Count eligibility traversals instead of repeatedly timing the entire index build.
        // Keep instrumentation inside declaration visits when changing these traversals.
        for count in [100, 200, 400] {
            let snapshot = dependency_schema_alias_resolution_snapshot(count);
            crate::navigation::reset_schema_alias_declaration_visits();
            let _ = snapshot.navigation_index();
            assert_eq!(
                crate::navigation::schema_alias_declaration_visits(),
                count * 4,
                "eligibility must visit each resolved alias, alias declaration, export check, and target once",
            );
            let result = query_snapshot(&snapshot, "main.veln", 4, 19).unwrap();
            assert_eq!(result.selected_symbol.name, "Alias0");
            assert_eq!(locations(&result.references), [("main.veln", 4, 18)]);
            let last = query_snapshot(&snapshot, "main.veln", 5, 19).unwrap();
            assert_eq!(last.selected_symbol.name, format!("Alias{}", count - 1));
            assert_eq!(locations(&last.references), [("main.veln", 5, 18)]);
        }
    }

    #[test]
    fn dependency_schema_alias_chains_resolve_with_bounded_index_work() {
        for count in [64, 128, 256] {
            let snapshot = dependency_schema_alias_chain_snapshot(count);
            crate::navigation::reset_schema_alias_declaration_visits();
            let _ = snapshot.navigation_index();
            assert!(
                crate::navigation::schema_alias_declaration_visits() <= count * 4,
                "chain eligibility work must remain linear"
            );
            let selected = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
            assert_eq!(selected.selected_symbol.name, format!("Alias{}", count - 1));
            assert_eq!(locations(&selected.references), [("main.veln", 4, 15)]);
        }
    }

    #[test]
    fn dependency_schema_alias_shared_suffix_and_disconnected_cycle_stay_bounded() {
        for count in [64, 128, 256] {
            let mut dependency_source = String::from(
                "pub schema Packet\n  value: Int\nend\n\n",
            );
            dependency_source.push_str("pub schema Shared0 = Packet\n");
            for index in 1..count {
                dependency_source.push_str(&format!(
                    "pub schema Shared{index} = Shared{}\n",
                    index - 1
                ));
            }
            for index in 0..count {
                dependency_source.push_str(&format!(
                    "pub schema Left{index} = Shared{}\npub schema Right{index} = Shared{}\n",
                    count - 1,
                    count - 1
                ));
            }
            dependency_source.push_str(
                "pub schema CycleA = CycleB\npub schema CycleB = CycleA\n",
            );
            let dependency = dependency_snapshot(
                "example/dep",
                &[("dep.veln", &dependency_source)],
                ["dep.veln"],
            );
            let main_source = format!(
                "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode dep::Left{} from view at byte_offset(0)?\n  decode dep::Right{} from view at byte_offset(0)?\n  decode dep::CycleA from view at byte_offset(0)?\nend\n",
                count - 1,
                count - 1
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &main_source)],
                vec![dependency],
            );

            crate::navigation::reset_schema_alias_eligibility_visits();
            let _ = snapshot.navigation_index();
            assert!(
                crate::navigation::schema_alias_eligibility_visits() <= count * 3 + 2,
                "package eligibility must not re-expand shared suffixes or cycles"
            );
            let left = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
            assert_eq!(left.selected_symbol.name, format!("Left{}", count - 1));
            let right = query_snapshot(&snapshot, "main.veln", 5, 16).unwrap();
            assert_eq!(right.selected_symbol.name, format!("Right{}", count - 1));
            assert!(query_snapshot(&snapshot, "main.veln", 6, 16).is_none());
        }
    }

    #[test]
    fn dependency_schema_alias_chain_keeps_each_identity_and_isolates_cycles() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "dep.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Mid = Packet\n",
                    "pub schema Top = Mid\n",
                    "pub schema Sibling = Packet\n",
                    "pub schema BrokenA = BrokenB\n",
                    "pub schema BrokenB = BrokenA\n",
                    "pub schema Valid = Packet\n",
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
                    "  decode dep::Top from view at byte_offset(0)?\n",
                    "  decode dep::Mid from view at byte_offset(0)?\n",
                    "  decode dep::Packet from view at byte_offset(0)?\n",
                    "  decode dep::BrokenA from view at byte_offset(0)?\n",
                    "  decode dep::Valid from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let top = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
        assert_eq!(locations(&top.references), [("main.veln", 4, 15)]);
        let mid = query_snapshot(&snapshot, "main.veln", 5, 16).unwrap();
        assert_eq!(locations(&mid.references), [("main.veln", 5, 15)]);
        let packet = query_snapshot(&snapshot, "main.veln", 6, 16).unwrap();
        assert_eq!(locations(&packet.references), [("main.veln", 6, 15)]);
        assert!(query_snapshot(&snapshot, "main.veln", 7, 16).is_none());
        let valid = query_snapshot(&snapshot, "main.veln", 8, 16).unwrap();
        assert_eq!(locations(&valid.references), [("main.veln", 8, 15)]);
    }

    #[test]
    fn dependency_schema_alias_chain_rejects_self_cycle_chain_cycle_and_deep_terminal_blocker() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "dep.veln",
                    concat!(
                        "use private\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Valid = Packet\n",
                        "pub schema SelfCycle = SelfCycle\n",
                        "pub schema ChainCycle = CycleTail\n",
                        "pub schema CycleTail = ChainCycle\n",
                        "pub schema DeepPrivate = DeepPrivateMid\n",
                        "pub schema DeepPrivateMid = private::Packet\n",
                    ),
                ),
                (
                    "private.veln",
                    "mod private\n\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::SelfCycle from view at byte_offset(0)?\n",
                    "  decode dep::ChainCycle from view at byte_offset(0)?\n",
                    "  decode dep::DeepPrivate from view at byte_offset(0)?\n",
                    "  decode dep::Valid from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for line in [4, 5, 6] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, 16).is_none(),
                "chain blocker at line {line} must return the successful empty result",
            );
        }
        let valid = query_snapshot(&snapshot, "main.veln", 7, 16).unwrap();
        assert_eq!(valid.selected_symbol.name, "Valid");
        assert_eq!(locations(&valid.references), [("main.veln", 7, 15)]);
    }

    #[test]
    fn dependency_schema_alias_chain_intermediate_blockers_return_empty() {
        let cases = [
            (
                "private intermediate",
                vec![ (
                    "facade.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "schema Mid = Packet\n",
                        "pub schema Top = Mid\n",
                    ),
                )],
                vec!["facade.veln"],
                "Top",
            ),
            (
                "non-exported intermediate source",
                vec![
                    (
                        "packet.veln",
                        "mod facade\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "mid.veln",
                        "mod facade\n\npub schema Mid = Packet\n",
                    ),
                    (
                        "top.veln",
                        "mod facade\n\npub schema Top = Mid\n",
                    ),
                ],
                vec!["packet.veln", "top.veln"],
                "Top",
            ),
            (
                "invalid-cased intermediate",
                vec![ (
                    "facade.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema mid = Packet\n",
                        "pub schema Top = mid\n",
                    ),
                )],
                vec!["facade.veln"],
                "Top",
            ),
            (
                "duplicate intermediate aliases",
                vec![ (
                    "facade.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Mid = Packet\n",
                        "pub schema Mid = Packet\n",
                        "pub schema Top = Mid\n",
                    ),
                )],
                vec!["facade.veln"],
                "Top",
            ),
            (
                "wrong-kind intermediate",
                vec![ (
                    "facade.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub type Mid\n  Ready(Int)\nend\n\n",
                        "pub schema Top = Mid\n",
                    ),
                )],
                vec!["facade.veln"],
                "Top",
            ),
            (
                "missing intermediate",
                vec![ (
                    "facade.veln",
                    "mod facade\n\npub schema Top = Mid\n",
                )],
                vec!["facade.veln"],
                "Top",
            ),
            (
                "recovered schema collision",
                vec![
                    (
                        "packet.veln",
                        "mod facade\n\npub schema Packet\n  value: Int\nend\n",
                    ),
                    (
                        "mid.veln",
                        "mod facade\n\npub schema Mid = Packet\n",
                    ),
                    (
                        "recovered.veln",
                        "mod facade\n\npub schema Mid\n  value: Int\n",
                    ),
                    (
                        "top.veln",
                        "mod facade\n\npub schema Top = Mid\n",
                    ),
                ],
                vec!["packet.veln", "mid.veln", "top.veln"],
                "Top",
            ),
            (
                "dictionary-order non-exported intermediate",
                vec![ (
                    "facade.veln",
                    concat!(
                        "mod facade\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema ATop = ZMid\n",
                        "schema ZMid = Packet\n",
                    ),
                )],
                vec!["facade.veln"],
                "ATop",
            ),
        ];

        for (name, sources, exported, selected_alias) in cases {
            let dependency = dependency_snapshot("example/dep", &sources, exported);
            let main_source = format!(
                "use facade from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode facade::{selected_alias} from view at byte_offset(0)?\nend\n"
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &main_source)],
                vec![dependency],
            );
            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 16).is_none(),
                "ineligible chain must be empty: {name}"
            );
        }
    }

    #[test]
    fn dependency_schema_alias_chain_deep_terminal_blockers_return_empty() {
        let cases = [
            (
                "missing terminal",
                "pub schema Mid = Missing\npub schema Top = Mid\n",
                ["dep.veln"],
            ),
            (
                "invalid-cased terminal",
                "pub schema badPacket\n  value: Int\nend\npub schema Mid = badPacket\npub schema Top = Mid\n",
                ["dep.veln"],
            ),
            (
                "wrong-kind terminal",
                "pub type Packet\n  Ready(Int)\nend\npub schema Mid = Packet\npub schema Top = Mid\n",
                ["dep.veln"],
            ),
            (
                "recovered terminal",
                "pub schema Packet\n  value: Int\npub schema Mid = Packet\npub schema Top = Mid\n",
                ["dep.veln"],
            ),
        ];

        for (name, source_text, exported) in cases {
            let dependency =
                dependency_snapshot("example/dep", &[("dep.veln", source_text)], exported);
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode dep::Top from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );
            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 16).is_none(),
                "deep terminal blocker must be empty: {name}"
            );
        }

        let duplicate = dependency_snapshot(
            "example/dep",
            &[
                ("first.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("second.veln", "pub schema Packet\n  other: Int\nend\n"),
                (
                    "chain.veln",
                    "pub schema Mid = Packet\npub schema Top = Mid\n",
                ),
            ],
            ["first.veln", "second.veln", "chain.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Top from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![duplicate],
        );
        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
    }

    #[test]
    fn dependency_schema_alias_recovered_deep_terminal_does_not_hide_valid_chain() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "packet.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema OtherPacket\n  value: Int\nend\n",
                    ),
                ),
                (
                    "chain.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Mid = Packet\n",
                        "pub schema Top = Mid\n",
                        "pub schema OtherMid = OtherPacket\n",
                        "pub schema OtherTop = OtherMid\n",
                    ),
                ),
                (
                    "recovered.veln",
                    "mod dep\n\npub schema Packet\n  value: Int\n",
                ),
            ],
            ["packet.veln", "chain.veln", "recovered.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Top from view at byte_offset(0)?\n",
                    "  decode dep::OtherTop from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
        let valid = query_snapshot(&snapshot, "main.veln", 5, 16).unwrap();
        assert_eq!(valid.selected_symbol.name, "OtherTop");
        assert_eq!(locations(&valid.references), [("main.veln", 5, 15)]);
    }

    fn dependency_schema_alias_chain_snapshot(count: usize) -> EffectiveProjectSnapshot {
        let mut dependency_source = String::from("pub schema Packet\n  value: Int\nend\n\n");
        dependency_source.push_str("pub schema Alias0 = Packet\n");
        for index in 1..count {
            dependency_source.push_str(&format!("pub schema Alias{index} = Alias{}\n", index - 1));
        }
        let dependency = dependency_snapshot(
            "example/dep",
            &[("dep.veln", &dependency_source)],
            ["dep.veln"],
        );
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                &format!(
                    "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode dep::Alias{} from view at byte_offset(0)?\nend\n",
                    count - 1
                ),
            )],
            vec![dependency],
        )
    }

    fn dependency_schema_alias_resolution_snapshot(count: usize) -> EffectiveProjectSnapshot {
        let mut targets = String::from("mod core\n\n");
        let mut aliases = String::from("mod facade\n\n");
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
            &[
                ("core.veln", &targets),
                // Keep the qualified import in a different retained source
                // from the alias declarations. Every hop must use the same
                // cross-source import index as a direct target.
                ("facade-import.veln", "mod facade\nuse core\n"),
                ("facade.veln", &aliases),
            ],
            ["core.veln", "facade-import.veln", "facade.veln"],
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

    #[test]
    fn dependency_schema_composition_index_work_grows_linearly() {
        let mut field_token_visits_per_schema = None;
        for count in [100, 200, 400] {
            let mut declarations = String::new();
            let mut fields = String::from("schema Host\n");
            for index in 0..count {
                declarations.push_str(&format!(
                    "pub schema Packet{index}\n  value: Int\nend\n\n"
                ));
                fields.push_str(&format!("  field{index}: dep::Packet{index}\n"));
            }
            fields.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    &format!("use dep from \"example/dep\"\n\n{fields}"),
                )],
                vec![dependency_snapshot(
                    "example/dep",
                    &[("dep.veln", &declarations)],
                    ["dep.veln"],
                )],
            );

            crate::navigation::reset_schema_composition_index_work();
            crate::navigation::reset_schema_alias_declaration_visits();
            let _ = snapshot.navigation_index();
            let (declaration_visits, field_token_visits) =
                crate::navigation::schema_composition_index_work();
            // Declaration eligibility is shared with alias indexing; composition only
            // visits its schema candidates.
            assert_eq!(declaration_visits, count);
            assert_eq!(
                crate::navigation::schema_composition_target_lookups(),
                count,
                "each composition leaf must use one indexed target lookup"
            );
            assert_eq!(crate::navigation::schema_alias_declaration_visits(), count);
            assert_eq!(field_token_visits % count, 0);
            let visits_per_schema = field_token_visits / count;
            assert_eq!(
                *field_token_visits_per_schema.get_or_insert(visits_per_schema),
                visits_per_schema,
            );

            let first = query_snapshot(&snapshot, "main.veln", 4, 18).unwrap();
            assert_eq!(first.selected_symbol.name, "Packet0");
            let last = query_snapshot(&snapshot, "main.veln", count + 3, 18).unwrap();
            assert_eq!(last.selected_symbol.name, format!("Packet{}", count - 1));
        }
    }

    #[test]
    fn eligible_schema_alias_composition_index_work_grows_linearly() {
        let mut field_token_visits_per_field = None;
        for count in [100, 200, 400] {
            let started = std::time::Instant::now();
            let mut declarations = String::new();
            let mut fields = String::from("schema Host\n");
            for index in 0..count {
                declarations.push_str(&format!(
                    "pub schema Packet{index}\n  value: Int\nend\n\n"
                ));
                declarations.push_str(&format!("pub schema Alias{index} = Packet{index}\n"));
                fields.push_str(&format!("  field{index}: dep::Alias{index}\n"));
            }
            fields.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    &format!("use dep from \"example/dep\"\n\n{fields}"),
                )],
                vec![dependency_snapshot(
                    "example/dep",
                    &[("dep.veln", &declarations)],
                    ["dep.veln"],
                )],
            );

            crate::navigation::reset_schema_composition_index_work();
            crate::navigation::reset_schema_alias_declaration_visits();
            let _ = snapshot.navigation_index();
            let elapsed = started.elapsed();
            let (schema_visits, field_token_visits) =
                crate::navigation::schema_composition_index_work();
            assert_eq!(schema_visits, count);
            assert_eq!(
                crate::navigation::schema_alias_declaration_visits(),
                count * 4,
                "eligible aliases must be indexed once per declaration-resolution boundary, including export checks",
            );
            assert_eq!(field_token_visits % count, 0);
            let visits_per_field = field_token_visits / count;
            assert_eq!(
                *field_token_visits_per_field.get_or_insert(visits_per_field),
                visits_per_field,
            );
            eprintln!(
                "eligible schema-alias composition index: aliases={count} fields={count} elapsed={elapsed:?} schema_visits={schema_visits} field_token_visits={field_token_visits}"
            );
        }
    }

    #[test]
    fn standard_library_bare_schema_alias_composition_index_work_is_adjacent_linear() {
        for matching in [true, false] {
            let mut standard_body = String::from(
                "pub schema Packet\n  value: Int\nend\npub schema AliasPacket = Packet\n",
            );
            let mut fields = String::from("schema Host\n");
            for index in 0..400 {
                let name = if matching { "AliasPacket" } else { "MissingPacket" };
                fields.push_str(&format!("  field{index}: {name}\n"));
                standard_body.push_str(&format!("pub schema Noise{index}\n  value: Int\nend\n"));
            }
            fields.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &fields)])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", &standard_body)],
                    ["prelude.veln"],
                ));

            crate::navigation::reset_schema_composition_index_work();
            let _ = snapshot.navigation_index();
            let (_, field_token_visits) = crate::navigation::schema_composition_index_work();
            let (prelude_lookups, blocker_lookups) =
                crate::navigation::schema_composition_bare_lookup_work();
            assert!(
                field_token_visits <= 400 * 12,
                "bare composition indexing must stay linear: {field_token_visits}"
            );
            assert_eq!(prelude_lookups, 400, "each bare leaf uses one prelude lookup");
            assert!(
                blocker_lookups <= 400,
                "workspace blocker checks must remain bounded per bare leaf"
            );
            let selection = query_snapshot(&snapshot, "main.veln", 2, 18);
            if matching {
                assert_eq!(selection.unwrap().selected_symbol.name, "AliasPacket");
            } else {
                assert!(selection.is_none());
            }
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
    fn standard_library_schema_alias_package_source_occurrences_are_not_selectable() {
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

        assert!(query_snapshot(&snapshot, "prelude.veln", 8, 10).is_none());
    }

    #[test]
    fn local_type_names_block_bare_standard_library_schema_alias_fallback() {
        for local_declaration in [
            "type AliasPacket\n  Value\nend\n\n",
            "type Packet\n  Value\nend\npub type AliasPacket = Packet\n\n",
        ] {
            let source_text = format!(
                "{local_declaration}schema Host\n  nested: AliasPacket\nend\n"
            );
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)])
                .with_standard_library(standard_library_snapshot(
                    &[(
                        "prelude.veln",
                        "pub schema Packet\n  value: Int\nend\n\npub schema AliasPacket = Packet\n",
                    )],
                    ["prelude.veln"],
                ));
            assert!(query_snapshot(&snapshot, "main.veln", local_declaration.lines().count() + 2, 12)
                .is_none());
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
            let leaf_lines: Vec<_> = main
                .lines()
                .enumerate()
                .filter(|(_, text)| {
                    text.contains(&format!("wire::{name}"))
                        && !text.trim_start().starts_with("//")
                        && !text.trim_start().starts_with('"')
                })
                .map(|(line, _)| line + 1)
                .collect();
            assert_eq!(leaf_lines.len(), 5, "{name} must have every leaf role");
            let selections: Vec<_> = leaf_lines
                .iter()
                .map(|&line| {
                    (0..main.lines().nth(line - 1).unwrap().len())
                        .find_map(|column| {
                            query_snapshot(&snapshot, "main.veln", line, column)
                                .filter(|result| result.selected_symbol.name == name)
                        })
                        .unwrap_or_else(|| panic!("missing standard-library identity for {name} at line {line}"))
                })
                .collect::<Vec<_>>();
            let selected = &selections[0];
            assert_eq!(selected.selected_symbol.package_origin, Some(PackageOrigin::StandardLibrary));
            let expected = selections
                .iter()
                .map(|result| ("main.veln", result.selection.start.line, result.selection.start.column))
                .collect::<Vec<_>>();
            assert_eq!(locations(&selected.references), expected, "{name} must keep each exact leaf role");

            let selected_locations = locations(&selected.references);
            for other in ["Top", "Mid", "Packet", "Sibling"] {
                if other == name {
                    continue;
                }
                let other_line = main
                    .lines()
                    .enumerate()
                    .find_map(|(line, text)| text.contains(&format!("wire::{other}")).then_some(line + 1))
                    .expect("all chain identities must be selectable");
                let other_text = main.lines().nth(other_line - 1).unwrap();
                let other_selected = (0..other_text.len())
                    .find_map(|column| {
                        query_snapshot(&snapshot, "main.veln", other_line, column)
                            .filter(|result| result.selected_symbol.name == other)
                    })
                    .expect("all chain identities must be selectable");
                assert!(
                    selected_locations
                        .iter()
                        .all(|location| !locations(&other_selected.references).contains(location)),
                    "{name} and {other} must not exchange exact occurrence ranges"
                );
            }
        }
    }

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
                        text.find(selected).map(|column| (line + 1, column + 6))
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
        let field = query_snapshot(&snapshot, "main.veln", 2, 4);
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

    #[test]
    fn standard_library_schema_eligibility_and_import_failures_are_empty() {
        let cases: &[(&str, &str, &[&str], bool)] = &[
            (
                "wire.veln",
                "schema Packet\n  value: Int\nend\n",
                &["wire.veln"],
                false,
            ),
            (
                "wire.veln",
                "pub schema Packet\n  value: Int\nend\n",
                &[],
                false,
            ),
            (
                "wire.veln",
                "pub schema packet\n  value: Int\nend\n",
                &["wire.veln"],
                false,
            ),
            (
                "wire.veln",
                "pub schema Base\n  value: Int\nend\n\npub schema Packet = Base\n",
                &["wire.veln"],
                true,
            ),
            (
                "wire.veln",
                "pub schema Packet =\n",
                &["wire.veln"],
                false,
            ),
        ];
        for (path, body, exports, eligible) in cases {
            let main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Packet\n  repeated: Repeat(count, wire::Packet)\n  array: [wire::Packet; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Packet from view at byte_offset(0)?\n  encode wire::Packet from packet\nend\n";
            let snapshot = EffectiveProjectSnapshot::new(vec![source(
                "main.veln",
                main,
            )])
            .with_standard_library(standard_library_snapshot(
                &[(path, body)],
                exports.iter().copied(),
            ));
            let leaf_lines: Vec<_> = main
                .lines()
                .enumerate()
                .filter(|(_, text)| text.contains("wire::Packet"))
                .map(|(line, _)| line + 1)
                .collect();
            assert_eq!(leaf_lines.len(), 5, "{path}");
            for line in leaf_lines {
                let text = main.lines().nth(line - 1).unwrap();
                let result = (0..text.len()).find_map(|column| {
                    query_snapshot(&snapshot, "main.veln", line, column)
                        .filter(|selection| selection.selected_symbol.name == "Packet")
                });
                if *eligible {
                    assert_eq!(
                        locations(&result.expect("eligible standard-library alias").references),
                        [
                            ("main.veln", 5, 17),
                            ("main.veln", 6, 33),
                            ("main.veln", 7, 17),
                            ("main.veln", 11, 16),
                            ("main.veln", 12, 16),
                        ],
                        "{path} at line {line}"
                    );
                } else {
                    assert!(
                        result.is_none_or(|selection| selection.references.is_empty()),
                        "{path} at line {line}"
                    );
                }
            }
        }

        let ambiguous = EffectiveProjectSnapshot::new(vec![
            source("workspace_import.veln", "mod app\nuse workspace::wire\n"),
            source(
                "main.veln",
                concat!(
                    "mod app\n",
                    "use alpha::wire from \"std\"\n",
                    "use beta::wire from \"std\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        assert!(query_snapshot(&ambiguous, "main.veln", 5, 16).is_none());

        let exact = EffectiveProjectSnapshot::new(vec![
            source(
                "imports.veln",
                "mod app\nuse alpha::wire from \"std\"\nuse beta::wire from \"std\"\n",
            ),
            source(
                "main.veln",
                concat!(
                    "mod app\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode alpha::wire::Packet from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        let selected = query_snapshot(&exact, "main.veln", 4, 23)
            .expect("an exact standard-library import must beat an implicit collision");
        assert_eq!(locations(&selected.references), [("main.veln", 4, 23)]);
    }

    #[test]
    fn standard_library_schema_alias_eligibility_matrix_rejects_target_boundaries() {
        let cases = [
            (
                "private alias",
                "pub schema Packet\n  value: Int\nend\nschema Alias = Packet\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "unresolved target",
                "pub schema Alias = Missing\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "wrong-kind target",
                "pub type Wrong\n  Ready(Int)\nend\npub schema Alias = Wrong\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "private target",
                "schema Hidden\n  value: Int\nend\npub schema Alias = Hidden\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "cycle",
                "pub schema Alias = Other\npub schema Other = Alias\n",
                ["wire.veln"].as_slice(),
            ),
            (
                "non-exported source",
                "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n",
                [].as_slice(),
            ),
        ];

        for (name, body, exports) in cases {
            let main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Alias\n  repeated: Repeat(count, wire::Alias)\n  array: [wire::Alias; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Alias from view at byte_offset(0)?\n  encode wire::Alias from packet\nend\n";
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
                .with_standard_library(standard_library_snapshot(
                    &[("wire.veln", body)],
                    exports.iter().copied(),
                ));
            for (line, text) in main.lines().enumerate() {
                if let Some(column) = text.find("wire::Alias") {
                    let result = query_snapshot(&snapshot, "main.veln", line + 1, column + 6);
                    assert!(
                        result.as_ref().is_none_or(|selection| selection.references.is_empty()),
                        "{name} must remain a successful empty selection at {}:{}: {result:#?}",
                        line + 1,
                        column + 6
                    );
                }
            }
        }

        let ambiguous_main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Alias\n  repeated: Repeat(count, wire::Alias)\n  array: [wire::Alias; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Alias from view at byte_offset(0)?\n  encode wire::Alias from packet\nend\n";
        let ambiguous = EffectiveProjectSnapshot::new(vec![source("main.veln", ambiguous_main)])
        .with_standard_library(standard_library_snapshot(
            &[
                ("alpha/wire.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n"),
                ("beta/wire.veln", "pub schema Packet\n  value: Int\nend\npub schema Alias = Packet\n"),
            ],
            ["alpha/wire.veln", "beta/wire.veln"],
        ));
        for (line, text) in ambiguous_main.lines().enumerate() {
            if let Some(column) = text.find("wire::Alias") {
                assert!(
                    query_snapshot(&ambiguous, "main.veln", line + 1, column + 6)
                        .is_none_or(|result| result.references.is_empty()),
                    "ambiguous alias must be empty at {}:{}",
                    line + 1,
                    column + 6
                );
            }
        }

        let boundary_cases = [
            (
                "invalid-cased target",
                vec![(
                    "wire.veln",
                    "pub schema packet\n  value: Int\nend\npub schema Alias = packet\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "blocked nonterminal hop",
                vec![(
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\nschema Mid = Packet\npub schema Alias = Mid\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "target in non-exported source",
                vec![
                    (
                        "wire.veln",
                        "use hidden from \"std\"\npub schema Alias = hidden::Packet\n",
                    ),
                    ("hidden.veln", "pub schema Packet\n  value: Int\nend\n"),
                ],
                vec!["wire.veln"],
            ),
            (
                "invalid-cased alias",
                vec![(
                    "wire.veln",
                    "pub schema Packet\n  value: Int\nend\npub schema alias = Packet\n",
                )],
                vec!["wire.veln"],
            ),
            (
                "external-package target",
                vec![(
                    "wire.veln",
                    "use external from \"example/external\"\npub schema Alias = external::Packet\n",
                )],
                vec!["wire.veln"],
            ),
        ];
        for (name, sources, exports) in boundary_cases {
            let main = "use wire from \"std\"\n\nschema Host\n  count: UInt8\n  direct: wire::Alias\n  repeated: Repeat(count, wire::Alias)\n  array: [wire::Alias; count]\nend\n\nfn read(view: ByteView, packet: {value: Int}) -> ()\n  decode wire::Alias from view at byte_offset(0)?\n  encode wire::Alias from packet\nend\n";
            let snapshot = if name == "external-package target" {
                EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source("main.veln", main)],
                    vec![dependency_snapshot(
                        "example/external",
                        &[("external.veln", "pub schema Packet\n  value: Int\nend\n")],
                        ["external.veln"],
                    )],
                )
                .with_standard_library(standard_library_snapshot(&sources, exports))
            } else {
                EffectiveProjectSnapshot::new(vec![source("main.veln", main)])
                    .with_standard_library(standard_library_snapshot(&sources, exports))
            };
            for (line, text) in main.lines().enumerate() {
                if let Some(column) = text.find("wire::Alias") {
                    let result = query_snapshot(&snapshot, "main.veln", line + 1, column + 6);
                    assert!(
                        result.as_ref().is_none_or(|selection| selection.references.is_empty()),
                        "{name} must remain a successful empty selection at {}:{}: {result:#?}",
                        line + 1,
                        column + 6
                    );
                }
            }
        }
    }

    #[test]
    fn standard_library_schema_invalid_casing_alias_blockers_and_recovered_leaves_are_empty() {
        let invalid_casing = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "fn read(view: ByteView) -> ()\n",
                "  decode wire::packet from view at byte_offset(0)?\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));
        let invalid = query_snapshot(&invalid_casing, "main.veln", 4, 16)
            .expect("invalid-cased standard schema should retain its definition");
        assert!(matches!(invalid.definition.source, NavigationSource::Package { .. }));
        assert!(invalid.references.is_empty());

        let alias_collision = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "use wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[
                (
                    "alias/wire.veln",
                    "mod wire\npub schema Base\n  value: Int\nend\npub schema Packet = Base\n",
                ),
                (
                    "fallback/wire.veln",
                    "mod wire\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["alias/wire.veln", "fallback/wire.veln"],
        ));
        assert!(query_snapshot(&alias_collision, "main.veln", 4, 16)
            .is_none_or(|result| result.references.is_empty()));

        let recovered_alias_collision = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "use wire from \"std\"\n\nfn read(view: ByteView) -> ()\n  decode wire::Packet from view at byte_offset(0)?\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[
                (
                    "wire/valid.veln",
                    "mod wire\npub schema Packet\n  value: Int\nend\n",
                ),
                ("wire/recovered.veln", "mod wire\npub schema Packet =\n"),
            ],
            ["wire/valid.veln", "wire/recovered.veln"],
        ));
        assert!(query_snapshot(&recovered_alias_collision, "main.veln", 4, 16)
            .is_none_or(|result| result.references.is_empty()));

        let recovered_leaves = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use wire from \"std\"\n\n",
                "schema Host\n",
                "  count: UInt8\n",
                "  direct: wire::Packet\n",
                "  bad_repeat: Repeat(, wire::Packet)\n",
                "  bad_array: [wire::Packet;]\n",
                "end\n\n",
                "fn broken(view: ByteView) -> ()\n",
                "  decode wire::Packet junk from view at byte_offset(0)?\n",
                "  encode wire::Packet junk from {value: 1}\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[("wire.veln", "pub schema Packet\n  value: Int\nend\n")],
            ["wire.veln"],
        ));
        let valid = query_snapshot(&recovered_leaves, "main.veln", 5, 20)
            .expect("the valid composition leaf should remain navigable");
        assert_eq!(locations(&valid.references), [("main.veln", 5, 17)]);
        for (line, column) in [(6, 31), (7, 25), (11, 16), (12, 16)] {
            if let Some(result) = query_snapshot(&recovered_leaves, "main.veln", line, column)
            {
                assert!(result.references.is_empty(), "recovered leaf at {line}:{column}");
            }
        }
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
    fn dependency_schema_imports_unify_all_leaf_roles_across_explicit_module_sources() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "import.veln",
                    "mod app\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "composition.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Host\n",
                        "  count: UInt8\n",
                        "  direct: lib::wire::Packet\n",
                        "  repeated: Repeat(count, wire::Packet)\n",
                        "  canonical: [wire::Packet; count]\n",
                        "end\n",
                    ),
                ),
                source(
                    "decode.veln",
                    concat!(
                        "mod app\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "encode.veln",
                    concat!(
                        "mod app\n\n",
                        "fn write(packet: {value: Int}) -> ()\n",
                        "  encode lib::wire::Packet from packet\n",
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
        let expected = [
            ("composition.veln", 5, 22),
            ("composition.veln", 6, 33),
            ("composition.veln", 7, 21),
            ("decode.veln", 4, 16),
            ("encode.veln", 4, 21),
        ];

        for (path, line, column) in expected {
            let result = query_snapshot(&snapshot, path, line, column).unwrap();
            assert_eq!(locations(&result.references), expected, "{path}:{line}");
        }
    }

    #[test]
    fn invalid_module_wide_dependency_schema_imports_block_composition_and_operations() {
        let cases = [
            (
                "duplicate import",
                vec![
                    ("a.veln", "mod app\nuse dep from \"first/dep\"\n"),
                    ("b.veln", "mod app\nuse dep from \"first/dep\"\n"),
                ],
                "dep",
                "Alias",
                16,
                15,
            ),
            (
                "recovered import",
                vec![(
                    "a.veln",
                    "mod app\nuse dep from \"first/dep\" unexpected\n",
                )],
                "dep",
                "Alias",
                16,
                15,
            ),
            (
                "conflicting exact imports",
                vec![
                    ("a.veln", "mod app\nuse shared from \"first/dep\"\n"),
                    ("b.veln", "mod app\nuse shared from \"second/dep\"\n"),
                    (
                        "c.veln",
                        "mod app\nuse fallback::shared from \"first/dep\"\n",
                    ),
                ],
                "shared",
                "Packet",
                19,
                18,
            ),
            (
                "ambiguous implicit alias",
                vec![
                    (
                        "a.veln",
                        "mod app\nuse alpha::wire from \"first/dep\"\n",
                    ),
                    (
                        "b.veln",
                        "mod app\nuse beta::wire from \"second/dep\"\n",
                    ),
                ],
                "wire",
                "Alias",
                17,
                16,
            ),
        ];

        for (name, imports, qualifier, target, composition_column, operation_column) in cases {
            for reverse in [false, true] {
                let mut sources = imports
                    .iter()
                    .map(|(path, text)| source(path, text))
                    .collect::<Vec<_>>();
                sources.push(source(
                    "selection.veln",
                    &format!(
                        concat!(
                            "mod app\n",
                            "schema Host\n",
                            "  nested: {0}::{1}\n",
                            "end\n",
                            "fn read(view: ByteView) -> ()\n",
                            "  decode {0}::{1} from view at byte_offset(0)?\n",
                            "end\n",
                        ),
                        qualifier,
                        target,
                    ),
                ));
                if reverse {
                    sources.reverse();
                }
                let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                    sources,
                    vec![
                        dependency_snapshot(
                            "first/dep",
                            &[
                                (
                                    "dep.veln",
                                    "mod dep\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                                (
                                    "shared.veln",
                                    "mod shared\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                                (
                                    "fallback/shared.veln",
                                    "mod fallback::shared\npub schema Packet\n  value: Int\nend\n",
                                ),
                                (
                                    "alpha/wire.veln",
                                    "mod alpha::wire\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                            ],
                            [
                                "dep.veln",
                                "shared.veln",
                                "fallback/shared.veln",
                                "alpha/wire.veln",
                            ],
                        ),
                        dependency_snapshot(
                            "second/dep",
                            &[
                                (
                                    "shared.veln",
                                    "mod shared\npub schema Packet\n  value: Int\nend\n",
                                ),
                                (
                                    "beta/wire.veln",
                                    "mod beta::wire\npub schema Packet\n  value: Int\nend\npub schema Mid = Packet\npub schema Alias = Mid\n",
                                ),
                            ],
                            ["shared.veln", "beta/wire.veln"],
                        ),
                    ],
                );

                assert!(
                    query_snapshot(
                        &snapshot,
                        "selection.veln",
                        3,
                        composition_column,
                    )
                    .is_none(),
                    "{name}, reverse={reverse}: composition",
                );
                assert!(
                    query_snapshot(
                        &snapshot,
                        "selection.veln",
                        6,
                        operation_column,
                    )
                    .is_none(),
                    "{name}, reverse={reverse}: operation",
                );
            }
        }
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
                    ("boundaries.veln", 8, 17),
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
        let boundary = query_snapshot(&snapshot, "boundaries.veln", 8, 18).unwrap();
        assert_eq!(
            locations(&boundary.references),
            [
                ("boundaries.veln", 8, 17),
                ("main.veln", 4, 21),
                ("main.veln", 5, 16),
                ("other.veln", 4, 16),
            ]
        );
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
                    "pub schema Mid = Packet\n",
                    "pub schema Alias = Mid\n",
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
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
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
                [
                    ("read.veln", 4, 16),
                    ("read.veln", 8, 16),
                    ("write.veln", 4, 16),
                ]
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
                        "pub schema Mid = Packet\n",
                        "pub schema Alias = Mid\n",
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
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
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
                    "pub schema Mid = Packet\n",
                    "pub schema Alias = Mid\n",
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
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
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
                        "pub schema Mid = Packet\n",
                        "pub schema Alias = Mid\n",
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
                    "schema Frame\n",
                    "  value: dep::Alias\n",
                    "end\n\n",
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
                query_snapshot(&snapshot, "operation.veln", 4, 15).is_none(),
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
        assert_eq!(locations(&standard.references), [("main.veln", 10, 16)]);
        let invalid_casing = query_snapshot(&snapshot, "main.veln", 11, 18).unwrap();
        assert!(matches!(
            invalid_casing.definition.source,
            NavigationSource::Package { .. }
        ));
        assert!(invalid_casing.references.is_empty());
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
    fn ineligible_dependency_schema_alias_composition_leaves_stay_empty() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "lib/wire.veln",
                    concat!(
                        "mod lib::wire\n",
                        "use private\n\n",
                        "pub schema PrivateTarget = private::Packet\n",
                    ),
                ),
                (
                    "private.veln",
                    "mod private\n\npub schema Packet\n  value: Int\nend\n",
                ),
            ],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use lib::wire from \"example/dep\"\n\n",
                    "schema Frame\n",
                    "  count: UInt8\n",
                    "  direct: lib::wire::PrivateTarget\n",
                    "  repeated: Repeat(count, lib::wire::PrivateTarget)\n",
                    "  array: [lib::wire::PrivateTarget; count]\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(5, 24), (6, 40), (7, 23)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "ineligible alias composition leaf at {line}:{column} must not be selectable"
            );
        }
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
                    "mod dep\n\npub schema Packet\n  value: Int\nend\n",
                ),
                (
                    "hidden.veln",
                    concat!(
                        "mod dep\n\n",
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
                    concat!(
                        "mod facade\n\n",
                        "pub schema Intermediate = core::Packet\n",
                        "pub schema Alias = Intermediate\n",
                    ),
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
                true,
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
                    "mod facade\n\npub schema Mid = nested::core::Packet\npub schema Alias = Mid\n",
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
