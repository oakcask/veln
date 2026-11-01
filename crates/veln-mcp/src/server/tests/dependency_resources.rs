use super::*;
use std::collections::BTreeSet;
use std::time::Instant;
use veln_analysis::CapturedDependencyProject;
use veln_language_service::{
    PackageDocGeneratorContract, PackageDocResult, VirtualSourceCatalog,
    render_package_documentation,
};
use veln_project::{
    PackageIdentity, PackageSnapshotSource, Project, capture_embedded_package_snapshot,
    parse_manifest_text,
};
use veln_source::SourceFile;

mod capacity;
mod documentation;
mod lifecycle;

pub(super) use capacity::fill_dependency_resource_capacity_completely;
use lifecycle::refresh_workspace;

fn assert_resources_are_sorted(resources: &[Value]) {
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<Vec<_>>();
    let mut sorted = uris.clone();
    sorted.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    assert_eq!(uris, sorted);
}
fn read_resource(server: &mut Server, id: u64, uri: &str) -> Value {
    server
        .handle_request(
            json!({"jsonrpc":"2.0","id":id,"method":"resources/read","params":{"uri":uri}}),
        )
        .unwrap()
}
fn assert_unknown_resource_reads_rejected(
    server: &mut Server,
    unknown_uris: impl IntoIterator<Item = String>,
) {
    for uri in unknown_uris {
        let response = server
            .handle_request(
                json!({"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":uri}}),
            )
            .unwrap();
        assert_eq!(response["error"]["code"], -32002, "{uri}");
        assert_eq!(response["error"]["data"]["code"], "resource_not_found");
        assert!(response.get("result").is_none());
    }
}

fn standard_library_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"state-list","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|resource| {
            resource["uri"]
                .as_str()
                .unwrap()
                .starts_with("veln-pkg:///std/snapshot/")
        })
        .cloned()
        .collect::<Vec<_>>();
    let reads = resources
        .iter()
        .map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            server
                .handle_request(json!({"jsonrpc":"2.0","id":"state-read","method":"resources/read","params":{"uri":uri}}))
                .unwrap()["result"]["contents"][0]
                .clone()
        })
        .collect::<Vec<_>>();
    json!({"resources": resources, "reads": reads})
}

fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"all-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

fn dependency_resource_state(server: &mut Server, identity: &str, name: &str) -> Value {
    dependency_resource_states(server, identity, name)
        .into_iter()
        .next()
        .unwrap()
}

fn dependency_resource_states(server: &mut Server, identity: &str, name: &str) -> Vec<Value> {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"dep-list","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|resource| {
            resource["uri"].as_str().unwrap().starts_with(&format!(
                "veln-pkg:///{}/snapshot/",
                identity.replace('/', "%2F")
            )) && resource["name"] == name
        })
        .cloned()
        .collect::<Vec<_>>();
    resources
        .into_iter()
        .map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            let read = read_resource(server, 50, uri)["result"]["contents"][0].clone();
            json!({"metadata": resource, "read": read})
        })
        .collect()
}

fn dependency_resource_texts(states: &[Value]) -> BTreeSet<String> {
    states
        .iter()
        .map(|state| state["read"]["text"].as_str().unwrap().to_string())
        .collect()
}

fn dependency_resource_uris(server: &mut Server, identity: &str) -> Vec<String> {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"dep-uris","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect()
}

fn resource_uri_prefix_is_listed(resources: &LanguageResources, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    resources.list_result()["resources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

fn write_workspace_with_dependency(workspace: &TempWorkspace, body: &str) {
    write_workspace_with_dependency_source(workspace, &dependency_source(body));
}

fn write_workspace_with_dependency_source(workspace: &TempWorkspace, source: &str) {
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\nend\n",
    );
    workspace.write("vendor/dep/veln.toml", &dependency_manifest("example/dep"));
    workspace.write("vendor/dep/dep.veln", source);
    workspace.write(
        "vendor/dep/private.veln",
        "fn private_value() -> Int\n  1\nend\n",
    );
    workspace.write(
        "vendor/dep/dep_test.veln",
        "test excluded() -> Int\n  1\nend\n",
    );
}

fn dependency_manifest(identity: &str) -> String {
    format!("[package]\nname = \"{identity}\"\n\n[lib]\nexports = [\"dep.veln\"]\n")
}

fn dependency_source(body: &str) -> String {
    format!("pub fn value() -> Int\n  # {body}\n  1\nend\n")
}

fn synthetic_dependency_project(identity: &str, body: &str) -> CapturedDependencyProject {
    CapturedDependencyProject {
        package: identity.to_string(),
        source: format!("vendor/{identity}"),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new("dep.veln", dependency_source(body))],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &dependency_manifest(identity),
            )),
        }),
    }
}

fn expected_resource_metadata() -> Vec<Value> {
    let catalog: Value =
        serde_json::from_str(veln_repo_language_reference::checked_catalog_bytes()).unwrap();
    let digest = veln_repo_language_reference::checked_catalog_digest();
    let base = format!("veln-doc:///language/snapshot/{digest}");
    let mut resources = vec![json!({
        "uri": format!("{base}/index"),
        "name": "language-index",
        "title": "Veln Language Reference",
        "mimeType": veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
    })];
    for topic in catalog["topics"].as_array().unwrap() {
        let id = topic["id"].as_str().unwrap();
        resources.push(json!({
            "uri": format!("{base}/topic/{id}"),
            "name": id,
            "title": topic["title"].as_str().unwrap(),
            "description": topic["summary"].as_str().unwrap(),
            "mimeType": veln_repo_language_reference::LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE,
        }));
    }
    let standard_library = crate::language_resources::StandardLibraryResources::checked()
        .unwrap()
        .resources;
    resources.extend(
        standard_library
            .iter()
            .filter(|resource| resource.listed)
            .map(crate::language_resources::PublishedResource::metadata),
    );
    sort_resource_metadata(&mut resources);
    resources
}

fn expected_dependency_resource_metadata(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<Value> {
    let identity = PackageIdentity::new(identity).unwrap();
    let manifest = dependency_manifest(identity.as_str());
    let source_inputs = sources.into_iter().collect::<Vec<_>>();
    let snapshot = capture_embedded_package_snapshot(
        manifest.as_bytes(),
        source_inputs
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let catalog = VirtualSourceCatalog::new([(identity.clone(), snapshot.clone())]).unwrap();
    snapshot
        .sources()
        .iter()
        .enumerate()
        .map(|(source_index, source)| {
            let entry = catalog.entry_for_source(0, source_index).unwrap();
            json!({
                "uri": entry.uri(),
                "name": source.path(),
                "title": format!("Veln package source: {}: {}", identity.as_str(), source.path()),
                "mimeType": "text/x-veln; charset=utf-8",
            })
        })
        .collect()
}

fn expected_dependency_documentation_metadata(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<Value> {
    expected_dependency_documentation_resources(identity, sources)
        .into_iter()
        .filter(|resource| resource.listed)
        .map(|resource| {
            let mut value = json!({
                "uri": resource.uri,
                "name": resource.name,
                "title": resource.title,
                "mimeType": resource.mime_type,
            });
            if let Some(description) = resource.description {
                value["description"] = json!(description);
            }
            value
        })
        .collect()
}

fn expected_dependency_documentation_resources(
    identity: &str,
    sources: impl IntoIterator<Item = (&'static str, String)>,
) -> Vec<veln_language_service::RenderedPackageDocResource> {
    let identity = PackageIdentity::new(identity).unwrap();
    let manifest = parse_manifest_text("veln.toml", &dependency_manifest(identity.as_str()));
    let source_inputs = sources.into_iter().collect::<Vec<_>>();
    let snapshot = capture_embedded_package_snapshot(
        manifest.source_bytes.as_slice(),
        source_inputs
            .iter()
            .map(|(path, text)| PackageSnapshotSource::new(path, text.as_bytes())),
    )
    .unwrap();
    let result = PackageDocResult::generate(
        &identity,
        &snapshot,
        &manifest,
        std::path::Path::new("."),
        &[],
        PackageDocGeneratorContract::new(veln_repo_mcp_standard_library_docs::GENERATOR_CONTRACT),
    );
    render_package_documentation(&result)
}

fn sort_resource_metadata(resources: &mut [Value]) {
    resources.sort_by(|left, right| {
        left["uri"]
            .as_str()
            .unwrap()
            .as_bytes()
            .cmp(right["uri"].as_str().unwrap().as_bytes())
    });
}

fn listed_resources(server: &mut Server) -> Vec<Value> {
    server
        .handle_request(json!({"jsonrpc":"2.0","id":"listed","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone()
}

fn listed_dependency_source_uris(server: &mut Server, identity: &str) -> Vec<String> {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    listed_resources(server)
        .into_iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect()
}

fn package_search(server: &mut Server, query: &str) -> Vec<String> {
    server
        .handle_request(json!({
            "jsonrpc":"2.0",
            "id":"dependency-package-search",
            "method":"tools/call",
            "params":{
                "name":"search_docs",
                "arguments":{"query":query,"scope":"package","limit":50}
            }
        }))
        .unwrap()["result"]["structuredContent"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["uri"].as_str().unwrap().to_string())
        .collect()
}

fn listed_dependency_documentation_uri(server: &mut Server, identity: &str) -> String {
    dependency_documentation_uris(server, identity)
        .into_iter()
        .next()
        .unwrap()
}

fn dependency_documentation_uris(server: &mut Server, identity: &str) -> Vec<String> {
    let prefix = format!(
        "veln-doc:///package/{}/snapshot/",
        identity.replace('/', "%2F")
    );
    listed_resources(server)
        .into_iter()
        .filter_map(|resource| {
            let uri = resource["uri"].as_str().unwrap();
            uri.starts_with(&prefix).then(|| uri.to_string())
        })
        .collect()
}

fn documentation_uri_digests_for<'a>(
    index_uri: &'a str,
    encoded_identity: &str,
) -> (&'a str, &'a str) {
    let prefix = format!("veln-doc:///package/{encoded_identity}/snapshot/");
    let rest = index_uri.strip_prefix(&prefix).unwrap();
    let mut parts = rest.split('/');
    let snapshot_digest = parts.next().unwrap();
    assert_eq!(parts.next(), Some("documentation"));
    (snapshot_digest, parts.next().unwrap())
}

fn linked_dependency_declaration_uri(server: &mut Server, index_uri: &str) -> String {
    let module_uri = linked_dependency_module_uri(server, index_uri);
    let module = read_resource(server, 108, &module_uri);
    let module_text = module["result"]["contents"][0]["text"].as_str().unwrap();
    extract_uris_with_prefix(module_text, "veln-doc:///package/")
        .into_iter()
        .find(|uri| uri.contains("/declaration/"))
        .unwrap()
}

fn linked_dependency_module_uri(server: &mut Server, index_uri: &str) -> String {
    let index = read_resource(server, 107, index_uri);
    let index_text = index["result"]["contents"][0]["text"].as_str().unwrap();
    extract_uris_with_prefix(index_text, "veln-doc:///package/")
        .into_iter()
        .find(|uri| uri.contains("/module/"))
        .unwrap()
}

fn write_context_revision_workspace(workspace: &TempWorkspace, dependency_value: &str) {
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
        "use api from \"a\"\nfn main() -> Int\n  api::anchor()\nend\n",
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
        concat!(
            "use shared from \"b\"\n",
            "pub let exposed: Int = shared::answer\n",
            "pub fn anchor() -> Int\n",
            "  1\n",
            "end\n",
        ),
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
    workspace.write("vendor/b/shared.veln", dependency_value);
}

fn extract_uris_with_prefix(text: &str, prefix: &str) -> BTreeSet<String> {
    let mut uris = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find(prefix) {
        let after_start = &rest[start..];
        let end = after_start
            .find(|character: char| character == ')' || character.is_whitespace())
            .unwrap_or(after_start.len());
        uris.insert(after_start[..end].to_string());
        rest = &after_start[end..];
    }
    uris
}

fn assert_resource_uris_are_unique(resources: &[Value], case: &str) {
    let uris = resources
        .iter()
        .map(|resource| resource["uri"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(uris.len(), resources.len(), "{case}");
}
