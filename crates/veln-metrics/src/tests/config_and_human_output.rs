use super::*;

#[test]
fn reads_metrics_cycle_policy_from_manifest_table() {
    let cases = [
        ("omitted", None, Ok(false)),
        ("false", Some("false"), Ok(false)),
        ("true", Some("true"), Ok(true)),
        ("invalid", Some("yes"), Err("metrics.policy.invalid_value")),
    ];

    for (name, value, expected) in cases {
        let manifest = value.map(|value| metrics_manifest(&[("deny_cycles", value)]));
        let actual = read_metrics_policy(manifest.as_ref());
        match expected {
            Ok(deny_cycles) => {
                assert_eq!(actual.expect(name).deny_cycles, deny_cycles, "{name}");
            }
            Err(id) => {
                let diagnostics = actual.expect_err(name);
                assert_eq!(diagnostics[0].id, id, "{name}");
                assert_eq!(
                    diagnostics[0].span.as_ref().unwrap().file.as_str(),
                    "veln.toml"
                );
            }
        }
    }
}

#[test]
fn reads_metrics_human_output_limit_from_manifest_table() {
    let cases = [
        ("omitted", None, Ok(DEFAULT_HUMAN_OUTPUT_MAX_FINDINGS)),
        ("valid", Some("5"), Ok(5)),
        ("zero", Some("0"), Err("metrics.policy.invalid_value")),
        (
            "malformed",
            Some("many"),
            Err("metrics.policy.invalid_value"),
        ),
        (
            "overflow",
            Some("184467440737095516160"),
            Err("metrics.policy.invalid_value"),
        ),
    ];

    for (name, value, expected) in cases {
        let manifest = value.map(|value| metrics_manifest(&[("max_findings", value)]));
        let actual = read_metrics_config(manifest.as_ref());
        match expected {
            Ok(max_findings) => {
                assert_eq!(
                    actual.expect(name).human_output_max_findings,
                    max_findings,
                    "{name}"
                );
            }
            Err(id) => {
                let diagnostics = actual.expect_err(name);
                assert_eq!(diagnostics[0].id, id, "{name}");
                assert_eq!(
                    diagnostics[0].span.as_ref().unwrap().file.as_str(),
                    "veln.toml"
                );
            }
        }
    }

    if usize::MAX > max_json_usize() {
        let manifest = metrics_manifest(&[("max_findings", "9223372036854775808")]);
        let diagnostics =
            read_metrics_config(Some(&manifest)).expect_err("above JSON number maximum");
        assert_eq!(diagnostics[0].id, "metrics.policy.invalid_value");
        assert_eq!(
            diagnostics[0].span.as_ref().unwrap().file.as_str(),
            "veln.toml"
        );
    }
}

#[test]
fn rejects_unknown_metrics_policy_fields() {
    let manifest = metrics_manifest(&[("deny_cycles", "true"), ("unknown", "5")]);
    let diagnostics = read_metrics_policy(Some(&manifest)).expect_err("unsupported field");

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "metrics.policy.unsupported_field");
    assert_eq!(
        diagnostics[0].span.as_ref().unwrap().file.as_str(),
        "veln.toml"
    );
}

#[test]
fn reports_all_invalid_metrics_config_fields_in_manifest_order() {
    let manifest = metrics_manifest(&[
        ("deny_cycles", "yes"),
        ("similarity_min_tokens", "0"),
        ("max_findings", "many"),
        ("unknown", "5"),
    ]);

    let diagnostics = read_metrics_config(Some(&manifest)).expect_err("invalid fields");

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "metrics.policy.invalid_value",
            "metrics.policy.invalid_value",
            "metrics.policy.invalid_value",
            "metrics.policy.unsupported_field",
        ]
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .is_some_and(|span| span.file.as_str() == "veln.toml")
    }));
    assert!(diagnostics[0].message.contains("`deny_cycles`"));
    assert!(diagnostics[1].message.contains("`similarity_min_tokens`"));
    assert!(diagnostics[2].message.contains("`max_findings`"));
    assert!(diagnostics[3].message.contains("`unknown`"));
}

#[test]
fn render_human_applies_limit_to_each_section() {
    let mut report = report_from_edges(&[
        ("app", "util"),
        ("util", "app"),
        ("zeta", "app"),
        ("alpha", "app"),
    ]);
    report.human_output_max_findings = 3;

    let human = render_human(&report);

    assert!(human.contains("app (app.veln) fan-in=3 fan-out=1 pressure=3 external=0"));
    assert!(human.contains("util (util.veln) fan-in=1 fan-out=1 pressure=1 external=0"));
    assert!(human.contains("alpha (alpha.veln) fan-in=0 fan-out=1 pressure=0 external=0"));
    assert!(!human.contains("zeta (zeta.veln) fan-in=0 fan-out=1 pressure=0 external=0"));
    assert!(human.contains("app, util | path: app -> util -> app"));
    assert!(human.contains("showing 3 of 4 module rows; 1 omitted"));
    assert!(human.contains("showing 3 of 4 ABC subjects; 1 omitted"));
    assert!(human.contains("app.veln::app_value"));
    assert_before(&human, "Cycles\n", "\nModules\n");
    assert!(
        human.contains(
            "Detailed findings omitted: 2; use veln metrics --json for complete evidence."
        )
    );
}

#[test]
fn render_human_keeps_similarity_related_lines_at_truncation_boundary() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![SourceFile::new(
            "app.veln",
            "fn first() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn second() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n\nfn third() -> Int\n  let value = add(4, 5)\n  value\nend\n\nfn fourth() -> Int\n  let value = add(4, 5)\n  value\nend\n",
        )],
    };
    let selected = ["app.veln".to_string()].into_iter().collect();
    let graph = DependencyGraph::from_project(&project).expect("graph");
    let mut report = graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: vec!["app.veln".to_string()],
        },
        &selected,
        MetricsConfig {
            similarity_min_tokens: 8,
            ..default_metrics_config()
        },
        Vec::new(),
        MetricsCompleteness::default(),
    );
    report.human_output_max_findings = 1;

    let human = render_human(&report);

    assert!(human.contains("primary=app.veln::first"));
    assert!(human.contains("related: app.veln::second"));
    assert!(!human.contains("primary=app.veln::third"));
    assert!(!human.contains("related: app.veln::fourth"));
    assert!(human.contains("showing 1 of 2 similarity instances; 1 omitted"));
    assert!(
        human.contains(
            "Detailed findings omitted: 4; use veln metrics --json for complete evidence."
        )
    );
}

#[test]
fn render_check_human_applies_limit_to_each_section() {
    let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
    report.human_output_max_findings = 1;
    let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

    let human = render_check_human(&check);

    assert!(human.contains("policy result: fail"));
    assert!(human.contains("deny_cycles: dependency cycle path: app -> util -> app"));
    assert!(human.contains("app (app.veln) fan-in=1 fan-out=1 pressure=1 external=0"));
    assert!(human.contains("app.veln::app_value"));
    assert!(human.contains("showing 1 of 2 module rows; 1 omitted"));
    assert!(human.contains("showing 1 of 2 ABC subjects; 1 omitted"));
    assert!(
        human.contains(
            "Detailed findings omitted: 2; use veln metrics --json for complete evidence."
        )
    );
}

#[test]
fn report_json_exposes_human_output_projection_metadata() {
    let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
    report.human_output_max_findings = 1;

    let json = report_to_json(&report, tool_info()).to_json();

    assert!(json.contains("\"human_output\":{\"max_findings\":1"));
    assert!(json.contains("\"total_findings\":5"));
    assert!(json.contains("\"omitted_findings\":2"));
    assert!(json.contains("\"truncated\":true"));
    assert!(json.contains("\"modules\":["));
    assert!(json.contains("\"cycles\":["));
    assert!(json.contains("\"abc_subjects\":["));
}

#[test]
fn checked_json_counts_policy_violations_in_human_output_projection() {
    let mut report = report_from_edges(&[("app", "util"), ("util", "app")]);
    report.human_output_max_findings = 1;
    let check = evaluate_metrics_check(report, MetricsPolicy { deny_cycles: true });

    let json = report_check_to_json(&check, tool_info()).to_json();

    assert!(json.contains("\"human_output\":{\"max_findings\":1"));
    assert!(json.contains("\"total_findings\":6"));
    assert!(json.contains("\"omitted_findings\":2"));
    assert!(json.contains("\"status\":\"policy_violation\""));
    assert!(json.contains("\"violations\":["));
}
