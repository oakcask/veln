use super::*;
use std::collections::BTreeSet;

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

pub(super) fn complete_resource_state(server: &mut Server) -> Value {
    let resources = all_resource_state(server);
    let listed_uris = resources
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| resource["uri"].as_str())
        .filter(|uri| {
            (uri.starts_with("veln-pkg:///") && !uri.starts_with("veln-pkg:///std/"))
                || (uri.starts_with("veln-doc:///package/")
                    && !uri.starts_with("veln-doc:///package/std/"))
        })
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let listed_reads = read_resource_responses(server, listed_uris.iter());
    let module_uris = linked_resource_uris(&listed_reads, "/module/");
    let module_reads = read_resource_responses(server, module_uris.iter());
    let declaration_uris = linked_resource_uris(&module_reads, "/declaration/");
    let declaration_reads = read_resource_responses(server, declaration_uris.iter());
    assert!(
        !module_reads.is_empty() && !declaration_reads.is_empty(),
        "saved recovery fixture must publish linked dependency documentation"
    );
    json!({
        "resources":resources,
        "listed_reads":listed_reads,
        "module_reads":module_reads,
        "declaration_reads":declaration_reads
    })
}

fn read_resource_responses<'a>(
    server: &mut Server,
    uris: impl Iterator<Item = &'a String>,
) -> Vec<Value> {
    uris.map(|uri| {
        let response = server
            .handle_request(json!({
                "jsonrpc":"2.0",
                "id":"complete-resource-state",
                "method":"resources/read",
                "params":{"uri":uri}
            }))
            .unwrap();
        json!({"uri":uri,"response":response})
    })
    .collect()
}

fn linked_resource_uris(reads: &[Value], segment: &str) -> BTreeSet<String> {
    reads
        .iter()
        .filter_map(|read| read["response"]["result"]["contents"][0]["text"].as_str())
        .flat_map(extract_package_documentation_uris)
        .filter(|uri| uri.contains(segment))
        .collect()
}

fn extract_package_documentation_uris(text: &str) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find("veln-doc:///package/") {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        uris.insert(after_start[..end].to_string());
        rest = &after_start[end..];
    }
    uris
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
