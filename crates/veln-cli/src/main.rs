mod cli;
mod commands;
mod diagnostics;
mod java;

use std::env;
use std::process::ExitCode;

use cli::Command;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(exit_code) => exit_code,
        Err(message) => {
            eprintln!("veln: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let command = Command::parse(args)?;
    let analysis_start = analysis_start_for(&command)?;
    let analysis_start = || {
        analysis_start
            .clone()
            .expect("analysis commands should select a package root")
    };
    run_command(command, analysis_start)
}

fn analysis_start_for(command: &Command) -> Result<Option<commands::CommandAnalysisStart>, String> {
    analysis_command(command)
        .then(commands::CommandAnalysisStart::select)
        .transpose()
}

fn analysis_command(command: &Command) -> bool {
    matches!(
        command,
        Command::Check { .. }
            | Command::Doc { .. }
            | Command::Fmt { .. }
            | Command::Metrics { .. }
            | Command::Run { .. }
            | Command::Test { .. }
            | Command::Repair { .. }
            | Command::PackageLock
    )
}

fn run_command(
    command: Command,
    analysis_start: impl Fn() -> commands::CommandAnalysisStart,
) -> Result<ExitCode, String> {
    if analysis_command(&command) {
        return run_analysis_command(command, &analysis_start);
    }
    run_standalone_command(command)
}

fn run_analysis_command(
    command: Command,
    analysis_start: &impl Fn() -> commands::CommandAnalysisStart,
) -> Result<ExitCode, String> {
    match command {
        Command::Check { json, inputs } => commands::check::check(analysis_start(), json, inputs),
        Command::Doc { inputs } => commands::doc::doc(analysis_start(), inputs),
        Command::Fmt { inputs } => commands::fmt::fmt(analysis_start(), inputs),
        Command::Metrics {
            json,
            check,
            baseline,
            write_baseline,
            inputs,
        } => commands::metrics::metrics(
            analysis_start(),
            json,
            check,
            baseline,
            write_baseline,
            inputs,
        ),
        Command::Run {
            json,
            entry,
            inputs,
            entry_args,
        } => commands::run::run_entry(analysis_start(), json, entry, inputs, entry_args),
        Command::Test {
            json,
            jobs,
            targets,
        } => commands::test::test(analysis_start(), json, jobs, targets),
        Command::Repair {
            json,
            apply,
            candidate_id,
            confirm_id,
            override_requested,
            inputs,
        } => commands::repair::repair(
            analysis_start(),
            json,
            apply,
            candidate_id,
            confirm_id,
            override_requested,
            inputs,
        ),
        Command::Explain {
            list,
            diagnostic_id,
        } => commands::explain::explain(list, diagnostic_id),
        Command::PackageLock => commands::package::lock(analysis_start()),
        _ => unreachable!("analysis command predicate should only route analysis commands"),
    }
}

fn run_standalone_command(command: Command) -> Result<ExitCode, String> {
    match command {
        Command::Explain {
            list,
            diagnostic_id,
        } => commands::explain::explain(list, diagnostic_id),
        Command::Lsp => {
            veln_lsp::run_stdio().map_err(|error| format!("lsp failed: {error}"))?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Mcp => {
            veln_mcp::run_stdio().map_err(|error| format!("mcp failed: {error}"))?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Help { text } => {
            print!("{text}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("veln {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
        _ => unreachable!("analysis commands should be routed before standalone execution"),
    }
}
