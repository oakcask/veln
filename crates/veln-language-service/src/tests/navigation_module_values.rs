mod navigation_module_value_tests {
    use super::*;

    fn module_value_sources() -> Vec<SourceFile> {
        vec![
            source(
                "values.veln",
                "pub let answer: Int = 42\nlet direct: Int = answer\n",
            ),
            source("other.veln", "pub let answer: Int = 42\n"),
            source(
                "functions.veln",
                "pub fn answer() -> Int\n  42\nend\n",
            ),
            source(
                "main.veln",
                concat!(
                    "use values\n",
                    "use other as equal\n",
                    "use functions as callable\n\n",
                    "let qualified: Int = values::answer\n",
                    "let equal_data: Int = equal::answer\n",
                    "let same_spelling: Int = callable::answer()\n",
                ),
            ),
            source(
                "alias.veln",
                "use values as config\n\nlet aliased: Int = config::answer\n",
            ),
        ]
    }

    #[test]
    fn module_value_navigation_uses_one_identity_across_import_spellings() {
        let snapshot = EffectiveProjectSnapshot::new(module_value_sources());
        for (path, line, column) in [
            ("values.veln", 2, 19),
            ("main.veln", 5, 30),
            ("alias.veln", 3, 29),
        ] {
            let result = query_snapshot(&snapshot, path, line, column)
                .unwrap_or_else(|| panic!("missing navigation at {path}:{line}:{column}"));
            assert_eq!(result.selected_symbol.kind, SymbolKind::ModuleValue);
            assert_location(&result.definition, "values.veln", 1, 9);
            assert_eq!(
                locations(&result.references),
                [
                    ("alias.veln", 3, 28),
                    ("main.veln", 5, 30),
                    ("values.veln", 2, 19),
                ]
            );
            assert!(validate_rename(&result, "result").is_ok());
        }

        let declaration = query_snapshot(&snapshot, "values.veln", 1, 10).unwrap();
        assert_eq!(declaration.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_eq!(declaration.references.len(), 3);
    }

    #[test]
    fn equal_initializer_and_same_spelled_function_keep_distinct_identities() {
        let snapshot = EffectiveProjectSnapshot::new(module_value_sources());
        let equal_data = query_snapshot(&snapshot, "main.veln", 6, 31).unwrap();
        assert_eq!(equal_data.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_location(&equal_data.definition, "other.veln", 1, 9);
        assert_eq!(locations(&equal_data.references), [("main.veln", 6, 30)]);

        let function = query_snapshot(&snapshot, "main.veln", 7, 39).unwrap();
        assert_eq!(function.selected_symbol.kind, SymbolKind::Function);
        assert_location(&function.definition, "functions.veln", 1, 8);
    }

    #[test]
    fn package_owned_module_values_keep_distinct_identities() {
        let first = dependency_snapshot(
            "one/pkg",
            &[("shared.veln", "pub let answer: Int = 42\n")],
            ["shared.veln"],
        );
        let second = dependency_snapshot(
            "two/pkg",
            &[("shared.veln", "pub let answer: Int = 42\n")],
            ["shared.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use shared as first from \"one/pkg\"\n",
                    "use shared as second from \"two/pkg\"\n\n",
                    "let left: Int = first::answer\n",
                    "let right: Int = second::answer\n",
                ),
            )],
            vec![first, second],
        );

        let first = query_snapshot(&snapshot, "main.veln", 4, 25).unwrap();
        let second = query_snapshot(&snapshot, "main.veln", 5, 27).unwrap();
        assert_eq!(first.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_eq!(second.selected_symbol.kind, SymbolKind::ModuleValue);
        assert_ne!(first.definition.source, second.definition.source);
        assert_eq!(locations(&first.references), [("main.veln", 4, 24)]);
        assert_eq!(locations(&second.references), [("main.veln", 5, 26)]);
    }
}
