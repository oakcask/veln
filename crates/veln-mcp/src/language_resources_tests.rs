use super::*;
use std::path::PathBuf;
use veln_project::{Project, parse_manifest_text};
use veln_source::SourceFile;

#[test]
fn checked_resources_use_the_default_retained_package_capacity() {
    let resources = LanguageResources::checked().unwrap();

    assert_eq!(resources.retained_package_capacity(), 256);
}

#[test]
fn evicted_analysis_scope_remains_rejected_and_nonpublishing() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    resources.set_test_analyzed_dependency_scope_capacity(1);
    let scope = DependencyAnalysisScope::Project("first".to_string());
    let second_scope = DependencyAnalysisScope::Project("second".to_string());
    let first = test_dependency("example/first", "rejected first");

    resources.reject_dependency_analysis(scope.clone(), &first);
    resources.reject_dependency_analysis(second_scope, &first);

    assert_eq!(resources.retained_analyzed_dependency_scopes(), 1);
    assert!(matches!(
        resources.dependency_analysis_selection(&scope, &first),
        DependencyAnalysisSelection::Rejected(None)
    ));
    resources
        .navigation_snapshot(
            Vec::new(),
            &first,
            scope.clone(),
            json!("rejected-mutating"),
        )
        .unwrap();
    resources.read_only_navigation_snapshot(
        Vec::new(),
        &first,
        scope.clone(),
        json!("rejected-read-only"),
    );
    assert!(
        resources.list_result()["resources"]
            .as_array()
            .unwrap()
            .iter()
            .all(|resource| !resource["uri"]
                .as_str()
                .unwrap()
                .contains("example%2Ffirst"))
    );
}

#[test]
fn accepted_navigation_reuse_excludes_unanalyzed_workspace_overlays() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let scope = DependencyAnalysisScope::Project("project".to_string());
    let accepted = vec![SourceFile::new(
        "main.veln",
        "fn first() -> Int\n  1\nend\n",
    )];
    let changed = vec![SourceFile::new(
        "main.veln",
        "fn second() -> Int\n  2\nend\n",
    )];
    resources
        .admit_analyzed_dependencies(scope.clone(), accepted.clone(), &[])
        .unwrap();
    let first = resources
        .navigation_snapshot(accepted, &[], scope.clone(), json!("first"))
        .unwrap();
    let overlay = resources.read_only_navigation_snapshot(
        changed.clone(),
        &[],
        scope.clone(),
        json!("overlay"),
    );
    assert!(!Arc::ptr_eq(&first, &overlay));
    assert!(
        resources
            .workspace_navigation_caches
            .get(WorkspaceNavigationSlot::ReadOnly)
            .as_ref()
            .unwrap()
            .accepted_identity
            .is_none()
    );

    resources
        .admit_analyzed_dependencies(scope.clone(), changed.clone(), &[])
        .unwrap();
    let second = resources
        .navigation_snapshot(changed.clone(), &[], scope.clone(), json!("second"))
        .unwrap();
    assert!(!Arc::ptr_eq(&first, &second));
    let read_only =
        resources.read_only_navigation_snapshot(changed, &[], scope, json!("second-read-only"));
    assert!(Arc::ptr_eq(&second, &read_only));
}

#[test]
fn rejected_navigation_reuses_one_accepted_snapshot_across_both_cache_slots() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let scope = DependencyAnalysisScope::Project("project".to_string());
    let accepted_files = vec![SourceFile::new(
        "main.veln",
        "fn accepted() -> Int\n  dependency::value()\nend\n",
    )];
    let rejected_files = vec![SourceFile::new(
        "main.veln",
        "fn rejected() -> Int\n  changed::value()\nend\n",
    )];
    let accepted_dependencies = test_dependency("dependency", "accepted");
    let rejected_dependencies = test_dependency("changed", "rejected");
    resources
        .admit_analyzed_dependencies(
            scope.clone(),
            accepted_files.clone(),
            &accepted_dependencies,
        )
        .unwrap();
    reset_workspace_navigation_builds();
    let accepted = resources
        .navigation_snapshot(
            accepted_files,
            &accepted_dependencies,
            scope.clone(),
            json!("accepted"),
        )
        .unwrap();

    resources.reject_dependency_analysis(scope.clone(), &rejected_dependencies);
    let read_only = resources.read_only_navigation_snapshot(
        rejected_files.clone(),
        &rejected_dependencies,
        scope.clone(),
        json!("rejected-read-only"),
    );
    let mutable = resources
        .navigation_snapshot(
            rejected_files,
            &rejected_dependencies,
            scope,
            json!("rejected-mutable"),
        )
        .unwrap();

    assert!(Arc::ptr_eq(&accepted, &read_only));
    assert!(Arc::ptr_eq(&accepted, &mutable));
    assert_eq!(workspace_navigation_builds(), 1);
}

#[test]
fn rejected_analysis_discards_unaccepted_read_only_overlay() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let scope = DependencyAnalysisScope::Project("project".to_string());
    let accepted_files = vec![SourceFile::new(
        "main.veln",
        "fn accepted() -> Int\n  dependency::value()\nend\n",
    )];
    let rejected_files = vec![SourceFile::new(
        "main.veln",
        "fn rejected() -> Int\n  changed::value()\nend\n",
    )];
    let accepted_dependencies = test_dependency("dependency", "accepted");
    let rejected_dependencies = test_dependency("changed", "rejected");
    resources
        .admit_analyzed_dependencies(
            scope.clone(),
            accepted_files.clone(),
            &accepted_dependencies,
        )
        .unwrap();
    let accepted = resources
        .navigation_snapshot(
            accepted_files,
            &accepted_dependencies,
            scope.clone(),
            json!("accepted"),
        )
        .unwrap();
    let overlay = resources.read_only_navigation_snapshot(
        rejected_files.clone(),
        &accepted_dependencies,
        scope.clone(),
        json!("shared-key"),
    );
    assert!(!Arc::ptr_eq(&accepted, &overlay));

    resources.reject_dependency_analysis(scope.clone(), &rejected_dependencies);
    let retained = resources.read_only_navigation_snapshot(
        rejected_files,
        &rejected_dependencies,
        scope,
        json!("shared-key"),
    );

    assert!(Arc::ptr_eq(&accepted, &retained));
}

#[test]
fn successful_analysis_replaces_a_rejected_scope() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let scope = DependencyAnalysisScope::Project("first".to_string());
    let first = test_dependency("example/first", "rejected first");

    resources.reject_dependency_analysis(scope.clone(), &first);

    resources
        .admit_analyzed_dependencies(scope.clone(), Vec::new(), &first)
        .unwrap();
    assert!(
        resources
            .workspace_navigation_caches
            .get(WorkspaceNavigationSlot::Mutable)
            .is_none()
    );
    assert!(
        resources
            .workspace_navigation_caches
            .get(WorkspaceNavigationSlot::ReadOnly)
            .is_none()
    );
    assert!(matches!(
        resources.dependency_analysis_selection(&scope, &first),
        DependencyAnalysisSelection::Admitted(_)
    ));
    assert!(
        resources.list_result()["resources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|resource| resource["uri"]
                .as_str()
                .unwrap()
                .contains("example%2Ffirst"))
    );
}

#[test]
fn rejected_dependency_revision_tracking_remains_bounded_and_global() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    resources.set_test_rejected_dependency_revision_capacity(1);
    let first = test_dependency("example/first", "rejected first");
    let second = test_dependency("example/second", "rejected second");

    resources.reject_dependency_analysis(
        DependencyAnalysisScope::Project("first".to_string()),
        &first,
    );
    resources.reject_dependency_analysis(
        DependencyAnalysisScope::Project("second".to_string()),
        &second,
    );

    assert_eq!(resources.retained_rejected_dependency_revisions(), 1);
    assert!(matches!(
        resources.dependency_analysis_selection(
            &DependencyAnalysisScope::Project("other".to_string()),
            &first,
        ),
        DependencyAnalysisSelection::Rejected(None)
    ));
}

#[test]
fn analyzed_dependency_scopes_share_revision_payloads() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let shared = test_dependency("example/shared", "shared");
    let project_scope = DependencyAnalysisScope::Project("project-a".to_string());
    let file_scope = DependencyAnalysisScope::SingleFile {
        project: "project-a".to_string(),
        source: "first.veln".to_string(),
    };

    resources
        .admit_analyzed_dependencies(project_scope, Vec::new(), &shared)
        .unwrap();
    resources
        .admit_analyzed_dependencies(file_scope, Vec::new(), &shared)
        .unwrap();

    assert_eq!(resources.retained_analyzed_dependency_scopes(), 2);
    assert_eq!(resources.retained_analyzed_dependency_payloads(), 1);
}

#[test]
fn analyzed_dependency_scopes_evict_the_oldest_entry_at_capacity() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    resources.set_test_analyzed_dependency_scope_capacity(2);
    let shared = test_dependency("example/shared", "shared");
    let other = test_dependency("example/other", "other");
    let oldest_scope = DependencyAnalysisScope::Project("project-a".to_string());
    let retained_scope = DependencyAnalysisScope::SingleFile {
        project: "project-a".to_string(),
        source: "first.veln".to_string(),
    };
    let newest_scope = DependencyAnalysisScope::Project("project-b".to_string());

    resources
        .admit_analyzed_dependencies(oldest_scope.clone(), Vec::new(), &shared)
        .unwrap();
    resources
        .admit_analyzed_dependencies(retained_scope.clone(), Vec::new(), &shared)
        .unwrap();
    resources
        .admit_analyzed_dependencies(newest_scope.clone(), Vec::new(), &other)
        .unwrap();

    assert_eq!(resources.retained_analyzed_dependency_scopes(), 2);
    assert_eq!(resources.retained_analyzed_dependency_payloads(), 2);
    assert!(!resources.analysis_scopes.contains_key(&oldest_scope));
    assert!(resources.analysis_scopes.contains_key(&retained_scope));
    assert!(resources.analysis_scopes.contains_key(&newest_scope));
}

#[test]
fn analyzed_dependency_scopes_retire_by_project() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let shared = test_dependency("example/shared", "shared");
    let project_scope = DependencyAnalysisScope::Project("project-a".to_string());
    let retained_scope = DependencyAnalysisScope::Project("project-b".to_string());

    resources
        .admit_analyzed_dependencies(project_scope, Vec::new(), &shared)
        .unwrap();
    resources
        .admit_analyzed_dependencies(retained_scope.clone(), Vec::new(), &shared)
        .unwrap();

    resources.retain_analyzed_dependency_projects(&["project-b".to_string()]);

    assert_eq!(resources.retained_analyzed_dependency_scopes(), 1);
    assert_eq!(resources.retained_analyzed_dependency_payloads(), 1);
    assert!(resources.analysis_scopes.contains_key(&retained_scope));
}

#[test]
fn shared_analyzed_dependencies_restore_their_documentation_selection() {
    let mut resources = LanguageResources::for_test(Vec::new(), Vec::new());
    let first = test_dependency("example/first", "first");
    let mut extended = first.clone();
    extended.extend(test_dependency("example/second", "second"));

    resources
        .admit_analyzed_dependencies(
            DependencyAnalysisScope::Project("project-a".to_string()),
            Vec::new(),
            &first,
        )
        .unwrap();
    let first_revision = current_package_revision(&resources, "example/first");

    resources
        .admit_analyzed_dependencies(
            DependencyAnalysisScope::Project("project-b".to_string()),
            Vec::new(),
            &extended,
        )
        .unwrap();
    assert_ne!(
        current_package_revision(&resources, "example/first"),
        first_revision
    );

    resources
        .admit_analyzed_dependencies(
            DependencyAnalysisScope::Project("project-c".to_string()),
            Vec::new(),
            &first,
        )
        .unwrap();
    assert_eq!(
        current_package_revision(&resources, "example/first"),
        first_revision
    );
    assert_eq!(resources.retained_analyzed_dependency_payloads(), 2);
}

fn current_package_revision(
    resources: &LanguageResources,
    identity: &str,
) -> DocumentationRevisionKey {
    resources
        .current_package_docs
        .iter()
        .find(|(key, _)| key.identity == identity)
        .map(|(_, revision)| revision.clone())
        .unwrap()
}

fn test_dependency(identity: &str, body: &str) -> Vec<CapturedDependencyProject> {
    vec![CapturedDependencyProject {
        package: identity.to_string(),
        source: format!("vendor/{identity}"),
        project: Some(Project {
            root: PathBuf::new(),
            files: vec![SourceFile::new(
                "dep.veln",
                format!("pub fn value() -> Int\n  # {body}\n  1\nend\n"),
            )],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &format!("[package]\nname = \"{identity}\"\n[lib]\nexports = [\"dep.veln\"]\n"),
            )),
        }),
    }]
}

#[test]
fn checked_resources_retain_the_validated_standard_library_snapshot() {
    let resources = LanguageResources::checked().unwrap();

    assert!(resources.standard_library_snapshot().is_some());
}

#[test]
fn checked_resources_publish_each_owned_resource_family() {
    let resources = LanguageResources::checked().unwrap();
    let listed = resources.list_result();
    let uris = listed
        .get("resources")
        .and_then(Value::as_array)
        .unwrap()
        .iter()
        .filter_map(|resource| resource.get("uri").and_then(Value::as_str))
        .collect::<Vec<_>>();

    assert!(
        uris.iter()
            .any(|uri| uri.starts_with("veln-doc:///language/"))
    );
    assert!(uris.iter().any(|uri| uri.starts_with("veln-pkg:///std/")));
    assert!(
        uris.iter()
            .any(|uri| uri.starts_with("veln-doc:///package/std/"))
    );
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
fn checked_standard_library_resources_load_prebuilt_search_index() {
    let standard_library = StandardLibraryResources::from_checked_embedded_inputs().unwrap();

    assert!(
        !standard_library
            .package_documentation
            .search_candidates(&standard_library.key)
            .is_empty()
    );
}

#[test]
fn checked_standard_library_resources_are_built_once() {
    let before = standard_library_resource_builds();

    StandardLibraryResources::checked().unwrap();
    StandardLibraryResources::checked().unwrap();

    assert!(standard_library_resource_builds() - before <= 1);
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
fn checked_resources_return_independent_reference_pagination_state() {
    let mut first = LanguageResources::checked().unwrap();
    let mut second = LanguageResources::checked().unwrap();
    let first_cursor = match first.reference_pagination().initial_page(
            vec![
                serde_json::json!({"uri":"file://a","range":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}}}),
                serde_json::json!({"uri":"file://b","range":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}}}),
            ],
            serde_json::json!({}),
            1,
        ) {
        crate::outcome::ToolOutcome::Success(value) => value["next_cursor"].as_str().unwrap().to_owned(),
        crate::outcome::ToolOutcome::DomainFailure { .. } => panic!("expected page"),
    };
    let second_cursor = match second.reference_pagination().initial_page(
        vec![
            serde_json::json!({"uri":"file://a","range":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}}}),
            serde_json::json!({"uri":"file://b","range":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}}}),
        ],
        serde_json::json!({}),
        1,
    ) {
        crate::outcome::ToolOutcome::Success(value) => value["next_cursor"].as_str().unwrap().to_owned(),
        crate::outcome::ToolOutcome::DomainFailure { .. } => panic!("expected page"),
    };

    assert_ne!(first_cursor, second_cursor);
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
