use super::*;

pub(super) fn references_result(
    workspace: &TempWorkspace,
    source: &str,
    line: usize,
    column: usize,
) -> Value {
    initialized_server(workspace)
        .references_tool(&json!({"source": source, "line": line, "column": column}))
}

pub(super) fn assert_snapshot_changed_without_references_or_scope(result: &Value) {
    assert_eq!(result["isError"], true, "{result:#}");
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    let structured = result["structuredContent"].as_object().unwrap();
    assert!(!structured.contains_key("references"), "{result:#}");
    assert!(!structured.contains_key("scope"), "{result:#}");
}

pub(super) fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"references-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

pub(super) fn dependency_resource_is_listed(server: &mut Server, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    all_resource_state(server)
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

pub(super) fn write_workspace_with_dependency_and_sources(
    workspace: &TempWorkspace,
    main: &str,
    helper: Option<&str>,
) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write("main.veln", main);
    if let Some(helper) = helper {
        workspace.write("helper.veln", helper);
    }
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub fn value() -> Int\n  1\nend\n");
}

pub(super) fn assert_reference_ranges(
    result: &Value,
    expected: &[(&str, usize, usize, usize, usize)],
    name: &str,
) {
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: references must be an array: {result:#}"));
    assert_eq!(
        references.len(),
        expected.len(),
        "{name}: unexpected reference count: {result:#}"
    );
    for (reference, (file, start_line, start_column, end_line, end_column)) in
        references.iter().zip(expected)
    {
        let uri = reference["uri"].as_str().unwrap();
        assert!(
            uri.starts_with("file://"),
            "{name}: expected canonical file URI: {reference:#}"
        );
        assert!(
            !uri.contains("/./") && !uri.contains("/../"),
            "{name}: expected normalized file URI: {reference:#}"
        );
        assert!(uri.ends_with(file), "{name}: {reference:#}");
        assert_eq!(
            reference["range"],
            json!({
                "start": {"line": start_line, "column": start_column},
                "end": {"line": end_line, "column": end_column}
            }),
            "{name}: {reference:#}"
        );
    }
}

pub(super) fn assert_package_declaration(
    result: &Value,
    path_suffix: &str,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    name: &str,
) {
    let declaration = result["structuredContent"]["references"]
        .as_array()
        .unwrap_or_else(|| panic!("{name}: references must be an array: {result:#}"))
        .iter()
        .find(|location| {
            location["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///")
        })
        .unwrap_or_else(|| panic!("{name}: package declaration missing: {result:#}"));
    assert!(
        declaration["uri"].as_str().unwrap().ends_with(path_suffix),
        "{name}: {declaration:#}"
    );
    assert_eq!(
        declaration["range"],
        json!({
            "start": {"line": start_line, "column": start_column},
            "end": {"line": end_line, "column": end_column}
        }),
        "{name}: {declaration:#}"
    );
}
