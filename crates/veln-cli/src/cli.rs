use std::path::PathBuf;

use clap::{Arg, ArgAction, Command as ClapCommand};

pub(crate) enum Command {
    Check {
        json: bool,
        inputs: Vec<PathBuf>,
    },
    Doc {
        inputs: Vec<PathBuf>,
    },
    Fmt {
        inputs: Vec<PathBuf>,
    },
    Run {
        json: bool,
        entry: String,
        inputs: Vec<PathBuf>,
        entry_args: Vec<String>,
    },
    Test {
        json: bool,
        targets: Vec<PathBuf>,
    },
    Repair {
        json: bool,
        apply: bool,
        candidate_id: Option<String>,
        confirm_id: Option<String>,
        override_requested: bool,
        inputs: Vec<PathBuf>,
    },
    Explain {
        list: bool,
        diagnostic_id: Option<String>,
    },
    PackageLock,
    Lsp,
    Help {
        text: String,
    },
    Version,
}

impl Command {
    pub(crate) fn parse(args: Vec<String>) -> Result<Self, String> {
        if let Some(command) = parse_help_or_version(&args)? {
            return Ok(command);
        }
        validate_command_args(&args)?;

        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push("veln".to_string());
        argv.extend(args);

        let matches = app()
            .try_get_matches_from(argv)
            .map_err(|error| error.to_string())?;

        Ok(command_from_matches(&matches))
    }
}

fn app() -> ClapCommand {
    ClapCommand::new("veln")
        .version(env!("CARGO_PKG_VERSION"))
        .disable_help_subcommand(false)
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(check_command())
        .subcommand(doc_command())
        .subcommand(fmt_command())
        .subcommand(run_command())
        .subcommand(test_command())
        .subcommand(repair_command())
        .subcommand(explain_command())
        .subcommand(package_command())
        .subcommand(lsp_command())
}

fn check_command() -> ClapCommand {
    ClapCommand::new("check")
        .about("Check source files")
        .arg(json_arg())
        .arg(path_args(
            "inputs",
            "Source files or directories to check",
            "INPUTS",
        ))
}

fn doc_command() -> ClapCommand {
    ClapCommand::new("doc")
        .about("Generate documentation")
        .arg(path_args(
            "inputs",
            "Source files or directories to document",
            "INPUTS",
        ))
}

fn fmt_command() -> ClapCommand {
    ClapCommand::new("fmt")
        .about("Format source files")
        .arg(path_args(
            "inputs",
            "Source files or directories to format",
            "INPUTS",
        ))
}

fn run_command() -> ClapCommand {
    ClapCommand::new("run")
        .about("Run an entry function")
        .arg(json_arg())
        .arg(
            Arg::new("entry")
                .help("Entry function name")
                .value_name("ENTRY")
                .required(true),
        )
        .arg(path_args(
            "inputs",
            "Source files or directories to run",
            "INPUTS",
        ))
        .arg(
            Arg::new("entry_args")
                .help("Arguments passed to the entry function after `--`")
                .value_name("ENTRY_ARGS")
                .num_args(0..)
                .last(true)
                .allow_hyphen_values(true),
        )
}

fn test_command() -> ClapCommand {
    ClapCommand::new("test")
        .about("Run tests")
        .arg(json_arg())
        .arg(path_args(
            "targets",
            "Source files, directories, or test targets",
            "TARGETS",
        ))
}

fn repair_command() -> ClapCommand {
    ClapCommand::new("repair")
        .about("Preview or apply repair candidates")
        .arg(json_arg())
        .arg(repair_apply_arg())
        .arg(repair_dry_run_arg())
        .arg(repair_candidate_arg())
        .arg(repair_confirm_arg())
        .arg(repair_override_arg())
        .arg(path_args(
            "inputs",
            "Source files, directories, or saved repair JSON files",
            "INPUTS",
        ))
}

fn repair_apply_arg() -> Arg {
    Arg::new("apply")
        .long("apply")
        .help("Apply one safe repair candidate")
        .conflicts_with("dry_run")
        .action(ArgAction::SetTrue)
}

fn repair_dry_run_arg() -> Arg {
    Arg::new("dry_run")
        .long("dry-run")
        .help("Preview repair candidates without writing files")
        .action(ArgAction::SetTrue)
}

fn repair_candidate_arg() -> Arg {
    Arg::new("candidate")
        .long("candidate")
        .help("Repair candidate id to apply or select")
        .value_name("CANDIDATE_ID")
        .num_args(1)
}

fn repair_confirm_arg() -> Arg {
    Arg::new("confirm")
        .long("confirm")
        .help("Confirm a repair candidate id before applying")
        .value_name("CANDIDATE_ID")
        .requires("apply")
        .num_args(1)
}

fn repair_override_arg() -> Arg {
    Arg::new("override")
        .long("override")
        .help("Apply a confirmed manual-review repair candidate")
        .requires("apply")
        .requires("confirm")
        .action(ArgAction::SetTrue)
}

fn explain_command() -> ClapCommand {
    ClapCommand::new("explain")
        .about("Explain diagnostics")
        .arg(
            Arg::new("list")
                .long("list")
                .help("List known diagnostics")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("diagnostic_id")
                .help("Diagnostic id to explain")
                .value_name("DIAGNOSTIC_ID")
                .num_args(0..=1),
        )
}

fn package_command() -> ClapCommand {
    ClapCommand::new("package")
        .about("Manage package dependencies")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            ClapCommand::new("lock").about("Write veln.lock for path and local git dependencies"),
        )
}

fn lsp_command() -> ClapCommand {
    ClapCommand::new("lsp").about("Run the language server on stdio")
}

fn path_args(name: &'static str, help: &'static str, value_name: &'static str) -> Arg {
    Arg::new(name)
        .help(help)
        .value_name(value_name)
        .num_args(0..)
        .value_parser(clap::value_parser!(PathBuf))
}

fn json_arg() -> Arg {
    Arg::new("json")
        .long("json")
        .help("Emit machine-readable JSON")
        .action(ArgAction::SetTrue)
}

fn parse_help_or_version(args: &[String]) -> Result<Option<Command>, String> {
    if args.is_empty() {
        return Ok(Some(Command::Help {
            text: render_help(&[]),
        }));
    }

    let first = args
        .first()
        .expect("empty arguments are handled before reading the first argument");
    match first.as_str() {
        "package" if has_help_flag(args.iter().skip(1)) => Ok(Some(Command::Help {
            text: render_help(package_help_path(args)),
        })),
        "check" | "doc" | "fmt" | "test" | "repair" | "explain" | "lsp"
            if has_help_flag(args.iter().skip(1)) =>
        {
            Ok(Some(Command::Help {
                text: render_help(&args[..1]),
            }))
        }
        "run" if has_help_flag_before_separator(args.iter().skip(1)) => Ok(Some(Command::Help {
            text: render_help(&args[..1]),
        })),
        "--help" | "-h" => Ok(Some(Command::Help {
            text: render_help(&[]),
        })),
        "help" => {
            reject_unknown_help_topic(&args[1..])?;
            Ok(Some(Command::Help {
                text: render_help(&args[1..]),
            }))
        }
        "--version" | "-V" | "version" => Ok(Some(Command::Version)),
        _ => Ok(None),
    }
}

fn validate_command_args(args: &[String]) -> Result<(), String> {
    let first = args
        .first()
        .expect("help and version handling rejects empty arguments");
    match first.as_str() {
        "check" => reject_unknown_check_flags(args.iter().skip(1)),
        "doc" => reject_unknown_doc_flags(args.iter().skip(1)),
        "fmt" => reject_unknown_fmt_flags(args.iter().skip(1)),
        "run" => {
            reject_unknown_run_flags(args.iter().skip(1))?;
            reject_missing_run_entry(args.iter().skip(1))
        }
        "test" => reject_unknown_test_flags(args.iter().skip(1)),
        "repair" => reject_unknown_repair_flags(args.iter().skip(1)),
        "explain" => reject_unknown_explain_flags(args.iter().skip(1)),
        "package" => reject_unknown_package_args(args.iter().skip(1)),
        "lsp" => reject_lsp_arguments(args.iter().skip(1)),
        command => Err(format!("unknown command `{command}`")),
    }
}

fn command_from_matches(matches: &clap::ArgMatches) -> Command {
    match matches.subcommand() {
        Some(("check", matches)) => Command::Check {
            json: matches.get_flag("json"),
            inputs: path_values(matches, "inputs"),
        },
        Some(("doc", matches)) => Command::Doc {
            inputs: path_values(matches, "inputs"),
        },
        Some(("fmt", matches)) => Command::Fmt {
            inputs: path_values(matches, "inputs"),
        },
        Some(("run", matches)) => run_from_matches(matches),
        Some(("test", matches)) => Command::Test {
            json: matches.get_flag("json"),
            targets: path_values(matches, "targets"),
        },
        Some(("repair", matches)) => Command::Repair {
            json: matches.get_flag("json"),
            apply: matches.get_flag("apply"),
            candidate_id: matches.get_one::<String>("candidate").cloned(),
            confirm_id: matches.get_one::<String>("confirm").cloned(),
            override_requested: matches.get_flag("override"),
            inputs: path_values(matches, "inputs"),
        },
        Some(("explain", matches)) => Command::Explain {
            list: matches.get_flag("list"),
            diagnostic_id: matches.get_one::<String>("diagnostic_id").cloned(),
        },
        Some(("package", matches)) => match matches.subcommand() {
            Some(("lock", _)) => Command::PackageLock,
            _ => Command::Help {
                text: render_help(&["package".to_string()]),
            },
        },
        Some(("lsp", _)) => Command::Lsp,
        _ => Command::Help {
            text: render_help(&[]),
        },
    }
}

fn run_from_matches(matches: &clap::ArgMatches) -> Command {
    let entry = matches
        .get_one::<String>("entry")
        .expect("clap requires an entry argument")
        .to_string();
    Command::Run {
        json: matches.get_flag("json"),
        entry,
        inputs: path_values(matches, "inputs"),
        entry_args: string_values(matches, "entry_args"),
    }
}

fn render_help(path: &[String]) -> String {
    if path.is_empty() {
        return app().render_help().to_string();
    }

    let mut argv = Vec::with_capacity(path.len() + 2);
    argv.push("veln".to_string());
    argv.extend(path.iter().cloned());
    argv.push("--help".to_string());
    match app().try_get_matches_from(argv) {
        Ok(_) => app().render_help().to_string(),
        Err(error) => error.to_string(),
    }
}

fn path_values(matches: &clap::ArgMatches, name: &str) -> Vec<PathBuf> {
    matches
        .get_many::<PathBuf>(name)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

fn string_values(matches: &clap::ArgMatches, name: &str) -> Vec<String> {
    matches
        .get_many::<String>(name)
        .map(|values| values.cloned().collect())
        .unwrap_or_default()
}

fn has_help_flag<'a>(args: impl Iterator<Item = &'a String>) -> bool {
    args.into_iter()
        .any(|arg| matches!(arg.as_str(), "--help" | "-h"))
}

fn has_help_flag_before_separator<'a>(args: impl Iterator<Item = &'a String>) -> bool {
    for arg in args {
        match arg.as_str() {
            "--" => return false,
            "--help" | "-h" => return true,
            _ => {}
        }
    }
    false
}

fn package_help_path(args: &[String]) -> &[String] {
    if args.get(1).is_some_and(|arg| arg == "lock") {
        &args[..2]
    } else {
        &args[..1]
    }
}

fn reject_unknown_check_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--json" | "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown check flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_unknown_doc_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown doc flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_unknown_fmt_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown fmt flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_unknown_run_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--json" | "--help" | "-h" => {}
            "--" => return Ok(()),
            flag if flag.starts_with('-') => return Err(format!("unknown run flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_missing_run_entry<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--json" => {}
            "--" => return Err("run requires an entry function name".to_string()),
            _ => return Ok(()),
        }
    }
    Err("run requires an entry function name".to_string())
}

fn reject_unknown_test_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--json" | "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown test flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_unknown_repair_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" | "--apply" | "--dry-run" | "--override" | "--help" | "-h" => {}
            "--candidate" | "--confirm" => {
                let Some(value) = args.next() else {
                    return Err(format!("repair flag `{arg}` requires a value"));
                };
                if value.starts_with('-') {
                    return Err(format!("repair flag `{arg}` requires a value"));
                }
            }
            flag if flag.starts_with('-') => return Err(format!("unknown repair flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_unknown_explain_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    let mut diagnostic_id = None;
    for arg in args {
        match arg.as_str() {
            "--list" | "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown explain flag `{flag}`")),
            id if diagnostic_id.is_some() => {
                return Err(format!("unexpected explain argument `{id}`"));
            }
            id => diagnostic_id = Some(id),
        }
    }
    Ok(())
}

fn reject_unknown_package_args<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    let args = args.collect::<Vec<_>>();
    let Some(first) = args.first() else {
        return Err("package requires a subcommand".to_string());
    };
    match first.as_str() {
        "lock" => {
            for arg in args.iter().skip(1) {
                match arg.as_str() {
                    "--help" | "-h" => {}
                    flag if flag.starts_with('-') => {
                        return Err(format!("unknown package lock flag `{flag}`"));
                    }
                    value => return Err(format!("unexpected package lock argument `{value}`")),
                }
            }
            Ok(())
        }
        "--help" | "-h" => Ok(()),
        subcommand if subcommand.starts_with('-') => {
            Err(format!("unknown package flag `{subcommand}`"))
        }
        subcommand => Err(format!("unknown package subcommand `{subcommand}`")),
    }
}

fn reject_lsp_arguments<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown lsp flag `{flag}`")),
            value => return Err(format!("unexpected lsp argument `{value}`")),
        }
    }
    Ok(())
}

fn reject_unknown_help_topic(path: &[String]) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }

    if path.first().is_some_and(|command| command == "package") {
        return reject_package_help_topic(path);
    }

    if path.len() > 1 {
        return Err(format!("unexpected help argument `{}`", path[1]));
    }

    match path[0].as_str() {
        "check" | "fmt" | "run" | "test" | "repair" | "explain" | "package" | "lsp" | "help" => {
            Ok(())
        }
        command => Err(format!("unknown command `{command}`")),
    }
}

fn reject_package_help_topic(path: &[String]) -> Result<(), String> {
    if path.len() > 2 {
        return Err(format!("unexpected help argument `{}`", path[2]));
    }
    match path.get(1).map(String::as_str) {
        None | Some("lock") => Ok(()),
        Some(subcommand) => Err(format!("unknown package subcommand `{subcommand}`")),
    }
}

#[cfg(test)]
mod tests {
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
        let command =
            parse(&["run", "main", "--", "-h", "--help"]).expect("run command should parse");

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
    }

    #[test]
    fn test_parser_accepts_json_and_targets() {
        let command = parse(&["test", "--json", "src/main.veln", "tests"])
            .expect("test command should parse");

        let Command::Test { json, targets } = command else {
            panic!("expected test command");
        };

        assert!(json);
        assert_eq!(
            targets,
            [PathBuf::from("src/main.veln"), PathBuf::from("tests")]
        );
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
}
