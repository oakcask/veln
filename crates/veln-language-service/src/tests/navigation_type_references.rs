
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
    fn variant_refinement_constructor_role_lookup_work_is_adjacent_linear() {
        fn lookup_work(annotation_count: usize, rename: bool) -> usize {
            let mut text = String::from(
                "type State\n  Ready(Int)\nend\npub type Alias = State\n\n",
            );
            for index in 0..annotation_count {
                text.push_str(&format!(
                    "fn observe_{index}(value: Alias::Ready) -> State\n  State::Ready({index})\nend\n"
                ));
            }
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", &text)]);
            reset_classified_role_lookups();
            let position = SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 3,
            };
            let result = if rename {
                navigate_for_rename(&snapshot, position)
            } else {
                query_snapshot(&snapshot, "main.veln", 2, 3)
            }
            .expect("constructor declaration resolves");
            assert_eq!(result.references.len(), annotation_count * 2);
            classified_role_lookups()
        }

        for rename in [false, true] {
            let smaller = lookup_work(128, rename);
            let larger = lookup_work(256, rename);
            assert!(smaller > 0);
            assert!(larger > smaller);
            assert!(larger <= smaller * 2 + 16, "{smaller} -> {larger}");
        }
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
