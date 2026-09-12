    #[test]
    fn dependency_definition_requires_exported_source_and_public_function() {
        let fixtures = [
            (
                "private declaration",
                "fn increment(value: Int) -> Int\n  value + 1\nend\n",
                vec!["math.veln"],
            ),
            (
                "unexported source",
                "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
                Vec::new(),
            ),
        ];

        for (case, source_text, exports) in fixtures {
            let dependency =
                dependency_snapshot("example/pkg", &[("math.veln", source_text)], exports);
            assert!(
                dependency_query(dependency, "math::increment(1)").is_none(),
                "accepted {case}"
            );
        }
    }

    #[test]
    fn dependency_definition_requires_exact_external_import() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
            )],
            ["math.veln"],
        );
        let cases = [
            (
                "missing import",
                "pub fn main() -> Int\n  increment(1)\nend\n",
                2,
                4,
            ),
            (
                "workspace unqualified same module",
                "module math\n\npub fn main() -> Int\n  increment(1)\nend\n",
                4,
                4,
            ),
            (
                "different package",
                "use math from \"other/pkg\"\n\npub fn main() -> Int\n  math::increment(1)\nend\n",
                4,
                10,
            ),
            (
                "different module",
                "use other from \"example/pkg\"\n\npub fn main() -> Int\n  other::increment(1)\nend\n",
                4,
                11,
            ),
        ];

        for (case, text, line, column) in cases {
            let result = navigate(
                &EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source("main.veln", text)],
                    vec![dependency.clone()],
                ),
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            );
            assert!(result.is_none(), "accepted {case}");
        }
    }

    #[test]
    fn direct_dependency_function_references_cover_qualified_calls_and_values() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                concat!(
                    "pub fn increment(value: Int) -> Int\n",
                    "  increment(value - 1)\n",
                    "end\n",
                ),
            )],
            ["math.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use math from \"example/pkg\"\n\n",
                        "pub fn first(value: Int) -> Int\n",
                        "  math::increment(value)\n",
                        "end\n\n",
                        "pub fn second(value: Int) -> Int\n",
                        "  let callback: fn(Int) -> Int = math::increment\n",
                        "  callback(math::increment(value))\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use math from \"example/pkg\"\n\n",
                        "pub fn other(value: Int) -> Int\n",
                        "  math::increment(value)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (line, column) in [(4, 10), (8, 40), (9, 18)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();
            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(result.definition.span.file.as_str(), "math.veln");
            assert!(matches!(
                result.definition.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 4, 9),
                    ("main.veln", 8, 40),
                    ("main.veln", 9, 18),
                    ("other.veln", 4, 9),
                ]
            );
        }
    }

    #[test]
    fn direct_dependency_function_references_keep_identity_boundaries() {
        let selected = dependency_snapshot(
            "example/pkg",
            &[("lib/math.veln", "pub fn increment(value: Int) -> Int\n  value + 1\nend\n")],
            ["lib/math.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[("other_math.veln", "pub fn increment(value: Int) -> Int\n  value + 2\nend\n")],
            ["other_math.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use lib::math from \"example/pkg\"\n",
                        "use other_math from \"other/pkg\"\n\n",
                        "pub fn read(record: {field: Int}, value: Int) -> Int\n",
                        "  let local = value\n",
                        "  math::increment(value)\n",
                        "  other_math::increment(value)\n",
                        "  record.field + local\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        let result = query_snapshot(&snapshot, "main.veln", 6, 10).unwrap();

        assert_eq!(result.definition.span.file.as_str(), "lib/math.veln");
        assert_eq!(locations(&result.references), [("main.veln", 6, 9)]);
    }

    #[test]
    fn direct_dependency_public_function_alias_definition_has_no_references() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["math.veln"],
        );
        let result = dependency_query(dependency, "math::renamed(1)").unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(result.selected_symbol.declaration_kind, SymbolDeclarationKind::PublicAlias);
        assert_eq!(result.selected_symbol.package_origin, Some(PackageOrigin::DirectDependency));
        assert_eq!(result.definition.span.file.as_str(), "math.veln");
        assert_eq!(
            (
                result.definition.span.start.line,
                result.definition.span.start.column
            ),
            (5, 8)
        );
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
        assert!(result.references.is_empty());
    }

    #[test]
    fn explicit_standard_library_function_references_cover_qualified_calls_and_values() {
        let standard_library = standard_library_snapshot(
            &[(
                "api.veln",
                "pub fn exported(value: Int) -> Int\n  value\nend\n",
            )],
            ["api.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn first(value: Int) -> Int\n",
                "  api::exported(value)\n",
                "end\n\n",
                "pub fn second(value: Int) -> Int\n",
                "  let callback: fn(Int) -> Int = api::exported\n",
                "  callback(api::exported(value))\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);
        for (line, column) in [(4, 9), (8, 40), (9, 18)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(result.selected_symbol.declaration_kind, SymbolDeclarationKind::Declaration);
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(result.definition.span.file.as_str(), "api.veln");
            assert!(matches!(
                result.definition.source,
                NavigationSource::Package { .. }
            ));
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 4, 8),
                    ("main.veln", 8, 39),
                    ("main.veln", 9, 17),
                ]
            );
        }
    }

    #[test]
    fn standard_library_function_references_exclude_collisions() {
        let standard_library = standard_library_snapshot(
            &[(
                "api.veln",
                concat!(
                    "pub fn exported(value: Int) -> Int\n",
                    "  exported(value - 1)\n",
                    "end\n",
                ),
            )],
            ["api.veln"],
        );
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "dep.veln",
                "pub fn exported(value: Int) -> Int\n  value\nend\n",
            )],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use api from \"std\"\n",
                    "use dep from \"example/pkg\"\n\n",
                    "# exported mention\n",
                    "pub fn exported(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn first(record: {exported: Int}, value: Int) -> Int\n",
                    "  api::exported(value)\n",
                    "  dep::exported(value)\n",
                    "  record.exported\n",
                    "  \"exported\"\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn second(value: Int) -> Int\n",
                    "  let exported = value\n",
                    "  let callback: fn(Int) -> Int = api::exported\n",
                    "  callback(api::exported(exported))\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 10, 9).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(result.definition.span.file.as_str(), "api.veln");
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 10, 8),
                ("main.veln", 19, 39),
                ("main.veln", 20, 17),
            ]
        );
    }

    #[test]
    fn standard_library_prelude_function_references_cover_implicit_forms_and_shadowing() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "pub fn first(value: Int) -> Int\n",
                    "  byte(value)\n",
                    "end\n\n",
                    "pub fn second(value: Int) -> Int\n",
                    "  let callback: fn(Int) -> Int = prelude::byte\n",
                    "  callback(prelude::byte(value))\n",
                    "end\n\n",
                    "pub fn local_shadow(byte: fn(Int) -> Int) -> Int\n",
                    "  byte(value)\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "pub fn other(value: Int) -> Int\n",
                    "  prelude::byte(value)\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library);

        for (line, column) in [(2, 4), (6, 44), (7, 21), (2, 12)] {
            let (source_path, line, column) = if line == 2 && column == 12 {
                ("other.veln", line, column)
            } else {
                ("main.veln", line, column)
            };
            let result = query_snapshot(&snapshot, source_path, line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 2, 3),
                    ("main.veln", 6, 43),
                    ("main.veln", 7, 21),
                    ("other.veln", 2, 12),
                ]
            );
        }

        let shadowed = query_snapshot(&snapshot, "main.veln", 11, 4).unwrap();
        assert_eq!(shadowed.selected_symbol.kind, SymbolKind::ValueBinding);
        assert_eq!(locations(&shadowed.references), [("main.veln", 11, 3)]);
    }

    #[test]
    fn explicit_standard_library_function_alias_references_cover_qualified_calls_and_values() {
        let standard_library = standard_library_snapshot(
            &[(
                "api.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
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

        for (line, column) in [(4, 9), (8, 40), (9, 18)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(result.definition.span.file.as_str(), "api.veln");
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 4, 8),
                    ("main.veln", 8, 39),
                    ("main.veln", 9, 17),
                ]
            );
        }

        let target = query_snapshot(&snapshot, "main.veln", 9, 40).unwrap();
        assert_eq!(target.selected_symbol.declaration_kind, SymbolDeclarationKind::Declaration);
        assert_eq!(locations(&target.references), [("main.veln", 9, 40)]);
    }

    #[test]
    fn standard_library_function_alias_references_keep_identity_and_project_boundaries() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "api.veln",
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
                (
                    "other_function.veln",
                    "pub fn renamed(value: Int) -> Int\n  value + 1\nend\n",
                ),
                (
                    "other_alias.veln",
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value + 2\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
                (
                    "private_alias.veln",
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value + 5\n",
                        "end\n\n",
                        "fn renamed = target\n",
                    ),
                ),
                (
                    "hidden_alias.veln",
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value + 6\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                    ),
                ),
            ],
            [
                "api.veln",
                "other_function.veln",
                "other_alias.veln",
                "private_alias.veln",
            ],
        );
        let dependency_function = dependency_snapshot(
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
        );
        let dependency_alias = dependency_snapshot(
            "example/aliases",
            &[(
                "dep_alias.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value + 4\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
            )],
            ["dep_alias.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
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
                    concat!(
                        "use api from \"std\"\n\n",
                        "pub fn other(value: Int) -> Int\n",
                        "  api::renamed(value)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency_function, dependency_alias],
        )
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 14, 9).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(
            locations(&result.references),
            [("main.veln", 14, 8), ("other.veln", 4, 8)]
        );

        for (case, line, column, origin, declaration_kind, references) in [
            (
                "standard library function",
                15,
                21,
                Some(PackageOrigin::StandardLibrary),
                SymbolDeclarationKind::Declaration,
                vec![("main.veln", 15, 19)],
            ),
            (
                "standard library alias with another target",
                16,
                18,
                Some(PackageOrigin::StandardLibrary),
                SymbolDeclarationKind::PublicAlias,
                vec![("main.veln", 16, 16)],
            ),
            (
                "direct dependency function",
                19,
                18,
                Some(PackageOrigin::DirectDependency),
                SymbolDeclarationKind::Declaration,
                vec![("main.veln", 19, 17)],
            ),
            (
                "direct dependency alias",
                20,
                15,
                Some(PackageOrigin::DirectDependency),
                SymbolDeclarationKind::PublicAlias,
                Vec::new(),
            ),
            (
                "workspace function",
                21,
                4,
                None,
                SymbolDeclarationKind::Declaration,
                vec![("main.veln", 21, 3)],
            ),
        ] {
            let collision = query_snapshot(&snapshot, "main.veln", line, column)
                .unwrap_or_else(|| panic!("did not select {case}"));
            assert_eq!(collision.selected_symbol.kind, SymbolKind::Function, "{case}");
            assert_eq!(collision.selected_symbol.package_origin, origin, "{case}");
            assert_eq!(
                collision.selected_symbol.declaration_kind, declaration_kind,
                "{case}"
            );
            assert_eq!(locations(&collision.references), references, "{case}");
        }

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
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "fn private_alias = target\n",
                    ),
                ),
                (
                    "hidden.veln",
                    concat!(
                        "pub fn target(value: Int) -> Int\n",
                        "  value\n",
                        "end\n\n",
                        "pub fn hidden_alias = target\n",
                    ),
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
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                ),
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

        for (source_path, line, column) in [
            ("main.veln", 2, 4),
            ("main.veln", 6, 36),
            ("main.veln", 7, 50),
            ("main.veln", 8, 53),
            ("other.veln", 2, 4),
            ("other.veln", 2, 24),
        ] {
            let result = query_snapshot(&snapshot, source_path, line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::StandardLibrary)
            );
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 2, 3),
                    ("main.veln", 6, 36),
                    ("main.veln", 7, 50),
                    ("main.veln", 8, 53),
                    ("other.veln", 2, 3),
                    ("other.veln", 2, 24),
                ]
            );
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

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(
            result.selected_symbol.declaration_kind,
            SymbolDeclarationKind::PublicAlias
        );
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert!(result.references.is_empty());
    }
