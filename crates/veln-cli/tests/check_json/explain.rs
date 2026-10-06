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

    let unknown_command = project.veln(&[], &["wat"]);
    let unknown_doc_flag = project.veln(&["doc"], &["--wat"]);
    let unknown_repair_flag = project.veln(&["repair"], &["--wat"]);
    let unknown_check_flag = project.veln(&["check"], &["--wat"]);
    let unknown_run_flag = project.veln(&["run"], &["--wat"]);
    let unknown_test_flag = project.veln(&["test"], &["--wat"]);
    let unknown_explain_flag = project.veln(&["explain"], &["--wat"]);
    let unexpected_explain_argument = project.veln(&["explain"], &["hole.unfilled", "extra"]);
    let missing_run_entry = project.veln(&["run"], &[]);

    assert_eq!(unknown_command.status.code(), Some(2));
    assert_eq!(stdout(&unknown_command), "");
    assert!(stderr(&unknown_command).contains("wat"));
    assert!(stderr(&unknown_command).contains("Usage:"));

    assert_eq!(unknown_doc_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_doc_flag), "");
    assert!(stderr(&unknown_doc_flag).contains("--wat"));
    assert!(stderr(&unknown_doc_flag).contains("Usage:"));

    assert_eq!(unknown_repair_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_repair_flag), "");
    assert!(stderr(&unknown_repair_flag).contains("--wat"));
    assert!(stderr(&unknown_repair_flag).contains("Usage:"));

    assert_eq!(unknown_check_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_check_flag), "");
    assert!(stderr(&unknown_check_flag).contains("--wat"));
    assert!(stderr(&unknown_check_flag).contains("Usage:"));

    assert_eq!(unknown_run_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_run_flag), "");
    assert!(stderr(&unknown_run_flag).contains("--wat"));
    assert!(stderr(&unknown_run_flag).contains("Usage:"));

    assert_eq!(unknown_test_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_test_flag), "");
    assert!(stderr(&unknown_test_flag).contains("--wat"));
    assert!(stderr(&unknown_test_flag).contains("Usage:"));

    assert_eq!(unknown_explain_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_explain_flag), "");
    assert!(stderr(&unknown_explain_flag).contains("--wat"));
    assert!(stderr(&unknown_explain_flag).contains("Usage:"));

    assert_eq!(unexpected_explain_argument.status.code(), Some(2));
    assert_eq!(stdout(&unexpected_explain_argument), "");
    assert!(stderr(&unexpected_explain_argument).contains("extra"));
    assert!(stderr(&unexpected_explain_argument).contains("Usage:"));

    assert_eq!(missing_run_entry.status.code(), Some(2));
    assert_eq!(stdout(&missing_run_entry), "");
    assert!(stderr(&missing_run_entry).contains("<ENTRY>"));
    assert!(stderr(&missing_run_entry).contains("Usage:"));
}
