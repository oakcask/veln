    #[test]
    fn direct_dependency_public_function_alias_references_keep_alias_identity() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "lib/math.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["lib/math.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::math from \"example/pkg\"\n\n",
                        "pub fn first(value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n\n",
                        "pub fn second(record: {renamed: Int}, value: Int) -> Int\n",
                        "  let callback: fn(Int) -> Int = math::renamed\n",
                        "  let renamed = record.renamed\n",
                        "  callback(math::renamed(value)) + math::target(value) + renamed\n",
                        "end\n\n",
                        "pub fn type_collision(value: math::renamed) -> Int\n",
                        "  math::renamed(1)\n",
                        "end\n\n",
                        "pub fn nested_type_collision(value: List<math::renamed>) -> Int\n",
                        "  math::renamed(2)\n",
                        "end\n\n",
                        "# renamed mention\n",
                        "pub fn renamed(value: Int) -> Int\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use lib::math from \"example/pkg\"\n\n",
                        "pub fn other(value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        {
            let (line, column) = (4, 10);
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::DirectDependency)
            );
            assert_eq!(result.definition.span.file.as_str(), "lib/math.veln");
            assert_eq!(
                (
                    result.definition.span.start.line,
                    result.definition.span.start.column
                ),
                (5, 8)
            );
            assert!(matches!(
                result.definition.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 4, 9),
                    ("main.veln", 8, 40),
                    ("main.veln", 10, 18),
                    ("main.veln", 14, 9),
                    ("main.veln", 18, 9),
                    ("other.veln", 4, 9),
                ]
            );
        }

        let type_collision = query_snapshot(&snapshot, "main.veln", 13, 35);
        assert!(type_collision.is_none_or(|result| result.references.is_empty()));
        let nested_type_collision = query_snapshot(&snapshot, "main.veln", 17, 47);
        assert!(nested_type_collision.is_none_or(|result| result.references.is_empty()));

        let target = query_snapshot(&snapshot, "main.veln", 10, 42).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(locations(&target.references), [("main.veln", 10, 42)]);
    }

    #[test]
    fn direct_dependency_function_alias_reference_boundaries_are_empty() {
        let fixtures = [
            (
                "alias chain",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                    "pub fn chain = renamed\n",
                ),
                "math::chain()",
            ),
            (
                "invalid-casing function alias",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn Renamed = target\n",
                ),
                "math::Renamed()",
            ),
            (
                "function alias chain through invalid-casing alias",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn Bad = target\n",
                    "pub fn chain = Bad\n",
                ),
                "math::chain()",
            ),
            (
                "type alias",
                concat!(
                    "pub type Item\n",
                    "end\n\n",
                    "pub type Renamed = Item\n",
                ),
                "math::Renamed",
            ),
            (
                "schema alias",
                concat!(
                    "pub schema Packet\n",
                    "  value: Int\n",
                    "end\n\n",
                    "pub schema Renamed = Packet\n",
                ),
                "math::Renamed",
            ),
        ];

        for (case, dependency_text, expression) in fixtures {
            let dependency =
                dependency_snapshot("example/pkg", &[("math.veln", dependency_text)], ["math.veln"]);
            let result = dependency_query(dependency, expression);

            assert!(
                result.is_none_or(|result| result.references.is_empty()),
                "accepted {case}"
            );
        }
    }

    #[test]
    fn direct_dependency_function_alias_chain_through_non_exported_module_is_empty() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "math.veln",
                    concat!(
                        "use internal\n\n",
                        "pub fn chain = internal::renamed\n",
                    ),
                ),
                (
                    "internal.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
            ],
            ["math.veln"],
        );
        let result = dependency_query(dependency, "math::chain()");

        assert!(
            result.is_none_or(|result| result.references.is_empty()),
            "accepted function alias chain through non-exported module"
        );
    }

    #[test]
    fn direct_dependency_function_alias_chain_through_invalid_casing_non_exported_module_is_empty() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "math.veln",
                    concat!(
                        "use internal\n\n",
                        "pub fn chain = internal::Bad\n",
                        "pub fn direct = internal::target\n",
                    ),
                ),
                (
                    "internal.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn Bad = target\n",
                    ),
                ),
            ],
            ["math.veln"],
        );

        let chain = dependency_query(dependency.clone(), "math::chain()");
        assert!(
            chain.is_none_or(|result| result.references.is_empty()),
            "accepted function alias chain through invalid-casing non-exported alias"
        );

        let invalid = dependency_query(dependency.clone(), "internal::Bad()");
        assert!(
            invalid.is_none_or(|result| result.references.is_empty()),
            "accepted invalid-casing alias selection"
        );

        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn first() -> Int\n",
                    "  math::direct()\n",
                    "end\n\n",
                    "pub fn second() -> Int\n",
                    "  math::direct()\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );
        let direct = query_snapshot(&snapshot, "main.veln", 4, 9).unwrap();
        assert_eq!(
            locations(&direct.references),
            [("main.veln", 4, 9), ("main.veln", 8, 9)]
        );
    }

    #[test]
    fn package_function_alias_chain_rejection_handles_many_aliases() {
        let aliases = (0..128)
            .map(|index| format!("pub fn Bad{index} = target\npub fn chain{index} = Bad{index}\n"))
            .collect::<String>();
        let dependency_text = format!(
            "pub fn target() -> Int\n  1\nend\n\npub fn direct = target\n{aliases}"
        );
        let dependency = dependency_snapshot(
            "example/pkg",
            &[("math.veln", dependency_text.as_str())],
            ["math.veln"],
        );

        let chain = dependency_query(dependency.clone(), "math::chain127()");
        assert!(
            chain.is_none_or(|result| result.references.is_empty()),
            "accepted high-count function alias chain through invalid-casing alias"
        );

        let direct = dependency_query(dependency, "math::direct()").unwrap();
        assert_eq!(locations(&direct.references), [("main.veln", 4, 9)]);
    }

    #[test]
    fn package_function_alias_references_are_limited_to_selected_project_sources() {
        struct Case {
            name: &'static str,
            snapshot: EffectiveProjectSnapshot,
            selected_source: &'static str,
            isolated_text: &'static str,
            selected_line: usize,
            selected_column: usize,
        }

        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["math.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["prelude.veln"],
        );
        let cases = [
            Case {
                name: "direct dependency",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![
                        source(
                            "main.veln",
                            concat!(
                                "use math from \"example/pkg\"\n\n",
                                "pub fn main(value: Int) -> Int\n",
                                "  math::renamed(value)\n",
                                "end\n",
                            ),
                        ),
                        source(
                            "other.veln",
                            concat!(
                                "use math from \"example/pkg\"\n\n",
                                "pub fn other(value: Int) -> Int\n",
                                "  math::renamed(value)\n",
                                "end\n",
                            ),
                        ),
                    ],
                    vec![dependency],
                ),
                selected_source: "main.veln",
                isolated_text: concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main(value: Int) -> Int\n",
                    "  math::renamed(value)\n",
                    "end\n",
                ),
                selected_line: 4,
                selected_column: 10,
            },
            Case {
                name: "standard library",
                snapshot: EffectiveProjectSnapshot::new(vec![
                    source(
                        "main.veln",
                        concat!(
                            "pub fn main(value: Int) -> Int\n",
                            "  renamed(value)\n",
                            "end\n",
                        ),
                    ),
                    source(
                        "other.veln",
                        concat!(
                            "pub fn other(value: Int) -> Int\n",
                            "  prelude::renamed(value)\n",
                            "end\n",
                        ),
                    ),
                ])
                .with_standard_library(standard_library),
                selected_source: "main.veln",
                isolated_text: concat!(
                    "pub fn main(value: Int) -> Int\n",
                    "  renamed(value)\n",
                    "end\n",
                ),
                selected_line: 2,
                selected_column: 4,
            },
        ];

        for case in cases {
            let selected_project = query_snapshot(
                &case.snapshot,
                case.selected_source,
                case.selected_line,
                case.selected_column,
            )
            .unwrap_or_else(|| panic!("missing {}", case.name));

            assert_eq!(
                selected_project.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias,
                "{}",
                case.name
            );
            assert!(matches!(
                selected_project.definition.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(
                locations(&selected_project.references),
                match case.name {
                    "direct dependency" => vec![("main.veln", 4, 9), ("other.veln", 4, 9)],
                    "standard library" => vec![("main.veln", 2, 3), ("other.veln", 2, 12)],
                    _ => unreachable!(),
                },
                "{}",
                case.name
            );

            let isolated_source = EffectiveProjectSnapshot::new(vec![source(
                case.selected_source,
                case.isolated_text,
            )]);
            let isolated_result = query_snapshot(
                &isolated_source,
                case.selected_source,
                case.selected_line,
                case.selected_column,
            );
            assert!(
                isolated_result.is_none_or(|result| result.references.is_empty()),
                "{} leaked package alias references into an isolated source",
                case.name
            );
        }
    }

    #[test]
    fn direct_dependency_public_function_alias_references_exclude_collisions_and_package_sources() {
        let selected = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n\n",
                    "pub fn package_body() -> Int\n",
                    "  renamed()\n",
                    "end\n",
                ),
            )],
            ["math.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[(
                "math.veln",
                concat!(
                    "pub fn target() -> Int\n",
                    "  2\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["math.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use math from \"example/pkg\"\n",
                        "use other_math from \"other/pkg\"\n\n",
                        "fn renamed() -> Int\n",
                        "  0\n",
                        "end\n\n",
                        "fn read(record: {renamed: Int}) -> Int\n",
                        "  math::renamed()\n",
                        "  other_math::renamed()\n",
                        "  renamed()\n",
                        "  record.renamed\n",
                        "end\n",
                    ),
                ),
                source(
                    "helper.veln",
                    concat!(
                        "use math from \"example/pkg\"\n\n",
                        "fn helper() -> Int\n",
                        "  math::renamed()\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        let result = query_snapshot(&snapshot, "main.veln", 9, 9).unwrap();

        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            locations(&result.references),
            [("helper.veln", 4, 9), ("main.veln", 9, 9)]
        );
    }

    #[test]
    fn direct_dependency_function_alias_references_require_public_exported_aliases() {
        let cases = [
            (
                "private alias",
                dependency_snapshot(
                    "example/pkg",
                    &[(
                        "math.veln",
                        concat!(
                            "pub fn target() -> Int\n",
                            "  1\n",
                            "end\n\n",
                            "fn hidden = target\n",
                        ),
                    )],
                    ["math.veln"],
                ),
                "use math from \"example/pkg\"\n\nfn main() -> Int\n  math::hidden()\nend\n",
                4,
                9,
            ),
            (
                "non-exported alias source",
                dependency_snapshot(
                    "example/pkg",
                    &[
                        ("public.veln", "pub fn ok() -> Int\n  1\nend\n"),
                        (
                            "hidden.veln",
                            concat!(
                                "pub fn target() -> Int\n",
                                "  1\n",
                                "end\n\n",
                                "pub fn hidden = target\n",
                            ),
                        ),
                    ],
                    ["public.veln"],
                ),
                "use hidden from \"example/pkg\"\n\nfn main() -> Int\n  hidden::hidden()\nend\n",
                4,
                11,
            ),
        ];

        for (name, dependency, source_text, line, column) in cases {
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", source_text)],
                vec![dependency],
            );

            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "accepted {name}"
            );
        }
    }


    #[test]
    fn standard_library_public_function_alias_references_keep_alias_identity() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "pub fn first() -> Int\n",
                "  renamed(1)\n",
                "end\n\n",
                "pub fn target_use(value: Int) -> Int\n",
                "  target(value)\n",
                "end\n\n",
                "pub fn second() -> fn(Int) -> Int\n",
                "  prelude::renamed\n",
                "end\n\n",
                "pub fn type_collision(value: prelude::renamed) -> Int\n",
                "  prelude::renamed(1)\n",
                "end\n\n",
                "pub fn nested_type_collision(value: List<prelude::renamed>) -> Int\n",
                "  prelude::renamed(2)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (line, column) in [(2, 3), (10, 12)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 2, 3),
                    ("main.veln", 10, 12),
                    ("main.veln", 14, 12),
                    ("main.veln", 18, 12),
                ]
            );
        }

        let type_collision = query_snapshot(&snapshot, "main.veln", 13, 38);
        assert!(type_collision.is_none_or(|result| result.references.is_empty()));
        let nested_type_collision = query_snapshot(&snapshot, "main.veln", 17, 50);
        assert!(nested_type_collision.is_none_or(|result| result.references.is_empty()));

        let target = query_snapshot(&snapshot, "main.veln", 6, 4).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(locations(&target.references), [("main.veln", 6, 3)]);
    }

    #[test]
    fn standard_library_function_alias_chain_selection_is_empty() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                    "pub fn chain = renamed\n",
                ),
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "pub fn main() -> Int\n  chain(1)\nend\n",
        )])
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 2, 4);
        assert!(result.is_none_or(|result| result.references.is_empty()));
    }
