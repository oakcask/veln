    #[test]
    fn workspace_and_dependency_schema_alias_imports_collide_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "lib/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            )],
            ["lib/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "workspace/wire.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Mid = Packet\n",
                        "pub schema Alias = Mid\n",
                    ),
                ),
                source("workspace_import.veln", "mod app\n\nuse workspace::wire\n"),
                source(
                    "dependency_import.veln",
                    "mod app\n\nuse lib::wire from \"example/dep\"\n",
                ),
                source(
                    "operation.veln",
                    concat!(
                        "mod app\n\n",
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        assert!(
            query_snapshot(&snapshot, "operation.veln", 4, 16).is_none(),
            "a workspace import and dependency import with the same implicit qualifier must be ambiguous across module sources"
        );
    }

    #[test]
    fn exact_workspace_and_dependency_schema_alias_imports_are_ambiguous() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "workspace/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                    "pub schema Fallback = Packet\n",
                ),
            )],
            ["workspace/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "workspace/wire.veln",
                    concat!(
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Alias = Packet\n",
                        "pub schema Fallback\n  value: Int\nend\n",
                    ),
                ),
                source(
                    "main.veln",
                    concat!(
                        "use workspace::wire\n",
                        "use workspace::wire from \"example/dep\"\n\n",
                        "fn operations(view: ByteView) -> ()\n",
                        "  decode workspace::wire::Alias from view at byte_offset(0)?\n",
                        "  decode workspace::wire::Fallback from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![dependency],
        );

        for (line, column) in [(5, 27), (6, 27)] {
            assert!(
                query_snapshot(&snapshot, "main.veln", line, column).is_none(),
                "an exact workspace and dependency import collision must block alias selection and schema fallback"
            );
        }
    }

    #[test]
    fn dependency_alias_and_schema_imports_collide_across_module_sources() {
        let alias_dependency = dependency_snapshot(
            "example/alias",
            &[(
                "a/wire.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Mid = Packet\n",
                    "pub schema Alias = Mid\n",
                ),
            )],
            ["a/wire.veln"],
        );
        let schema_dependency = dependency_snapshot(
            "example/schema",
            &[("b/wire.veln", "pub schema Alias\n  value: Int\nend\n")],
            ["b/wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "alias_import.veln",
                    "mod app\n\nuse a::wire from \"example/alias\"\n",
                ),
                source(
                    "operation.veln",
                    concat!(
                        "mod app\n\n",
                        "use b::wire from \"example/schema\"\n\n",
                        "schema Frame\n",
                        "  value: wire::Alias\n",
                        "end\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode wire::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![alias_dependency, schema_dependency],
        );

        assert!(
            query_snapshot(&snapshot, "operation.veln", 6, 16).is_none(),
            "module-wide alias ambiguity must block file-local schema fallback"
        );
    }

    #[test]
    fn invalid_dependency_schema_alias_imports_block_across_module_sources() {
        let dependency = dependency_snapshot(
            "example/dep",
            &[
                (
                    "alias.veln",
                    concat!(
                        "mod dep\n\n",
                        "pub schema Packet\n  value: Int\nend\n\n",
                        "pub schema Mid = Packet\n",
                        "pub schema Alias = Mid\n",
                    ),
                ),
                (
                    "schema.veln",
                    "mod dep\n\npub schema Alias\n  value: Int\nend\n",
                ),
            ],
            ["alias.veln", "schema.veln"],
        );

        for (name, import_sources) in [
            (
                "duplicate import",
                vec![
                    source(
                        "import_a.veln",
                        "mod app\n\nuse dep from \"example/dep\"\n",
                    ),
                    source(
                        "import_b.veln",
                        "mod app\n\nuse dep from \"example/dep\"\n",
                    ),
                ],
            ),
            (
                "recovered import",
                vec![source(
                    "import.veln",
                    "mod app\n\nuse dep from \"example/dep\" unexpected\n",
                )],
            ),
        ] {
            let mut sources = import_sources;
            sources.push(source(
                "operation.veln",
                concat!(
                    "mod app\n\n",
                    "schema Frame\n",
                    "  value: dep::Alias\n",
                    "end\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::Alias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            ));
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                sources,
                vec![dependency.clone()],
            );

            assert!(
                query_snapshot(&snapshot, "operation.veln", 4, 15).is_none(),
                "{name} must block dependency alias fallback across module sources"
            );
        }
    }

    #[test]
    fn package_schema_references_require_public_exported_direct_dependencies() {
        let direct = dependency_snapshot(
            "example/dep",
            &[
                (
                    "public.veln",
                    concat!(
                        "pub schema Public\n  value: Int\nend\n\n",
                        "pub schema badSchema\n  value: Int\nend\n\n",
                        "pub schema Alias = Public\n",
                    ),
                ),
                ("private.veln", "schema Private\n  value: Int\nend\n"),
                ("hidden.veln", "pub schema Hidden\n  value: Int\nend\n"),
            ],
            ["public.veln", "private.veln"],
        );
        let standard = standard_library_snapshot(
            &[("wire.veln", "pub schema Standard\n  value: Int\nend\n")],
            ["wire.veln"],
        );
        let mismatched = dependency_snapshot(
            "other/dep",
            &[("other.veln", "pub schema Public\n  value: Int\nend\n")],
            ["other.veln"],
        );
        let bridge = dependency_snapshot(
            "bridge/dep",
            &[(
                "bridge.veln",
                concat!(
                    "use public from \"transitive/dep\"\n\n",
                    "pub fn consume(view: ByteView) -> ()\n",
                    "  decode public::Public from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            ["bridge.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![
                source(
                    "main.veln",
                    concat!(
                    "use public from \"example/dep\"\n",
                    "use private from \"example/dep\"\n",
                    "use hidden from \"example/dep\"\n",
                    "use wire from \"std\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode public::Public from view at byte_offset(0)?\n",
                    "  decode private::Private from view at byte_offset(0)?\n",
                    "  decode hidden::Hidden from view at byte_offset(0)?\n",
                    "  decode wire::Standard from view at byte_offset(0)?\n",
                    "  decode public::badSchema from view at byte_offset(0)?\n",
                        "  decode public::Alias from view at byte_offset(0)?\n",
                        "  decode public::Missing from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "mismatch.veln",
                    concat!(
                        "use public from \"other/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode public::Public from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
                source(
                    "transitive.veln",
                    concat!(
                        "use public from \"transitive/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode public::Public from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            vec![direct, mismatched, bridge],
        )
        .with_standard_library(standard);

        let public = query_snapshot(&snapshot, "main.veln", 7, 18).unwrap();
        assert_eq!(locations(&public.references), [("main.veln", 7, 18)]);
        assert!(query_snapshot(&snapshot, "main.veln", 8, 19).is_none());
        assert!(query_snapshot(&snapshot, "main.veln", 9, 18).is_none());
        let standard = query_snapshot(&snapshot, "main.veln", 10, 16).unwrap();
        assert_eq!(locations(&standard.references), [("main.veln", 10, 16)]);
        let invalid_casing = query_snapshot(&snapshot, "main.veln", 11, 18).unwrap();
        assert!(matches!(
            invalid_casing.definition.source,
            NavigationSource::Package { .. }
        ));
        assert!(invalid_casing.references.is_empty());
        let alias = query_snapshot(&snapshot, "main.veln", 12, 18).unwrap();
        assert_eq!(locations(&alias.references), [("main.veln", 12, 18)]);
        assert!(query_snapshot(&snapshot, "main.veln", 13, 18).is_none());
        assert!(query_snapshot(&snapshot, "mismatch.veln", 4, 18).is_none());
        assert!(query_snapshot(&snapshot, "transitive.veln", 4, 18).is_none());
    }

    #[test]
    fn direct_dependency_schema_alias_references_require_a_unique_bare_public_target() {
        let cases = [
            (
                "private target",
                concat!(
                    "schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "missing target",
                "pub schema Alias = Missing\n",
            ),
            (
                "invalid-casing target",
                "pub schema Alias = badTarget\n",
            ),
            (
                "wrong kind target",
                "pub type Packet\nend\n\npub schema Alias = Packet\n",
            ),
            (
                "qualified target",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = dep::Packet\n",
                ),
            ),
            (
                "duplicate target",
                concat!(
                    "pub schema Packet\n  left: Int\nend\n\n",
                    "pub schema Packet\n  right: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "duplicate alias",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "private schema collides with public alias",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "schema Alias\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "private schema collides with public target",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "schema Packet\n  hidden: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "alias cycle",
                "pub schema Alias = Other\npub schema Other = Alias\n",
            ),
            (
                "schema collision",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias\n  value: Int\nend\n\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "target alias collision",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Other\n  value: Int\nend\n\n",
                    "pub schema Packet = Other\n",
                    "pub schema Alias = Packet\n",
                ),
            ),
            (
                "recovered alias declaration",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema Alias =\n",
                ),
            ),
        ];

        for (name, dependency_source) in cases {
            let dependency = dependency_snapshot(
                "example/dep",
                &[("dep.veln", dependency_source)],
                ["dep.veln"],
            );
            let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "main.veln",
                    concat!(
                        "use dep from \"example/dep\"\n\n",
                        "fn read(view: ByteView) -> ()\n",
                        "  decode dep::Alias from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                )],
                vec![dependency],
            );

            assert!(
                query_snapshot(&snapshot, "main.veln", 4, 16).is_none(),
                "{name} must not select an alias or fall back to another schema"
            );
        }

        let dependency = dependency_snapshot(
            "example/dep",
            &[(
                "dep.veln",
                concat!(
                    "pub schema Packet\n  value: Int\nend\n\n",
                    "pub schema badAlias = Packet\n",
                ),
            )],
            ["dep.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use dep from \"example/dep\"\n\n",
                    "fn read(view: ByteView) -> ()\n",
                    "  decode dep::badAlias from view at byte_offset(0)?\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );
        assert!(query_snapshot(&snapshot, "main.veln", 4, 16).is_none());
    }
