use std::path::{Path, PathBuf};

pub mod inventory;
pub mod manifest_syntax;

use manifest_syntax::Statement;

#[derive(Debug)]
pub struct CaseManifest {
    command: Vec<String>,
    cwd: Option<PathBuf>,
    exit: i32,
    has_manifest_error: bool,
    skip_platforms: Vec<String>,
    source_errors: String,
}

impl CaseManifest {
    pub fn parse_for_source_selection(path: &Path, text: &str) -> Result<Self, String> {
        Self::parse(path, text, false)
    }

    pub fn parse_for_accepted_source_selection(path: &Path, text: &str) -> Result<Self, String> {
        Self::parse(path, text, true)
    }

    fn parse(path: &Path, text: &str, parse_acceptance: bool) -> Result<Self, String> {
        let mut section = String::new();
        let mut manifest = Self {
            command: Vec::new(),
            cwd: None,
            exit: 0,
            has_manifest_error: false,
            skip_platforms: Vec::new(),
            source_errors: "forbidden".to_string(),
        };

        for statement in manifest_syntax::parse_document(path, text) {
            match statement {
                Statement::Section { name, .. } => {
                    manifest.has_manifest_error |= parse_acceptance && name == "manifest_error";
                    section = name;
                }
                Statement::Assignment { key, value, .. } => match (section.as_str(), key) {
                    ("", "command") => manifest.command = value.parse_string_array(path),
                    ("", "cwd") => manifest.cwd = Some(PathBuf::from(value.parse_string(path))),
                    ("", "exit") if parse_acceptance => {
                        manifest.exit = value.raw().parse().map_err(|error| {
                            format!(
                                "{}:{}: use an integer exit expectation: {error}",
                                path.display(),
                                value.line()
                            )
                        })?;
                    }
                    ("", "source_errors") if parse_acceptance => {
                        manifest.source_errors = value.parse_string(path)
                    }
                    ("skip", "platforms") if parse_acceptance => {
                        manifest.skip_platforms = value.parse_string_array(path)
                    }
                    _ => {}
                },
            }
        }
        Ok(manifest)
    }

    pub fn has_command(&self) -> bool {
        !self.command.is_empty()
    }

    pub fn is_accepted_source_case(&self) -> bool {
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

    pub fn command_root(&self, case_dir: &Path) -> PathBuf {
        self.cwd
            .as_deref()
            .map_or_else(|| case_dir.to_path_buf(), |cwd| case_dir.join(cwd))
    }

    pub fn selected_source_inputs(&self, case_dir: &Path, package_root: &Path) -> Vec<PathBuf> {
        let command_root = self.command_root(case_dir);
        command_source_inputs(&self.command)
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
            .collect()
    }
}

fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    match command.first().map(String::as_str) {
        Some("run") => command[1..]
            .iter()
            .take_while(|argument| argument.as_str() != "--")
            .filter(|argument| argument.as_str() != "--json")
            .skip(1)
            .map(PathBuf::from)
            .collect(),
        Some("check" | "doc" | "fmt" | "metrics" | "test") => {
            source_inputs_after_flags(&command[1..])
        }
        _ => Vec::new(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_case_selection_requires_successful_source_commands() {
        let path = Path::new("case.toml");
        let accepted = CaseManifest::parse_for_accepted_source_selection(
            path,
            "command = [\"check\", \"main.veln\"]\n",
        )
        .expect("manifest should parse");
        assert!(accepted.is_accepted_source_case());

        let failed = CaseManifest::parse_for_accepted_source_selection(
            path,
            "command = [\"check\", \"main.veln\"]\nexit = 1\n",
        )
        .expect("manifest should parse");
        assert!(!failed.is_accepted_source_case());
        let expected_errors = CaseManifest::parse_for_accepted_source_selection(
            path,
            "command = [\"check\", \"main.veln\"]\nsource_errors = \"expected\"\n",
        )
        .expect("manifest should parse");
        assert!(!expected_errors.is_accepted_source_case());
        let help = CaseManifest::parse_for_accepted_source_selection(
            path,
            "command = [\"check\", \"--help\"]\n",
        )
        .expect("manifest should parse");
        assert!(!help.is_accepted_source_case());
    }

    #[test]
    fn source_inputs_follow_command_flags_and_working_directory() {
        let path = Path::new("case.toml");
        let manifest = CaseManifest::parse_for_source_selection(
            path,
            "command = [\"check\", \"--jobs\", \"2\", \"src/main.veln\"]\ncwd = \"package\"\n",
        )
        .expect("manifest should parse");
        assert_eq!(
            manifest.selected_source_inputs(Path::new("case"), Path::new("case")),
            [PathBuf::from("case/package/src/main.veln")]
        );
    }

    #[test]
    fn source_selection_ignores_acceptance_only_fields() {
        let manifest = CaseManifest::parse_for_source_selection(
            Path::new("case.toml"),
            "command = [\"check\", \"main.veln\"]\nexit = not_an_integer\nsource_errors = 7\n[skip]\nplatforms = 8\n",
        )
        .expect("acceptance-only fields should not affect source selection");

        assert!(manifest.has_command());
    }
}
