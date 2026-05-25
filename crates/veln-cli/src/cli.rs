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

#[cfg(test)]
mod tests {
    use super::Command;

    fn parse(args: &[&str]) -> Result<Command, String> {
        Command::parse(args.iter().map(|arg| arg.to_string()).collect())
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
}
