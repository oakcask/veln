use super::references::{assert_reference_ranges, references_result};
use super::*;

#[test]
fn references_return_package_function_alias_locations() {
    direct_dependency_function_alias_reference_case();
    standard_library_function_alias_reference_case();
}

fn direct_dependency_function_alias_reference_case() {
    let alias_workspace = TempWorkspace::new("references-dependency-alias-boundary");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main() -> Int\n",
            "  dep::renamed()\n",
            "  dep::target()\n",
            "end\n\n",
            "pub fn callback() -> fn() -> Int\n",
            "  dep::renamed\n",
            "end\n",
            "\n",
            "pub fn type_collision(value: dep::renamed) -> Int\n",
            "  dep::renamed()\n",
            "end\n",
            "\n",
            "pub fn nested_type_collision(value: List<dep::renamed>) -> Int\n",
            "  dep::renamed()\n",
            "end\n",
            "\n# renamed mention\n",
            "pub fn renamed(record: {renamed: Int}) -> Int\n",
            "  let renamed = record.renamed\n",
            "  renamed\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "other.veln",
        "use dep from \"example/dep\"\n\npub fn other() -> Int\n  dep::renamed()\nend\n",
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub fn target() -> Int\n",
            "  1\n",
            "end\n\n",
            "pub fn renamed = target\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    let alias_definition =
        alias_server.definition_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_definition["isError"], false, "{alias_definition:#}");
    let alias_uri = alias_definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap();
    assert!(alias_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"));
    assert!(alias_uri.ends_with("/dep.veln"));
    assert_eq!(
        alias_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":5,"column":8},"end":{"line":5,"column":15}})
    );
    let alias_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_references["isError"], false, "{alias_references:#}");
    assert_reference_ranges(
        &alias_references,
        &[
            ("main.veln", 4, 8, 4, 15),
            ("main.veln", 9, 8, 9, 15),
            ("main.veln", 13, 8, 13, 15),
            ("main.veln", 17, 8, 17, 15),
            ("other.veln", 4, 8, 4, 15),
        ],
        "direct dependency function alias",
    );

    let target_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":5,"column":8}));
    assert_eq!(target_references["isError"], false, "{target_references:#}");
    assert_reference_ranges(
        &target_references,
        &[("main.veln", 5, 8, 5, 14)],
        "direct dependency alias target separation",
    );
}

fn standard_library_function_alias_reference_case() {
    let std_workspace = TempWorkspace::new("references-standard-library-alias-boundary");
    std_workspace.write("veln.toml", "");
    std_workspace.write(
        "main.veln",
        concat!(
            "use prelude from \"std\"\n\n",
            "pub fn first(chunk: ByteChunk) -> ByteCount\n",
            "  byte_chunk_len(chunk)\n",
            "end\n\n",
            "pub fn second() -> fn(ByteChunk) -> ByteCount\n",
            "  prelude::byte_chunk_len\n",
            "end\n\n",
            "pub fn target(chunk: ByteChunk) -> ByteCount\n",
            "  byte_chunk_count(chunk)\n",
            "end\n",
            "\n",
            "pub fn type_collision(chunk: prelude::byte_chunk_len) -> ByteCount\n",
            "  prelude::byte_chunk_len(chunk)\n",
            "end\n",
            "\n",
            "pub fn nested_type_collision(chunk: List<prelude::byte_chunk_len>) -> ByteCount\n",
            "  prelude::byte_chunk_len(chunk)\n",
            "end\n",
        ),
    );
    std_workspace.write(
        "other.veln",
        concat!(
            "use prelude from \"std\"\n\n",
            "pub fn other(chunk: ByteChunk) -> ByteCount\n",
            "  prelude::byte_chunk_len(chunk)\n",
            "end\n",
        ),
    );
    let mut std_server = initialized_server_with_embedded_resources(&std_workspace);

    let std_alias_references =
        std_server.references_tool(&json!({"source":"main.veln","line":4,"column":3}));
    assert_eq!(
        std_alias_references["isError"], false,
        "{std_alias_references:#}"
    );
    assert_reference_ranges(
        &std_alias_references,
        &[
            ("main.veln", 4, 3, 4, 17),
            ("main.veln", 8, 12, 8, 26),
            ("main.veln", 16, 12, 16, 26),
            ("main.veln", 20, 12, 20, 26),
            ("other.veln", 4, 12, 4, 26),
        ],
        "standard library function alias",
    );

    let std_target_references =
        std_server.references_tool(&json!({"source":"main.veln","line":12,"column":3}));
    assert_eq!(
        std_target_references["isError"], false,
        "{std_target_references:#}"
    );
    assert_reference_ranges(
        &std_target_references,
        &[("main.veln", 12, 3, 12, 19)],
        "standard library alias target separation",
    );

    let std_type_collision =
        std_server.references_tool(&json!({"source":"main.veln","line":15,"column":41}));
    assert_eq!(
        std_type_collision["structuredContent"]["references"],
        json!([]),
        "{std_type_collision:#}"
    );

    let std_nested_type_collision =
        std_server.references_tool(&json!({"source":"main.veln","line":19,"column":50}));
    assert_eq!(
        std_nested_type_collision["structuredContent"]["references"],
        json!([]),
        "{std_nested_type_collision:#}"
    );
}

#[test]
fn references_keep_dependency_function_alias_boundary_cases() {
    let workspace = TempWorkspace::new("references-dependency-alias-exclusions");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n",
            "use other from \"other/dep\"\n\n",
            "fn renamed() -> Int\n",
            "  0\n",
            "end\n\n",
            "fn main(record: {renamed: Int}) -> Int\n",
            "  dep::renamed()\n",
            "  other::renamed()\n",
            "  renamed()\n",
            "  record.renamed\n",
            "end\n",
        ),
    );
    workspace.write(
        "helper.veln",
        "use dep from \"example/dep\"\n\nfn helper() -> Int\n  dep::renamed()\nend\n",
    );
    workspace.write(
        "nested/veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"../vendor/dep\"\n",
    );
    workspace.write(
        "nested/main.veln",
        "use dep from \"example/dep\"\n\nfn nested() -> Int\n  dep::renamed()\nend\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub fn target() -> Int\n",
            "  1\n",
            "end\n\n",
            "pub fn renamed = target\n\n",
            "pub fn package_body() -> Int\n",
            "  renamed()\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"other.veln\"]\n",
    );
    workspace.write(
        "vendor/other/other.veln",
        "pub fn target() -> Int\n  2\nend\n\npub fn renamed = target\n",
    );

    let result = references_result(&workspace, "main.veln", 9, 8);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("helper.veln", 4, 8, 4, 15), ("main.veln", 9, 8, 9, 15)],
        "dependency function alias exclusions",
    );
}

#[test]
fn references_return_empty_for_hidden_dependency_function_aliases() {
    struct Case {
        name: &'static str,
        alias_source: &'static str,
        alias_declaration: &'static str,
        exports: &'static str,
        main: &'static str,
        line: usize,
        column: usize,
    }

    let cases = [
        Case {
            name: "private dependency function alias",
            alias_source: "dep.veln",
            alias_declaration: "fn hidden = target\n",
            exports: "\"dep.veln\"",
            main: "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::hidden()\nend\n",
            line: 4,
            column: 8,
        },
        Case {
            name: "non-exported dependency function alias",
            alias_source: "hidden.veln",
            alias_declaration: "pub fn hidden = target\n",
            exports: "\"public.veln\"",
            main: "use hidden from \"example/dep\"\n\nfn main() -> Int\n  hidden::hidden()\nend\n",
            line: 4,
            column: 11,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        workspace.write(
            "veln.toml",
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
        );
        workspace.write("main.veln", case.main);
        workspace.write(
            "vendor/dep/veln.toml",
            &format!(
                "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [{}]\n",
                case.exports
            ),
        );
        workspace.write("vendor/dep/public.veln", "pub fn ok() -> Int\n  1\nend\n");
        workspace.write(
            &format!("vendor/dep/{}", case.alias_source),
            &format!(
                "{}{}",
                concat!("pub fn target() -> Int\n", "  1\n", "end\n\n",),
                case.alias_declaration
            ),
        );

        let result = references_result(&workspace, "main.veln", case.line, case.column);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}
