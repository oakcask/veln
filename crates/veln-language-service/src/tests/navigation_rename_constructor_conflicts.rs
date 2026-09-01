#[cfg(test)]
mod navigation_rename_constructor_conflicts {
    use crate::{
        EffectiveProjectSnapshot, RenameAffectedScope, RenameFailureKind, RenameNameClass,
        validate_rename_in_snapshot,
    };

    use super::{assert_rename_conflict, locations, query_snapshot, source};

    #[test]
    fn rename_validation_rejects_constructor_ambiguity_through_public_alias_reexport() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source(
                "bridge.veln",
                concat!("use left\n\n", "pub type Alias = left::Source\n"),
            ),
            source("right.veln", "pub type Other\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use bridge\n",
                    "use right\n\n",
                    "fn current() -> Other\n",
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
    fn rename_validation_does_not_treat_unrelated_alias_as_constructor_visibility() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source("unrelated.veln", "pub type Unrelated\n  Other\nend\n"),
            source(
                "bridge.veln",
                concat!("use unrelated\n\n", "pub type Alias = unrelated::Unrelated\n"),
            ),
            source("right.veln", "pub type Other\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use bridge\n",
                    "use right\n\n",
                    "fn current() -> Other\n",
                    "  Done\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "Done").is_ok());
    }

    #[test]
    fn rename_validation_does_not_treat_unimported_alias_as_constructor_visibility() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source(
                "bridge.veln",
                concat!("use left\n\n", "pub type Alias = left::Source\n"),
            ),
            source("right.veln", "pub type Other\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use right\n\n",
                    "fn current() -> Other\n",
                    "  Done\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        assert!(validate_rename_in_snapshot(&snapshot, &result, "Done").is_ok());
    }

    #[test]
    fn rename_validation_ignores_operation_roles_for_constructor_visibility() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source("right.veln", "pub type Other\n  pub Done\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n\n",
                    "effect Task\n",
                    "  Done() -> Int\n",
                    "end\n\n",
                    "handler task() handles Task\n",
                    "  Done() => 1\n",
                    "end\n\n",
                    "fn current() -> Int\n",
                    "  task()\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        assert_eq!(locations(&result.references), []);
        assert!(validate_rename_in_snapshot(&snapshot, &result, "Done").is_ok());
    }

    #[test]
    fn constructor_rename_does_not_collect_equal_spelled_operation_roles() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("left.veln", "pub type Source\n  pub Ready\nend\n"),
            source(
                "main.veln",
                concat!(
                    "use left\n\n",
                    "effect Task\n",
                    "  Ready() -> Int\n",
                    "end\n\n",
                    "handler task() handles Task\n",
                    "  Ready() => 1\n",
                    "end\n\n",
                    "fn current() -> Source\n",
                    "  Ready\n",
                    "end\n",
                ),
            ),
        ]);
        let result = query_snapshot(&snapshot, "left.veln", 2, 7).unwrap();

        assert_eq!(locations(&result.references), [("main.veln", 12, 3)]);
        assert!(validate_rename_in_snapshot(&snapshot, &result, "Done").is_ok());
    }
}
