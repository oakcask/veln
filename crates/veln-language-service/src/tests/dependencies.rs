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

        for (case, line, column) in [("parameter", 2, 4), ("local", 7, 4)] {
            assert!(
                navigate(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line,
                        column,
                    },
                )
                .is_none(),
                "accepted shadowed {case} call"
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
