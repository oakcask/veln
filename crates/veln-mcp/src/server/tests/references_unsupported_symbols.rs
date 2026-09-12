use super::references::*;
use super::*;

struct Case {
    name: &'static str,
    files: Vec<(&'static str, &'static str)>,
    source: &'static str,
    line: usize,
    column: usize,
    scope: Option<Value>,
}

#[test]
fn references_reject_recovery_package_and_unsupported_symbols() {
    let cases = [
        Case {
            name: "recovery value binding",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", "fn main(Bad: Int) -> Int\n  Bad\nend\n"),
            ],
            source: "main.veln",
            line: 2,
            column: 4,
            scope: None,
        },
        Case {
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
            scope: None,
        },
        Case {
            name: "package type alias",
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
                    "pub type Item\nend\n\npub type Alias = Item\n",
                ),
            ],
            source: "main.veln",
            line: 3,
            column: 25,
            scope: None,
        },
        Case {
            name: "workspace public schema alias",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
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
            scope: None,
        },
        Case {
            name: "workspace schema composition target",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    concat!(
                        "pub schema Packet\n",
                        "  format binary\n",
                        "  value: UInt8\n",
                        "end\n\n",
                        "schema Frame\n",
                        "  format binary\n",
                        "  nested: Packet\n",
                        "end\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 8,
            column: 11,
            scope: None,
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
            scope: None,
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
            scope: None,
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
            scope: None,
        },
        Case {
            name: "package schema",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode dep::Packet from view at byte_offset(0)?\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub schema Packet\n  value: Int\nend\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 15,
            scope: None,
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
            scope: None,
        },
        Case {
            name: "effect",
            files: vec![
                ("veln.toml", ""),
                (
                    "main.veln",
                    "effect Task\n  run() -> Int\nend\n\nfn main() -> Int effects [Task]\n  1\nend\n",
                ),
            ],
            source: "main.veln",
            line: 5,
            column: 32,
            scope: None,
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
            scope: None,
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
            scope: None,
        },
        Case {
            name: "no symbol",
            files: vec![("main.veln", "fn main() -> Int\n  1\nend\n")],
            source: "main.veln",
            line: 2,
            column: 3,
            scope: None,
        },
    ];

    assert_empty_reference_cases(cases);
}

#[test]
fn references_reject_package_function_alias_unsupported_symbols() {
    let cases = [
        Case {
            name: "package function alias chain",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read() -> Int\n  dep::chain()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn renamed = target\n",
                        "pub fn chain = renamed\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 8,
            scope: None,
        },
        Case {
            name: "package function alias chain through non-exported module",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use math from \"example/dep\"\n\nfn read() -> Int\n  math::chain()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"math.veln\"]\n",
                ),
                (
                    "vendor/dep/math.veln",
                    "use internal\n\npub fn chain = internal::renamed\n",
                ),
                (
                    "vendor/dep/internal.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 9,
            scope: None,
        },
        Case {
            name: "package function alias chain through invalid-casing alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read() -> Int\n  dep::chain()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    concat!(
                        "pub fn target() -> Int\n",
                        "  1\n",
                        "end\n\n",
                        "pub fn Bad = target\n",
                        "pub fn chain = Bad\n",
                    ),
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 8,
            scope: None,
        },
        Case {
            name: "package function alias chain through invalid-casing non-exported alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use math from \"example/dep\"\n\nfn read() -> Int\n  math::chain()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"math.veln\"]\n",
                ),
                (
                    "vendor/dep/math.veln",
                    "use internal\n\npub fn chain = internal::Bad\n",
                ),
                (
                    "vendor/dep/internal.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn Bad = target\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 9,
            scope: None,
        },
        Case {
            name: "private package function alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read() -> Int\n  dep::renamed()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub fn target() -> Int\n  1\nend\n\nfn renamed = target\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 8,
            scope: None,
        },
        Case {
            name: "non-exported package function alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use hidden from \"example/dep\"\n\nfn read() -> Int\n  hidden::renamed()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                ("vendor/dep/dep.veln", "pub fn ok() -> Int\n  1\nend\n"),
                (
                    "vendor/dep/hidden.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 11,
            scope: None,
        },
        Case {
            name: "anonymous source direct-dependency function alias",
            files: vec![
                (
                    "app/veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"../vendor/dep\"\n",
                ),
                (
                    "app/main.veln",
                    "use dep from \"example/dep\"\n\nfn selected() -> Int\n  dep::renamed()\nend\n",
                ),
                (
                    "loose.veln",
                    "use dep from \"example/dep\"\n\nfn outside() -> Int\n  dep::renamed()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n",
                ),
            ],
            source: "loose.veln",
            line: 4,
            column: 8,
            scope: Some(json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            })),
        },
        Case {
            name: "descendant package source direct-dependency function alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn selected() -> Int\n  dep::renamed()\nend\n",
                ),
                (
                    "nested/veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"../vendor/dep\"\n",
                ),
                (
                    "nested/main.veln",
                    "use dep from \"example/dep\"\n\nfn outside() -> Int\n  dep::renamed()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n",
                ),
            ],
            source: "nested/main.veln",
            line: 4,
            column: 8,
            scope: Some(json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "nested/main.veln",
                "project_wide": false
            })),
        },
        Case {
            name: "package schema alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read(view: ByteView) -> ()\n  decode dep::Alias from view at byte_offset(0)?\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub schema Packet\n  value: Int\nend\n\npub schema Alias = Packet\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 15,
            scope: None,
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
            scope: None,
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
            scope: None,
        },
        Case {
            name: "package invalid-casing function alias",
            files: vec![
                (
                    "veln.toml",
                    "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
                ),
                (
                    "main.veln",
                    "use dep from \"example/dep\"\n\nfn read() -> Int\n  dep::Renamed()\nend\n",
                ),
                (
                    "vendor/dep/veln.toml",
                    "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
                ),
                (
                    "vendor/dep/dep.veln",
                    "pub fn target() -> Int\n  1\nend\n\npub fn Renamed = target\n",
                ),
            ],
            source: "main.veln",
            line: 4,
            column: 8,
            scope: None,
        },
    ];

    assert_empty_reference_cases(cases);
}

fn assert_empty_reference_cases(cases: impl IntoIterator<Item = Case>) {
    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        if let Some(scope) = case.scope {
            assert_eq!(
                result["structuredContent"]["scope"], scope,
                "{}: {result:#}",
                case.name
            );
        }
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}
