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

    #[test]
    fn standard_library_bare_schema_alias_operation_lookup_is_adjacent_linear() {
        for count in [200, 400] {
            let mut standard_body =
                String::from("pub schema Packet\n  value: Int\nend\n");
            let mut operations = String::from("fn read(view: ByteView) -> ()\n");
            for index in 0..count {
                standard_body.push_str(&format!("pub schema Noise{index} = Packet\n"));
                operations.push_str(
                    "  decode AliasPacket from view at byte_offset(0)?\n",
                );
            }
            standard_body.push_str("pub schema AliasPacket = Packet\n");
            operations.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &operations)])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", &standard_body)],
                    ["prelude.veln"],
                ));

            let _ = snapshot.navigation_index();
            crate::navigation::reset_schema_operation_bare_lookup_work();
            let started = std::time::Instant::now();
            let selected = query_snapshot(&snapshot, "main.veln", 2, 10).unwrap();
            let elapsed = started.elapsed();
            assert_eq!(selected.references.len(), count);
            let (prelude_lookups, blocker_lookups, leaf_lookups) =
                crate::navigation::schema_operation_bare_lookup_work();
            assert_eq!(prelude_lookups, count + 1);
            assert_eq!(blocker_lookups, count + 1);
            assert_eq!(leaf_lookups, count + 1);
            eprintln!(
                "bare operation lookup: aliases={count} occurrences={count} elapsed={elapsed:?} prelude_lookups={prelude_lookups} blocker_lookups={blocker_lookups} leaf_lookups={leaf_lookups}"
            );
        }
    }

    #[test]
    fn dependency_schema_operation_lookup_uses_qualified_target_index() {
        for count in [100, 200, 400] {
            let mut declarations = String::new();
            for index in 0..count {
                declarations.push_str(&format!(
                    "pub schema Packet{index}\n  value: Int\nend\n\n"
                ));
            }
            let mut operations = String::from(
                "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n",
            );
            for _ in 0..count {
                operations.push_str(
                    "  decode dep::Packet0 from view at byte_offset(0)?\n",
                );
            }
            operations.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &operations)],
                vec![dependency_snapshot(
                    "example/dep",
                    &[("dep.veln", &declarations)],
                    ["dep.veln"],
                )],
            );

            let _ = snapshot.navigation_index();
            crate::navigation::reset_schema_operation_qualified_lookup_work();
            let started = std::time::Instant::now();
            let selected = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
            let elapsed = started.elapsed();
            assert_eq!(selected.references.len(), count);
            let (candidate_visits, target_lookups) =
                crate::navigation::schema_operation_qualified_lookup_work();
            assert_eq!(
                target_lookups,
                count + 2,
                "selection probes the alias and schema indexes, then each reference probes the schema index once",
            );
            assert!(
                candidate_visits <= count + 1,
                "qualified lookup must not rescan all declarations: {candidate_visits}",
            );
            eprintln!(
                "qualified schema operation lookup: declarations={count} occurrences={count} elapsed={elapsed:?} target_lookups={target_lookups} candidate_visits={candidate_visits}"
            );
        }
    }

    #[test]
    fn dependency_schema_alias_operation_lookup_uses_qualified_target_index() {
        for count in [100, 200, 400] {
            let mut declarations =
                String::from("pub schema Packet\n  value: Int\nend\n\n");
            for index in 0..count {
                declarations.push_str(&format!(
                    "pub schema Noise{index} = Packet\n"
                ));
            }
            declarations.push_str("pub schema Target = Packet\n");
            let mut operations = String::from(
                "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n",
            );
            for _ in 0..count {
                operations.push_str(
                    "  decode dep::Target from view at byte_offset(0)?\n",
                );
            }
            operations.push_str("end\n");
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &operations)],
                vec![dependency_snapshot(
                    "example/dep",
                    &[("dep.veln", &declarations)],
                    ["dep.veln"],
                )],
            );

            let _ = snapshot.navigation_index();
            crate::navigation::reset_schema_operation_qualified_lookup_work();
            let started = std::time::Instant::now();
            let selected = query_snapshot(&snapshot, "main.veln", 4, 16).unwrap();
            let elapsed = started.elapsed();
            assert_eq!(selected.references.len(), count);
            let (candidate_visits, target_lookups) =
                crate::navigation::schema_operation_qualified_lookup_work();
            assert_eq!(target_lookups, count + 1);
            assert_eq!(candidate_visits, count + 1);
            eprintln!(
                "qualified schema-alias operation lookup: declarations={count} occurrences={count} elapsed={elapsed:?} target_lookups={target_lookups} candidate_visits={candidate_visits}"
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
