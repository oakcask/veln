use super::*;
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

#[test]
fn references_project_capture_exhausts_retries_after_owned_source_changes() {
    let workspace = TempWorkspace::new("references-project-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "fn main() -> Int\n  value()\nend\n\nfn value() -> Int\n  1\nend\n",
        Some("fn helper() -> Int\n  1\nend\n"),
    );
    let (mut server, before_resources, before_selection) =
        initialized_server_with_captured_state(&workspace);
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        rewrite_owned_sources(&root, attempt);
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":4}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

fn rewrite_owned_sources(root: &Path, attempt: usize) {
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
}

fn initialized_server_with_captured_state(workspace: &TempWorkspace) -> (Server, Value, Value) {
    let mut server = initialized_server(workspace);
    let resources = all_resource_state(&mut server);
    let selection = server.selection_result();
    (server, resources, selection)
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
            "end\n\n",
            "schema Frame\n",
            "  nested: Packet\n",
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
                "schema Packet\n  {field}: Int\nend\n\nfn read(view: ByteView) -> ()\n  decode Packet from view at byte_offset(0)?\nend\n\nschema Frame\n  nested: Packet\nend\n"
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
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\nend\n",
        None,
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
        fs::write(&source, format!("pub fn value() -> Int\n  {value}\nend\n")).unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_schema_selection() {
    let workspace = TempWorkspace::new("references-dependency-schema-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn read(view: ByteView) -> ()\n",
            "  decode dep::Packet from view at byte_offset(0)?\n",
            "end\n",
        ),
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub schema Packet\n  value: Int\nend\n",
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
        let field = if attempt % 2 == 0 { "other" } else { "value" };
        fs::write(&source, format!("pub schema Packet\n  {field}: Int\nend\n")).unwrap();
    });

    let result = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 16
    }));

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
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
            format!("use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::renamed() + {value}\nend\n"),
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
fn references_project_capture_exhausts_retries_for_dependency_schema_alias_selection() {
    let workspace = TempWorkspace::new("references-dependency-schema-alias-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        concat!(
            "use dep from \"example/dep\"\n\n",
            "fn main(view: ByteView) -> ()\n",
            "  decode dep::Alias from view at byte_offset(0)?\n",
            "end\n",
        ),
        None,
    );
    workspace.write(
        "vendor/dep/veln.toml",
        concat!(
            "[package]\nname = \"example/dep\"\n\n",
            "[lib]\nexports = [\"dep.veln\", \"core.veln\"]\n",
        ),
    );
    workspace.write(
        "vendor/dep/dep.veln",
        concat!("use core\n\n", "pub schema Alias = core::Packet\n",),
    );
    workspace.write(
        "vendor/dep/core.veln",
        "pub schema Packet\n  value: Int\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let admitted = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 16
    }));
    assert_eq!(admitted["isError"], false, "{admitted:#}");
    assert!(dependency_resource_is_listed(&mut server, "example/dep"));
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let source = root.join("vendor/dep/core.veln");
        fs::remove_file(&source).unwrap();
        let field = if attempt % 2 == 0 { "other" } else { "value" };
        fs::write(&source, format!("pub schema Packet\n  {field}: Int\nend\n")).unwrap();
    });

    let result = server.references_tool(&json!({
        "source": "main.veln",
        "line": 4,
        "column": 16
    }));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_project_capture_exhausts_retries_for_dependency_type_alias_selection() {
    let workspace = TempWorkspace::new("references-dependency-type-alias-capture-retry");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "use dep from \"example/dep\"\n\nfn main(input: dep::Alias) -> dep::Alias\n  input\nend\n",
        None,
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub type Item\nend\n\npub type Alias = Item\n",
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
        let source = root.join("main.veln");
        fs::remove_file(&source).unwrap();
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            &source,
            format!("use dep from \"example/dep\"\n\nfn main(input: dep::Alias) -> dep::Alias\n  let marker: Int = {value}\n  input\nend\n"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":3,"column":21}));

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
fn references_project_capture_exhausts_retries_for_standard_library_selection() {
    let workspace = TempWorkspace::new("references-standard-library-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use math from \"std\"\n\nfn main() -> Int\n  math::value()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new(
            "math.veln",
            b"pub fn value() -> Int\n  1\nend\n",
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
            format!("use math from \"std\"\n\nfn main() -> Int\n  math::value() + {value}\nend\n"),
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
