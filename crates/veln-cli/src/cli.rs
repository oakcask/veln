use std::path::PathBuf;

pub(crate) enum Command {
    Check {
        json: bool,
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
    Explain {
        list: bool,
        diagnostic_id: Option<String>,
    },
    Lsp,
    Help,
    Version,
}

impl Command {
    pub(crate) fn parse(args: Vec<String>) -> Result<Self, String> {
        let Some(first) = args.first() else {
            return Ok(Self::Help);
        };
        match first.as_str() {
            "check" => parse_check(args.into_iter().skip(1)),
            "fmt" => parse_fmt(args.into_iter().skip(1)),
            "run" => parse_run(args.into_iter().skip(1)),
            "test" => parse_test(args.into_iter().skip(1)),
            "explain" => parse_explain(args.into_iter().skip(1)),
            "lsp" => parse_lsp(args.into_iter().skip(1)),
            "--help" | "-h" | "help" => Ok(Self::Help),
            "--version" | "-V" | "version" => Ok(Self::Version),
            command => Err(format!("unknown command `{command}`")),
        }
    }
}

pub(crate) fn print_help() {
    println!("veln check [--json] [path ...]");
    println!("veln fmt [path ...]");
    println!("veln run [--json] <entry> [path ...] [-- arg ...]");
    println!("veln test [--json] [target ...]");
    println!("veln explain [--list] [diagnostic-id]");
    println!("veln lsp");
}

fn parse_check(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut json = false;
    let mut inputs = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown check flag `{flag}`")),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Check { json, inputs })
}

fn parse_fmt(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut inputs = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown fmt flag `{flag}`")),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Fmt { inputs })
}

fn parse_run(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut json = false;
    let mut entry = None;
    let mut inputs = Vec::new();
    let mut entry_args = Vec::new();
    let mut after_separator = false;
    for arg in args {
        if after_separator {
            entry_args.push(arg);
            continue;
        }
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Command::Help),
            "--" => after_separator = true,
            flag if flag.starts_with('-') => return Err(format!("unknown run flag `{flag}`")),
            value if entry.is_none() => entry = Some(value.to_string()),
            path => inputs.push(PathBuf::from(path)),
        }
    }
    let Some(entry) = entry else {
        return Err("run requires an entry function name".to_string());
    };
    Ok(Command::Run {
        json,
        entry,
        inputs,
        entry_args,
    })
}

fn parse_test(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut json = false;
    let mut targets = Vec::new();
    for arg in args {
        match arg.as_str() {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown test flag `{flag}`")),
            path => targets.push(PathBuf::from(path)),
        }
    }
    Ok(Command::Test { json, targets })
}

fn parse_explain(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut list = false;
    let mut diagnostic_id = None;
    for arg in args {
        match arg.as_str() {
            "--list" => list = true,
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown explain flag `{flag}`")),
            id if diagnostic_id.is_none() => diagnostic_id = Some(id.to_string()),
            id => return Err(format!("unexpected explain argument `{id}`")),
        }
    }
    Ok(Command::Explain {
        list,
        diagnostic_id,
    })
}

fn parse_lsp(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    if let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(Command::Help),
            flag if flag.starts_with('-') => return Err(format!("unknown lsp flag `{flag}`")),
            value => return Err(format!("unexpected lsp argument `{value}`")),
        }
    }
    Ok(Command::Lsp)
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
        assert!(matches!(parse(&[]).unwrap(), Command::Help));
        assert!(matches!(parse(&["help"]).unwrap(), Command::Help));
        assert!(matches!(parse(&["--help"]).unwrap(), Command::Help));
        assert!(matches!(parse(&["-h"]).unwrap(), Command::Help));
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
            Command::Help
        ));
        assert!(matches!(parse(&["fmt", "-h"]).unwrap(), Command::Help));
        assert!(matches!(parse(&["run", "--help"]).unwrap(), Command::Help));
        assert!(matches!(parse(&["test", "-h"]).unwrap(), Command::Help));
        assert!(matches!(
            parse(&["explain", "--help"]).unwrap(),
            Command::Help
        ));
        assert!(matches!(parse(&["lsp", "--help"]).unwrap(), Command::Help));
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
