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
