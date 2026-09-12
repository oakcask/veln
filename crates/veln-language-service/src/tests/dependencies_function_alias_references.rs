mod dependencies_function_alias_references_tests {
    use super::*;

    fn assert_direct_dependency_alias(result: &NavigationResult) {
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
    }

    fn direct_alias_snapshot() -> EffectiveProjectSnapshot {
        let selected = dependency_snapshot(
            "example/pkg",
            &[(
                "lib/math.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["lib/math.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[(
                "other_math.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value + 2\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["other_math.veln"],
        );
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::math from \"example/pkg\"\n",
                        "use other_math from \"other/pkg\"\n\n",
                        "pub fn renamed(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "pub fn first(record: {renamed: Int}, value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n\n",
                        "pub fn second(value: Int) -> Int\n",
                        "  let callback: fn(Int) -> Int = math::renamed\n",
                        "  callback(lib::math::renamed(value)) + math::target(value)\n",
                        "end\n\n",
                        "pub fn collisions(record: {renamed: Int}, value: Int) -> Int\n",
                        "  other_math::renamed(value)\n",
                        "  renamed(value)\n",
                        "  record.renamed\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use lib::math from \"example/pkg\"\n\n",
                        "pub fn other(value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        )
    }

    #[test]
    fn direct_dependency_function_alias_references_cover_qualified_calls_and_values() {
        let snapshot = direct_alias_snapshot();
        let expected = [
            ("main.veln", 9, 9),
            ("main.veln", 13, 40),
            ("main.veln", 14, 23),
            ("other.veln", 4, 9),
        ];

        for (source_path, line, column) in [
            ("main.veln", 9, 10),
            ("main.veln", 13, 41),
            ("main.veln", 14, 24),
            ("other.veln", 4, 10),
        ] {
            let result = query_snapshot(&snapshot, source_path, line, column).unwrap();
            assert_direct_dependency_alias(&result);
            assert_eq!(locations(&result.references), expected);
        }
    }

    #[test]
    fn direct_dependency_function_alias_references_keep_alias_and_target_separate() {
        let snapshot = direct_alias_snapshot();

        let alias = query_snapshot(&snapshot, "main.veln", 9, 10).unwrap();
        assert_direct_dependency_alias(&alias);
        assert_eq!(
            locations(&alias.references),
            [
                ("main.veln", 9, 9),
                ("main.veln", 13, 40),
                ("main.veln", 14, 23),
                ("other.veln", 4, 9),
            ]
        );

        let target = query_snapshot(&snapshot, "main.veln", 14, 51).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(
            target.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(locations(&target.references), [("main.veln", 14, 47)]);
    }

    #[test]
    fn direct_dependency_function_alias_references_exclude_collisions() {
        let snapshot = direct_alias_snapshot();

        let other_package_alias = query_snapshot(&snapshot, "main.veln", 18, 16).unwrap();
        assert_direct_dependency_alias(&other_package_alias);
        assert_eq!(
            locations(&other_package_alias.references),
            [("main.veln", 18, 15)]
        );

        let workspace_function = query_snapshot(&snapshot, "main.veln", 19, 4).unwrap();
        assert_eq!(workspace_function.selected_symbol.package_origin, None);
        assert_eq!(
            locations(&workspace_function.references),
            [("main.veln", 19, 3)]
        );
    }

    #[test]
    fn direct_dependency_function_alias_references_keep_unsupported_alias_chains_empty() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                (
                    "api.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                        "pub fn same_module_chain = renamed\n",
                        "pub fn other_module_chain = impl::renamed\n",
                        "pub fn private_module_chain = private_impl::renamed\n",
                    ),
                ),
                (
                    "impl.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  2\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
                (
                    "private_impl.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  3\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
            ],
            ["api.veln", "impl.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use api from \"example/pkg\"\n\n",
                    "pub fn main() -> Int\n",
                    "  api::same_module_chain()\n",
                    "  api::other_module_chain()\n",
                    "  api::private_module_chain()\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        for (line, column) in [(4, 9), (5, 9), (6, 9)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_direct_dependency_alias(&result);
            assert!(result.references.is_empty());
        }
    }
}
