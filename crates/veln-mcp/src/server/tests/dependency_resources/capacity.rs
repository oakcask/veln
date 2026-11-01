use super::*;

#[test]
fn dependency_resource_capacity_is_atomic() {
    single_dependency_over_capacity_is_atomic();
    multiple_dependency_over_capacity_is_atomic();
}

#[test]
fn dependency_admission_resolves_worst_position_imports_with_linear_work() {
    let mut evidence = Vec::new();
    for importer_count in [8, 16, 32] {
        let dependencies = shared_dependency_projects(importer_count);
        let mut resources = LanguageResources::checked().unwrap();
        resources.set_test_remaining_package_capacity(dependencies.len());
        crate::language_resources::reset_dependency_preparation_work();

        let started = Instant::now();
        resources.admit_dependencies(&dependencies).unwrap();
        let elapsed = started.elapsed();
        let work = crate::language_resources::dependency_preparation_work();

        assert_eq!(work.snapshot_captures, dependencies.len());
        assert_eq!(work.package_doc_generations, dependencies.len());
        assert_eq!(work.semantic_indexed_projects, dependencies.len());
        assert_eq!(work.semantic_exact_probes, importer_count);
        assert_eq!(work.semantic_local_root_probes, importer_count);
        evidence.push((importer_count, work, elapsed));
    }

    for adjacent in evidence.windows(2) {
        let [(smaller_count, smaller, _), (larger_count, larger, _)] = adjacent else {
            unreachable!("adjacent evidence has exactly two entries")
        };
        assert_eq!(*larger_count, smaller_count * 2);
        assert_eq!(
            semantic_resolution_work(larger),
            semantic_resolution_work(smaller) * 2 - 1
        );
    }
    eprintln!("dependency admission semantic-resolution evidence: {evidence:?}");
}

fn semantic_resolution_work(work: &crate::language_resources::DependencyPreparationWork) -> usize {
    work.semantic_indexed_projects + work.semantic_exact_probes + work.semantic_local_root_probes
}

#[test]
fn over_capacity_dependency_admission_stops_after_linear_preparation() {
    let dependencies = fixed_width_dependency_projects(32);
    let mut resources = resources_with_retained_seed();
    resources.set_test_remaining_package_capacity(0);
    let retained = RetainedDependencyState::capture(&resources);
    crate::language_resources::reset_dependency_preparation_work();

    let started = Instant::now();
    let error = resources.admit_dependencies(&dependencies).unwrap_err();
    let elapsed = started.elapsed();
    let work = crate::language_resources::dependency_preparation_work();

    assert_eq!(error, crate::language_resources::ResourceCapacityError);
    assert_eq!(work.snapshot_captures, dependencies.len());
    assert_eq!(work.package_doc_generations, 0);
    assert_eq!(work.semantic_indexed_projects, dependencies.len());
    assert_eq!(work.semantic_exact_probes, 0);
    assert_eq!(work.semantic_local_root_probes, 0);
    retained.assert_unchanged(&resources);
    eprintln!(
        "over-capacity dependency preparation evidence: count={}, work={work:?}, elapsed={elapsed:?}",
        dependencies.len()
    );
}

fn resources_with_retained_seed() -> LanguageResources {
    let mut resources = LanguageResources::checked().unwrap();
    resources.set_test_remaining_package_capacity(1);
    resources
        .admit_dependencies(&[synthetic_dependency_project(
            "example/seed-0000",
            "retained seed",
        )])
        .unwrap();
    resources
}

struct RetainedDependencyState {
    list: Value,
    search: Vec<crate::language_resources::PackageSearchCandidate>,
    reads: Vec<(String, Value)>,
}

impl RetainedDependencyState {
    fn capture(resources: &LanguageResources) -> Self {
        let list = resources.list_result();
        let search = resources.package_search_candidates();
        let mut reads = retained_seed_reads(resources, &list);
        reads.extend(linked_seed_documentation_reads(resources, &reads));
        Self {
            list,
            search,
            reads,
        }
    }

    fn assert_unchanged(self, resources: &LanguageResources) {
        assert_eq!(resources.list_result(), self.list);
        assert_eq!(resources.package_search_candidates(), self.search);
        for (uri, read) in self.reads {
            assert_eq!(resources.read_result(&uri), Some(read), "{uri}");
        }
    }
}

fn retained_seed_reads(resources: &LanguageResources, list: &Value) -> Vec<(String, Value)> {
    list["resources"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|resource| resource["uri"].as_str())
        .filter(|uri| uri.contains("/example%2Fseed-0000/"))
        .map(|uri| (uri.to_string(), resources.read_result(uri).unwrap()))
        .collect()
}

fn linked_seed_documentation_reads(
    resources: &LanguageResources,
    retained_reads: &[(String, Value)],
) -> [(String, Value); 2] {
    let index_text = retained_reads
        .iter()
        .find(|(uri, _)| uri.ends_with("/index"))
        .unwrap()
        .1["contents"][0]["text"]
        .as_str()
        .unwrap();
    let module_uri = linked_package_documentation_uri(index_text, "/module/");
    let module_read = resources.read_result(&module_uri).unwrap();
    let module_text = module_read["contents"][0]["text"].as_str().unwrap();
    let declaration_uri = linked_package_documentation_uri(module_text, "/declaration/");
    let declaration_read = resources.read_result(&declaration_uri).unwrap();
    [
        (module_uri, module_read),
        (declaration_uri, declaration_read),
    ]
}

fn linked_package_documentation_uri(text: &str, kind: &str) -> String {
    extract_uris_with_prefix(text, "veln-doc:///package/")
        .into_iter()
        .find(|uri| uri.contains(kind))
        .unwrap()
}

fn fixed_width_dependency_projects(size: usize) -> Vec<CapturedDependencyProject> {
    (0..size)
        .map(|index| synthetic_dependency_project(&format!("example/dep-{index:04}"), "fixed body"))
        .collect()
}

fn shared_dependency_projects(importer_count: usize) -> Vec<CapturedDependencyProject> {
    let mut dependencies = (0..importer_count)
        .map(|index| {
            let identity = format!("example/importer-{index:04}");
            let root = format!("vendor/importer-{index:04}");
            let manifest = format!(
                "[package]\nname = \"{}\"\n\n[lib]\nexports = [\"dep.veln\"]\n\n[dependencies.\"example/shared\"]\npath = \"../shared\"\n",
                identity
            );
            CapturedDependencyProject {
                package: identity,
                source: root.clone(),
                project: Some(Project {
                    root: PathBuf::from(root),
                    files: vec![SourceFile::new(
                        "dep.veln",
                        "use shared from \"example/shared\"\npub let exposed: Int = shared::answer\n",
                    )],
                    manifest: Some(parse_manifest_text("veln.toml", &manifest)),
                }),
            }
        })
        .collect::<Vec<_>>();
    dependencies.push(CapturedDependencyProject {
        package: "example/shared".to_string(),
        source: "vendor/shared".to_string(),
        project: Some(Project {
            root: PathBuf::from("vendor/shared"),
            files: vec![SourceFile::new("dep.veln", "pub let answer: Int = 1\n")],
            manifest: Some(parse_manifest_text(
                "veln.toml",
                &dependency_manifest("example/shared"),
            )),
        }),
    });
    dependencies
}

fn single_dependency_over_capacity_is_atomic() {
    let mut resources = LanguageResources::checked().unwrap();
    resources.set_test_remaining_package_capacity(2);
    let boundary = (0..2)
        .map(|index| synthetic_dependency_project(&format!("example/dep{index}"), "body"))
        .collect::<Vec<_>>();
    resources.admit_dependencies(&boundary).unwrap();
    let before = resources.list_result();

    let over_capacity = vec![synthetic_dependency_project("example/overflow", "body")];
    let error = resources.admit_dependencies(&over_capacity).unwrap_err();
    assert_eq!(error, crate::language_resources::ResourceCapacityError);
    assert_eq!(resources.list_result(), before);
}

fn multiple_dependency_over_capacity_is_atomic() {
    let mut resources = LanguageResources::checked().unwrap();
    resources.set_test_remaining_package_capacity(2);
    let boundary = (0..1)
        .map(|index| synthetic_dependency_project(&format!("example/multi{index}"), "body"))
        .collect::<Vec<_>>();
    resources.admit_dependencies(&boundary).unwrap();
    let before = resources.list_result();

    let over_capacity = vec![
        synthetic_dependency_project("example/overflow-a", "body"),
        synthetic_dependency_project("example/overflow-b", "body"),
    ];
    let error = resources.admit_dependencies(&over_capacity).unwrap_err();
    assert_eq!(error, crate::language_resources::ResourceCapacityError);
    assert_eq!(resources.list_result(), before);
    assert!(!resource_uri_prefix_is_listed(
        &resources,
        "example/overflow-a"
    ));
    assert!(!resource_uri_prefix_is_listed(
        &resources,
        "example/overflow-b"
    ));
}

#[test]
fn saved_project_capacity_failures_match_advertised_result_schemas() {
    struct Case {
        name: &'static str,
        call: fn(&mut Server) -> Value,
    }

    let cases = [
        Case {
            name: "check_project",
            call: |server| server.check_project_tool(&json!({"project":"."})),
        },
        Case {
            name: "definition",
            call: |server| {
                server.definition_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
        Case {
            name: "references",
            call: |server| {
                server.references_tool(&json!({"source":"main.veln","line":4,"column":8}))
            },
        },
    ];

    for case in cases {
        let workspace = TempWorkspace::new(case.name);
        write_workspace_with_dependency(&workspace, "overflow");
        let mut server = initialized_server_with_embedded_resources(&workspace);
        server
            .language_resources
            .set_test_remaining_package_capacity(2);
        let boundary = (0..2)
            .map(|index| synthetic_dependency_project(&format!("example/full{index}"), "body"))
            .collect::<Vec<_>>();
        server
            .language_resources
            .admit_dependencies(&boundary)
            .unwrap();
        let before = all_resource_state(&mut server);

        let result = (case.call)(&mut server);
        assert_eq!(result["isError"], true, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["code"], "resource_capacity",
            "{}: {result:#}",
            case.name
        );
        assert!(
            schema::tool(case.name)
                .unwrap()
                .accepts_result(&result["structuredContent"]),
            "{}: {result:#}",
            case.name
        );
        assert_eq!(all_resource_state(&mut server), before, "{}", case.name);
    }
}

#[test]
fn references_capacity_failure_preserves_a_prior_live_cursor() {
    let workspace = TempWorkspace::new("references-capacity-preservation");
    write_workspace_with_dependency(&workspace, "overflow");
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn main() -> Int\n  dep::value()\n  dep::value()\nend\n",
    );
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let first = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":8, "page_size":1,
        "include_declaration": true
    }));
    let cursor = first["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    fill_dependency_resource_capacity(&mut server);
    let before_failure = all_resource_state(&mut server);
    workspace.write("vendor/dep/dep.veln", &dependency_source("rejected digest"));
    assert_reference_capacity_failure(&mut server);
    assert_eq!(all_resource_state(&mut server), before_failure);
    assert_live_reference_cursor(&mut server, &cursor);
}

fn fill_dependency_resource_capacity(server: &mut Server) {
    server
        .language_resources
        .set_test_remaining_package_capacity(1);
    let boundary = [synthetic_dependency_project("example/full", "body")];
    server
        .language_resources
        .admit_dependencies(&boundary)
        .unwrap();
}

pub(in crate::server::tests) fn fill_dependency_resource_capacity_completely(server: &mut Server) {
    server
        .language_resources
        .set_test_remaining_package_capacity(2);
    let boundary = (0..2)
        .map(|index| synthetic_dependency_project(&format!("example/rename-full{index}"), "body"))
        .collect::<Vec<_>>();
    server
        .language_resources
        .admit_dependencies(&boundary)
        .unwrap();
}

fn assert_reference_capacity_failure(server: &mut Server) {
    let initial_failure = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":8, "include_declaration": true
    }));
    assert_eq!(initial_failure["isError"], true);
    assert_eq!(
        initial_failure["structuredContent"]["code"],
        "resource_capacity"
    );
    assert!(
        !initial_failure["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("references")
    );
    assert!(
        !initial_failure["structuredContent"]
            .as_object()
            .unwrap()
            .contains_key("scope")
    );
}

fn assert_live_reference_cursor(server: &mut Server, cursor: &str) {
    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false);
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
