use super::*;

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
fn references_include_eligible_package_declaration_after_workspace_references() {
    let workspace = TempWorkspace::new("references-package-declaration-inclusion");
    write_dependency_reference_workspace(&workspace, "path", "vendor/dep", None);
    let mut server = initialized_server(&workspace);

    let first = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 10,
        "page_size": 3,
        "include_declaration": true
    }));
    assert_eq!(first["isError"], false, "{first:#}");
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .iter()
            .all(|location| location["uri"].as_str().unwrap().starts_with("file://"))
    );
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();

    let invalid_continuation = server
        .handle_request(json!({
            "jsonrpc": "2.0",
            "id": "invalid-continuation",
            "method": "tools/call",
            "params": {
                "name": "references",
                "arguments": {"cursor": cursor, "include_declaration": true}
            }
        }))
        .unwrap();
    assert_eq!(invalid_continuation["error"]["code"], -32602);

    let second = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(second["isError"], false, "{second:#}");
    let package_declaration = second["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .find(|location| {
            location["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///")
        })
        .unwrap_or_else(|| panic!("package declaration missing: {second:#}"));
    assert!(
        package_declaration["uri"]
            .as_str()
            .unwrap()
            .ends_with("/lib/math.veln"),
        "{second:#}"
    );
    assert_eq!(
        package_declaration["range"]["start"],
        json!({"line": 1, "column": 8})
    );
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
fn references_return_direct_dependency_type_alias_locations_from_saved_project() {
    let workspace = TempWorkspace::new("references-dependency-type-alias");
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
            "use core from \"example/dep\"\n",
            "use other_model from \"other/dep\"\n\n",
            "pub type LocalAlias = model::Same\n\n",
            "fn alias(input: model::Same) -> model::Same\n",
            "  let current: model::Same = input\n",
            "  model::Same::Ready(1)\n",
            "end\n\n",
            "fn target(input: core::Same) -> core::Same\n",
            "  core::Same::Ready(1)\n",
            "end\n\n",
            "fn collisions(input: other_model::Same, Same: Int, record: {Same: Int}) -> Int\n",
            "  Same + record.Same\n",
            "end\n\n",
            "type Same\n",
            "  Ready(Int)\n",
            "end\n\n",
            "fn local_constructor() -> Same\n",
            "  Same::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn other(input: model::Same) -> model::Same\n",
            "  model::Same::Ready(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\", \"core.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "use core\n\npub type Same = core::Same\n",
    );
    workspace.write(
        "vendor/dep/core.veln",
        "pub type Same\n  pub Ready(Int)\nend\n",
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write("vendor/other/model.veln", "pub type Same\nend\n");

    let alias = references_result(&workspace, "main.veln", 7, 24);

    assert_eq!(alias["isError"], false, "{alias:#}");
    assert_eq!(
        alias["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );
    assert_reference_ranges(
        &alias,
        &[
            ("main.veln", 5, 30, 5, 34),
            ("main.veln", 7, 24, 7, 28),
            ("main.veln", 7, 40, 7, 44),
            ("main.veln", 8, 23, 8, 27),
            ("main.veln", 9, 10, 9, 14),
            ("other.veln", 3, 24, 3, 28),
            ("other.veln", 3, 40, 3, 44),
            ("other.veln", 4, 10, 4, 14),
        ],
        "dependency type alias references",
    );
    let references = alias["structuredContent"]["references"].as_array().unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
            && !reference["uri"].as_str().unwrap().contains("vendor/dep")
    }));

    let target = references_result(&workspace, "main.veln", 12, 24);

    assert_eq!(target["isError"], false, "{target:#}");
    assert_reference_ranges(
        &target,
        &[
            ("main.veln", 12, 24, 12, 28),
            ("main.veln", 12, 39, 12, 43),
            ("main.veln", 13, 9, 13, 13),
        ],
        "dependency type target references",
    );
}

#[test]
fn references_keep_descendant_project_sources_isolated_for_dependency_type_aliases() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-descendant-isolation");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use model from \"example/dep\"\n\nfn root(input: model::Same) -> model::Same\n  input\nend\n",
    );
    workspace.write(
        "nested/veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"../vendor/dep\"\n",
    );
    workspace.write(
        "nested/main.veln",
        "use model from \"example/dep\"\n\nfn nested(input: model::Same) -> model::Same\n  input\nend\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/model.veln",
        "pub type Target\nend\n\npub type Same = Target\n",
    );

    let result = references_result(&workspace, "nested/main.veln", 3, 24);

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "nested/main.veln",
            "project_wide": false
        })
    );
    assert_eq!(result["structuredContent"]["references"], json!([]));
}

#[test]
fn references_resolve_type_alias_written_module_path_and_leaf_alias_to_same_identity() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-qualified-identity");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::model from \"example/dep\"\n\n",
            "fn written(input: lib::model::Alias) -> model::Alias\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"lib/model.veln\", \"lib/core.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/lib/model.veln",
        "use lib::core\n\npub type Alias = core::Target\n",
    );
    workspace.write(
        "vendor/dep/lib/core.veln",
        "pub type Target\n  pub Ready(Int)\nend\n",
    );

    for (name, column) in [("written module path", 31), ("leaf import alias", 48)] {
        let result = references_result(&workspace, "main.veln", 3, column);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_reference_ranges(
            &result,
            &[("main.veln", 3, 31, 3, 36), ("main.veln", 3, 48, 3, 53)],
            name,
        );
    }
}

#[test]
fn references_keep_type_alias_selection_order_and_unsupported_boundaries() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-selection-order");
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
            "type Same\n",
            "end\n\n",
            "fn local(record: {Same: Int}, input: Same) -> Same\n",
            "  input\n",
            "end\n\n",
            "fn unsupported(input: model::MissingAlias, transitive: model::Transitive) -> other_model::MissingAlias\n",
            "  input\n",
            "end\n\n",
            "fn private_alias(input: model::PrivateAlias) -> Int\n",
            "  0\n",
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
            "use upstream from \"up/pkg\"\n\n",
            "pub type Target\n",
            "end\n\n",
            "pub type Same = Target\n",
            "pub type MissingAlias = Missing\n",
            "pub type Transitive = upstream::Alias\n",
            "type PrivateAlias = Target\n",
        ),
    );
    workspace.write(
        "vendor/other/veln.toml",
        "[package]\nname = \"other/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write("vendor/other/model.veln", "pub type MissingAlias\nend\n");

    let local = references_result(&workspace, "main.veln", 7, 39);
    assert_eq!(local["isError"], false, "{local:#}");
    assert_reference_ranges(
        &local,
        &[("main.veln", 7, 38, 7, 42), ("main.veln", 7, 47, 7, 51)],
        "local type precedence",
    );

    let field = references_result(&workspace, "main.veln", 7, 19);
    assert_eq!(field["isError"], false, "{field:#}");
    assert_eq!(field["structuredContent"]["references"], json!([]));

    let unsupported = references_result(&workspace, "main.veln", 11, 30);
    assert_eq!(unsupported["isError"], false, "{unsupported:#}");
    assert_eq!(unsupported["structuredContent"]["references"], json!([]));
    assert_eq!(
        unsupported["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        })
    );

    let transitive = references_result(&workspace, "main.veln", 11, 63);
    assert_eq!(transitive["isError"], false, "{transitive:#}");
    assert_eq!(transitive["structuredContent"]["references"], json!([]));

    let private_alias = references_result(&workspace, "main.veln", 15, 32);
    assert_eq!(private_alias["isError"], false, "{private_alias:#}");
    assert_eq!(private_alias["structuredContent"]["references"], json!([]));
}

#[test]
fn references_reject_type_alias_targets_that_do_not_resolve_semantically() {
    struct Case {
        name: &'static str,
        dependency_sources: Vec<(&'static str, &'static str)>,
    }

    let cases = [
        Case {
            name: "invalid-module-target",
            dependency_sources: vec![
                ("model.veln", "use Bad\n\npub type Alias = Bad::Target\n"),
                ("Bad.veln", "pub type Target\nend\n"),
            ],
        },
        Case {
            name: "parse-diagnostic-target",
            dependency_sources: vec![
                (
                    "model.veln",
                    "use broken\n\npub type Alias = broken::Target\n",
                ),
                ("broken.veln", "pub type Target\n  pub Ready(Int)\n"),
            ],
        },
        Case {
            name: "ambiguous-target",
            dependency_sources: vec![(
                "model.veln",
                concat!(
                    "pub type Target\n",
                    "end\n\n",
                    "pub type Target\n",
                    "end\n\n",
                    "pub type Alias = Target\n",
                ),
            )],
        },
    ];

    for case in cases {
        let workspace =
            TempWorkspace::new(&format!("references-dependency-type-alias-{}", case.name));
        workspace.write(
            "veln.toml",
            "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
        );
        workspace.write(
            "main.veln",
            concat!(
                "use model from \"example/dep\"\n\n",
                "fn read(input: model::Alias) -> model::Alias\n",
                "  input\n",
                "end\n",
            ),
        );
        workspace.write(
            "vendor/dep/veln.toml",
            "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
        );
        for (path, text) in case.dependency_sources {
            workspace.write(&format!("vendor/dep/{path}"), text);
        }

        let result = references_result(&workspace, "main.veln", 3, 23);

        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}",
            case.name
        );
    }
}
