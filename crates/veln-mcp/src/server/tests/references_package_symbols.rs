use super::references::*;
use super::*;
use veln_project::PackageSnapshotSource;

#[test]
fn references_return_direct_dependency_function_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-function");
    write_dependency_reference_workspace(&workspace, "path", "vendor/dep", None);

    let result = references_result(&workspace, "main.veln", 4, 10);

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
            ("main.veln", 4, 9, 4, 17),
            ("main.veln", 8, 40, 8, 48),
            ("main.veln", 9, 18, 9, 26),
        ],
        "dependency function references",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
    }));
}

#[test]
fn references_accept_direct_dependency_function_source_forms() {
    struct Case {
        name: &'static str,
        field: &'static str,
        source: &'static str,
        selector: Option<&'static str>,
    }

    let cases = [
        Case {
            name: "path",
            field: "path",
            source: "vendor/path-dep",
            selector: None,
        },
        Case {
            name: "vendor",
            field: "vendor",
            source: "vendor/vendor-dep",
            selector: None,
        },
        Case {
            name: "mirror",
            field: "mirror",
            source: "vendor/mirror-dep",
            selector: None,
        },
        Case {
            name: "local git",
            field: "git",
            source: "vendor/git-dep",
            selector: Some("rev = \"abc123\""),
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_dependency_reference_workspace(&workspace, case.field, case.source, case.selector);

        let result = references_result(&workspace, "main.veln", 4, 10);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_reference_ranges(
            &result,
            &[
                ("main.veln", 4, 9, 4, 17),
                ("main.veln", 8, 40, 8, 48),
                ("main.veln", 9, 18, 9, 26),
            ],
            case.name,
        );
    }
}

#[test]
fn references_keep_direct_dependency_function_identity_boundaries() {
    let workspace = TempWorkspace::new("references-dependency-function-boundaries");
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
            "use lib::math from \"example/dep\"\n",
            "use other_math from \"other/dep\"\n\n",
            "pub fn read(record: {field: Int}, value: Int) -> Int\n",
            "  let local = value\n",
            "  math::target(value)\n",
            "  other_math::target(value)\n",
            "  record.field + local\n",
            "end\n",
        ),
    );
    workspace.write(
        "other/main.veln",
        concat!(
            "use lib::math from \"example/dep\"\n\n",
            "pub fn read(value: Int) -> Int\n",
            "  math::target(value)\n",
            "end\n",
        ),
    );
    workspace.write("other/veln.toml", "");
    write_dependency_package(&workspace, "vendor/dep", "example/dep", "lib/math.veln");
    write_dependency_package(&workspace, "vendor/other", "other/dep", "other_math.veln");

    let result = references_result(&workspace, "main.veln", 6, 10);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[("main.veln", 6, 9, 6, 15)],
        "dependency function identity",
    );
}

#[test]
fn references_return_direct_dependency_type_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-type");
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
    );
    workspace.write(
        "other.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn second(input: model::Item) -> model::Item\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "pub type Item\n  pub Ready(Int)\nend\n\nfn package_body(input: Item) -> Item\n  input\nend\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write("vendor/other/model.veln", "pub type Item\nend\n");

    let result = references_result(&workspace, "main.veln", 13, 23);

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
            ("main.veln", 8, 15, 8, 19),
            ("main.veln", 11, 25, 11, 29),
            ("main.veln", 13, 23, 13, 27),
            ("main.veln", 13, 39, 13, 43),
            ("main.veln", 14, 10, 14, 14),
            ("main.veln", 22, 30, 22, 34),
            ("main.veln", 22, 51, 22, 55),
            ("other.veln", 3, 25, 3, 29),
            ("other.veln", 3, 41, 3, 45),
        ],
        "dependency type references",
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
fn references_return_standard_library_function_locations() {
    let std_workspace = TempWorkspace::new("references-standard-library-boundary");
    std_workspace.write("veln.toml", "");
    std_workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n\n",
            "fn first(value: Int) -> Int\n",
            "  math::exported(value)\n",
            "end\n\n",
            "fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(value))\n",
            "end\n",
        ),
    );
    let mut std_server = initialized_server(&std_workspace);
    std_server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn exported(value: Int) -> Int\n  value\nend\n",
        )],
    );

    let std_definition =
        std_server.definition_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_definition["isError"], false, "{std_definition:#}");
    let std_uri = std_definition["structuredContent"]["definition"]["uri"]
        .as_str()
        .unwrap_or_else(|| {
            panic!("standard library definition should resolve: {std_definition:#}")
        });
    assert!(std_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert!(std_uri.ends_with("/math.veln"));
    let std_references =
        std_server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));
    assert_eq!(std_references["isError"], false, "{std_references:#}");
    assert_reference_ranges(
        &std_references,
        &[
            ("main.veln", 4, 9, 4, 17),
            ("main.veln", 8, 40, 8, 48),
            ("main.veln", 9, 18, 9, 26),
        ],
        "standard library function",
    );
}

#[test]
fn references_keep_standard_library_function_collision_boundaries() {
    let workspace = TempWorkspace::new("references-standard-library-collisions");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n",
            "use dep from \"example/dep\"\n\n",
            "# exported mention\n",
            "pub fn exported(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "pub fn first(record: {exported: Int}, value: Int) -> Int\n",
            "  math::exported(value)\n",
            "  dep::exported(value)\n",
            "  record.exported\n",
            "  \"exported\"\n",
            "  value\n",
            "end\n\n",
            "pub fn second(value: Int) -> Int\n",
            "  let exported = value\n",
            "  let callback: fn(Int) -> Int = math::exported\n",
            "  callback(math::exported(exported))\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn exported(value: Int) -> Int\n  value\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            concat!(
                "pub fn exported(value: Int) -> Int\n",
                "  exported(value - 1)\n",
                "end\n",
            )
            .as_bytes(),
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":10,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 10, 9, 10, 17),
            ("main.veln", 19, 40, 19, 48),
            ("main.veln", 20, 18, 20, 26),
        ],
        "standard library collision boundary",
    );

    let alias_segment = server.references_tool(&json!({"source":"main.veln","line":1,"column":5}));
    assert_eq!(alias_segment["isError"], false, "{alias_segment:#}");
    assert_eq!(
        alias_segment["structuredContent"]["references"],
        json!([]),
        "{alias_segment:#}"
    );
}

#[test]
fn references_return_standard_library_type_locations() {
    let workspace = TempWorkspace::new("references-standard-library-type");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(items: Vec<Int>) -> Vec<Int>\n",
            "  items\n",
            "end\n\n",
            "fn second(items: prelude::Vec<Int>) -> prelude::Vec<Int>\n",
            "  prelude::Vec::Empty()\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third(items: Vec<Int>) -> prelude::Vec<Int>\n  items\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Vec\n  pub Empty\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":17}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 1, 17, 1, 20),
            ("main.veln", 1, 30, 1, 33),
            ("main.veln", 5, 27, 5, 30),
            ("main.veln", 5, 49, 5, 52),
            ("main.veln", 6, 12, 6, 15),
            ("other.veln", 1, 17, 1, 20),
            ("other.veln", 1, 39, 1, 42),
        ],
        "standard library type",
    );
}

#[test]
fn references_return_standard_library_constructor_locations() {
    let workspace = TempWorkspace::new("references-standard-library-constructor");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn first(input: Option<Int>) -> Int\n",
            "  match input\n",
            "    Some(value) => value\n",
            "    None => 0\n",
            "  end\n",
            "end\n\n",
            "fn second() -> Option<Int>\n",
            "  prelude::Option::Some(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn third() -> Option<Int>\n  prelude::Some(2)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Option\n  pub Some(Int)\n  pub None\nend\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":6}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 3, 5, 3, 9),
            ("main.veln", 9, 20, 9, 24),
            ("other.veln", 2, 12, 2, 16),
        ],
        "standard library constructor",
    );
}
