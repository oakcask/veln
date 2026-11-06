use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use veln_project::Project;
use veln_repo_toolchain_case::CaseManifest;

use crate::{Descriptor, normalize_source_text, slash_path};

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
            let manifest_path = case_dir.join("case.toml");
            let text = fs::read_to_string(&manifest_path).map_err(|error| {
                format!(
                    "{}: read the toolchain case manifest before selecting example source: {error}",
                    manifest_path.display()
                )
            })?;
            let manifest = CaseManifest::parse_for_source_selection(&manifest_path, &text)?;
            if !manifest.has_command() {
                return Err(format!(
                    "{}: selected language-reference example case has no command",
                    manifest_path.display()
                ));
            }
            let selected = selected_sources(&manifest, &case_dir)?;
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

fn selected_sources(manifest: &CaseManifest, case_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let command_root = manifest.command_root(case_dir);
    let package_root = veln_project::select_package_root(&command_root).map_err(|error| {
        format!(
            "{}: select package root before checking language-reference example source: {error}",
            case_dir.display()
        )
    })?;
    let inputs = manifest.selected_source_inputs(case_dir, &package_root);
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
