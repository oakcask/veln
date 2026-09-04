use std::path::PathBuf;

use veln_repo_mcp_standard_library_docs::{
    generate_checked_bundle, verify_checked_artifact, verify_freshness, write_checked_outputs,
};

struct Cli {
    repository_root: PathBuf,
    command: Command,
}

enum Command {
    Generate,
    Check,
    CheckFresh,
}

fn main() {
    let cli = parse_cli().unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2);
    });
    let result = match cli.command {
        Command::Generate => generate_checked_bundle()
            .and_then(|generated| write_checked_outputs(&cli.repository_root, &generated)),
        Command::Check => verify_checked_artifact(),
        Command::CheckFresh => verify_freshness().map_err(|mismatch| {
            format!(
                "run `cargo run --locked -p veln-repo-mcp-standard-library-docs -- . generate` to regenerate the MCP standard-library package-documentation resources; artifact_matches={}, digest_matches={}, checked_digest={}, generated_digest={}",
                mismatch.artifact_matches,
                mismatch.digest_matches,
                mismatch.checked_digest,
                mismatch.generated_digest
            )
        }),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn parse_cli() -> Result<Cli, String> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "-h" | "--help"))
    {
        return Err(usage());
    }
    let (repository_root, command) = match arguments.as_slice() {
        [command] => (PathBuf::from("."), parse_command(command)?),
        [root, command] => (PathBuf::from(root), parse_command(command)?),
        _ => return Err(usage()),
    };
    Ok(Cli {
        repository_root,
        command,
    })
}

fn parse_command(command: &str) -> Result<Command, String> {
    match command {
        "generate" => Ok(Command::Generate),
        "check" => Ok(Command::Check),
        "check-fresh" => Ok(Command::CheckFresh),
        _ => Err(format!(
            "remove unknown command `{command}`; expected generate, check, or check-fresh"
        )),
    }
}

fn usage() -> String {
    "usage: veln-repo-mcp-standard-library-docs [REPOSITORY_ROOT] <generate|check|check-fresh>"
        .to_string()
}
