mod navigation_module_value_tests {
    use super::*;

    fn module_value_sources() -> Vec<SourceFile> {
        vec![
            source(
                "values.veln",
                "pub let answer: Int = 42\nlet direct: Int = answer\n",
            ),
            source("other.veln", "pub let answer: Int = 42\n"),
            source(
                "functions.veln",
                "pub fn answer() -> Int\n  42\nend\n",
            ),
            source(
                "main.veln",
                concat!(
                    "use values\n",
                    "use other as equal\n",
                    "use functions as callable\n\n",
                    "let qualified: Int = values::answer\n",
                    "let equal_data: Int = equal::answer\n",
                    "let same_spelling: Int = callable::answer()\n",
                ),
            ),
            source(
                "alias.veln",
                "use values as config\n\nlet aliased: Int = config::answer\n",
            ),
        ]
    }

    #[test]
    fn module_value_navigation_uses_one_identity_across_import_spellings() {
        let snapshot = EffectiveProjectSnapshot::new(module_value_sources());
        for (path, line, column) in [
            ("values.veln", 2, 19),
            ("main.veln", 5, 30),
            ("alias.veln", 3, 29),
        ] {
            let result = query_snapshot(&snapshot, path, line, column)
                .unwrap_or_else(|| panic!("missing navigation at {path}:{line}:{column}"));
            assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
            assert_location(&result.definition, "values.veln", 1, 9);
            assert_eq!(
                locations(&result.references),
                [
                    ("alias.veln", 3, 28),
                    ("main.veln", 5, 30),
                    ("values.veln", 2, 19),
                ]
            );
            assert!(validate_rename(&result, "result").is_ok());
        }

        let declaration = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        assert_eq!(declaration.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_eq!(declaration.references.len(), 3);
    }

    #[test]
    fn equal_initializer_and_same_spelled_function_keep_distinct_identities() {
        let snapshot = EffectiveProjectSnapshot::new(module_value_sources());
        let equal_data = query_snapshot(&snapshot, "main.veln", 6, 31).unwrap();
        assert_eq!(equal_data.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_location(&equal_data.definition, "other.veln", 1, 9);
        assert_eq!(locations(&equal_data.references), [("main.veln", 6, 30)]);

        let function = query_snapshot(&snapshot, "main.veln", 7, 39).unwrap();
        assert_eq!(function.selected_symbol.kind, SymbolKind::Function);
        assert_location(&function.definition, "functions.veln", 1, 8);
    }

    #[test]
    fn package_owned_module_values_keep_distinct_identities() {
        let first = dependency_snapshot(
            "one/pkg",
            &[("shared.veln", "pub let answer: Int = 42\n")],
            ["shared.veln"],
        );
        let second = dependency_snapshot(
            "two/pkg",
            &[("shared.veln", "pub let answer: Int = 42\n")],
            ["shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use shared as first from \"one/pkg\"\n",
                    "use shared as second from \"two/pkg\"\n\n",
                    "let left: Int = first::answer\n",
                    "let right: Int = second::answer\n",
                ),
            )],
            vec![first, second],
        );

        let first = query_snapshot(&snapshot, "main.veln", 4, 25).unwrap();
        let second = query_snapshot(&snapshot, "main.veln", 5, 27).unwrap();
        assert_eq!(first.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_eq!(second.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_ne!(first.definition.source, second.definition.source);
        assert_eq!(locations(&first.references), [("main.veln", 4, 24)]);
        assert_eq!(locations(&second.references), [("main.veln", 5, 26)]);
    }

    #[test]
    fn dependency_function_and_module_value_navigation_stays_separate() {
        let data = dependency_snapshot(
            "data/pkg",
            &[("shared.veln", "pub let answer: Int = 42\n")],
            ["shared.veln"],
        );
        let callable = dependency_snapshot(
            "callable/pkg",
            &[("shared.veln", "pub fn answer() -> Int\n  42\nend\n")],
            ["shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use shared as data from \"data/pkg\"\n",
                    "use shared as callable from \"callable/pkg\"\n\n",
                    "let value: Int = data::answer\n",
                    "let result: Int = callable::answer()\n",
                ),
            )],
            vec![data, callable],
        );

        let value = query_snapshot(&snapshot, "main.veln", 4, 27).unwrap();
        assert_eq!(value.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_eq!(locations(&value.references), [("main.veln", 4, 24)]);

        let function = query_snapshot(&snapshot, "main.veln", 5, 34).unwrap();
        assert_eq!(function.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(locations(&function.references), [("main.veln", 5, 29)]);
        assert_ne!(value.definition.source, function.definition.source);
    }

    #[test]
    fn standard_library_module_values_do_not_enter_navigation() {
        let standard_library = standard_library_snapshot(
            &[("values.veln", "pub let answer: Int = 42\n")],
            ["values.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "use values from \"std\"\n\n",
                    "let direct: Int = answer\n",
                    "let qualified: Int = values::answer\n",
                ),
            ),
            source(
                "alias.veln",
                concat!(
                    "use values as config from \"std\"\n\n",
                    "let aliased: Int = config::answer\n",
                ),
            ),
        ])
        .with_standard_library(standard_library);

        for (path, line, column) in [
            ("values.veln", 1, 10),
            ("main.veln", 3, 19),
            ("main.veln", 4, 30),
            ("alias.veln", 3, 29),
        ] {
            assert!(
                query_snapshot(&snapshot, path, line, column).is_none(),
                "standard-library module value navigated at {path}:{line}:{column}"
            );
        }
    }

    #[test]
    fn package_occurrences_do_not_match_colliding_workspace_spans() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                ("values.veln", "pub let answer: Int = 42\n"),
                (
                    "shared.veln",
                    "use values\n\nlet selected: Int = values::answer\n",
                ),
            ],
            ["values.veln", "shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source("values.veln", "let x = answer\n"),
                source(
                    "shared.veln",
                    "use foobar\n\nlet selected: Int = foobar::answer\n",
                ),
                source(
                    "main.veln",
                    concat!(
                        "use values from \"example/pkg\"\n\n",
                        "let selected: Int = values::answer\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        assert!(query_snapshot(&snapshot, "values.veln", 1, 10).is_none());
        assert!(query_snapshot(&snapshot, "shared.veln", 3, 30).is_none());

        let valid = query_snapshot(&snapshot, "main.veln", 3, 30).unwrap();
        assert_eq!(valid.selected_symbol.kind, SymbolKind::ModuleValue);
        assert!(matches!(valid.definition.source, NavigationSource::Package { .. }));
        assert_eq!(locations(&valid.references), [("main.veln", 3, 29)]);
    }

    #[test]
    fn invalid_module_values_do_not_enter_navigation() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("invalid.veln", "pub let answer: String = 42\n"),
            source(
                "main.veln",
                "use invalid\n\nlet selected: String = invalid::answer\n",
            ),
        ]);

        assert!(query_snapshot(&snapshot, "invalid.veln", 1, 10).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 3, 31).is_none());
    }

    #[test]
    fn result_binding_does_not_hide_a_same_named_module_value_in_the_body() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "values.veln",
            concat!(
                "pub let answer: Int = 42\n",
                "fn read() -> answer: Int\n",
                "ensure answer == 42\n",
                "  answer\n",
                "end\n",
            ),
        )]);

        for (line, column) in [(1, 10), (4, 4)] {
            let result = query_snapshot(&snapshot, "values.veln", line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
            assert_location(&result.definition, "values.veln", 1, 9);
            assert_eq!(locations(&result.references), [("values.veln", 4, 3)]);
            assert!(validate_rename_in_snapshot(&snapshot, &result, "renamed").is_ok());
        }
    }

    #[test]
    fn qualified_module_value_selection_precedes_same_named_lexical_bindings() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("values.veln", "pub let answer: Int = 42\n"),
            source(
                "main.veln",
                concat!(
                    "use values\n\n",
                    "fn from_parameter(answer: Int) -> Int\n",
                    "  values::answer + answer\n",
                    "end\n\n",
                    "fn from_local() -> Int\n",
                    "  let answer = 1\n",
                    "  values::answer + answer\n",
                    "end\n",
                ),
            ),
        ]);

        for (line, qualified_column, bare_column) in [(4, 12, 21), (9, 12, 21)] {
            let result = query_snapshot(&snapshot, "main.veln", line, qualified_column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
            assert_location(&result.definition, "values.veln", 1, 9);
            assert_eq!(
                locations(&result.references),
                [("main.veln", 4, 11), ("main.veln", 9, 11)]
            );
            assert!(validate_rename_in_snapshot(&snapshot, &result, "renamed").is_ok());

            let lexical = query_snapshot(&snapshot, "main.veln", line, bare_column).unwrap();
            assert_eq!(lexical.selected_symbol.kind, SymbolKind::ValueBinding);
        }

        let renamed = EffectiveProjectSnapshot::new(vec![
            source("values.veln", "pub let renamed: Int = 42\n"),
            source(
                "main.veln",
                concat!(
                    "use values\n\n",
                    "fn from_parameter(answer: Int) -> Int\n",
                    "  values::renamed + answer\n",
                    "end\n\n",
                    "fn from_local() -> Int\n",
                    "  let answer = 1\n",
                    "  values::renamed + answer\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&renamed, "main.veln", 4, 12).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_location(&result.definition, "values.veln", 1, 9);
    }

    #[test]
    fn module_value_rename_rejects_shared_namespace_and_local_capture_conflicts() {
        for (source_text, requested_name, conflict_line, conflict_column) in [
            (
                "pub let answer: Int = 42\npub let result: Int = 1\n",
                "result",
                2,
                9,
            ),
            (
                "pub let answer: Int = 42\npub fn result() -> Int\n  1\nend\n",
                "result",
                2,
                8,
            ),
        ] {
            let snapshot = EffectiveProjectSnapshot::new(vec![source("values.veln", source_text)]);
            let result = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
            assert_rename_conflict(
                validate_rename_in_snapshot(&snapshot, &result, requested_name).unwrap_err(),
                RenameNameClass::ValueBinding,
                requested_name,
                "values.veln",
                conflict_line,
                conflict_column,
            );
        }

        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "values.veln",
            concat!(
                "pub let answer: Int = 42\n",
                "fn read() -> Int\n",
                "  let result = 1\n",
                "  answer\n",
                "end\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "result").unwrap_err(),
            RenameNameClass::ValueBinding,
            "result",
            "values.veln",
            3,
            7,
        );
    }

    #[test]
    fn module_value_rename_rejects_consumer_capture_and_import_ambiguity() {
        for consumer_declaration in [
            "let target: Int = 1\n",
            "fn target() -> Int\n  1\nend\n",
        ] {
            let snapshot = EffectiveProjectSnapshot::new(vec![
                source("values.veln", "pub let answer: Int = 42\n"),
                source(
                    "main.veln",
                    &format!(
                        "use values\n{consumer_declaration}fn read() -> Int\n  answer\nend\n"
                    ),
                ),
            ]);
            let result = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
            assert_rename_conflict(
                validate_rename_in_snapshot(&snapshot, &result, "target").unwrap_err(),
                RenameNameClass::ValueBinding,
                "target",
                "main.veln",
                2,
                if consumer_declaration.starts_with("let") { 5 } else { 4 },
            );
        }

        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub let target: Int = 1\n"),
            source("right.veln", "pub let answer: Int = 42\n"),
            source(
                "main.veln",
                "use left\nuse right\n\nfn read() -> Int\n  target\nend\n",
            ),
        ]);
        let result = query_snapshot(&snapshot, "right.veln", 1, 10).unwrap();
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "target").unwrap_err(),
            RenameNameClass::ValueBinding,
            "target",
            "left.veln",
            1,
            9,
        );

        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "left.veln",
                "pub fn target() -> Int\n  1\nend\n",
            ),
            source("right.veln", "pub let answer: Int = 42\n"),
            source(
                "main.veln",
                "use left\nuse right\n\nfn read() -> Int\n  target()\nend\n",
            ),
        ]);
        let result = query_snapshot(&snapshot, "right.veln", 1, 10).unwrap();
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "target").unwrap_err(),
            RenameNameClass::ValueBinding,
            "target",
            "left.veln",
            1,
            8,
        );

        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub let target: Int = 1\n"),
            source("right.veln", "pub let answer: Int = 42\n"),
            source(
                "main.veln",
                "use left\nuse right as config\n\nfn read() -> Int\n  target + config::answer\nend\n",
            ),
        ]);
        let result = query_snapshot(&snapshot, "right.veln", 1, 10).unwrap();
        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());
    }

    #[test]
    fn module_value_rename_keeps_imported_function_and_value_candidates_distinct() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("data.veln", "pub let target: Int = 1\n"),
            source("callable.veln", "pub fn target() -> Int\n  2\nend\n"),
            source("values.veln", "pub let answer: Int = 42\n"),
            source(
                "main.veln",
                concat!(
                    "use data\n",
                    "use callable\n",
                    "use values\n\n",
                    "fn read() -> Int\n",
                    "  answer + target()\n",
                    "end\n",
                ),
            ),
        ]);
        let selected = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();

        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &selected, "target").unwrap_err(),
            RenameNameClass::ValueBinding,
            "target",
            "data.veln",
            1,
            9,
        );
    }

    #[test]
    fn module_value_rename_treats_bare_dependency_exports_as_consumer_conflicts() {
        for dependency_source in [
            "pub let target: Int = 1\n",
            "pub fn target() -> Int\n  1\nend\n",
        ] {
            let dependency = dependency_snapshot(
                "example/pkg",
                &[("dependency.veln", dependency_source)],
                ["dependency.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![
                    source("values.veln", "pub let answer: Int = 42\n"),
                    source(
                        "main.veln",
                        concat!(
                            "use dependency from \"example/pkg\"\n",
                            "use values\n\n",
                            "fn read() -> Int\n",
                            "  answer + target\n",
                            "end\n",
                        ),
                    ),
                ],
                vec![dependency],
            );
            let selected = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
            let failure =
                validate_rename_in_snapshot(&snapshot, &selected, "target").unwrap_err();
            assert_eq!(failure.code, "rename.conflict");
            let RenameFailureKind::Conflict {
                conflicting_declaration,
                ..
            } = failure.kind
            else {
                panic!("rename failure was not a conflict");
            };
            assert!(matches!(
                conflicting_declaration.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(conflicting_declaration.span.file.as_str(), "dependency.veln");
        }

        let first = dependency_snapshot(
            "one/pkg",
            &[("shared.veln", "pub let target: Int = 1\n")],
            ["shared.veln"],
        );
        let second = dependency_snapshot(
            "two/pkg",
            &[("shared.veln", "pub let target: Int = 2\n")],
            ["shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source("values.veln", "pub let answer: Int = 42\n"),
                source(
                    "main.veln",
                    concat!(
                        "use shared from \"one/pkg\"\n",
                        "use values\n\n",
                        "fn read() -> Int\n",
                        "  answer + target\n",
                        "end\n",
                    ),
                ),
            ],
            vec![first, second],
        );
        let selected = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        let failure = validate_rename_in_snapshot(&snapshot, &selected, "target").unwrap_err();
        let RenameFailureKind::Conflict {
            conflicting_declaration,
            ..
        } = failure.kind
        else {
            panic!("rename failure was not a conflict");
        };
        let NavigationSource::Package { uri } = &conflicting_declaration.source else {
            panic!("dependency conflict did not retain package provenance");
        };
        assert!(uri.contains("one%2Fpkg"), "{uri}");
        assert!(!uri.contains("two%2Fpkg"), "{uri}");

        let dependency = dependency_snapshot(
            "example/pkg",
            &[("dependency.veln", "pub let target: Int = 1\n")],
            ["dependency.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source("values.veln", "pub let answer: Int = 42\n"),
                source(
                    "consumer_imports.veln",
                    concat!(
                        "mod consumer\n",
                        "use dependency from \"example/pkg\"\n",
                    ),
                ),
                source(
                    "consumer_read.veln",
                    concat!(
                        "mod consumer\n",
                        "use values\n\n",
                        "fn read() -> Int\n",
                        "  answer + target\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );
        let selected = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        let failure = validate_rename_in_snapshot(&snapshot, &selected, "target").unwrap_err();
        assert_eq!(failure.code, "rename.conflict");
    }

    #[test]
    fn module_value_owner_precedence_allows_an_imported_replacement_name() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("imported.veln", "pub let target: Int = 1\n"),
            source(
                "values.veln",
                concat!(
                    "use imported\n\n",
                    "pub let answer: Int = 42\n",
                    "fn read() -> Int\n",
                    "  answer\n",
                    "end\n",
                ),
            ),
        ]);
        let selected = query_snapshot(&snapshot, "values.veln", 3, 10).unwrap();
        assert!(validate_rename_in_snapshot(&snapshot, &selected, "target").is_ok());

        let renamed = EffectiveProjectSnapshot::new(vec![
            source("imported.veln", "pub let target: Int = 1\n"),
            source(
                "values.veln",
                concat!(
                    "use imported\n\n",
                    "pub let target: Int = 42\n",
                    "fn read() -> Int\n",
                    "  target\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&renamed, "values.veln", 5, 4).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_location(&result.definition, "values.veln", 3, 9);
        assert_eq!(locations(&result.references), [("values.veln", 5, 3)]);
    }

    #[test]
    fn function_rename_rejects_module_value_namespace_conflict() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "values.veln",
            concat!(
                "pub fn answer() -> Int\n",
                "  42\n",
                "end\n",
                "pub let result: Int = 1\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "values.veln", 1, 9).unwrap();
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "result").unwrap_err(),
            RenameNameClass::Function,
            "result",
            "values.veln",
            4,
            9,
        );
    }

    fn measured_module_value_projection_work(
        count: usize,
    ) -> crate::navigation::ModuleValueProjectionWork {
        let mut sources = (0..count)
            .map(|index| {
                source(
                    &format!("module_{index}.veln"),
                    &format!("pub let value_{index}: Int = {index}\n"),
                )
            })
            .collect::<Vec<_>>();
        let imports = (0..count)
            .map(|index| format!("use module_{index}\n"))
            .collect::<String>();
        let reads = (0..count)
            .map(|index| format!("  let local_{index} = module_{index}::value_{index}\n"))
            .collect::<String>();
        sources.push(source(
            "main.veln",
            &format!("{imports}\nfn read() -> Int\n{reads}  0\nend\n"),
        ));

        crate::navigation::reset_module_value_projection_work();
        let snapshot = EffectiveProjectSnapshot::new(sources);
        assert!(query_snapshot(&snapshot, "main.veln", count + 3, 28).is_some());
        crate::navigation::module_value_projection_work()
    }

    #[test]
    fn module_value_file_projection_work_is_adjacent_linear() {
        let smaller = measured_module_value_projection_work(32);
        let larger = measured_module_value_projection_work(64);

        assert_eq!(smaller.file_index_entries, 33, "{smaller:#?}");
        assert_eq!(larger.file_index_entries, 65, "{larger:#?}");
        assert_eq!(smaller.workspace_path_entries, 33, "{smaller:#?}");
        assert_eq!(larger.workspace_path_entries, 65, "{larger:#?}");
        assert_eq!(smaller.declaration_lookups, 32, "{smaller:#?}");
        assert_eq!(larger.declaration_lookups, 64, "{larger:#?}");
        assert_eq!(smaller.reference_lookups, 32, "{smaller:#?}");
        assert_eq!(larger.reference_lookups, 64, "{larger:#?}");
    }

    fn measured_module_value_rename_validation_work(
        count: usize,
    ) -> crate::navigation::ModuleValueRenameValidationWork {
        let reads = (0..count)
            .map(|index| format!("  let local_{index} = answer\n"))
            .collect::<String>();
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "values.veln",
            &format!(
                "pub let answer: Int = 42\nfn read() -> Int\n{reads}  answer\nend\n"
            ),
        )]);
        let result = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        assert_eq!(result.references.len(), count + 1);

        reset_module_value_rename_validation_work();
        let started = std::time::Instant::now();
        assert!(validate_rename_in_snapshot(&snapshot, &result, "renamed").is_ok());
        eprintln!(
            "module value rename validation: {} edits in {:?}",
            count + 2,
            started.elapsed()
        );
        module_value_rename_validation_work()
    }

    #[test]
    fn module_value_rename_validation_work_is_adjacent_linear() {
        let smaller = measured_module_value_rename_validation_work(1000);
        let larger = measured_module_value_rename_validation_work(2000);

        assert_eq!(smaller.file_lookups, 2004, "{smaller:#?}");
        assert_eq!(larger.file_lookups, 4004, "{larger:#?}");
        assert!(larger.token_comparisons <= smaller.token_comparisons * 3, "{smaller:#?} {larger:#?}");
        assert_eq!(smaller.scope_collections, 1, "{smaller:#?}");
        assert_eq!(larger.scope_collections, 1, "{larger:#?}");
        assert!(
            larger.scope_lookup_comparisons <= smaller.scope_lookup_comparisons * 3,
            "{smaller:#?} {larger:#?}"
        );
    }

    fn measured_module_value_consumer_candidate_work(
        count: usize,
    ) -> crate::navigation::ModuleValueRenameValidationWork {
        let mut sources = vec![source("values.veln", "pub let answer: Int = 42\n")];
        for index in 0..count {
            sources.push(source(
                &format!("unrelated_{index}.veln"),
                &format!("pub let value_{index}: Int = {index}\n"),
            ));
            sources.push(source(
                &format!("consumer_{index}.veln"),
                &format!(
                    "use values\nuse unrelated_{index}\n\nfn read() -> Int\n  answer\nend\n"
                ),
            ));
        }
        let snapshot = EffectiveProjectSnapshot::new(sources);
        let selected = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        assert_eq!(selected.references.len(), count);

        reset_module_value_rename_validation_work();
        let started = std::time::Instant::now();
        assert!(validate_rename_in_snapshot(&snapshot, &selected, "renamed").is_ok());
        eprintln!(
            "module value consumer candidate validation: {count} consumers in {:?}",
            started.elapsed()
        );
        module_value_rename_validation_work()
    }

    #[test]
    fn module_value_consumer_candidate_lookup_work_is_adjacent_linear() {
        let smaller = measured_module_value_consumer_candidate_work(800);
        let larger = measured_module_value_consumer_candidate_work(1600);

        assert_eq!(smaller.candidate_lookups, 4802, "{smaller:#?}");
        assert_eq!(larger.candidate_lookups, 9602, "{larger:#?}");
    }

}
