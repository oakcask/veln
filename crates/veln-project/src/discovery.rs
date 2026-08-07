use std::env;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

pub fn discover_source_paths(root: &Path, inputs: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let root_identity = absolute_lexical_path(root)?;
    let root_output = normalize_lexical_path(root);
    let mut paths = Vec::new();
    if inputs.is_empty() {
        collect_veln_files(&root_output, &mut paths)?;
    } else {
        for input in inputs {
            let joined = if input.is_absolute() {
                input.clone()
            } else {
                root.join(input)
            };
            let input_identity = absolute_lexical_path(&joined)?;
            let relative = input_identity
                .strip_prefix(&root_identity)
                .map_err(|_| rejected_input_error(input, "is outside the supplied package root"))?;
            let path = explicit_output_path(&root_output, input, relative);
            match classify_explicit_input(&root_identity, relative, input, &joined)? {
                ExplicitInputKind::Directory => collect_veln_files(&path, &mut paths)?,
                ExplicitInputKind::FileOrMissing => paths.push(path),
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

enum ExplicitInputKind {
    Directory,
    FileOrMissing,
}

fn classify_explicit_input(
    root: &Path,
    relative: &Path,
    input: &Path,
    joined: &Path,
) -> io::Result<ExplicitInputKind> {
    if relative.as_os_str().is_empty() {
        return Ok(if fs::metadata(root)?.is_dir() {
            ExplicitInputKind::Directory
        } else {
            ExplicitInputKind::FileOrMissing
        });
    }
    validate_explicit_components(root, input, joined)?;
    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        let file_type = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata.file_type(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(ExplicitInputKind::FileOrMissing);
            }
            Err(error) => return Err(error),
        };
        if file_type.is_symlink() {
            return Err(rejected_input_error(
                input,
                &format!("traverses the symbolic link `{}`", current.display()),
            ));
        }
        if file_type.is_dir() {
            if has_manifest_boundary(&current)? {
                return Err(rejected_input_error(
                    input,
                    &format!(
                        "is owned by the nested package rooted at `{}`",
                        current.display()
                    ),
                ));
            }
            if index + 1 == components.len() {
                return Ok(ExplicitInputKind::Directory);
            }
        } else {
            return Ok(ExplicitInputKind::FileOrMissing);
        }
    }

    Ok(ExplicitInputKind::FileOrMissing)
}

fn validate_explicit_components(root: &Path, input: &Path, joined: &Path) -> io::Result<()> {
    let relative = if input.is_absolute() {
        joined.strip_prefix(root).unwrap_or(input)
    } else {
        input
    };
    let mut current = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if current == root {
                    return Err(rejected_input_error(
                        input,
                        "escapes the supplied package root through `..`",
                    ));
                }
                current.pop();
            }
            Component::Normal(part) => {
                current.push(part);
                let file_type = match fs::symlink_metadata(&current) {
                    Ok(metadata) => metadata.file_type(),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error),
                };
                if file_type.is_symlink() {
                    return Err(rejected_input_error(
                        input,
                        &format!("traverses the symbolic link `{}`", current.display()),
                    ));
                }
            }
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    Ok(())
}

fn collect_veln_files(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_name() == ".git" {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !has_manifest_boundary(&path)? {
                collect_veln_files(&path, paths)?;
            }
        } else if file_type.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension == "veln")
        {
            paths.push(path);
        }
    }
    Ok(())
}

fn has_manifest_boundary(dir: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(dir.join("veln.toml")) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn rejected_input_error(input: &Path, reason: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("source input `{}` {reason}", input.display()),
    )
}

fn explicit_output_path(root: &Path, input: &Path, relative: &Path) -> PathBuf {
    if input.is_absolute() {
        normalize_lexical_path(input)
    } else {
        normalize_lexical_path(&root.join(relative))
    }
}

fn absolute_lexical_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(normalize_lexical_path(path))
    } else {
        Ok(normalize_lexical_path(&env::current_dir()?.join(path)))
    }
}

pub(crate) fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match normalized.components().next_back() {
                Some(Component::Normal(_)) => {
                    normalized.pop();
                }
                Some(Component::ParentDir) | None if !path.is_absolute() => {
                    normalized.push(component.as_os_str());
                }
                _ => {}
            },
            _ => normalized.push(component.as_os_str()),
        }
    }
    if normalized.as_os_str().is_empty() {
        normalized.push(".");
    }
    normalized
}
