pub(super) fn validate_command_args(args: &[String]) -> Result<(), String> {
    let first = args
        .first()
        .expect("help and version handling rejects empty arguments");
    let Some((_, validate)) = command_validators()
        .iter()
        .find(|(command, _)| command == first)
    else {
        return Err(format!("unknown command `{first}`"));
    };
    validate(&args[1..])
}

type Validator = fn(&[String]) -> Result<(), String>;

fn command_validators() -> &'static [(&'static str, Validator)] {
    &[
        ("check", validate_check_args),
        ("doc", validate_doc_args),
        ("fmt", validate_fmt_args),
        ("metrics", validate_metrics_args),
        ("run", validate_run_args),
        ("test", validate_test_args),
        ("repair", validate_repair_args),
        ("explain", validate_explain_args),
        ("package", validate_package_args),
        ("lsp", validate_lsp_args),
        ("mcp", validate_mcp_args),
    ]
}

fn validate_check_args(args: &[String]) -> Result<(), String> {
    reject_unknown_check_flags(args.iter())
}

fn validate_doc_args(args: &[String]) -> Result<(), String> {
    reject_unknown_doc_flags(args.iter())
}

fn validate_fmt_args(args: &[String]) -> Result<(), String> {
    reject_unknown_fmt_flags(args.iter())
}

fn validate_metrics_args(args: &[String]) -> Result<(), String> {
    reject_unknown_metrics_flags(args.iter())
}

fn validate_run_args(args: &[String]) -> Result<(), String> {
    reject_unknown_run_flags(args.iter())?;
    reject_missing_run_entry(args.iter())
}

fn validate_test_args(args: &[String]) -> Result<(), String> {
    reject_unknown_test_flags(args.iter())
}

fn validate_repair_args(args: &[String]) -> Result<(), String> {
    reject_unknown_repair_flags(args.iter())
}

fn validate_explain_args(args: &[String]) -> Result<(), String> {
    reject_unknown_explain_flags(args.iter())
}

fn validate_package_args(args: &[String]) -> Result<(), String> {
    reject_unknown_package_args(args.iter())
}

fn validate_lsp_args(args: &[String]) -> Result<(), String> {
    reject_lsp_arguments(args.iter())
}

fn validate_mcp_args(args: &[String]) -> Result<(), String> {
    reject_mcp_arguments(args.iter())
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

fn reject_unknown_metrics_flags<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" | "--check" | "--help" | "-h" => {}
            "--baseline" | "--write-baseline" => {
                let Some(value) = args.next() else {
                    return Err(format!("metrics flag `{arg}` requires a value"));
                };
                if value.starts_with('-') {
                    return Err(format!("metrics flag `{arg}` requires a value"));
                }
            }
            flag if flag.starts_with("--baseline=") || flag.starts_with("--write-baseline=") => {}
            flag if flag.starts_with('-') => return Err(format!("unknown metrics flag `{flag}`")),
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
    let mut args = args.peekable();
    let mut seen_jobs = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json" | "--help" | "-h" => {}
            "--" => return Ok(()),
            "-j" | "--jobs" => {
                reject_repeated_test_jobs(&mut seen_jobs)?;
                let Some(value) = args.next() else {
                    return Err(format!("test flag `{arg}` requires a value"));
                };
                validate_test_jobs_value(arg, value)?;
            }
            flag if flag.starts_with("--jobs=") => {
                reject_repeated_test_jobs(&mut seen_jobs)?;
                let value = flag
                    .split_once('=')
                    .expect("prefix check guarantees an equals sign")
                    .1;
                validate_test_jobs_value("--jobs", value)?;
            }
            flag if flag.starts_with('-') => return Err(format!("unknown test flag `{flag}`")),
            _ => {}
        }
    }
    Ok(())
}

fn reject_repeated_test_jobs(seen_jobs: &mut bool) -> Result<(), String> {
    if *seen_jobs {
        return Err("test jobs flag may only be provided once".to_string());
    }
    *seen_jobs = true;
    Ok(())
}

fn validate_test_jobs_value(flag: &str, value: &str) -> Result<(), String> {
    let jobs = value
        .parse::<usize>()
        .map_err(|_| format!("test flag `{flag}` requires a positive integer value"))?;
    if jobs == 0 {
        return Err(format!(
            "test flag `{flag}` requires a positive integer value"
        ));
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

fn reject_mcp_arguments<'a>(args: impl Iterator<Item = &'a String>) -> Result<(), String> {
    for arg in args {
        match arg.as_str() {
            "--help" | "-h" => {}
            flag if flag.starts_with('-') => return Err(format!("unknown mcp flag `{flag}`")),
            value => return Err(format!("unexpected mcp argument `{value}`")),
        }
    }
    Ok(())
}

pub(super) fn reject_unknown_help_topic(path: &[String]) -> Result<(), String> {
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
        "check" | "doc" | "fmt" | "metrics" | "run" | "test" | "repair" | "explain" | "package"
        | "lsp" | "mcp" | "help" => Ok(()),
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
