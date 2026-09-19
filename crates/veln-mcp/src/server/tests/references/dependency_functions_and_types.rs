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

    let complete = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 10,
        "page_size": 1000,
        "include_declaration": true
    }));
    assert_eq!(complete["isError"], false, "{complete:#}");
    let expected = complete["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .clone();

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

    assert_declaration_continuation_is_cursor_only(&mut server, &cursor);
    assert_terminal_package_declaration_page(&mut server, &first, &expected, &cursor);
}

fn assert_declaration_continuation_is_cursor_only(server: &mut Server, cursor: &str) {
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
}

fn assert_terminal_package_declaration_page(
    server: &mut Server,
    first: &Value,
    expected: &[Value],
    cursor: &str,
) {
    let second = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(second["isError"], false, "{second:#}");
    let mut concatenated = first["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .clone();
    concatenated.extend(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .iter()
            .cloned(),
    );
    assert_eq!(
        concatenated, expected,
        "paged result differs from unpaged result"
    );
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
    assert_eq!(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "declaration must be the complete terminal page"
    );
    assert!(
        !second["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );
    assert_eq!(
        server.references_tool(&json!({"cursor": cursor}))["structuredContent"]["code"],
        "invalid_cursor"
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
