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
    fn rename_validation_rejects_unedited_imported_function_value_ambiguity() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("right.veln", "pub fn target() -> Int\n  2\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "fn caller() -> Int\n",
                    "  let callback = target\n",
                    "  0\n",
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
    fn rename_validation_keeps_shadowed_consumer_function_rename_linear() {
        let elapsed = [400, 800, 1_600].map(|call_count| {
            let mut samples = (0..3)
                .map(|_| shadowed_consumer_validation_time(call_count))
                .collect::<Vec<_>>();
            samples.sort();
            samples[1]
        });

        assert!(elapsed[1] <= elapsed[0] * 3 + std::time::Duration::from_millis(50));
        assert!(elapsed[2] <= elapsed[1] * 3 + std::time::Duration::from_millis(50));
    }

    #[test]
    fn rename_validation_keeps_unrelated_local_bindings_consumer_function_rename_linear() {
        let elapsed = [200, 400, 800].map(|count| {
            let mut samples = (0..3)
                .map(|_| unrelated_local_bindings_consumer_validation_time(count))
                .collect::<Vec<_>>();
            samples.sort();
            samples[1]
        });

        assert!(elapsed[1] <= elapsed[0] * 3 + std::time::Duration::from_millis(50));
        assert!(elapsed[2] <= elapsed[1] * 3 + std::time::Duration::from_millis(50));
    }

    fn shadowed_consumer_validation_time(call_count: usize) -> std::time::Duration {
        let mut consumer = String::from(
            concat!(
                "use left\n\n",
                "fn caller(target: Int) -> Int\n",
                "  let value = target\n",
            ),
        );
        for _ in 0..call_count {
            consumer.push_str("  target()\n");
        }
        consumer.push_str("  value\nend\n");
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("main.veln", &consumer),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();
        let start = std::time::Instant::now();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());

        start.elapsed()
    }

    fn unrelated_local_bindings_consumer_validation_time(count: usize) -> std::time::Duration {
        let mut consumer = String::from(
            concat!(
                "use left\n\n",
                "fn caller(seed: Int) -> Int\n",
            ),
        );
        for index in 0..count {
            consumer.push_str(&format!("  let local{index} = seed\n"));
        }
        for _ in 0..count {
            consumer.push_str("  target()\n");
        }
        consumer.push_str("  seed\nend\n");
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub fn source() -> Int\n  1\nend\n"),
            source("main.veln", &consumer),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 1, 8).unwrap();
        let start = std::time::Instant::now();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "target").is_ok());

        start.elapsed()
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
