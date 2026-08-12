use super::Command;
use std::path::PathBuf;

fn parse(args: &[&str]) -> Result<Command, String> {
    Command::parse(args.iter().map(|arg| arg.to_string()).collect())
}

#[test]
fn top_level_parser_handles_help_and_version_aliases() {
    assert!(matches!(parse(&[]).unwrap(), Command::Help { .. }));
    assert!(matches!(parse(&["help"]).unwrap(), Command::Help { .. }));
    assert!(matches!(parse(&["--help"]).unwrap(), Command::Help { .. }));
    assert!(matches!(parse(&["-h"]).unwrap(), Command::Help { .. }));
    assert!(matches!(parse(&["version"]).unwrap(), Command::Version));
    assert!(matches!(parse(&["--version"]).unwrap(), Command::Version));
    assert!(matches!(parse(&["-V"]).unwrap(), Command::Version));
}

#[test]
fn top_level_parser_reports_unknown_commands() {
    let error = match parse(&["build"]) {
        Ok(_) => panic!("unknown command should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown command `build`");
}

#[test]
fn help_parser_reports_unknown_topics() {
    let error = match parse(&["help", "build"]) {
        Ok(_) => panic!("unknown help topic should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown command `build`");
}

#[test]
fn help_parser_accepts_package_lock_topic() {
    assert!(matches!(
        parse(&["help", "package", "lock"]).unwrap(),
        Command::Help { .. }
    ));
}

#[test]
fn check_parser_accepts_json_and_input_paths() {
    let command = parse(&["check", "--json", "src/main.veln", "tests/case.veln"])
        .expect("check command should parse");

    let Command::Check { json, inputs } = command else {
        panic!("expected check command");
    };

    assert!(json);
    assert_eq!(
        inputs,
        [
            PathBuf::from("src/main.veln"),
            PathBuf::from("tests/case.veln")
        ]
    );
}

#[test]
fn check_parser_reports_unknown_flags() {
    let error = match parse(&["check", "--strict"]) {
        Ok(_) => panic!("unknown check flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown check flag `--strict`");
}

#[test]
fn doc_parser_accepts_input_paths() {
    let command = parse(&["doc", "src/main.veln", "docs"]).expect("doc command should parse");

    let Command::Doc { inputs } = command else {
        panic!("expected doc command");
    };

    assert_eq!(
        inputs,
        [PathBuf::from("src/main.veln"), PathBuf::from("docs")]
    );
}

#[test]
fn doc_parser_reports_unknown_flags() {
    let error = match parse(&["doc", "--json"]) {
        Ok(_) => panic!("unknown doc flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown doc flag `--json`");
}

#[test]
fn metrics_parser_accepts_json_check_and_input_paths() {
    let command =
        parse(&["metrics", "--check", "--json", "src"]).expect("metrics command should parse");

    let Command::Metrics {
        json,
        check,
        baseline,
        write_baseline,
        inputs,
    } = command
    else {
        panic!("expected metrics command");
    };

    assert!(json);
    assert!(check);
    assert_eq!(baseline, None);
    assert_eq!(write_baseline, None);
    assert_eq!(inputs, [PathBuf::from("src")]);
}

#[test]
fn metrics_parser_accepts_baseline_modes() {
    let check = parse(&["metrics", "--check", "--baseline", "metrics.json", "src"])
        .expect("baseline check command should parse");
    let Command::Metrics {
        json,
        check,
        baseline,
        write_baseline,
        inputs,
    } = check
    else {
        panic!("expected metrics command");
    };
    assert!(!json);
    assert!(check);
    assert_eq!(baseline, Some(PathBuf::from("metrics.json")));
    assert_eq!(write_baseline, None);
    assert_eq!(inputs, [PathBuf::from("src")]);

    let write = parse(&["metrics", "--write-baseline", "metrics.json", "src"])
        .expect("write-baseline command should parse");
    let Command::Metrics {
        json,
        check,
        baseline,
        write_baseline,
        inputs,
    } = write
    else {
        panic!("expected metrics command");
    };
    assert!(!json);
    assert!(!check);
    assert_eq!(baseline, None);
    assert_eq!(write_baseline, Some(PathBuf::from("metrics.json")));
    assert_eq!(inputs, [PathBuf::from("src")]);
}

#[test]
fn metrics_parser_reports_unknown_flags() {
    let error = match parse(&["metrics", "--strict"]) {
        Ok(_) => panic!("unknown metrics flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown metrics flag `--strict`");
}

#[test]
fn fmt_parser_accepts_input_paths() {
    let command =
        parse(&["fmt", "src/main.veln", "tests/case.veln"]).expect("fmt command should parse");

    let Command::Fmt { inputs } = command else {
        panic!("expected fmt command");
    };

    assert_eq!(
        inputs,
        [
            PathBuf::from("src/main.veln"),
            PathBuf::from("tests/case.veln")
        ]
    );
}

#[test]
fn fmt_parser_reports_unknown_flags() {
    let error = match parse(&["fmt", "--check"]) {
        Ok(_) => panic!("unknown fmt flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown fmt flag `--check`");
}

#[test]
fn run_parser_preserves_entry_args_after_separator() {
    let command = parse(&[
        "run",
        "--json",
        "main",
        "src/main.veln",
        "--",
        "--name",
        "Ada",
    ])
    .expect("run command should parse");

    let Command::Run {
        json,
        entry,
        inputs,
        entry_args,
    } = command
    else {
        panic!("expected run command");
    };

    assert!(json);
    assert_eq!(entry, "main");
    assert_eq!(inputs, [std::path::PathBuf::from("src/main.veln")]);
    assert_eq!(entry_args, ["--name", "Ada"]);
}

#[test]
fn run_parser_keeps_help_like_entry_args_after_separator() {
    let command = parse(&["run", "main", "--", "-h", "--help"]).expect("run command should parse");

    let Command::Run { entry_args, .. } = command else {
        panic!("expected run command");
    };

    assert_eq!(entry_args, ["-h", "--help"]);
}

#[test]
fn run_parser_reports_flags_before_separator_as_run_flags() {
    let error = match parse(&["run", "main", "--name"]) {
        Ok(_) => panic!("flag before separator should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown run flag `--name`");
}

#[test]
fn run_parser_requires_entry_before_separator() {
    let error = match parse(&["run", "--", "--name", "Ada"]) {
        Ok(_) => panic!("separator before entry should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "run requires an entry function name");
}

#[test]
fn subcommands_return_help_for_help_flags() {
    assert!(matches!(
        parse(&["check", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["doc", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["fmt", "-h"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["run", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["test", "-h"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["repair", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["explain", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["package", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["package", "lock", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["lsp", "--help"]).unwrap(),
        Command::Help { .. }
    ));
    assert!(matches!(
        parse(&["mcp", "--help"]).unwrap(),
        Command::Help { .. }
    ));
}

#[test]
fn test_parser_accepts_json_and_targets() {
    let command =
        parse(&["test", "--json", "src/main.veln", "tests"]).expect("test command should parse");

    let Command::Test {
        json,
        jobs,
        targets,
    } = command
    else {
        panic!("expected test command");
    };

    assert!(json);
    assert_eq!(jobs, None);
    assert_eq!(
        targets,
        [PathBuf::from("src/main.veln"), PathBuf::from("tests")]
    );
}

#[test]
fn test_parser_accepts_jobs_spellings_and_placement() {
    let command = parse(&["test", "--json", "src/main.veln", "-j", "2"])
        .expect("short jobs flag after a target should parse");
    let Command::Test {
        json,
        jobs,
        targets,
    } = command
    else {
        panic!("expected test command");
    };
    assert!(json);
    assert_eq!(jobs, Some(2));
    assert_eq!(targets, [PathBuf::from("src/main.veln")]);

    let command = parse(&["test", "--jobs", "3", "src/main.veln", "tests"])
        .expect("long jobs flag should parse");
    let Command::Test { jobs, targets, .. } = command else {
        panic!("expected test command");
    };
    assert_eq!(jobs, Some(3));
    assert_eq!(
        targets,
        [PathBuf::from("src/main.veln"), PathBuf::from("tests")]
    );
}

#[test]
fn test_parser_treats_jobs_after_separator_as_target() {
    let command = parse(&["test", "--json", "--", "--jobs", "2"])
        .expect("jobs after separator should be a target");
    let Command::Test {
        json,
        jobs,
        targets,
    } = command
    else {
        panic!("expected test command");
    };

    assert!(json);
    assert_eq!(jobs, None);
    assert_eq!(targets, [PathBuf::from("--jobs"), PathBuf::from("2")]);
}

#[test]
fn test_parser_rejects_invalid_jobs_values() {
    for args in [
        &["test", "--jobs", "0"][..],
        &["test", "--jobs"][..],
        &["test", "--jobs", "many"][..],
        &["test", "-j", "-1"][..],
        &["test", "--jobs", "184467440737095516160"][..],
        &["test", "-j", "2", "--jobs", "3"][..],
    ] {
        assert!(
            parse(args).is_err(),
            "invalid jobs arguments should fail: {args:?}"
        );
    }
}

#[test]
fn test_parser_reports_unknown_flags() {
    let error = match parse(&["test", "--filter"]) {
        Ok(_) => panic!("unknown test flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown test flag `--filter`");
}

#[test]
fn repair_parser_accepts_apply_candidate_json_and_inputs() {
    let command = parse(&[
        "repair",
        "--json",
        "--apply",
        "--candidate",
        "repair-1",
        "src/main.veln",
    ])
    .expect("repair command should parse");

    let Command::Repair {
        json,
        apply,
        candidate_id,
        confirm_id,
        override_requested,
        inputs,
    } = command
    else {
        panic!("expected repair command");
    };

    assert!(json);
    assert!(apply);
    assert_eq!(candidate_id.as_deref(), Some("repair-1"));
    assert_eq!(confirm_id, None);
    assert!(!override_requested);
    assert_eq!(inputs, [PathBuf::from("src/main.veln")]);
}

#[test]
fn repair_parser_accepts_confirmed_override() {
    let command = parse(&[
        "repair",
        "--apply",
        "--override",
        "--confirm",
        "symbol-1",
        "src/main.veln",
    ])
    .expect("confirmed override should parse");

    let Command::Repair {
        apply,
        candidate_id,
        confirm_id,
        override_requested,
        inputs,
        ..
    } = command
    else {
        panic!("expected repair command");
    };

    assert!(apply);
    assert_eq!(candidate_id, None);
    assert_eq!(confirm_id.as_deref(), Some("symbol-1"));
    assert!(override_requested);
    assert_eq!(inputs, [PathBuf::from("src/main.veln")]);
}

#[test]
fn repair_parser_reports_unknown_flags() {
    let error = match parse(&["repair", "--force"]) {
        Ok(_) => panic!("unknown repair flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown repair flag `--force`");
}

#[test]
fn explain_parser_accepts_list_with_diagnostic_id() {
    let command =
        parse(&["explain", "--list", "hole.unfilled"]).expect("explain command should parse");

    let Command::Explain {
        list,
        diagnostic_id,
    } = command
    else {
        panic!("expected explain command");
    };

    assert!(list);
    assert_eq!(diagnostic_id.as_deref(), Some("hole.unfilled"));
}

#[test]
fn explain_parser_rejects_extra_diagnostic_ids() {
    let error = match parse(&["explain", "hole.unfilled", "type.mismatch"]) {
        Ok(_) => panic!("extra explain argument should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unexpected explain argument `type.mismatch`");
}

#[test]
fn explain_parser_reports_unknown_flags() {
    let error = match parse(&["explain", "--json"]) {
        Ok(_) => panic!("unknown explain flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown explain flag `--json`");
}

#[test]
fn package_parser_accepts_lock_subcommand() {
    assert!(matches!(
        parse(&["package", "lock"]).unwrap(),
        Command::PackageLock
    ));
}

#[test]
fn package_parser_reports_unknown_lock_flags() {
    let error = match parse(&["package", "lock", "--json"]) {
        Ok(_) => panic!("unknown package lock flag should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown package lock flag `--json`");
}

#[test]
fn package_parser_reports_unknown_subcommands() {
    let error = match parse(&["package", "fetch"]) {
        Ok(_) => panic!("unknown package subcommand should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unknown package subcommand `fetch`");
}

#[test]
fn lsp_parser_accepts_no_arguments() {
    assert!(matches!(parse(&["lsp"]).unwrap(), Command::Lsp));
}

#[test]
fn lsp_parser_rejects_arguments() {
    let error = match parse(&["lsp", "main.veln"]) {
        Ok(_) => panic!("lsp arguments should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unexpected lsp argument `main.veln`");
}

#[test]
fn mcp_parser_accepts_no_arguments() {
    assert!(matches!(parse(&["mcp"]).unwrap(), Command::Mcp));
}

#[test]
fn mcp_parser_rejects_arguments() {
    let error = match parse(&["mcp", "main.veln"]) {
        Ok(_) => panic!("mcp arguments should fail"),
        Err(error) => error,
    };

    assert_eq!(error, "unexpected mcp argument `main.veln`");
}
