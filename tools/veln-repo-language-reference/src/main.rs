use std::path::PathBuf;

use veln_repo_language_reference::{
    generate_checked_catalog, verify_freshness, write_checked_outputs,
};

struct Cli {
    repository_root: PathBuf,
    command: Command,
}

enum Command {
    Generate,
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
        return Err(usage());
    }
    let (repository_root, command) = match arguments.as_slice() {
        [command] => (PathBuf::from("."), parse_command(command)?),
        [root, command] => (PathBuf::from(root), parse_command(command)?),
        _ => {
            return Err(usage());
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
        "check-fresh" => Ok(Command::CheckFresh),
        _ => Err(format!(
            "remove unknown language-reference command `{command}`; expected generate or check-fresh"
        )),
    }
}

fn usage() -> String {
    "usage: veln-repo-language-reference [REPOSITORY_ROOT] <generate|check-fresh>".to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_command, usage};

    #[test]
    fn check_is_not_a_supported_command() {
        assert!(parse_command("check").is_err());
        assert!(!usage().contains("|check|"));
    }
}
