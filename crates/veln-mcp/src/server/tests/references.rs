use super::references_support::{
    all_resource_state, assert_reference_ranges,
    assert_snapshot_changed_without_references_or_scope, dependency_resource_is_listed,
    references_result, write_workspace_with_dependency_and_sources,
};
use super::*;

mod capture_failures;
mod dependency_constructors_and_aliases;
mod dependency_functions_and_types;
mod local_bindings;
mod standard_library_and_scope;
mod unsupported_and_coordinates;
mod workspace_symbols;

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
