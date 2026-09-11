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
    fn direct_dependency_public_function_alias_references_keep_alias_identity() {
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
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use math from \"example/pkg\"\n\n",
                        "pub fn first(value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n\n",
                        "pub fn second(record: {renamed: Int}, value: Int) -> Int\n",
                        "  let callback: fn(Int) -> Int = math::renamed\n",
                        "  let renamed = record.renamed\n",
                        "  callback(math::renamed(value)) + math::target(value) + renamed\n",
                        "end\n\n",
                        "# renamed mention\n",
                        "pub fn renamed(value: Int) -> Int\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use math from \"example/pkg\"\n\n",
                        "pub fn other(value: Int) -> Int\n",
                        "  math::renamed(value)\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (line, column) in [(4, 10)] {
            let result = query_snapshot(&snapshot, "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
            assert_eq!(
                result.selected_symbol.declaration_kind,
                SymbolDeclarationKind::PublicAlias
            );
            assert_eq!(
                result.selected_symbol.package_origin,
                Some(PackageOrigin::DirectDependency)
            );
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
            assert_eq!(
                locations(&result.references),
                [
                    ("main.veln", 4, 9),
                    ("main.veln", 8, 40),
                    ("main.veln", 10, 18),
                    ("other.veln", 4, 9),
                ]
            );
        }

        let target = query_snapshot(&snapshot, "main.veln", 10, 42).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(locations(&target.references), [("main.veln", 10, 42)]);
    }

    #[test]
    fn direct_dependency_function_alias_reference_boundaries_are_empty() {
        let fixtures = [
            (
                "alias chain",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                    "pub fn chain = renamed\n",
                ),
                "math::chain()",
            ),
            (
                "invalid-casing function alias",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn Renamed = target\n",
                ),
                "math::Renamed()",
            ),
            (
                "type alias",
                concat!(
                    "pub type Item\n",
                    "end\n\n",
                    "pub type Renamed = Item\n",
                ),
                "math::Renamed",
            ),
            (
                "schema alias",
                concat!(
                    "pub schema Packet\n",
                    "  value: Int\n",
                    "end\n\n",
                    "pub schema Renamed = Packet\n",
                ),
                "math::Renamed",
            ),
        ];

        for (case, dependency_text, expression) in fixtures {
            let dependency =
                dependency_snapshot("example/pkg", &[("math.veln", dependency_text)], ["math.veln"]);
            let result = dependency_query(dependency, expression);

            assert!(
                result.is_none_or(|result| result.references.is_empty()),
                "accepted {case}"
            );
        }
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
    fn standard_library_public_function_alias_references_keep_alias_identity() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                concat!(
                    "pub fn target(value: Int) -> Int\n",
                    "  value\n",
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
                    "  renamed(1)\n",
                    "end\n\n",
                    "# renamed mention\n",
                    "pub fn second() -> fn(Int) -> Int\n",
                    "  prelude::renamed\n",
                    "end\n\n",
                    "pub fn local(record: {renamed: Int}) -> Int\n",
                    "  let renamed = record.renamed\n",
                    "  renamed\n",
                    "end\n\n",
                    "pub fn target_call() -> Int\n",
                    "  prelude::target(2)\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                concat!(
                    "pub fn bare() -> Int\n",
                    "  renamed(0)\n",
                    "end\n\n",
                    "pub fn other() -> Int\n",
                    "  prelude::renamed(3)\n",
                    "end\n",
                ),
            ),
        ])
        .with_standard_library(standard_library);

        for (source_path, line, column) in [
            ("main.veln", 2, 3),
            ("main.veln", 7, 12),
            ("other.veln", 2, 3),
            ("other.veln", 6, 12),
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
                    ("main.veln", 7, 12),
                    ("other.veln", 2, 3),
                    ("other.veln", 6, 12),
                ]
            );
        }

        let target = query_snapshot(&snapshot, "main.veln", 16, 13).unwrap();
        assert_eq!(
            target.selected_symbol.declaration_kind,
            SymbolDeclarationKind::Declaration
        );
        assert_eq!(locations(&target.references), [("main.veln", 16, 12)]);
    }
