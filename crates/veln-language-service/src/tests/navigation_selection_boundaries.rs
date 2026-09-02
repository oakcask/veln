    #[test]
    fn invalid_recovery_navigation_rejects_qualified_occurrences() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "fn Bad() -> Int\n",
                "  1\n",
                "end\n\n",
                "fn caller() -> Int\n",
                "  missing::Bad()\n",
                "end\n",
            ),
        )];

        assert!(query(sources.clone(), "main.veln", 6, 12).is_none());
        assert!(query(sources, "main.veln", 6, 4).is_none());
    }

    #[test]
    fn invalid_binding_recovery_navigation_rejects_shadowing_and_ambiguity() {
        let shadowed = vec![source(
            "main.veln",
            concat!(
                "fn main(Bad: Int) -> Int\n",
                "  match Bad\n",
                "    Bad => Bad\n",
                "  end\n",
                "end\n",
            ),
        )];
        assert!(query(shadowed, "main.veln", 3, 12).is_none());

        let ambiguous = vec![source(
            "main.veln",
            "fn main(Bad: Int, Bad: Int) -> Int\n  Bad\nend\n",
        )];
        assert!(query(ambiguous, "main.veln", 2, 4).is_none());
    }

    #[test]
    fn rename_validation_preserves_function_case_class() {
        let result = query(
            vec![source(
                "main.veln",
                "fn increment(value: Int) -> Int\n  increment(value)\nend\n",
            )],
            "main.veln",
            1,
            4,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert!(validate_rename(&result, "advance").is_ok());
        assert_rename_invalid_case(
            validate_rename(&result, "Advance").unwrap_err(),
            RenameNameClass::Function,
            "Advance",
            RenameRequiredInitial::AsciiLowercase,
        );
    }

    #[test]
    fn rename_validation_preserves_type_case_class() {
        let result = query(
            vec![source(
                "main.veln",
                "type Item\n  Value(value: Int)\nend\n\nfn main(input: Item) -> Item\n  input\nend\n",
            )],
            "main.veln",
            1,
            6,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 5, 16), ("main.veln", 5, 25)]
        );
        assert!(validate_rename(&result, "Entry").is_ok());
        assert_rename_invalid_case(
            validate_rename(&result, "entry").unwrap_err(),
            RenameNameClass::Type,
            "entry",
            RenameRequiredInitial::AsciiUppercase,
        );
    }

    #[test]
    fn recovery_rename_validation_preserves_case_classes() {
        let cases = [
            (
                "type",
                "type item\n  Value\nend\n\nfn read(value: item) -> item\n  value\nend\n",
                5,
                16,
                "Entry",
                "entry",
                RenameNameClass::Type,
                RenameRequiredInitial::AsciiUppercase,
            ),
            (
                "constructor",
                "type Item\n  value(input: Int)\nend\n\nfn read() -> Item\n  value(1)\nend\n",
                6,
                4,
                "Value",
                "value",
                RenameNameClass::Constructor,
                RenameRequiredInitial::AsciiUppercase,
            ),
            (
                "function",
                "fn Bad() -> Int\n  Bad()\nend\n\nfn read() -> Int\n  Bad()\nend\n",
                6,
                4,
                "good",
                "Good",
                RenameNameClass::Function,
                RenameRequiredInitial::AsciiLowercase,
            ),
            (
                "binding",
                "fn read(Input: Int) -> Int\n  Input\nend\n",
                2,
                4,
                "input",
                "Input",
                RenameNameClass::ValueBinding,
                RenameRequiredInitial::AsciiLowercase,
            ),
        ];

        for (name, text, line, column, valid, invalid, class, required) in cases {
            let result = query(vec![source("main.veln", text)], "main.veln", line, column)
                .unwrap_or_else(|| panic!("{name} recovery should be selected"));
            assert!(result.is_recovery, "{name} selected a valid symbol");
            assert!(validate_rename(&result, valid).is_ok(), "{name}");
            assert_rename_invalid_case(
                validate_rename(&result, invalid).unwrap_err(),
                class,
                invalid,
                required,
            );
        }
    }

    #[test]
    fn type_selection_keeps_same_named_non_type_namespace_tokens_in_role() {
        let source_text = concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "schema Item\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "effect Item\n",
            "  Item() -> Int\n",
            "end\n\n",
            "fn main(input: Item) -> Item\n",
            "  input\n",
            "end\n",
        );
        let sources = vec![source("main.veln", source_text)];

        for (line, column, kind) in [
            (5, 8, SymbolKind::Schema),
            (10, 8, SymbolKind::Effect),
            (11, 4, SymbolKind::EffectOperation),
        ] {
            let result = query(sources.clone(), "main.veln", line, column)
                .unwrap_or_else(|| panic!("{line}:{column} should select its own namespace"));
            assert_eq!(result.selected_symbol.kind, kind);
        }

        let result = query(sources, "main.veln", 14, 16).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 14, 16), ("main.veln", 14, 25)]
        );
    }

    #[test]
    fn type_selection_requires_unique_visible_type_identity() {
        let sources = vec![
            source("left.veln", "pub type Item\n  Left\nend\n"),
            source("right.veln", "pub type Item\n  Right\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n",
                    "\n",
                    "fn bare(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                    "\n",
                    "fn left_value(value: left::Item) -> left::Item\n",
                    "  value\n",
                    "end\n",
                    "\n",
                    "fn right_value(value: right::Item) -> right::Item\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ];

        assert!(query(sources.clone(), "main.veln", 4, 16).is_none());
        assert!(query(sources.clone(), "main.veln", 4, 25).is_none());

        let left_result = query(sources.clone(), "main.veln", 8, 28).unwrap();
        assert_eq!(left_result.selected_symbol.kind, SymbolKind::Type);
        assert_location(&left_result.definition, "left.veln", 1, 10);
        assert_eq!(
            locations(&left_result.references),
            [("main.veln", 8, 28), ("main.veln", 8, 43)]
        );

        let right_result = query(sources, "main.veln", 12, 30).unwrap();
        assert_eq!(right_result.selected_symbol.kind, SymbolKind::Type);
        assert_location(&right_result.definition, "right.veln", 1, 10);
        assert_eq!(
            locations(&right_result.references),
            [("main.veln", 12, 30), ("main.veln", 12, 46)]
        );
    }
