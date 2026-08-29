use super::*;

#[test]
fn evaluates_cycle_policy_table_and_preserves_report_data() {
    let cases = [
        (
            "acyclic pass",
            "fn value() -> ()\n  ()\nend\n",
            0,
            false,
            "\"status\":\"ok\"",
        ),
        (
            "cycle violation",
            "use app\nfn value() -> ()\n  ()\nend\n",
            1,
            true,
            "\"status\":\"policy_violation\"",
        ),
    ];

    for (name, util_source, expected_cycles, expected_violation, expected_status) in cases {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![
                SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                SourceFile::new("util.veln", util_source),
            ],
        };
        let selected = ["app.veln".to_string(), "util.veln".to_string()]
            .into_iter()
            .collect();
        let graph = DependencyGraph::from_project(&project).expect(name);
        let report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: vec!["app.veln".to_string(), "util.veln".to_string()],
            },
            &selected,
            default_metrics_config(),
            Vec::new(),
            MetricsCompleteness::default(),
        );

        let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

        assert_eq!(check.has_violations(), expected_violation, "{name}");
        assert_eq!(check.report.summary.cycle_count, expected_cycles, "{name}");
        assert_eq!(check.report.modules.len(), 2, "{name}");
        assert!(
            report_check_to_json(&check, ToolInfo::new("veln", "0.1.0"))
                .to_json()
                .contains(expected_status),
            "{name}"
        );

        if expected_violation {
            assert_eq!(check.violations[0].policy, "deny_cycles", "{name}");
            assert_eq!(
                check.violations[0].path.first().map(String::as_str),
                Some("app"),
                "{name}"
            );
            assert_eq!(
                check.violations[0].path.last().map(String::as_str),
                Some("app"),
                "{name}"
            );
            assert!(
                render_check_human(&check).contains("review module ownership"),
                "{name}"
            );
        } else {
            assert!(
                render_check_human(&check).contains("policy result: pass"),
                "{name}"
            );
        }
    }
}

#[test]
fn evaluates_cycle_policy_only_for_selected_cycle_subjects() {
    let cases = [
        (
            "unselected cycle is advisory only",
            vec![
                SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                SourceFile::new("util.veln", "use app\nfn value() -> ()\n  ()\nend\n"),
                SourceFile::new("entry.veln", "fn entry() -> ()\n  ()\nend\n"),
            ],
            vec!["entry.veln"],
            0,
            false,
        ),
        (
            "selected mutual cycle fails",
            vec![
                SourceFile::new("app.veln", "use util\nfn main() -> ()\n  ()\nend\n"),
                SourceFile::new("util.veln", "use app\nfn value() -> ()\n  ()\nend\n"),
                SourceFile::new("entry.veln", "fn entry() -> ()\n  ()\nend\n"),
            ],
            vec!["app.veln"],
            1,
            true,
        ),
        (
            "selected self cycle fails",
            vec![SourceFile::new(
                "self.veln",
                "use self\nfn value() -> ()\n  ()\nend\n",
            )],
            vec!["self.veln"],
            1,
            true,
        ),
    ];

    for (name, files, selected, expected_cycles, expected_violation) in cases {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files,
        };
        let selected = selected
            .into_iter()
            .map(str::to_string)
            .collect::<BTreeSet<_>>();
        let graph = DependencyGraph::from_project(&project).expect(name);
        let report = graph.report(
            &project,
            ProjectIdentity {
                root: ".".to_string(),
                selected_paths: selected.iter().cloned().collect(),
            },
            &selected,
            default_metrics_config(),
            Vec::new(),
            MetricsCompleteness::default(),
        );

        let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

        assert_eq!(check.report.summary.cycle_count, expected_cycles, "{name}");
        assert_eq!(check.has_violations(), expected_violation, "{name}");
    }
}

#[test]
fn evaluates_cycle_policy_against_baseline_regressions() {
    let cases = [
        (
            "unchanged cycle is allowed",
            vec![("app", "util"), ("util", "app")],
            vec![vec!["app", "util"]],
            vec![("app", "util"), ("util", "app")],
            false,
        ),
        (
            "cycle that loses an edge is allowed",
            vec![
                ("app", "util"),
                ("util", "app"),
                ("util", "core"),
                ("core", "app"),
            ],
            vec![vec!["app", "core", "util"]],
            vec![("app", "util"), ("util", "app")],
            false,
        ),
        (
            "cycle with same members that loses only a cyclic edge is allowed",
            vec![
                ("app", "util"),
                ("app", "core"),
                ("util", "core"),
                ("core", "app"),
            ],
            vec![vec!["app", "core", "util"]],
            vec![("app", "util"), ("util", "core"), ("core", "app")],
            false,
        ),
        (
            "cycle that adds an edge fails",
            vec![("app", "util"), ("util", "app")],
            vec![vec!["app", "util"]],
            vec![("app", "util"), ("util", "core"), ("core", "app")],
            true,
        ),
        (
            "cycle with same members that adds only a cyclic edge fails",
            vec![("app", "util"), ("util", "core"), ("core", "app")],
            vec![vec!["app", "core", "util"]],
            vec![
                ("app", "util"),
                ("util", "core"),
                ("core", "app"),
                ("core", "util"),
            ],
            true,
        ),
        (
            "renamed cycle fails",
            vec![("app", "util"), ("util", "app")],
            vec![vec!["app", "util"]],
            vec![("renamed", "util"), ("util", "renamed")],
            true,
        ),
        (
            "new self cycle fails",
            vec![("app", "util"), ("util", "app")],
            vec![vec!["app", "util"]],
            vec![("selfish", "selfish")],
            true,
        ),
    ];

    for (name, baseline_edges, baseline_cycles, current_edges, expected_violation) in cases {
        let report = report_from_edges(&current_edges);
        let baseline = baseline_from_edges(&baseline_edges, &baseline_cycles);
        let check = evaluate_metrics_check_with_baseline(
            report,
            MetricsPolicy { deny_cycles: true },
            baseline,
            "metrics.baseline.json".to_string(),
        );

        assert_eq!(check.has_violations(), expected_violation, "{name}");
    }
}

#[test]
fn reports_stale_baseline_subjects_without_failing() {
    let report = report_from_edges(&[("app", "util"), ("util", "app")]);
    let baseline = MetricsBaseline {
        modules: vec![
            BaselineModule {
                module: "app".to_string(),
                path: "app.veln".to_string(),
            },
            BaselineModule {
                module: "util".to_string(),
                path: "util.veln".to_string(),
            },
            BaselineModule {
                module: "deleted".to_string(),
                path: "deleted.veln".to_string(),
            },
        ],
        edges: vec![
            BaselineEdge {
                source: "app".to_string(),
                target: "util".to_string(),
            },
            BaselineEdge {
                source: "util".to_string(),
                target: "app".to_string(),
            },
        ],
        cycles: vec![BaselineCycle {
            members: vec!["app".to_string(), "util".to_string()],
        }],
    };

    let check = evaluate_metrics_check_with_baseline(
        report,
        MetricsPolicy { deny_cycles: true },
        baseline,
        "metrics.baseline.json".to_string(),
    );

    assert!(!check.has_violations());
    assert_eq!(
        check.baseline.unwrap().stale_subjects,
        vec!["deleted".to_string()]
    );
}

#[test]
fn parses_baseline_schema_and_metric_model_versions() {
    let report = report_from_edges(&[("app", "util"), ("util", "app")]);
    let json = baseline_to_json(&report, ToolInfo::new("veln", "0.1.0")).to_json();
    let baseline = baseline_from_json(&json).expect("baseline should parse");

    assert_eq!(baseline.modules.len(), 2);
    assert_eq!(baseline.cycles.len(), 1);

    let unsupported_schema = json.replace(BASELINE_SCHEMA_VERSION, "veln-metrics-baseline/v999");
    assert_eq!(
        baseline_from_json(&unsupported_schema).unwrap_err()[0].id,
        "metrics.baseline.unsupported_schema"
    );

    let unsupported_model = json.replace(METRIC_MODEL_VERSION, "veln-metrics-model/v999");
    assert_eq!(
        baseline_from_json(&unsupported_model).unwrap_err()[0].id,
        "metrics.baseline.unsupported_metric_model"
    );
}
