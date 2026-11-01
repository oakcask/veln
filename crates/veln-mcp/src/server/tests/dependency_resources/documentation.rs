use super::*;

#[test]
fn saved_dependency_documentation_uses_other_captured_dependencies() {
    let workspace = TempWorkspace::new("dependency-doc-transitive-capture");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"a\"]\n",
            "path = \"vendor/a\"\n",
            "[dependencies.\"b\"]\n",
            "path = \"vendor/b\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use api from \"a\"\n",
            "use shared from \"b\"\n",
            "fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/a/veln.toml",
        concat!(
            "[package]\n",
            "name = \"a\"\n",
            "[lib]\n",
            "exports = [\"api.veln\"]\n",
            "[dependencies.\"b\"]\n",
            "path = \"../b\"\n",
        ),
    );
    workspace.write(
        "vendor/a/api.veln",
        "use shared from \"b\"\npub let exposed: Int = shared::answer\n",
    );
    workspace.write(
        "vendor/b/veln.toml",
        concat!(
            "[package]\n",
            "name = \"b\"\n",
            "[lib]\n",
            "exports = [\"shared.veln\"]\n",
        ),
    );
    workspace.write("vendor/b/shared.veln", "pub let answer: Int = 1\n");
    let mut server = initialized_server(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let index_uri = listed_dependency_documentation_uri(&mut server, "a");
    assert!(index_uri.ends_with("/index"), "{index_uri}");
    let declaration_uri = linked_dependency_declaration_uri(&mut server, &index_uri);
    let declaration = read_resource(&mut server, 109, &declaration_uri);
    let text = declaration["result"]["contents"][0]["text"]
        .as_str()
        .unwrap();
    assert!(text.starts_with("# Module_value exposed\n\n"), "{text}");
    assert!(text.contains("- Signature: let exposed: Int"), "{text}");
}

#[test]
fn dependency_context_revision_replaces_failure_with_current_success() {
    let workspace = TempWorkspace::new("dependency-doc-context-failure-success");
    write_context_revision_workspace(&workspace, "pub let answer: Int = \"wrong\"\n");
    let mut server = initialized_server(&workspace);

    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let source_uris = listed_dependency_source_uris(&mut server, "a");
    let retained = RetainedFailedDocumentation::capture(&mut server);
    assert!(package_search(&mut server, "exposed").is_empty());
    replace_dependency_context(&workspace, &mut server, "pub let answer: Int = 1\n");

    assert_eq!(listed_dependency_source_uris(&mut server, "a"), source_uris);
    let index_uri = listed_dependency_documentation_uri(&mut server, "a");
    assert_current_dependency_documentation_success(&mut server, &index_uri);
    retained.assert_exact_read_unchanged(&mut server);
    assert_eq!(
        dependency_documentation_uris(&mut server, "a"),
        vec![index_uri]
    );
}

struct RetainedFailedDocumentation {
    uri: String,
    read: Value,
}

impl RetainedFailedDocumentation {
    fn capture(server: &mut Server) -> Self {
        let uri = listed_dependency_documentation_uri(server, "a");
        assert!(uri.ends_with("/status"), "{uri}");
        let read = read_resource(server, 112, &uri);
        Self { uri, read }
    }

    fn assert_exact_read_unchanged(&self, server: &mut Server) {
        assert_eq!(
            read_resource(server, 114, &self.uri)["result"],
            self.read["result"]
        );
    }
}

fn assert_current_dependency_documentation_success(server: &mut Server, index_uri: &str) {
    assert!(index_uri.ends_with("/index"), "{index_uri}");
    let search = package_search(server, "exposed");
    assert_eq!(search.len(), 1, "{search:#?}");
    let definition = server.definition_tool(&json!({"source":"main.veln","line":3,"column":8}));
    assert_eq!(definition["isError"], false, "{definition:#}");
    let documentation_uri =
        definition["structuredContent"]["definition"]["packageDocumentationUri"]
            .as_str()
            .expect("accepted sibling definition has a documentation URI");
    assert!(documentation_uri.contains("/declaration/"));
    let declaration = read_resource(server, 113, documentation_uri);
    let text = declaration["result"]["contents"][0]["text"]
        .as_str()
        .unwrap();
    assert!(
        text.starts_with("# Function anchor\n"),
        "{documentation_uri}"
    );
}

#[test]
fn dependency_context_revision_replaces_success_with_current_failure() {
    let workspace = TempWorkspace::new("dependency-doc-context-success-failure");
    write_context_revision_workspace(&workspace, "pub let answer: Int = 1\n");
    let mut server = initialized_server(&workspace);

    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let source_uris = listed_dependency_source_uris(&mut server, "a");
    let retained = RetainedSuccessfulDocumentation::capture(&mut server);
    replace_dependency_context(&workspace, &mut server, "pub let answer: Int = \"wrong\"\n");

    assert_eq!(listed_dependency_source_uris(&mut server, "a"), source_uris);
    let status_uri = listed_dependency_documentation_uri(&mut server, "a");
    assert_current_dependency_documentation_failure(&mut server, &status_uri);
    retained.assert_exact_reads_unchanged(&mut server);
    assert_eq!(
        dependency_documentation_uris(&mut server, "a"),
        vec![status_uri]
    );
}

struct RetainedSuccessfulDocumentation {
    index: (String, Value),
    module: (String, Value),
    declaration: (String, Value),
}

impl RetainedSuccessfulDocumentation {
    fn capture(server: &mut Server) -> Self {
        let index_uri = listed_dependency_documentation_uri(server, "a");
        let index_read = read_resource(server, 115, &index_uri);
        let module_uri = linked_dependency_module_uri(server, &index_uri);
        let module_read = read_resource(server, 116, &module_uri);
        let declaration_uri = package_search(server, "exposed")
            .into_iter()
            .next()
            .expect("accepted module value is searchable");
        let declaration_read = read_resource(server, 117, &declaration_uri);
        Self {
            index: (index_uri, index_read),
            module: (module_uri, module_read),
            declaration: (declaration_uri, declaration_read),
        }
    }

    fn assert_exact_reads_unchanged(&self, server: &mut Server) {
        for (request_id, (uri, retained)) in [
            (119, &self.index),
            (120, &self.module),
            (121, &self.declaration),
        ] {
            assert_eq!(
                read_resource(server, request_id, uri)["result"],
                retained["result"],
                "{uri}"
            );
        }
    }
}

fn replace_dependency_context(workspace: &TempWorkspace, server: &mut Server, source: &str) {
    server
        .language_resources
        .set_test_remaining_package_capacity(1);
    server
        .language_resources
        .set_test_remaining_documentation_revision_capacity(2);
    workspace.write("vendor/b/shared.veln", source);
    refresh_workspace(server);
    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
}

fn assert_current_dependency_documentation_failure(server: &mut Server, status_uri: &str) {
    assert!(status_uri.ends_with("/status"), "{status_uri}");
    assert!(package_search(server, "exposed").is_empty());
    assert!(package_search(server, "anchor").is_empty());
    let definition = server.definition_tool(&json!({"source":"main.veln","line":3,"column":8}));
    assert_eq!(definition["isError"], false, "{definition:#}");
    let location = &definition["structuredContent"]["definition"];
    assert!(
        location.get("packageDocumentationUri").is_none(),
        "{location:#}"
    );
    let status = read_resource(server, 118, status_uri);
    let status_text = status["result"]["contents"][0]["text"].as_str().unwrap();
    assert!(!status_text.contains("anchor"), "{status_text}");
}

#[test]
fn dependency_documentation_revision_capacity_is_atomic_before_generation() {
    let workspace = TempWorkspace::new("dependency-doc-revision-capacity");
    write_context_revision_workspace(&workspace, "pub let answer: Int = 1\n");
    let mut server = initialized_server(&workspace);
    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let retained = RetainedRevisionCapacityState::capture(&mut server);

    server
        .language_resources
        .set_test_remaining_package_capacity(1);
    server
        .language_resources
        .set_test_remaining_documentation_revision_capacity(1);
    workspace.write("vendor/b/shared.veln", "pub let answer: Int = 2\n");
    refresh_workspace(&mut server);
    crate::language_resources::reset_dependency_preparation_work();
    let rejected = server.check_project_tool(&json!({"project":"."}));
    let work = crate::language_resources::dependency_preparation_work();

    assert_eq!(rejected["isError"], true, "{rejected:#}");
    assert_eq!(rejected["structuredContent"]["code"], "resource_capacity");
    assert_eq!(work.snapshot_captures, 2);
    assert_eq!(work.package_doc_generations, 0);
    retained.assert_unchanged(&mut server);
}

struct RetainedRevisionCapacityState {
    index_uri: String,
    index_read: Value,
    list: Value,
    search: Vec<String>,
    definition_uri: String,
}

impl RetainedRevisionCapacityState {
    fn capture(server: &mut Server) -> Self {
        let index_uri = listed_dependency_documentation_uri(server, "a");
        let index_read = read_resource(server, 122, &index_uri);
        let list = all_resource_state(server);
        let search = package_search(server, "exposed");
        let definition = server.definition_tool(&json!({"source":"main.veln","line":3,"column":8}));
        let definition_uri =
            definition["structuredContent"]["definition"]["packageDocumentationUri"]
                .as_str()
                .unwrap()
                .to_string();
        Self {
            index_uri,
            index_read,
            list,
            search,
            definition_uri,
        }
    }

    fn assert_unchanged(&self, server: &mut Server) {
        assert_eq!(all_resource_state(server), self.list);
        assert_eq!(package_search(server, "exposed"), self.search);
        assert_eq!(
            read_resource(server, 123, &self.index_uri)["result"],
            self.index_read["result"]
        );
        let read = read_resource(server, 124, &self.definition_uri);
        assert_eq!(read["result"]["contents"][0]["uri"], self.definition_uri);
    }
}

#[test]
fn dependency_failure_status_links_to_the_retained_causal_source() {
    let workspace = TempWorkspace::new("dependency-doc-causal-source");
    workspace.write(
        "veln.toml",
        concat!(
            "[dependencies.\"a\"]\n",
            "path = \"vendor/a\"\n",
            "[dependencies.\"b\"]\n",
            "path = \"vendor/b\"\n",
        ),
    );
    workspace.write(
        "main.veln",
        "use api from \"a\"\nuse shared from \"b\"\nfn main() -> Int\n  1\nend\n",
    );
    workspace.write(
        "vendor/a/veln.toml",
        concat!(
            "[package]\n",
            "name = \"a\"\n",
            "[lib]\n",
            "exports = [\"api.veln\"]\n",
            "[dependencies.\"b\"]\n",
            "path = \"../b\"\n",
        ),
    );
    workspace.write(
        "vendor/a/api.veln",
        "use shared from \"b\"\npub let exposed: Int = shared::answer\n",
    );
    workspace.write(
        "vendor/b/veln.toml",
        concat!(
            "[package]\n",
            "name = \"b\"\n",
            "[lib]\n",
            "exports = [\"shared.veln\"]\n",
        ),
    );
    let causal_source = "pub let answer: Int = \"wrong\"\n";
    workspace.write("vendor/b/shared.veln", causal_source);
    let mut server = initialized_server(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let status_uri = listed_dependency_documentation_uri(&mut server, "a");
    assert!(status_uri.ends_with("/status"), "{status_uri}");
    let status = read_resource(&mut server, 110, &status_uri);
    let status_text = status["result"]["contents"][0]["text"].as_str().unwrap();
    let source_uri = extract_uris_with_prefix(status_text, "veln-pkg:///b/snapshot/")
        .into_iter()
        .next()
        .expect("dependency diagnostic source URI");
    let source = read_resource(&mut server, 111, &source_uri);

    assert_eq!(source["result"]["contents"][0]["uri"], source_uri);
    assert_eq!(source["result"]["contents"][0]["text"], causal_source);
}

#[test]
fn failed_dependency_documentation_publishes_only_status_resource() {
    let workspace = TempWorkspace::new("dependency-doc-status");
    write_workspace_with_dependency_source(
        &workspace,
        concat!(
            "## Missing reference {@schema Missing}.\n",
            "pub fn value() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let listed = listed_resources(&mut server);
    let rendered = expected_dependency_documentation_resources(
        "example/dep",
        [
            (
                "dep.veln",
                concat!(
                    "## Missing reference {@schema Missing}.\n",
                    "pub fn value() -> Int\n",
                    "  1\n",
                    "end\n",
                )
                .to_string(),
            ),
            (
                "private.veln",
                "fn private_value() -> Int\n  1\nend\n".to_string(),
            ),
        ],
    );

    assert_eq!(rendered.len(), 1);
    let status = &rendered[0];
    assert_eq!(status.name, "example-dep-documentation-status");
    assert!(status.listed);
    assert!(status.uri.ends_with("/status"));
    assert!(listed.iter().any(|resource| resource["uri"] == status.uri));
    let read = read_resource(&mut server, 102, &status.uri);
    assert_eq!(read["result"]["contents"][0]["text"], status.text);
    assert!(
        status
            .text
            .contains("package_doc.unresolved_schema_reference")
    );

    let unpublished = status.uri.replace("/status", "/index");
    assert_unknown_resource_reads_rejected(&mut server, [unpublished]);
    assert!(!dependency_resource_uris(&mut server, "example/dep").is_empty());
}

#[test]
fn dependency_documentation_resource_rejections_are_exact() {
    let workspace = TempWorkspace::new("dependency-doc-rejections");
    write_workspace_with_dependency(&workspace, "retained-doc");
    let mut server = initialized_server(&workspace);
    let result = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(result["isError"], false, "{result:#}");
    let before = all_resource_state(&mut server);

    let index_uri = listed_dependency_documentation_uri(&mut server, "example/dep");
    let (snapshot_digest, doc_digest) = documentation_uri_digests_for(&index_uri, "example%2Fdep");
    let declaration_uri = linked_dependency_declaration_uri(&mut server, &index_uri);
    let wrong_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let rejected = [
        index_uri.replace(snapshot_digest, wrong_digest),
        index_uri.replace(doc_digest, wrong_digest),
        declaration_uri.replace(declaration_uri.rsplit('/').next().unwrap(), "missing"),
        format!(
            "veln-doc:///package/missing%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/index"
        ),
        format!(
            "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/module/missing"
        ),
        format!(
            "veln-doc:///package/example%2Fdep/snapshot/{snapshot_digest}/documentation/{doc_digest}/status"
        ),
        index_uri.replacen("veln-doc", "VELN-doc", 1),
        index_uri.replace("/index", "/Index"),
        format!("{index_uri}?x=1"),
    ];

    assert_unknown_resource_reads_rejected(&mut server, rejected);
    assert_eq!(all_resource_state(&mut server), before);
    assert_eq!(
        read_resource(&mut server, 103, &index_uri)["result"]["contents"][0]["uri"],
        index_uri
    );
}

#[test]
fn dependency_documentation_snapshots_coexist_and_remain_retained() {
    let workspace = TempWorkspace::new("dependency-doc-coexistence");
    write_workspace_with_dependency(&workspace, "old-doc");
    let mut server = initialized_server(&workspace);
    let first = server.check_project_tool(&json!({"project":"."}));
    assert_eq!(first["isError"], false, "{first:#}");
    let first_uri = listed_dependency_documentation_uri(&mut server, "example/dep");
    let first_text = read_resource(&mut server, 104, &first_uri)["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .to_string();

    fs::remove_dir_all(workspace.path("vendor/dep")).unwrap();
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", &dependency_source("new-doc"));
    refresh_workspace(&mut server);
    let second = server.definition_tool(&json!({"source":"main.veln","line":5,"column":3}));
    assert_eq!(second["isError"], false, "{second:#}");

    let doc_uris = dependency_documentation_uris(&mut server, "example/dep");
    assert_eq!(doc_uris.len(), 2, "{doc_uris:#?}");
    assert!(doc_uris.contains(&first_uri));
    assert_eq!(
        read_resource(&mut server, 105, &first_uri)["result"]["contents"][0]["text"],
        first_text
    );
    assert!(doc_uris.iter().any(|uri| {
        uri != &first_uri
            && read_resource(&mut server, 106, uri)["result"]["contents"][0]["text"] != first_text
    }));
}
