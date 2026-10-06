use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, error::ErrorKind};

#[derive(Parser)]
#[command(name = "veln", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Check source files
    Check {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Source files or directories to check
        inputs: Vec<PathBuf>,
    },
    /// Generate documentation
    Doc {
        /// Source files or directories to document
        inputs: Vec<PathBuf>,
    },
    /// Format source files
    Fmt {
        /// Source files or directories to format
        inputs: Vec<PathBuf>,
    },
    /// Report source dependency metrics
    Metrics {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Fail when enabled metrics policy is violated
        #[arg(long)]
        check: bool,
        /// Compare enabled metrics policy against a reviewed baseline
        #[arg(
            long,
            value_name = "PATH",
            requires = "check",
            conflicts_with = "write_baseline"
        )]
        baseline: Option<PathBuf>,
        /// Write the current metrics report as a baseline
        #[arg(long, value_name = "PATH", conflicts_with_all = ["check", "json"])]
        write_baseline: Option<PathBuf>,
        /// Source files or directories to report
        inputs: Vec<PathBuf>,
    },
    /// Run an entry function
    Run {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Entry function name
        entry: String,
        /// Source files or directories to run
        inputs: Vec<PathBuf>,
        /// Arguments passed to the entry function after `--`
        #[arg(last = true, num_args = 0.., allow_hyphen_values = true)]
        entry_args: Vec<String>,
    },
    /// Run tests
    Test {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Maximum runnable test cases to execute concurrently
        #[arg(short = 'j', long, value_parser = positive_jobs)]
        jobs: Option<usize>,
        /// Source files, directories, or test targets
        targets: Vec<PathBuf>,
    },
    /// Preview or apply repair candidates
    Repair {
        /// Emit machine-readable JSON
        #[arg(long)]
        json: bool,
        /// Apply one safe repair candidate
        #[arg(long, conflicts_with = "dry_run")]
        apply: bool,
        /// Preview repair candidates without writing files
        #[arg(long)]
        dry_run: bool,
        /// Repair candidate id to apply or select
        #[arg(long = "candidate", value_name = "CANDIDATE_ID")]
        candidate_id: Option<String>,
        /// Confirm a repair candidate id before applying
        #[arg(long = "confirm", value_name = "CANDIDATE_ID", requires = "apply")]
        confirm_id: Option<String>,
        /// Apply a confirmed manual-review repair candidate
        #[arg(long = "override", requires_all = ["apply", "confirm_id"])]
        override_requested: bool,
        /// Source files, directories, or saved repair JSON files
        inputs: Vec<PathBuf>,
    },
    /// Explain diagnostics
    Explain {
        /// List known diagnostics
        #[arg(long)]
        list: bool,
        /// Diagnostic id to explain
        diagnostic_id: Option<String>,
    },
    /// Manage package dependencies
    Package {
        #[command(subcommand)]
        command: PackageCommand,
    },
    /// Run the language server on stdio
    Lsp,
    /// Run the MCP server on stdio
    Mcp,
    #[command(skip)]
    Help { text: String },
    #[command(hide = true)]
    Version,
}

#[derive(Subcommand)]
pub(crate) enum PackageCommand {
    /// Write veln.lock for path, git, vendor, and mirror dependencies
    Lock,
}

impl Command {
    pub(crate) fn parse(args: Vec<String>) -> Result<Self, String> {
        match Cli::try_parse_from(std::iter::once("veln".to_owned()).chain(args)) {
            Ok(cli) => Ok(cli.command.unwrap_or_else(|| Self::Help {
                text: Cli::command().render_help().to_string(),
            })),
            Err(error) => match error.kind() {
                ErrorKind::DisplayHelp => Ok(Self::Help {
                    text: error.to_string(),
                }),
                ErrorKind::DisplayVersion => Ok(Self::Version),
                _ => Err(error.to_string()),
            },
        }
    }
}

fn positive_jobs(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(jobs) if jobs > 0 => Ok(jobs),
        _ => Err("jobs must be a positive integer".to_owned()),
    }
}

#[cfg(test)]
mod tests;
