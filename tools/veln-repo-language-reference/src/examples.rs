use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use veln_project::Project;

use crate::{Descriptor, ManifestStatement, manifest_syntax, normalize_source_text, slash_path};

pub trait ExampleSource {
    fn selected_examples(
        &self,
        repo_root: &Path,
        descriptors: &[Descriptor],
    ) -> Result<BTreeMap<String, BTreeMap<String, String>>, String>;
}

pub struct RepositoryExampleSource;

impl ExampleSource for RepositoryExampleSource {
    fn selected_examples(
        &self,
        repo_root: &Path,
        descriptors: &[Descriptor],
    ) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
        validate_examples(repo_root, descriptors)
    }
}

pub(crate) fn validate_examples(
    repo_root: &Path,
    descriptors: &[Descriptor],
) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
    let mut cache = BTreeMap::new();
    for descriptor in descriptors {
        for example in descriptor.examples {
            let case_dir = repo_root.join("examples/specification").join(example.case);
            let manifest = SourceCaseManifest::read(&case_dir.join("case.toml"))?;
            let selected = manifest.selected_sources(&case_dir)?;
            let mut selected_relative = BTreeMap::new();
            for source in selected {
                let relative = source.strip_prefix(&case_dir).map_err(|_| {
                    format!(
                        "{}: selected example source is outside its specification case",
                        example.case
                    )
                })?;
                let relative = slash_path(relative);
                let source_text = fs::read_to_string(&source).map_err(|error| {
                    format!(
                        "{}: read selected example source: {error}",
                        source.display()
                    )
                })?;
                selected_relative.insert(relative, normalize_source_text(&source_text));
            }
            for file in example.files {
                if !selected_relative.contains_key(*file) {
                    return Err(format!(
                        "topic `{}` selects example file `{file}` that is not a source input of `{}`",
                        descriptor.id, example.case
                    ));
                }
            }
            cache.insert(example.case.to_string(), selected_relative);
        }
    }
    Ok(cache)
}

#[derive(Debug)]
struct SourceCaseManifest {
    command: Vec<String>,
    cwd: Option<PathBuf>,
}

impl SourceCaseManifest {
    fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "{}: read the toolchain case manifest before selecting example source: {error}",
                path.display()
            )
        })?;
        let mut section = String::new();
        let mut manifest = Self {
            command: Vec::new(),
            cwd: None,
        };
        for statement in manifest_syntax::parse_document(path, &text) {
            match statement {
                ManifestStatement::Section { name, .. } => section = name,
                ManifestStatement::Assignment { key, value, .. } => match (section.as_str(), key) {
                    ("", "command") => manifest.command = value.parse_string_array(path),
                    ("", "cwd") => manifest.cwd = Some(PathBuf::from(value.parse_string(path))),
                    _ => {}
                },
            }
        }
        if manifest.command.is_empty() {
            return Err(format!(
                "{}: selected language-reference example case has no command",
                path.display()
            ));
        }
        Ok(manifest)
    }

    fn selected_sources(&self, case_dir: &Path) -> Result<Vec<PathBuf>, String> {
        let command_root = self
            .cwd
            .as_deref()
            .map_or_else(|| case_dir.to_path_buf(), |cwd| case_dir.join(cwd));
        let package_root = veln_project::select_package_root(&command_root).map_err(|error| {
            format!(
                "{}: select package root before checking language-reference example source: {error}",
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
                "{}: discover selected specification source inputs before publishing examples: {error}",
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
