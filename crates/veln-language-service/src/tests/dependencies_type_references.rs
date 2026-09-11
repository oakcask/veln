    #[test]
    fn direct_dependency_type_references_cover_project_type_roles_and_collisions() {
        let selected = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "model.veln",
                    "pub type Item\n  pub Ready(Int)\n  pub Item(Int)\nend\n",
                ),
                ("sibling.veln", "pub type Item\nend\n"),
            ],
            ["model.veln", "sibling.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[("model.veln", "pub type Item\nend\n")],
            ["model.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use model from \"example/pkg\"\n",
                        "use other_model from \"other/pkg\"\n",
                        "use sibling from \"example/pkg\"\n\n",
                        "type Item\n",
                        "end\n\n",
                        "type Box\n",
                        "  Wrap(model::Item)\n",
                        "end\n\n",
                        "pub type Alias = model::Item\n\n",
                        "fn make(input: model::Item) -> model::Item\n",
                        "  model::Item::Ready(1)\n",
                        "  model::Item::Item(2)\n",
                        "end\n\n",
                        "fn other(input: other_model::Item, sibling: sibling::Item) -> Item\n",
                        "  \"Item\"\n",
                        "end\n\n",
                        "# Item in a comment is not a type reference.\n",
                        "fn wrapped(items: Vec<model::Item>) -> Vec<model::Item>\n",
                        "  items\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use model from \"example/pkg\"\n\n",
                        "fn second(input: model::Item) -> model::Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        let result = query_snapshot(&snapshot, "main.veln", 14, 23).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(result.definition.span.file.as_str(), "model.veln");
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 9, 15),
                ("main.veln", 12, 25),
                ("main.veln", 14, 23),
                ("main.veln", 14, 39),
                ("main.veln", 15, 10),
                ("main.veln", 16, 10),
                ("main.veln", 24, 30),
                ("main.veln", 24, 51),
                ("other.veln", 3, 25),
                ("other.veln", 3, 41),
            ]
        );
    }

    #[test]
    fn standard_library_type_references_cover_prelude_and_qualified_forms() {
        let standard_library = standard_library_snapshot(
            &[("prelude.veln", "pub type Vec\n  pub Empty\nend\n")],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "fn first(items: Vec<Int>) -> Vec<Int>\n",
                    "  items\n",
                    "end\n\n",
                    "fn second(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n",
                    "  prelude::Vec::Empty()\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                "fn third(items: Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
            ),
        ])
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 1, 17).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 1, 17),
                ("main.veln", 1, 30),
                ("main.veln", 5, 27),
                ("main.veln", 5, 49),
                ("main.veln", 6, 12),
                ("other.veln", 1, 17),
                ("other.veln", 1, 39),
            ]
        );
    }

    #[test]
    fn direct_dependency_constructor_references_cover_qualified_calls_and_patterns() {
        let selected = dependency_snapshot(
            "example/pkg",
            &[("model.veln", "pub type Item\n  pub Ready(Int)\nend\n")],
            ["model.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[("model.veln", "pub type Item\n  pub Ready(Int)\nend\n")],
            ["model.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use model from \"example/pkg\"\n",
                        "use other_model from \"other/pkg\"\n\n",
                        "type Local\n",
                        "  Ready(Int)\n",
                        "end\n\n",
                        "fn make(input: Int) -> model::Item\n",
                        "  model::Item::Ready(input)\n",
                        "end\n\n",
                        "fn collisions(record: {Ready: Int}, Ready: Int) -> Int\n",
                        "  other_model::Ready(1)\n",
                        "  Local::Ready(2)\n",
                        "  record.Ready + Ready\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use model from \"example/pkg\"\n\n",
                        "fn other(input: model::Item) -> Int\n",
                        "  match input\n",
                        "    Ready(value) => value\n",
                        "  end\n",
                        "end\n\n",
                        "fn qualified() -> model::Item\n",
                        "  model::Ready(1)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        let result = query_snapshot(&snapshot, "main.veln", 9, 17).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(result.definition.span.file.as_str(), "model.veln");
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 9, 16),
                ("other.veln", 5, 5),
                ("other.veln", 10, 10),
            ]
        );
    }

    #[test]
    fn standard_library_constructor_references_cover_prelude_and_qualified_forms() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub type Option\n  pub Some(Int)\n  pub None\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "fn first(input: Option<Int>) -> Int\n",
                    "  match input\n",
                    "    Some(value) => value\n",
                    "    None => 0\n",
                    "  end\n",
                    "end\n\n",
                    "fn second() -> Option<Int>\n",
                    "  prelude::Option::Some(1)\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                "fn third() -> Option<Int>\n  prelude::Some(2)\nend\n",
            ),
        ])
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 3, 6).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 3, 5),
                ("main.veln", 9, 20),
                ("other.veln", 2, 12),
            ]
        );
    }

    #[test]
    fn package_constructor_alias_routes_do_not_expand_references() {
        let direct = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use model from \"example/pkg\"\n\n",
                    "fn make() -> model::Alias\n",
                    "  model::Alias::Ready(1)\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/pkg",
                &[(
                    "model.veln",
                    "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n",
                )],
                ["model.veln"],
            )],
        );
        let standard = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "fn make() -> prelude::Alias\n  prelude::Alias::Some(1)\nend\n",
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub type Option\n  pub Some(Int)\nend\n\npub type Alias = Option\n",
            )],
            ["prelude.veln"],
        ));

        for (name, snapshot, line, column) in [
            ("direct dependency alias", direct, 4, 17),
            ("standard library alias", standard, 2, 19),
        ] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "{name} unexpectedly selected a constructor"
            );
        }
    }

    #[test]
    fn unsupported_package_type_selections_do_not_expand_references() {
        struct Case {
            name: &'static str,
            snapshot: EffectiveProjectSnapshot,
            source_path: &'static str,
            line: usize,
            column: usize,
            expect_symbol: Option<SymbolKind>,
        }

        let cases = [
            Case {
                name: "private direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::Item) -> model::Item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "type Item\nend\n")],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "non-exported direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use hidden from \"example/pkg\"\n\n",
                            "fn read(input: hidden::Item) -> hidden::Item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[
                            ("public.veln", "pub fn ok() -> Int\n  1\nend\n"),
                            ("hidden.veln", "pub type Item\nend\n"),
                        ],
                        ["public.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 28,
                expect_symbol: None,
            },
            Case {
                name: "public direct dependency type alias",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::Alias) -> model::Alias\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[(
                            "model.veln",
                            "pub type Item\nend\n\npub type Alias = Item\n",
                        )],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "invalid-casing direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::item) -> model::item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "pub type item\nend\n")],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "standard library public type alias",
                snapshot: EffectiveProjectSnapshot::new(vec![source(
                    "main.veln",
                    concat!(
                        "fn read(input: prelude::Alias) -> prelude::Alias\n",
                        "  input\n",
                        "end\n",
                    ),
                )])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", "pub type Vec\nend\n\npub type Alias = Vec\n")],
                    ["prelude.veln"],
                )),
                source_path: "main.veln",
                line: 1,
                column: 27,
                expect_symbol: None,
            },
            Case {
                name: "standard library invalid-casing type",
                snapshot: EffectiveProjectSnapshot::new(vec![source(
                    "main.veln",
                    "fn read(input: prelude::vec) -> prelude::vec\n  input\nend\n",
                )])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", "pub type vec\nend\n")],
                    ["prelude.veln"],
                )),
                source_path: "main.veln",
                line: 1,
                column: 27,
                expect_symbol: None,
            },
        ];

        for case in cases {
            let result = query_snapshot(&case.snapshot, case.source_path, case.line, case.column);
            match case.expect_symbol {
                Some(kind) => {
                    let result = result.unwrap_or_else(|| {
                        panic!("{} should navigate to an unsupported non-type symbol", case.name)
                    });
                    assert_eq!(result.selected_symbol.kind, kind, "{}", case.name);
                    assert!(
                        result.references.is_empty(),
                        "{} unexpectedly returned references: {:?}",
                        case.name,
                        locations(&result.references)
                    );
                }
                None => assert!(
                    result.is_none(),
                    "{} unexpectedly navigated to {:?}",
                    case.name,
                    result.map(|result| result.selected_symbol)
                ),
            }
        }
    }
