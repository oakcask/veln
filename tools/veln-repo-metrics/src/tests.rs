use std::path::{Path, PathBuf};

use super::*;

#[test]
fn reports_function_over_threshold() {
    let source = r#"
fn complex(input: Result<i32, ()>) -> i32 {
    let mut total = 0;
    total += helper();
    if total > 1 && helper() > 2 {
        total = helper();
    }
    match input {
        Ok(value) => value,
        Err(_) => fallback(),
    }
}

fn helper() -> i32 {
    1
}

fn fallback() -> i32 {
    0
}
"#;

    let findings = analyze_source(Path::new("sample.rs"), source, 5.0).unwrap();

    assert_eq!(findings.len(), 1);
    let Finding::Function(finding) = &findings[0] else {
        panic!("expected function finding");
    };
    assert_eq!(finding.name, "complex");
    assert_eq!(
        finding.metrics,
        AbcMetrics {
            assignments: 3,
            branches: 4,
            conditionals: 5,
        }
    );
}

#[test]
fn escapes_json_strings() {
    let mut output = String::new();
    output::push_json_string(&mut output, "quote=\" slash=\\ line=\n");
    assert_eq!(output, "\"quote=\\\" slash=\\\\ line=\\n\"");
}

#[test]
fn reports_file_over_line_threshold() {
    let config = Config {
        dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
        dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
        dependency_graph: false,
        file_line_threshold: 3,
        format: OutputFormat::Human,
        max_findings: 10,
        roots: Vec::new(),
        threshold: 100.0,
    };
    let source = "fn tiny() {}\n\nfn other() {}\n";
    let path = Path::new("sample.rs");

    let mut findings = analyze_source(path, source, config.threshold).unwrap();
    let lines = source.lines().count();
    if lines >= config.file_line_threshold {
        findings.push(Finding::File(FileFinding {
            file: path.to_path_buf(),
            line: 1,
            lines,
        }));
    }

    assert!(matches!(findings.last(), Some(Finding::File(finding)) if finding.lines == 3));
}

#[test]
fn parses_dependency_graph_and_output_options() {
    let config = Config::parse([
        "--dependency-graph".to_string(),
        "--dependency-hotspots".to_string(),
        "3".to_string(),
        "--dependency-cycle-limit".to_string(),
        "2".to_string(),
        "--format".to_string(),
        "json".to_string(),
        "crates".to_string(),
    ])
    .unwrap();

    assert!(config.dependency_graph);
    assert_eq!(config.format, OutputFormat::Json);
    assert_eq!(config.dependency_hotspots, 3);
    assert_eq!(config.dependency_cycle_limit, 2);
    assert_eq!(config.roots, vec![PathBuf::from("crates")]);
}

#[test]
fn rejects_non_finite_thresholds_that_cannot_be_json_numbers() {
    for value in ["NaN", "inf", "-inf"] {
        let error = Config::parse([
            "--threshold".to_string(),
            value.to_string(),
            "crates".to_string(),
        ])
        .unwrap_err();
        assert_eq!(
            error,
            "--threshold must be a finite number greater than zero"
        );
    }
}

#[test]
fn renders_complete_machine_report_without_policy_fields() {
    let config = Config {
        dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
        dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
        dependency_graph: false,
        file_line_threshold: 700,
        format: OutputFormat::Json,
        max_findings: 1,
        roots: vec![PathBuf::from("crates")],
        threshold: 30.0,
    };
    let report = Report {
        files: vec![PathBuf::from("crates/sample/src/lib.rs")],
        findings: vec![
            Finding::Function(FunctionFinding {
                file: PathBuf::from("crates/sample/src/lib.rs"),
                line: 4,
                name: "sample".to_string(),
                metrics: AbcMetrics {
                    assignments: 3,
                    branches: 4,
                    conditionals: 0,
                },
            }),
            Finding::File(FileFinding {
                file: PathBuf::from("crates/sample/src/lib.rs"),
                line: 1,
                lines: 701,
            }),
        ],
        dependency_graph: None,
    };

    let json = output::render_json(&report, &config);
    assert!(json.contains("\"schema_version\":\"veln-repo-metrics-json/v0\""));
    assert!(json.contains("\"magnitude\":5.000000"));
    assert!(json.contains("\"kind\":\"file_line_count\""));
    assert!(json.contains("\"finding_count\":2"));
    assert!(!json.contains("severity"));
    assert!(!json.contains("blocks_merge"));
    assert!(!json.contains("GitHub"));
}

#[test]
fn renders_bounded_human_output_with_machine_report_route() {
    let config = Config {
        dependency_cycle_limit: DEFAULT_DEPENDENCY_CYCLE_LIMIT,
        dependency_hotspots: DEFAULT_DEPENDENCY_HOTSPOTS,
        dependency_graph: false,
        file_line_threshold: 700,
        format: OutputFormat::Human,
        max_findings: 1,
        roots: vec![PathBuf::from("crates")],
        threshold: 30.0,
    };
    let report = Report {
        files: vec![PathBuf::from("first.rs"), PathBuf::from("second.rs")],
        findings: vec![
            Finding::File(FileFinding {
                file: PathBuf::from("first.rs"),
                line: 1,
                lines: 701,
            }),
            Finding::File(FileFinding {
                file: PathBuf::from("second.rs"),
                line: 1,
                lines: 702,
            }),
        ],
        dependency_graph: None,
    };

    let human = output::render_human(&report, &config);
    assert!(human.starts_with("Rust repository metrics\n"));
    assert!(human.contains("first.rs:1: file has 701 lines"));
    assert!(!human.contains("second.rs:1: file has 702 lines"));
    assert!(human.contains("use --format json for the complete report"));
}
