use super::*;

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

#[test]
fn workspace_module_value_references_share_selection_declaration_and_cursor_paths() {
    let workspace = TempWorkspace::new("references-workspace-module-value");
    workspace_module_value_project(&workspace);
    let expected = json!([
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
    ]);

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
