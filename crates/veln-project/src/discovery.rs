use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
