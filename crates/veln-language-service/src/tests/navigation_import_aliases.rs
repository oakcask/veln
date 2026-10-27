mod explicit_import_alias_navigation_tests {
    use super::*;

    fn alias_sources() -> Vec<SourceFile> {
        vec![
            source(
                "math.veln",
                concat!(
                    "pub fn double(value: Int) -> Int\n",
                    "  value + value\n",
                    "end\n\n",
                    "pub type Item\n",
                    "  pub Ready(Int)\n",
                    "end\n",
                ),
            ),
            source("extra.veln", "pub fn value() -> Int\n  1\nend\n"),
            source(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use math as calc\n",
                    "use extra as helper\n\n",
                    "fn first() -> Int\n",
                    "  calc::double(1)\n",
                    "end\n\n",
                    "fn make() -> calc::Item\n",
                    "  calc::Item::Ready(1)\n",
                    "end\n",
                ),
            ),
            source(
                "app_more.veln",
                "mod app\n\nfn second() -> Int\n  calc::double(2)\nend\n",
            ),
            source(
                "other.veln",
                concat!(
                    "mod other\n",
                    "use math as calc\n\n",
                    "fn third() -> Int\n",
                    "  calc::double(3)\n",
                    "end\n",
                ),
            ),
        ]
    }

    #[test]
    fn explicit_alias_identity_is_module_local_and_distinct_from_its_member() {
        let sources = alias_sources();
        let declaration = query(sources.clone(), "app.veln", 2, 13).unwrap();
        let qualifier = query(sources.clone(), "app_more.veln", 4, 3).unwrap();
        for alias in [&declaration, &qualifier] {
            assert_eq!(alias.selected_symbol.kind, SymbolKind::ImportAlias);
            assert_location(&alias.definition, "app.veln", 2, 13);
            assert_eq!(
                locations(&alias.references),
                [
                    ("app.veln", 6, 3),
                    ("app.veln", 9, 14),
                    ("app.veln", 10, 3),
                    ("app_more.veln", 4, 3),
                ]
            );
        }

        let member = query(sources, "app_more.veln", 4, 9).unwrap();
        assert_eq!(member.selected_symbol.kind, SymbolKind::Function);
        assert_location(&member.definition, "math.veln", 1, 8);
        assert_eq!(
            locations(&member.references),
            [
                ("app.veln", 6, 9),
                ("app_more.veln", 4, 9),
                ("other.veln", 5, 9),
            ]
        );

        let ty = query(alias_sources(), "app.veln", 9, 20).unwrap();
        assert_eq!(ty.selected_symbol.kind, SymbolKind::Type);
        assert_location(&ty.definition, "math.veln", 5, 10);
        assert_eq!(
            locations(&ty.references),
            [("app.veln", 9, 20), ("app.veln", 10, 9)]
        );
        let constructor = query(alias_sources(), "app.veln", 10, 15).unwrap();
        assert_eq!(constructor.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&constructor.definition, "math.veln", 6, 7);
        assert_eq!(locations(&constructor.references), [("app.veln", 10, 15)]);
    }

    #[test]
    fn explicit_alias_rename_validates_reserved_case_and_import_namespace() {
        let snapshot = EffectiveProjectSnapshot::new(alias_sources());
        let result = query_snapshot(&snapshot, "app.veln", 6, 3).unwrap();
        assert!(validate_rename_in_snapshot(&snapshot, &result, "numbers").is_ok());
        assert_rename_invalid_case(
            validate_rename_in_snapshot(&snapshot, &result, "Numbers").unwrap_err(),
            RenameNameClass::ImportAlias,
            "Numbers",
            RenameRequiredInitial::AsciiLowercase,
        );
        let reserved = validate_rename_in_snapshot(&snapshot, &result, "prelude").unwrap_err();
        assert_eq!(reserved.code, "rename.reserved");
        assert_eq!(reserved.kind, RenameFailureKind::ReservedName);
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "helper").unwrap_err(),
            RenameNameClass::ImportAlias,
            "helper",
            "app.veln",
            3,
            14,
        );
        assert_rename_conflict(
            validate_rename_in_snapshot(&snapshot, &result, "extra").unwrap_err(),
            RenameNameClass::ImportAlias,
            "extra",
            "app.veln",
            3,
            14,
        );
    }

    #[test]
    fn invalid_and_recovered_explicit_aliases_have_no_navigation_identity() {
        let invalid = vec![
            source("math.veln", "pub fn double() -> Int\n  2\nend\n"),
            source(
                "invalid.veln",
                "use math as Calc\n\nfn main() -> Int\n  Calc::double()\nend\n",
            ),
        ];
        assert!(query(invalid.clone(), "invalid.veln", 1, 13).is_none());
        assert!(query(invalid, "invalid.veln", 4, 3).is_none());

        let recovered = vec![
            source("math.veln", "pub fn double() -> Int\n  2\nend\n"),
            source(
                "recovered.veln",
                "use math as calc as nested\n\nfn main() -> Int\n  calc::double()\nend\n",
            ),
        ];
        assert!(query(recovered.clone(), "recovered.veln", 1, 13).is_none());
        assert!(query(recovered, "recovered.veln", 4, 3).is_none());

        let reserved = vec![
            source("math.veln", "pub fn double() -> Int\n  2\nend\n"),
            source(
                "reserved.veln",
                "use math as prelude\n\nfn main() -> Int\n  prelude::double()\nend\n",
            ),
        ];
        assert!(query(reserved.clone(), "reserved.veln", 1, 13).is_none());
        assert!(query(reserved, "reserved.veln", 4, 3).is_none());

        let collision = vec![
            source("math.veln", "pub fn double() -> Int\n  2\nend\n"),
            source("extra.veln", "pub fn value() -> Int\n  1\nend\n"),
            source(
                "collision.veln",
                concat!(
                    "use math as shared\n",
                    "use extra as shared\n\n",
                    "fn main() -> Int\n",
                    "  shared::double()\n",
                    "end\n",
                ),
            ),
        ];
        assert!(query(collision.clone(), "collision.veln", 1, 13).is_none());
        assert!(query(collision.clone(), "collision.veln", 2, 14).is_none());
        assert!(query(collision, "collision.veln", 5, 3).is_none());

        let duplicate_target = vec![
            source("math.veln", "pub fn double() -> Int\n  2\nend\n"),
            source(
                "duplicate.veln",
                concat!(
                    "use math as first\n",
                    "use math as second\n\n",
                    "fn main() -> Int\n",
                    "  first::double()\n",
                    "end\n",
                ),
            ),
        ];
        assert!(query(duplicate_target.clone(), "duplicate.veln", 1, 13).is_none());
        assert!(query(duplicate_target.clone(), "duplicate.veln", 2, 13).is_none());
        assert!(query(duplicate_target, "duplicate.veln", 5, 3).is_none());
    }

    #[test]
    fn invalid_alias_declarations_do_not_fall_back_to_prelude_symbols() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn vec_len(items: Vec<A>) -> Int\n  prelude_builtin::vec_len(items)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source("alpha.veln", "pub fn value() -> Int\n  1\nend\n"),
            source("beta.veln", "pub fn value() -> Int\n  2\nend\n"),
            source(
                "invalid.veln",
                "use alpha as vec_len\nuse beta as vec_len\n",
            ),
        ])
        .with_standard_library(standard_library);

        assert!(query_snapshot(&snapshot, "invalid.veln", 1, 14).is_none());
        assert!(query_snapshot(&snapshot, "invalid.veln", 2, 13).is_none());
    }

    #[test]
    fn explicit_alias_navigation_is_stable_when_combined_sources_are_reordered() {
        let sources = alias_sources();
        let expected = query(sources.clone(), "app_more.veln", 4, 3).unwrap();
        let mut reordered = sources;
        reordered.reverse();
        let actual = query(reordered, "app_more.veln", 4, 3).unwrap();
        assert_eq!(expected.selection, actual.selection);
        assert_eq!(expected.definition, actual.definition);
        assert_eq!(expected.references, actual.references);
    }
}
