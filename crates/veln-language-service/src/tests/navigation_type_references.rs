
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
