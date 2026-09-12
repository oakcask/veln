use super::*;
use std::cell::Cell;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

fn assert_reference_ranges(
    result: &Value,
    expected: &[(&str, usize, usize, usize, usize)],
    case: &str,
) {
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), expected.len(), "{case}: {result:#}");
    for (reference, (path, start_line, start_column, end_line, end_column)) in
        references.iter().zip(expected)
    {
        assert!(
            reference["uri"].as_str().unwrap().ends_with(path),
            "{case}: {reference:#}"
        );
        assert_eq!(reference["range"]["start"]["line"], *start_line, "{case}");
        assert_eq!(
            reference["range"]["start"]["column"], *start_column,
            "{case}"
        );
        assert_eq!(reference["range"]["end"]["line"], *end_line, "{case}");
        assert_eq!(reference["range"]["end"]["column"], *end_column, "{case}");
    }
}

fn install_alias_standard_library(server: &mut Server, source: &str) {
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new("math.veln", source.as_bytes())],
    );
}

fn assert_snapshot_changed_without_references_or_scope(result: &Value) {
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );
}

fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"references-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

fn dependency_resource_is_listed(server: &mut Server, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    all_resource_state(server)
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

#[test]
fn references_return_standard_library_function_alias_locations() {
    let workspace = TempWorkspace::new("references-standard-library-alias");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n\n",
            "fn first(value: Int) -> Int\n",
            "  math::renamed(value)\n",
            "end\n\n",
            "fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::renamed\n",
            "  callback(math::renamed(value)) + math::target(value)\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    install_alias_standard_library(
        &mut server,
        "pub fn target(value: Int) -> Int\n  value\nend\n\npub fn renamed = target\n",
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 4, 9, 4, 16),
            ("main.veln", 8, 40, 8, 47),
            ("main.veln", 9, 18, 9, 25),
        ],
        "standard library function alias",
    );
}

#[test]
fn references_keep_standard_library_function_alias_chain_boundary_empty() {
    let workspace = TempWorkspace::new("references-standard-library-alias-chain");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn main() -> Int\n  prelude::chained()\nend\n");
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            concat!(
                "pub fn target() -> Int\n",
                "  1\n",
                "end\n\n",
                "pub fn renamed = target\n",
                "pub fn chained = renamed\n",
            )
            .as_bytes(),
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":12}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["references"],
        json!([]),
        "{result:#}"
    );
}

#[test]
fn references_project_capture_exhausts_retries_for_standard_library_alias_selection() {
    let workspace = TempWorkspace::new("references-standard-library-alias-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use math from \"std\"\n\nfn main() -> Int\n  math::renamed()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    install_alias_standard_library(
        &mut server,
        "pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
    );
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            root.join("main.veln"),
            format!(
                "use math from \"std\"\n\nfn main() -> Int\n  math::renamed() + {value}\nend\n"
            ),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}
