    #[test]
    fn recovery_rename_validation_rejects_same_namespace_conflicts() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "type item\n",
                "  value(input: Int)\n",
                "  Ready\n",
                "end\n\n",
                "type Entry\n",
                "  Existing\n",
                "end\n\n",
                "fn Bad() -> Int\n",
                "  Bad()\n",
                "end\n\n",
                "fn good() -> Int\n",
                "  1\n",
                "end\n\n",
                "fn read(Input: Int, other: Int) -> Int\n",
                "  Input\n",
                "end\n",
            ),
        )]);

        let cases = [
            (1, 6, "Entry", RenameNameClass::Type, 6, 6),
            (2, 3, "Ready", RenameNameClass::Constructor, 3, 3),
            (10, 4, "good", RenameNameClass::Function, 14, 4),
            (18, 9, "other", RenameNameClass::ValueBinding, 18, 21),
        ];

        for (line, column, requested, class, conflict_line, conflict_column) in cases {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert!(result.is_recovery);
            assert_rename_conflict(
                validate_rename_in_snapshot(&snapshot, &result, requested).unwrap_err(),
                class,
                requested,
                "main.veln",
                conflict_line,
                conflict_column,
            );
        }
    }

    fn assert_recovery_public_visibility_conflict(
        sources: Vec<SourceFile>,
        selection: (&str, usize, usize),
        requested: &str,
        class: RenameNameClass,
        conflict: (&str, usize, usize),
    ) {
        let snapshot = EffectiveProjectSnapshot::new(sources);
        let result = query_snapshot(&snapshot, selection.0, selection.1, selection.2).unwrap();
        assert!(result.is_recovery);
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, requested).unwrap_err(),
            class,
            requested,
            conflict.0,
            conflict.1,
            conflict.2,
        );
    }

    #[test]
    fn recovery_function_rename_validation_rejects_public_visibility_conflict() {
        assert_recovery_public_visibility_conflict(
            vec![
                source("left.veln", "pub fn Bad() -> Int\n  1\nend\n"),
                source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
                source(
                    "main.veln",
                    "use left\nuse right\n\nfn main() -> Int\n  target()\nend\n",
                ),
            ],
            ("left.veln", 1, 8),
            "target",
            RenameNameClass::Function,
            ("right.veln", 1, 8),
        );
    }

    #[test]
    fn recovery_type_rename_validation_rejects_public_visibility_conflict() {
        assert_recovery_public_visibility_conflict(
            vec![
                source("left.veln", "pub type bad\n  Left\nend\n"),
                source("right.veln", "pub type Target\n  Right\nend\n"),
                source(
                    "main.veln",
                    "use left\nuse right\n\nfn read(value: Target) -> Target\n  value\nend\n",
                ),
            ],
            ("left.veln", 1, 10),
            "Target",
            RenameNameClass::Type,
            ("right.veln", 1, 10),
        );
    }

    #[test]
    fn recovery_alias_rename_validation_rejects_public_visibility_conflict() {
        assert_recovery_public_visibility_conflict(
            vec![
                source("left.veln", "pub type bad = Int\n"),
                source("right.veln", "pub type Target = Int\n"),
                source(
                    "main.veln",
                    "use left\nuse right\n\nfn read(value: Target) -> Target\n  value\nend\n",
                ),
            ],
            ("left.veln", 1, 10),
            "Target",
            RenameNameClass::Type,
            ("right.veln", 1, 10),
        );
    }

    #[test]
    fn recovery_constructor_rename_validation_rejects_public_visibility_conflict() {
        assert_recovery_public_visibility_conflict(
            vec![
                source("left.veln", "pub type Item\n  pub bad\nend\n"),
                source("right.veln", "pub type Other\n  pub Target\nend\n"),
                source(
                    "main.veln",
                    "use left\nuse right\n\nfn main() -> Other\n  Target\nend\n",
                ),
            ],
            ("left.veln", 2, 7),
            "Target",
            RenameNameClass::Constructor,
            ("right.veln", 2, 7),
        );
    }

    #[test]
    fn recovery_rename_validation_rejects_unused_lexical_declaration_conflicts() {
        let cases = [
            (
                "local",
                concat!(
                    "fn read(input: Int) -> Int\n",
                    "  let Bad = input\n",
                    "  let other = input\n",
                    "  other\n",
                    "end\n",
                ),
                2,
                7,
                3,
                7,
            ),
            (
                "parameter",
                "fn read(Bad: Int, other: Int) -> Int\n  other\nend\n",
                1,
                9,
                1,
                19,
            ),
            (
                "pattern",
                concat!(
                    "fn read(input: Int, other: Int) -> Int\n",
                    "  match input\n",
                    "    Bad => other\n",
                    "  end\n",
                    "end\n",
                ),
                3,
                5,
                1,
                21,
            ),
            (
                "satisfy",
                concat!(
                    "fn read(input: Int, other: Int) -> Int\n",
                    "  _value satisfy Bad => other\n",
                    "  other\n",
                    "end\n",
                ),
                2,
                18,
                1,
                21,
            ),
        ];

        for (name, text, line, column, conflict_line, conflict_column) in cases {
            let snapshot = EffectiveProjectSnapshot::new(vec![source("main.veln", text)]);
            let result = query_snapshot(&snapshot, "main.veln", line, column)
                .unwrap_or_else(|| panic!("{name} recovery should be selected"));
            assert!(result.is_recovery, "{name} selected a valid symbol");
            assert!(result.references.is_empty(), "{name} unexpectedly had references");
            assert_rename_conflict(
                validate_rename_in_snapshot(&snapshot, &result, "other").unwrap_err(),
                RenameNameClass::ValueBinding,
                "other",
                "main.veln",
                conflict_line,
                conflict_column,
            );
        }
    }
