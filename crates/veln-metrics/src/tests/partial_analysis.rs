use super::*;

#[test]
fn partial_source_analysis_excludes_invalid_identity_from_graph_but_keeps_path_subjects() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new("app.veln", "use App\nfn main() -> ()\n  ()\nend\n"),
            SourceFile::new("App.veln", "pub fn entry() -> Int\n  1\nend\n"),
            SourceFile::new("probe.veln", "fn probe() -> Int\n  1\nend\n"),
        ],
    };
    let diagnostics = source_graph_diagnostics(&project);
    let partial = partial_source_analysis(&project, &diagnostics).expect("partial analysis");
    let graph_project = project_without_paths(project.clone(), &partial.excluded_paths);
    let graph = DependencyGraph::from_project(&graph_project).expect("retained graph");
    let selected = ["App.veln", "app.veln", "probe.veln"]
        .into_iter()
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let report = graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: selected.iter().cloned().collect(),
        },
        &selected,
        default_metrics_config(),
        partial.diagnostics,
        partial.completeness,
    );

    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].id, "name.invalid_case");
    assert_eq!(
        report.completeness.excluded_sources,
        vec![ExcludedSource {
            path: "App.veln".to_string(),
            reason: "invalid_module_identity".to_string()
        }]
    );
    assert_eq!(
        report
            .modules
            .iter()
            .map(|module| module.module.as_str())
            .collect::<Vec<_>>(),
        ["app", "probe"]
    );
    assert!(report.edges.is_empty());
    assert_eq!(report.summary.project_module_count, 2);
    assert_eq!(
        report
            .abc_subjects
            .iter()
            .map(|subject| subject.identity.as_str())
            .collect::<Vec<_>>(),
        ["App.veln::entry", "app.veln::main", "probe.veln::probe"]
    );
}

#[test]
fn partial_source_analysis_rejects_unrelated_source_errors() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new(
                "app.veln",
                "use App\nuse missing::thing\nfn main() -> ()\n  ()\nend\n",
            ),
            SourceFile::new("App.veln", "pub fn entry() -> Int\n  1\nend\n"),
        ],
    };
    let diagnostics = source_graph_diagnostics(&project);

    let rejected = partial_source_analysis(&project, &diagnostics).expect_err("source errors");

    assert!(rejected.iter().any(|diagnostic| {
        diagnostic.id == "module.unresolved_import"
            && json_string_field(&diagnostic.details, "module_path") == Some("missing::thing")
    }));
}

#[test]
fn partial_source_analysis_uses_source_kind_aware_excluded_identities() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new(
                "app.veln",
                "use App\nuse App__test_companion\nfn main() -> Int\n  1\nend\n",
            ),
            SourceFile::new("App.veln", "pub fn entry() -> Int\n  1\nend\n"),
            SourceFile::new(
                "App.test.veln",
                "use app\ntest companion_imports_retained_source() -> Int\n  app::main()\nend\n",
            ),
        ],
    };
    let diagnostics = source_graph_diagnostics(&project);

    let partial = partial_source_analysis(&project, &diagnostics).expect("partial analysis");

    assert_eq!(
        partial
            .diagnostics
            .iter()
            .map(|diagnostic| {
                (
                    diagnostic.id.as_str(),
                    json_string_field(&diagnostic.details, "source_path"),
                    json_string_field(&diagnostic.details, "source_kind"),
                )
            })
            .collect::<Vec<_>>(),
        [
            ("name.invalid_case", Some("App.veln"), Some("regular")),
            (
                "name.invalid_case",
                Some("App.test.veln"),
                Some("companion")
            )
        ]
    );
    assert_eq!(
        partial.excluded_paths,
        ["App.test.veln".to_string(), "App.veln".to_string()]
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn partial_source_analysis_rejects_missing_imports_from_excluded_sources() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new("app.veln", "use App\nfn main() -> ()\n  ()\nend\n"),
            SourceFile::new(
                "App.veln",
                "use missing::excluded\npub fn entry() -> Int\n  1\nend\n",
            ),
        ],
    };
    let diagnostics = source_graph_diagnostics(&project);

    let rejected = partial_source_analysis(&project, &diagnostics).expect_err("source errors");

    assert!(rejected.iter().any(|diagnostic| {
        diagnostic.id == "module.unresolved_import"
            && diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.file.as_str() == "App.veln")
            && json_string_field(&diagnostic.details, "module_path") == Some("missing::excluded")
    }));
}

#[test]
fn partial_check_result_is_incomplete_unless_known_cycle_fails() {
    let mut acyclic = report_from_edges(&[]);
    mark_report_partial(&mut acyclic, "App.veln");
    let acyclic_check = evaluate_metrics_check(acyclic, MetricsPolicy { deny_cycles: true });
    let acyclic_json = report_check_to_json(&acyclic_check, tool_info()).to_json();

    assert!(!acyclic_check.has_violations());
    assert!(acyclic_json.contains("\"status\":\"incomplete\""));
    assert!(acyclic_json.contains("\"result\":\"incomplete\""));

    let mut cyclic = report_from_edges(&[("app", "util"), ("util", "app")]);
    mark_report_partial(&mut cyclic, "App.veln");
    let cyclic_check = evaluate_metrics_check(cyclic, MetricsPolicy { deny_cycles: true });
    let cyclic_json = report_check_to_json(&cyclic_check, tool_info()).to_json();

    assert!(cyclic_check.has_violations());
    assert!(cyclic_json.contains("\"status\":\"policy_violation\""));
    assert!(cyclic_json.contains("\"result\":\"fail\""));
    assert!(cyclic_json.contains("\"completeness\":{\"status\":\"partial\""));
}

#[test]
fn baseline_subjects_with_invalid_current_identity_are_excluded_not_stale() {
    let mut report = report_from_edges(&[]);
    report.modules.push(ModuleMetric {
        module: "app".to_string(),
        path: "app.veln".to_string(),
        generated: false,
        fan_in: 0,
        fan_out: 0,
        dependency_pressure: 0,
        external_dependency_count: 0,
        span: SourceFile::new("app.veln", "").span(TextRange::new(0, 0)),
    });
    mark_report_partial(&mut report, "Beta.veln");
    mark_report_partial(&mut report, "App.veln");
    let baseline = MetricsBaseline {
        modules: vec![
            BaselineModule {
                module: "Beta".to_string(),
                path: "Beta.veln".to_string(),
            },
            BaselineModule {
                module: "App".to_string(),
                path: "App.veln".to_string(),
            },
            BaselineModule {
                module: "deleted".to_string(),
                path: "deleted.veln".to_string(),
            },
        ],
        edges: Vec::new(),
        cycles: Vec::new(),
    };

    let check = evaluate_metrics_check_with_baseline(
        report,
        MetricsPolicy { deny_cycles: true },
        baseline,
        "metrics.baseline.json".to_string(),
    );

    assert_eq!(
        check.baseline.as_ref().unwrap().stale_subjects,
        vec!["deleted".to_string()]
    );
    assert_eq!(
        check.report.completeness.excluded_baseline_subjects,
        vec!["App".to_string(), "Beta".to_string()]
    );
}
