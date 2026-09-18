use super::references_support::{
    all_resource_state, assert_reference_ranges,
    assert_snapshot_changed_without_references_or_scope, dependency_resource_is_listed,
    references_result, write_workspace_with_dependency_and_sources,
};
use super::*;

mod capture_failures;
mod dependency_constructors_and_aliases;
mod dependency_functions_and_types;
mod dependency_schemas;
mod local_bindings;
mod standard_library_and_scope;
mod unsupported_and_coordinates;
mod workspace_symbols;

#[test]
fn references_cursor_transitions_are_server_bound_single_use_and_refresh_aware() {
    let workspace = TempWorkspace::new("references-cursor-transitions");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn helper(value: Int) -> Int\n",
            "  helper(value - 1)\n",
            "end\n\n",
            "fn main() -> Int\n",
            "  helper(1)\n",
            "end\n",
        ),
    );

    let mut first = initialized_server(&workspace);
    let mut second = initialized_server(&workspace);
    let first_page = first.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1
    }));
    let cursor = first_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();

    let foreign = second.references_tool(&json!({"cursor": cursor}));
    assert_eq!(foreign["structuredContent"]["code"], "invalid_cursor");
    let tampered = first.references_tool(&json!({"cursor": cursor.to_owned() + "x"}));
    assert_eq!(tampered["structuredContent"]["code"], "invalid_cursor");

    let mut restarted = initialized_server(&workspace);
    let post_restart = restarted.references_tool(&json!({"cursor": cursor}));
    assert_eq!(post_restart["structuredContent"]["code"], "invalid_cursor");

    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  value\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
    let continuation = first.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false);
    assert!(continuation["result"].is_null());
    assert!(continuation["structuredContent"]["references"].is_array());

    let replay = first.references_tool(&json!({"cursor": cursor}));
    assert_eq!(replay["structuredContent"]["code"], "invalid_cursor");

    workspace.write(
        "main.veln",
        concat!(
            "fn helper(value: Int) -> Int\n",
            "  helper(value - 1)\n",
            "end\n\n",
            "fn main() -> Int\n",
            "  helper(1)\n",
            "end\n",
        ),
    );
    let refresh_page = first.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1
    }));
    let refresh_cursor = refresh_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let refreshed = first.refresh_workspace_tool(|_| Ok(()));
    assert_eq!(refreshed["isError"], false);
    let stale = first.references_tool(&json!({"cursor": refresh_cursor}));
    assert_eq!(stale["structuredContent"]["code"], "stale_snapshot");
}

#[test]
fn references_pages_preserve_order_scope_and_captured_locations() {
    let workspace = TempWorkspace::new("references-page-contract");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn helper(value: Int) -> Int\n",
            "  helper(value - 1)\n",
            "end\n\n",
            "fn main() -> Int\n",
            "  helper(1)\n",
            "end\n",
        ),
    );

    let mut server = initialized_server(&workspace);
    let complete = server.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1000
    }));
    let expected = complete["structuredContent"]["references"].clone();
    let expected_scope = complete["structuredContent"]["scope"].clone();

    let first = server.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1
    }));
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(first["structuredContent"]["scope"], expected_scope);
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let captured_first = first["structuredContent"]["references"][0].clone();
    workspace.write("main.veln", "fn helper(value: Int) -> Int\n  value\nend\n");
    let second = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(second["structuredContent"]["scope"], expected_scope);
    assert_eq!(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(captured_first, expected[0]);
    assert_eq!(second["structuredContent"]["references"][0], expected[1]);
    assert!(
        !second["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );
}

#[test]
fn failed_refresh_and_invalid_continuation_requests_preserve_live_state() {
    let workspace = TempWorkspace::new("references-failure-atomicity");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let first = server.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1
    }));
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();

    let initial_failure = server.references_tool(&json!({
        "source": "missing.veln",
        "line": 1,
        "column": 1
    }));
    assert_eq!(initial_failure["isError"], true);
    assert_eq!(initial_failure["structuredContent"]["code"], "invalid_path");

    let invalid_shape = server
        .handle_request(json!({
            "jsonrpc":"2.0", "id": 1, "method":"tools/call",
            "params":{"name":"references","arguments":{"cursor":cursor,"page_size":1}}
        }))
        .unwrap();
    assert_eq!(invalid_shape["error"]["code"], -32602);

    let failed_refresh = server.refresh_workspace_tool(|selection| {
        selection.refresh_with(|| Err(std::io::Error::other("injected failure")))
    });
    assert_eq!(failed_refresh["isError"], true);
    assert_eq!(
        failed_refresh["structuredContent"]["code"],
        "generation_failed"
    );

    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false);
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

struct WorkspaceSymbolCase {
    name: &'static str,
    files: Vec<(&'static str, &'static str)>,
    source: &'static str,
    line: usize,
    column: usize,
    ranges: Vec<(&'static str, usize, usize, usize, usize)>,
}

fn assert_workspace_symbol_cases(cases: impl IntoIterator<Item = WorkspaceSymbolCase>) {
    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in case.files {
            workspace.write(path, text);
        }
        let result = references_result(&workspace, case.source, case.line, case.column);
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["scope"]["project_wide"], true,
            "{}: {result:#}",
            case.name
        );
        assert_reference_ranges(&result, &case.ranges, case.name);
    }
}

fn write_dependency_reference_workspace(
    workspace: &TempWorkspace,
    field: &str,
    source: &str,
    selector: Option<&str>,
) {
    let selector = selector
        .map(|selector| format!("{selector}\n"))
        .unwrap_or_default();
    workspace.write(
        "veln.toml",
        &format!("[dependencies.\"example/dep\"]\n{field} = \"{source}\"\n{selector}"),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use lib::math from \"example/dep\"\n\n",
            "pub fn first(value: Int) -> Int\n",
            "  math::increase(value)\n",
            "end\n\n",
            "pub fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::increase\n",
            "  callback(math::increase(value))\n",
            "end\n",
        ),
    );
    write_dependency_package(workspace, source, "example/dep", "lib/math.veln");
}

fn write_dependency_package(workspace: &TempWorkspace, root: &str, identity: &str, export: &str) {
    workspace.write(
        &format!("{root}/veln.toml"),
        &format!("[package]\nname = \"{identity}\"\n\n[lib]\nexports = [\"{export}\"]\n"),
    );
    workspace.write(
        &format!("{root}/{export}"),
        concat!(
            "pub fn increase(value: Int) -> Int\n",
            "  increase(value - 1)\n",
            "end\n",
            "\n",
            "pub fn target(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
        ),
    );
}
