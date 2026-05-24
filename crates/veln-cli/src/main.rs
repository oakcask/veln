mod cli;
mod commands;
mod diagnostics;
mod java;
mod surface;

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
        Command::Fmt { inputs } => commands::fmt::fmt(inputs),
        Command::Run { entry, inputs } => commands::run::run_entry(entry, inputs),
        Command::Test { json, targets } => commands::test::test(json, targets),
        Command::Help => {
            cli::print_help();
            Ok(ExitCode::SUCCESS)
        }
        Command::Version => {
            println!("veln {}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::SUCCESS)
        }
    }
}
