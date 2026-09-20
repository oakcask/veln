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
