use super::*;
use std::path::Path;

#[test]
fn definition_resolves_the_supported_workspace_symbol_set() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        line: usize,
        column: usize,
        definition_file: &'static str,
        definition_line: usize,
        definition_column: usize,
    }

    let cases = [
        Case {
            name: "function",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "fn helper() -> Int\n  1\nend\n\nfn main() -> Int\n  helper()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            definition_file: "main.veln",
            definition_line: 1,
            definition_column: 4,
        },
        Case {
            name: "constructor",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "type Token\n  Byte(Int)\nend\n\nfn main() -> Token\n  Byte(1)\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 4,
            definition_file: "main.veln",
            definition_line: 2,
            definition_column: 3,
        },
        Case {
            name: "handler context parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Adjust\n  amount(value: Int) -> Int\nend\n\nhandler adjust(callback: fn(Int) -> Int) handles Adjust\n  amount(value) => callback(value)\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 22,
            definition_file: "main.veln",
            definition_line: 5,
            definition_column: 16,
        },
        Case {
            name: "handler operation clause parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Choose\n  pick(value: Bool) -> Int\nend\n\nhandler choose() handles Choose\n  pick(value) => value\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 18,
            definition_file: "main.veln",
            definition_line: 6,
            definition_column: 8,
        },
        Case {
            name: "exact companion private function",
            files: vec![
                ("veln.toml", ""),
                ("math.veln", "fn helper() -> Int\n  1\nend\n"),
                (
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::helper()\nend\n",
                ),
            ],
            source: "math.test.veln",
            line: 4,
            column: 11,
            definition_file: "math.veln",
            definition_line: 1,
            definition_column: 4,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = definition_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        let location = &result["structuredContent"]["definition"];
        assert!(
            location["uri"]
                .as_str()
                .unwrap()
                .ends_with(case.definition_file),
            "{}: {location:#}",
            case.name
        );
        assert_eq!(
            location["range"]["start"]["line"], case.definition_line,
            "{}",
            case.name
        );
        assert_eq!(
            location["range"]["start"]["column"], case.definition_column,
            "{}",
            case.name
        );
    }
}

#[test]
fn definition_infers_project_and_isolates_other_sources_and_descendant_manifests() {
    let workspace = TempWorkspace::new("definition-scope");
    workspace.write("app/veln.toml", "");
    workspace.write("app/math.veln", "pub type Token\n  pub Byte(Int)\nend\n");
    workspace.write(
        "app/main.veln",
        "use math\n\nfn main() -> Token\n  math::Byte(1)\nend\n",
    );
    workspace.write("loose.veln", "fn main() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  1\nend\n");
    workspace.write("app/nested/veln.toml", "");
    workspace.write(
        "app/nested/main.veln",
        "use math\n\nfn main() -> Token\n  math::Byte(1)\nend\n",
    );

    let project = definition_result(&workspace, "app/main.veln", 4, 10);
    assert!(
        project["structuredContent"]["definition"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("app/math.veln")),
        "{project:#}"
    );
    for source in ["loose.veln", "app/nested/main.veln"] {
        let isolated = definition_result(
            &workspace,
            source,
            if source == "loose.veln" { 2 } else { 4 },
            if source == "loose.veln" { 4 } else { 10 },
        );
        assert_eq!(isolated["isError"], false, "{source}: {isolated:#}");
        assert_eq!(
            isolated["structuredContent"]["definition"],
            Value::Null,
            "{source}"
        );
    }
}

#[test]
fn definition_resolves_public_package_symbol_classes() {
    struct Case {
        name: &'static str,
        line: usize,
        column: usize,
        path: &'static str,
        declaration_line: usize,
        declaration_column: usize,
        declaration_end_column: usize,
    }

    let cases = [
        Case {
            name: "type",
            line: 3,
            column: 25,
            path: "dep.veln",
            declaration_line: 1,
            declaration_column: 10,
            declaration_end_column: 15,
        },
        Case {
            name: "schema",
            line: 12,
            column: 29,
            path: "dep.veln",
            declaration_line: 7,
            declaration_column: 12,
            declaration_end_column: 18,
        },
        Case {
            name: "constructor",
            line: 13,
            column: 8,
            path: "dep.veln",
            declaration_line: 2,
            declaration_column: 7,
            declaration_end_column: 12,
        },
        Case {
            name: "function",
            line: 13,
            column: 19,
            path: "dep.veln",
            declaration_line: 11,
            declaration_column: 8,
            declaration_end_column: 17,
        },
        Case {
            name: "function alias",
            line: 13,
            column: 39,
            path: "dep.veln",
            declaration_line: 15,
            declaration_column: 8,
            declaration_end_column: 15,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_workspace_with_navigation_dependency(&workspace, DependencySourceKind::Path);
        let mut server = initialized_server(&workspace);

        let result = server
            .definition_tool(&json!({"source":"main.veln","line":case.line,"column":case.column}));
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        let location = &result["structuredContent"]["definition"];
        let uri = location["uri"]
            .as_str()
            .unwrap_or_else(|| panic!("{} returned no definition: {result:#}", case.name));
        assert!(
            uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"),
            "{}: {uri}",
            case.name
        );
        assert!(uri.ends_with(case.path), "{}: {uri}", case.name);
        assert!(!uri.contains("vendor/dep"), "{}: {uri}", case.name);
        assert_eq!(
            location["range"],
            json!({
                "start": {"line": case.declaration_line, "column": case.declaration_column},
                "end": {"line": case.declaration_line, "column": case.declaration_end_column}
            }),
            "{}",
            case.name
        );

        let read = read_resource(&mut server, uri);
        assert_eq!(read["result"]["contents"][0]["uri"], uri);
        assert_eq!(
            read["result"]["contents"][0]["text"],
            navigation_dependency_source()
        );
    }
}

#[test]
fn definition_round_trips_crlf_non_ascii_package_source() {
    let workspace = TempWorkspace::new("definition-package-crlf-unicode");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use unicode from \"example/dep\"\n\n",
            "fn main() -> String\n",
            "  unicode::label()\n",
            "end\n",
        ),
    );
    workspace.write("vendor/dep/veln.toml", &unicode_dependency_manifest());
    workspace.write_bytes(
        "vendor/dep/unicode.veln",
        unicode_dependency_source().as_bytes(),
    );
    let mut server = initialized_server(&workspace);

    let result = server.definition_tool(&json!({"source":"main.veln","line":4,"column":12}));
    assert_eq!(result["isError"], false, "{result:#}");
    let location = &result["structuredContent"]["definition"];
    let uri = location["uri"].as_str().unwrap();
    assert!(
        uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"),
        "{uri}"
    );
    assert!(uri.ends_with("/unicode.veln"), "{uri}");
    assert_eq!(
        location["range"],
        json!({"start":{"line":1,"column":8},"end":{"line":1,"column":13}})
    );

    let read = read_resource(&mut server, uri);
    assert_eq!(read["result"]["contents"][0]["uri"], uri);
    assert_eq!(
        read["result"]["contents"][0]["text"],
        unicode_dependency_source()
    );
}

#[test]
fn definition_resolves_implicit_and_explicit_standard_library_symbols() {
    let workspace = TempWorkspace::new("definition-standard-library");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use prelude from \"std\"\n\n",
            "fn implicit() -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n\n",
            "fn explicit() -> Result<Byte, String>\n",
            "  prelude::byte(1)\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    for (case, line, column) in [("implicit", 4, 4), ("explicit", 8, 12)] {
        let result =
            server.definition_tool(&json!({"source":"main.veln","line":line,"column":column}));
        assert_eq!(result["isError"], false, "{case}: {result:#}");
        let location = &result["structuredContent"]["definition"];
        let uri = location["uri"].as_str().unwrap();
        assert!(
            uri.starts_with("veln-pkg:///std/snapshot/"),
            "{case}: {uri}"
        );
        assert!(uri.ends_with("/prelude.veln"), "{case}: {uri}");
        assert_eq!(
            location["range"],
            json!({"start":{"line":98,"column":8},"end":{"line":98,"column":12}}),
            "{case}"
        );

        let read = read_resource(&mut server, uri);
        let text = read["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("pub fn byte(value: Int)"), "{case}");
    }
}

#[test]
fn definition_dependency_package_uris_are_independent_of_source_kind() {
    for source_kind in [
        DependencySourceKind::Path,
        DependencySourceKind::Vendor,
        DependencySourceKind::Mirror,
        DependencySourceKind::Git,
    ] {
        let workspace = TempWorkspace::new(source_kind.name());
        write_workspace_with_navigation_dependency(&workspace, source_kind);
        let result = definition_result(&workspace, "main.veln", 13, 19);
        assert_eq!(
            result["isError"],
            false,
            "{}: {result:#}",
            source_kind.name()
        );
        let uri = result["structuredContent"]["definition"]["uri"]
            .as_str()
            .unwrap();
        assert!(
            uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"),
            "{}: {uri}",
            source_kind.name()
        );
        assert!(uri.ends_with("/dep.veln"), "{}: {uri}", source_kind.name());
        assert!(
            !uri.contains(source_kind.name()),
            "{}: {uri}",
            source_kind.name()
        );
    }
}

#[test]
fn definition_rejects_ineligible_package_selections_without_reinterpreting_them() {
    struct Case {
        name: &'static str,
        dependency_source: &'static str,
        exports: &'static str,
        import: &'static str,
        expression: &'static str,
        column: usize,
    }

    let cases = [
        Case {
            name: "private declaration",
            dependency_source: "fn hidden(value: Int) -> Int\n  value\nend\n",
            exports: "\"dep.veln\"",
            import: "use dep from \"example/dep\"",
            expression: "dep::hidden(1)",
            column: 9,
        },
        Case {
            name: "non-exported source",
            dependency_source: "pub fn hidden(value: Int) -> Int\n  value\nend\n",
            exports: "",
            import: "use dep from \"example/dep\"",
            expression: "dep::hidden(1)",
            column: 9,
        },
        Case {
            name: "invalid function casing",
            dependency_source: "pub fn Bad(value: Int) -> Int\n  value\nend\n",
            exports: "\"dep.veln\"",
            import: "use dep from \"example/dep\"",
            expression: "dep::Bad(1)",
            column: 9,
        },
        Case {
            name: "mismatched package import",
            dependency_source: "pub fn hidden(value: Int) -> Int\n  value\nend\n",
            exports: "\"dep.veln\"",
            import: "use dep from \"other/dep\"",
            expression: "dep::hidden(1)",
            column: 9,
        },
        Case {
            name: "module segment",
            dependency_source: "pub fn hidden(value: Int) -> Int\n  value\nend\n",
            exports: "\"dep.veln\"",
            import: "use dep from \"example/dep\"",
            expression: "dep::hidden(1)",
            column: 4,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        workspace.write(
            "veln.toml",
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
        );
        workspace.write(
            "main.veln",
            &format!(
                "{}\n\nfn main() -> Int\n  {}\nend\n",
                case.import, case.expression
            ),
        );
        workspace.write(
            "vendor/dep/veln.toml",
            &format!(
                "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [{}]\n",
                case.exports
            ),
        );
        workspace.write("vendor/dep/dep.veln", case.dependency_source);

        let result = definition_result(&workspace, "main.veln", 4, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["definition"],
            Value::Null,
            "{}",
            case.name
        );
    }
}

#[test]
fn definition_retains_package_snapshot_bytes_across_dependency_changes() {
    let workspace = TempWorkspace::new("definition-package-snapshot-lifetime");
    write_workspace_with_navigation_dependency(&workspace, DependencySourceKind::Path);
    let mut server = initialized_server(&workspace);

    let first = server.definition_tool(&json!({"source":"main.veln","line":13,"column":19}));
    let first_uri = first["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap()
        .to_string();
    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    workspace.write("vendor/dep/veln.toml", &navigation_dependency_manifest());
    workspace.write(
        "vendor/dep/dep.veln",
        &navigation_dependency_source().replace("value + 1", "value + 2"),
    );
    refresh_workspace(&mut server);

    let retained = read_resource(&mut server, &first_uri);
    assert_eq!(
        retained["result"]["contents"][0]["text"],
        navigation_dependency_source()
    );

    let second = server.definition_tool(&json!({"source":"main.veln","line":13,"column":19}));
    let second_uri = second["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert_ne!(second_uri, first_uri);
    assert_eq!(
        read_resource(&mut server, second_uri)["result"]["contents"][0]["text"],
        navigation_dependency_source().replace("value + 1", "value + 2")
    );
    assert_eq!(
        read_resource(&mut server, &first_uri)["result"]["contents"][0]["text"],
        navigation_dependency_source()
    );
}

#[test]
fn definition_exposes_unique_invalid_name_recovery_but_not_unsupported_boundaries() {
    let workspace = TempWorkspace::new("definition-invalid-name-recovery");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  byte(value: Int)\n",
            "end\n\n",
            "fn byte() -> Int\n",
            "  2\n",
            "end\n\n",
            "fn Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn Dup() -> Int\n",
            "  1\n",
            "end\n\n",
            "fn Dup() -> Int\n",
            "  2\n",
            "end\n\n",
            "fn caller() -> Int\n",
            "  Bad() + Dup() + byte()\n",
            "end\n",
        ),
    );

    let recovery = definition_result(&workspace, "main.veln", 22, 4);
    assert_eq!(recovery["isError"], false, "{recovery:#}");
    assert_eq!(
        recovery["structuredContent"]["definition"]["range"],
        json!({"start": {"line": 9, "column": 4}, "end": {"line": 9, "column": 7}})
    );

    let ambiguous = definition_result(&workspace, "main.veln", 22, 12);
    assert_eq!(ambiguous["isError"], false, "{ambiguous:#}");
    assert_eq!(ambiguous["structuredContent"]["definition"], Value::Null);

    let valid_precedence = definition_result(&workspace, "main.veln", 22, 20);
    assert_eq!(valid_precedence["isError"], false, "{valid_precedence:#}");
    assert_eq!(
        valid_precedence["structuredContent"]["definition"]["range"],
        json!({"start": {"line": 5, "column": 4}, "end": {"line": 5, "column": 8}})
    );
}

#[test]
fn definition_uses_canonical_uri_and_range() {
    let workspace = TempWorkspace::new("definition coordinates and uri");
    write_definition_coordinate_fixture(&workspace);

    let found = definition_result(&workspace, "main.veln", 6, 9);
    let uri = found["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert!(uri.starts_with("file:///"), "{uri}");
    assert!(
        uri.contains("definition%20coordinates%20and%20uri"),
        "{uri}"
    );
    assert_eq!(
        found["structuredContent"]["definition"]["range"]["start"],
        json!({"line": 1, "column": 4})
    );
    assert_eq!(
        found["structuredContent"]["definition"]["range"]["end"],
        json!({"line": 1, "column": 10})
    );
}

#[test]
fn definition_returns_null_when_no_symbol_is_selected() {
    let workspace = TempWorkspace::new("definition-no-symbol");
    write_definition_coordinate_fixture(&workspace);

    for column in [8, 15] {
        let no_symbol = definition_result(&workspace, "main.veln", 6, column);
        assert_eq!(no_symbol["isError"], false, "{column}: {no_symbol:#}");
        assert_eq!(
            no_symbol["structuredContent"]["definition"],
            Value::Null,
            "{column}"
        );
    }
}

#[test]
fn definition_rejects_invalid_positions() {
    let workspace = TempWorkspace::new("definition-invalid-position");
    write_definition_coordinate_fixture(&workspace);

    for (line, column) in [(9, 1), (1, 20)] {
        let invalid = definition_result(&workspace, "main.veln", line, column);
        assert_eq!(invalid["isError"], true, "{line}:{column} {invalid:#}");
        assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    }
    assert_invalid_definition_position(
        &workspace,
        json!({
            "source": "main.veln",
            "line": u64::MAX,
            "column": 1
        }),
    );
    let above_u64_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":18446744073709551616,"column":1}"#)
            .unwrap();
    assert_invalid_definition_position(&workspace, above_u64_arguments);

    let huge_positive_exponent_arguments =
        serde_json::from_str(r#"{"source":"main.veln","line":1e9223372036854775807,"column":1}"#)
            .unwrap();
    assert_invalid_definition_position(&workspace, huge_positive_exponent_arguments);
}

#[test]
fn definition_accepts_integral_coordinate_spelling() {
    let workspace = TempWorkspace::new("definition-integral-spelling");
    write_definition_coordinate_fixture(&workspace);

    for arguments in [
        json!({"source": "main.veln", "line": 6.0, "column": 9}),
        json!({"source": "main.veln", "line": 6, "column": 9e0}),
    ] {
        let integral_spelling = initialized_server(&workspace).definition_tool(&arguments);
        assert_eq!(integral_spelling["isError"], false, "{integral_spelling:#}");
        assert_eq!(
            integral_spelling["structuredContent"]["definition"]["range"]["start"],
            json!({"line": 1, "column": 4})
        );
    }
}

#[test]
fn definition_rejects_non_integer_coordinate_spelling() {
    let workspace = TempWorkspace::new("definition-non-integer-spelling");
    write_definition_coordinate_fixture(&workspace);

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "definition",
                "arguments": {
                    "source": "main.veln",
                    "line": 6.0000000000000001,
                    "column": 9
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");

    let huge_negative_exponent_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "definition",
                "arguments": {
                    "source": "main.veln",
                    "line": 1e-9223372036854775808,
                    "column": 9
                }
            }
        }"#,
    )
    .unwrap();
    let huge_negative_exponent = initialized_server(&workspace)
        .handle_request(huge_negative_exponent_request)
        .unwrap();
    assert_eq!(
        huge_negative_exponent["error"]["code"], -32602,
        "{huge_negative_exponent:#}"
    );
}

fn write_definition_coordinate_fixture(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn helper() -> Int\r\n  1\r\nend\r\n\r\nfn main() -> Int\r\n  \"😀\" + helper()\r\nend\r\n");
}

fn assert_invalid_definition_position(workspace: &TempWorkspace, arguments: Value) {
    let result = initialized_server(workspace).definition_tool(&arguments);
    assert_eq!(result["isError"], true, "{result:#}");
    assert_eq!(result["structuredContent"]["code"], "invalid_position");
}

#[test]
fn definition_rejects_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("definition-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = definition_result(&workspace, source, 1, 1);
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{source}"
        );
    }

    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    let mut server = Server {
        base,
        selection,
        initialized: true,
        language_resources: LanguageResources::checked().unwrap(),
    };
    let result = server.definition_tool(&json!({"source":"main.veln","line":2,"column":4}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("definition")
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn definition_rejects_symlink_paths_and_spells_uris_from_the_resolved_base() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("definition-resolved-base");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper() -> Int\n  1\nend\n\nfn main() -> Int\n  helper()\nend\n",
    );
    symlink(workspace.path("main.veln"), workspace.path("linked.veln")).unwrap();
    let rejected = definition_result(&workspace, "linked.veln", 1, 1);
    assert_eq!(rejected["structuredContent"]["code"], "invalid_path");

    let alias = workspace.root.with_file_name(format!(
        "{}-alias",
        workspace.root.file_name().unwrap().to_string_lossy()
    ));
    symlink(&workspace.root, &alias).unwrap();
    let mut server = server_from_workspace_base_alias(&alias);
    let result = server.definition_tool(&json!({"source":"main.veln","line":6,"column":4}));
    assert_definition_uri_uses_resolved_base(&result, &workspace);
    fs::remove_file(alias).unwrap();
}

#[cfg(unix)]
fn server_from_workspace_base_alias(alias: &Path) -> Server {
    let base = WorkspaceBase::open(alias.to_path_buf()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    Server {
        base,
        selection,
        initialized: true,
        language_resources: LanguageResources::checked().unwrap(),
    }
}

#[cfg(unix)]
fn assert_definition_uri_uses_resolved_base(result: &Value, workspace: &TempWorkspace) {
    let uri = result["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    let resolved_name = workspace.root.file_name().unwrap().to_string_lossy();
    assert!(uri.contains(resolved_name.as_ref()), "{uri}");
    assert!(!uri.contains("-alias/main.veln"), "{uri}");
}

#[derive(Clone, Copy)]
enum DependencySourceKind {
    Path,
    Vendor,
    Mirror,
    Git,
}

impl DependencySourceKind {
    fn name(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Vendor => "vendor",
            Self::Mirror => "mirror",
            Self::Git => "git",
        }
    }
}

fn write_workspace_with_navigation_dependency(
    workspace: &TempWorkspace,
    source_kind: DependencySourceKind,
) {
    workspace.write("veln.toml", &navigation_dependency_table(source_kind));
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn type_use(value: dep::Token) -> dep::Token\n",
            "  value\n",
            "end\n\n",
            "fn alias_use(value: dep::item_alias) -> dep::item_alias\n",
            "  value\n",
            "end\n\n",
            "fn main() -> dep::Token\n",
            "  let encoded = encode dep::packet from {value: 1}\n",
            "  dep::Value(dep::increment(1) + dep::add_one(1))\n",
            "end\n",
        ),
    );
    let dependency_root = navigation_dependency_root(workspace, source_kind);
    workspace.write(
        &format!("{dependency_root}/veln.toml"),
        &navigation_dependency_manifest(),
    );
    workspace.write(
        &format!("{dependency_root}/dep.veln"),
        &navigation_dependency_source(),
    );
}

fn navigation_dependency_table(source_kind: DependencySourceKind) -> String {
    match source_kind {
        DependencySourceKind::Path => {
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n".to_string()
        }
        DependencySourceKind::Vendor => {
            "[dependencies.\"example/dep\"]\nvendor = \"vendor/dep\"\n".to_string()
        }
        DependencySourceKind::Mirror => {
            "[dependencies.\"example/dep\"]\nmirror = \"mirror/example/dep\"\n".to_string()
        }
        DependencySourceKind::Git => concat!(
            "[dependencies.\"example/dep\"]\n",
            "git = \"https://example.invalid/dep.git\"\n",
            "rev = \"abc123\"\n",
        )
        .to_string(),
    }
}

fn navigation_dependency_root(
    workspace: &TempWorkspace,
    source_kind: DependencySourceKind,
) -> String {
    match source_kind {
        DependencySourceKind::Path | DependencySourceKind::Vendor => "vendor/dep".to_string(),
        DependencySourceKind::Mirror => "mirror/example/dep".to_string(),
        DependencySourceKind::Git => veln_project::materialized_git_repository_root(
            &workspace.root,
            "https://example.invalid/dep.git",
        )
        .strip_prefix(&workspace.root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/"),
    }
}

fn navigation_dependency_manifest() -> String {
    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n".to_string()
}

fn unicode_dependency_manifest() -> String {
    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"unicode.veln\"]\n".to_string()
}

fn unicode_dependency_source() -> String {
    "pub fn label() -> String\r\n  \"café\"\r\nend\r\n".to_string()
}

fn navigation_dependency_source() -> String {
    concat!(
        "pub type Token\n",
        "  pub Value(Int)\n",
        "end\n\n",
        "pub type item_alias = Token\n\n",
        "pub schema packet\n",
        "  value: Int\n",
        "end\n\n",
        "pub fn increment(value: Int) -> Int\n",
        "  value + 1\n",
        "end\n\n",
        "pub fn add_one = increment\n",
    )
    .to_string()
}

fn read_resource(server: &mut Server, uri: &str) -> Value {
    server
        .handle_request(
            json!({"jsonrpc":"2.0","id":"read-definition-resource","method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap()
}

fn refresh_workspace(server: &mut Server) {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"refresh-definition-workspace","method":"tools/call","params":{"name":"refresh_workspace","arguments":{}}}))
        .unwrap();
}

fn definition_result(workspace: &TempWorkspace, source: &str, line: usize, column: usize) -> Value {
    initialized_server(workspace)
        .definition_tool(&json!({"source": source, "line": line, "column": column}))
}
