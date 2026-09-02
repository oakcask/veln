
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
    fn invalid_function_recovery_navigation_links_declaration_and_in_scope_uses() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "fn Bad() -> Int\n",
                    "  Bad()\n",
                    "end\n\n",
                    "fn caller() -> Int\n",
                    "  Bad()\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            4,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_location(&result.definition, "main.veln", 1, 4);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 2, 3), ("main.veln", 6, 3)]
        );

        let declaration = query(
            vec![source(
                "main.veln",
                concat!(
                    "fn Bad() -> Int\n",
                    "  Bad()\n",
                    "end\n\n",
                    "fn caller() -> Int\n",
                    "  Bad()\n",
                    "end\n",
                ),
            )],
            "main.veln",
            1,
            4,
        )
        .unwrap();
        assert!(declaration.is_recovery);
        assert_eq!(declaration.selection, declaration.definition.span);
        assert_eq!(declaration.references, result.references);
    }

    #[test]
    fn invalid_declaration_recovery_navigation_covers_source_declaration_forms() {
        struct Case {
            name: &'static str,
            text: &'static str,
            kind: SymbolKind,
            declaration: (usize, usize),
            use_position: (usize, usize),
            references: &'static [(usize, usize)],
        }

        let cases = [
            Case {
                name: "constructor",
                text: concat!(
                    "type Item\n",
                    "  value(input: Int)\n",
                    "end\n\n",
                    "fn read() -> Item\n",
                    "  value(1)\n",
                    "end\n",
                ),
                kind: SymbolKind::Constructor,
                declaration: (2, 3),
                use_position: (6, 4),
                references: &[(6, 3)],
            },
            Case {
                name: "test",
                text: concat!(
                    "test Bad() -> Int\n",
                    "  Bad()\n",
                    "end\n\n",
                    "fn read() -> Int\n",
                    "  Bad()\n",
                    "end\n",
                ),
                kind: SymbolKind::Function,
                declaration: (1, 6),
                use_position: (6, 4),
                references: &[(2, 3), (6, 3)],
            },
            Case {
                name: "public type alias",
                text: concat!(
                    "type Item\n",
                    "  Value\n",
                    "end\n\n",
                    "pub type item_alias = Item\n\n",
                    "fn read(value: item_alias) -> item_alias\n",
                    "  value\n",
                    "end\n",
                ),
                kind: SymbolKind::Type,
                declaration: (5, 10),
                use_position: (7, 20),
                references: &[(7, 16), (7, 31)],
            },
            Case {
                name: "public function alias",
                text: concat!(
                    "fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn BadAlias = target\n\n",
                    "fn read() -> Int\n",
                    "  BadAlias()\n",
                    "end\n",
                ),
                kind: SymbolKind::Function,
                declaration: (5, 8),
                use_position: (8, 4),
                references: &[(8, 3)],
            },
        ];

        for case in cases {
            let sources = vec![source("main.veln", case.text)];
            let from_use = query(
                sources.clone(),
                "main.veln",
                case.use_position.0,
                case.use_position.1,
            )
            .unwrap_or_else(|| panic!("{} use should select recovery", case.name));
            assert!(from_use.is_recovery, "{} use selected a valid symbol", case.name);
            assert_eq!(from_use.selected_symbol.kind, case.kind, "{}", case.name);
            assert_location(
                &from_use.definition,
                "main.veln",
                case.declaration.0,
                case.declaration.1,
            );
            assert_eq!(
                locations(&from_use.references),
                case
                    .references
                    .iter()
                    .map(|(line, column)| ("main.veln", *line, *column))
                    .collect::<Vec<_>>(),
                "{}",
                case.name
            );

            let from_declaration = query(
                sources,
                "main.veln",
                case.declaration.0,
                case.declaration.1,
            )
            .unwrap_or_else(|| panic!("{} declaration should select recovery", case.name));
            assert!(from_declaration.is_recovery);
            assert_eq!(from_declaration.selected_symbol.kind, case.kind, "{}", case.name);
            assert_eq!(from_declaration.selection, from_declaration.definition.span);
            assert_eq!(
                from_declaration.references, from_use.references,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn invalid_function_recovery_reference_lookup_prepares_scopes_once() {
        let linked_calls = 800;
        let mut source_text = String::from("fn Bad() -> Int\n");
        for _ in 0..linked_calls {
            source_text.push_str("  Bad()\n");
        }
        source_text.push_str("end\n");
        let snapshot =
            EffectiveProjectSnapshot::new(vec![source("main.veln", &source_text)]);
        reset_function_scope_collections();

        let result = query_snapshot(&snapshot, "main.veln", 1, 4).unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.references.len(), linked_calls);
        assert_eq!(function_scope_collections(), 1);
    }

    #[test]
    fn invalid_recovery_navigation_keeps_valid_symbol_precedence() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  byte(value: Int)\n",
                    "end\n\n",
                    "fn byte() -> Int\n",
                    "  2\n",
                    "end\n\n",
                    "fn caller(Bad: fn() -> Int) -> Int\n",
                    "  byte()\n",
                    "end\n",
                ),
            )],
            "main.veln",
            10,
            4,
        )
        .unwrap();

        assert!(!result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_location(&result.definition, "main.veln", 5, 4);
    }

    #[test]
    fn valid_bare_nullary_constructor_expression_precedes_invalid_binding_recovery() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Value\n",
                    "end\n\n",
                    "fn main(Value: Int) -> Item\n",
                    "  Value\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            4,
        )
        .unwrap();

        assert!(!result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 3);
    }

    #[test]
    fn valid_bare_nullary_constructor_pattern_precedes_invalid_function_recovery() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Value\n",
                    "end\n\n",
                    "fn Value() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "fn read(item: Item) -> Int\n",
                    "  match item\n",
                    "    Value => 1\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            "main.veln",
            11,
            6,
        )
        .unwrap();

        assert!(!result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 3);
    }

    #[test]
    fn valid_bare_nullary_constructor_pattern_precedes_invalid_binding_recovery() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Value\n",
                    "end\n\n",
                    "fn read(Value: Int, item: Item) -> Int\n",
                    "  match item\n",
                    "    Value => Value\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            "main.veln",
            7,
            6,
        )
        .unwrap();

        assert!(!result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 3);
    }

    #[test]
    fn multiple_invalid_recovery_records_are_not_selected() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "fn Bad() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "fn Bad() -> Int\n",
                    "  2\n",
                    "end\n\n",
                    "fn caller() -> Int\n",
                    "  Bad()\n",
                    "end\n",
                ),
            )],
            "main.veln",
            10,
            4,
        );

        assert!(result.is_none());
    }

    #[test]
    fn ambiguous_invalid_declarations_are_not_selected_at_declaration_positions() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "fn Bad() -> Int\n",
                "  1\n",
                "end\n\n",
                "fn Bad() -> Int\n",
                "  2\n",
                "end\n",
            ),
        )];

        assert!(query(sources.clone(), "main.veln", 1, 4).is_none());
        assert!(query(sources, "main.veln", 5, 4).is_none());
    }

    #[test]
    fn valid_callable_parameter_blocks_constructor_recovery_at_call_site() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  byte(value: Int)\n",
                    "end\n\n",
                    "fn caller(byte: fn() -> Int) -> Int\n",
                    "  byte()\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            4,
        );

        assert!(result.is_none());
    }

    #[test]
    fn invalid_local_binding_recovery_stays_in_lexical_scope() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "fn main(input: Int) -> Int\n",
                    "  let Bad = input\n",
                    "  Bad\n",
                    "end\n\n",
                    "fn other() -> Int\n",
                    "  Bad\n",
                    "end\n",
                ),
            )],
            "main.veln",
            3,
            4,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 2, 7);
        assert_eq!(locations(&result.references), [("main.veln", 3, 3)]);
        assert!(query(
            vec![source(
                "main.veln",
                concat!(
                    "fn main(input: Int) -> Int\n",
                    "  let Bad = input\n",
                    "  Bad\n",
                    "end\n\n",
                    "fn other() -> Int\n",
                    "  Bad\n",
                    "end\n",
                ),
            )],
            "main.veln",
            7,
            4,
        )
        .is_none());
    }

    #[test]
    fn invalid_local_binding_recovery_starts_after_declaration_initializer() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "fn main(input: Int) -> Int\n",
                "  let Bad = Bad\n",
                "  Bad\n",
                "end\n",
            ),
        )];

        assert!(query(sources.clone(), "main.veln", 2, 14).is_none());

        let result = query(sources, "main.veln", 3, 4).unwrap();
        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 2, 7);
        assert_eq!(locations(&result.references), [("main.veln", 3, 3)]);
    }

    #[test]
    fn invalid_parameter_recovery_navigation_links_in_scope_uses() {
        let result = query(
            vec![source(
                "main.veln",
                "fn main(Bad: Int) -> Int\n  Bad\nend\n",
            )],
            "main.veln",
            2,
            4,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 1, 9);
        assert_eq!(locations(&result.references), [("main.veln", 2, 3)]);
    }

    #[test]
    fn invalid_callable_parameter_recovery_links_value_and_call_uses() {
        let sources = vec![source(
            "main.veln",
            "fn main(Bad: fn() -> Int) -> Int\n  Bad\n  Bad()\nend\n",
        )];

        let value = query(sources.clone(), "main.veln", 2, 4)
            .expect("the invalid callable parameter should recover at its value use");
        assert!(value.is_recovery);
        assert_eq!(value.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&value.definition, "main.veln", 1, 9);
        assert_eq!(
            locations(&value.references),
            [("main.veln", 2, 3), ("main.veln", 3, 3)]
        );

        let call = query(sources, "main.veln", 3, 4)
            .expect("the invalid callable parameter should recover at its call use");
        assert!(call.is_recovery);
        assert_eq!(call.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&call.definition, "main.veln", 1, 9);
        assert_eq!(call.references, value.references);
        assert_eq!((call.selection.start.line, call.selection.start.column), (3, 3));
    }

    #[test]
    fn invalid_callable_local_binding_recovery_links_value_and_call_uses() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "fn target() -> Int\n",
                "  1\n",
                "end\n\n",
                "fn main() -> Int\n",
                "  let Worker = target\n",
                "  Worker\n",
                "  Worker()\n",
                "end\n",
            ),
        )];

        let value = query(sources.clone(), "main.veln", 7, 4)
            .expect("the invalid callable local binding should recover at its value use");
        assert!(value.is_recovery);
        assert_eq!(value.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&value.definition, "main.veln", 6, 7);
        assert_eq!(
            locations(&value.references),
            [("main.veln", 7, 3), ("main.veln", 8, 3)]
        );

        let call = query(sources, "main.veln", 8, 4)
            .expect("the invalid callable local binding should recover at its call use");
        assert!(call.is_recovery);
        assert_eq!(call.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&call.definition, "main.veln", 6, 7);
        assert_eq!(call.references, value.references);
        assert_eq!((call.selection.start.line, call.selection.start.column), (8, 3));
    }

    #[test]
    fn invalid_result_binding_recovery_navigation_links_ensure_uses() {
        let result = query(
            vec![source(
                "main.veln",
                "fn main(value: Int) -> Output: Int\n  ensure Output >= value\n  value\nend\n",
            )],
            "main.veln",
            2,
            10,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 1, 24);
        assert_eq!(locations(&result.references), [("main.veln", 2, 10)]);
    }

    #[test]
    fn invalid_handler_binding_recovery_navigation_links_clause_body_uses() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "  echo(value: Int) -> Int\n",
                "end\n\n",
                "handler adjust(Callback: fn(Int) -> Int) handles Adjust\n",
                "  amount(value) => Callback(value)\n",
                "  echo(Result) => Callback(Result)\n",
                "end\n",
            ),
        )];

        let context = query(sources.clone(), "main.veln", 7, 22).unwrap();
        assert!(context.is_recovery);
        assert_eq!(context.selected_symbol.kind, SymbolKind::HandlerContextParameter);
        assert_location(&context.definition, "main.veln", 6, 16);
        assert_eq!(
            locations(&context.references),
            [("main.veln", 7, 20), ("main.veln", 8, 19)]
        );

        let clause = query(sources, "main.veln", 8, 28).unwrap();
        assert!(clause.is_recovery);
        assert_eq!(
            clause.selected_symbol.kind,
            SymbolKind::HandlerOperationClauseParameter
        );
        assert_location(&clause.definition, "main.veln", 8, 8);
        assert_eq!(locations(&clause.references), [("main.veln", 8, 28)]);
    }

    #[test]
    fn invalid_pattern_binding_recovery_navigation_links_arm_body_uses() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Item\n",
                    "  Some(Int)\n",
                    "end\n\n",
                    "fn read(item: Item) -> Int\n",
                    "  match item\n",
                    "    Some(Bad) => Bad\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            "main.veln",
            7,
            19,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 7, 10);
        assert_eq!(locations(&result.references), [("main.veln", 7, 18)]);
    }

    #[test]
    fn invalid_satisfy_candidate_recovery_navigation_links_predicate_uses() {
        let result = query(
            vec![source(
                "main.veln",
                "fn main(limit: Int) -> Int\n  _value satisfy Candidate => Candidate <= limit\n  limit\nend\n",
            )],
            "main.veln",
            2,
            32,
        )
        .unwrap();

        assert!(result.is_recovery);
        assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&result.definition, "main.veln", 2, 18);
        assert_eq!(locations(&result.references), [("main.veln", 2, 31)]);
    }

    #[test]
    fn equal_spelled_navigation_uses_the_selected_namespace() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "schema Common\n",
                "  value: Int\n",
                "end\n\n",
                "effect Common\n",
                "  Common() -> Int\n",
                "end\n\n",
                "handler Common() handles Common\n",
                "  Common() => 1\n",
                "end\n\n",
                "type Common\n",
                "  Common(Int)\n",
                "end\n\n",
                "fn common(input: Common) -> Common effects [Common]\n",
                "  let common = perform Common::Common()\n",
                "  Common(common)\n",
                "end\n",
            ),
        )]);

        let type_result = query_snapshot(&snapshot, "main.veln", 17, 18).unwrap();
        assert_eq!(type_result.selected_symbol.kind, SymbolKind::Type);
        assert_location(&type_result.definition, "main.veln", 13, 6);

        let constructor_result = query_snapshot(&snapshot, "main.veln", 19, 4).unwrap();
        assert_eq!(constructor_result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&constructor_result.definition, "main.veln", 14, 3);

        let function_result = query_snapshot(&snapshot, "main.veln", 17, 4).unwrap();
        assert_eq!(function_result.selected_symbol.kind, SymbolKind::Function);
        assert_location(&function_result.definition, "main.veln", 17, 4);

        let binding_result = query_snapshot(&snapshot, "main.veln", 19, 10).unwrap();
        assert_eq!(binding_result.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_location(&binding_result.definition, "main.veln", 18, 7);
    }
