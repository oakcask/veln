use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use veln_project::Project;

#[allow(dead_code)]
#[path = "../../../crates/veln-cli/tests/toolchain_harness/manifest_syntax.rs"]
mod manifest_syntax;
#[allow(dead_code)]
#[path = "../../../crates/veln-cli/toolchain_case_inventory.rs"]
mod toolchain_case_inventory;

use manifest_syntax::Statement as ManifestStatement;

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
        let manifest = SurfaceCaseManifest::read(&manifest_path)?;
        if !manifest.is_accepted_source_case() {
            continue;
        }
        case_count += 1;
        sources.extend(manifest.selected_sources(&case_dir)?);
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

#[derive(Debug)]
struct SurfaceCaseManifest {
    command: Vec<String>,
    cwd: Option<PathBuf>,
    exit: i32,
    has_manifest_error: bool,
    skip_platforms: Vec<String>,
    source_errors: String,
}

impl SurfaceCaseManifest {
    fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "{}: read the toolchain case manifest before scanning accepted sources: {error}",
                path.display()
            )
        })?;
        let mut section = String::new();
        let mut manifest = Self {
            command: Vec::new(),
            cwd: None,
            exit: 0,
            has_manifest_error: false,
            skip_platforms: Vec::new(),
            source_errors: "forbidden".to_string(),
        };

        for statement in manifest_syntax::parse_document(path, &text) {
            match statement {
                ManifestStatement::Section { name, .. } => {
                    manifest.has_manifest_error |= name == "manifest_error";
                    section = name;
                }
                ManifestStatement::Assignment { key, value, .. } => match (section.as_str(), key) {
                    ("", "command") => manifest.command = value.parse_string_array(path),
                    ("", "cwd") => manifest.cwd = Some(PathBuf::from(value.parse_string(path))),
                    ("", "exit") => {
                        manifest.exit = value.raw().parse().map_err(|error| {
                            format!(
                                "{}:{}: use an integer exit expectation: {error}",
                                path.display(),
                                value.line()
                            )
                        })?;
                    }
                    ("", "source_errors") => manifest.source_errors = value.parse_string(path),
                    ("skip", "platforms") => {
                        manifest.skip_platforms = value.parse_string_array(path)
                    }
                    _ => {}
                },
            }
        }
        Ok(manifest)
    }

    fn is_accepted_source_case(&self) -> bool {
        self.exit == 0
            && self.source_errors == "forbidden"
            && !self.has_manifest_error
            && !self
                .skip_platforms
                .iter()
                .any(|platform| platform_matches(platform))
            && !self
                .command
                .iter()
                .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
            && matches!(
                self.command.first().map(String::as_str),
                Some("check" | "doc" | "fmt" | "metrics" | "run" | "test")
            )
    }

    fn selected_sources(&self, case_dir: &Path) -> Result<Vec<PathBuf>, String> {
        let command_root = self
            .cwd
            .as_deref()
            .map_or_else(|| case_dir.to_path_buf(), |cwd| case_dir.join(cwd));
        let package_root = veln_project::select_package_root(&command_root).map_err(|error| {
            format!(
                "{}: select the package root before checking accepted toolchain sources: {error}",
                case_dir.display()
            )
        })?;
        let inputs = command_source_inputs(&self.command)
            .into_iter()
            .map(|input| {
                if input.is_absolute() || command_root == package_root {
                    input
                } else {
                    command_root.join(input)
                }
            })
            .filter(|input| {
                input.is_dir()
                    || input
                        .extension()
                        .is_some_and(|extension| extension == "veln")
            })
            .collect::<Vec<_>>();
        let project = Project::discover(package_root, &inputs).map_err(|error| {
            format!(
                "{}: discover accepted toolchain sources before checking the executable grammar: {error}",
                case_dir.display()
            )
        })?;
        Ok(project
            .files
            .into_iter()
            .map(|source| project.root.join(source.path().as_str()))
            .collect())
    }
}

fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    match command.first().map(String::as_str) {
        Some("run") => run_command_source_inputs(&command[1..]),
        Some("check" | "doc" | "fmt" | "metrics" | "test") => {
            source_inputs_after_flags(&command[1..])
        }
        _ => Vec::new(),
    }
}

fn run_command_source_inputs(arguments: &[String]) -> Vec<PathBuf> {
    arguments
        .iter()
        .take_while(|argument| argument.as_str() != "--")
        .filter(|argument| argument.as_str() != "--json")
        .skip(1)
        .map(PathBuf::from)
        .collect()
}

fn source_inputs_after_flags(arguments: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        if argument == "--json" {
            continue;
        }
        if matches!(
            argument.as_str(),
            "--baseline" | "--write-baseline" | "--jobs" | "-j"
        ) {
            let _ = arguments.next();
            continue;
        }
        if argument.starts_with("--baseline=")
            || argument.starts_with("--write-baseline=")
            || argument.starts_with("--jobs=")
        {
            continue;
        }
        inputs.push(PathBuf::from(argument));
    }
    inputs
}

fn platform_matches(platform: &str) -> bool {
    match platform {
        "unix" => cfg!(unix),
        "windows" => cfg!(windows),
        "macos" => cfg!(target_os = "macos"),
        "linux" => cfg!(target_os = "linux"),
        _ => false,
    }
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
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
    fn accepted_case_selection_requires_successful_source_commands() {
        let mut manifest = SurfaceCaseManifest {
            command: vec!["check".to_string(), "main.veln".to_string()],
            cwd: None,
            exit: 0,
            has_manifest_error: false,
            skip_platforms: Vec::new(),
            source_errors: "forbidden".to_string(),
        };
        assert!(manifest.is_accepted_source_case());

        manifest.exit = 1;
        assert!(!manifest.is_accepted_source_case());
        manifest.exit = 0;
        manifest.source_errors = "expected".to_string();
        assert!(!manifest.is_accepted_source_case());
        manifest.source_errors = "forbidden".to_string();
        manifest.command.push("--help".to_string());
        assert!(!manifest.is_accepted_source_case());
    }

    #[test]
    fn repository_paths_reject_parent_and_absolute_components() {
        assert!(repository_relative(Path::new(
            "examples/specification/check/example"
        )));
        assert!(!repository_relative(Path::new("../outside")));
        assert!(!repository_relative(Path::new("/outside")));
    }
}
