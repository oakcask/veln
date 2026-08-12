use std::path::PathBuf;

use clap::{Arg, ArgAction, Command as ClapCommand};

mod validation;

use validation::{reject_unknown_help_topic, validate_command_args};

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
    Metrics {
        json: bool,
        check: bool,
        baseline: Option<PathBuf>,
        write_baseline: Option<PathBuf>,
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
        jobs: Option<usize>,
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
    Mcp,
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
        .subcommand(metrics_command())
        .subcommand(run_command())
        .subcommand(test_command())
        .subcommand(repair_command())
        .subcommand(explain_command())
        .subcommand(package_command())
        .subcommand(lsp_command())
        .subcommand(mcp_command())
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

fn metrics_command() -> ClapCommand {
    ClapCommand::new("metrics")
        .about("Report source dependency metrics")
        .arg(json_arg())
        .arg(check_arg())
        .arg(baseline_arg())
        .arg(write_baseline_arg())
        .arg(path_args(
            "inputs",
            "Source files or directories to report",
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
        .arg(test_jobs_arg())
        .arg(path_args(
            "targets",
            "Source files, directories, or test targets",
            "TARGETS",
        ))
}

fn test_jobs_arg() -> Arg {
    Arg::new("jobs")
        .short('j')
        .long("jobs")
        .help("Maximum runnable test cases to execute concurrently")
        .value_name("JOBS")
        .num_args(1)
        .value_parser(clap::value_parser!(usize))
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
            ClapCommand::new("lock")
                .about("Write veln.lock for path, git, vendor, and mirror dependencies"),
        )
}

fn lsp_command() -> ClapCommand {
    ClapCommand::new("lsp").about("Run the language server on stdio")
}

fn mcp_command() -> ClapCommand {
    ClapCommand::new("mcp").about("Run the MCP server on stdio")
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

fn check_arg() -> Arg {
    Arg::new("check")
        .long("check")
        .help("Fail when enabled metrics policy is violated")
        .action(ArgAction::SetTrue)
}

fn baseline_arg() -> Arg {
    Arg::new("baseline")
        .long("baseline")
        .help("Compare enabled metrics policy against a reviewed baseline")
        .value_name("PATH")
        .num_args(1)
        .value_parser(clap::value_parser!(PathBuf))
        .requires("check")
        .conflicts_with("write_baseline")
}

fn write_baseline_arg() -> Arg {
    Arg::new("write_baseline")
        .long("write-baseline")
        .help("Write the current metrics report as a baseline")
        .value_name("PATH")
        .num_args(1)
        .value_parser(clap::value_parser!(PathBuf))
        .conflicts_with("check")
        .conflicts_with("json")
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
    if matches!(first.as_str(), "--help" | "-h") {
        return Ok(Some(Command::Help {
            text: render_help(&[]),
        }));
    }
    if first == "help" {
        reject_unknown_help_topic(&args[1..])?;
        return Ok(Some(Command::Help {
            text: render_help(&args[1..]),
        }));
    }
    if matches!(first.as_str(), "--version" | "-V" | "version") {
        return Ok(Some(Command::Version));
    }
    Ok(help_for_subcommand(args).map(|text| Command::Help { text }))
}

fn help_for_subcommand(args: &[String]) -> Option<String> {
    let first = args.first()?;
    if first == "package" && has_help_flag(args.iter().skip(1)) {
        return Some(render_help(package_help_path(args)));
    }
    if first == "run" && has_help_flag_before_separator(args.iter().skip(1)) {
        return Some(render_help(&args[..1]));
    }
    if help_path_commands().contains(&first.as_str()) && has_help_flag(args.iter().skip(1)) {
        return Some(render_help(&args[..1]));
    }
    None
}

fn help_path_commands() -> &'static [&'static str] {
    &[
        "check", "doc", "fmt", "metrics", "test", "repair", "explain", "lsp", "mcp",
    ]
}

fn command_from_matches(matches: &clap::ArgMatches) -> Command {
    analysis_command_from_matches(matches)
        .or_else(|| utility_command_from_matches(matches))
        .unwrap_or_else(|| Command::Help {
            text: render_help(&[]),
        })
}

fn analysis_command_from_matches(matches: &clap::ArgMatches) -> Option<Command> {
    simple_analysis_command_from_matches(matches)
        .or_else(|| configured_analysis_command_from_matches(matches))
}

fn simple_analysis_command_from_matches(matches: &clap::ArgMatches) -> Option<Command> {
    match matches.subcommand() {
        Some(("check", matches)) => Some(Command::Check {
            json: matches.get_flag("json"),
            inputs: path_values(matches, "inputs"),
        }),
        Some(("doc", matches)) => Some(Command::Doc {
            inputs: path_values(matches, "inputs"),
        }),
        Some(("fmt", matches)) => Some(Command::Fmt {
            inputs: path_values(matches, "inputs"),
        }),
        Some(("run", matches)) => Some(run_from_matches(matches)),
        _ => None,
    }
}

fn configured_analysis_command_from_matches(matches: &clap::ArgMatches) -> Option<Command> {
    match matches.subcommand() {
        Some(("metrics", matches)) => Some(Command::Metrics {
            json: matches.get_flag("json"),
            check: matches.get_flag("check"),
            baseline: matches.get_one::<PathBuf>("baseline").cloned(),
            write_baseline: matches.get_one::<PathBuf>("write_baseline").cloned(),
            inputs: path_values(matches, "inputs"),
        }),
        Some(("test", matches)) => Some(Command::Test {
            json: matches.get_flag("json"),
            jobs: matches.get_one::<usize>("jobs").copied(),
            targets: path_values(matches, "targets"),
        }),
        Some(("repair", matches)) => Some(Command::Repair {
            json: matches.get_flag("json"),
            apply: matches.get_flag("apply"),
            candidate_id: matches.get_one::<String>("candidate").cloned(),
            confirm_id: matches.get_one::<String>("confirm").cloned(),
            override_requested: matches.get_flag("override"),
            inputs: path_values(matches, "inputs"),
        }),
        _ => None,
    }
}

fn utility_command_from_matches(matches: &clap::ArgMatches) -> Option<Command> {
    match matches.subcommand() {
        Some(("explain", matches)) => Some(Command::Explain {
            list: matches.get_flag("list"),
            diagnostic_id: matches.get_one::<String>("diagnostic_id").cloned(),
        }),
        Some(("package", matches)) => Some(match matches.subcommand() {
            Some(("lock", _)) => Command::PackageLock,
            _ => Command::Help {
                text: render_help(&["package".to_string()]),
            },
        }),
        Some(("lsp", _)) => Some(Command::Lsp),
        Some(("mcp", _)) => Some(Command::Mcp),
        _ => None,
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

#[cfg(test)]
mod tests;
