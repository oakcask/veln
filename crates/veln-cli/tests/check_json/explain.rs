use super::support::*;

#[test]
fn explain_reports_diagnostic_help() {
    let project = TestProject::new("cli-explain");

    let output = project.veln(&["explain"], &["hole.unfilled"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("hole.unfilled: unfilled typed hole"));
    assert!(stdout(&output).contains("Meaning:"));
    assert!(stdout(&output).contains("Repair:"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_lists_known_diagnostics() {
    let project = TestProject::new("cli-explain-list");

    let output = project.veln(&["explain"], &["--list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("hole.unfilled - unfilled typed hole"));
    assert!(
        stdout(&output)
            .contains("parse.contract_predicate - unsupported contract predicate syntax")
    );
    assert!(stdout(&output).contains("parse.satisfy_candidate - missing satisfy candidate"));
    assert!(stdout(&output).contains("parse.satisfy_arrow - missing satisfy arrow"));
    assert!(stdout(&output).contains("hole.satisfy_candidate_unused - unused satisfy candidate"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_list_takes_precedence_over_diagnostic_id() {
    let project = TestProject::new("cli-explain-list-with-id");

    let output = project.veln(&["explain"], &["--list", "hole.unfilled"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout.contains("hole.unfilled - unfilled typed hole"));
    assert!(!stdout.contains("Meaning:"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_reports_missing_and_unknown_diagnostic_ids() {
    let project = TestProject::new("cli-explain-errors");

    let missing = project.veln(&["explain"], &[]);
    let unknown = project.veln(&["explain"], &["unknown.id"]);

    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(
        stderr(&missing),
        "veln: explain requires a diagnostic id or --list\n"
    );
    assert_eq!(stdout(&missing), "");
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(
        stderr(&unknown),
        "veln: no explanation for diagnostic `unknown.id`\n"
    );
    assert_eq!(stdout(&unknown), "");
}

#[test]
fn cli_prints_version() {
    let project = TestProject::new("cli-version");

    let output = project.veln(&[], &["--version"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "veln 0.1.0\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn cli_reports_parser_errors_before_project_discovery() {
    let project = TestProject::new("cli-parser-errors");

    let cases: &[(&[&str], &[&str], &str)] = &[
        (&[], &["wat"], "wat"),
        (&["doc"], &["--wat"], "--wat"),
        (&["repair"], &["--wat"], "--wat"),
        (&["check"], &["--wat"], "--wat"),
        (&["run"], &["--wat"], "--wat"),
        (&["test"], &["--wat"], "--wat"),
        (&["explain"], &["--wat"], "--wat"),
        (&["explain"], &["hole.unfilled", "extra"], "extra"),
        (&["run"], &[], "<ENTRY>"),
    ];

    for &(command_args, args, expected_detail) in cases {
        let output = project.veln(command_args, args);
        let error = stderr(&output);

        assert_eq!(output.status.code(), Some(2), "{command_args:?} {args:?}");
        assert_eq!(stdout(&output), "", "{command_args:?} {args:?}");
        assert!(
            error.contains(expected_detail),
            "missing `{expected_detail}` in {error}"
        );
        assert!(error.contains("Usage:"), "missing `Usage:` in {error}");
    }
}
