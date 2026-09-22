use super::*;

struct UnsupportedReferenceCase {
    name: &'static str,
    files: Vec<(&'static str, &'static str)>,
    source: &'static str,
    line: usize,
    column: usize,
}

type Case = UnsupportedReferenceCase;

fn assert_references_rejected(cases: impl IntoIterator<Item = UnsupportedReferenceCase>) {
    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
        let with_declaration = initialized_server(&workspace).references_tool(&json!({
            "source": case.source,
            "line": case.line,
            "column": case.column,
            "include_declaration": true
        }));
        assert_eq!(
            with_declaration["isError"], false,
            "{}: {with_declaration:#}",
            case.name
        );
        assert_eq!(
            with_declaration["structuredContent"]["references"],
            json!([]),
            "{}: {with_declaration:#}",
            case.name
        );
    }
}

#[test]
fn references_reject_recovery_package_and_unsupported_symbols() {
    let cases = [
        UnsupportedReferenceCase {
            name: "recovery value binding",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "fn main(Bad: Int) -> Int\n  Bad\nend\n"),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
        },
        UnsupportedReferenceCase {
            name: "package private type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use example::dep\n\nfn read(input: dep::Item) -> dep::Item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "type Item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "package private type alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(input: dep::Alias) -> dep::Alias\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub type Item\nend\n\ntype Alias = Item\n",
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "package invalid-casing type alias declaration",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(input: dep::alias) -> dep::alias\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub type Item\nend\n\npub type alias = Item\n",
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "package non-exported type alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use hidden from \"example/dep\"\n\nfn read(input: hidden::Alias) -> hidden::Alias\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type Item\nend\n"),
                (
                    "vendor/dep/hidden.veln",
                    "pub type Item\nend\n\npub type Alias = Item\n",
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 29,
        },
        Case {
            name: "package non-exported type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use hidden from \"example/dep\"\n\nfn read(input: hidden::Item) -> hidden::Item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"public.veln\"]\n",
                ),
                ("vendor/dep/public.veln", "pub fn ok() -> Int\n  1\nend\n"),
                ("vendor/dep/hidden.veln", "pub type Item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 28,
        },
        Case {
            name: "package invalid-casing type",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(input: dep::item) -> dep::item\n  input\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type item\nend\n"),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
        },
        Case {
            name: "workspace schema alias with private target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "pub schema AliasPacket = Packet\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 12,
        },
        Case {
            name: "workspace schema alias-chain composition target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "pub schema FirstAlias = Packet\n",
                        "pub schema AliasPacket = FirstAlias\n\n",
                        "schema Frame\n",
                        "  format binary\n",
                        "  nested: AliasPacket\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 11,
            column: 11,
        },
        Case {
            name: "workspace schema decode qualifier",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "fn imported(view: ByteView) -> ()\n",
                        "  decode main::Packet from view at byte_offset(0)?\n",
                        "end\n",
                    ),
                ),
            ],
            source: "other.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "workspace schema encode qualifier",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n",
                    ),
                ),
                (
                    "other.veln",
                    concat!(
                        "use main\n\n",
                        "fn imported(packet: {value: Int}) -> ()\n",
                        "  encode main::Packet from packet\n",
                        "end\n",
                    ),
                ),
            ],
            source: "other.veln",
            line: 4,
            column: 12,
        },
        Case {
            name: "private package constructor",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn make() -> dep::Item\n  dep::Item::Ready(1)\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub type Item\n  Ready(Int)\nend\n"),
            ],
            source: "main.veln",
            line: 4,
            column: 15,
        },
        Case {
            name: "effect operation",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task]\n  perform Task::run()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 6,
            column: 17,
        },
        Case {
            name: "generic effect row parameter",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect E\n  run() -> Int\nend\n\nfn main<effect E>() -> Int effects [...E]\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 40,
        },
        Case {
            name: "qualified workspace effect",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [foreign::Task]\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 36,
        },
        Case {
            name: "syntax recovered effect row",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 29,
        },
        Case {
            name: "ambiguous same module effect",
            files: vec![
                ("veln.toml", ""),
                (
                    "first.veln",
                    "mod shared\n\neffect Task\n  first() -> Int\nend\n",
                ),
                (
                    "second.veln",
                    "mod shared\n\neffect Task\n  second() -> Int\nend\n\nfn main() -> Int effects [Task]\n  1\nend\n",
                ),
            ],
            source: "second.veln",
            line: 7,
            column: 27,
        },
        Case {
            name: "direct dependency effect",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn main() -> Int effects [dep::Task]\n  1\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub effect Task\n  run() -> Int\nend\n",
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 32,
        },
        Case {
            name: "handler",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nhandler task() handles Task\n  run() => 1\nend\n\nfn main() -> Int effects [Task]\n  handle perform Task::run() with task()\nend\n",
                ),
            ],
            source: "main.veln",
            line: 10,
            column: 34,
        },
        Case {
            name: "casing neutral type selection",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "type item\n  Value\nend\n\nfn read(input: item) -> item\n  input\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 17,
        },
        Case {
            name: "no symbol",
            files: vec![("main.veln", "fn main() -> Int\n  1\nend\n")],
            source: "main.veln",
            line: 2,
            column: 3,
        },
    ];

    assert_references_rejected(cases);
}

#[test]
fn references_preserve_unicode_coordinates_and_token_end_exclusion() {
    let workspace = TempWorkspace::new("references-coordinate-boundaries");
    workspace.write(
        "main.veln",
        "fn main() -> Int\r\n  let emoji = \"🙂\"\r\n  main()\r\nend\r\n",
    );

    let selected = references_result(&workspace, "main.veln", 3, 4);
    assert_eq!(selected["isError"], false, "{selected:#}");
    assert_reference_ranges(
        &selected,
        &[("main.veln", 3, 3, 3, 7)],
        "unicode coordinate selection",
    );

    let token_end = references_result(&workspace, "main.veln", 3, 7);
    assert_eq!(token_end["isError"], false, "{token_end:#}");
    assert_eq!(token_end["structuredContent"]["references"], json!([]));

    let invalid = references_result(&workspace, "main.veln", 2, 21);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
}

#[test]
fn references_use_single_file_scope_for_sources_outside_selected_projects() {
    let workspace = TempWorkspace::new("references-single-file");
    workspace.write("app/veln.toml", "");
    workspace.write("app/main.veln", "fn selected() -> Int\n  selected()\nend\n");
    workspace.write("loose.veln", "fn helper() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  helper()\nend\n");

    let result = references_result(&workspace, "loose.veln", 2, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 1, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("loose.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 9}})
    );
}

#[test]
fn references_do_not_expose_function_shaped_recovery_records() {
    let workspace = TempWorkspace::new("references-recovery-boundary");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "test Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 6, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(result["structuredContent"]["scope"]["project_wide"], true);
}

#[test]
fn references_report_invalid_positions_and_schema_coordinate_failures() {
    let workspace = TempWorkspace::new("references-invalid-position");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");

    let invalid = references_result(&workspace, "main.veln", 5, 1);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    assert!(
        invalid["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "references",
                "arguments": {
                    "source": "main.veln",
                    "line": 2.0000000000000001,
                    "column": 4
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");
    assert!(non_integer.get("result").is_none(), "{non_integer:#}");
}

#[test]
fn references_reject_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("references-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = references_result(&workspace, source, 1, 1);
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
        language_resources: minimal_language_resources(),
        capture_cache: CaptureCache::default(),
    };
    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );
}
