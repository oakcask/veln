    fn assert_standard_library_alias(result: &NavigationResult) {
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
    }

    fn standard_alias_library() -> DirectDependencySnapshot {
        standard_library_snapshot(
            &[
                (
                    "api.veln",
                    "pub fn target(value: Int) -> Int\n  value\nend\n\npub fn renamed = target\n",
                ),
                (
                    "other_function.veln",
                    "pub fn renamed(value: Int) -> Int\n  value + 1\nend\n",
                ),
                (
                    "other_alias.veln",
                    "pub fn target(value: Int) -> Int\n  value + 2\nend\n\npub fn renamed = target\n",
                ),
                (
                    "private_alias.veln",
                    "pub fn target(value: Int) -> Int\n  value + 5\nend\n\nfn renamed = target\n",
                ),
                (
                    "hidden_alias.veln",
                    "pub fn target(value: Int) -> Int\n  value + 6\nend\n\npub fn renamed = target\n",
                ),
            ],
            [
                "api.veln",
                "other_function.veln",
                "other_alias.veln",
                "private_alias.veln",
            ],
        )
    }

    fn alias_boundary_dependencies() -> Vec<DirectDependencySnapshot> {
        vec![
            dependency_snapshot(
                "example/functions",
                &[(
                    "dep_function.veln",
                    concat!(
                        "use api from \"std\"\n\n",
                        "pub fn outside_project(value: Int) -> Int\n",
                        "  api::renamed(value)\n",
                        "end\n\n",
                        "pub fn renamed(value: Int) -> Int\n",
                        "  value + 3\n",
                        "end\n",
                    ),
                )],
                ["dep_function.veln"],
            ),
            dependency_snapshot(
                "example/aliases",
                &[(
                    "dep_alias.veln",
                    "pub fn target(value: Int) -> Int\n  value + 4\nend\n\npub fn renamed = target\n",
                )],
                ["dep_alias.veln"],
            ),
        ]
    }

    fn alias_boundary_snapshot() -> EffectiveProjectSnapshot {
        EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use api from \"std\"\n",
                        "use other_function from \"std\"\n",
                        "use other_alias from \"std\"\n",
                        "use private_alias from \"std\"\n",
                        "use hidden_alias from \"std\"\n",
                        "use dep_function from \"example/functions\"\n",
                        "use dep_alias from \"example/aliases\"\n\n",
                        "pub fn renamed(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "pub fn main(value: Int) -> Int\n",
                        "  api::renamed(value)\n",
                        "  other_function::renamed(value)\n",
                        "  other_alias::renamed(value)\n",
                        "  private_alias::renamed(value)\n",
                        "  hidden_alias::renamed(value)\n",
                        "  dep_function::renamed(value)\n",
                        "  dep_alias::renamed(value)\n",
                        "  renamed(value)\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    "use api from \"std\"\n\npub fn other(value: Int) -> Int\n  api::renamed(value)\nend\n",
                ),
            ],
            alias_boundary_dependencies(),
        )
        .with_standard_library(standard_alias_library())
    }

    fn assert_alias_selection(
        snapshot: &EffectiveProjectSnapshot,
        source_path: &str,
        line: usize,
        column: usize,
        expected_references: impl IntoIterator<Item = (&'static str, usize, usize)>,
    ) {
        let result = query_snapshot(snapshot, source_path, line, column).unwrap();
        assert_standard_library_alias(&result);
        assert_eq!(
            locations(&result.references),
            expected_references.into_iter().collect::<Vec<_>>()
        );
    }

    fn assert_function_reference_case(
        snapshot: &EffectiveProjectSnapshot,
        case: &str,
        position: (usize, usize),
        origin: Option<PackageOrigin>,
        declaration_kind: SymbolDeclarationKind,
        references: &[(&'static str, usize, usize)],
    ) {
        let result = query_snapshot(snapshot, "main.veln", position.0, position.1)
            .unwrap_or_else(|| panic!("did not select {case}"));
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function, "{case}");
        assert_eq!(result.selected_symbol.package_origin, origin, "{case}");
        assert_eq!(
            result.selected_symbol.declaration_kind, declaration_kind,
            "{case}"
        );
        assert_eq!(locations(&result.references), references, "{case}");
    }

    #[test]
    fn explicit_standard_library_function_alias_references_cover_qualified_calls_and_values() {
        let standard_library = standard_library_snapshot(
            &[(
                "api.veln",
                "pub fn target(value: Int) -> Int\n  value\nend\n\npub fn renamed = target\n",
            )],
            ["api.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn first(value: Int) -> Int\n",
                "  api::renamed(value)\n",
                "end\n\n",
                "pub fn second(value: Int) -> Int\n",
                "  let callback: fn(Int) -> Int = api::renamed\n",
                "  callback(api::renamed(value)) + api::target(value)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);
        let expected = [
            ("main.veln", 4, 8),
            ("main.veln", 8, 39),
            ("main.veln", 9, 17),
        ];

        for (line, column) in [(4, 9), (8, 40), (9, 18)] {
            assert_alias_selection(&snapshot, "main.veln", line, column, expected);
        }

        let target = query_snapshot(&snapshot, "main.veln", 9, 40).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(locations(&target.references), [("main.veln", 9, 40)]);
    }

    #[test]
    fn standard_library_function_alias_references_keep_identity_and_project_boundaries() {
        let snapshot = alias_boundary_snapshot();

        assert_alias_selection(
            &snapshot,
            "main.veln",
            14,
            9,
            [("main.veln", 14, 8), ("other.veln", 4, 8)],
        );

        assert_function_reference_case(
            &snapshot,
            "standard library function",
            (15, 21),
            Some(PackageOrigin::StandardLibrary),
            SymbolDeclarationKind::Declaration,
            &[("main.veln", 15, 19)],
        );
        assert_function_reference_case(
            &snapshot,
            "standard library alias with another target",
            (16, 18),
            Some(PackageOrigin::StandardLibrary),
            SymbolDeclarationKind::PublicAlias,
            &[("main.veln", 16, 16)],
        );
        assert_function_reference_case(
            &snapshot,
            "direct dependency function",
            (19, 18),
            Some(PackageOrigin::DirectDependency),
            SymbolDeclarationKind::Declaration,
            &[("main.veln", 19, 17)],
        );
        assert_function_reference_case(
            &snapshot,
            "direct dependency alias",
            (20, 15),
            Some(PackageOrigin::DirectDependency),
            SymbolDeclarationKind::PublicAlias,
            &[("main.veln", 20, 14)],
        );
        assert_function_reference_case(
            &snapshot,
            "workspace function",
            (21, 4),
            None,
            SymbolDeclarationKind::Declaration,
            &[("main.veln", 21, 3)],
        );

        for (case, line, column) in [
            ("private standard library alias", 17, 18),
            ("non-exported standard library alias", 18, 17),
        ] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "selected {case}"
            );
        }
    }

    #[test]
    fn standard_library_function_alias_selection_requires_public_exported_alias() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "api.veln",
                    "pub fn target(value: Int) -> Int\n  value\nend\n\nfn private_alias = target\n",
                ),
                (
                    "hidden.veln",
                    "pub fn target(value: Int) -> Int\n  value\nend\n\npub fn hidden_alias = target\n",
                ),
            ],
            ["api.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n",
                "use hidden from \"std\"\n\n",
                "pub fn main(value: Int) -> Int\n",
                "  api::private_alias(value)\n",
                "  hidden::hidden_alias(value)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (case, line, column) in [("private alias", 5, 9), ("non-exported alias", 6, 12)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "selected {case}"
            );
        }
    }

    #[test]
    fn standard_library_prelude_function_alias_references_cover_implicit_forms_and_shadowing() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "pub fn first() -> Int\n",
                    "  renamed()\n",
                    "end\n\n",
                    "pub fn second() -> Int\n",
                    "  let bare_callback: fn() -> Int = renamed\n",
                    "  let qualified_callback: fn() -> Int = prelude::renamed\n",
                    "  bare_callback() + qualified_callback() + prelude::renamed()\n",
                    "end\n\n",
                    "pub fn local_shadow(renamed: fn() -> Int) -> Int\n",
                    "  renamed()\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                "pub fn other() -> Int\n  renamed() + prelude::renamed()\nend\n",
            ),
        ])
        .with_standard_library(standard_library);
        let expected = [
            ("main.veln", 2, 3),
            ("main.veln", 6, 36),
            ("main.veln", 7, 50),
            ("main.veln", 8, 53),
            ("other.veln", 2, 3),
            ("other.veln", 2, 24),
        ];

        for (source_path, line, column) in [
            ("main.veln", 2, 4),
            ("main.veln", 6, 36),
            ("main.veln", 7, 50),
            ("main.veln", 8, 53),
            ("other.veln", 2, 4),
            ("other.veln", 2, 24),
        ] {
            assert_alias_selection(&snapshot, source_path, line, column, expected);
        }

        let shadowed = query_snapshot(&snapshot, "main.veln", 12, 4).unwrap();
        assert_eq!(shadowed.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_eq!(locations(&shadowed.references), [("main.veln", 12, 3)]);
    }

    #[test]
    fn standard_library_function_alias_chains_return_no_references() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                    "pub fn chained = renamed\n",
                ),
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "pub fn main() -> Int\n  prelude::chained()\nend\n",
        )])
        .with_standard_library(standard_library);
        let result = query_snapshot(&snapshot, "main.veln", 2, 12).unwrap();

        assert_standard_library_alias(&result);
        assert!(result.references.is_empty());
    }

    #[test]
    fn standard_library_function_alias_chains_through_non_exported_sources_return_no_references() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "api.veln",
                    "pub fn chained = implementation::renamed\n",
                ),
                (
                    "implementation.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
            ],
            ["api.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  api::chained()\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);
        let result = query_snapshot(&snapshot, "main.veln", 4, 9).unwrap();

        assert_standard_library_alias(&result);
        assert!(result.references.is_empty());
    }

    #[test]
    fn standard_library_function_alias_references_require_resolved_function_targets() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "api.veln",
                    concat!(
                        "use implementation\n\n",
                        "pub type Document\n",
                        "  pub Text(String)\n",
                        "end\n\n",
                        "pub fn valid = implementation::target\n",
                        "pub fn missing = missing\n",
                        "pub fn wrong_kind = Document\n",
                        "pub fn invalid_case = Missing\n",
                    ),
                ),
                (
                    "implementation.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n",
                    ),
                ),
            ],
            ["api.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  api::valid()\n",
                "  api::missing()\n",
                "  api::wrong_kind()\n",
                "  api::invalid_case()\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        assert_function_reference_case(
            &snapshot,
            "valid alias target in non-exported source",
            (4, 9),
            Some(PackageOrigin::StandardLibrary),
            SymbolDeclarationKind::PublicAlias,
            &[("main.veln", 4, 8)],
        );

        for (case, line, column) in [
            ("unresolved target", 5, 9),
            ("wrong-kind target", 6, 9),
            ("invalid-casing target", 7, 9),
        ] {
            assert_function_reference_case(
                &snapshot,
                case,
                (line, column),
                Some(PackageOrigin::StandardLibrary),
                SymbolDeclarationKind::PublicAlias,
                &[],
            );
        }
    }
