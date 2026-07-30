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
    match command {
        Command::Check { json, inputs } => commands::check::check(json, inputs),
        Command::Doc { inputs } => commands::doc::doc(inputs),
        Command::Fmt { inputs } => commands::fmt::fmt(inputs),
        Command::Run {
            json,
            entry,
            inputs,
            entry_args,
        } => commands::run::run_entry(json, entry, inputs, entry_args),
        Command::Test {
            json,
            jobs,
            targets,
        } => commands::test::test(json, jobs, targets),
        Command::Repair {
            json,
            apply,
            candidate_id,
            confirm_id,
            override_requested,
            inputs,
        } => commands::repair::repair(
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
        Command::PackageLock => commands::package::lock(),
        Command::Lsp => {
            veln_lsp::run_stdio().map_err(|error| format!("lsp failed: {error}"))?;
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
    }
}
