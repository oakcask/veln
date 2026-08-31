
    #[test]
    fn repeated_navigation_reuses_the_prepared_symbol_index() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "fn identity(value: Int) -> Int\n  identity(value)\nend\n",
        )]);

        let first_index = snapshot.navigation_index();
        let first = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 4,
            },
        )
        .unwrap();
        let second_index = snapshot.navigation_index();
        let second = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 4,
            },
        )
        .unwrap();

        assert!(Arc::ptr_eq(&first_index, &second_index));
        assert_eq!(first, second);
    }

    #[test]
    fn function_definition_and_references_are_deterministic() {
        let result = query(
            vec![
                source(
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
                ),
                source(
                    "math.veln",
                    "fn increment(value: Int) -> Int\n  increment(value - 1)\nend\n",
                ),
            ],
            "math.test.veln",
            4,
            11,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_location(&result.definition, "math.veln", 1, 4);
        assert_eq!(
            locations(&result.references),
            [("math.test.veln", 4, 9), ("math.veln", 2, 3)]
        );
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
    fn type_selection_excludes_same_named_non_type_namespace_tokens() {
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

        for (line, column) in [(5, 8), (10, 8), (11, 4)] {
            assert!(
                query(sources.clone(), "main.veln", line, column).is_none(),
                "{line}:{column} selected a type symbol"
            );
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

    #[test]
    fn rename_validation_rejects_same_namespace_conflicts_for_supported_classes() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "type Item\n",
                "  Value(value: Int)\n",
                "  Ready\n",
                "  Waiting\n",
                "end\n\n",
                "type Entry\n",
                "  Existing\n",
                "end\n\n",
                "effect Choose\n",
                "  pick(value: Bool, other: Bool) -> Bool\n",
                "end\n\n",
                "handler choose() handles Choose\n",
                "  pick(value, other) => value\n",
                "end\n\n",
                "fn convert(input: Item) -> Item\n",
                "  Value(1)\n",
                "end\n\n",
                "fn adapt(input: Item) -> Item\n",
                "  input\n",
                "end\n",
            ),
        )]);

        let cases = [
            (
                1,
                6,
                "Entry",
                RenameNameClass::Type,
                "main.veln",
                7,
                6,
            ),
            (
                3,
                3,
                "Waiting",
                RenameNameClass::Constructor,
                "main.veln",
                4,
                3,
            ),
            (
                19,
                4,
                "adapt",
                RenameNameClass::Function,
                "main.veln",
                23,
                4,
            ),
            (
                16,
                8,
                "other",
                RenameNameClass::ValueBinding,
                "main.veln",
                16,
                15,
            ),
        ];

        for (line, column, requested, class, path, conflict_line, conflict_column) in cases {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_rename_conflict(
                validate_rename_in_snapshot(&snapshot, &result, requested).unwrap_err(),
                class,
                requested,
                path,
                conflict_line,
                conflict_column,
            );
        }
    }

    #[test]
    fn rename_validation_rejects_type_conflict_with_public_type_alias() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Value\n",
                    "end\n\n",
                    "type Existing\n",
                    "  Present\n",
                    "end\n\n",
                    "pub type Entry = Existing\n",
                ),
            ),
            source("control.veln", "pub type Unrelated = Int\n"),
        ]);
        let result = query_snapshot(&snapshot, "main.veln", 1, 6).unwrap();

        let failure = validate_rename_in_snapshot(&snapshot, &result, "Entry").unwrap_err();

        assert_eq!(failure.code, "rename.conflict");
        assert_eq!(failure.symbol_class, RenameNameClass::Type);
        assert_eq!(failure.requested_name, "Entry");
        let RenameFailureKind::Conflict {
            conflicting_declaration,
            affected_scope,
        } = failure.kind
        else {
            panic!("rename failure was not a conflict");
        };
        assert_location(&conflicting_declaration, "main.veln", 9, 10);
        assert_eq!(
            *affected_scope,
            RenameAffectedScope::Module {
                name: "main".to_string(),
            }
        );
    }

    #[test]
    fn rename_validation_preserves_non_conflicting_same_class_renames() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Item\n  Left\nend\n"),
            source("right.veln", "pub type Other\n  Right\nend\n"),
            source("aliases.veln", "pub type Entry = Other\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn read(value: left::Item) -> left::Item\n",
                    "  value\n",
                    "end\n\n",
                    "fn shadow_control(value: Int) -> Int\n",
                    "  let entry = value\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "main.veln", 4, 24).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "Entry").is_ok());
    }

    #[test]
    fn rename_validation_allows_unedited_clause_parameter_shadowing() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  choose(value: Int) -> Int\n",
                "  current() -> Int\n",
                "end\n\n",
                "handler choose(ctx: Int) handles Choose\n",
                "  choose(value) => value\n",
                "  current() => ctx\n",
                "end\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "main.veln", 6, 16).unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerContextParameter
        );
        assert_eq!(locations(&result.references), [("main.veln", 8, 16)]);
        assert!(validate_rename_in_snapshot(&snapshot, &result, "value").is_ok());
    }

    #[test]
    fn rename_validation_rejects_unused_clause_parameter_declaration_conflict() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  choose(left: Int, right: Int) -> Int\n",
                "  other(value: Int, extra: Int) -> Int\n",
                "end\n\n",
                "handler choose() handles Choose\n",
                "  choose(left, right) => right\n",
                "  other(value, extra) => extra\n",
                "end\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "main.veln", 7, 10).unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerOperationClauseParameter
        );
        assert_eq!(locations(&result.references), []);
        let failure = validate_rename_in_snapshot(&snapshot, &result, "right").unwrap_err();

        assert_eq!(failure.code, "rename.conflict");
        assert_eq!(failure.symbol_class, RenameNameClass::ValueBinding);
        assert_eq!(failure.requested_name, "right");
        let RenameFailureKind::Conflict {
            conflicting_declaration,
            affected_scope,
        } = failure.kind
        else {
            panic!("rename failure was not a conflict");
        };
        assert_location(&conflicting_declaration, "main.veln", 7, 16);
        let RenameAffectedScope::Lexical {
            file,
            start_offset,
            end_offset,
        } = *affected_scope
        else {
            panic!("rename conflict did not report a lexical scope");
        };
        assert_eq!(file, "main.veln");
        assert!(start_offset > result.selected_symbol.declaration.span.end.offset);
        assert!(end_offset > start_offset);
    }

    #[test]
    fn rename_validation_allows_clause_parameter_name_in_unedited_clause() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  choose(left: Int, right: Int) -> Int\n",
                "  other(value: Int, extra: Int) -> Int\n",
                "end\n\n",
                "handler choose() handles Choose\n",
                "  choose(left, right) => right\n",
                "  other(value, extra) => extra\n",
                "end\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "main.veln", 7, 10).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "value").is_ok());
    }

    #[test]
    fn rename_validation_reports_local_binding_declaration_conflict() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "fn target(value: Int) -> Int\n",
                "  value\n",
                "end\n\n",
                "fn caller(value: Int) -> Int\n",
                "  let conflict = value\n",
                "  let observed = conflict\n",
                "  target(observed)\n",
                "end\n",
            ),
        )]);
        let result = query_snapshot(&snapshot, "main.veln", 8, 4).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "conflict").unwrap_err(),
            RenameNameClass::Function,
            "conflict",
            "main.veln",
            6,
            7,
        );
    }

    #[test]
    fn rename_validation_rejects_provable_multi_scope_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Item\n  Left\nend\n"),
            source("right.veln", "pub type Entry\n  Right\nend\n"),
            source("extra.veln", "pub type Entry\n  Extra\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "use extra\n\n",
                    "fn local(value: left::Item) -> left::Item\n",
                    "  value\n",
                    "end\n\n",
                    "fn imported(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 10).unwrap();

        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "Entry").unwrap_err(),
            RenameNameClass::Type,
            "Entry",
            "right.veln",
            1,
            10,
        );
    }

    #[test]
    fn rename_validation_rejects_provable_constructor_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source("right.veln", "pub type Target\n  pub Done\nend\n"),
            source("extra.veln", "pub type Other\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n",
                    "use extra\n\n",
                    "fn make() -> left::Source\n",
                    "  Ready\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "Done").unwrap_err(),
            RenameNameClass::Constructor,
            "Done",
            "right.veln",
            2,
            7,
        );
    }

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
