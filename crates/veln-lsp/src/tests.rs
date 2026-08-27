use super::*;
use std::env;
use veln_diagnostics::Severity;
use veln_diagnostics::{DiagnosticKind, JsonValue};
use veln_project::materialized_git_repository_root;
use veln_source::SourceSpan;

#[test]
fn legend_exposes_standard_types_and_custom_modifiers() {
    let legend = legend();

    assert!(legend.token_types.contains(&"function"));
    assert!(legend.token_types.contains(&"parameter"));
    assert!(legend.token_types.contains(&"namespace"));
    assert!(legend.token_modifiers.contains(&"declaration"));
    assert!(legend.token_modifiers.contains(&"defaultLibrary"));
    assert!(legend.token_modifiers.contains(&"test"));
    assert!(legend.token_modifiers.contains(&"result"));
    assert!(legend.token_modifiers.contains(&"hole"));
}

#[test]
fn full_tokens_are_flat_lsp_integer_data() {
    let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");

    let response = semantic_tokens_full(&source);

    assert_eq!(response.data.len() % 5, 0);
    assert!(response.data.len() >= 10);
}

#[test]
fn server_returns_semantic_tokens_for_handler_clause_satisfy_body() {
    let mut server = Server::default();
    let project = TempProject::new("semantic-handler-satisfy-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => _choice satisfy candidate => candidate == value\n",
            "  fallback() => 0\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&semantic_tokens_request(&main_uri));

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""id":2,"result":{"data":["#));
    assert!(!responses[0].contains(r#""data":[]"#), "{}", responses[0]);
}

#[test]
fn server_initializes_with_semantic_token_capability() {
    let mut server = Server::default();
    let project = TempProject::new("initialize-empty-root");
    let root_uri = path_to_uri(&project.root);

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    assert!(responses[0].contains(r#""semanticTokensProvider""#));
    assert!(responses[0].contains(r#""definitionProvider":true"#));
    assert!(responses[0].contains(r#""renameProvider":{"prepareProvider":true}"#));
    assert!(responses[0].contains(r#""tokenTypes":["namespace","type""#));
    assert_eq!(
        server.workspace_roots.as_slice(),
        std::slice::from_ref(&project.root)
    );
}

#[test]
fn server_uses_anonymous_workspace_root_when_no_manifest_exists() {
    let mut server = Server::default();
    let project = TempProject::new("initialize-workspace-folder");
    let root_uri = path_to_uri(&project.root);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"fixture"}}]}}}}"#
        ));

    assert_eq!(
        server.workspace_roots.as_slice(),
        std::slice::from_ref(&project.root)
    );
}

#[test]
fn server_initializes_all_workspace_roots_from_workspace_folders() {
    let mut server = Server::default();
    let alpha = TempProject::new("initialize-alpha-workspace-folder");
    let beta = TempProject::new("initialize-beta-workspace-folder");
    let alpha_uri = path_to_uri(&alpha.root);
    let beta_uri = path_to_uri(&beta.root);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alpha_uri}","name":"alpha"}},{{"uri":"{beta_uri}","name":"beta"}}]}}}}"#
        ));

    let mut expected = vec![alpha.root.clone(), beta.root.clone()];
    expected.sort();
    assert_eq!(server.workspace_roots, expected);
}

#[test]
fn server_stops_workspace_root_selection_at_manifest_root() {
    let mut server = Server::default();
    let workspace = TempProject::new("manifest-workspace-root");
    workspace.write("veln.toml", "[package]\nname = \"outer\"\n");
    workspace.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    workspace.write("nested/main.veln", "pub fn nested() -> Int\n  1\nend\n");
    let root_uri = path_to_uri(&workspace.root);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
}

#[test]
fn server_selects_first_manifest_root_on_each_workspace_branch() {
    let mut server = Server::default();
    let workspace = TempProject::new("manifest-roots-on-branches");
    workspace.write("alpha/package/veln.toml", "[package]\nname = \"alpha\"\n");
    workspace.write(
        "alpha/package/nested/veln.toml",
        "[package]\nname = \"alpha-nested\"\n",
    );
    workspace.write(
        "beta/deep/package/veln.toml",
        "[package]\nname = \"beta\"\n",
    );
    let root_uri = path_to_uri(&workspace.root);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

    assert_eq!(
        server.workspace_roots,
        vec![
            workspace.root.join("alpha/package"),
            workspace.root.join("beta/deep/package"),
        ]
    );
}

#[test]
fn server_keeps_explicit_outer_and_nested_workspace_projects() {
    let mut server = Server::default();
    let workspace = TempProject::new("explicit-outer-and-nested-roots");
    workspace.write("veln.toml", "[package]\nname = \"outer\"\n");
    workspace.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    let outer_uri = path_to_uri(&workspace.root);
    let nested_uri = path_to_uri(&workspace.root.join("nested"));

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{outer_uri}","name":"outer"}},{{"uri":"{nested_uri}","name":"nested"}},{{"uri":"{nested_uri}","name":"nested-again"}}]}}}}"#
        ));

    assert_eq!(
        server.workspace_roots,
        vec![workspace.root.clone(), workspace.root.join("nested")]
    );
}

#[test]
fn server_does_not_initialize_loaded_dependency_as_workspace_project() {
    let mut server = Server::default();
    let workspace = TempProject::new("dependency-workspace-isolation");
    workspace.write(
        "veln.toml",
        "[package]\nname = \"app\"\n\n[dependencies.\"example.com/lib\"]\npath = \"vendor/lib\"\n",
    );
    workspace.write(
        "app.veln",
        "use lib from \"example.com/lib\"\n\nfn main() -> Int\n  add_one(1)\nend\n",
    );
    workspace.write(
        "vendor/lib/veln.toml",
        "[package]\nname = \"example.com/lib\"\n\n[lib]\nexports = [\"lib.veln\"]\n",
    );
    workspace.write(
        "vendor/lib/lib.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );
    let root_uri = path_to_uri(&workspace.root);
    let app_uri = path_to_uri(&workspace.root.join("app.veln"));
    let dependency_uri = path_to_uri(&workspace.root.join("vendor/lib/lib.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"app"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(!publish.contains("module.missing_identity"), "{publish}");
    assert!(
        responses
            .iter()
            .all(|response| !response.contains(&dependency_uri)),
        "dependency sources must not be published as a workspace project"
    );
}

#[cfg(unix)]
#[test]
fn server_deduplicates_workspace_folders_by_filesystem_identity() {
    use std::os::unix::fs::symlink;

    let mut server = Server::default();
    let workspace = TempProject::new("workspace-filesystem-identity");
    workspace.write("package/veln.toml", "[package]\nname = \"package\"\n");
    symlink(workspace.root.join("package"), workspace.root.join("alias"))
        .expect("workspace alias should be created");
    let package_uri = path_to_uri(&workspace.root.join("package"));
    let alias_uri = path_to_uri(&workspace.root.join("alias"));

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alias_uri}","name":"alias"}},{{"uri":"{package_uri}","name":"package"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![workspace.root.join("package")]);
}

#[cfg(unix)]
#[test]
fn server_keeps_symlink_workspace_alias_documents_in_project() {
    use std::os::unix::fs::symlink;

    let mut server = Server::default();
    let workspace = TempProject::new("workspace-alias-document-identity");
    workspace.write("package/veln.toml", "[package]\nname = \"package\"\n");
    workspace.write(
        "package/math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  increment(value - 1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "package/math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "test increment_test() -> Int\n",
            "  math::increment(1)\n",
            "end\n",
        ),
    );
    workspace.write("package/main.veln", "pub fn main() -> Int\n  1\nend\n");
    symlink(workspace.root.join("package"), workspace.root.join("alias"))
        .expect("workspace alias should be created");
    let alias_uri = path_to_uri(&workspace.root.join("alias"));
    let alias_main_uri = path_to_uri(&workspace.root.join("alias/main.veln"));
    let alias_math_uri = path_to_uri(&workspace.root.join("alias/math.veln"));
    let alias_companion_uri = path_to_uri(&workspace.root.join("alias/math.test.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alias_uri}","name":"alias"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![workspace.root.join("package")]);
    let publish = publish_for_uri(&responses, &alias_main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{alias_main_uri}","text":"pub fn main() -> Int\n  \"bad\"\nend\n"}}}}}}"#
        ));
    let publish = publish_for_uri(&responses, &alias_main_uri);
    assert!(publish.contains(r#""code":"type.mismatch""#), "{publish}");

    let responses = server.handle_message(&semantic_tokens_request(&alias_main_uri));
    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""id":2,"result":{"data":["#));
    assert!(!responses[0].contains(r#""data":[]"#), "{}", responses[0]);

    let responses = server.handle_message(&definition_request(&alias_companion_uri, 7, 10));
    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(&escape_json(&alias_math_uri)),
        "{}",
        responses[0]
    );
}

#[cfg(unix)]
#[test]
fn server_does_not_follow_directory_symlinks_during_manifest_discovery() {
    use std::os::unix::fs::symlink;

    let mut server = Server::default();
    let workspace = TempProject::new("workspace-directory-symlink");
    workspace.write("folder/readme.txt", "workspace without a manifest\n");
    workspace.write("linked-package/veln.toml", "[package]\nname = \"linked\"\n");
    symlink(
        workspace.root.join("linked-package"),
        workspace.root.join("folder/package-link"),
    )
    .expect("directory symlink should be created");
    let folder = workspace.root.join("folder");
    let folder_uri = path_to_uri(&folder);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{folder_uri}","name":"folder"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![folder]);
}

#[test]
fn server_excludes_git_directories_from_manifest_discovery() {
    let mut server = Server::default();
    let workspace = TempProject::new("workspace-git-exclusion");
    workspace.write(
        ".git/generated/veln.toml",
        "[package]\nname = \"generated\"\n",
    );
    let root_uri = path_to_uri(&workspace.root);

    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

    assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
}

#[test]
fn server_uses_nested_manifest_roots_from_workspace_folders() {
    let mut server = Server::default();
    let workspace = TempProject::new("nested-manifest-workspace-folder");
    workspace.write(
        "examples/specification/check/external-package-imports/veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    workspace.write(
        "examples/specification/check/external-package-imports/app.veln",
        "use foo from \"github.com/oakcask/foo\"\n\nfn main() -> Int\n  add_one(1)\nend\n",
    );
    workspace.write(
        "examples/specification/check/external-package-imports/vendor/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    workspace.write(
        "examples/specification/check/external-package-imports/vendor/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );
    let root_uri = path_to_uri(&workspace.root);
    let app_uri = path_to_uri(
        &workspace
            .root
            .join("examples/specification/check/external-package-imports/app.veln"),
    );

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

    assert_eq!(
        server.workspace_roots,
        vec![
            workspace
                .root
                .join("examples/specification/check/external-package-imports")
        ]
    );
    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(!publish.contains("module.missing_identity"), "{publish}");
}

#[test]
fn server_uses_nested_manifest_roots_below_target_workspace_directories() {
    let mut server = Server::default();
    let workspace = TempProject::new("nested-manifest-target-workspace-folder");
    workspace.write(
        "target/generated-package/veln.toml",
        "[package]\nname = \"generated\"\n",
    );
    workspace.write(
        "target/generated-package/main.veln",
        "pub fn generated() -> Int\n  1\nend\n",
    );
    let root_uri = path_to_uri(&workspace.root);
    let generated_uri = path_to_uri(&workspace.root.join("target/generated-package/main.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

    assert_eq!(
        server.workspace_roots,
        vec![workspace.root.join("target/generated-package")]
    );
    let publish = publish_for_uri(&responses, &generated_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
}

#[test]
fn server_does_not_infer_workspace_root_without_client_identity() {
    let mut server = Server::default();

    let responses = server.handle_message(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
    );

    assert_eq!(server.workspace_roots, Vec::<PathBuf>::new());
    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""semanticTokensProvider""#));
}

#[test]
fn server_publishes_unopened_workspace_file_diagnostics() {
    let mut server = Server::default();
    let project = TempProject::new("unopened-workspace-diagnostics");
    project.write("broken.veln", "fn broken() -> Int\n  missing\nend\n");
    let root_uri = path_to_uri(&project.root);
    let broken_uri = path_to_uri(&project.root.join("broken.veln"));

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let publish = publish_for_uri(&responses, &broken_uri);
    assert!(publish.contains(r#""code":"name.unresolved""#), "{publish}");
}

#[test]
fn server_uses_unsaved_workspace_text_over_disk_text() {
    let mut server = Server::default();
    let project = TempProject::new("unsaved-workspace-overlay");
    project.write("main.veln", "fn main() -> Int\n  missing\nend\n");
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn main() -> Int\n  1\nend\n"}}}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(!publish.contains("name.unresolved"), "{publish}");
}

#[test]
fn selected_snapshot_invalid_casing_publishes_and_excludes_symbol() {
    let mut server = Server::default();
    let project = TempProject::new("snapshot-invalid-casing");
    project.write(
        "main.veln",
        concat!(
            "fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn caller() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));

    let responses = server.handle_message(&initialize_request(&root_uri));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(
        publish.contains(r#""code":"name.invalid_case""#),
        "{publish}"
    );
    let declaration = server.handle_message(&definition_request(&main_uri, 0, 3));
    assert!(
        declaration[0].contains(r#""result":null"#),
        "{}",
        declaration[0]
    );
    let call = server.handle_message(&definition_request(&main_uri, 5, 2));
    assert!(call[0].contains(r#""result":null"#), "{}", call[0]);
    let references = server.handle_message(&references_request(&main_uri, 0, 3));
    assert!(
        references[0].contains(r#""result":[]"#),
        "{}",
        references[0]
    );
    let rename = server.handle_message(&rename_request(&main_uri, 0, 3, "renamed"));
    assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
}

#[test]
fn selected_overlay_invalid_casing_replaces_saved_symbol() {
    let mut server = Server::default();
    let project = TempProject::new("overlay-invalid-casing");
    project.write(
        "main.veln",
        concat!(
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn caller() -> Int\n",
            "  good()\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn Bad() -> Int\n  1\nend\n\nfn caller() -> Int\n  Bad()\nend\n"}}}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(
        publish.contains(r#""code":"name.invalid_case""#),
        "{publish}"
    );
    let call = server.handle_message(&definition_request(&main_uri, 5, 2));
    assert!(call[0].contains(r#""result":null"#), "{}", call[0]);
}

#[test]
fn unselected_workspace_invalid_casing_does_not_publish_or_index() {
    let mut server = Server::default();
    let selected = TempProject::new("selected-valid-casing-root");
    let unselected = TempProject::new("unselected-invalid-casing-root");
    selected.write("main.veln", "fn good() -> Int\n  good()\nend\n");
    unselected.write("main.veln", "fn Bad() -> Int\n  Bad()\nend\n");
    let selected_root_uri = path_to_uri(&selected.root);
    let selected_main_uri = path_to_uri(&selected.root.join("main.veln"));
    let unselected_main_uri = path_to_uri(&unselected.root.join("main.veln"));

    let responses = server.handle_message(&initialize_request(&selected_root_uri));

    let publish = publish_for_uri(&responses, &selected_main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(
        responses
            .iter()
            .all(|response| !response.contains(&unselected_main_uri)),
        "{responses:#?}"
    );
    let definition = server.handle_message(&definition_request(&selected_main_uri, 1, 2));
    assert!(
        definition[0].contains(r#""result":{"uri":"file://"#),
        "{}",
        definition[0]
    );
    assert!(
        !definition[0].contains(&escape_json(&unselected_main_uri)),
        "{}",
        definition[0]
    );
}

#[test]
fn server_does_not_overlay_open_documents_owned_by_nested_manifest() {
    let mut server = Server::default();
    let project = TempProject::new("nested-open-document-overlay-boundary");
    project.write(
        "veln.toml",
        "[package]\nname = \"outer\"\n\n[lib]\nexports = [\"app.veln\", \"nested/hidden.veln\"]\n",
    );
    project.write("app.veln", "pub fn app() -> Int\n  1\nend\n");
    project.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    project.write("nested/hidden.veln", "pub fn hidden() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let manifest_uri = path_to_uri(&project.root.join("veln.toml"));
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{nested_uri}","text":"pub fn hidden() -> Int\n  2\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{app_uri}","version":2}},"contentChanges":[{{"text":"pub fn app() -> Int\n  1\nend\n"}}]}}}}"#
        ));

    let publish = publish_for_uri(&responses, &manifest_uri);
    assert!(
        publish.contains(r#""code":"manifest.unselected_export""#),
        "{publish}"
    );
}

#[test]
fn server_clears_stale_workspace_diagnostics_after_change() {
    let mut server = Server::default();
    let project = TempProject::new("workspace-diagnostics-change-clear");
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn main() -> Int\n  missing\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{main_uri}","version":2}},"contentChanges":[{{"text":"fn main() -> Int\n  1\nend\n"}}]}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
}

#[test]
fn server_analysis_respects_manifest_boundaries_and_owned_target_sources() {
    let mut server = Server::default();
    let project = TempProject::new("manifest-boundary-analysis");
    project.write("veln.toml", "[package]\nname = \"outer\"\n");
    project.write("app.veln", "pub fn app() -> Int\n\t1\nend\n");
    project.write("target/owned.veln", "pub fn owned() -> Int\n\t2\nend\n");
    project.write("nested/veln.toml", "malformed nested manifest");
    project.write("nested/hidden.veln", "this source must not be parsed");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let target_uri = path_to_uri(&project.root.join("target/owned.veln"));
    let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));

    assert!(publish_for_uri(&responses, &app_uri).contains(r#""diagnostics":[]"#));
    assert!(publish_for_uri(&responses, &target_uri).contains(r#""diagnostics":[]"#));
    assert!(
        responses
            .iter()
            .all(|response| !response.contains(&nested_uri)),
        "nested package source should not receive outer-project diagnostics"
    );
}

#[test]
fn server_clears_workspace_diagnostics_when_file_leaves_discovery() {
    let mut server = Server::default();
    let project = TempProject::new("workspace-diagnostics-left-discovery");
    project.write("main.veln", "fn main() -> Int\n  missing\nend\n");
    let root_uri = path_to_uri(&project.root);
    let main_path = project.root.join("main.veln");
    let main_uri = path_to_uri(&main_path);
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));
    fs::remove_file(main_path).expect("fixture source should be removable");

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{main_uri}"}}}}}}"#
        ));

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
}

#[test]
fn server_reports_cross_file_workspace_diagnostics() {
    let mut server = Server::default();
    let project = TempProject::new("cross-file-workspace-diagnostics");
    project.write(
        "app.veln",
        "use math\n\nfn main() -> Int\n  double(\"bad\")\nend\n",
    );
    project.write(
        "math.veln",
        "pub fn double(value: Int) -> Int\n  value * 2\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""code":"type.mismatch""#), "{publish}");
}

#[test]
fn server_publishes_same_leaf_files_from_multiple_roots_separately() {
    let mut server = Server::default();
    let alpha = TempProject::new("same-leaf-alpha-root");
    let beta = TempProject::new("same-leaf-beta-root");
    alpha.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
    beta.write("main.veln", "pub fn main() -> Int\n  \"bad\"\nend\n");
    let alpha_root_uri = path_to_uri(&alpha.root);
    let beta_root_uri = path_to_uri(&beta.root);
    let alpha_main_uri = path_to_uri(&alpha.root.join("main.veln"));
    let beta_main_uri = path_to_uri(&beta.root.join("main.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alpha_root_uri}","name":"alpha"}},{{"uri":"{beta_root_uri}","name":"beta"}}]}}}}"#
        ));

    let alpha_publish = publish_for_uri(&responses, &alpha_main_uri);
    let beta_publish = publish_for_uri(&responses, &beta_main_uri);
    assert!(
        alpha_publish.contains(r#""diagnostics":[]"#),
        "{alpha_publish}"
    );
    assert!(
        beta_publish.contains(r#""code":"type.mismatch""#),
        "{beta_publish}"
    );
}

#[test]
fn server_keeps_same_leaf_workspace_files_in_distinct_modules() {
    let mut server = Server::default();
    let project = TempProject::new("same-leaf-workspace-diagnostics");
    project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
    project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));

    let responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let publish = publish_for_uri(&responses, &app_uri);
    assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    assert!(
        !publish.contains("module.duplicate_source_path"),
        "{publish}"
    );
}

#[test]
fn server_overlays_same_leaf_workspace_files_by_relative_path() {
    let mut server = Server::default();
    let project = TempProject::new("same-leaf-workspace-overlay");
    project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
    project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
    let root_uri = path_to_uri(&project.root);
    let app_uri = path_to_uri(&project.root.join("app.veln"));
    let beta_uri = path_to_uri(&project.root.join("beta/item.veln"));
    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    ));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{beta_uri}","text":"pub fn value() -> String\n  \"two\"\nend\n"}}}}}}"#
        ));

    let app_publish = publish_for_uri(&responses, &app_uri);
    let beta_publish = publish_for_uri(&responses, &beta_uri);
    assert!(
        app_publish.contains(r#""code":"type.mismatch""#),
        "{app_publish}"
    );
    assert!(
        beta_publish.contains(r#""diagnostics":[]"#),
        "{beta_publish}"
    );
    assert!(
        !app_publish.contains("module.duplicate_source_path"),
        "{app_publish}"
    );
}

#[test]
fn companion_private_function_definition_returns_target_declaration() {
    let mut server = Server::default();
    let project = companion_private_function_project("definition");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&definition_request(&companion_uri, 7, 10));

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(&format!(r#""uri":"{}""#, escape_json(&math_uri))));
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_prepare_rename_returns_reference_leaf() {
    let mut server = Server::default();
    let project = companion_private_function_project("prepare-rename");
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&prepare_rename_request(&companion_uri, 7, 10));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(
            r#""result":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_edits_target_and_matching_companion_references() {
    let mut server = Server::default();
    let project = companion_private_function_project("rename");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(&format!(r#""{}":["#, escape_json(&math_uri))),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(&format!(r#""{}":["#, escape_json(&companion_uri))),
        "{}",
        responses[0]
    );
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn rename_accepts_same_class_replacements_for_cased_symbols() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-success");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "effect Choose\n",
            "  pick(value: Bool) -> Bool\n",
            "end\n\n",
            "handler choose() handles Choose\n",
            "  pick(value) => value\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let type_rename = server.handle_message(&rename_request(&main_uri, 0, 5, "Entry"));
    let constructor_rename = server.handle_message(&rename_request(&main_uri, 1, 2, "Created"));
    let function_rename = server.handle_message(&rename_request(&main_uri, 12, 3, "adapt"));
    let value_rename = server.handle_message(&rename_request(&main_uri, 9, 7, "input"));

    assert_eq!(type_rename[0].matches(r#""newText":"Entry""#).count(), 3);
    assert_eq!(
        constructor_rename[0]
            .matches(r#""newText":"Created""#)
            .count(),
        2
    );
    assert_eq!(
        function_rename[0].matches(r#""newText":"adapt""#).count(),
        1
    );
    assert_eq!(value_rename[0].matches(r#""newText":"input""#).count(), 2);
}

#[test]
fn rename_rejects_class_changing_replacements_for_cased_symbols() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-invalid-case");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "effect Choose\n",
            "  pick(value: Bool) -> Bool\n",
            "end\n\n",
            "handler choose() handles Choose\n",
            "  pick(value) => value\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let cases = [
        (0, 5, "entry", "type", "ascii_uppercase"),
        (1, 2, "created", "constructor", "ascii_uppercase"),
        (12, 3, "Adapt", "function", "ascii_lowercase"),
        (9, 7, "Input", "value_binding", "ascii_lowercase"),
    ];
    for (line, character, new_name, symbol_class, required_initial) in cases {
        let responses =
            server.handle_message(&rename_request(&main_uri, line, character, new_name));

        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(r#""code":-32602"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(r#""code":"rename.invalid_case""#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""symbol_class":"{symbol_class}""#)),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""requested_name":"{new_name}""#)),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""required_initial":"{required_initial}""#)),
            "{}",
            responses[0]
        );
        assert!(!responses[0].contains(r#""changes""#), "{}", responses[0]);
    }
}

#[test]
fn rename_excludes_same_named_non_type_namespace_tokens() {
    let mut server = Server::default();
    let project = TempProject::new("rename-cased-symbol-namespace-boundary");
    project.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "schema Item\n",
            "  format binary\n",
            "  value: UInt8\n",
            "end\n\n",
            "effect Item\n",
            "  Item() -> Int\n",
            "end\n\n",
            "fn main(input: Item) -> Item\n",
            "  input\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(4, 7), (9, 7), (10, 2)] {
        let prepare_rename =
            server.handle_message(&prepare_rename_request(&main_uri, line, character));
        assert_eq!(prepare_rename.len(), 1);
        assert!(
            prepare_rename[0].contains(r#""result":null"#),
            "{}",
            prepare_rename[0]
        );

        let rename = server.handle_message(&rename_request(&main_uri, line, character, "Entry"));
        assert_eq!(rename.len(), 1);
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
        assert!(
            !rename[0].contains(r#""rename.invalid_case""#),
            "{}",
            rename[0]
        );
    }

    let type_rename = server.handle_message(&rename_request(&main_uri, 13, 15, "Entry"));
    assert_eq!(type_rename[0].matches(r#""newText":"Entry""#).count(), 3);
    assert!(
        !type_rename[0].contains(r#""line":4,"character":7"#),
        "{}",
        type_rename[0]
    );
    assert!(
        !type_rename[0].contains(r#""line":9,"character":7"#),
        "{}",
        type_rename[0]
    );
    assert!(
        !type_rename[0].contains(r#""line":10,"character":2"#),
        "{}",
        type_rename[0]
    );
}

#[test]
fn type_rename_requires_unique_visible_type_identity() {
    let mut server = Server::default();
    let project = TempProject::new("rename-type-semantic-identity");
    project.write("left.veln", "pub type Item\n  Left\nend\n");
    project.write("right.veln", "pub type Item\n  Right\nend\n");
    project.write(
        "main.veln",
        concat!(
            "use left\n",
            "use right\n",
            "\n",
            "fn bare(value: Item) -> Item\n",
            "  value\n",
            "end\n",
            "\n",
            "fn left_value(value: left::Item) -> left::Item\n",
            "  value\n",
            "end\n",
            "\n",
            "fn right_value(value: right::Item) -> right::Item\n",
            "  value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let ambiguous_prepare = server.handle_message(&prepare_rename_request(&main_uri, 3, 15));
    let ambiguous_rename = server.handle_message(&rename_request(&main_uri, 3, 15, "Entry"));
    let qualified_rename = server.handle_message(&rename_request(&main_uri, 11, 29, "RightEntry"));

    assert_eq!(ambiguous_prepare.len(), 1);
    assert!(
        ambiguous_prepare[0].contains(r#""result":null"#),
        "{}",
        ambiguous_prepare[0]
    );
    assert_eq!(ambiguous_rename.len(), 1);
    assert!(
        ambiguous_rename[0].contains(r#""changes":{}"#),
        "{}",
        ambiguous_rename[0]
    );
    assert_eq!(qualified_rename.len(), 1);
    assert_eq!(
        qualified_rename[0]
            .matches(r#""newText":"RightEntry""#)
            .count(),
        3
    );
    assert!(
        qualified_rename[0].contains("right.veln"),
        "{}",
        qualified_rename[0]
    );
    assert!(
        !qualified_rename[0].contains("left.veln"),
        "{}",
        qualified_rename[0]
    );
    assert!(
        !qualified_rename[0].contains(r#""line":3,"character":15"#),
        "{}",
        qualified_rename[0]
    );
}

#[test]
fn companion_private_function_rename_preserves_target_symbol_identity() {
    let mut server = Server::default();
    let project = TempProject::new("rename-target-identity");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  increment(value)\n",
            "  increment\n",
            "end\n",
            "\n",
            "fn apply(increment: fn(Int) -> Int) -> Int\n",
            "  increment(1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  math::increment\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":2,"character":2},"end":{"line":2,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":4,"character":8"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_unrelated_text_and_qualified_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-source-isolation");
    project.write(
        "math.veln",
        concat!(
            "use support\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  increment(value)\n",
            "  support::increment(value)\n",
            "  \"increment(1)\"\n",
            "  value\n",
            "end\n",
        ),
    );
    project.write(
        "support.veln",
        "pub fn increment(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  increment(1)\n",
            "  math::increment\n",
            "  \"math::increment(2)\"\n",
            "  # math::increment(3)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":2,"character":3},"end":{"line":2,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":3,"character":2},"end":{"line":3,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":9,"character":8"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":4,"character":11"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":3"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":10,"character":9"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":10"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_target_references_after_nested_blocks() {
    let mut server = Server::default();
    let project = TempProject::new("rename-nested-target-blocks");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn use_nested(value: Int) -> Int\n",
            "  if value > 0\n",
            "    increment(value)\n",
            "  else\n",
            "    0\n",
            "  end\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":4},"end":{"line":6,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":10,"character":2},"end":{"line":10,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_local_callable_bindings() {
    let mut server = Server::default();
    let project = TempProject::new("rename-local-callable-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int, identity: fn(Int) -> Int) -> Int\n",
            "  increment(value)\n",
            "  let increment = identity\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":2},"end":{"line":5,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":6"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_unannotated_callable_parameter_shadow() {
    let mut server = Server::default();
    let project = TempProject::new("rename-unannotated-callable-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int, increment) -> Int\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        !responses[0].contains(r#""line":5,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_limits_pattern_binding_shadow_to_branch() {
    let mut server = Server::default();
    let project = TempProject::new("rename-pattern-binding-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn branch(value: Int, identity: fn(Int) -> Int) -> Int\n",
            "  if value > 0\n",
            "    let {callback: increment} = {callback: identity}\n",
            "    increment(value)\n",
            "  else\n",
            "    increment(value)\n",
            "  end\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        !responses[0].contains(r#""line":6,"character":20"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":4"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":9,"character":4},"end":{"line":9,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":11,"character":2},"end":{"line":11,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_record_fields() {
    let mut server = Server::default();
    let project = TempProject::new("rename-record-field-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn inspect(value: Int) -> Int\n",
            "  let record = {increment: value}\n",
            "  record.increment\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        !responses[0].contains(r#""line":5,"character":16"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":9"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_same_named_let_initializer_reference() {
    let mut server = Server::default();
    let project = TempProject::new("rename-let-initializer-shadow");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn apply(value: Int) -> Int\n",
            "  let increment = increment\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":18},"end":{"line":5,"character":27}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":6"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":6,"character":2"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_match_arm_pattern_bindings() {
    let mut server = Server::default();
    let project = TempProject::new("rename-match-arm-pattern-shadow");
    project.write(
        "math.veln",
        concat!(
            "type Choice\n",
            "  Use {callback: fn(Int) -> Int}\n",
            "  Skip\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(choice: Choice, value: Int) -> Int\n",
            "  match choice\n",
            "    Use {callback: increment} => increment(value)\n",
            "    Skip => increment(value)\n",
            "  end\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        !responses[0].contains(r#""line":11,"character":19"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":11,"character":33"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":12},"end":{"line":12,"character":21}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_keeps_target_references_after_else_if() {
    let mut server = Server::default();
    let project = TempProject::new("rename-else-if-target-blocks");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(value: Int) -> Int\n",
            "  if value == 0\n",
            "    0\n",
            "  else if value == 1\n",
            "    increment(value)\n",
            "  else\n",
            "    2\n",
            "  end\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":4},"end":{"line":8,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":2},"end":{"line":12,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_rejects_suffix_qualified_references() {
    let mut server = Server::default();
    let project = TempProject::new("rename-qualified-path-boundary");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "other/math.veln",
        "pub fn increment(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "use other::math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  other::math::increment(1)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 4, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":4,"character":8},"end":{"line":4,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":15"#),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_result_binding_contract_scope() {
    let mut server = Server::default();
    let project = TempProject::new("rename-result-binding-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> increment: Int\n",
            "  ensure increment >= value\n",
            "  increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        !responses[0].contains(r#""line":0,"character":28"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":1,"character":9"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":2,"character":2},"end":{"line":2,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_skips_satisfy_candidate_scope() {
    let mut server = Server::default();
    let project = TempProject::new("rename-satisfy-candidate-isolation");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn choose(fallback: Int) -> Int\n",
            "  _choice satisfy increment => increment > 0\n",
            "  increment(fallback)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        !responses[0].contains(r#""line":5,"character":19"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":32"#),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":2},"end":{"line":6,"character":11}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_includes_handler_operation_clause_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-call");
    project.write(
        "math.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => increment(value)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":9,"character":19},"end":{"line":9,"character":28}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_rename_from_multiline_clause_call_covers_clause_body_calls() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-multiline-call");
    project.write(
        "math.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => if value == 0\n",
            "    increment(value)\n",
            "  else\n",
            "    increment(value + 1)\n",
            "  end\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 10, 6, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":10,"character":4},"end":{"line":10,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":12,"character":4},"end":{"line":12,"character":13}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_skips_record_fields() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-field-isolation");
    project.write(
        "main.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust() handles Adjust\n",
            "  amount(value) => { value: value, other: 1 }.value + value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 5, 10, "amount_value"));

    assert_eq!(responses.len(), 1);
    assert_eq!(
        responses[0].matches(r#""newText":"amount_value""#).count(),
        3
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":9},"end":{"line":5,"character":14}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":28},"end":{"line":5,"character":33}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":54},"end":{"line":5,"character":59}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":21"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":5,"character":46"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_covers_multiline_body_references() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-multiline-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => match value\n",
            "    true => value\n",
            "    value => value\n",
            "    false => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 5, 8, "input"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 4);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":6,"character":12},"end":{"line":6,"character":17}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":13},"end":{"line":8,"character":18}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":4"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":7,"character":13"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_rename_keeps_else_if_body_scope_bounded() {
    let mut server = Server::default();
    let project = TempProject::new("rename-handler-operation-clause-else-if-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => if value == 0\n",
            "    value\n",
            "  else if value == 1\n",
            "    value\n",
            "  else\n",
            "    value\n",
            "  end\n",
            "  fallback(value) => value\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&main_uri, 6, 8, "input"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 6);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":8,"character":10},"end":{"line":8,"character":15}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":11,"character":4},"end":{"line":11,"character":9}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":13,"character":11"#),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":13,"character":21"#),
        "{}",
        responses[0]
    );
}

#[test]
fn handler_operation_clause_binding_definition_uses_multiline_body_scope() {
    let mut server = Server::default();
    let project = TempProject::new("definition-handler-operation-clause-multiline-body");
    project.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
            "  pick(value) => match value\n",
            "    true => value\n",
            "    value => value\n",
            "    false => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&definition_request(&main_uri, 8, 15));

    assert_eq!(responses.len(), 1);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":5,"character":7},"end":{"line":5,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    let shadowed = server.handle_message(&definition_request(&main_uri, 7, 15));
    assert_eq!(shadowed.len(), 1);
    assert!(shadowed[0].contains(r#""result":null"#), "{}", shadowed[0]);
}

#[test]
fn handler_context_callable_binding_shadows_top_level_function_in_clause_body() {
    let mut server = Server::default();
    let project = TempProject::new("handler-context-callable-binding");
    project.write(
        "main.veln",
        concat!(
            "fn callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "  echo(value: Int) -> Int\n",
            "  reset(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
            "  amount(value) => callback(value)\n",
            "  echo(value) => callback(value) + callback(1)\n",
            "  reset(callback) => callback\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 11, 21));
    let references = server.handle_message(&references_request(&main_uri, 10, 17));
    let context_rename = server.handle_message(&rename_request(&main_uri, 10, 17, "project"));
    let clause_rename = server.handle_message(&rename_request(&main_uri, 13, 8, "value"));

    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":10,"character":15},"end":{"line":10,"character":23}}"#
        ),
        "{}",
        definition[0]
    );
    assert_eq!(references.len(), 1);
    assert!(
        references[0].contains(
            r#""range":{"start":{"line":11,"character":19},"end":{"line":11,"character":27}}"#
        ),
        "{}",
        references[0]
    );
    assert!(
        references[0].contains(
            r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#
        ),
        "{}",
        references[0]
    );
    assert!(
        references[0].contains(
            r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#
        ),
        "{}",
        references[0]
    );
    assert!(
        !references[0].contains(r#""line":0,"character":3"#),
        "{}",
        references[0]
    );
    assert!(
        !references[0].contains(r#""line":13,"character":7"#),
        "{}",
        references[0]
    );
    assert_eq!(context_rename.len(), 1);
    assert_eq!(
        context_rename[0].matches(r#""newText":"project""#).count(),
        4
    );
    assert!(
        context_rename[0].contains(
            r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#
        ),
        "{}",
        context_rename[0]
    );
    assert!(
        context_rename[0].contains(
            r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#
        ),
        "{}",
        context_rename[0]
    );
    assert!(
        !context_rename[0].contains(r#""line":13,"character":7"#),
        "{}",
        context_rename[0]
    );
    assert_eq!(clause_rename.len(), 1);
    assert_eq!(clause_rename[0].matches(r#""newText":"value""#).count(), 2);
    assert!(
        clause_rename[0].contains(
            r#""range":{"start":{"line":13,"character":8},"end":{"line":13,"character":16}}"#
        ),
        "{}",
        clause_rename[0]
    );
    assert!(
        clause_rename[0].contains(
            r#""range":{"start":{"line":13,"character":21},"end":{"line":13,"character":29}}"#
        ),
        "{}",
        clause_rename[0]
    );
}

#[test]
fn handler_context_parameter_does_not_bind_same_named_operation_heading() {
    let mut server = Server::default();
    let project = TempProject::new("handler-context-operation-heading-isolation");
    project.write(
        "main.veln",
        concat!(
            "fn callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "effect Adjust\n",
            "  callback(value: Int) -> Int\n",
            "  amount(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
            "  callback(value) => callback(value)\n",
            "  amount(value) => callback(value)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 10, 4));
    let references = server.handle_message(&references_request(&main_uri, 10, 4));
    let rename = server.handle_message(&rename_request(&main_uri, 10, 4, "project"));
    let body_definition = server.handle_message(&definition_request(&main_uri, 10, 22));

    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(r#""result":null"#),
        "{}",
        definition[0]
    );
    assert_eq!(references.len(), 1);
    assert!(
        references[0].contains(r#""result":[]"#),
        "{}",
        references[0]
    );
    assert_eq!(rename.len(), 1);
    assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
    assert_eq!(body_definition.len(), 1);
    assert!(
        body_definition[0].contains(
            r#""range":{"start":{"line":9,"character":15},"end":{"line":9,"character":23}}"#
        ),
        "{}",
        body_definition[0]
    );
}

#[test]
fn invalid_handler_bindings_do_not_enter_lsp_navigation() {
    let mut server = Server::default();
    let project = TempProject::new("invalid-handler-binding-navigation");
    project.write(
        "main.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "  echo(value: Int) -> Int\n",
            "end\n",
            "\n",
            "handler adjust(Callback: fn(Int) -> Int) handles Adjust\n",
            "  amount(value) => Callback(value)\n",
            "  echo(Result) => Callback(Result)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));

    let responses = server.handle_message(&initialize_request(&root_uri));
    let publish = publish_for_uri(&responses, &main_uri);
    assert!(
        publish.contains(r#""code":"name.invalid_case""#),
        "{publish}"
    );

    for (line, character) in [(5, 15), (6, 19), (7, 7), (7, 27)] {
        let definition = server.handle_message(&definition_request(&main_uri, line, character));
        assert_eq!(definition.len(), 1);
        assert!(
            definition[0].contains(r#""result":null"#),
            "{}",
            definition[0]
        );

        let references = server.handle_message(&references_request(&main_uri, line, character));
        assert_eq!(references.len(), 1);
        assert!(
            references[0].contains(r#""result":[]"#),
            "{}",
            references[0]
        );

        let prepare_rename =
            server.handle_message(&prepare_rename_request(&main_uri, line, character));
        assert_eq!(prepare_rename.len(), 1);
        assert!(
            prepare_rename[0].contains(r#""result":null"#),
            "{}",
            prepare_rename[0]
        );

        let rename = server.handle_message(&rename_request(&main_uri, line, character, "fixed"));
        assert_eq!(rename.len(), 1);
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
    }
}

#[test]
fn companion_private_function_rename_includes_target_function_alias_target() {
    let mut server = Server::default();
    let project = TempProject::new("rename-function-alias-target");
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "pub fn advance = increment\n",
        ),
    );
    project.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "bump"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"bump""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":4,"character":17},"end":{"line":4,"character":26}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn companion_private_function_lsp_rejects_companion_function_values_and_aliases() {
    let mut server = Server::default();
    let project = TempProject::new("reject-companion-function-values");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "pub fn expose = math::increment\n",
            "\n",
            "test companion() -> ()\n",
            "  let mapper: fn(Int) -> Int = math::increment\n",
            "  ()\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let alias_definition = server.handle_message(&definition_request(&companion_uri, 2, 23));
    let value_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 37));
    let value_rename = server.handle_message(&rename_request(&companion_uri, 5, 37, "bump"));

    assert!(
        alias_definition[0].contains(r#""result":null"#),
        "{}",
        alias_definition[0]
    );
    assert!(
        value_prepare[0].contains(r#""result":null"#),
        "{}",
        value_prepare[0]
    );
    assert!(
        value_rename[0].contains(r#""changes":{}"#),
        "{}",
        value_rename[0]
    );
}

#[test]
fn companion_private_function_requests_ignore_comment_and_string_origins() {
    let mut server = Server::default();
    let project = TempProject::new("request-origin");
    project.write(
        "math.veln",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "test companion() -> Int\n",
            "  math::increment(1)\n",
            "  \"math::increment(2)\"\n",
            "  # math::increment(3)\n",
            "end\n",
        ),
    );
    let root_uri = path_to_uri(&project.root);
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let string_definition = server.handle_message(&definition_request(&companion_uri, 4, 10));
    let comment_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 11));
    let comment_rename = server.handle_message(&rename_request(&companion_uri, 5, 11, "advance"));

    assert!(
        string_definition[0].contains(r#""result":null"#),
        "{}",
        string_definition[0]
    );
    assert!(
        comment_prepare[0].contains(r#""result":null"#),
        "{}",
        comment_prepare[0]
    );
    assert!(
        comment_rename[0].contains(r#""changes":{}"#),
        "{}",
        comment_rename[0]
    );
}

#[test]
fn companion_private_function_lsp_rejects_other_private_boundaries() {
    let mut server = Server::default();
    let project = TempProject::new("rejected-private-boundaries");
    project.write(
        "math.veln",
        "use support\n\nfn increment(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "support.veln",
        "fn private_helper(value: Int) -> Int\n  value\nend\n",
    );
    project.write(
        "other.test.veln",
        "use math\n\ntest wrong() -> Int\n  math::increment(1)\nend\n",
    );
    project.write(
        "math_test.veln",
        "use math\n\ntest integration() -> Int\n  math::increment(1)\nend\n",
    );
    project.write(
        "math.test.veln",
        "use support\n\ntest transitive() -> Int\n  support::private_helper(1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    server.handle_message(&initialize_request(&root_uri));

    let wrong_uri = path_to_uri(&project.root.join("other.test.veln"));
    let integration_uri = path_to_uri(&project.root.join("math_test.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));

    let wrong = server.handle_message(&definition_request(&wrong_uri, 3, 10));
    let integration = server.handle_message(&definition_request(&integration_uri, 3, 10));
    let transitive = server.handle_message(&definition_request(&companion_uri, 3, 13));

    assert!(wrong[0].contains(r#""result":null"#), "{}", wrong[0]);
    assert!(
        integration[0].contains(r#""result":null"#),
        "{}",
        integration[0]
    );
    assert!(
        transitive[0].contains(r#""result":null"#),
        "{}",
        transitive[0]
    );
}

#[test]
fn companion_private_function_definition_uses_open_document_overlay() {
    let mut server = Server::default();
    let project = companion_private_function_project("definition-overlay");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  value + 1\nend\n"}}}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&definition_request(&companion_uri, 3, 10));

    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
        ),
        "{}",
        responses[0]
    );
}

#[test]
fn dependency_definition_round_trips_through_retained_virtual_document() {
    let mut server = Server::default();
    let project = TempProject::new("dependency-virtual-document");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use math from \"example/pkg\"\n\n",
            "pub fn main() -> Int\n",
            "  math::exposed(1)\n",
            "  math::secret(1)\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"./math.veln\"]\n",
        ),
    );
    let retained_text = concat!(
        "pub fn exposed(value: Int) -> Int\r\n",
        "  value + 1\r\n",
        "end\r\n\r\n",
        "fn secret(value: Int) -> Int\r\n",
        "  value\r\n",
        "end\r\n",
    );
    project.write("vendor/lib/math.veln", retained_text);
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    project.write(
        "vendor/lib/math.veln",
        "pub fn changed() -> Int\n  0\nend\n",
    );
    project.write(
        "main.veln",
        "use math from \"example/pkg\"\n\npub fn main() -> Int\n  math::changed()\nend\n",
    );
    let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
    assert_eq!(definition.len(), 1);
    let virtual_uri = package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
    assert!(
        !virtual_uri.contains("vendor") && !virtual_uri.contains("veln-lsp-"),
        "{}",
        definition[0]
    );
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        definition[0]
    );

    assert_virtual_document_text(&mut server, "3", &virtual_uri, retained_text);

    let prepare_rename = server.handle_message(&prepare_rename_request(&main_uri, 3, 10));
    assert_null_result(&prepare_rename[0]);
    let rename = server.handle_message(&rename_request(&main_uri, 3, 10, "renamed"));
    assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);

    let private_definition = server.handle_message(&definition_request(&main_uri, 4, 10));
    assert_null_result(&private_definition[0]);

    for include_declaration in [false, true] {
        let references = server.handle_message(&references_request_with_declaration(
            &main_uri,
            3,
            10,
            include_declaration,
        ));
        assert_empty_result_array(&references[0]);
    }
    for rejected_uri in [
        format!("{virtual_uri}/missing"),
        virtual_uri.replacen("%2F", "%2f", 1),
    ] {
        let rejected = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
            ));
        assert_invalid_params(&rejected[0]);
    }
}

#[test]
fn path_vendor_and_mirror_dependencies_share_retained_virtual_uris() {
    let mut retained_virtual_uri = None;

    for (source_field, source_root) in [
        ("path", "path/lib"),
        ("vendor", "vendor/lib"),
        ("mirror", "mirror/example/pkg"),
    ] {
        let project = TempProject::new(&format!("dependency-virtual-document-{source_field}"));
        let manifest_text = format!(
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\n",
                "{} = \"{}\"\n",
            ),
            source_field, source_root
        );
        project.write("veln.toml", &manifest_text);
        project.write(
            "main.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "use hidden from \"example/pkg\"\n\n",
                "pub fn main() -> Int\n",
                "  math::exposed(1)\n",
                "  math::secret(1)\n",
                "  hidden::published(1)\n",
                "end\n",
            ),
        );
        project.write(
            &format!("{source_root}/veln.toml"),
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        let retained_text = "pub fn exposed(value: Int) -> Int\r\n  value + 1\r\nend\r\n";
        project.write(&format!("{source_root}/math.veln"), retained_text);
        project.write(
            &format!("{source_root}/hidden.veln"),
            "pub fn published(value: Int) -> Int\n  value\nend\n",
        );

        if let Some(virtual_uri) = retained_virtual_uri.as_deref() {
            let manifest = parse_manifest_text("veln.toml", &manifest_text);
            let dependencies = retained_direct_dependencies(&project.root, &manifest);
            assert_eq!(dependencies.len(), 1, "{source_field}");
            let snapshot =
                EffectiveProjectSnapshot::with_direct_dependencies(Vec::new(), dependencies);
            assert_eq!(
                snapshot.resolve_virtual_source(virtual_uri),
                Some(retained_text.as_bytes()),
                "{source_field}"
            );
            continue;
        }

        let mut server = Server::default();
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 5, 10));
        let virtual_uri =
            package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
        assert!(!virtual_uri.contains(source_root), "{}", definition[0]);

        assert_virtual_document_text(&mut server, "3", &virtual_uri, retained_text);
        let private_definition = server.handle_message(&definition_request(&main_uri, 6, 10));
        assert_null_result(&private_definition[0]);
        let unexported_definition = server.handle_message(&definition_request(&main_uri, 7, 12));
        assert_null_result(&unexported_definition[0]);
        retained_virtual_uri = Some(virtual_uri);
    }

    assert!(retained_virtual_uri.is_some());
}

#[test]
fn git_dependency_subdir_definition_uses_retained_virtual_document() {
    let mut server = Server::default();
    let project = TempProject::new("git-dependency-virtual-document");
    let remote_url = "https://example.invalid/mono.git";
    let repository_root = materialized_git_repository_root(&project.root, remote_url);
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/lib\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use math from \"example/pkg\"\n\n",
            "pub fn main() -> Int\n",
            "  math::exposed(1)\n",
            "  math::secret(1)\n",
            "end\n",
        ),
    );
    let package_root = repository_root.join("packages/lib");
    project.write(
        &package_root.join("veln.toml").display().to_string(),
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    let retained_text = concat!(
        "pub fn exposed(value: Int) -> Int\r\n",
        "  value + 1\r\n",
        "end\r\n\r\n",
        "fn secret(value: Int) -> Int\r\n",
        "  value\r\n",
        "end\r\n",
    );
    project.write(
        &package_root.join("math.veln").display().to_string(),
        retained_text,
    );

    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));
    project.write(
        &package_root.join("math.veln").display().to_string(),
        "pub fn changed() -> Int\n  0\nend\n",
    );

    let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
    let virtual_uri = package_virtual_definition_uri(&definition[0], "example%2Fpkg", "math.veln");
    assert!(
        !virtual_uri.contains(".veln/package/git") && !virtual_uri.contains("packages/lib"),
        "{}",
        definition[0]
    );

    assert_virtual_document_text(&mut server, "3", &virtual_uri, retained_text);

    let private_definition = server.handle_message(&definition_request(&main_uri, 4, 10));
    assert_null_result(&private_definition[0]);
}

#[test]
fn standard_library_definition_round_trips_through_embedded_virtual_document() {
    let mut server = Server::default();
    let project = standard_library_virtual_document_project();
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let prelude_uri = assert_standard_prelude_navigation(&mut server, &main_uri);
    assert_standard_import_navigation(&mut server, &main_uri);

    assert_virtual_document_text(
        &mut server,
        "3",
        &prelude_uri,
        standard_source_text("prelude.veln"),
    );

    let private_definition = server.handle_message(&definition_request(&main_uri, 24, 12));
    assert!(private_definition[0].contains(r#""result":null"#));
    let prepare_rename = server.handle_message(&prepare_rename_request(&main_uri, 3, 4));
    let rename = server.handle_message(&rename_request(&main_uri, 3, 4, "renamed"));
    assert!(prepare_rename[0].contains(r#""result":null"#));
    assert!(rename[0].contains(r#""changes":{}"#));

    for rejected_uri in [
        format!("{prelude_uri}/missing"),
        prelude_uri.replacen("veln-pkg", "VELN-pkg", 1),
    ] {
        let rejected = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
            ));
        assert_invalid_params(&rejected[0]);
    }
}

fn standard_library_virtual_document_project() -> TempProject {
    let project = TempProject::new("standard-library-virtual-document");
    project.write(
        "main.veln",
        concat!(
            "use http2::diagnostic from \"std\"\n\n",
            "pub fn implicit() -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn qualified() -> Result<Byte, String>\n",
            "  prelude::byte(1)\n",
            "end\n\n",
            "pub fn parameter_shadow(byte: fn(Int) -> Result<Byte, String>) -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn local_shadow() -> Result<Byte, String>\n",
            "  let byte: fn(Int) -> Result<Byte, String> = prelude::byte\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn imported() -> Result<(), RuntimeDiagnostic>\n",
            "  http2::diagnostic::protocol_invalid_frame_kind(0, 0, 0, 0, \"open\", \"rule\", byte_view(byte_chunk([]), ByteOffset(0), ByteCount(0)))\n",
            "end\n\n",
            "pub fn private_helper() -> Vec<Int>\n",
            "  prelude::vec_append([], 1)\n",
            "end\n",
        ),
    );
    project
}

fn assert_standard_prelude_navigation(server: &mut Server, main_uri: &str) -> String {
    let implicit = server.handle_message(&definition_request(main_uri, 3, 4));
    let qualified = server.handle_message(&definition_request(main_uri, 7, 12));
    let shadowed_parameter = server.handle_message(&definition_request(main_uri, 11, 2));
    let shadowed_local = server.handle_message(&definition_request(main_uri, 16, 2));
    let prelude_uri = package_virtual_definition_uri(&implicit[0], "std", "prelude.veln");

    assert_eq!(
        extract_string_field(&qualified[0], "uri"),
        Some(prelude_uri.clone())
    );
    assert!(
        implicit[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        implicit[0]
    );
    assert_null_result(&shadowed_parameter[0]);
    assert_null_result(&shadowed_local[0]);
    prelude_uri
}

fn assert_standard_import_navigation(server: &mut Server, main_uri: &str) {
    let imported = server.handle_message(&definition_request(main_uri, 20, 31));
    let diagnostic_uri =
        package_virtual_definition_uri(&imported[0], "std", "http2/diagnostic.veln");
    assert!(diagnostic_uri.contains("/http2/diagnostic.veln"));
}

fn package_virtual_definition_uri(response: &str, package: &str, source_path: &str) -> String {
    let uri = extract_string_field(response, "uri").unwrap();
    assert!(
        uri.starts_with(&format!("veln-pkg:///{package}/snapshot/")),
        "{response}"
    );
    assert!(uri.ends_with(&format!("/{source_path}")), "{response}");
    uri
}

fn assert_virtual_document_text(server: &mut Server, id: &str, uri: &str, expected: &str) {
    let read = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"veln/virtualDocument","params":{{"uri":"{uri}"}}}}"#
    ));
    assert_eq!(
        read,
        [response(id, &format!(r#""{}""#, escape_json(expected)))]
    );
}

fn standard_source_text(path: &str) -> &'static str {
    veln_stdlib::package_bundle()
        .files
        .iter()
        .find(|file| file.path == path)
        .unwrap()
        .text
}

fn assert_null_result(response: &str) {
    assert!(response.contains(r#""result":null"#), "{response}");
}

fn assert_empty_result_array(response: &str) {
    assert!(response.contains(r#""result":[]"#), "{response}");
}

fn assert_invalid_params(response: &str) {
    assert!(response.contains(r#""code":-32602"#), "{response}");
}

#[test]
fn ambiguous_bare_prelude_fallback_returns_no_definition() {
    let mut server = Server::default();
    let project = TempProject::new("ambiguous-bare-prelude-definition");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "math.veln",
        concat!(
            "use math from \"example/pkg\"\n\n",
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    assert!(
        definition[0].contains(r#""result":null"#),
        "{}",
        definition[0]
    );
}

#[test]
fn private_workspace_import_does_not_hide_bare_prelude_definition() {
    let mut server = Server::default();
    let project = TempProject::new("private-import-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use math\n\n",
            "pub fn main() -> Result<Byte, String>\n",
            "  byte(1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.veln",
        "fn byte(value: Int) -> Result<Byte, String>\n  Ok(Byte(value))\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    let prelude_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        definition[0]
    );
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        definition[0]
    );
}

#[test]
fn invalid_imported_constructor_casing_falls_back_to_bare_prelude_function() {
    let mut server = Server::default();
    let project = TempProject::new("imported-constructor-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use model\n\n",
            "pub fn main() -> Token\n",
            "  byte(1)\n",
            "end\n",
        ),
    );
    project.write(
        "model.veln",
        concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

    assert_eq!(definition.len(), 1);
    let prelude_uri = extract_string_field(&definition[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        definition[0]
    );
    assert!(
        definition[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        definition[0]
    );
}

#[test]
fn invalid_reexported_constructor_casing_does_not_hide_bare_prelude_function() {
    let mut server = Server::default();
    let project = TempProject::new("reexported-constructor-bare-prelude-definition");
    project.write(
        "main.veln",
        concat!(
            "use facade\n\n",
            "pub fn bare() -> Token\n",
            "  byte(1)\n",
            "end\n\n",
            "pub fn qualified() -> Token\n",
            "  facade::byte(2)\n",
            "end\n",
        ),
    );
    project.write(
        "facade.veln",
        concat!("use model\n\n", "pub type Token = model::Token\n"),
    );
    project.write(
        "model.veln",
        concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let bare = server.handle_message(&definition_request(&main_uri, 3, 4));
    assert_eq!(bare.len(), 1);
    let prelude_uri = extract_string_field(&bare[0], "uri").unwrap();
    assert!(
        prelude_uri.starts_with("veln-pkg:///std/snapshot/")
            && prelude_uri.ends_with("/prelude.veln"),
        "{}",
        bare[0]
    );
    assert!(
        bare[0].contains(
            r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
        ),
        "{}",
        bare[0]
    );

    let qualified = server.handle_message(&definition_request(&main_uri, 7, 10));
    assert_eq!(qualified.len(), 1);
    assert!(
        qualified[0].contains(r#""result":null"#),
        "{}",
        qualified[0]
    );
}

#[test]
fn retained_direct_dependencies_use_the_supplied_workspace_manifest() {
    let project = TempProject::new("retained-dependency-supplied-manifest");
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    let manifest = parse_manifest_text(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );

    let dependencies = retained_direct_dependencies(&project.root, &manifest);

    assert_eq!(dependencies.len(), 1);
    let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
        vec![SourceFile::new(
            "main.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "pub fn main() -> Int\n",
                "  math::exposed(1)\n",
                "end\n",
            ),
        )],
        dependencies,
    );
    assert!(
        navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 10,
            }
        )
        .is_some()
    );
}

#[test]
fn dependency_definition_requires_external_import_path() {
    let mut server = Server::default();
    let project = TempProject::new("dependency-import-boundary");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "use math from \"other/pkg\"\n",
            "use other from \"example/pkg\"\n\n",
            "pub fn missing_import() -> Int\n",
            "  exposed(1)\n",
            "end\n\n",
            "pub fn wrong_package() -> Int\n",
            "  math::exposed(1)\n",
            "end\n\n",
            "pub fn wrong_module() -> Int\n",
            "  other::exposed(1)\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(4, 4), (8, 10), (12, 11)] {
        let response = server.handle_message(&definition_request(&main_uri, line, character));
        assert!(response[0].contains(r#""result":null"#), "{}", response[0]);
    }
}

#[test]
fn workspace_references_and_rename_ignore_dependency_sources() {
    let mut server = Server::default();
    let project = TempProject::new("dependency-reference-isolation");
    project.write(
        "veln.toml",
        concat!(
            "[package]\nname = \"app\"\n\n",
            "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
        ),
    );
    project.write(
        "math.veln",
        "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
    );
    project.write(
        "vendor/lib/veln.toml",
        concat!(
            "[package]\nname = \"example/pkg\"\n\n",
            "[lib]\nexports = [\"math.veln\"]\n",
        ),
    );
    project.write(
        "vendor/lib/math.veln",
        "pub fn exposed(value: Int) -> Int\n  exposed(value - 1)\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    server.handle_message(&initialize_request(&root_uri));

    let references = server.handle_message(&references_request(&math_uri, 0, 7));
    assert!(
        references[0].contains(
            r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        references[0]
    );
    assert!(
        !references[0].contains(r#""line":1,"character":2"#),
        "{}",
        references[0]
    );

    let rename = server.handle_message(&rename_request(&math_uri, 0, 7, "renamed"));
    assert!(
        rename[0].contains(
            r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
        ),
        "{}",
        rename[0]
    );
    assert!(
        !rename[0].contains(r#""line":1,"character":2"#),
        "{}",
        rename[0]
    );
    assert!(!rename[0].contains("vendor"), "{}", rename[0]);
}

#[test]
fn companion_private_function_rename_uses_open_document_overlay() {
    let mut server = Server::default();
    let project = companion_private_function_project("rename-overlay");
    let root_uri = path_to_uri(&project.root);
    let math_uri = path_to_uri(&project.root.join("math.veln"));
    let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
    server.handle_message(&initialize_request(&root_uri));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  bump(value)\nend\n"}}}}}}"#
        ));
    server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\n  math::bump\nend\n"}}}}}}"#
        ));

    let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

    assert_eq!(responses.len(), 1);
    assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":6}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        responses[0].contains(
            r#""range":{"start":{"line":3,"character":8},"end":{"line":3,"character":12}}"#
        ),
        "{}",
        responses[0]
    );
    assert!(
        !responses[0].contains(r#""line":4,"character":8"#),
        "{}",
        responses[0]
    );
}

#[test]
fn server_returns_full_semantic_tokens_for_open_document() {
    let mut server = Server::default();
    server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn main() -> Int\n  main()\nend\n"}}}"#,
        );

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""id":2"#));
    assert!(responses[0].contains(r#""data":["#));
}

#[test]
fn server_publishes_parse_diagnostics_for_open_document() {
    let mut server = Server::default();

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""method":"textDocument/publishDiagnostics""#));
    assert!(responses[0].contains(r#""source":"veln""#));
    assert!(responses[0].contains(r#""severity":1"#));
    assert!(responses[0].contains(r#""code":"parse."#));
}

#[test]
fn lsp_diagnostic_wire_fields_are_stable() {
    let diagnostic = Diagnostic::new(
        "parse.expected_item",
        Severity::Error,
        DiagnosticKind::Parse,
        "expected a function or test declaration",
        Some(SourceSpan {
            file: veln_source::SourcePath::new("main.veln"),
            start: veln_source::LineCol {
                line: 2,
                column: 3,
                offset: 4,
            },
            end: veln_source::LineCol {
                line: 2,
                column: 5,
                offset: 6,
            },
        }),
        JsonValue::Null,
    );

    assert_eq!(
        lsp_diagnostic_json(&diagnostic),
        concat!(
            "{\"range\":{\"start\":{\"line\":1,\"character\":2},",
            "\"end\":{\"line\":1,\"character\":4}},\"severity\":1,",
            "\"code\":\"parse.expected_item\",\"source\":\"veln\",",
            "\"message\":\"expected a function or test declaration\"}"
        )
    );
}

#[test]
fn server_publishes_semantic_diagnostics_after_parse_succeeds() {
    let mut server = Server::default();

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"pub fn main() -> ()\n  stdio::println(\"hello\")\nend\n"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""code":"effect.missing_public""#));
}

#[test]
fn server_clears_diagnostics_for_closed_document() {
    let mut server = Server::default();
    server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

    let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

    assert_eq!(responses.len(), 1);
    assert!(responses[0].contains(r#""diagnostics":[]"#));
}

#[test]
fn server_reads_and_writes_content_length_frames() {
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let input = format!("Content-Length: {}\r\n\r\n{request}", request.len());
    let mut output = Vec::new();
    let mut server = Server::default();

    server.run(input.as_bytes(), &mut output).unwrap();

    let output = String::from_utf8(output).unwrap();
    assert!(output.starts_with("Content-Length: "));
    assert!(output.contains(r#""id":1"#));
}

fn publish_for_uri<'a>(responses: &'a [String], uri: &str) -> &'a str {
    responses
        .iter()
        .find(|response| {
            response.contains(r#""method":"textDocument/publishDiagnostics""#)
                && response.contains(&format!(r#""uri":"{}""#, escape_json(uri)))
        })
        .map(String::as_str)
        .unwrap_or_else(|| panic!("expected publish diagnostics for {uri}: {responses:#?}"))
}

fn companion_private_function_project(name: &str) -> TempProject {
    let project = TempProject::new(name);
    project.write(
        "math.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  increment(value - 1)\n",
            "end\n",
        ),
    );
    project.write(
        "math.test.veln",
        concat!(
            "use math\n",
            "\n",
            "fn increment(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "test increment_test() -> Int\n",
            "  math::increment(1)\n",
            "end\n",
            "\n",
            "test local_increment_test() -> Int\n",
            "  increment(1)\n",
            "end\n",
        ),
    );
    project
}

fn initialize_request(root_uri: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
    )
}

fn definition_request(uri: &str, line: usize, character: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
    )
}

fn references_request(uri: &str, line: usize, character: usize) -> String {
    references_request_with_declaration(uri, line, character, true)
}

fn references_request_with_declaration(
    uri: &str,
    line: usize,
    character: usize,
    include_declaration: bool,
) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"context":{{"includeDeclaration":{include_declaration}}}}}}}"#
    )
}

fn prepare_rename_request(uri: &str, line: usize, character: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
    )
}

fn rename_request(uri: &str, line: usize, character: usize, new_name: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"newName":"{new_name}"}}}}"#
    )
}

fn semantic_tokens_request(uri: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#
    )
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let root = env::temp_dir().join(format!(
            "veln-lsp-{name}-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        fs::create_dir_all(&root).expect("temp project should be created");
        Self { root }
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, contents).expect("fixture source should be written");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should produce a temp suffix")
        .as_nanos()
}
