use super::references_support::{
    all_resource_state, assert_package_declaration, assert_reference_ranges,
    assert_snapshot_changed_without_references_or_scope, dependency_resource_is_listed,
    references_result, write_workspace_with_dependency_and_sources,
};
use super::*;

mod capture_failures;
mod dependency_constructors_and_aliases;
mod dependency_functions_and_types;
mod dependency_schemas;
mod local_bindings;
mod scope_and_symbol_boundaries;
mod standard_library_and_scope;
mod unsupported_and_coordinates;
mod workspace_symbols;

#[test]
fn references_cursor_transitions_are_server_bound_single_use_and_refresh_aware() {
    let workspace = TempWorkspace::new("references-cursor-transitions");
    write_recursive_reference_workspace(&workspace);

    let mut first = initialized_server(&workspace);
    let cursor = first_reference_cursor(&mut first);
    assert_cursor_authentication_rejections(&workspace, &mut first, &cursor);
    assert_cursor_uses_captured_locations(&workspace, &mut first, &cursor);
    write_recursive_reference_source(&workspace);
    let refresh_cursor = first_reference_cursor(&mut first);
    assert_refresh_invalidates_cursor(&workspace, &mut first, &refresh_cursor);
    let consumed_after_refresh = first.references_tool(&json!({"cursor": cursor}));
    assert_eq!(
        consumed_after_refresh["structuredContent"]["code"],
        "invalid_cursor"
    );
}

fn write_recursive_reference_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    write_recursive_reference_source(workspace);
}

fn write_recursive_reference_source(workspace: &TempWorkspace) {
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
}

fn first_reference_cursor(server: &mut Server) -> String {
    server.references_tool(&json!({
        "source": "main.veln", "line": 6, "column": 4, "page_size": 1,
        "include_declaration": true
    }))["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn assert_cursor_authentication_rejections(
    workspace: &TempWorkspace,
    server: &mut Server,
    cursor: &str,
) {
    let mut foreign_server = initialized_server(workspace);
    let foreign = foreign_server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(foreign["structuredContent"]["code"], "invalid_cursor");
    let tampered = server.references_tool(&json!({"cursor": format!("{cursor}x")}));
    assert_eq!(tampered["structuredContent"]["code"], "invalid_cursor");
    let mut restarted = initialized_server(workspace);
    let post_restart = restarted.references_tool(&json!({"cursor": cursor}));
    assert_eq!(post_restart["structuredContent"]["code"], "invalid_cursor");
}

fn assert_cursor_uses_captured_locations(
    workspace: &TempWorkspace,
    server: &mut Server,
    cursor: &str,
) {
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  value\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false);
    assert!(continuation["result"].is_null());
    assert!(continuation["structuredContent"]["references"].is_array());
}

fn assert_refresh_invalidates_cursor(workspace: &TempWorkspace, server: &mut Server, cursor: &str) {
    let refreshed = server.refresh_workspace_tool(|_| Ok(()));
    assert_eq!(refreshed["isError"], false);
    let stale = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(stale["structuredContent"]["code"], "stale_snapshot");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  value\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
    let restored = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(restored["structuredContent"]["code"], "stale_snapshot");
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
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  value\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
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
    assert_eq!(
        server.references_tool(&json!({"cursor": cursor}))["structuredContent"]["code"],
        "invalid_cursor"
    );
}

#[test]
fn references_continuation_survives_source_removal() {
    let workspace = TempWorkspace::new("references-source-removal-continuation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );

    let mut server = initialized_server(&workspace);
    let complete = server.references_tool(&json!({
        "source": "main.veln",
        "line": 6,
        "column": 4,
        "page_size": 1000
    }));
    let expected = complete["structuredContent"]["references"]
        .as_array()
        .unwrap()
        .clone();
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
    fs::remove_file(workspace.path("main.veln")).unwrap();

    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false, "{continuation:#}");
    assert_eq!(continuation["structuredContent"]["scope"], expected_scope);
    assert_eq!(
        continuation["structuredContent"]["references"],
        json!([expected[1].clone()])
    );
    assert!(
        !continuation["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );
}

#[test]
fn references_initial_empty_and_final_pages_omit_next_cursor() {
    let workspace = TempWorkspace::new("references-initial-final-cursor-omission");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );

    let mut server = initialized_server(&workspace);
    let empty = server.references_tool(&json!({
        "source": "main.veln", "line": 1, "column": 1, "page_size": 1
    }));
    assert_eq!(empty["isError"], false, "{empty:#}");
    assert_eq!(empty["structuredContent"]["references"], json!([]));
    assert!(
        !empty["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );

    let final_page = server.references_tool(&json!({
        "source": "main.veln", "line": 6, "column": 4, "page_size": 1000
    }));
    assert_eq!(final_page["isError"], false, "{final_page:#}");
    assert_eq!(
        final_page["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        !final_page["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );
}

#[test]
fn references_server_pages_concatenate_ordered_multi_file_results() {
    let workspace = TempWorkspace::new("references-multi-file-pagination");
    write_multi_file_schema_reference_workspace(&workspace);
    let mut server = initialized_server(&workspace);
    let complete = server.references_tool(&json!({
        "source":"app/wire.veln", "line":1, "column":12, "page_size":1000,
        "include_declaration": true
    }));
    let expected = complete["structuredContent"]["references"].clone();
    let expected_scope = complete["structuredContent"]["scope"].clone();
    let first = server.references_tool(&json!({
        "source":"app/wire.veln", "line":1, "column":12, "page_size":3,
        "include_declaration": true
    }));
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor": cursor}));
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
    assert_eq!(first["structuredContent"]["scope"], expected_scope);
    assert_eq!(second["structuredContent"]["scope"], expected_scope);
    assert_eq!(concatenated, expected.as_array().unwrap().clone());
    assert!(
        !second["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("next_cursor")
    );
}

fn write_multi_file_schema_reference_workspace(workspace: &TempWorkspace) {
    workspace.write("veln.toml", "");
    workspace.write(
        "app/wire.veln",
        concat!(
            "pub schema Packet\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "fn local(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "  encode Packet from packet\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use app::wire\n\n",
            "fn imported(view: ByteView, packet: {value: Int}) -> ()\n",
            "  decode app::wire::Packet from view at byte_offset(0)?\n",
            "  encode wire::Packet from packet\n",
            "end\n",
        ),
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
        "column": 1,
        "include_declaration": true
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

#[test]
fn references_page_size_defaults_to_100_and_rejects_all_invalid_boundaries() {
    let workspace = TempWorkspace::new("references-page-size-boundaries");
    workspace.write("veln.toml", "");
    let calls = (0..1001)
        .map(|index| format!("  helper({index})\n"))
        .collect::<String>();
    workspace.write(
        "main.veln",
        &format!(
            "fn helper(value: Int) -> Int\n  value\nend\n\nfn main() -> Int\n{calls}  0\nend\n"
        ),
    );
    let mut server = initialized_server(&workspace);
    assert_default_reference_page_size(&mut server);
    assert_maximum_reference_page_size(&mut server);
    assert_invalid_reference_page_sizes(&mut server);
}

fn assert_default_reference_page_size(server: &mut Server) {
    let default_page = server.references_tool(&json!({
        "source": "main.veln",
        "line": 1,
        "column": 4
    }));
    assert_eq!(default_page["isError"], false, "{default_page:#}");
    assert_eq!(
        default_page["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        100
    );
    let cursor = default_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let final_page = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        100
    );
}

fn assert_maximum_reference_page_size(server: &mut Server) {
    let maximum_page = server.references_tool(&json!({
        "source": "main.veln",
        "line": 1,
        "column": 4,
        "page_size": 1000
    }));
    assert_eq!(
        maximum_page["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1000
    );
    let maximum_cursor = maximum_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap();
    let maximum_final = server.references_tool(&json!({"cursor": maximum_cursor}));
    assert_eq!(
        maximum_final["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

fn assert_invalid_reference_page_sizes(server: &mut Server) {
    for page_size in [json!(0), json!(1001), json!(1.5), Value::Null] {
        let response = server
            .handle_request(json!({
                "jsonrpc":"2.0", "id":"invalid-page-size", "method":"tools/call",
                "params":{"name":"references","arguments":{
                    "source":"main.veln", "line":1, "column":4, "page_size":page_size
                }}
            }))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602, "{response:#}");
    }
}

#[test]
fn references_server_eviction_marks_the_oldest_live_cursor_stale() {
    let workspace = TempWorkspace::new("references-server-eviction");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let cursors = (0..65)
        .map(|_| {
            server.references_tool(&json!({
                "source":"main.veln", "line":6, "column":4, "page_size":1
            }))["structuredContent"]["next_cursor"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<Vec<_>>();

    let evicted = server.references_tool(&json!({"cursor": cursors[0]}));
    assert_eq!(evicted["structuredContent"]["code"], "stale_snapshot");
    let retained = server.references_tool(&json!({"cursor": cursors[1]}));
    assert_eq!(retained["isError"], false);
    assert_eq!(
        retained["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn references_continuation_admission_order_remains_fifo_after_progress() {
    let workspace = TempWorkspace::new("references-continuation-eviction-order");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn helper(value: Int) -> Int\n  helper(value - 1)\nend\n\nfn main() -> Int\n  helper(1)\n  helper(2)\nend\n",
    );
    let mut server = initialized_server(&workspace);

    let first = server.references_tool(&json!({
        "source": "main.veln", "line": 6, "column": 4, "page_size": 1
    }));
    let first_cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let successor = server.references_tool(&json!({"cursor": first_cursor}));
    let successor_cursor = successor["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();

    for _ in 0..64 {
        let page = server.references_tool(&json!({
            "source": "main.veln", "line": 6, "column": 4, "page_size": 1
        }));
        assert!(page["structuredContent"].get("next_cursor").is_some());
    }

    let evicted_successor = server.references_tool(&json!({"cursor": successor_cursor}));
    assert_eq!(
        evicted_successor["structuredContent"]["code"],
        "stale_snapshot"
    );
    let replayed_first =
        server.references_tool(&json!({"cursor": first["structuredContent"]["next_cursor"]}));
    assert_eq!(
        replayed_first["structuredContent"]["code"],
        "invalid_cursor"
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
