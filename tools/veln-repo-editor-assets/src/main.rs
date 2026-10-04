use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args().skip(1).collect();
    let [repo, mode] = args.as_slice() else {
        eprintln!(
            "Use `veln-repo-editor-assets <repo> --check|--write` to check or regenerate compiler-owned editor metadata."
        );
        return ExitCode::FAILURE;
    };
    if !matches!(mode.as_str(), "--check" | "--write") {
        eprintln!(
            "Choose --check or --write so editor metadata is verified or regenerated explicitly."
        );
        return ExitCode::FAILURE;
    }
    match veln_repo_editor_assets::synchronize(Path::new(repo), mode == "--write") {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
