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
    fn rename_validation_rejects_unedited_imported_type_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Item\n  Left\nend\n"),
            source("right.veln", "pub type Entry\n  Right\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn imported(value: Entry) -> Entry\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 10).unwrap();

        let failure = validate_rename_in_snapshot(&snapshot, &result, "Entry").unwrap_err();

        assert_rename_conflict(
            failure.clone(),
            RenameNameClass::Type,
            "Entry",
            "right.veln",
            1,
            10,
        );
        let RenameFailureKind::Conflict { affected_scope, .. } = failure.kind else {
            panic!("rename failure was not a conflict");
        };
        assert_eq!(
            *affected_scope,
            RenameAffectedScope::Module {
                name: "main".to_string(),
            }
        );
    }

    #[test]
    fn rename_validation_allows_unedited_local_type_shadowing() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Item\n  Left\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n\n",
                    "type Entry\n",
                    "  Local\n",
                    "end\n\n",
                    "fn imported(value: Entry) -> Entry\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 10).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "Entry").is_ok());
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
    fn rename_validation_rejects_unedited_imported_constructor_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source("right.veln", "pub type Target\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn make() -> right::Target\n",
                    "  Done\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        let failure = validate_rename_in_snapshot(&snapshot, &result, "Done").unwrap_err();

        assert_rename_conflict(
            failure.clone(),
            RenameNameClass::Constructor,
            "Done",
            "right.veln",
            2,
            7,
        );
        let RenameFailureKind::Conflict { affected_scope, .. } = failure.kind else {
            panic!("rename failure was not a conflict");
        };
        assert_eq!(
            *affected_scope,
            RenameAffectedScope::Module {
                name: "main".to_string(),
            }
        );
    }

    #[test]
    fn rename_validation_rejects_unedited_imported_function_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn caller() -> Int\n",
                    "  target()\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();

        let failure = validate_rename_in_snapshot(&snapshot, &result, "target").unwrap_err();

        assert_rename_conflict(
            failure.clone(),
            RenameNameClass::Function,
            "target",
            "right.veln",
            1,
            8,
        );
        let RenameFailureKind::Conflict { affected_scope, .. } = failure.kind else {
            panic!("rename failure was not a conflict");
        };
        assert_eq!(
            *affected_scope,
            RenameAffectedScope::Module {
                name: "main".to_string(),
            }
        );
    }

    #[test]
    fn rename_validation_preserves_unedited_local_function_resolution() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn target() -> Int\n",
                    "  3\n",
                    "end\n\n",
                    "fn caller() -> Int\n",
                    "  target()\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());
    }

    #[test]
    fn rename_validation_preserves_unedited_lexical_callable_shadowing() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn caller(target: Int) -> Int\n",
                    "  target()\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());
    }

    #[test]
    fn rename_validation_preserves_qualified_function_identity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn caller() -> Int\n",
                    "  left::source()\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());
    }
