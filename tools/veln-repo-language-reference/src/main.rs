use std::path::PathBuf;

use veln_repo_language_reference::{
    generate_checked_catalog, verify_checked_digest, verify_freshness, write_checked_outputs,
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
        Command::Generate => generate_checked_catalog(&cli.repository_root)
            .and_then(|generated| write_checked_outputs(&cli.repository_root, &generated)),
        Command::Check => verify_checked_digest(),
        Command::CheckFresh => verify_freshness(&cli.repository_root).map_err(|mismatch| {
            format!(
                "regenerate the language-reference catalog; artifact_matches={}, digest_matches={}, checked_digest={}, generated_digest={}",
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
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        return Err(
            "usage: veln-repo-language-reference [REPOSITORY_ROOT] <generate|check|check-fresh>"
                .to_string(),
        );
    }
    let (repository_root, command) = match arguments.as_slice() {
        [command] => (PathBuf::from("."), parse_command(command)?),
        [root, command] => (PathBuf::from(root), parse_command(command)?),
        _ => {
            return Err(
                "usage: veln-repo-language-reference [REPOSITORY_ROOT] <generate|check|check-fresh>"
                    .to_string(),
            );
        }
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
            "remove unknown language-reference command `{command}`; expected generate, check, or check-fresh"
        )),
    }
}
