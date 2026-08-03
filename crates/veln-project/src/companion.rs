use std::path::{Path, PathBuf};

use crate::discover_source_paths;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompanionSourceKind {
    Test,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompanionSource {
    pub kind: CompanionSourceKind,
    pub companion_path: String,
    pub target_path: String,
    pub chained: bool,
}

pub fn classify_companion_source(path: &str) -> Option<CompanionSource> {
    let target_path = path.strip_suffix(".test.veln")?;
    let target_path = format!("{target_path}.veln");
    let chained = classify_companion_source(&target_path).is_some();
    Some(CompanionSource {
        kind: CompanionSourceKind::Test,
        companion_path: path.to_string(),
        target_path,
        chained,
    })
}

pub fn is_companion_source_path(path: &str) -> bool {
    classify_companion_source(path).is_some()
}

pub fn companion_analysis_inputs(root: &Path, inputs: &[PathBuf]) -> std::io::Result<Vec<PathBuf>> {
    let mut paths = discover_source_paths(root, inputs)?;
    for path in paths.clone() {
        let relative = project_relative_path(root, &path);
        let Some(companion) = classify_companion_source(&relative) else {
            continue;
        };
        let target = root.join(companion.target_path);
        if target.is_file() {
            paths.push(target);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub fn production_analysis_inputs(
    root: &Path,
    inputs: &[PathBuf],
) -> std::io::Result<Vec<PathBuf>> {
    let paths = discover_source_paths(root, inputs)?;
    Ok(paths
        .into_iter()
        .filter(|path| !classify_companion_source(&project_relative_path(root, path)).is_some())
        .collect())
}

pub fn explicit_companion_inputs(root: &Path, inputs: &[PathBuf]) -> Vec<String> {
    inputs
        .iter()
        .filter(|input| !input_is_directory(root, input))
        .filter_map(|input| {
            let path = if input.is_absolute() {
                input.clone()
            } else {
                root.join(input)
            };
            let relative = project_relative_path(root, &path);
            classify_companion_source(&relative).map(|companion| companion.companion_path)
        })
        .collect()
}

fn input_is_directory(root: &Path, input: &Path) -> bool {
    let path = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    path.is_dir()
}

fn project_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), PathBuf::from)
        .to_string_lossy()
        .replace('\\', "/")
}
