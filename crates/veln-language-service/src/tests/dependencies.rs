    #[test]
    fn workspace_function_wins_over_bare_standard_prelude_fallback() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn visible(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "fn visible(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n\n",
                "pub fn main() -> Int\n",
                "  visible(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column: 4,
            },
        )
        .unwrap();

        assert_location(&result.definition, "main.veln", 1, 4);
    }

    #[test]
    fn standard_library_definition_requires_public_exported_visibility() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "fn hidden(value: Int) -> Int\n  value\nend\n",
                ),
                (
                    "private.veln",
                    "pub fn unavailable(value: Int) -> Int\n  value\nend\n",
                ),
            ],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use private from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  prelude::hidden(1)\n",
                "  private::unavailable(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (line, column) in [(4, 12), (5, 13)] {
            assert!(
                navigate(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line,
                        column,
                    },
                )
                .is_none()
            );
        }
    }

    #[test]
    fn standard_library_prelude_qualified_type_segments_share_navigation() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub type Vec\n  pub Empty\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "pub fn main(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
        )])
        .with_standard_library(standard_library);

        let module_selection = SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 1,
            column: 22,
        };
        assert!(navigate(&snapshot, module_selection).is_none());

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 30,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(locations(&result.references), [("main.veln", 1, 29), ("main.veln", 1, 51)]);
        let NavigationSource::Package { uri } = &result.definition.source else {
            panic!("standard prelude type definition did not use a package location");
        };
        assert!(uri.starts_with("veln-pkg:///std/snapshot/"), "{uri}");
        assert!(uri.ends_with("/prelude.veln"), "{uri}");
        assert!(validate_rename(&result, "Items").is_ok());
        assert_rename_invalid_case(
            validate_rename(&result, "items").unwrap_err(),
            RenameNameClass::Type,
            "items",
            RenameRequiredInitial::AsciiUppercase,
        );
    }

    #[test]
    fn standard_library_bare_prelude_fallback_respects_local_shadowing() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Result<Byte, String>\n  prelude_builtin::byte(value)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "pub fn parameter_shadow(byte: fn(Int) -> Result<Byte, String>) -> Result<Byte, String>\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn local_shadow() -> Result<Byte, String>\n",
                "  let byte: fn(Int) -> Result<Byte, String> = prelude::byte\n",
                "  byte(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (case, line, column, definition_line, definition_column) in
            [("parameter", 2, 4, 1, 25), ("local", 7, 4, 6, 7)]
        {
            let result = navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            )
            .unwrap_or_else(|| panic!("shadowed {case} call should select the local binding"));
            assert_eq!(result.selected_symbol.kind, SymbolKind::ValueBinding);
            assert_location(
                &result.definition,
                "main.veln",
                definition_line,
                definition_column,
            );
        }
    }

    #[test]
    fn standard_library_bare_prelude_fallback_rejects_ambiguous_imports() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
            )],
            ["math.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn vec_len(items: Vec<A>) -> Int\n  prelude_builtin::vec_len(items)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main(items: Vec<Int>) -> Int\n",
                    "  vec_len(items)\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 4,
                    column: 4,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn invalid_imported_constructor_casing_falls_back_to_bare_prelude_function() {
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
                    "use model\n\n",
                    "pub fn main() -> Token\n",
                    "  byte(1)\n",
                    "end\n",
                ),
            ),
            source(
                "model.veln",
                concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
            ),
        ])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(
            (
                result.definition.span.start.line,
                result.definition.span.start.column
            ),
            (1, 8)
        );
    }

    #[test]
    fn invalid_reexported_constructor_casing_does_not_hide_bare_prelude_function() {
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
                    "use facade\n\n",
                    "pub fn bare() -> Token\n",
                    "  byte(1)\n",
                    "end\n\n",
                    "pub fn qualified() -> Token\n",
                    "  facade::byte(2)\n",
                    "end\n",
                ),
            ),
            source(
                "facade.veln",
                concat!("use model\n\n", "pub type Token = model::Token\n"),
            ),
            source(
                "model.veln",
                concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
            ),
        ])
        .with_standard_library(standard_library);

        let bare = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();
        assert_eq!(bare.selected_symbol.kind, SymbolKind::Function);
        assert!(matches!(
            bare.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(bare.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(
            (
                bare.definition.span.start.line,
                bare.definition.span.start.column
            ),
            (1, 8)
        );

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 8,
                    column: 11,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn standard_library_bare_prelude_fallback_ignores_private_workspace_imports() {
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
                    "use math\n\n",
                    "pub fn main() -> Int\n",
                    "  byte(1)\n",
                    "end\n",
                ),
            ),
            source("math.veln", "fn byte(value: Int) -> Int\n  0\nend\n"),
        ])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        let NavigationSource::Package { uri } = result.definition.source else {
            panic!("prelude definition did not use a package location");
        };
        assert!(uri.starts_with("veln-pkg:///std/snapshot/"), "{uri}");
    }

    #[test]
    fn standard_library_bare_prelude_fallback_rejects_same_module_package_imports() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
            )],
            ["math.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn vec_len(items: Vec<A>) -> Int\n  prelude_builtin::vec_len(items)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "math.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main(items: Vec<Int>) -> Int\n",
                    "  vec_len(items)\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("math.veln"),
                    line: 4,
                    column: 4,
                },
            )
            .is_none()
        );
    }

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
    fn standard_library_public_function_alias_definition_has_no_references() {
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
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "pub fn main() -> Int\n  prelude::renamed()\nend\n",
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

    #[test]
    fn direct_dependency_type_references_cover_project_type_roles_and_collisions() {
        let selected = dependency_snapshot(
            "example/pkg",
            &[("model.veln", "pub type Item\n  pub Ready(Int)\nend\n")],
            ["model.veln"],
        );
        let collision = dependency_snapshot(
            "other/pkg",
            &[("model.veln", "pub type Item\nend\n")],
            ["model.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                        "use model from \"example/pkg\"\n",
                        "use other_model from \"other/pkg\"\n\n",
                        "type Item\n",
                        "end\n\n",
                        "type Box\n",
                        "  Wrap(model::Item)\n",
                        "end\n\n",
                        "pub type Alias = model::Item\n\n",
                        "fn make(input: model::Item) -> model::Item\n",
                        "  model::Item::Ready(1)\n",
                        "end\n\n",
                        "fn other(input: other_model::Item) -> Item\n",
                        "  \"Item\"\n",
                        "end\n\n",
                        "# Item in a comment is not a type reference.\n",
                        "fn wrapped(items: Vec<model::Item>) -> Vec<model::Item>\n",
                        "  items\n",
                        "end\n",
                    ),
                ),
                source(
                    "other.veln",
                    concat!(
                        "use model from \"example/pkg\"\n\n",
                        "fn second(input: model::Item) -> model::Item\n",
                        "  input\n",
                        "end\n",
                    ),
                ),
            ],
            vec![selected, collision],
        );

        let result = query_snapshot(&snapshot, "main.veln", 13, 23).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::DirectDependency)
        );
        assert_eq!(result.definition.span.file.as_str(), "model.veln");
        assert!(matches!(
            result.definition.source,
            NavigationSource::Package { .. }
        ));
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 8, 15),
                ("main.veln", 11, 25),
                ("main.veln", 13, 23),
                ("main.veln", 13, 39),
                ("main.veln", 14, 10),
                ("main.veln", 22, 30),
                ("main.veln", 22, 51),
                ("other.veln", 3, 25),
                ("other.veln", 3, 41),
            ]
        );
    }

    #[test]
    fn standard_library_type_references_cover_prelude_and_qualified_forms() {
        let standard_library = standard_library_snapshot(
            &[("prelude.veln", "pub type Vec\n  pub Empty\nend\n")],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "fn first(items: Vec<Int>) -> Vec<Int>\n",
                    "  items\n",
                    "end\n\n",
                    "fn second(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n",
                    "  prelude::Vec::Empty()\n",
                    "end\n",
                ),
            ),
            source(
                "other.veln",
                "fn third(items: Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
            ),
        ])
        .with_standard_library(standard_library);

        let result = query_snapshot(&snapshot, "main.veln", 1, 17).unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(
            result.selected_symbol.package_origin,
            Some(PackageOrigin::StandardLibrary)
        );
        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        assert_eq!(
            locations(&result.references),
            [
                ("main.veln", 1, 17),
                ("main.veln", 1, 30),
                ("main.veln", 5, 27),
                ("main.veln", 5, 49),
                ("main.veln", 6, 12),
                ("other.veln", 1, 17),
                ("other.veln", 1, 39),
            ]
        );
    }

    #[test]
    fn unsupported_package_type_selections_do_not_expand_references() {
        struct Case {
            name: &'static str,
            snapshot: EffectiveProjectSnapshot,
            source_path: &'static str,
            line: usize,
            column: usize,
            expect_symbol: Option<SymbolKind>,
        }

        let cases = [
            Case {
                name: "private direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::Item) -> model::Item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "type Item\nend\n")],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "non-exported direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use hidden from \"example/pkg\"\n\n",
                            "fn read(input: hidden::Item) -> hidden::Item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[
                            ("public.veln", "pub fn ok() -> Int\n  1\nend\n"),
                            ("hidden.veln", "pub type Item\nend\n"),
                        ],
                        ["public.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 28,
                expect_symbol: None,
            },
            Case {
                name: "public direct dependency type alias",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::Alias) -> model::Alias\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[(
                            "model.veln",
                            "pub type Item\nend\n\npub type Alias = Item\n",
                        )],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "invalid-casing direct dependency type",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn read(input: model::item) -> model::item\n",
                            "  input\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "pub type item\nend\n")],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 3,
                column: 25,
                expect_symbol: None,
            },
            Case {
                name: "direct dependency constructor",
                snapshot: EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source(
                        "main.veln",
                        concat!(
                            "use model from \"example/pkg\"\n\n",
                            "fn make() -> model::Item\n",
                            "  model::Item::Ready(1)\n",
                            "end\n",
                        ),
                    )],
                    vec![dependency_snapshot(
                        "example/pkg",
                        &[("model.veln", "pub type Item\n  pub Ready(Int)\nend\n")],
                        ["model.veln"],
                    )],
                ),
                source_path: "main.veln",
                line: 4,
                column: 16,
                expect_symbol: Some(SymbolKind::Constructor),
            },
            Case {
                name: "standard library public type alias",
                snapshot: EffectiveProjectSnapshot::new(vec![source(
                    "main.veln",
                    concat!(
                        "fn read(input: prelude::Alias) -> prelude::Alias\n",
                        "  input\n",
                        "end\n",
                    ),
                )])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", "pub type Vec\nend\n\npub type Alias = Vec\n")],
                    ["prelude.veln"],
                )),
                source_path: "main.veln",
                line: 1,
                column: 27,
                expect_symbol: None,
            },
            Case {
                name: "standard library invalid-casing type",
                snapshot: EffectiveProjectSnapshot::new(vec![source(
                    "main.veln",
                    "fn read(input: prelude::vec) -> prelude::vec\n  input\nend\n",
                )])
                .with_standard_library(standard_library_snapshot(
                    &[("prelude.veln", "pub type vec\nend\n")],
                    ["prelude.veln"],
                )),
                source_path: "main.veln",
                line: 1,
                column: 27,
                expect_symbol: None,
            },
        ];

        for case in cases {
            let result = query_snapshot(&case.snapshot, case.source_path, case.line, case.column);
            match case.expect_symbol {
                Some(kind) => {
                    let result = result.unwrap_or_else(|| {
                        panic!("{} should navigate to an unsupported non-type symbol", case.name)
                    });
                    assert_eq!(result.selected_symbol.kind, kind, "{}", case.name);
                    assert!(
                        result.references.is_empty(),
                        "{} unexpectedly returned references: {:?}",
                        case.name,
                        locations(&result.references)
                    );
                }
                None => assert!(
                    result.is_none(),
                    "{} unexpectedly navigated to {:?}",
                    case.name,
                    result.map(|result| result.selected_symbol)
                ),
            }
        }
    }

    #[test]
    fn direct_dependency_invalid_function_casing_is_not_navigable() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[("math.veln", "pub fn Bad(value: Int) -> Int\n  value\nend\n")],
            ["math.veln"],
        );

        assert!(dependency_query(dependency, "math::Bad(1)").is_none());
    }

    #[test]
    fn workspace_references_ignore_dependency_sources_with_matching_modules() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\n  increment(value - 1)\nend\n",
            )],
            ["math.veln"],
        );

        let result = navigate(
            &EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "math.veln",
                    "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
                )],
                vec![dependency],
            ),
            SourcePosition {
                source: SourcePath::new("math.veln"),
                line: 1,
                column: 8,
            },
        )
        .unwrap();

        assert_location(&result.definition, "math.veln", 1, 8);
        assert!(result.references.is_empty());
    }

    #[test]
    fn direct_dependency_snapshot_derives_visibility_from_manifest() {
        let root = TempDependency::new(
            "example/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"example/pkg\"\n\n[lib]\nexports = [\"./math.veln\"]\n",
        );

        let dependency =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap();
        let result = dependency_query(dependency, "math::exposed()").unwrap();

        assert_eq!(result.definition.span.file.as_str(), "math.veln");
    }

    #[test]
    fn direct_dependency_snapshot_excludes_invalid_cased_exported_source_identity() {
        let root = TempDependency::new(
            "example/pkg",
            &[
                (
                    "App/value.veln",
                    "pub fn value() -> Int\n  1\nend\n",
                ),
                (
                    "ok.veln",
                    "pub fn value() -> Int\n  2\nend\n",
                ),
            ],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            concat!(
                "[package]\n",
                "name = \"example/pkg\"\n",
                "\n",
                "[lib]\n",
                "exports = [\"App/value.veln\", \"ok.veln\"]\n",
            ),
        );
        let dependency =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap();
        let project = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use App from \"example/pkg\"\n",
                    "use ok from \"example/pkg\"\n",
                    "\n",
                    "pub fn main() -> Int\n",
                    "  ok::value() + App::value()\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let valid = navigate(
            &project,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 7,
            },
        )
        .expect("valid sibling export should remain navigable");
        assert_eq!(valid.definition.span.file.as_str(), "ok.veln");

        assert!(
            navigate(
                &project,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 5,
                    column: 22,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn direct_dependency_snapshot_rejects_mismatched_manifest_identity() {
        let root = TempDependency::new(
            "other/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"other/pkg\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        );

        let error =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap_err();

        assert_eq!(
            error,
            DirectDependencySnapshotError::PackageNameMismatch {
                expected: "example/pkg".to_string(),
                actual: "other/pkg".to_string(),
            }
        );
    }

    #[test]
    fn direct_dependency_snapshot_rejects_manifest_without_package_name() {
        let root = TempDependency::new(
            "example/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nversion = \"0.1.0\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        );

        let error =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap_err();

        assert_eq!(error, DirectDependencySnapshotError::MissingPackageName);
    }

    #[test]
    fn dependency_virtual_sources_retain_nonexported_and_private_source_bytes() {
        let root = TempDependency::new(
            "example/pkg",
            &[
                ("public.veln", "pub fn exposed() -> Int\r\n  1\r\nend\r\n"),
                ("internal.veln", "fn hidden() -> Int\r\n  2\r\nend\r\n"),
            ],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"example/pkg\"\n\n[lib]\nexports = [\"public.veln\"]\n",
        );
        let dependency =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap();
        let retained = dependency
            .virtual_sources
            .entries()
            .map(|entry| entry.uri().to_string())
            .collect::<Vec<_>>();
        let project = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", "pub fn main() -> Int\n  0\nend\n")],
            vec![dependency],
        );

        assert_eq!(retained.len(), 2);
        for uri in retained {
            let expected = if uri.ends_with("/internal.veln") {
                b"fn hidden() -> Int\r\n  2\r\nend\r\n".as_slice()
            } else if uri.ends_with("/public.veln") {
                b"pub fn exposed() -> Int\r\n  1\r\nend\r\n".as_slice()
            } else {
                panic!("unexpected retained source URI {uri}");
            };
            assert_eq!(project.resolve_virtual_source(&uri), Some(expected));
        }
    }

    #[test]
    fn workspace_overlays_reuse_prepared_dependency_sources() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[("math.veln", "pub fn answer() -> Int\n  42\nend\n")],
            ["math.veln"],
        );
        let source_text = concat!(
            "use math from \"example/pkg\"\n\n",
            "pub fn main() -> Int\n",
            "  math::answer()\n",
            "end\n",
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", source_text)],
            vec![dependency],
        );

        reset_dependency_source_indexes();
        reset_dependency_source_parses();
        reset_dependency_path_classifications();
        assert!(query_snapshot(&snapshot, "main.veln", 4, 10).is_some());
        assert_eq!(dependency_source_indexes(), 1);
        assert_eq!(dependency_source_parses(), 1);
        assert_eq!(dependency_path_classifications(), 1);

        let overlay = snapshot.with_workspace_overlays([source("main.veln", source_text)]);
        assert!(query_snapshot(&overlay, "main.veln", 4, 10).is_some());
        assert_eq!(
            dependency_source_indexes(),
            1,
            "workspace overlays should reuse indexed dependency sources",
        );
        assert_eq!(
            dependency_source_parses(),
            1,
            "workspace overlays should reuse parsed dependency sources",
        );
        assert_eq!(
            dependency_path_classifications(),
            1,
            "workspace overlays should reuse classified dependency paths",
        );
    }
