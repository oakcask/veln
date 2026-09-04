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
    let (mut server, workspace, uris, responses) = initialize_symlink_alias_workspace();

    assert_eq!(server.workspace_roots, vec![workspace.root.join("package")]);
    let publish = publish_for_uri(&responses, &uris.main);
    assert_contains_json(publish, r#""diagnostics":[]"#);
    assert_alias_open_document_diagnostics(&mut server, &uris.main);
    assert_alias_semantic_tokens(&mut server, &uris.main);
    assert_alias_companion_definition(&mut server, &uris);
}

struct AliasWorkspaceUris {
    main: String,
    math: String,
    companion: String,
}

fn initialize_symlink_alias_workspace() -> (Server, TempProject, AliasWorkspaceUris, Vec<String>) {
    use std::os::unix::fs::symlink;

    let mut server = Server::default();
    let workspace = symlink_workspace_alias_project();
    symlink(workspace.root.join("package"), workspace.root.join("alias"))
        .expect("workspace alias should be created");
    let alias_uri = path_to_uri(&workspace.root.join("alias"));
    let alias_main_uri = path_to_uri(&workspace.root.join("alias/main.veln"));
    let alias_math_uri = path_to_uri(&workspace.root.join("alias/math.veln"));
    let alias_companion_uri = path_to_uri(&workspace.root.join("alias/math.test.veln"));

    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alias_uri}","name":"alias"}}]}}}}"#
        ));
    let uris = AliasWorkspaceUris {
        main: alias_main_uri,
        math: alias_math_uri,
        companion: alias_companion_uri,
    };
    (server, workspace, uris, responses)
}

fn assert_alias_open_document_diagnostics(server: &mut Server, alias_main_uri: &str) {
    let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{alias_main_uri}","text":"pub fn main() -> Int\n  \"bad\"\nend\n"}}}}}}"#
    ));
    let publish = publish_for_uri(&responses, alias_main_uri);
    assert_contains_json(publish, r#""code":"type.mismatch""#);
}

fn assert_alias_semantic_tokens(server: &mut Server, alias_main_uri: &str) {
    let responses = server.handle_message(&semantic_tokens_request(alias_main_uri));
    let response = assert_single_response(&responses, r#""id":2,"result":{"data":["#);
    assert_not_contains_json(response, r#""data":[]"#);
}

fn assert_alias_companion_definition(server: &mut Server, uris: &AliasWorkspaceUris) {
    let responses = server.handle_message(&definition_request(&uris.companion, 7, 10));
    assert_single_response(&responses, &escape_json(&uris.math));
}

fn symlink_workspace_alias_project() -> TempProject {
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
    workspace
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
    let (mut server, _project, main_uri, responses) = snapshot_invalid_casing_server();

    let publish = publish_for_uri(&responses, &main_uri);
    assert!(
        publish.contains(r#""code":"name.invalid_case""#),
        "{publish}"
    );
    let declaration = server.handle_message(&definition_request(&main_uri, 0, 3));
    assert_response_contains_bad_declaration_range(&declaration[0]);
    let call = server.handle_message(&definition_request(&main_uri, 5, 2));
    assert_response_contains_bad_declaration_range(&call[0]);
    let references = server.handle_message(&references_request(&main_uri, 0, 3));
    assert_snapshot_invalid_casing_references(&references[0]);
    let prepare = server.handle_message(&prepare_rename_request(&main_uri, 5, 2));
    assert_response_contains_bad_call_prepare_range(&prepare[0]);
    let rename = server.handle_message(&rename_request(&main_uri, 0, 3, "renamed"));
    assert_eq!(rename[0].matches(r#""newText":"renamed""#).count(), 2);
    assert_response_contains_bad_declaration_range(&rename[0]);
    assert_response_contains_bad_call_edit_range(&rename[0]);
}

fn snapshot_invalid_casing_server() -> (Server, TempProject, String, Vec<String>) {
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
    (server, project, main_uri, responses)
}

fn assert_response_contains_bad_declaration_range(response: &str) {
    assert!(
        response.contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":6}}"#
        ),
        "{}",
        response
    );
}

fn assert_snapshot_invalid_casing_references(response: &str) {
    assert!(
        response.contains(r#""result":[{"uri":"file://"#)
            && response.contains(r#""line":0,"character":3"#)
            && response.contains(r#""line":5,"character":2"#),
        "{}",
        response
    );
}

fn assert_response_contains_bad_call_prepare_range(response: &str) {
    assert!(
        response.contains(
            r#""result":{"start":{"line":5,"character":2},"end":{"line":5,"character":5}}"#
        ),
        "{}",
        response
    );
}

fn assert_response_contains_bad_call_edit_range(response: &str) {
    assert!(
        response.contains(
            r#""range":{"start":{"line":5,"character":2},"end":{"line":5,"character":5}}"#
        ),
        "{}",
        response
    );
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
    assert!(
        call[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":6}}"#
        ),
        "{}",
        call[0]
    );

    let cached_snapshot = server
        .overlaid_project_snapshots
        .get(&project.root)
        .cloned()
        .expect("navigation should retain the overlaid project snapshot");
    let references = server.handle_message(&references_request(&main_uri, 5, 2));
    assert_snapshot_invalid_casing_references(&references[0]);
    assert!(
        Arc::ptr_eq(
            server
                .overlaid_project_snapshots
                .get(&project.root)
                .expect("navigation should keep the snapshot cached"),
            &cached_snapshot,
        ),
        "unchanged overlays should reuse their navigation snapshot",
    );

    let unchanged_responses = server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{main_uri}"}},"contentChanges":[{{"text":"fn Bad() -> Int\n  1\nend\n\nfn caller() -> Int\n  Bad()\nend\n"}}]}}}}"#
    ));
    assert!(
        unchanged_responses.is_empty(),
        "an unchanged document should not republish diagnostics",
    );
    assert!(Arc::ptr_eq(
        server
            .overlaid_project_snapshots
            .get(&project.root)
            .expect("an unchanged document should keep the snapshot cached"),
        &cached_snapshot,
    ));

    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{main_uri}"}},"contentChanges":[{{"text":"fn Worse() -> Int\n  1\nend\n\nfn caller() -> Int\n  Worse()\nend\n"}}]}}}}"#
    ));
    assert!(
        !Arc::ptr_eq(
            server
                .overlaid_project_snapshots
                .get(&project.root)
                .expect("a document change should retain a refreshed snapshot"),
            &cached_snapshot,
        ),
        "a document change should invalidate the cached navigation snapshot",
    );
    let changed_call = server.handle_message(&definition_request(&main_uri, 5, 2));
    assert!(
        changed_call[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":8}}"#
        ),
        "{}",
        changed_call[0]
    );

    server.handle_message(&format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{main_uri}"}}}}}}"#
    ));
    assert!(Arc::ptr_eq(
        server
            .overlaid_project_snapshots
            .get(&project.root)
            .expect("closing the overlay should restore the base snapshot"),
        server
            .project_snapshots
            .get(&project.root)
            .expect("the base project snapshot should remain retained"),
    ));
    let saved_call = server.handle_message(&definition_request(&main_uri, 5, 2));
    assert!(
        saved_call[0].contains(
            r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
        ),
        "{}",
        saved_call[0]
    );
}
