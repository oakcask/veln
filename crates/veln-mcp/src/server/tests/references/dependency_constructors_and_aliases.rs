use super::*;
use veln_project::PackageSnapshotSource;

#[test]
fn references_support_type_alias_target_in_non_exported_dependency_module() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-hidden-target");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use facade from \"example/dep\"\n\n",
            "pub type Local = facade::PublicHidden\n\n",
            "fn read(input: facade::PublicHidden) -> Vec<facade::PublicHidden>\n",
            "  facade::PublicHidden::Ready(1)\n",
            "end\n",
            "\n",
            "fn bare(input: PublicHidden) -> PublicHidden\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use facade from \"example/dep\"\n\n",
            "fn other(input: facade::PublicHidden) -> facade::PublicHidden\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"facade.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/facade.veln",
        "use internal::core\n\npub type PublicHidden = core::Hidden\n",
    );
    workspace.write(
        "vendor/dep/internal/core.veln",
        "type Hidden\n  pub Ready(Int)\nend\n",
    );

    let result = references_result(&workspace, "main.veln", 5, 26);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 3, 26, 3, 38),
            ("main.veln", 5, 24, 5, 36),
            ("main.veln", 5, 53, 5, 65),
            ("main.veln", 6, 11, 6, 23),
            ("other.veln", 3, 25, 3, 37),
            ("other.veln", 3, 50, 3, 62),
        ],
        "hidden type target alias references",
    );

    let bare = references_result(&workspace, "main.veln", 9, 17);
    assert_eq!(bare["isError"], false, "{bare:#}");
    assert_eq!(bare["structuredContent"]["references"], json!([]));
}

#[test]
fn references_return_direct_dependency_constructor_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-constructor");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example/dep\"]\n",
            "path = \"vendor/dep\"\n",
            "\n",
            "[dependencies.\"other/dep\"]\n",
            "path = \"vendor/other\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n",
            "use other_model from \"other/dep\"\n\n",
            "type Local\n",
            "  Ready(Int)\n",
            "end\n\n",
            "fn make(input: Int) -> model::Item\n",
            "  model::Item::Ready(input)\n",
            "end\n\n",
            "fn alias_route(input: Int) -> model::Alias\n",
            "  model::Alias::Ready(input)\n",
            "end\n\n",
            "fn collisions(record: {Ready: Int}, Ready: Int) -> Int\n",
            "  other_model::Ready(1)\n",
            "  Local::Ready(2)\n",
            "  record.Ready + Ready\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn other(input: model::Item) -> Int\n",
            "  match input\n",
            "    Ready(value) => value\n",
            "  end\n",
            "end\n\n",
            "fn qualified() -> model::Item\n",
            "  model::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n\nfn package_body() -> Item\n  Ready(1)\nend\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/other/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n",
    );

    let result = references_result(&workspace, "main.veln", 9, 17);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 9, 16, 9, 21),
            ("other.veln", 5, 5, 5, 10),
            ("other.veln", 10, 10, 10, 15),
        ],
        "dependency constructor references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));
}

#[test]
fn references_keep_ambiguous_package_constructor_leaf_empty() {
    let workspace = TempWorkspace::new("references-ambiguous-package-constructor");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn ambiguous() -> model::Left\n",
            "  model::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        concat!(
            "pub type Left\n",
            "  pub Ready(Int)\n",
            "end\n\n",
            "pub type Right\n",
            "  pub Ready(Int)\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 4, 10);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["references"],
        json!([]),
        "{result:#}"
    );
}

#[test]
fn references_keep_package_constructor_alias_boundary_empty() {
    let dependency_workspace = TempWorkspace::new("references-dependency-constructor-alias");
    dependency_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    dependency_workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn make() -> model::Alias\n",
            "  model::Alias::Ready(1)\n",
            "end\n",
        ),
    );
    dependency_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    dependency_workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\npub type Alias = Item\n",
    );
    let mut dependency_server = initialized_server(&dependency_workspace);

    let dependency_definition =
        dependency_server.definition_tool(&json!({"source":"main.veln","line":4,"column":17}));
    assert_eq!(
        dependency_definition["isError"], false,
        "{dependency_definition:#}"
    );
    assert_eq!(
        dependency_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}})
    );
    let dependency_references =
        dependency_server.references_tool(&json!({"source":"main.veln","line":4,"column":17}));
    assert_eq!(
        dependency_references["isError"], false,
        "{dependency_references:#}"
    );
    assert_eq!(
        dependency_references["structuredContent"]["references"],
        json!([]),
        "{dependency_references:#}"
    );

    let standard_workspace = TempWorkspace::new("references-standard-constructor-alias");
    standard_workspace.write(
        "main.veln",
        "fn make() -> prelude::Alias\n  prelude::Alias::Some(1)\nend\n",
    );
    let mut standard_server = initialized_server(&standard_workspace);
    standard_server
        .language_resources
        .replace_test_standard_library(
            "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
            [PackageSnapshotSource::new(
                "prelude.veln",
                b"pub type Option\n  pub Some(Int)\nend\n\npub type Alias = Option\n",
            )],
        );

    let standard_definition =
        standard_server.definition_tool(&json!({"source":"main.veln","line":2,"column":19}));
    assert_eq!(
        standard_definition["isError"], false,
        "{standard_definition:#}"
    );
    assert_eq!(
        standard_definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":11}})
    );
    let standard_references =
        standard_server.references_tool(&json!({"source":"main.veln","line":2,"column":19}));
    assert_eq!(
        standard_references["isError"], false,
        "{standard_references:#}"
    );
    assert_eq!(
        standard_references["structuredContent"]["references"],
        json!([]),
        "{standard_references:#}"
    );
}

#[test]
fn references_return_direct_dependency_function_alias_locations_from_saved_project() {
    let alias_workspace = dependency_function_alias_workspace();
    let mut alias_server = initialized_server(&alias_workspace);
    crate::language_resources::reset_dependency_snapshot_captures();

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
            ("main.veln", 8, 36, 8, 43),
            ("main.veln", 9, 21, 9, 28),
            ("other.veln", 4, 8, 4, 15),
        ],
        "dependency function alias references",
    );
    let references = alias_references["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));

    let alias_with_declaration = alias_server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 8,
        "include_declaration": true
    }));
    assert_eq!(
        alias_with_declaration["isError"], false,
        "{alias_with_declaration:#}"
    );
    let declaration = alias_with_declaration["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .find(|location| {
            location["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///")
        })
        .expect("eligible package alias declaration");
    let declaration_uri = declaration["uri"].as_str().unwrap();
    let read = alias_server
        .handle_request(json!({
            "jsonrpc": "2.0",
            "id": "package-declaration",
            "method": "resources/read",
            "params": {"uri": declaration_uri}
        }))
        .unwrap();
    assert_eq!(read["result"]["contents"][0]["uri"], declaration_uri);
    assert_eq!(
        read["result"]["contents"][0]["text"],
        "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n"
    );

    let target_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":9,"column":38}));
    assert_eq!(target_references["isError"], false, "{target_references:#}");
    assert_reference_ranges(
        &target_references,
        &[("main.veln", 9, 38, 9, 44)],
        "dependency function target references",
    );
    assert_eq!(crate::language_resources::dependency_snapshot_captures(), 1);
}

fn dependency_function_alias_workspace() -> TempWorkspace {
    let alias_workspace = TempWorkspace::new("references-dependency-alias");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn first() -> Int\n",
            "  dep::renamed()\n",
            "end\n\n",
            "pub fn second() -> Int\n",
            "  let callback: fn() -> Int = dep::renamed\n",
            "  callback() + dep::renamed() + dep::target()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "other.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn other() -> Int\n",
            "  dep::renamed()\n",
            "end\n",
        ),
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
    alias_workspace
}

#[test]
fn references_keep_direct_dependency_function_alias_chains_empty() {
    let alias_workspace = TempWorkspace::new("references-dependency-alias-chain");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main() -> Int\n",
            "  dep::chained()\n",
            "end\n",
        ),
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
            "pub fn chained = renamed\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    let alias_references =
        alias_server.references_tool(&json!({"source":"main.veln","line":4,"column":8}));
    assert_eq!(alias_references["isError"], false, "{alias_references:#}");
    assert_eq!(
        alias_references["structuredContent"]["references"],
        json!([]),
        "{alias_references:#}"
    );
}

#[test]
fn references_keep_invalid_direct_dependency_function_alias_targets_empty() {
    let alias_workspace = TempWorkspace::new("references-dependency-invalid-alias-targets");
    alias_workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    alias_workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "pub fn main() -> Int\n",
            "  dep::missing()\n",
            "  dep::wrong_kind()\n",
            "  dep::invalid_case()\n",
            "end\n",
        ),
    );
    alias_workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    alias_workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub type Document\n",
            "  pub Text(String)\n",
            "end\n\n",
            "pub fn missing = missing\n",
            "pub fn wrong_kind = Document\n",
            "pub fn invalid_case = Missing\n",
        ),
    );
    let mut alias_server = initialized_server(&alias_workspace);

    for (case, line) in [
        ("unresolved target", 4),
        ("wrong-kind target", 5),
        ("invalid-casing target", 6),
    ] {
        let alias_references = alias_server.references_tool(
            &json!({"source":"main.veln","line":line,"column":8,"include_declaration":true}),
        );
        assert_eq!(
            alias_references["isError"], false,
            "{case}: {alias_references:#}"
        );
        assert_eq!(
            alias_references["structuredContent"]["references"],
            json!([]),
            "{case}: {alias_references:#}"
        );
    }
}

#[test]
fn references_keep_direct_dependency_function_aliases_inside_selected_project() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        source: &'static str,
        scope: Value,
    }

    let source = concat!(
        "use dep from \"example/dep\"\n\n",
        "fn main() -> Int\n",
        "  dep::renamed()\n",
        "end\n",
    );
    let dependency_manifest =
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n";
    let dependency_source = "pub fn target() -> Int\n  1\nend\n\npub fn renamed = target\n";

    for case in [
        Case {
            name: "anonymous source",
            files: vec![("loose.veln", source)],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "descendant project source",
            files: vec![
                ("veln.toml", ""),
                ("main.veln", source),
                ("nested/veln.toml", ""),
                ("nested/main.veln", source),
                ("vendor/dep/veln.toml", dependency_manifest),
                ("vendor/dep/dep.veln", dependency_source),
            ],
            source: "nested/main.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "nested/main.veln",
                "project_wide": false
            }),
        },
        Case {
            name: "outside selected project",
            files: vec![
                ("app/veln.toml", ""),
                ("app/main.veln", source),
                ("loose.veln", source),
                ("app/vendor/dep/veln.toml", dependency_manifest),
                ("app/vendor/dep/dep.veln", dependency_source),
            ],
            source: "loose.veln",
            scope: json!({
                "mode": "single_file",
                "generation": 0,
                "project": ".",
                "source": "loose.veln",
                "project_wide": false
            }),
        },
    ] {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }

        let result = references_result(&workspace, case.source, 4, 8);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"], case.scope,
            "{}: {result:#}",
            case.name
        );
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}
