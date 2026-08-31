
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
    fn nested_import_alias_qualified_segments_share_navigation() {
        let sources = vec![
            source(
                "app/math.veln",
                concat!(
                    "pub type Item\n",
                    "  pub Ready(Int)\n",
                    "end\n\n",
                    "pub fn double(value: Int) -> Int\n",
                    "  value + value\n",
                    "end\n",
                ),
            ),
            source(
                "main.veln",
                concat!(
                    "use app::math\n\n",
                    "fn make() -> math::Item\n",
                    "  math::Item::Ready(math::double(1))\n",
                    "end\n\n",
                    "fn read(input: math::Item) -> Int\n",
                    "  match input\n",
                    "    math::Item::Ready(value) => value\n",
                    "  end\n",
                    "end\n",
                ),
            ),
        ];

        let function = query(sources.clone(), "main.veln", 4, 27).unwrap();
        assert_eq!(function.selected_symbol.kind, SymbolKind::Function);
        assert_location(&function.definition, "app/math.veln", 5, 8);
        assert_eq!(locations(&function.references), [("main.veln", 4, 27)]);
        assert!(validate_rename(&function, "twice").is_ok());

        let ty = query(sources.clone(), "main.veln", 3, 20).unwrap();
        assert_eq!(ty.selected_symbol.kind, SymbolKind::Type);
        assert_location(&ty.definition, "app/math.veln", 1, 10);
        assert_eq!(
            locations(&ty.references),
            [
                ("main.veln", 3, 20),
                ("main.veln", 4, 9),
                ("main.veln", 7, 22),
                ("main.veln", 9, 11),
            ]
        );
        assert!(validate_rename(&ty, "Entry").is_ok());

        let constructor = query(sources, "main.veln", 9, 17).unwrap();
        assert_eq!(constructor.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&constructor.definition, "app/math.veln", 2, 7);
        assert_eq!(
            locations(&constructor.references),
            [("main.veln", 4, 15), ("main.veln", 9, 17)]
        );
        assert!(validate_rename(&constructor, "Done").is_ok());
    }

    #[test]
    fn declaration_type_path_carriers_share_navigation() {
        let sources = vec![
            source("helper.veln", "pub type Item\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use helper\n\n",
                    "type Box\n",
                    "  Wrap(helper::Item)\n",
                    "  Record { item: helper::Item }\n",
                    "end\n\n",
                    "effect Store\n",
                    "  fetch(input: helper::Item) -> helper::Item\n",
                    "end\n\n",
                    "fn read(input: helper::Item) -> helper::Item\n",
                    "  input\n",
                    "end\n",
                ),
            ),
        ];

        let result = query(sources, "main.veln", 4, 17).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_location(&result.definition, "helper.veln", 1, 10);
        assert_classified_segment(
            &result,
            "Item",
            NameClass::Type,
            QualifiedPathSegmentEvidence::Syntax,
            1,
            4,
            16,
        );
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 4, 16),
                ("main.veln", 5, 26),
                ("main.veln", 9, 24),
                ("main.veln", 9, 41),
                ("main.veln", 12, 24),
                ("main.veln", 12, 41),
            ]
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
    fn invalid_qualified_use_segments_do_not_get_independent_navigation_roles() {
        let sources = vec![
            source(
                "helper.veln",
                concat!(
                    "pub type Item\n",
                    "  pub Ready(Int)\n",
                    "end\n\n",
                    "pub fn make() -> Int\n",
                    "  1\n",
                    "end\n",
                ),
            ),
            source(
                "main.veln",
                concat!(
                    "use helper\n\n",
                    "use foo::bar\n\n",
                    "fn main(flag: helper::Item) -> Int\n",
                    "  let a: Helper::Item = helper::Item::ready(1)\n",
                    "  let b: helper::item = helper::Item::Ready(1)\n",
                    "  let c = Helper::make()\n",
                    "  helper::Make()\n",
                    "  let nested = foo::bar::double(3)\n",
                    "  let nested_first = Foo::bar::double(3)\n",
                    "  let nested_middle = foo::Bar::double(3)\n",
                    "  let nested_leaf = foo::bar::Double(3)\n",
                    "end\n",
                ),
            ),
            source(
                "foo/bar.veln",
                "pub fn double(value: Int) -> Int\n  value + value\nend\n",
            ),
        ];

        for (line, column) in [
            (6, 10),
            (6, 39),
            (7, 18),
            (8, 11),
            (9, 11),
            (11, 22),
            (12, 28),
            (13, 31),
        ] {
            assert!(
                query(sources.clone(), "main.veln", line, column).is_none(),
                "invalid qualified segment unexpectedly navigated at {line}:{column}"
            );
        }

        let valid_type = query(sources.clone(), "main.veln", 7, 33).unwrap();
        assert_eq!(valid_type.selected_symbol.kind, SymbolKind::Type);
        assert_location(&valid_type.definition, "helper.veln", 1, 10);
        assert_classified_segment(
            &valid_type,
            "Item",
            NameClass::Type,
            QualifiedPathSegmentEvidence::Resolved,
            1,
            7,
            33,
        );

        let valid_annotation_type = query(sources.clone(), "main.veln", 5, 24).unwrap();
        assert_eq!(valid_annotation_type.selected_symbol.kind, SymbolKind::Type);
        assert_location(&valid_annotation_type.definition, "helper.veln", 1, 10);
        assert_classified_segment(
            &valid_annotation_type,
            "Item",
            NameClass::Type,
            QualifiedPathSegmentEvidence::Syntax,
            1,
            5,
            23,
        );

        let valid_function = query(sources.clone(), "main.veln", 10, 28).unwrap();
        assert_eq!(valid_function.selected_symbol.kind, SymbolKind::Function);
        assert_location(&valid_function.definition, "foo/bar.veln", 1, 8);
        assert_classified_segment(
            &valid_function,
            "double",
            NameClass::Function,
            QualifiedPathSegmentEvidence::Resolved,
            2,
            10,
            26,
        );

        let valid_constructor = query(sources, "main.veln", 7, 39).unwrap();
        assert_eq!(valid_constructor.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&valid_constructor.definition, "helper.veln", 2, 7);
        assert_classified_segment(
            &valid_constructor,
            "Ready",
            NameClass::Constructor,
            QualifiedPathSegmentEvidence::Resolved,
            2,
            7,
            39,
        );
    }

    #[test]
    fn rename_validation_preserves_constructor_case_class() {
        let result = query(
            vec![source(
                "main.veln",
                "type Item\n  Value(value: Int)\nend\n\nfn main() -> Item\n  Value(1)\nend\n",
            )],
            "main.veln",
            6,
            4,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert!(validate_rename(&result, "Entry").is_ok());
        assert_rename_invalid_case(
            validate_rename(&result, "entry").unwrap_err(),
            RenameNameClass::Constructor,
            "entry",
            RenameRequiredInitial::AsciiUppercase,
        );
    }

    #[test]
    fn constructor_references_keep_cross_file_reference_at_declaration_offset() {
        let result = query(
            vec![
                source("f.veln", "pub type Flag\n  pub Done\nend\n"),
                source("main.veln", "use f\n\nfn a()-> X\n  Done\nend\n"),
            ],
            "f.veln",
            2,
            7,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(
            result.definition.span.start.offset,
            result.references[0].start.offset
        );
        assert_eq!(locations(&result.references), [("main.veln", 4, 3)]);
    }

    #[test]
    fn constructor_references_cover_bare_nullary_expression_and_pattern() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Status\n",
                    "  Ready\n",
                    "  Waiting\n",
                    "end\n\n",
                    "fn ready() -> Status\n",
                    "  Ready\n",
                    "end\n\n",
                    "fn observe(status: Status) -> Bool\n",
                    "  match status\n",
                    "    Ready => true\n",
                    "    Waiting => false\n",
                    "end\n",
                ),
            )],
            "main.veln",
            2,
            4,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 7, 3), ("main.veln", 12, 5)]
        );
    }

    #[test]
    fn rename_validation_preserves_value_binding_case_class() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "effect Choose\n",
                    "  pick(value: Bool) -> Int\n",
                    "end\n\n",
                    "handler choose() handles Choose\n",
                    "  pick(value) => value\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            8,
        )
        .unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerOperationClauseParameter
        );
        assert!(validate_rename(&result, "input").is_ok());
        assert_rename_invalid_case(
            validate_rename(&result, "Input").unwrap_err(),
            RenameNameClass::ValueBinding,
            "Input",
            RenameRequiredInitial::AsciiLowercase,
        );
    }

    #[test]
    fn exact_companion_boundary_excludes_other_test_files() {
        let result = query(
            vec![
                source(
                    "other.test.veln",
                    "use math\n\ntest unrelated() -> Int\n  math::increment(2)\nend\n",
                ),
                source(
                    "math.veln",
                    "fn increment(value: Int) -> Int\n  value + 1\nend\n",
                ),
                source(
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
                ),
            ],
            "math.test.veln",
            4,
            11,
        )
        .unwrap();

        assert_eq!(locations(&result.references), [("math.test.veln", 4, 9)]);
    }

    #[test]
    fn handler_clause_binding_excludes_shadowing_patterns_and_fields() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "effect Choose\n",
                    "  pick(value: Bool) -> Int\n",
                    "end\n\n",
                    "handler choose() handles Choose\n",
                    "  pick(value) => match value\n",
                    "    true => value\n",
                    "    value => value\n",
                    "    false => record.value\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            "main.veln",
            7,
            16,
        )
        .unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerOperationClauseParameter
        );
        assert_location(&result.definition, "main.veln", 6, 8);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 6, 24), ("main.veln", 7, 13)]
        );
    }

    #[test]
    fn handler_context_binding_stays_in_clause_bodies() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "effect Adjust\n",
                    "  amount(value: Int) -> Int\n",
                    "end\n\n",
                    "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                    "  amount(value) => callback(value)\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            22,
        )
        .unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerContextParameter
        );
        assert_location(&result.definition, "main.veln", 5, 16);
        assert_eq!(locations(&result.references), [("main.veln", 6, 20)]);
    }

    #[test]
    fn invalid_handler_context_binding_is_not_navigable() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "end\n\n",
                "handler adjust(Callback: fn(Int) -> Int) handles Adjust\n",
                "  amount(value) => Callback(value)\n",
                "end\n",
            ),
        )];

        for (line, column) in [(5, 16), (6, 20)] {
            assert!(
                query(sources.clone(), "main.veln", line, column).is_none(),
                "invalid handler context binding was navigable at {line}:{column}"
            );
        }
    }

    #[test]
    fn invalid_handler_operation_clause_binding_is_not_navigable() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "end\n\n",
                "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                "  amount(Value) => callback(Value)\n",
                "end\n",
            ),
        )];

        for (line, column) in [(6, 10), (6, 29)] {
            assert!(
                query(sources.clone(), "main.veln", line, column).is_none(),
                "invalid handler operation binding was navigable at {line}:{column}"
            );
        }
    }

    #[test]
    fn unsupported_positions_have_no_selected_symbol() {
        let sources = vec![source(
            "main.veln",
            "fn increment(value: Int) -> Int\n  value.field\n  \"increment()\"\n  # increment()\nend\n",
        )];

        for (line, column) in [(2, 9), (3, 5), (4, 5), (1, 1)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn exported_direct_dependency_definition_has_virtual_location() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\r\n  value + 1\r\nend\r\n",
            )],
            ["./math.veln"],
        );
        let result = dependency_query(dependency, "math::increment(1)").unwrap();

        assert_eq!(result.definition.span.file.as_str(), "math.veln");
        assert_eq!(result.definition.span.start.line, 1);
        assert_eq!(result.definition.span.start.column, 8);
        let NavigationSource::Package { uri } = &result.definition.source else {
            panic!("dependency definition did not use a package location");
        };
        assert!(uri.starts_with("veln-pkg:///example%2Fpkg/snapshot/"));
        assert!(uri.ends_with("/math.veln"));
        assert!(!uri.contains("veln-language-service-navigation"));
        assert!(result.references.is_empty());
    }

    #[test]
    fn standard_library_functions_resolve_through_implicit_and_explicit_imports() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    concat!(
                        "pub fn visible(value: Int) -> Int\n  value\nend\n\n",
                        "fn hidden(value: Int) -> Int\n  value\nend\n",
                    ),
                ),
                (
                    "api.veln",
                    "pub fn exported(value: Int) -> Int\n  value\nend\n",
                ),
                (
                    "private.veln",
                    "pub fn unavailable(value: Int) -> Int\n  value\nend\n",
                ),
            ],
            ["prelude.veln", "api.veln"],
        );
        let sources = vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  visible(1)\n",
                "  prelude::visible(1)\n",
                "  api::exported(1)\n",
                "end\n",
            ),
        )];
        let snapshot =
            EffectiveProjectSnapshot::new(sources).with_standard_library(standard_library);

        for (line, column, path, declaration_column) in [
            (4, 4, "prelude.veln", 8),
            (5, 12, "prelude.veln", 8),
            (6, 9, "api.veln", 8),
        ] {
            let result = navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            )
            .unwrap();
            assert_eq!(result.definition.span.file.as_str(), path);
            assert_eq!(result.definition.span.start.column, declaration_column);
            let NavigationSource::Package { uri } = result.definition.source else {
                panic!("standard definition did not use a package location");
            };
            assert!(uri.starts_with("veln-pkg:///std/snapshot/"), "{uri}");
            assert!(uri.ends_with(path), "{uri}");
        }
    }
