use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_repo_language_reference::{generate_from_workspace, write_checked_artifacts};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    if args.next().is_some() || !matches!(command.as_str(), "generate" | "verify") {
        eprintln!("usage: cargo run -p veln-repo-language-reference -- generate|verify");
        return ExitCode::FAILURE;
    }

    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("maintenance package should be under tools")
        .to_path_buf();
    let generated = match generate_from_workspace(&workspace) {
        Ok(generated) => generated,
        Err(error) => {
            eprintln!("language-reference generation failed: {error}");
            return ExitCode::FAILURE;
        }
    };

    if command == "generate" {
        if let Err(error) = write_checked_artifacts(&workspace, &generated) {
            eprintln!("language-reference generation failed: {error}");
            return ExitCode::FAILURE;
        }
        println!("updated the checked language-reference artifact and digest");
        return ExitCode::SUCCESS;
    }

    match veln_repo_language_reference::verify_checked_artifacts(&workspace, &generated) {
        Ok(()) => {
            println!("checked language-reference artifact and digest are current");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("language-reference freshness check failed: {error}");
            ExitCode::FAILURE
        }
    }
}
