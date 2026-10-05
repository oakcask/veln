use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use veln_project::Project;
use veln_repo_toolchain_case::{CaseManifest, inventory as toolchain_case_inventory};

const ROOTS: [toolchain_case_inventory::DiscoveryRoot; 2] = [
    toolchain_case_inventory::DiscoveryRoot {
        id: "crates/veln-cli/tests/toolchain_cases",
        relative: "crates/veln-cli/tests/toolchain_cases",
    },
    toolchain_case_inventory::DiscoveryRoot {
        id: "examples/specification",
        relative: "examples/specification",
    },
];

fn main() {
    if env::args()
        .skip(1)
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        println!("usage: veln-repo-toolchain-cases [REPOSITORY_ROOT]");
        return;
    }
    let mut arguments = env::args().skip(1);
    let repo_root = arguments
        .next()
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if let Some(argument) = arguments.next() {
        exit_with_message(
            2,
            format!("remove unexpected argument {argument:?}; pass only the repository root"),
        );
    }

    match check_source_surface(&repo_root) {
        Ok(report) => println!(
            "Executable source-surface grammar accepted {} sources selected from {} toolchain cases.",
            report.source_count, report.case_count
        ),
        Err(message) => exit_with_message(1, message),
    }
}

fn exit_with_message(code: i32, message: String) -> ! {
    eprintln!("{message}");
    std::process::exit(code);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CheckReport {
    case_count: usize,
    source_count: usize,
}

fn check_source_surface(repo_root: &Path) -> Result<CheckReport, String> {
    let repo_root = repo_root.canonicalize().map_err(|error| {
        format!(
            "select a readable repository root before checking source-surface coverage: {error}"
        )
    })?;
    let inventory = toolchain_case_inventory::run_preflight_with_roots(&repo_root, &ROOTS)?;
    let mut case_count = 0;
    let mut sources = BTreeSet::new();

    for descriptor in inventory.cases {
        if !repository_relative(&descriptor.manifest_relative) {
            return Err(format!(
                "{}: keep discovered toolchain cases inside the repository before checking source-surface coverage",
                descriptor.manifest_relative.display()
            ));
        }
        let case_dir = repo_root.join(&descriptor.manifest_relative);
        let manifest_path = case_dir.join("case.toml");
        let text = fs::read_to_string(&manifest_path).map_err(|error| {
            format!(
                "{}: read the toolchain case manifest before scanning accepted sources: {error}",
                manifest_path.display()
            )
        })?;
        let manifest = CaseManifest::parse_for_accepted_source_selection(&manifest_path, &text)?;
        if !manifest.is_accepted_source_case() {
            continue;
        }
        case_count += 1;
        sources.extend(selected_sources(&manifest, &case_dir, &repo_root)?);
    }

    if sources.is_empty() {
        return Err(
            "add or restore at least one successful source-command case; the executable grammar needs toolchain-owned acceptance evidence"
                .to_string(),
        );
    }

    let prolog_spec = repo_root.join("docs/specification/source-surface-executable.pl");
    let output = Command::new("swipl")
        .current_dir(&repo_root)
        .args(["-q", "-s"])
        .arg(&prolog_spec)
        .args(["--", "--check"])
        .args(&sources)
        .output()
        .map_err(|error| {
            format!(
                "install SWI-Prolog and rerun the source-surface check; executable grammar validation could not start: {error}"
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "update the executable source-surface grammar for the toolchain-accepted sources listed below; keeping both parsers aligned prevents accepted Veln syntax from missing its specification coverage:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(CheckReport {
        case_count,
        source_count: sources.len(),
    })
}

fn selected_sources(
    manifest: &CaseManifest,
    case_dir: &Path,
    repo_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    let command_root = manifest.command_root(case_dir);
    let package_root = veln_project::select_package_root(&command_root).map_err(|error| {
        format!(
            "{}: select the package root before checking accepted toolchain sources: {error}",
            case_dir.display()
        )
    })?;
    let mut inputs = manifest.selected_source_inputs(case_dir, &package_root);
    for input in &mut inputs {
        if !input.is_absolute() {
            *input = package_root.join(&*input);
        }
    }
    // The execution harness materializes this shared source in the copied case.
    // Check its repository source while discovering the ordinary case inputs.
    let shared_fake = package_root.join("fake_effects.veln");
    let has_shared_fake = case_dir.join("fixture_effects.veln").is_file()
        && !shared_fake.exists()
        && inputs.contains(&shared_fake);
    if has_shared_fake {
        inputs.retain(|input| input != &shared_fake);
    }
    let project = Project::discover(package_root, &inputs).map_err(|error| {
            format!(
                "{}: discover accepted toolchain sources before checking the executable grammar: {error}",
                case_dir.display()
            )
        })?;
    let mut sources: Vec<_> = project
        .files
        .into_iter()
        .map(|source| project.root.join(source.path().as_str()))
        .collect();
    if has_shared_fake {
        sources.push(repo_root.join("examples/test-support/fake_effects.veln"));
    }
    Ok(sources)
}

fn repository_relative(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_paths_reject_parent_and_absolute_components() {
        assert!(repository_relative(Path::new(
            "examples/specification/check/example"
        )));
        assert!(!repository_relative(Path::new("../outside")));
        assert!(!repository_relative(Path::new("/outside")));
    }

    #[test]
    fn injected_fixture_checks_shared_fake_and_local_sources() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let case_dir = repo_root.join("examples/specification/run/transport-boundary");
        let manifest_path = case_dir.join("case.toml");
        let text = fs::read_to_string(&manifest_path).unwrap();
        let manifest =
            CaseManifest::parse_for_accepted_source_selection(&manifest_path, &text).unwrap();
        let sources = selected_sources(&manifest, &case_dir, &repo_root).unwrap();
        assert_eq!(sources.len(), 3);
        assert!(sources.contains(&case_dir.join("main.veln")));
        assert!(sources.contains(&case_dir.join("fixture_effects.veln")));
        assert!(sources.contains(&repo_root.join("examples/test-support/fake_effects.veln")));
    }

    #[test]
    fn unmarked_case_does_not_replace_missing_source_with_shared_fake() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let manifest = CaseManifest::parse_for_accepted_source_selection(
            Path::new("case.toml"),
            "command = [\"check\", \"fake_effects.veln\"]\n",
        )
        .unwrap();
        assert!(selected_sources(&manifest, &repo_root, &repo_root).is_err());
    }
}
