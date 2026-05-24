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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    #[test]
    fn discovers_veln_files_recursively_and_skips_ignored_directories() {
        let temp = TempProject::new("recursive-discovery");
        temp.write("src/main.veln", "main");
        temp.write("src/nested/lib.veln", "lib");
        temp.write("src/readme.txt", "not source");
        temp.write("target/generated.veln", "ignored");
        temp.write(".git/hooks/hook.veln", "ignored");

        let paths = discover_source_paths(temp.root(), &[]).unwrap();

        assert_eq!(
            paths,
            vec![temp.path("src/main.veln"), temp.path("src/nested/lib.veln"),]
        );
    }

    #[test]
    fn discovers_veln_files_from_explicit_directories() {
        let temp = TempProject::new("directory-input");
        temp.write("src/main.veln", "main");
        temp.write("tests/case.veln", "case");
        temp.write("tests/case.txt", "ignored");

        let paths = discover_source_paths(temp.root(), &[PathBuf::from("tests")]).unwrap();

        assert_eq!(paths, vec![temp.path("tests/case.veln")]);
    }

    #[test]
    fn keeps_explicit_non_veln_files() {
        let temp = TempProject::new("explicit-non-veln");
        temp.write("notes.txt", "notes");

        let paths = discover_source_paths(temp.root(), &[PathBuf::from("notes.txt")]).unwrap();

        assert_eq!(paths, vec![temp.path("notes.txt")]);
    }

    #[test]
    fn project_discover_reads_sources_with_project_relative_paths() {
        let temp = TempProject::new("project-discover");
        temp.write("src/b.veln", "second");
        temp.write("src/a.veln", "first");

        let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();

        assert_eq!(project.root, temp.root().to_path_buf());
        let files = project
            .files
            .iter()
            .map(|file| (file.path().as_str().to_string(), file.text().to_string()))
            .collect::<Vec<_>>();
        assert_eq!(
            files,
            vec![
                ("src/a.veln".to_string(), "first".to_string()),
                ("src/b.veln".to_string(), "second".to_string()),
            ]
        );
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "veln-project-test-{name}-{}-{unique}",
                std::process::id()
            ));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn path(&self, path: &str) -> PathBuf {
            self.root.join(path)
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.path(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
