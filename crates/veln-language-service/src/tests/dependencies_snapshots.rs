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
