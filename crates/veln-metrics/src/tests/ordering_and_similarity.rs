use super::*;

#[test]
fn canonical_path_ordering_survives_source_insertion_order_and_separators() {
    let forward = stable_ordering_project(vec![
        SourceFile::new("nested\\util.veln", STABLE_UTIL_SOURCE),
        SourceFile::new("nested/app.veln", STABLE_APP_SOURCE),
    ]);
    let reversed = stable_ordering_project(vec![
        SourceFile::new("nested\\app.veln", STABLE_APP_SOURCE),
        SourceFile::new("nested/util.veln", STABLE_UTIL_SOURCE),
    ]);
    let selected = [
        "nested/app.veln".to_string(),
        "nested/util.veln".to_string(),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    let forward_report = metrics_report_from_project(&forward, &selected);
    let reversed_report = metrics_report_from_project(&reversed, &selected);
    let forward_json = report_to_json(&forward_report, tool_info()).to_json();
    let reversed_json = report_to_json(&reversed_report, tool_info()).to_json();
    let forward_human = render_human(&forward_report);
    let reversed_human = render_human(&reversed_report);
    let baseline_json = baseline_to_json(&forward_report, tool_info()).to_json();

    assert_eq!(forward_json, reversed_json);
    assert_eq!(forward_human, reversed_human);
    assert!(!forward_json.contains('\\'));
    assert!(!forward_human.contains('\\'));
    assert!(!baseline_json.contains('\\'));
    assert_eq!(
        forward_report.project.selected_paths,
        ["nested/app.veln", "nested/util.veln"]
    );
    assert_eq!(
        forward_report
            .modules
            .iter()
            .map(|module| module.module.as_str())
            .collect::<Vec<_>>(),
        ["nested::app", "nested::util"]
    );
    assert_eq!(
        forward_report
            .edges
            .iter()
            .map(|edge| (edge.source.as_str(), edge.target.as_str()))
            .collect::<Vec<_>>(),
        [
            ("nested::app", "nested::util"),
            ("nested::util", "nested::app")
        ]
    );
    assert_eq!(
        forward_report
            .abc_subjects
            .iter()
            .map(|subject| subject.identity.as_str())
            .collect::<Vec<_>>(),
        [
            "nested/app.veln::duplicate_app",
            "nested/app.veln::variant_app",
            "nested/util.veln::duplicate_util",
            "nested/util.veln::variant_util",
            "nested/app.veln::add",
            "nested/util.veln::add"
        ]
    );
    assert_eq!(forward_report.similarities.len(), 2);
    assert_eq!(
        forward_report.similarities[0]
            .declarations
            .iter()
            .map(|declaration| declaration.identity.as_str())
            .collect::<Vec<_>>(),
        [
            "nested/app.veln::duplicate_app",
            "nested/util.veln::duplicate_util"
        ]
    );
    assert_eq!(
        forward_report.similarities[1]
            .declarations
            .iter()
            .map(|declaration| declaration.identity.as_str())
            .collect::<Vec<_>>(),
        [
            "nested/app.veln::variant_app",
            "nested/util.veln::variant_util"
        ]
    );
    assert_eq!(
        forward_report
            .similarities
            .iter()
            .map(|instance| instance.token_count)
            .collect::<Vec<_>>(),
        [19, 19]
    );
}

#[test]
fn orders_abc_subjects_deterministically() {
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![
            SourceFile::new("b.veln", "fn small() -> Int\n  call()\nend\n"),
            SourceFile::new(
                "a.veln",
                "fn large() -> Int\n  let value = call()\n  if value == 1\n    call()\n  else\n    call()\n  end\nend\n",
            ),
        ],
    };
    let selected = ["a.veln".to_string(), "b.veln".to_string()]
        .into_iter()
        .collect();
    let subjects = abc_subjects(&project, &selected);

    assert_eq!(
        subjects
            .iter()
            .map(|subject| subject.identity.as_str())
            .collect::<Vec<_>>(),
        ["a.veln::large", "b.veln::small"]
    );
}

#[test]
fn detects_similarity_from_table_driven_source_boundaries() {
    struct Case {
        name: &'static str,
        source: &'static str,
        min_tokens: usize,
        fingerprint_count: usize,
        groups: Vec<Vec<&'static str>>,
        token_counts: Vec<usize>,
    }

    let cases = [
        Case {
            name: "formatting and comments are ignored",
            source: "fn first() -> Int\n  # ignored\n  let value = add(1, 2)\n  value\nend\n\nfn second() -> Int\n  let value=add(\n    1,\n    2\n  )\n  value\nend\n",
            min_tokens: 8,
            fingerprint_count: 2,
            groups: vec![vec!["app.veln::first", "app.veln::second"]],
            token_counts: vec![10],
        },
        Case {
            name: "identifier spelling is significant",
            source: "fn first() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn renamed() -> Int\n  let other = add(1, 2)\n  other\nend\n",
            min_tokens: 8,
            fingerprint_count: 2,
            groups: vec![],
            token_counts: vec![],
        },
        Case {
            name: "literal spelling is significant",
            source: "fn one() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn two() -> Int\n  let value = add(1, 3)\n  value\nend\n",
            min_tokens: 8,
            fingerprint_count: 2,
            groups: vec![],
            token_counts: vec![],
        },
        Case {
            name: "only complete bodies match",
            source: "fn left() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn right() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn partial() -> Int\n  let value = add(1, 2)\n  value + 1\nend\n",
            min_tokens: 10,
            fingerprint_count: 3,
            groups: vec![vec!["app.veln::left", "app.veln::right"]],
            token_counts: vec![10],
        },
        Case {
            name: "minimum token boundary excludes short bodies",
            source: "fn left() -> Int\n  let value = add(1, 2)\n  value\nend\n\nfn right() -> Int\n  let value = add(1, 2)\n  value\nend\n",
            min_tokens: 11,
            fingerprint_count: 0,
            groups: vec![],
            token_counts: vec![],
        },
    ];

    for case in cases {
        let project = Project {
            root: ".".into(),
            manifest: None,
            files: vec![SourceFile::new("app.veln", case.source)],
        };
        let selected = ["app.veln".to_string()].into_iter().collect();
        let (instances, fingerprint_count) =
            similarity_instances(&project, &selected, case.min_tokens);
        let groups = instances
            .iter()
            .map(|instance| {
                instance
                    .declarations
                    .iter()
                    .map(|declaration| declaration.identity.as_str())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let token_counts = instances
            .iter()
            .map(|instance| instance.token_count)
            .collect::<Vec<_>>();

        assert_eq!(fingerprint_count, case.fingerprint_count, "{}", case.name);
        assert_eq!(groups, case.groups, "{}", case.name);
        assert_eq!(token_counts, case.token_counts, "{}", case.name);
    }
}

#[test]
fn groups_similarity_candidates_by_complete_tokens_after_fingerprint_matches() {
    let candidates = vec![
        similarity_candidate(
            "a.veln",
            "first",
            "same",
            vec![("Let", "let"), ("Ident", "a")],
        ),
        similarity_candidate(
            "b.veln",
            "second",
            "same",
            vec![("Let", "let"), ("Ident", "b")],
        ),
        similarity_candidate(
            "c.veln",
            "third",
            "same",
            vec![("Let", "let"), ("Ident", "a")],
        ),
        similarity_candidate(
            "d.veln",
            "fourth",
            "same",
            vec![("Let", "let"), ("Ident", "b")],
        ),
    ];
    let instances = similarity_instances_from_candidates(candidates);

    assert_eq!(instances.len(), 2);
    assert_eq!(
        instances
            .iter()
            .map(|instance| {
                instance
                    .declarations
                    .iter()
                    .map(|declaration| declaration.identity.as_str())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
        [
            vec!["a.veln::first", "c.veln::third"],
            vec!["b.veln::second", "d.veln::fourth"]
        ]
    );
}

#[test]
fn orders_similarity_instances_and_respects_structural_bounds() {
    let candidates = vec![
        similarity_candidate(
            "ignored.veln",
            "unique_a",
            "unique-a",
            token_texts(&["u", "a"]),
        ),
        similarity_candidate(
            "ignored.veln",
            "unique_b",
            "unique-b",
            token_texts(&["u", "b"]),
        ),
        similarity_candidate(
            "z.veln",
            "large_z",
            "large",
            token_texts(&["l", "a", "r", "g", "e"]),
        ),
        similarity_candidate(
            "a.veln",
            "large_a",
            "large",
            token_texts(&["l", "a", "r", "g", "e"]),
        ),
        similarity_candidate(
            "m.veln",
            "large_m",
            "large",
            token_texts(&["l", "a", "r", "g", "e"]),
        ),
        similarity_candidate(
            "b.veln",
            "large_b",
            "large",
            token_texts(&["l", "a", "r", "g", "e"]),
        ),
        similarity_candidate(
            "dir\\z.veln",
            "pair_z",
            "pair-one",
            token_texts(&["p", "a", "i", "r"]),
        ),
        similarity_candidate(
            "dir/a.veln",
            "pair_a",
            "pair-one",
            token_texts(&["p", "a", "i", "r"]),
        ),
        similarity_candidate(
            "c.veln",
            "pair_c",
            "pair-two",
            token_texts(&["t", "w", "o", "2"]),
        ),
        similarity_candidate(
            "d.veln",
            "pair_d",
            "pair-two",
            token_texts(&["t", "w", "o", "2"]),
        ),
        similarity_candidate("e.veln", "pair_e", "pair-three", token_texts(&["o", "k"])),
        similarity_candidate("f.veln", "pair_f", "pair-three", token_texts(&["o", "k"])),
    ];
    let fingerprint_count = candidates.len();
    let instances = similarity_instances_from_candidates(candidates);
    let region_count = instances
        .iter()
        .map(|instance| instance.declarations.len())
        .sum::<usize>();

    assert_eq!(instances.len(), 4);
    assert_eq!(region_count, 10);
    assert!(region_count <= fingerprint_count);
    assert!(instances.len() <= fingerprint_count / 2);
    assert_eq!(
        instances
            .iter()
            .map(|instance| {
                (
                    instance.token_count,
                    instance.declarations[0].identity.as_str(),
                    instance.declarations.len(),
                )
            })
            .collect::<Vec<_>>(),
        [
            (5, "a.veln::large_a", 4),
            (4, "c.veln::pair_c", 2),
            (4, "dir/a.veln::pair_a", 2),
            (2, "e.veln::pair_e", 2)
        ]
    );

    let mut report = report_from_edges(&[]);
    report.summary.similarity_fingerprint_count = fingerprint_count;
    report.summary.similarity_instance_count = instances.len();
    report.summary.similarity_region_count = region_count;
    report.similarities = instances;

    let json = report_to_json(&report, tool_info()).to_json();
    let human = render_human(&report);

    assert_before(
        &json,
        "\"identity\":\"c.veln::pair_c\"",
        "\"identity\":\"dir/a.veln::pair_a\"",
    );
    assert_before(
        &human,
        "primary=c.veln::pair_c",
        "primary=dir/a.veln::pair_a",
    );
}

#[test]
fn renders_similarity_fingerprint_tiebreak_order_in_public_outputs() {
    let instances = similarity_instances_from_candidates(vec![
        similarity_candidate(
            "same.veln",
            "primary",
            "fingerprint-b",
            token_texts(&["b", "b"]),
        ),
        similarity_candidate(
            "z.veln",
            "related_b",
            "fingerprint-b",
            token_texts(&["b", "b"]),
        ),
        similarity_candidate(
            "same.veln",
            "primary",
            "fingerprint-a",
            token_texts(&["a", "a"]),
        ),
        similarity_candidate(
            "z.veln",
            "related_a",
            "fingerprint-a",
            token_texts(&["a", "a"]),
        ),
    ]);
    let region_count = instances
        .iter()
        .map(|instance| instance.declarations.len())
        .sum::<usize>();
    let mut report = report_from_edges(&[]);
    report.summary.similarity_fingerprint_count = 4;
    report.summary.similarity_instance_count = instances.len();
    report.summary.similarity_region_count = region_count;
    report.similarities = instances;

    let json = report_to_json(&report, tool_info()).to_json();
    let human = render_human(&report);

    assert_before(
        &json,
        "\"fingerprint\":\"fingerprint-a\"",
        "\"fingerprint\":\"fingerprint-b\"",
    );
    assert_before(
        &human,
        "fingerprint=fingerprint-a",
        "fingerprint=fingerprint-b",
    );
}

#[test]
fn generated_similarity_workload_preserves_pipeline_bounds() {
    let workload = GeneratedSimilarityWorkload {
        unrelated_count: 8,
        large_group_count: 5,
        pair_count: 6,
        prefix_count: 7,
        min_tokens: 8,
    };
    let project = Project {
        root: ".".into(),
        manifest: None,
        files: vec![SourceFile::new("app.veln", workload.source())],
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
        MetricsConfig {
            similarity_min_tokens: workload.min_tokens,
            ..default_metrics_config()
        },
        Vec::new(),
        MetricsCompleteness::default(),
    );
    let eligible_declarations = workload.eligible_declaration_count();

    assert_eq!(
        report.summary.abc_subject_count, eligible_declarations,
        "the workload should exercise the full parsed function population"
    );
    assert_eq!(
        report.summary.similarity_fingerprint_count,
        eligible_declarations
    );
    assert_eq!(
        report.summary.similarity_instance_count,
        workload.expected_instance_count()
    );
    assert_eq!(
        report.summary.similarity_region_count,
        workload.expected_region_count()
    );
    assert!(report.summary.similarity_region_count <= eligible_declarations);
    assert!(report.summary.similarity_instance_count <= eligible_declarations / 2);

    let mut seen_declarations = BTreeSet::new();
    for instance in &report.similarities {
        for declaration in &instance.declarations {
            assert!(
                seen_declarations.insert(declaration.identity.clone()),
                "declaration reported in more than one similarity instance: {}",
                declaration.identity
            );
        }
    }
}
