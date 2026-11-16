use super::*;
use veln_project::PackageSnapshotSource;

fn workspace_module_value_project(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "values.veln",
        "pub let answer: Int = 42\nlet direct: Int = answer\n",
    );
    workspace.write(
        "main.veln",
        "use values\n\nlet qualified: Int = values::answer\n",
    );
    workspace.write(
        "alias.veln",
        "use values as config\n\nlet aliased: Int = config::answer\n",
    );
}

fn expected_workspace_module_value_references(workspace: &TempWorkspace) -> serde_json::Value {
    json!([
        {
            "uri": crate::definition::path_to_uri(&workspace.path("alias.veln")),
            "range": {"start":{"line":3,"column":28},"end":{"line":3,"column":34}}
        },
        {
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {"start":{"line":3,"column":30},"end":{"line":3,"column":36}}
        },
        {
            "uri": crate::definition::path_to_uri(&workspace.path("values.veln")),
            "range": {"start":{"line":2,"column":19},"end":{"line":2,"column":25}}
        }
    ])
}

#[test]
fn workspace_module_value_references_share_selection_paths() {
    let workspace = TempWorkspace::new("references-workspace-module-value-selection");
    workspace_module_value_project(&workspace);
    let expected = expected_workspace_module_value_references(&workspace);

    for (source, line, column) in [
        ("values.veln", 1, 10),
        ("values.veln", 2, 20),
        ("main.veln", 3, 31),
        ("alias.veln", 3, 29),
    ] {
        let result = initialized_server(&workspace).references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":false
        }));
        assert_eq!(result["structuredContent"]["references"], expected);
    }
}

#[test]
fn workspace_module_value_references_paginate_without_declaration() {
    let workspace = TempWorkspace::new("references-workspace-module-value-pagination");
    workspace_module_value_project(&workspace);
    let mut server = initialized_server(&workspace);
    let first = server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":31,
        "include_declaration":false, "page_size":1
    }));
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = server.references_tool(&json!({"cursor":cursor}));
    let cursor = second["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let third = server.references_tool(&json!({"cursor":cursor}));
    assert!(third["structuredContent"].get("next_cursor").is_none());
    let paged = [first, second, third]
        .into_iter()
        .flat_map(|page| {
            page["structuredContent"]["references"]
                .as_array()
                .unwrap()
                .clone()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paged,
        expected_workspace_module_value_references(&workspace)
            .as_array()
            .unwrap()
            .clone()
    );
}

#[test]
fn workspace_module_value_reference_cursor_captures_declaration_and_locations() {
    let workspace = TempWorkspace::new("references-workspace-module-value-cursor");
    workspace_module_value_project(&workspace);
    let mut server = initialized_server(&workspace);
    let first = server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":31,
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":3,"column":28},"end":{"line":3,"column":34}})
    );
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_module_value_initial_failure_preserves_state(&workspace, &mut server);
    assert_module_value_cursor_snapshot(&workspace, &mut server, cursor);
}

fn assert_module_value_cursor_snapshot(
    workspace: &TempWorkspace,
    server: &mut Server,
    cursor: String,
) {
    workspace.write("alias.veln", "use values as config\n");
    let unsupported = server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":1,
        "include_declaration":true
    }));
    assert_eq!(unsupported["structuredContent"]["references"], json!([]));

    let main_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        main_page["structuredContent"]["references"][0],
        json!({
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {"start":{"line":3,"column":30},"end":{"line":3,"column":36}}
        })
    );
    let cursor = main_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let declaration_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        declaration_page["structuredContent"]["references"][0],
        json!({
            "uri": crate::definition::path_to_uri(&workspace.path("values.veln")),
            "range": {"start":{"line":1,"column":9},"end":{"line":1,"column":15}}
        })
    );
    let cursor = declaration_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap();
    let final_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":2,"column":19},"end":{"line":2,"column":25}})
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}

fn assert_module_value_initial_failure_preserves_state(
    workspace: &TempWorkspace,
    server: &mut Server,
) {
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":31,
        "include_declaration":true
    }));
    let resources = all_resource_state(server);
    let selection = server.selection_result();

    let invalid = server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1,
        "include_declaration":true
    }));

    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    assert_eq!(all_resource_state(server), resources);
    assert_eq!(server.selection_result(), selection);
    let after = server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":31,
        "include_declaration":true
    }));
    assert_eq!(after, before);
    assert_eq!(
        after["structuredContent"]["references"],
        expected_workspace_module_value_references_with_declaration(workspace)
    );
}

fn expected_workspace_module_value_references_with_declaration(
    workspace: &TempWorkspace,
) -> serde_json::Value {
    let mut expected = expected_workspace_module_value_references(workspace)
        .as_array()
        .unwrap()
        .clone();
    expected.insert(
        2,
        json!({
            "uri": crate::definition::path_to_uri(&workspace.path("values.veln")),
            "range": {"start":{"line":1,"column":9},"end":{"line":1,"column":15}}
        }),
    );
    json!(expected)
}

#[test]
fn module_value_references_keep_negative_selections_on_existing_paths() {
    let workspace = TempWorkspace::new("references-module-value-exclusions");
    workspace.write("veln.toml", "");
    workspace.write("values.veln", "pub let answer: Int = 42\n");
    workspace.write(
        "main.veln",
        concat!(
            "use values\n\n",
            "fn answer(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "let callback: fn(Int) -> Int = answer\n",
            "let selected: Int = values::answer\n",
            "let unresolved: Int = values::missing\n",
            "\nfn local() -> Int\n",
            "  let answer = 1\n",
            "  answer\n",
            "end\n",
        ),
    );

    let function = initialized_server(&workspace).references_tool(&json!({
        "source":"main.veln", "line":3, "column":4,
        "include_declaration":false
    }));
    assert_reference_ranges(
        &function,
        &[("main.veln", 7, 32, 7, 38)],
        "same-spelled function",
    );

    let lexical = initialized_server(&workspace).references_tool(&json!({
        "source":"main.veln", "line":13, "column":4,
        "include_declaration":false
    }));
    assert_reference_ranges(
        &lexical,
        &[("main.veln", 13, 3, 13, 9)],
        "same-spelled lexical binding",
    );

    let unresolved = initialized_server(&workspace).references_tool(&json!({
        "source":"main.veln", "line":9, "column":32,
        "include_declaration":true
    }));
    assert_eq!(
        unresolved["structuredContent"]["references"],
        json!([]),
        "{unresolved:#}"
    );

    let standard_workspace = TempWorkspace::new("references-standard-library-module-value");
    standard_workspace.write("veln.toml", "");
    standard_workspace.write(
        "main.veln",
        "use values from \"std\"\n\nlet selected: Int = values::answer\n",
    );
    let mut standard_server = initialized_server(&standard_workspace);
    standard_server
        .language_resources
        .replace_test_standard_library(
            "[package]\nname = \"std\"\n\n[lib]\nexports = [\"values.veln\"]\n",
            [PackageSnapshotSource::new(
                "values.veln",
                b"pub let answer: Int = 42\n",
            )],
        );
    let standard_library = standard_server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":30,
        "include_declaration":true
    }));
    assert_eq!(
        standard_library["structuredContent"]["references"],
        json!([]),
        "{standard_library:#}"
    );
}

#[test]
fn invalid_module_value_declaration_and_occurrence_do_not_enter_references() {
    let workspace = TempWorkspace::new("references-invalid-module-value");
    workspace.write("veln.toml", "");
    workspace.write("invalid.veln", "pub let answer: String = 42\n");
    workspace.write(
        "main.veln",
        "use invalid\n\nlet selected: String = invalid::answer\n",
    );

    for (source, line, column) in [("invalid.veln", 1, 10), ("main.veln", 3, 31)] {
        let result = initialized_server(&workspace).references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

fn dependency_module_value_project(workspace: &TempWorkspace) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "let direct: Int = answer\n",
            "let qualified: Int = dep::answer\n",
        ),
    );
    workspace.write(
        "alias.veln",
        concat!(
            "use dep as config from \"example/dep\"\n\n",
            "let aliased: Int = config::answer\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub let answer: Int = 7\n");
}

#[test]
fn dependency_module_value_references_keep_package_owner_and_declaration() {
    let workspace = TempWorkspace::new("references-dependency-module-value");
    dependency_module_value_project(&workspace);
    let expected_ranges = [
        ("alias.veln", 3, 28, 3, 34),
        ("main.veln", 3, 19, 3, 25),
        ("main.veln", 4, 27, 4, 33),
    ];

    for (source, line, column) in [
        ("main.veln", 3, 20),
        ("main.veln", 4, 27),
        ("alias.veln", 3, 29),
    ] {
        let result = initialized_server(&workspace).references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":false
        }));
        assert_reference_ranges(&result, &expected_ranges, "dependency module value");
    }

    let result = initialized_server(&workspace).references_tool(&json!({
        "source":"alias.veln", "line":3, "column":29,
        "include_declaration":true
    }));
    assert_eq!(
        result["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let declaration = result["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .iter()
        .find(|location| {
            location["uri"]
                .as_str()
                .is_some_and(|uri| uri.starts_with("veln-pkg:"))
        })
        .expect("dependency declaration should retain its package URI");
    assert_eq!(
        declaration["range"],
        json!({"start":{"line":1,"column":9},"end":{"line":1,"column":15}})
    );
}

#[test]
fn module_value_references_isolate_workspace_and_dependency_package_owners() {
    let workspace = TempWorkspace::new("references-module-value-owner-isolation");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"one/pkg\"]\npath = \"vendor/one\"\n\n",
            "[dependencies.\"two/pkg\"]\npath = \"vendor/two\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use shared as first from \"one/pkg\"\n",
            "use shared as second from \"two/pkg\"\n\n",
            "let left: Int = first::answer\n",
            "let right: Int = second::answer\n",
        ),
    );
    workspace.write(
        "shared.veln",
        "pub let answer: Int = 42\nlet local: Int = answer\n",
    );
    for (directory, package) in [("one", "one/pkg"), ("two", "two/pkg")] {
        workspace.write(
            &format!("vendor/{directory}/veln.toml"),
            &format!("[package]\nname = \"{package}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"),
        );
        workspace.write(
            &format!("vendor/{directory}/shared.veln"),
            "pub let answer: Int = 42\n",
        );
    }

    for (line, column, package) in [(4, 25, "one%2Fpkg"), (5, 27, "two%2Fpkg")] {
        let result = initialized_server(&workspace).references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":true
        }));
        let references = result["structuredContent"]["references"]
            .as_array()
            .unwrap();
        assert_eq!(references.len(), 2, "{result:#}");
        assert_eq!(references[0]["range"]["start"]["line"], line);
        assert!(
            references[1]["uri"]
                .as_str()
                .is_some_and(|uri| uri.contains(package)),
            "{result:#}"
        );
    }

    let workspace_result = initialized_server(&workspace).references_tool(&json!({
        "source":"shared.veln", "line":2, "column":18,
        "include_declaration":true
    }));
    assert!(
        workspace_result["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .iter()
            .all(|location| location["uri"]
                .as_str()
                .is_some_and(|uri| uri.starts_with("file:")))
    );
}
