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
    fn package_constructor_module_qualified_leaf_requires_unique_identity() {
        let direct = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use model from \"example/pkg\"\n\n",
                    "fn ambiguous() -> model::Left\n",
                    "  model::Ready(1)\n",
                    "end\n\n",
                    "fn exact() -> model::Left\n",
                    "  model::Left::Ready(1)\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/pkg",
                &[(
                    "model.veln",
                    "pub type Left\n  pub Ready(Int)\nend\n\npub type Right\n  pub Ready(Int)\nend\n",
                )],
                ["model.veln"],
            )],
        );
        let standard = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "fn ambiguous() -> prelude::Left\n",
                "  prelude::Ready(1)\n",
                "end\n\n",
                "fn exact() -> prelude::Left\n",
                "  prelude::Left::Ready(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub type Left\n  pub Ready(Int)\nend\n\npub type Right\n  pub Ready(Int)\nend\n",
            )],
            ["prelude.veln"],
        ));

        for (name, snapshot, ambiguous_line, ambiguous_column, exact_line, exact_column, file) in [
            ("direct dependency", direct, 4, 10, 8, 16, "model.veln"),
            ("standard library", standard, 2, 12, 6, 18, "prelude.veln"),
        ] {
            assert!(
                query_snapshot(&snapshot, "main.veln", ambiguous_line, ambiguous_column).is_none(),
                "{name} module-qualified leaf unexpectedly selected a constructor"
            );

            let exact = query_snapshot(&snapshot, "main.veln", exact_line, exact_column)
                .unwrap_or_else(|| panic!("{name} type-qualified constructor did not resolve"));
            assert_eq!(exact.selected_symbol.kind, SymbolKind::Constructor, "{name}");
            assert_eq!(exact.definition.span.file.as_str(), file, "{name}");
        }
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
    fn package_constructor_alias_routes_preserve_definition_without_direct_reference_support() {
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
            let result = query_snapshot(&snapshot, "main.veln", line, column)
                .unwrap_or_else(|| panic!("{name} did not select the underlying constructor"));
            assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor, "{name}");
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias,
                "{name}"
            );
            assert!(matches!(
                result.definition.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(result.definition.span.start.column, 7, "{name}");
            assert!(
                result.references.is_empty(),
                "{name} unexpectedly expanded alias-qualified references: {:?}",
                locations(&result.references)
            );
        }
    }

    #[test]
    fn package_constructor_reference_collection_excludes_alias_routes_in_one_pass() {
        let direct_calls = (0..96)
            .map(|index| format!("  model::Item::Ready({index})\n"))
            .collect::<String>();
        let alias_calls = (0..96)
            .map(|index| format!("  model::Alias::Ready({index})\n"))
            .collect::<String>();
        let text = format!(
            "use model from \"example/pkg\"\n\nfn direct() -> model::Item\n{direct_calls}end\n\nfn alias() -> model::Alias\n{alias_calls}end\n"
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", &text)],
            vec![dependency_snapshot(
                "example/pkg",
                &[(
                    "model.veln",
                    "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n",
                )],
                ["model.veln"],
            )],
        );

        reset_constructor_reference_collections();
        let direct = query_snapshot(&snapshot, "main.veln", 4, 16)
            .expect("direct constructor selection resolves");

        assert_eq!(constructor_reference_collections(), 1);
        assert_eq!(direct.references.len(), 96);
        assert!(
            locations(&direct.references)
                .iter()
                .all(|(_, line, _)| *line >= 4 && *line < 100),
            "direct references included alias route spans: {:?}",
            locations(&direct.references)
        );

        reset_constructor_reference_collections();
        let alias = query_snapshot(&snapshot, "main.veln", 103, 17)
            .expect("alias-qualified constructor selection resolves");

        assert_eq!(constructor_reference_collections(), 0);
        assert_eq!(
            alias.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert!(alias.references.is_empty());
    }

    #[test]
    fn direct_dependency_type_alias_references_keep_alias_identity() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use model from \"example/pkg\"\n",
                        "use core from \"example/pkg\"\n",
                        "use other_model from \"other/pkg\"\n\n",
                        "type Local\n",
                        "  Same(Int)\n",
                        "end\n\n",
                        "pub type LocalAlias = model::Same\n\n",
                        "fn direct(input: model::Same) -> model::Same\n",
                        "  let current: model::Same = input\n",
                        "  model::Same::Ready(1)\n",
                        "end\n\n",
                        "fn target(input: core::Same) -> core::Same\n",
                        "  core::Same::Ready(1)\n",
                        "end\n\n",
                        "fn collisions(input: other_model::Same, Same: Int, record: {Same: Int}) -> Int\n",
                        "  Same + record.Same\n",
                        "end\n\n",
                        "# Same in a comment is lexical noise.\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use model from \"example/pkg\"\n\n",
                        "fn other(input: model::Same) -> model::Same\n",
                        "  model::Same::Ready(1)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![
                dependency_snapshot(
                    "example/pkg",
                    &[
                        (
                            "model.veln",
                            "use core\n\npub type Same = core::Same\n",
                        ),
                        (
                            "core.veln",
                            "pub type Same\n  pub Ready(Int)\nend\n",
                        ),
                    ],
                    ["model.veln", "core.veln"],
                ),
                dependency_snapshot(
                    "other/pkg",
                    &[("model.veln", "pub type Same\nend\n")],
                    ["model.veln"],
                ),
            ],
        );

        let alias = query_snapshot(&snapshot, "main.veln", 11, 25).unwrap();

        assert_eq!(alias.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            alias.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            alias.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(alias.definition.span.file.as_str(), "model.veln");
        assert_eq!(
            locations(&alias.references),
            [
                ("main.veln", 9, 30),
                ("main.veln", 11, 25),
                ("main.veln", 11, 41),
                ("main.veln", 12, 23),
                ("main.veln", 13, 10),
                ("other.veln", 3, 24),
                ("other.veln", 3, 40),
                ("other.veln", 4, 10),
            ]
        );

        let target = query_snapshot(&snapshot, "main.veln", 16, 24).unwrap();

        assert_eq!(target.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(target.definition.span.file.as_str(), "core.veln");
        assert_eq!(
            locations(&target.references),
            [
                ("main.veln", 16, 24),
                ("main.veln", 16, 39),
                ("main.veln", 17, 9),
            ]
        );
    }

    #[test]
    fn type_role_selection_prefers_local_type_and_rejects_field_alias_collision() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use model from \"example/pkg\"\n\n",
                    "type Same\n",
                    "end\n\n",
                    "fn read(record: {Same: Int}, input: Same) -> Same\n",
                    "  input\n",
                    "end\n",
                ),
            )],
            vec![dependency_snapshot(
                "example/pkg",
                &[("model.veln", "pub type Target\nend\n\npub type Same = Target\n")],
                ["model.veln"],
            )],
        );

        let local = query_snapshot(&snapshot, "main.veln", 6, 39).unwrap();

        assert_eq!(local.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            local.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(local.selected_symbol.package_origin, None);
        assert_eq!(local.definition.span.file.as_str(), "main.veln");
        assert_eq!(
            locations(&local.references),
            [("main.veln", 6, 37), ("main.veln", 6, 46)]
        );
        assert!(
            query_snapshot(&snapshot, "main.veln", 6, 18).is_none(),
            "record field selection must not bind to the imported type alias"
        );
    }

    #[test]
    fn unsupported_direct_dependency_type_alias_selection_does_not_fall_back_to_type() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use model from \"example/pkg\"\n",
                    "use other_model from \"other/pkg\"\n\n",
                    "fn read(input: model::Alias) -> other_model::Alias\n",
                    "  input\n",
                    "end\n",
                ),
            )],
            vec![
                dependency_snapshot(
                    "example/pkg",
                    &[("model.veln", "pub type Alias = Missing\n")],
                    ["model.veln"],
                ),
                dependency_snapshot(
                    "other/pkg",
                    &[("model.veln", "pub type Alias\nend\n")],
                    ["model.veln"],
                ),
            ],
        );

        let alias = query_snapshot(&snapshot, "main.veln", 4, 23).unwrap();

        assert_eq!(alias.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            alias.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            alias.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(alias.definition.span.file.as_str(), "model.veln");
        assert!(
            alias.references.is_empty(),
            "unsupported alias selection unexpectedly returned references: {:?}",
            locations(&alias.references)
        );
    }

    #[test]
    fn direct_dependency_type_alias_target_may_live_in_non_exported_module() {
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use facade from \"example/pkg\"\n\n",
                        "pub type Local = facade::PublicHidden\n\n",
                        "fn read(input: facade::PublicHidden) -> Vec<facade::PublicHidden>\n",
                        "  facade::PublicHidden::Ready(1)\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use facade from \"example/pkg\"\n\n",
                        "fn other(input: facade::PublicHidden) -> facade::PublicHidden\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency_snapshot(
                "example/pkg",
                &[
                    (
                        "facade.veln",
                        "use internal::core\n\npub type PublicHidden = core::Hidden\n",
                    ),
                    (
                        "internal/core.veln",
                        "pub type Hidden\n  pub Ready(Int)\nend\n",
                    ),
                ],
                ["facade.veln"],
            )],
        );

        let alias = query_snapshot(&snapshot, "main.veln", 5, 26).unwrap();

        assert_eq!(
            alias.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            alias.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(
            locations(&alias.references),
            [
                ("main.veln", 3, 26),
                ("main.veln", 5, 24),
                ("main.veln", 5, 53),
                ("other.veln", 3, 25),
                ("other.veln", 3, 50),
            ]
        );
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
                name: "private direct dependency type alias",
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
                            "pub type Item\nend\n\ntype Alias = Item\n",
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
                name: "non-exported direct dependency type alias",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use hidden from \"example/pkg\"\n\n",
                            "fn read(input: hidden::Alias) -> hidden::Alias\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[
                            ("model.veln", "pub type Item\nend\n"),
                            ("hidden.veln", "pub type Item\nend\n\npub type Alias = Item\n"),
                        ],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 29,
                expect_symbol: None,
            },
            Case {
                name: "invalid-casing direct dependency type alias declaration",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::alias) -> model::alias\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "pub type Item\nend\n\npub type alias = Item\n")],
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
