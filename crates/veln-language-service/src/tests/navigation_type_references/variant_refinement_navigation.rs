
    #[test]
    fn valid_variant_refinements_share_base_and_constructor_navigation() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "pub type State\n",
                "  pub Ready(Int)\n",
                "  pub Closed\n",
                "end\n\n",
                "pub type Other\n",
                "  pub Ready\n",
                "end\n\n",
                "pub type PublicState = State\n\n",
                "fn observe(direct: State::Ready, alias: PublicState::Ready | PublicState::Closed) -> Int\n",
                "  let made = State::Ready(1)\n",
                "  match direct\n",
                "    State::Ready(value) => value\n",
                "    State::Closed => 0\n",
                "  end\n",
                "end\n\n",
                "fn other(value: Other::Ready) -> Other\n",
                "  Other::Ready\n",
                "end\n",
            ),
        )];

        let parsed = veln_syntax::parse(&sources[0]);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let snapshot = EffectiveProjectSnapshot::new(sources.clone());
        let direct_base = query_snapshot(&snapshot, "main.veln", 12, 21).unwrap();
        assert_eq!(direct_base.selected_symbol.kind, SymbolKind::Type);
        assert_location(&direct_base.definition, "main.veln", 1, 10);

        let alias_base = query(sources.clone(), "main.veln", 12, 41).unwrap();
        assert_eq!(alias_base.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            alias_base.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_location(&alias_base.definition, "main.veln", 10, 10);

        let singleton_variant = query(sources.clone(), "main.veln", 12, 28).unwrap();
        let union_variant = query(sources.clone(), "main.veln", 12, 54).unwrap();
        for result in [&singleton_variant, &union_variant] {
            assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
            assert_location(&result.definition, "main.veln", 2, 7);
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 12, 27),
                    ("main.veln", 12, 54),
                    ("main.veln", 13, 21),
                    ("main.veln", 15, 12),
                ]
            );
            assert!(validate_rename(result, "Prepared").is_ok());
        }
    }

    #[test]
    fn variant_refinement_navigation_resolves_generic_aliases() {
        let snapshot = variant_refinement_alias_snapshot();
        let base = query_snapshot(&snapshot, "main.veln", 8, 19).unwrap();
        assert_eq!(base.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            base.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_location(&base.definition, "main.veln", 6, 10);

        let variant = query_snapshot(&snapshot, "main.veln", 8, 38).unwrap();
        assert_eq!(variant.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&variant.definition, "main.veln", 4, 7);
    }

    #[test]
    fn variant_refinement_navigation_resolves_transitive_aliases() {
        let snapshot = variant_refinement_alias_snapshot();
        let base = query_snapshot(&snapshot, "main.veln", 12, 29).unwrap();
        assert_location(&base.definition, "model.veln", 7, 10);

        let variant = query_snapshot(&snapshot, "main.veln", 12, 32).unwrap();
        assert_location(&variant.definition, "model.veln", 2, 7);
    }

    #[test]
    fn variant_refinement_navigation_resolves_imported_aliases() {
        let snapshot = variant_refinement_alias_snapshot();
        let base = query_snapshot(&snapshot, "main.veln", 16, 27).unwrap();
        assert_location(&base.definition, "model.veln", 8, 10);

        let variant = query_snapshot(&snapshot, "main.veln", 16, 34).unwrap();
        assert_location(&variant.definition, "model.veln", 2, 7);
        assert_eq!(
            locations(&variant.references),
            [
                ("main.veln", 12, 32),
                ("main.veln", 16, 34),
                ("main.veln", 17, 28),
                ("main.veln", 19, 19),
            ]
        );
        assert!(validate_rename(&variant, "Prepared").is_ok());

        let alias_rename = navigate_for_rename(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 8,
                column: 19,
            },
        )
        .unwrap();
        assert_location(&alias_rename.definition, "main.veln", 6, 10);
        assert_eq!(locations(&alias_rename.references), [("main.veln", 8, 19)]);
    }

    fn variant_refinement_alias_snapshot() -> EffectiveProjectSnapshot {
        EffectiveProjectSnapshot::new(vec![
            source(
                "model.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready(Int)\n",
                    "  pub Closed\n",
                    "end\n\n",
                    "pub type A = State\n",
                    "pub type B = A\n",
                    "pub type Alias = State\n",
                ),
            ),
            source(
                "main.veln",
                concat!(
                    "use model\n\n",
                    "pub type Box<A>\n",
                    "  pub Boxed(A)\n",
                    "end\n",
                    "pub type GenericAlias = Box\n\n",
                    "fn generic(value: GenericAlias<Int>::Boxed) -> Box<Int>\n",
                    "  value\n",
                    "end\n\n",
                    "fn transitive(value: model::B::Ready) -> model::State\n",
                    "  value\n",
                    "end\n\n",
                    "fn imported(value: model::Alias::Ready | model::Alias::Closed) -> model::State\n",
                    "  let made = model::State::Ready(1)\n",
                    "  match value\n",
                    "    model::State::Ready(value) => made\n",
                    "    model::State::Closed => made\n",
                    "  end\n",
                    "end\n",
                ),
            ),
        ])
    }

    fn assert_package_location(
        location: &NavigationLocation,
        path: &str,
        line: usize,
        column: usize,
    ) {
        assert!(matches!(location.source, NavigationSource::Package { .. }));
        assert_eq!(location.span.file.as_str(), path);
        assert_eq!(
            (location.span.start.line, location.span.start.column),
            (line, column)
        );
    }

    fn variant_refinement_target_origin_snapshot() -> EffectiveProjectSnapshot {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "facade.veln",
                    concat!(
                        "use core\n\n",
                        "pub type A = core::State\n",
                        "pub type B = A\n",
                    ),
                ),
                (
                    "core.veln",
                    "pub type State\n  pub Ready(Int)\n  pub Closed\nend\n",
                ),
            ],
            ["facade.veln", "core.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    concat!(
                        "use states\n\n",
                        "pub type StandardA = states::State\n",
                        "pub type StandardB = StandardA\n",
                    ),
                ),
                (
                    "states.veln",
                    "pub type State\n  pub Ready(Int)\nend\n",
                ),
            ],
            ["prelude.veln", "states.veln"],
        );
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/pkg\"\n\n",
                    "pub type Local = facade::B\n",
                    "pub type Transit = Local\n\n",
                    "fn local(value: Transit::Ready | Local::Closed) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn dependency(value: facade::B::Ready) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn standard(value: StandardB::Ready) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn ordinary_dependency(value: facade::B) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn ordinary_standard(value: StandardB) -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library)
    }

    fn assert_local_variant_refinement_target_origin(snapshot: &EffectiveProjectSnapshot) {
        let local_base = query_snapshot(snapshot, "main.veln", 6, 18).unwrap();
        assert_location(&local_base.definition, "main.veln", 4, 10);
        let local_variant = query_snapshot(snapshot, "main.veln", 6, 27).unwrap();
        assert_package_location(&local_variant.definition, "core.veln", 2, 7);
        assert_eq!(
            local_variant.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
    }

    fn assert_dependency_variant_refinement_target_origin(
        snapshot: &EffectiveProjectSnapshot,
    ) {
        let dependency_base = query_snapshot(snapshot, "main.veln", 10, 30).unwrap();
        assert_eq!(
            dependency_base.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_package_location(&dependency_base.definition, "facade.veln", 4, 10);
        let dependency_base_definition = definition_at(
            snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 10,
                column: 30,
            },
        )
        .expect("direct dependency refinement alias has a public definition");
        assert_package_location(&dependency_base_definition, "facade.veln", 4, 10);
        let dependency_variant = query_snapshot(snapshot, "main.veln", 10, 33).unwrap();
        assert_package_location(&dependency_variant.definition, "core.veln", 2, 7);
        let dependency_variant_definition = definition_at(
            snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 10,
                column: 33,
            },
        )
        .expect("direct dependency refinement variant has a public definition");
        assert_package_location(&dependency_variant_definition, "core.veln", 2, 7);
        assert_eq!(
            definition_at(
                snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 18,
                    column: 39,
                },
            ),
            None,
            "ordinary transitive direct-dependency alias stays ineligible"
        );
    }

    fn assert_standard_variant_refinement_target_origin(snapshot: &EffectiveProjectSnapshot) {
        let standard_base = query_snapshot(snapshot, "main.veln", 14, 20).unwrap();
        assert_package_location(&standard_base.definition, "prelude.veln", 4, 10);
        let standard_base_definition = definition_at(
            snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 14,
                column: 20,
            },
        )
        .expect("standard-library refinement alias has a public definition");
        assert_package_location(&standard_base_definition, "prelude.veln", 4, 10);
        let standard_variant = query_snapshot(snapshot, "main.veln", 14, 31).unwrap();
        assert_package_location(&standard_variant.definition, "states.veln", 2, 7);
        let standard_variant_definition = definition_at(
            snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 14,
                column: 31,
            },
        )
        .expect("standard-library refinement variant has a public definition");
        assert_package_location(&standard_variant_definition, "states.veln", 2, 7);
        assert_eq!(
            standard_variant.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(
            definition_at(
                snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 22,
                    column: 29,
                },
            ),
            None,
            "ordinary transitive standard-library alias stays ineligible"
        );
    }

    #[test]
    fn variant_refinement_aliases_resolve_target_origins_independently() {
        let snapshot = variant_refinement_target_origin_snapshot();
        assert_local_variant_refinement_target_origin(&snapshot);
        assert_dependency_variant_refinement_target_origin(&snapshot);
        assert_standard_variant_refinement_target_origin(&snapshot);
    }

    #[test]
    fn retained_standard_alias_targets_implicit_prelude_type() {
        let prelude_source = source(
            "prelude.veln",
            "pub type State\n  pub Ready\nend\n",
        );
        let bridge_source = source(
            "bridge.veln",
            concat!(
                "pub type Alias = State\n\n",
                "fn observe(value: Alias::Ready) -> Int\n",
                "  0\n",
                "end\n",
            ),
        );
        let mut semantic_module = veln_ast::lower_surface_ast(&veln_syntax::parse(&prelude_source).tree);
        for declaration in &mut semantic_module.types {
            declaration.module_name = Some("std::prelude".to_string());
        }
        let mut bridge_module = veln_ast::lower_surface_ast(&veln_syntax::parse(&bridge_source).tree);
        for alias in &mut bridge_module.aliases {
            alias.module_name = Some("std::bridge".to_string());
        }
        for function in &mut bridge_module.functions {
            function.module_name = Some("std::bridge".to_string());
        }
        semantic_module.aliases.extend(bridge_module.aliases);
        semantic_module.functions.extend(bridge_module.functions);
        let lowered = veln_sema::lower_checked_surface_module(&semantic_module);
        assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);

        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "pub type State\n  pub Ready\nend\n",
                ),
                ("bridge.veln", "pub type Alias = State\n"),
            ],
            ["prelude.veln", "bridge.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use bridge from \"std\"\n\n",
                "pub fn observe(value: bridge::Alias::Ready) -> Int\n",
                "  0\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let base = query_snapshot(&snapshot, "main.veln", 3, 31).unwrap();
        assert_eq!(
            base.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_package_location(&base.definition, "bridge.veln", 1, 10);

        let variant = query_snapshot(&snapshot, "main.veln", 3, 38).unwrap();
        assert_eq!(variant.selected_symbol.kind, SymbolKind::Constructor);
        assert_package_location(&variant.definition, "prelude.veln", 2, 7);
        assert_eq!(
            variant.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
    }

    #[test]
    fn retained_alias_target_prefers_declaring_module_to_implicit_prelude() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "pub type State\n  pub Ready\nend\n",
                ),
                (
                    "bridge.veln",
                    concat!(
                        "pub type State\n",
                        "  pub Ready\n",
                        "end\n\n",
                        "pub type Alias = State\n",
                    ),
                ),
            ],
            ["prelude.veln", "bridge.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use bridge from \"std\"\n\n",
                "fn observe(value: bridge::Alias::Ready) -> Int\n",
                "  0\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let variant = query_snapshot(&snapshot, "main.veln", 3, 38).unwrap();
        assert_package_location(&variant.definition, "bridge.veln", 2, 7);
    }

    #[test]
    fn retained_dependency_alias_targets_standard_prelude_identity() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[("bridge.veln", "pub type Alias = State\n")],
            ["bridge.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub type State\n  pub Ready\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use bridge from \"example/pkg\"\n\n",
                    "fn observe(value: bridge::Alias::Ready) -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        let base = query_snapshot(&snapshot, "main.veln", 3, 31).unwrap();
        assert_package_location(&base.definition, "bridge.veln", 1, 10);
        assert_eq!(
            base.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );

        let variant = query_snapshot(&snapshot, "main.veln", 3, 38).unwrap();
        assert_package_location(&variant.definition, "prelude.veln", 2, 7);
        assert_eq!(
            variant.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
    }

    #[test]
    fn variant_refinement_navigation_obeys_visibility_and_owner_boundaries() {
        let sources = vec![
            source(
                "model.veln",
                concat!(
                    "pub type State\n",
                    "  Private(Int)\n",
                    "  pub Ready(Int)\n",
                    "end\n\n",
                    "pub type Other\n",
                    "  pub Shared\n",
                    "end\n",
                ),
            ),
            source(
                "main.veln",
                concat!(
                    "use model\n\n",
                    "fn private(value: model::State::Private) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn unresolved(value: model::State::Missing) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn wrong_owner(value: model::State::Shared) -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            ),
            source(
                "model.test.veln",
                "use model\n\ntest companion(value: model::State::Private) -> Int\n  0\nend\n",
            ),
        ];

        assert!(query(sources.clone(), "main.veln", 3, 34).is_none());
        assert!(query(sources.clone(), "main.veln", 7, 37).is_none());
        assert!(query(sources.clone(), "main.veln", 11, 38).is_none());

        let companion = query(sources, "model.test.veln", 3, 43).unwrap();
        assert_location(&companion.definition, "model.veln", 2, 3);
        assert!(validate_rename(&companion, "Hidden").is_ok());
    }

    #[test]
    fn invalid_variant_refinement_bases_require_complete_identity() {
        let text = concat!(
            "type State\n",
            "  Ready\n",
            "end\n\n",
            "type Other\n",
            "  Shared\n",
            "end\n\n",
            "type Helper\n",
            "end\n\n",
            "pub type Alias = State\n\n",
            "fn valid_single(value: Alias::Ready) -> Int\n",
            "  0\n",
            "end\n\n",
            "fn valid_union(value: Alias::Ready | Alias::Ready) -> Int\n",
            "  0\n",
            "end\n\n",
            "fn missing(value: Alias::Missing) -> Int\n",
            "  0\n",
            "end\n\n",
            "fn wrong_owner(value: Alias::Shared) -> Int\n",
            "  0\n",
            "end\n\n",
            "fn non_constructor(value: Alias::Helper) -> Int\n",
            "  0\n",
            "end\n",
            "\nfn direct_missing(value: State::Missing) -> Int\n",
            "  0\n",
            "end\n",
            "\nfn direct_wrong_owner(value: State::Shared) -> Int\n",
            "  0\n",
            "end\n",
            "\nfn direct_non_constructor(value: State::Helper) -> Int\n",
            "  0\n",
            "end\n",
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", text)]);
        let position = |line: usize, needle: &str| SourcePosition {
            source: SourcePath::new("main.veln"),
            line,
            column: text.lines().nth(line - 1).unwrap().find(needle).unwrap() + 1,
        };

        for (line, needle) in [
            (22, "Alias::"),
            (26, "Alias::"),
            (30, "Alias::"),
            (34, "State::"),
            (38, "State::"),
            (42, "State::"),
        ] {
            assert!(definition_at(&snapshot, position(line, needle)).is_none());
            assert!(navigate(&snapshot, position(line, needle)).is_none());
            assert!(navigate_for_rename(&snapshot, position(line, needle)).is_none());
        }

        for (line, needle) in [(14, "Alias::"), (18, "Alias::")] {
            let result = navigate(&snapshot, position(line, needle)).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
            assert!(navigate_for_rename(&snapshot, position(line, needle)).is_some());
        }
    }

    #[test]
    fn variant_refinement_navigation_requires_exact_terminal_adt_arity() {
        let text = concat!(
            "pub type Box<A>\n",
            "  pub Boxed(A)\n",
            "end\n\n",
            "pub type GenericAlias = Box\n\n",
            "fn valid_direct(value: Box<Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn valid_union(value: GenericAlias<Int>::Boxed | GenericAlias<Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn missing_direct(value: Box::Boxed) -> Int\n  0\nend\n\n",
            "fn excess_direct(value: Box<Int, Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn missing_alias(value: GenericAlias::Boxed) -> Int\n  0\nend\n\n",
            "fn excess_alias(value: GenericAlias<Int, Int>::Boxed) -> Int\n  0\nend\n",
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", text)]);
        let position = |line_text: &str, needle: &str| {
            let (line, source_line) = text
                .lines()
                .enumerate()
                .find(|(_, candidate)| candidate.contains(line_text))
                .unwrap();
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: line + 1,
                column: source_line.find(needle).unwrap() + 1,
            }
        };

        for (line_text, base) in [
            ("missing_direct", "Box::"),
            ("excess_direct", "Box<Int"),
            ("missing_alias", "GenericAlias::"),
            ("excess_alias", "GenericAlias<Int"),
        ] {
            for needle in [base, "Boxed"] {
                assert!(
                    definition_at(&snapshot, position(line_text, needle)).is_none(),
                    "definition remained for {line_text} at {needle}"
                );
                assert!(
                    navigate(&snapshot, position(line_text, needle)).is_none(),
                    "navigation remained for {line_text} at {needle}"
                );
                assert!(
                    navigate_for_rename(&snapshot, position(line_text, needle)).is_none(),
                    "rename remained for {line_text} at {needle}"
                );
            }
        }

        for (line_text, needle) in [
            ("valid_direct", "Box<Int"),
            ("valid_direct", "Boxed"),
            ("valid_union", "GenericAlias<Int>"),
            ("valid_union", "Boxed"),
        ] {
            assert!(definition_at(&snapshot, position(line_text, needle)).is_some());
            assert!(navigate(&snapshot, position(line_text, needle)).is_some());
            assert!(navigate_for_rename(&snapshot, position(line_text, needle)).is_some());
        }

        let valid = navigate(&snapshot, position("valid_direct", "Boxed")).unwrap();
        let reference_lines = valid
            .references
            .iter()
            .map(|location| location.start.line)
            .collect::<Vec<_>>();
        assert_eq!(reference_lines, [7, 11, 11]);
        let rename = navigate_for_rename(&snapshot, position("valid_direct", "Boxed")).unwrap();
        assert_eq!(rename.references.len(), 3);
        assert!(validate_rename_in_snapshot(&snapshot, &rename, "Packed").is_ok());

        let alias_rename = navigate_for_rename(
            &snapshot,
            position("valid_union", "GenericAlias<Int>"),
        )
        .unwrap();
        assert_eq!(alias_rename.references.len(), 2);
        assert!(validate_rename_in_snapshot(&snapshot, &alias_rename, "GenericBox").is_ok());
    }

    #[test]
    fn variant_refinement_navigation_requires_a_valid_union_identity() {
        let text = concat!(
            "pub type Left\n  pub LeftReady\nend\n\n",
            "pub type Right\n  pub RightReady\nend\n\n",
            "pub type Box<A>\n  pub Boxed(A)\n  pub Empty\nend\n\n",
            "pub type Phase\n  pub Started\nend\n",
            "pub type PhaseAlias = Phase\n\n",
            "fn cross_base(value: Left::LeftReady | Right::RightReady) -> Int\n  0\nend\n\n",
            "fn different_args(value: Box<Int>::Boxed | Box<String>::Empty) -> Int\n  0\nend\n\n",
            "fn valid_alias_args(value: Box<PhaseAlias>::Boxed | Box<Phase>::Empty) -> Int\n  0\nend\n",
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", text)]);
        let position = |line_text: &str, needle: &str| {
            let (line, source_line) = text
                .lines()
                .enumerate()
                .find(|(_, candidate)| candidate.contains(line_text))
                .unwrap();
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: line + 1,
                column: source_line.find(needle).unwrap() + 1,
            }
        };

        for (line_text, needle) in [
            ("cross_base", "Left::"),
            ("cross_base", "LeftReady"),
            ("cross_base", "Right::"),
            ("cross_base", "RightReady"),
            ("different_args", "Box<Int>"),
            ("different_args", "Boxed"),
            ("different_args", "Box<String>"),
            ("different_args", "Empty"),
        ] {
            assert!(definition_at(&snapshot, position(line_text, needle)).is_none());
            assert!(navigate(&snapshot, position(line_text, needle)).is_none());
            assert!(navigate_for_rename(&snapshot, position(line_text, needle)).is_none());
        }

        for needle in ["Box<PhaseAlias>", "Boxed", "Box<Phase>", "Empty"] {
            assert!(definition_at(&snapshot, position("valid_alias_args", needle)).is_some());
            assert!(navigate(&snapshot, position("valid_alias_args", needle)).is_some());
            assert!(
                navigate_for_rename(&snapshot, position("valid_alias_args", needle)).is_some()
            );
        }

        let valid = navigate(&snapshot, position("valid_alias_args", "Boxed")).unwrap();
        assert!(
            valid
                .references
                .iter()
                .all(|location| location.start.line != position("different_args", "Boxed").line)
        );
    }

    #[test]
    fn retained_package_refinement_aliases_use_terminal_adt_arity() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                ("facade.veln", "use core\n\npub type Alias = core::Box\n"),
                (
                    "core.veln",
                    "pub type Box<A>\n  pub Boxed(A)\nend\n",
                ),
            ],
            ["facade.veln", "core.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[
                ("prelude.veln", "use boxes\n\npub type StandardAlias = boxes::Box\n"),
                (
                    "boxes.veln",
                    "pub type Box<A>\n  pub Boxed(A)\nend\n",
                ),
            ],
            ["prelude.veln", "boxes.veln"],
        );
        let text = concat!(
            "use facade from \"example/pkg\"\n\n",
            "fn dependency_valid(value: facade::Alias<Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn dependency_missing(value: facade::Alias::Boxed) -> Int\n  0\nend\n\n",
            "fn dependency_excess(value: facade::Alias<Int, Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn standard_valid(value: StandardAlias<Int>::Boxed) -> Int\n  0\nend\n\n",
            "fn standard_missing(value: StandardAlias::Boxed) -> Int\n  0\nend\n\n",
            "fn standard_excess(value: StandardAlias<Int, Int>::Boxed) -> Int\n  0\nend\n",
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", text)],
            vec![dependency],
        )
        .with_standard_library(standard_library);
        let position = |line_text: &str, needle: &str| {
            let (line, source_line) = text
                .lines()
                .enumerate()
                .find(|(_, candidate)| candidate.contains(line_text))
                .unwrap();
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: line + 1,
                column: source_line.find(needle).unwrap() + 1,
            }
        };

        for line_text in ["dependency_valid", "standard_valid"] {
            assert!(navigate(&snapshot, position(line_text, "Boxed")).is_some());
        }
        for line_text in [
            "dependency_missing",
            "dependency_excess",
            "standard_missing",
            "standard_excess",
        ] {
            assert!(navigate(&snapshot, position(line_text, "Boxed")).is_none());
        }
    }

    #[test]
    fn variant_refinement_navigation_preserves_unicode_scalar_ranges() {
        let source_text = concat!(
            "pub type State\n",
            "  pub Ready(Int)\n",
            "end\n\n",
            "fn observe(value: State::Ready) -> {label: String, state: State}\n",
            "  {label: \"😀\", state: keep<State::Ready>(\"x\", value)}\n",
            "end\n",
        );
        let parsed = veln_syntax::parse(&source("main.veln", source_text));
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", source_text)]);
        let result = query_snapshot(&snapshot, "main.veln", 6, 36).unwrap();

        assert_eq!(
            (
                result.selection.start.line,
                result.selection.start.column,
                result.selection.end.line,
                result.selection.end.column,
            ),
            (6, 35, 6, 40)
        );
        assert_eq!(
            (
                result.definition.span.start.line,
                result.definition.span.start.column,
                result.definition.span.end.line,
                result.definition.span.end.column,
            ),
            (2, 7, 2, 12)
        );
        assert_eq!(
            result
                .references
                .iter()
                .map(|span| (
                    span.start.line,
                    span.start.column,
                    span.end.line,
                    span.end.column,
                ))
                .collect::<Vec<_>>(),
            vec![(5, 26, 5, 31), (6, 35, 6, 40)]
        );
        assert!(validate_rename(&result, "Prepared").is_ok());
    }

    type VariantRefinementNavigationWork =
        (usize, usize, usize, usize, usize, usize, usize, usize, usize);

    fn variant_refinement_lookup_work(
        annotation_count: usize,
        rename: bool,
    ) -> VariantRefinementNavigationWork {
        reset_variant_refinement_navigation_work();
        let mut text =
            String::from("type State\n  Ready(Int)\nend\npub type Alias = State\n\n");
        for index in 0..annotation_count {
            text.push_str(&format!(
                "fn observe_{index}(value: Alias::Ready) -> State\n  State::Ready({index})\nend\n"
            ));
        }
        let snapshot_started = std::time::Instant::now();
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &text)]);
        let snapshot_elapsed = snapshot_started.elapsed();
        let index_started = std::time::Instant::now();
        snapshot.navigation_index();
        let index_elapsed = index_started.elapsed();
        let position = SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 2,
            column: 3,
        };
        let query_started = std::time::Instant::now();
        let result = if rename {
            navigate_for_rename(&snapshot, position)
        } else {
            query_snapshot(&snapshot, "main.veln", 2, 3)
        }
        .expect("constructor declaration resolves");
        let query_elapsed = query_started.elapsed();
        assert_eq!(result.references.len(), annotation_count * 2);
        eprintln!(
            "variant refinement stages count={annotation_count} rename={rename}: snapshot={snapshot_elapsed:?} index={index_elapsed:?} query={query_elapsed:?}"
        );
        variant_refinement_navigation_work()
    }

    fn assert_all_variant_refinement_work_recorded(work: VariantRefinementNavigationWork) {
        assert!(
            work.0 > 0
                && work.1 > 0
                && work.2 > 0
                && work.3 > 0
                && work.4 > 0
                && work.5 > 0
                && work.6 > 0
                && work.7 > 0
        );
    }

    fn assert_adjacent_variant_refinement_work(
        smaller: VariantRefinementNavigationWork,
        larger: VariantRefinementNavigationWork,
    ) {
        assert!(larger.0 > smaller.0);
        assert!(larger.2 > smaller.2);
        assert!(larger.3 > smaller.3);
        assert!(larger.4 > smaller.4);
        assert!(larger.0 <= smaller.0 * 2 + 32, "{smaller:?} -> {larger:?}");
        assert_eq!(larger.1, smaller.1, "{smaller:?} -> {larger:?}");
        assert!(larger.2 <= smaller.2 * 2 + 16, "{smaller:?} -> {larger:?}");
        assert!(larger.3 <= smaller.3 * 2 + 32, "{smaller:?} -> {larger:?}");
        assert!(larger.4 <= smaller.4 * 2 + 16, "{smaller:?} -> {larger:?}");
        assert_eq!(larger.5, smaller.5, "{smaller:?} -> {larger:?}");
        assert_eq!(larger.6, smaller.6, "{smaller:?} -> {larger:?}");
        assert!(larger.7 <= smaller.7 * 2 + 16, "{smaller:?} -> {larger:?}");
        assert!(larger.8 <= smaller.8 * 2 + 16, "{smaller:?} -> {larger:?}");
    }

    #[test]
    fn variant_refinement_navigation_work_is_adjacent_linear() {

        for rename in [false, true] {
            let mut evidence = Vec::new();
            for annotation_count in [256, 512, 1024, 2048] {
                let started = std::time::Instant::now();
                let work = variant_refinement_lookup_work(annotation_count, rename);
                evidence.push((annotation_count, work, started.elapsed()));
            }
            for window in evidence.windows(2) {
                let (_, smaller, _) = window[0];
                let (_, larger, _) = window[1];
                assert_all_variant_refinement_work_recorded(smaller);
                assert_adjacent_variant_refinement_work(smaller, larger);
            }
            eprintln!("variant refinement navigation evidence rename={rename}: {evidence:?}");
        }
    }

    fn workspace_variant_refinement_alias_depth_work(
        depth: usize,
    ) -> VariantRefinementNavigationWork {
        reset_variant_refinement_navigation_work();
        let mut text = String::from("type State\n  Ready(Int)\nend\n");
        for index in 0..depth {
            let target = if index == 0 {
                "State".to_string()
            } else {
                format!("Alias{}", index - 1)
            };
            text.push_str(&format!("pub type Alias{index} = {target}\n"));
        }
        let alias = format!("Alias{}", depth - 1);
        text.push_str(&format!(
            "\nfn observe(value: {alias}::Ready | {alias}::Ready) -> State\n  let made = {alias}::Ready(1)\n  match made\n    {alias}::Ready(inner) => made\n  end\nend\n"
        ));
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &text)]);
        let position = || SourcePosition {
            source: SourcePath::new("main.veln"),
            line: depth + 5,
            column: 21 + alias.len(),
        };
        let definition = definition_at(&snapshot, position()).unwrap();
        assert_location(&definition, "main.veln", 2, 3);
        let navigation = navigate(&snapshot, position()).unwrap();
        assert_eq!(navigation.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(navigation.references.len(), 4);
        let rename = navigate_for_rename(&snapshot, position()).unwrap();
        assert!(validate_rename_in_snapshot(&snapshot, &rename, "Prepared").is_ok());
        variant_refinement_navigation_work()
    }

    fn assert_adjacent_alias_resolution_work(
        smaller: VariantRefinementNavigationWork,
        larger: VariantRefinementNavigationWork,
    ) {
        assert!(smaller.5 > 0 && smaller.6 > 0 && smaller.7 > 0);
        assert!(larger.5 <= smaller.5 * 2 + 8, "{smaller:?} -> {larger:?}");
        assert!(larger.6 <= smaller.6 * 2 + 8, "{smaller:?} -> {larger:?}");
        assert!(larger.7 <= smaller.7 * 2 + 16, "{smaller:?} -> {larger:?}");
    }

    #[test]
    fn workspace_variant_refinement_alias_depth_has_bounded_work_and_shared_results() {

        let mut evidence = Vec::new();
        for depth in [64, 128, 256] {
            let started = std::time::Instant::now();
            evidence.push((
                depth,
                workspace_variant_refinement_alias_depth_work(depth),
                started.elapsed(),
            ));
        }
        for window in evidence.windows(2) {
            let (_, smaller, _) = window[0];
            let (_, larger, _) = window[1];
            assert_adjacent_alias_resolution_work(smaller, larger);
        }
        eprintln!("workspace variant-refinement alias depth evidence: {evidence:?}");
    }

    fn same_named_workspace_alias_chain_candidate_visits(
        depth: usize,
    ) -> (usize, std::time::Duration) {
        let mut sources = Vec::with_capacity(depth + 1);
        sources.push(source(
            "layer0.veln",
            concat!(
                "pub type State\n",
                "  pub Ready(Int)\n",
                "end\n\n",
                "pub type Alias = State\n",
            ),
        ));
        for index in 1..depth {
            let previous = format!("layer{}", index - 1);
            sources.push(SourceFile::new(
                format!("layer{index}.veln"),
                format!("use {previous}\n\npub type Alias = {previous}::Alias\n"),
            ));
        }
        let final_module = format!("layer{}", depth - 1);
        let consumer = format!(
            "use {final_module}\n\nfn observe(value: {final_module}::Alias::Ready) -> Int\n  0\nend\n"
        );
        let column = consumer
            .lines()
            .nth(2)
            .and_then(|line| line.find("Ready"))
            .expect("generated refinement contains Ready")
            + 1;
        sources.push(SourceFile::new("main.veln", consumer));

        let snapshot = EffectiveProjectSnapshot::new(sources);
        snapshot.navigation_index();
        crate::navigation::reset_type_namespace_candidate_visits();
        let started = std::time::Instant::now();
        let definition = definition_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 3,
                column,
            },
        )
        .expect("same-named qualified alias chain resolves");
        let elapsed = started.elapsed();
        assert_location(&definition, "layer0.veln", 2, 7);
        (
            crate::navigation::type_namespace_candidate_visits(),
            elapsed,
        )
    }

    #[test]
    fn same_named_workspace_alias_chain_candidate_visits_are_adjacent_linear() {
        let mut evidence = Vec::new();
        for depth in [64, 128, 256] {
            let (visits, elapsed) = same_named_workspace_alias_chain_candidate_visits(depth);
            evidence.push((depth, visits, elapsed));
        }
        eprintln!("same-named workspace alias chain evidence: {evidence:?}");
        for window in evidence.windows(2) {
            let (_, smaller, _) = window[0];
            let (_, larger, _) = window[1];
            assert!(smaller > 0, "candidate visits must cover alias target lookup");
            assert!(larger > smaller, "{smaller} -> {larger}");
            assert!(larger <= smaller * 2 + 16, "{smaller} -> {larger}");
        }
    }

    fn nested_generic_alias_rename_work(
        depth: usize,
        close_suffix: bool,
    ) -> (VariantRefinementNavigationWork, std::time::Duration) {
        reset_variant_refinement_navigation_work();
        let mut nested = "Alias<".repeat(depth);
        nested.push_str("Int");
        if close_suffix {
            nested.push_str(&">".repeat(depth));
            nested.push_str("::Ready");
        }
        let text = format!(
            "type Box<A>\n  Ready(A)\nend\n\npub type Alias = Box\n\nfn observe(value: {nested}) -> Int\n  0\nend\n"
        );
        let started = std::time::Instant::now();
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &text)]);
        let result = navigate_for_rename(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 10,
            },
        )
        .expect("alias declaration remains renameable");
        if close_suffix {
            // Deep parser recovery can withhold the enclosing refinement base,
            // while every nested ordinary alias reference remains selectable.
            assert!(
                result.references.len() >= depth.saturating_sub(1),
                "depth={depth}, references={}",
                result.references.len()
            );
        }
        (variant_refinement_navigation_work(), started.elapsed())
    }

    #[test]
    fn nested_generic_refinement_alias_rename_has_linear_lookup_work() {
        for (label, close_suffix, depths) in [
            ("valid", true, [64, 128]),
            ("bounded-malformed", false, [64, 128]),
        ] {
            let smaller = nested_generic_alias_rename_work(depths[0], close_suffix);
            let larger = nested_generic_alias_rename_work(depths[1], close_suffix);
            assert!(smaller.0.8 >= depths[0], "{label}: {smaller:?}");
            assert!(larger.0.8 > smaller.0.8, "{label}: {smaller:?} -> {larger:?}");
            assert!(
                larger.0.8 <= smaller.0.8 * 2 + 8,
                "{label}: {smaller:?} -> {larger:?}"
            );
            eprintln!(
                "nested generic refinement alias rename {label}: {}=({:?}, {:?}), {}=({:?}, {:?})",
                depths[0], smaller.0.8, smaller.1, depths[1], larger.0.8, larger.1
            );
        }
    }

    fn retained_variant_refinement_alias_snapshot(
        depth: usize,
    ) -> (EffectiveProjectSnapshot, usize) {
        let boundaries = [depth / 4, depth / 2, depth * 3 / 4, depth];
        let modules = ["core", "layer_a", "layer_b", "facade"];
        let mut contents = [
            String::from("mod core\n\npub type State\n  pub Ready(Int)\nend\n\n"),
            String::from("mod layer_a\nuse core\n\n"),
            String::from("mod layer_b\nuse layer_a\n\n"),
            String::from("mod facade\nuse layer_b\n\n"),
        ];
        let mut start = 0;
        for (part, end) in boundaries.into_iter().enumerate() {
            for index in start..end {
                let target = if index == 0 {
                    "State".to_string()
                } else if index == start {
                    format!("{}::Alias{}", modules[part - 1], index - 1)
                } else {
                    format!("Alias{}", index - 1)
                };
                contents[part].push_str(&format!("pub type Alias{index} = {target}\n"));
            }
            start = end;
        }
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                ("core.veln", &contents[0]),
                ("layer_a.veln", &contents[1]),
                ("layer_b.veln", &contents[2]),
                ("facade.veln", &contents[3]),
            ],
            ["core.veln", "layer_a.veln", "layer_b.veln", "facade.veln"],
        );
        let alias = format!("Alias{}", depth - 1);
        let base = format!("facade::{alias}");
        let text = format!(
            "use facade from \"example/dep\"\n\nfn observe(value: {base}::Ready | {base}::Ready) -> Int\n  0\nend\n"
        );
        (
            EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &text)],
                vec![dependency],
            ),
            21 + base.len(),
        )
    }

    fn retained_variant_refinement_alias_depth_work(
        depth: usize,
    ) -> VariantRefinementNavigationWork {
        reset_variant_refinement_navigation_work();
        let (snapshot, column) = retained_variant_refinement_alias_snapshot(depth);
        let position = || SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 3,
            column,
        };
        let result = navigate(&snapshot, position()).unwrap_or_else(|| {
            panic!(
                "retained depth {depth} did not resolve; base={:?} work={:?}",
                query_snapshot(&snapshot, "main.veln", 3, column - 9),
                variant_refinement_navigation_work()
            )
        });
        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(result.references.len(), 2);
        assert!(definition_at(&snapshot, position()).is_some());
        assert!(navigate_for_rename(&snapshot, position()).is_some());
        variant_refinement_navigation_work()
    }

    #[test]
    fn retained_variant_refinement_alias_depth_is_iterative_and_namespace_aware() {

        let mut evidence = Vec::new();
        for depth in [64, 128, 256] {
            let started = std::time::Instant::now();
            evidence.push((
                depth,
                retained_variant_refinement_alias_depth_work(depth),
                started.elapsed(),
            ));
        }
        for window in evidence.windows(2) {
            let (_, smaller, _) = window[0];
            let (_, larger, _) = window[1];
            assert_adjacent_alias_resolution_work(smaller, larger);
        }
        let started = std::time::Instant::now();
        let deep = retained_variant_refinement_alias_depth_work(2048);
        let deep_elapsed = started.elapsed();
        assert!(deep.6 <= 2056, "unexpected terminal work: {deep:?}");
        eprintln!(
            "retained variant-refinement alias depth evidence: {evidence:?}, deep=(2048, {deep:?}, {deep_elapsed:?})"
        );
    }

    #[test]
    fn variant_refinement_alias_cycles_cache_terminal_failure() {
        reset_variant_refinement_navigation_work();
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "type State\n",
                "  Ready(Int)\n",
                "end\n",
                "pub type A = B\n",
                "pub type B = A\n\n",
                "fn observe(first: A::Ready, second: B::Ready) -> Int\n",
                "  0\n",
                "end\n",
            ),
        )]);
        assert!(query_snapshot(&snapshot, "main.veln", 7, 25).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 7, 43).is_none());
        let work = variant_refinement_navigation_work();
        assert_eq!(work.6, 2, "each cyclic alias target is inspected once: {work:?}");
        assert!(work.7 >= 2, "cycle failures should be reused: {work:?}");
    }

    #[test]
    fn variant_refinement_constructor_bucket_ignores_unrelated_constructors() {
        fn candidate_work(unrelated_count: usize) -> usize {
            reset_variant_refinement_navigation_work();
            let mut text = String::from("type State\n  Ready(Int)\nend\n");
            for index in 0..unrelated_count {
                text.push_str(&format!("type Other{index}\n  Ready(Int)\nend\n"));
            }
            text.push_str("fn observe(value: State::Ready) -> Int\n  0\nend\n");
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &text)]);
            let line = unrelated_count * 3 + 4;
            query_snapshot(&snapshot, "main.veln", line, 27).unwrap();
            variant_refinement_navigation_work().2
        }

        assert_eq!(candidate_work(16), candidate_work(256));
    }
