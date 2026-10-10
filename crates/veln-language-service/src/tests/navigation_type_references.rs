
    #[test]
    fn type_references_cover_syntax_retained_type_roles() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Value(value: Int)\n",
                    "end\n\n",
                    "type Box\n",
                    "  Wrap(Item)\n",
                    "end\n\n",
                    "pub type Exported = Item\n\n",
                    "effect Choose\n",
                    "  pick(value: Bool) -> Item\n",
                    "end\n\n",
                    "fn main(input: Item) -> Item\n",
                    "  let current: Item = input\n",
                    "  current\n",
                    "end\n",
                ),
            )],
            "main.veln",
            1,
            6,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 6, 8),
                ("main.veln", 9, 21),
                ("main.veln", 12, 24),
                ("main.veln", 15, 16),
                ("main.veln", 15, 25),
                ("main.veln", 16, 16),
            ]
        );
    }

    #[test]
    fn type_references_cover_constructor_qualified_type_segments() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "type Item\n",
                "  Some(Int)\n",
                "  None\n",
                "end\n\n",
                "fn make() -> Item\n",
                "  Item::Some(1)\n",
                "end\n\n",
                "fn observe(input: Item) -> Int\n",
                "  match input\n",
                "    Item::None => 0\n",
                "    Item::Some(value) => value\n",
                "  end\n",
                "end\n",
            ),
        )];

        let declaration = query(sources.clone(), "main.veln", 1, 6).unwrap();

        assert_eq!(declaration.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            locations(&declaration.references),
            [
                ("main.veln", 6, 14),
                ("main.veln", 7, 3),
                ("main.veln", 10, 19),
                ("main.veln", 12, 5),
                ("main.veln", 13, 5),
            ]
        );

        let qualifier = query(sources, "main.veln", 7, 4).unwrap();
        assert_eq!(qualifier.selected_symbol.kind, SymbolKind::Type);
        assert_location(&qualifier.definition, "main.veln", 1, 6);
        assert!(validate_rename(&qualifier, "Entry").is_ok());
        assert_rename_invalid_case(
            validate_rename(&qualifier, "entry").unwrap_err(),
            RenameNameClass::Type,
            "entry",
            RenameRequiredInitial::AsciiUppercase,
        );
    }

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

    #[test]
    fn variant_refinement_aliases_resolve_target_origins_independently() {
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

        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "facade.veln",
                    "use model\n\npub type PublicState = model::State\n",
                ),
                (
                    "model.veln",
                    "pub type State\n  pub Ready(Int)\n  pub Closed\nend\n",
                ),
            ],
            ["facade.veln", "model.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "use states\n\npub type StandardState = states::State\n",
                ),
                (
                    "states.veln",
                    "pub type State\n  pub Ready(Int)\nend\n",
                ),
            ],
            ["prelude.veln", "states.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use facade from \"example/pkg\"\n\n",
                    "pub type Local = facade::PublicState\n",
                    "pub type Transit = Local\n\n",
                    "fn local(value: Transit::Ready | Local::Closed) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn dependency(value: facade::PublicState::Ready) -> Int\n",
                    "  0\n",
                    "end\n\n",
                    "fn standard(value: StandardState::Ready) -> Int\n",
                    "  0\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        let local_base = query_snapshot(&snapshot, "main.veln", 6, 18).unwrap();
        assert_location(&local_base.definition, "main.veln", 4, 10);
        let local_variant = query_snapshot(&snapshot, "main.veln", 6, 27).unwrap();
        assert_package_location(&local_variant.definition, "model.veln", 2, 7);
        assert_eq!(
            local_variant.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );

        let dependency_base = query_snapshot(&snapshot, "main.veln", 10, 37).unwrap();
        assert_eq!(
            dependency_base.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_package_location(&dependency_base.definition, "facade.veln", 3, 10);
        let dependency_base_definition = definition_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 10,
                column: 37,
            },
        )
        .expect("direct dependency refinement alias has a public definition");
        assert_package_location(&dependency_base_definition, "facade.veln", 3, 10);
        let dependency_variant = query_snapshot(&snapshot, "main.veln", 10, 44).unwrap();
        assert_package_location(&dependency_variant.definition, "model.veln", 2, 7);

        let standard_base = query_snapshot(&snapshot, "main.veln", 14, 23).unwrap();
        assert_package_location(&standard_base.definition, "prelude.veln", 3, 10);
        let standard_base_definition = definition_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 14,
                column: 23,
            },
        )
        .expect("standard-library refinement alias has a public definition");
        assert_package_location(&standard_base_definition, "prelude.veln", 3, 10);
        let standard_variant = query_snapshot(&snapshot, "main.veln", 14, 36).unwrap();
        assert_package_location(&standard_variant.definition, "states.veln", 2, 7);
        assert_eq!(
            standard_variant.selected_symbol.package_origin,
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

    #[test]
    fn variant_refinement_navigation_work_is_adjacent_linear() {
        fn lookup_work(
            annotation_count: usize,
            rename: bool,
        ) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
            reset_variant_refinement_navigation_work();
            let mut text = String::from(
                "type State\n  Ready(Int)\nend\npub type Alias = State\n\n",
            );
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

        for rename in [false, true] {
            let mut evidence = Vec::new();
            for annotation_count in [256, 512, 1024, 2048] {
                let started = std::time::Instant::now();
                let work = lookup_work(annotation_count, rename);
                evidence.push((annotation_count, work, started.elapsed()));
            }
            for window in evidence.windows(2) {
                let (_, smaller, _) = window[0];
                let (_, larger, _) = window[1];
                assert!(
                    smaller.0 > 0
                        && smaller.1 > 0
                        && smaller.2 > 0
                        && smaller.3 > 0
                        && smaller.4 > 0
                        && smaller.5 > 0
                        && smaller.6 > 0
                        && smaller.7 > 0
                );
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
            }
            eprintln!("variant refinement navigation evidence rename={rename}: {evidence:?}");
        }
    }

    #[test]
    fn workspace_variant_refinement_alias_depth_has_bounded_work_and_shared_results() {
        fn depth_work(
            depth: usize,
        ) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
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

        let mut evidence = Vec::new();
        for depth in [64, 128, 256] {
            let started = std::time::Instant::now();
            evidence.push((depth, depth_work(depth), started.elapsed()));
        }
        for window in evidence.windows(2) {
            let (_, smaller, _) = window[0];
            let (_, larger, _) = window[1];
            assert!(smaller.5 > 0 && smaller.6 > 0 && smaller.7 > 0);
            assert!(larger.5 <= smaller.5 * 2 + 8, "{smaller:?} -> {larger:?}");
            assert!(larger.6 <= smaller.6 * 2 + 8, "{smaller:?} -> {larger:?}");
            assert!(larger.7 <= smaller.7 * 2 + 16, "{smaller:?} -> {larger:?}");
        }
        eprintln!("workspace variant-refinement alias depth evidence: {evidence:?}");
    }

    #[test]
    fn retained_variant_refinement_alias_depth_is_iterative_and_namespace_aware() {
        fn retained_snapshot(depth: usize) -> (EffectiveProjectSnapshot, usize) {
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

        fn depth_work(
            depth: usize,
        ) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
            reset_variant_refinement_navigation_work();
            let (snapshot, column) = retained_snapshot(depth);
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
            assert_eq!(result.selected_symbol.package_origin, Some(PackageOrigin::DirectDependency));
            assert_eq!(result.references.len(), 2);
            assert!(definition_at(&snapshot, position()).is_some());
            assert!(navigate_for_rename(&snapshot, position()).is_some());
            variant_refinement_navigation_work()
        }

        let mut evidence = Vec::new();
        for depth in [64, 128, 256] {
            let started = std::time::Instant::now();
            evidence.push((depth, depth_work(depth), started.elapsed()));
        }
        for window in evidence.windows(2) {
            let (_, smaller, _) = window[0];
            let (_, larger, _) = window[1];
            assert!(smaller.5 > 0 && smaller.6 > 0 && smaller.7 > 0);
            assert!(larger.5 <= smaller.5 * 2 + 8, "{smaller:?} -> {larger:?}");
            assert!(larger.6 <= smaller.6 * 2 + 8, "{smaller:?} -> {larger:?}");
            assert!(larger.7 <= smaller.7 * 2 + 16, "{smaller:?} -> {larger:?}");
        }
        let started = std::time::Instant::now();
        let deep = depth_work(2048);
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

    #[test]
    fn imported_constructor_qualified_type_segments_share_navigation() {
        let sources = vec![
            source("helper.veln", "pub type Entry\n  pub Some(Int)\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use helper\n\n",
                    "fn make() -> helper::Entry\n",
                    "  helper::Entry::Some(1)\n",
                    "end\n\n",
                    "fn read(input: helper::Entry) -> Int\n",
                    "  match input\n",
                    "    helper::Entry::Some(value) => value\n",
                    "  end\n",
                    "end\n",
                ),
            ),
        ];

        assert!(query(sources.clone(), "main.veln", 3, 15).is_none());

        let qualifier = query(sources, "main.veln", 4, 12).unwrap();
        assert_eq!(qualifier.selected_symbol.kind, SymbolKind::Type);
        assert_location(&qualifier.definition, "helper.veln", 1, 10);
        assert_eq!(
            locations(&qualifier.references),
            [
                ("main.veln", 3, 22),
                ("main.veln", 4, 11),
                ("main.veln", 7, 24),
                ("main.veln", 9, 13),
            ]
        );
        assert!(validate_rename(&qualifier, "Item").is_ok());
        assert_rename_invalid_case(
            validate_rename(&qualifier, "item").unwrap_err(),
            RenameNameClass::Type,
            "item",
            RenameRequiredInitial::AsciiUppercase,
        );
    }

    #[test]
    fn invalid_type_recovery_reference_lookup_reuses_type_reference_collection() {
        for function_count in [40, 80] {
            let mut source_text = String::from("type item\n  Value\nend\n");
            for index in 0..function_count {
                source_text.push_str(&format!(
                    "\nfn use_item_{index}(input: item) -> item\n  let current: item = input\n  input\nend\n"
                ));
            }
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)]);
            reset_type_reference_collections();

            let result = query_snapshot(&snapshot, "main.veln", 1, 6).unwrap();

            assert!(result.is_recovery);
            assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
            assert_eq!(result.references.len(), function_count * 3);
            assert_eq!(type_reference_collections(), 1);
        }
    }

    #[test]
    fn named_type_reference_lookup_skips_unrelated_type_references() {
        let mut source_text = String::from("type Item\n  Value\nend\n");
        for index in 0..128 {
            source_text.push_str(&format!(
                "\nfn use_other_{index}(input: Other{index}) -> Other{index}\n  input\nend\n"
            ));
        }
        source_text.push_str("\nfn use_item(input: Item) -> Item\n  input\nend\n");
        let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)]);
        reset_type_reference_collections();

        let result = query_snapshot(&snapshot, "main.veln", 1, 6)
            .expect("declared type should resolve");

        assert_eq!(result.references.len(), 2);
        assert_eq!(type_reference_collections(), 1);
        assert_eq!(type_reference_candidate_visits(), 2);
    }

    #[test]
    fn cleanup_annotation_type_reference_token_work_is_adjacent_linear() {
        fn token_visits(annotation_count: usize) -> usize {
            let mut source_text = String::from("type Item\n  Value\nend\n\nfn main(input: Item) -> Item\n  let region: Item = begin\n");
            for index in 0..annotation_count {
                source_text.push_str(&format!("    let value{index}: Item = input\n"));
            }
            source_text.push_str("    input\n  end\n  region\nend\n");
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)]);
            reset_type_reference_collections();

            let result = query_snapshot(&snapshot, "main.veln", 1, 6)
                .expect("declared type should resolve");

            assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
            assert_eq!(result.references.len(), annotation_count + 3);
            assert_eq!(type_reference_collections(), 1);
            type_reference_token_visits()
        }

        let smaller = token_visits(128);
        let larger = token_visits(256);

        eprintln!("cleanup annotation indexing: 128={smaller} visits, 256={larger} visits");
        assert!(smaller > 0);
        assert!(larger > smaller);
        assert!(larger <= smaller * 2 + 64, "{smaller} -> {larger}");
    }

    #[test]
    fn sibling_cleanup_annotation_collection_is_linear() {
        fn token_visits(annotation_count: usize) -> usize {
            let mut source_text = String::from(
                "type Item\n  Value\nend\n\nfn main(input: Item) -> Item\n  let region = ",
            );
            for index in 0..annotation_count {
                if index > 0 {
                    source_text.push_str(" + ");
                }
                source_text.push_str(&format!(
                    "begin\n    let value{index}: Item = input\n    input\n  end"
                ));
            }
            source_text.push_str("\n  input\nend\n");
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)]);
            reset_type_reference_collections();

            let result = query_snapshot(&snapshot, "main.veln", 1, 6)
                .expect("declared type should resolve");

            assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
            assert_eq!(result.references.len(), annotation_count + 2);
            assert_eq!(type_reference_collections(), 1);
            type_reference_token_visits()
        }

        let smaller = token_visits(64);
        let larger = token_visits(128);

        eprintln!("sibling cleanup annotation indexing: 64={smaller}, 128={larger}");
        assert!(smaller > 0);
        assert!(larger > smaller);
        assert!(larger <= smaller * 2 + 64, "{smaller} -> {larger}");
    }
