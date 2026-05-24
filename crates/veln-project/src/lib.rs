//! Source discovery, module context, and import roots.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use veln_source::SourceFile;

#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub files: Vec<SourceFile>,
}

impl Project {
    pub fn discover(root: impl Into<PathBuf>, inputs: &[PathBuf]) -> io::Result<Self> {
        let root = root.into();
        let paths = discover_source_paths(&root, inputs)?;
        let mut files = Vec::new();
        for path in paths {
            files.push(SourceFile::read(&root, &path)?);
        }
        Ok(Self { root, files })
    }
}

pub fn discover_source_paths(root: &Path, inputs: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    if inputs.is_empty() {
        collect_veln_files(root, &mut paths)?;
    } else {
        for input in inputs {
            let path = if input.is_absolute() {
                input.clone()
            } else {
                root.join(input)
            };
            if path.is_dir() {
                collect_veln_files(&path, &mut paths)?;
            } else {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_veln_files(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name == ".git" || file_name == "target" {
            continue;
        }
        if path.is_dir() {
            collect_veln_files(&path, paths)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "veln")
        {
            paths.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_explicit_files_sorted_and_unique() {
        let root = PathBuf::from(".");
        let paths = discover_source_paths(
            &root,
            &[
                PathBuf::from("b.veln"),
                PathBuf::from("a.veln"),
                PathBuf::from("a.veln"),
            ],
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![PathBuf::from("./a.veln"), PathBuf::from("./b.veln")]
        );
    }
}
