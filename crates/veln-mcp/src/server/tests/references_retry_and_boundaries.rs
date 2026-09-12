use super::references::*;
use super::*;
use std::cell::Cell;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

#[test]
fn references_preserve_unicode_coordinates_and_token_end_exclusion() {
    let workspace = TempWorkspace::new("references-coordinate-boundaries");
    workspace.write(
        "main.veln",
        "fn main() -> Int\r\n  let emoji = \"🙂\"\r\n  main()\r\nend\r\n",
    );

    let selected = references_result(&workspace, "main.veln", 3, 4);
    assert_eq!(selected["isError"], false, "{selected:#}");
    assert_reference_ranges(
        &selected,
        &[("main.veln", 3, 3, 3, 7)],
        "unicode coordinate selection",
    );

    let token_end = references_result(&workspace, "main.veln", 3, 7);
    assert_eq!(token_end["isError"], false, "{token_end:#}");
    assert_eq!(token_end["structuredContent"]["references"], json!([]));

    let invalid = references_result(&workspace, "main.veln", 2, 21);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
}

#[test]
fn references_use_single_file_scope_for_sources_outside_selected_projects() {
    let workspace = TempWorkspace::new("references-single-file");
    workspace.write("app/veln.toml", "");
    workspace.write("app/main.veln", "fn selected() -> Int\n  selected()\nend\n");
    workspace.write("loose.veln", "fn helper() -> Int\n  helper()\nend\n");
    workspace.write("other.veln", "fn helper() -> Int\n  helper()\nend\n");

    let result = references_result(&workspace, "loose.veln", 2, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "loose.veln",
            "project_wide": false
        })
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 1, "{result:#}");
    assert!(
        references[0]["uri"]
            .as_str()
            .unwrap()
            .ends_with("loose.veln"),
        "{result:#}"
    );
    assert_eq!(
        references[0]["range"],
        json!({"start": {"line": 2, "column": 3}, "end": {"line": 2, "column": 9}})
    );
}

#[test]
fn references_do_not_expose_function_shaped_recovery_records() {
    let workspace = TempWorkspace::new("references-recovery-boundary");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "test Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn read() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );

    let result = references_result(&workspace, "main.veln", 6, 4);
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
    assert_eq!(result["structuredContent"]["scope"]["project_wide"], true);
}

#[test]
fn references_report_invalid_positions_and_schema_coordinate_failures() {
    let workspace = TempWorkspace::new("references-invalid-position");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");

    let invalid = references_result(&workspace, "main.veln", 5, 1);
    assert_eq!(invalid["isError"], true, "{invalid:#}");
    assert_eq!(invalid["structuredContent"]["code"], "invalid_position");
    assert!(
        invalid["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );

    let non_integer_request = serde_json::from_str(
        r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "references",
                "arguments": {
                    "source": "main.veln",
                    "line": 2.0000000000000001,
                    "column": 4
                }
            }
        }"#,
    )
    .unwrap();
    let non_integer = initialized_server(&workspace)
        .handle_request(non_integer_request)
        .unwrap();
    assert_eq!(non_integer["error"]["code"], -32602, "{non_integer:#}");
    assert!(non_integer.get("result").is_none(), "{non_integer:#}");
}

#[test]
fn references_reject_paths_and_changed_workspace_identity() {
    let workspace = TempWorkspace::new("references-boundaries");
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    for source in ["../main.veln", "missing.veln", "main.txt"] {
        let result = references_result(&workspace, source, 1, 1);
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{source}"
        );
    }

    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", "fn main() -> Int\n  main()\nend\n");
    let mut server = Server {
        base,
        selection,
        initialized: true,
        language_resources: minimal_language_resources(),
    };
    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));
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

#[test]
fn references_project_capture_exhausts_retries_after_owned_source_changes() {
    let workspace = TempWorkspace::new("references-project-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
        Some("fn helper() -> Int\n  1\nend\n"),
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let _hook = alternate_owned_source_capture_hook(&workspace, attempts.clone());

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

fn alternate_owned_source_capture_hook(
    workspace: &TempWorkspace,
    attempts: Rc<Cell<usize>>,
) -> impl Drop {
    let root = workspace.root.clone();
    crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts.get();
        attempts.set(attempt + 1);
        let main = root.join("main.veln");
        fs::remove_file(&main).unwrap();
        if attempt.is_multiple_of(2) {
            fs::write(
                &main,
                "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  2\nend\n",
            )
            .unwrap();
            fs::remove_file(root.join("helper.veln")).unwrap();
        } else {
            fs::write(
                &main,
                "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
            )
            .unwrap();
            fs::write(root.join("helper.veln"), "fn helper() -> Int\n  1\nend\n").unwrap();
        }
    })
}

#[test]
fn references_project_capture_exhausts_retries_for_workspace_schema_selection() {
    let workspace = TempWorkspace::new("references-workspace-schema-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "schema Packet\n",
            "  value: Int\n",
            "end\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode Packet from view at byte_offset(0)?\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let main = root.join("main.veln");
        fs::remove_file(&main).unwrap();
        let field = if attempt % 2 == 0 { "value" } else { "other" };
        fs::write(
            &main,
            format!(
                "schema Packet\n  {field}: Int\nend\n\nfn read(view: ByteView) -> ()\n  decode Packet from view at byte_offset(0)?\nend\n"
            ),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":8}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
}

#[test]
fn references_project_capture_exhausts_retries_after_dependency_source_changes() {
    let workspace = TempWorkspace::new("references-dependency-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::renamed()\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 2 } else { 1 };
        fs::write(
            &source,
            format!("pub fn value() -> Int\n  {value}\nend\n\npub fn renamed = value\n"),
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

#[test]
fn references_project_capture_exhausts_retries_for_dependency_type_selection() {
    let workspace = TempWorkspace::new("references-dependency-type-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main(input: dep::Item) -> dep::Item\n  input\nend\n",
        None,
    );
    workspace.write("vendor/dep/dep.veln", "pub type Item\nend\n");
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let body = if attempt % 2 == 0 {
            "pub type Item\n  pub Ready(Int)\nend\n"
        } else {
            "pub type Item\nend\n"
        };
        fs::write(&source, body).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":21}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_constructor_selection() {
    let workspace = TempWorkspace::new("references-dependency-constructor-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> dep::Item\n  dep::Item::Ready(1)\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub type Item\n  pub Ready(Int)\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let body = if attempt % 2 == 0 {
            "pub type Item\n  pub Ready(Int)\n  pub Other(Int)\nend\n"
        } else {
            "pub type Item\n  pub Ready(Int)\nend\n"
        };
        fs::write(&source, body).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":15}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_function_alias_selection() {
    let workspace = TempWorkspace::new("references-dependency-alias-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::renamed()\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/dep.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 2 } else { 1 };
        fs::write(
            &source,
            format!("pub fn value() -> Int\n  {value}\nend\n\npub fn renamed = value\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":8}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_standard_library_selection() {
    let workspace = TempWorkspace::new("references-standard-library-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use math from \"std\"\n\nfn main() -> Int\n  math::renamed()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
        )],
    );
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
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
}

#[test]
fn references_return_empty_for_standard_library_function_alias_chain() {
    let workspace = TempWorkspace::new("references-standard-library-alias-chain");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn main() -> Int\n  chain()\nend\n");
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\npub fn chain = renamed\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(result["structuredContent"]["references"], json!([]));
}

#[test]
fn references_anonymous_capture_exhausts_retries_after_requested_source_changes() {
    let workspace = TempWorkspace::new("references-anonymous-capture-retry");
    workspace.write(
        "loose.veln",
        "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("loose.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 2 } else { 1 };
        fs::write(
            &source,
            format!("fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  {value}\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"loose.veln","line":2,"column":4}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
}
