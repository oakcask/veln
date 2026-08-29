use super::*;

#[test]
fn counts_internal_edges_external_imports_and_self_cycles() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new(
                "app.veln",
                "use util\nuse vendor from \"example/pkg\"\nfn main() -> ()\n  ()\nend\n",
            ),
            SourceFile::new(
                "util.veln",
                "use app\nuse util\nfn value() -> ()\n  ()\nend\n",
            ),
        ],
    };

    let graph = DependencyGraph::from_project(&project).expect("graph");
    let selected = ["app.veln".to_string(), "util.veln".to_string()]
        .into_iter()
        .collect();
    let report = graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: Vec::new(),
        },
        &selected,
        default_metrics_config(),
        Vec::new(),
        MetricsCompleteness::default(),
    );

    assert_eq!(report.summary.internal_edge_count, 3);
    assert_eq!(report.summary.external_dependency_count, 1);
    assert_eq!(report.summary.cycle_count, 1);
    assert_eq!(report.cycles[0].path.first().unwrap(), "app");
    assert_eq!(report.cycles[0].path.last().unwrap(), "app");
    assert_eq!(report.modules[0].module, "util");
    assert_eq!(report.modules[0].dependency_pressure, 4);
}

#[test]
fn counts_abc_constructs_from_function_bodies() {
    let cases = [
        (
            "let binding",
            "fn subject() -> Int\n  let value = 1\n  value\nend\n",
            AbcVector {
                assignments: 1,
                branches: 0,
                conditionals: 0,
            },
        ),
        (
            "call",
            "fn subject() -> Int\n  add(1, 2)\nend\n",
            AbcVector {
                assignments: 0,
                branches: 1,
                conditionals: 0,
            },
        ),
        (
            "perform",
            "fn subject() -> Int effects [Console]\n  perform Console::read()\nend\n",
            AbcVector {
                assignments: 0,
                branches: 1,
                conditionals: 0,
            },
        ),
        (
            "handler application",
            "fn subject() -> Int effects [Ask]\n  handle perform Ask::value() with ask(1)\nend\n\neffect Ask\n  value() -> Int\nend\n\nhandler ask(context: Int) handles Ask\n  value() => provide(context)\nend\n",
            AbcVector {
                assignments: 0,
                branches: 2,
                conditionals: 0,
            },
        ),
        (
            "schema decode and try",
            "fn subject() -> DecodeStep<{value: Int}>\n  decode Packet from view()? at offset()?\nend\n",
            AbcVector {
                assignments: 0,
                branches: 3,
                conditionals: 2,
            },
        ),
        (
            "schema encode",
            "fn subject() -> ByteChunk\n  encode Packet from {value: make_value()}\nend\n",
            AbcVector {
                assignments: 0,
                branches: 2,
                conditionals: 0,
            },
        ),
        (
            "if else-if and short circuit",
            "fn subject() -> Int\n  if left() and right()\n    1\n  else if fallback() or other()\n    2\n  else\n    3\n  end\nend\n",
            AbcVector {
                assignments: 0,
                branches: 4,
                conditionals: 4,
            },
        ),
        (
            "match arms",
            "fn subject() -> Int\n  match value()\n    0 => one()\n    _ => two()\n  end\nend\n",
            AbcVector {
                assignments: 0,
                branches: 3,
                conditionals: 3,
            },
        ),
        (
            "nested expressions",
            "fn subject() -> Int\n  let value = outer(if ready()\n    inner(1)\n  else\n    inner(2)\n  end)\n  value?\nend\n",
            AbcVector {
                assignments: 1,
                branches: 4,
                conditionals: 2,
            },
        ),
    ];

    for (name, source, expected) in cases {
        assert_eq!(first_function_vector(source), expected, "{name}");
    }
}

#[test]
fn excludes_annotations_contracts_and_preserves_subject_kind_identity() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![SourceFile::new(
            "app.veln",
            "fn same(value: Int) -> Result<Int, String>\n  ensure value >= 0\n  value\nend\n\ntest same() -> Result<Int, String>\n  require perform Console::read() == 1\n  let value: Int = compute()?\n  value\nend\n",
        )],
    };
    let selected = ["app.veln".to_string()].into_iter().collect();
    let subjects = abc_subjects(&project, &selected);

    assert_eq!(subjects.len(), 2);
    assert_eq!(subjects[0].kind, AbcSubjectKind::Test);
    assert_eq!(
        subjects[0].vector,
        AbcVector {
            assignments: 1,
            branches: 1,
            conditionals: 1,
        }
    );
    assert!(!subjects[0].contracts_included);
    assert_eq!(subjects[1].kind, AbcSubjectKind::Function);
    assert_eq!(subjects[1].vector, AbcVector::default());
    assert_eq!(subjects[0].identity, "app.veln::same");
    assert_eq!(subjects[1].identity, "app.veln::same");
    assert_eq!(abc_contract_subject_count(&project, &selected), 2);
}

#[test]
fn excludes_generated_and_doctest_derived_abc_subjects() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new("app.veln", "fn source() -> Int\n  call()\nend\n"),
            SourceFile::new(
                "target/generated.veln",
                "fn generated() -> Int\n  call()\nend\n",
            ),
            SourceFile::new(
                "app.veln#doctest-1_test.veln",
                "test doctest_1() -> Int\n  call()\nend\n",
            ),
        ],
    };
    let selected = [
        "app.veln".to_string(),
        "target/generated.veln".to_string(),
        "app.veln#doctest-1_test.veln".to_string(),
    ]
    .into_iter()
    .collect();
    let subjects = abc_subjects(&project, &selected);

    assert_eq!(subjects.len(), 1);
    assert_eq!(subjects[0].identity, "app.veln::source");
}

#[test]
fn render_human_reports_abc_summary_counts() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![SourceFile::new(
            "app.veln",
            "fn plain() -> Int\n  call()\nend\n\nfn guarded(value: Int) -> Int\n  ensure value >= 0\n  value\nend\n",
        )],
    };
    let selected = ["app.veln".to_string()].into_iter().collect();
    let graph = DependencyGraph::from_project(&project).expect("graph");
    let report = graph.report(
        &project,
        ProjectIdentity {
            root: ".".to_string(),
            selected_paths: vec!["app.veln".to_string()],
        },
        &selected,
        default_metrics_config(),
        Vec::new(),
        MetricsCompleteness::default(),
    );
    let human = render_human(&report);

    assert!(human.contains("ABC subjects: 2, ABC contract subjects: 1"));
}

#[test]
fn render_human_keeps_empty_section_messages_in_report_order() {
    let human = render_human(&report_from_edges(&[]));

    assert!(human.contains("Cycles\n  none\n"));
    assert!(human.contains("Modules\n  no project modules selected\n"));
    assert!(human.contains("ABC size\n  no function or test subjects selected\n"));
    assert!(human.contains("Whole-body similarity (experimental)\n"));
    assert!(human.ends_with("  none\n"));
    assert_before(&human, "Cycles\n", "\nModules\n");
    assert_before(&human, "\nModules\n", "\nABC size\n");
    assert_before(
        &human,
        "\nABC size\n",
        "\nWhole-body similarity (experimental)\n",
    );
}
