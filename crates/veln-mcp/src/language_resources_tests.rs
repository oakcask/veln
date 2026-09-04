use super::*;

#[test]
fn checked_resources_retain_the_validated_standard_library_snapshot() {
    let resources = LanguageResources::checked().unwrap();

    assert!(resources.standard_library_snapshot().is_some());
}

#[test]
fn checked_standard_library_resources_load_the_prebuilt_documentation_bundle() {
    let standard_library = StandardLibraryResources::from_checked_embedded_inputs().unwrap();
    let checked = veln_repo_mcp_standard_library_docs::checked_bundle().unwrap();
    let documentation = standard_library
        .resources
        .iter()
        .filter(|resource| resource.uri.starts_with("veln-doc:///package/std/"))
        .collect::<Vec<_>>();

    assert_eq!(documentation.len(), checked.resources.len());
    for (published, embedded) in documentation.into_iter().zip(checked.resources) {
        assert_eq!(published.uri, embedded.uri);
        assert_eq!(published.name, embedded.name);
        assert_eq!(published.title, embedded.title);
        assert_eq!(published.description, embedded.description);
        assert_eq!(published.mime_type, embedded.mime_type);
        assert_eq!(published.text, embedded.text);
        assert_eq!(published.listed, embedded.listed);
    }
}

#[test]
fn checked_resources_return_independent_mutable_state() {
    let mut first = LanguageResources::checked().unwrap();
    let second = LanguageResources::checked().unwrap();
    let unique = RetainedPackageKey {
        identity: "test/package".to_string(),
        digest: "unique".to_string(),
    };

    first.retained_package_keys.insert(unique.clone());

    assert!(!second.retained_package_keys.contains(&unique));
}

#[test]
fn standard_library_capture_rejects_invalid_embedded_inputs() {
    let error = StandardLibraryResources::from_embedded_inputs(
        "name = \"std\"\n",
        [PackageSnapshotSource::new(
            "bad/../main.veln",
            b"pub fn main() -> Int\n  1\nend\n",
        )],
    )
    .unwrap_err();

    assert!(error.contains("capture embedded standard library snapshot"));
}

#[test]
fn standard_library_capture_rejects_invalid_manifest_identity() {
    let error = StandardLibraryResources::from_embedded_inputs(
        "name = \"other\"\n",
        [PackageSnapshotSource::new(
            "main.veln",
            b"pub fn main() -> Int\n  1\nend\n",
        )],
    )
    .unwrap_err();

    assert!(error.contains("validate embedded standard library snapshot"));
}

#[test]
fn standard_library_capture_propagates_catalog_construction_failure() {
    let error = StandardLibraryResources::from_embedded_inputs_with_catalog_builder(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub fn main() -> Int\n  1\nend\n",
        )],
        |_identity, _snapshot| {
            Err(
                "build embedded standard library source catalog: injected construction failure"
                    .to_string(),
            )
        },
    )
    .unwrap_err();

    assert!(error.contains("build embedded standard library source catalog"));
    assert!(error.contains("injected construction failure"));
}

#[test]
fn standard_library_documentation_failure_publishes_only_status_documentation() {
    let resources = StandardLibraryResources::from_embedded_inputs(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub fn broken(\n  1\nend\n",
        )],
    )
    .unwrap()
    .resources;
    let doc_resources = resources
        .iter()
        .filter(|resource| resource.uri.starts_with("veln-doc:///package/std/"))
        .collect::<Vec<_>>();

    assert_eq!(doc_resources.len(), 1);
    assert!(doc_resources[0].listed);
    assert!(doc_resources[0].uri.ends_with("/status"));
    assert_eq!(doc_resources[0].name, "std-documentation-status");
    assert!(doc_resources[0].text.contains("- State: failed"));
    assert!(doc_resources[0].text.contains("- Gate: parse"));
}
