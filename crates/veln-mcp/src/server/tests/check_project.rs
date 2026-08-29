use super::*;

#[test]
fn check_project_selection_table_reports_success_and_stable_domain_failures() {
    struct Case {
        name: &'static str,
        files: Vec<(&'static str, &'static str)>,
        arguments: Value,
        expect_error: Option<&'static str>,
        expect_mode: Option<&'static str>,
        expect_project: Option<&'static str>,
    }

    let cases = [
        Case {
            name: "explicit manifest project",
            files: vec![("veln.toml", ""), ("main.veln", clean_source())],
            arguments: json!({"project": "."}),
            expect_error: None,
            expect_mode: Some("project"),
            expect_project: Some("."),
        },
        Case {
            name: "inferred single manifest project",
            files: vec![("app/veln.toml", ""), ("app/main.veln", clean_source())],
            arguments: json!({}),
            expect_error: None,
            expect_mode: Some("project"),
            expect_project: Some("app"),
        },
        Case {
            name: "ambiguous manifest projects",
            files: vec![
                ("zeta/veln.toml", ""),
                ("zeta/main.veln", clean_source()),
                ("alpha/veln.toml", ""),
                ("alpha/main.veln", clean_source()),
            ],
            arguments: json!({}),
            expect_error: Some("project_ambiguous"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "anonymous single source",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"project": ".", "source": "main.veln"}),
            expect_error: None,
            expect_mode: Some("single_file"),
            expect_project: Some("."),
        },
        Case {
            name: "anonymous requires source",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"project": "."}),
            expect_error: Some("source_required"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "unselected project",
            files: vec![("app/veln.toml", ""), ("app/main.veln", clean_source())],
            arguments: json!({"project": "missing"}),
            expect_error: Some("project_not_selected"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "manifest source combination",
            files: vec![("veln.toml", ""), ("main.veln", clean_source())],
            arguments: json!({"project": ".", "source": "main.veln"}),
            expect_error: Some("invalid_query"),
            expect_mode: None,
            expect_project: None,
        },
        Case {
            name: "anonymous without explicit project",
            files: vec![("main.veln", clean_source())],
            arguments: json!({"source": "main.veln"}),
            expect_error: Some("source_required"),
            expect_mode: None,
            expect_project: None,
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        for (path, text) in &case.files {
            workspace.write(path, text);
        }
        let result = check_project_result(&workspace, case.arguments);
        if let Some(code) = case.expect_error {
            assert_eq!(result["isError"], true, "{}", case.name);
            assert_eq!(result["structuredContent"]["code"], code, "{}", case.name);
        } else {
            assert_eq!(result["isError"], false, "{}", case.name);
            assert_eq!(
                result["structuredContent"]["analysis"]["mode"],
                case.expect_mode.unwrap(),
                "{}",
                case.name
            );
            assert_eq!(
                result["structuredContent"]["analysis"]["project"],
                case.expect_project.unwrap(),
                "{}",
                case.name
            );
        }
    }
}

#[test]
fn check_project_does_not_reclassify_selection_before_refresh() {
    let workspace = TempWorkspace::new("selection-fixed-before-refresh");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    fs::remove_file(workspace.path("veln.toml")).unwrap();
    let mut server = Server {
        base,
        selection,
        initialized: true,
    };

    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "."}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn anonymous_check_project_ignores_manifest_added_before_refresh() {
    let workspace = TempWorkspace::new("anonymous-manifest-added-before-refresh");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    workspace.write(
        "veln.toml",
        "[lib]\nexports = [\"main.veln\", \"extra.veln\"]\n",
    );
    workspace.write("extra.veln", mismatch_source());
    let mut server = Server {
        base,
        selection,
        initialized: true,
    };

    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
    assert_eq!(
        result["structuredContent"]["analysis"],
        json!({
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "main.veln",
            "project_wide": false
        })
    );
}

#[test]
fn anonymous_check_project_does_not_expand_companion_named_source() {
    let workspace = TempWorkspace::new("anonymous-companion-shaped-source");
    workspace.write("main.test.veln", "fn companion_entry() -> Int\n  1\nend\n");
    workspace.write("main.veln", mismatch_source());

    let result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "main.test.veln"}),
    );

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic["span"]["file"] != "main.veln" && diagnostic["id"] != "type.mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[cfg(unix)]
#[test]
fn selected_project_root_symlink_replacement_reports_snapshot_changed() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("selected-root-symlink-replacement");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(workspace.path("alpha")).unwrap();
    workspace.write("outside/veln.toml", "");
    workspace.write("outside/main.veln", clean_source());
    symlink(workspace.path("outside"), workspace.path("alpha")).unwrap();

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "alpha"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn selected_project_root_directory_replacement_reports_snapshot_changed() {
    let workspace = TempWorkspace::new("selected-root-directory-replacement");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(workspace.path("alpha")).unwrap();
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": "alpha"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[cfg(unix)]
#[test]
fn anonymous_workspace_base_symlink_replacement_reports_snapshot_changed() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("anonymous-base-symlink-replacement");
    let outside = TempWorkspace::new("anonymous-base-symlink-replacement-outside");
    workspace.write("main.veln", clean_source());
    outside.write("main.veln", mismatch_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(&workspace.root).unwrap();
    symlink(&outside.root, &workspace.root).unwrap();

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn anonymous_workspace_base_directory_replacement_reports_snapshot_changed() {
    let workspace = TempWorkspace::new("anonymous-base-directory-replacement");
    workspace.write("main.veln", clean_source());
    let selection = Selection::discover(&workspace.root).unwrap();
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();

    fs::remove_dir_all(&workspace.root).unwrap();
    workspace.write("main.veln", mismatch_source());

    let mut server = Server {
        base,
        selection,
        initialized: true,
    };
    let result = server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": {"project": ".", "source": "main.veln"}}),
        ))
        .unwrap();

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn check_project_rejects_source_path_boundaries_before_analysis() {
    let workspace = TempWorkspace::new("source-boundaries");
    workspace.write("main.veln", clean_source());
    workspace.write("notes.txt", "not source");
    workspace.mkdir("directory.veln");

    let cases = [
        (
            "absolute",
            json!({"project": ".", "source": workspace.root.join("main.veln").to_string_lossy()}),
        ),
        (
            "escaping",
            json!({"project": ".", "source": "../main.veln"}),
        ),
        ("missing", json!({"project": ".", "source": "missing.veln"})),
        (
            "non regular",
            json!({"project": ".", "source": "directory.veln"}),
        ),
        ("non veln", json!({"project": ".", "source": "notes.txt"})),
    ];

    for (name, arguments) in cases {
        let result = check_project_result(&workspace, arguments);
        assert_eq!(result["isError"], true, "{name}");
        assert_eq!(
            result["structuredContent"]["code"], "invalid_path",
            "{name}"
        );
    }
}

#[cfg(unix)]
#[test]
fn check_project_rejects_symlink_traversing_sources() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("source-symlink");
    workspace.write("real/main.veln", clean_source());
    symlink(workspace.root.join("real"), workspace.root.join("linked")).unwrap();

    let result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "linked/main.veln"}),
    );

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "invalid_path");

    let parent_result = check_project_result(
        &workspace,
        json!({"project": ".", "source": "linked/../real/main.veln"}),
    );
    assert_eq!(parent_result["isError"], true);
    assert_eq!(parent_result["structuredContent"]["code"], "invalid_path");
}

#[cfg(unix)]
#[test]
fn manifest_check_project_does_not_read_symlinked_project_sources() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-source-symlink");
    workspace.write("alpha/veln.toml", "");
    workspace.write("outside/bad.veln", mismatch_source());
    symlink(
        workspace.path("outside/bad.veln"),
        workspace.path("alpha/main.veln"),
    )
    .unwrap();

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[cfg(unix)]
#[test]
fn manifest_check_project_does_not_read_symlinked_project_directories() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-directory-symlink");
    workspace.write("alpha/veln.toml", "");
    workspace.write("outside/bad.veln", mismatch_source());
    symlink(workspace.path("outside"), workspace.path("alpha/linked")).unwrap();

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[cfg(target_os = "linux")]
#[test]
fn manifest_check_project_ignores_symlinked_nested_manifest_boundary() {
    use std::os::unix::fs::symlink;

    let workspace = TempWorkspace::new("manifest-nested-symlink-boundary");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    workspace.mkdir("alpha/nested");
    symlink(
        workspace.path("alpha/veln.toml"),
        workspace.path("alpha/nested/veln.toml"),
    )
    .unwrap();
    workspace.write("alpha/nested/bad.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 1, "by_severity": {"error": 1}, "by_kind": {"type": 1}})
    );
}

#[cfg(not(target_os = "linux"))]
#[test]
fn check_project_fails_closed_without_handle_relative_capture_support() {
    let workspace = TempWorkspace::new("no-handle-relative-capture-support");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", clean_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
}

#[test]
fn manifest_check_project_stops_at_non_utf8_nested_manifest_boundary() {
    let workspace = TempWorkspace::new("manifest-non-utf8-nested-boundary");
    workspace.write("alpha/veln.toml", "");
    workspace.write("alpha/main.veln", clean_source());
    workspace.write_bytes("alpha/nested/veln.toml", b"not utf8: \xff");
    workspace.write("alpha/nested/bad.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "alpha"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

#[test]
fn anonymous_check_project_analyzes_only_the_selected_source() {
    let workspace = TempWorkspace::new("anonymous-isolation");
    workspace.write("clean.veln", clean_source());
    workspace.write("broken.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": ".", "source": "clean.veln"}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
    assert_eq!(
        result["structuredContent"]["analysis"]["project_wide"],
        false
    );
}

#[test]
fn check_project_returns_structured_language_diagnostics_as_successful_tool_result() {
    let workspace = TempWorkspace::new("structured-diagnostics");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", mismatch_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["id"] == "type.mismatch"
                && diagnostic["severity"] == "error"
                && diagnostic["span"]["start"]["line"] == 2
                && diagnostic["span"]["start"]["column"].as_u64().unwrap() >= 3
                && diagnostic.get("details").is_some()
                && diagnostic.get("related").is_some()
        }),
        "{diagnostics:#?}"
    );
    assert_eq!(
        result["structuredContent"]["summary"]["by_severity"]["error"],
        1
    );
}

#[test]
fn check_project_uses_captured_materialized_git_dependency() {
    let workspace = TempWorkspace::new("captured-materialized-git-dependency");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"https://example.invalid/foo.git\"\n",
            "rev = \"abc123\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use foo from \"github.com/oakcask/foo\"\n\n",
            "fn main() -> Int\n",
            "  add_one(1)\n",
            "end\n",
        ),
    );
    let materialized = veln_project::materialized_git_repository_root(
        &workspace.root,
        "https://example.invalid/foo.git",
    );
    let dependency_root = materialized
        .strip_prefix(&workspace.root)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "/");
    workspace.write(
        &format!("{dependency_root}/veln.toml"),
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/foo\"\n\n",
            "[lib]\n",
            "exports = [\"foo.veln\"]\n",
        ),
    );
    workspace.write(
        &format!("{dependency_root}/foo.veln"),
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    assert_eq!(
        result["structuredContent"]["summary"],
        json!({"diagnostic_count": 0, "by_severity": {}, "by_kind": {}})
    );
}

fn check_project_result(workspace: &TempWorkspace, arguments: Value) -> Value {
    let mut server = initialized_server(workspace);
    server
        .call_tool(Some(
            &json!({"name": "check_project", "arguments": arguments}),
        ))
        .unwrap()
}

fn clean_source() -> &'static str {
    "fn main() -> Int\n  1\nend\n"
}

fn mismatch_source() -> &'static str {
    "fn main() -> Int\n  \"bad\"\nend\n"
}

#[test]
fn check_project_keeps_spanless_related_notes_without_panicking() {
    let workspace = TempWorkspace::new("spanless-related");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", integer_literal_source());

    let result = check_project_result(&workspace, json!({"project": "."}));

    assert_eq!(result["isError"], false);
    let diagnostics = result["structuredContent"]["diagnostics"]
        .as_array()
        .unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic["id"] == "parse.integer_literal"
                && diagnostic["related"][0]["message"] == "Accepted integer form: 0 or 1."
                && diagnostic["related"][0].get("span").is_none()
        }),
        "{diagnostics:#?}"
    );
}

fn integer_literal_source() -> &'static str {
    "fn main() -> Int\n  0b102\nend\n"
}
