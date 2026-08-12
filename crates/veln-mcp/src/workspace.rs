use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Selection {
    generation: u64,
    roots: Vec<String>,
}

impl Selection {
    pub(crate) fn discover(base: &Path) -> io::Result<Self> {
        Ok(Self {
            generation: 0,
            roots: discover_roots(base)?,
        })
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn roots(&self) -> &[String] {
        &self.roots
    }

    pub(crate) fn refresh(&mut self, base: &Path) -> io::Result<()> {
        self.refresh_with(|| discover_roots(base))
    }

    pub(crate) fn refresh_with(
        &mut self,
        discover: impl FnOnce() -> io::Result<Vec<String>>,
    ) -> io::Result<()> {
        let roots = discover()?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| io::Error::other("workspace generation exhausted"))?;
        self.roots = roots;
        self.generation = generation;
        Ok(())
    }
}

fn discover_roots(base: &Path) -> io::Result<Vec<String>> {
    if is_regular_manifest(base)? {
        return Ok(vec![".".to_string()]);
    }

    let mut roots = Vec::new();
    discover_branches(base, base, &mut roots)?;
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        roots.push(".".to_string());
    }
    Ok(roots)
}

fn discover_branches(base: &Path, directory: &Path, roots: &mut Vec<String>) -> io::Result<()> {
    let mut children = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_name() == OsStr::new(".git") {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_dir() {
            children.push(entry.path());
        }
    }
    children.sort_by(|left, right| left.file_name().cmp(&right.file_name()));

    for child in children {
        if is_regular_manifest(&child)? {
            roots.push(relative_root(base, &child)?);
        } else {
            discover_branches(base, &child, roots)?;
        }
    }
    Ok(())
}

fn is_regular_manifest(directory: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(directory.join("veln.toml")) {
        Ok(metadata) => Ok(metadata.file_type().is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn relative_root(base: &Path, root: &Path) -> io::Result<String> {
    let relative = root.strip_prefix(base).map_err(io::Error::other)?;
    relative
        .components()
        .map(|component| {
            component.as_os_str().to_str().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "workspace project root is not representable as UTF-8",
                )
            })
        })
        .collect::<io::Result<Vec<_>>>()
        .map(|components| components.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn workspace_selection_table_is_deterministic_and_stops_at_manifests() {
        struct Case {
            name: &'static str,
            files: &'static [&'static str],
            expected: &'static [&'static str],
        }

        let cases = [
            Case {
                name: "base manifest",
                files: &["veln.toml", "nested/veln.toml"],
                expected: &["."],
            },
            Case {
                name: "first manifest on each branch",
                files: &[
                    "zeta/veln.toml",
                    "zeta/nested/veln.toml",
                    "alpha/deep/veln.toml",
                    "alpha/deep/nested/veln.toml",
                ],
                expected: &["alpha/deep", "zeta"],
            },
            Case {
                name: "anonymous base",
                files: &["src/main.veln"],
                expected: &["."],
            },
            Case {
                name: "manifest named directory",
                files: &["veln.toml/ignored", "nested/veln.toml/ignored"],
                expected: &["."],
            },
            Case {
                name: "git skipped and target ordinary",
                files: &[".git/hidden/veln.toml", "target/project/veln.toml"],
                expected: &["target/project"],
            },
        ];

        for case in cases {
            let project = TempWorkspace::new(case.name);
            for file in case.files {
                project.write(file, "");
            }
            let selection = Selection::discover(project.root()).unwrap();
            assert_eq!(selection.roots(), case.expected, "{}", case.name);
        }
    }

    #[cfg(unix)]
    #[test]
    fn implicit_discovery_does_not_follow_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let project = TempWorkspace::new("directory-symlink");
        project.write("outside/veln.toml", "");
        project.write("ordinary/veln.toml", "");
        symlink(project.path("outside"), project.path("linked")).unwrap();

        let selection = Selection::discover(project.root()).unwrap();
        assert_eq!(selection.roots(), ["ordinary", "outside"]);
    }

    #[test]
    fn refresh_replaces_selection_only_after_success() {
        let project = TempWorkspace::new("refresh");
        let mut selection = Selection::discover(project.root()).unwrap();
        assert_eq!(
            selection,
            Selection {
                generation: 0,
                roots: vec![".".into()]
            }
        );

        project.write("nested/veln.toml", "");
        assert_eq!(selection.roots(), ["."]);
        selection.refresh(project.root()).unwrap();
        assert_eq!(
            selection,
            Selection {
                generation: 1,
                roots: vec!["nested".into()]
            }
        );

        fs::remove_file(project.path("nested/veln.toml")).unwrap();
        project.write("renamed/veln.toml", "");
        assert_eq!(selection.roots(), ["nested"]);
        selection.refresh(project.root()).unwrap();
        assert_eq!(
            selection,
            Selection {
                generation: 2,
                roots: vec!["renamed".into()]
            }
        );
    }

    #[test]
    fn failed_refresh_preserves_roots_and_generation() {
        let project = TempWorkspace::new("failed-refresh");
        project.write("alpha/veln.toml", "");
        let mut selection = Selection::discover(project.root()).unwrap();
        let previous = selection.clone();

        let failure =
            selection.refresh_with(|| Err(io::Error::other("injected discovery failure")));
        assert_eq!(
            failure.unwrap_err().to_string(),
            "injected discovery failure"
        );
        assert_eq!(selection, previous);
    }

    #[test]
    fn exhausted_generation_preserves_roots_and_generation() {
        let mut selection = Selection {
            generation: u64::MAX,
            roots: vec!["alpha".into()],
        };
        let previous = selection.clone();

        let failure = selection.refresh_with(|| Ok(vec!["beta".into()]));

        assert_eq!(
            failure.unwrap_err().to_string(),
            "workspace generation exhausted"
        );
        assert_eq!(selection, previous);
    }

    #[cfg(unix)]
    #[test]
    fn manifest_symlinks_do_not_select_projects() {
        use std::os::unix::fs::symlink;

        let project = TempWorkspace::new("manifest-symlink");
        project.write("manifest", "");
        fs::create_dir_all(project.path("nested")).unwrap();
        symlink(project.path("manifest"), project.path("veln.toml")).unwrap();
        symlink(project.path("manifest"), project.path("nested/veln.toml")).unwrap();

        let selection = Selection::discover(project.root()).unwrap();
        assert_eq!(selection.roots(), ["."]);
    }

    #[cfg(unix)]
    #[test]
    fn unrepresentable_manifest_root_fails_discovery_instead_of_lossy_spelling() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let project = TempWorkspace::new("non-utf8-root");
        let root = project.root().join(OsString::from_vec(vec![b'p', 0xff]));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("veln.toml"), "").unwrap();

        let error = Selection::discover(project.root()).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            error.to_string(),
            "workspace project root is not representable as UTF-8"
        );
    }

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn new(name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root =
                env::temp_dir().join(format!("veln-mcp-{name}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            Self { root }
        }

        fn root(&self) -> &Path {
            &self.root
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.root.join(relative)
        }

        fn write(&self, relative: &str, contents: &str) {
            let path = self.path(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, contents).unwrap();
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
