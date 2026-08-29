use super::*;

pub(super) fn case_name(case_dir: &Path) -> String {
    case_dir
        .components()
        .rev()
        .take(2)
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("-")
}

pub(super) fn assert_no_metrics_baseline_temp_file(
    context: &CaseRunContext<'_>,
    project_root: &Path,
) {
    for entry in fs::read_dir(project_root).unwrap_or_else(|error| {
        panic!(
            "{}: failed to inspect project directory for temporary baseline files: {error}",
            context.label()
        )
    }) {
        let entry = entry.unwrap_or_else(|error| {
            panic!(
                "{}: failed to inspect project directory entry for temporary baseline files: {error}",
                context.label()
            )
        });
        let name = entry.file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.starts_with(".metrics.baseline.json.tmp-"),
            "{}: temporary metrics baseline file was left behind: {name}",
            context.label()
        );
    }
}

pub(super) fn assert_no_entries_with_prefix(root: &Path, prefix: &str) {
    for entry in fs::read_dir(root)
        .unwrap_or_else(|error| panic!("{}: failed to inspect directory: {error}", root.display()))
    {
        let entry = entry.expect("directory entry should be readable");
        let name = entry.file_name();
        let name = name.to_string_lossy();
        assert!(
            !name.starts_with(prefix),
            "{} should not contain entries beginning with `{prefix}`",
            root.display()
        );
    }
}

pub(super) fn assert_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

pub(super) struct TestProject {
    pub(super) root: PathBuf,
    pub(super) tool_path: Option<PathBuf>,
}

impl TestProject {
    pub(super) fn new(name: String, tools: &ToolSetup) -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-toolchain-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test project directory should be created");
        let tool_path = tools.needs_path().then(|| root.join(".veln-harness-tools"));
        Self { root, tool_path }
    }

    pub(super) fn copy_fixtures(&self, case_dir: &Path) {
        copy_fixture_dir(case_dir, case_dir, &self.root);
    }

    pub(super) fn source_diagnostic_artifact_path(&self, run_index: usize) -> PathBuf {
        self.root
            .join(format!(".veln-source-diagnostics-{}.json", run_index + 1))
    }

    pub(super) fn veln_with_artifact(
        &self,
        args: &[String],
        cwd: Option<&Path>,
        env: &[(String, String)],
        stdin: Option<&str>,
        artifact_path: Option<&Path>,
    ) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(cwd.map_or_else(|| self.root.clone(), |cwd| self.root.join(cwd)));
        command.args(args);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.env("VELN_CACHE_DIR", self.root.join(".veln-harness-cache"));
        if let Some(path) = &self.tool_path {
            command.env("PATH", path);
        }
        for (name, value) in env {
            command.env(name, value);
        }
        if let Some(path) = artifact_path {
            command.env(SOURCE_DIAGNOSTIC_ARTIFACT_ENV, path);
        }
        if stdin.is_some() {
            command.stdin(Stdio::piped());
        }
        let mut child = command.spawn().expect("veln should spawn");
        if let Some(input) = stdin {
            let mut child_stdin = child.stdin.take().expect("veln stdin should be piped");
            child_stdin
                .write_all(input.as_bytes())
                .expect("veln stdin should be written");
        }
        child.wait_with_output().expect("veln should run")
    }

    pub(super) fn setup_tools(&self, tools: &ToolSetup) {
        let Some(tool_path) = &self.tool_path else {
            return;
        };
        fs::create_dir_all(tool_path).expect("tool directory should be created");

        for tool in tools.configured() {
            tool.setup(tool_path);
        }
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn copy_fixture_dir(base: &Path, dir: &Path, target_root: &Path) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: failed to read fixtures: {error}", dir.display()));
    for entry in entries {
        let entry = entry.unwrap_or_else(|error| {
            panic!("{}: failed to read fixture entry: {error}", dir.display())
        });
        let source = entry.path();
        let relative = source
            .strip_prefix(base)
            .expect("fixture should be under case directory");
        if relative == Path::new("case.toml") {
            continue;
        }

        let target = target_root.join(relative);
        let metadata = fs::symlink_metadata(&source).unwrap_or_else(|error| {
            panic!(
                "{}: failed to inspect fixture entry: {error}",
                source.display()
            )
        });
        if is_link_like_metadata(&metadata) {
            panic!(
                "{}: replace the link or reparse point with a regular fixture entry before command execution",
                source.display()
            );
        }
        if metadata.is_dir() {
            fs::create_dir_all(&target).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to create fixture directory: {error}",
                    target.display()
                )
            });
            copy_fixture_dir(base, &source, target_root);
        } else if metadata.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).unwrap_or_else(|error| {
                    panic!(
                        "{}: failed to create fixture parent: {error}",
                        parent.display()
                    )
                });
            }
            fs::copy(&source, &target).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to copy fixture to {}: {error}",
                    source.display(),
                    target.display()
                )
            });
        } else {
            panic!(
                "{}: replace the non-regular fixture entry with a regular file or directory before command execution",
                source.display()
            );
        }
    }
}

pub(super) fn setup_tool(tool_path: &Path, name: &str, availability: ToolAvailability) {
    match availability {
        ToolAvailability::Missing => {}
        ToolAvailability::FakeSuccess => write_fake_success_tool(tool_path, name),
        ToolAvailability::FakeGitRevParse => write_fake_git_rev_parse_tool(tool_path, name),
        ToolAvailability::Real => {
            let host_tool = find_host_tool(name)
                .unwrap_or_else(|| panic!("host tool `{name}` should be available"));
            install_real_tool(tool_path, name, &host_tool);
        }
    }
}

pub(super) const FAKE_GIT_RESOLVED_REV: &str = "0123456789abcdef0123456789abcdef01234567";

#[cfg(unix)]
pub(super) fn write_fake_success_tool(tool_path: &Path, name: &str) {
    let tool = tool_path.join(name);
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("fake tool should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake tool metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake tool should be executable");
}

#[cfg(windows)]
pub(super) fn write_fake_success_tool(tool_path: &Path, name: &str) {
    for extension in ["bat", "cmd"] {
        let tool = tool_path.join(format!("{name}.{extension}"));
        fs::write(&tool, "@echo off\r\nexit /b 0\r\n").expect("fake tool should be written");
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn write_fake_success_tool(_tool_path: &Path, name: &str) {
    panic!("fake tool `{name}` is not supported on this platform");
}

#[cfg(unix)]
pub(super) fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
    assert_eq!(name, "git", "fake git rev-parse is only valid for git");
    let tool = tool_path.join(name);
    fs::write(
        &tool,
        format!(
            "#!/bin/sh\nset -eu\nif [ \"$1\" = \"clone\" ]; then\n  shift\n  if [ \"$1\" = \"--no-checkout\" ]; then\n    shift\n  fi\n  url=\"$1\"\n  dest=\"$2\"\n  name=\"${{url##*/}}\"\n  name=\"${{name%.git}}\"\n  remote=\"$PWD/.fake-git-remotes/$name\"\n  command -p mkdir -p \"$dest\"\n  command -p cp -R \"$remote/.\" \"$dest/\"\n  exit 0\nfi\nif [ \"$1\" = \"-C\" ]; then\n  shift 2\n  if [ \"$1\" = \"fetch\" ] || [ \"$1\" = \"checkout\" ] || [ \"$1\" = \"clean\" ]; then\n    exit 0\n  fi\n  if [ \"$1\" = \"rev-parse\" ] && [ \"$2\" = \"--verify\" ]; then\n    echo \"{FAKE_GIT_RESOLVED_REV}\"\n    exit 0\n  fi\nfi\nexit 1\n"
        ),
    )
    .expect("fake git should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake git metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake git should be executable");
}

#[cfg(windows)]
pub(super) fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
    assert_eq!(name, "git", "fake git rev-parse is only valid for git");
    for extension in ["bat", "cmd"] {
        let tool = tool_path.join(format!("{name}.{extension}"));
        fs::write(
            &tool,
            format!(
                "@echo off\r\nif \"%1\"==\"-C\" if \"%3\"==\"rev-parse\" if \"%4\"==\"--verify\" (\r\n  echo {FAKE_GIT_RESOLVED_REV}\r\n  exit /b 0\r\n)\r\nexit /b 1\r\n"
            ),
        )
        .expect("fake git should be written");
    }
}

#[cfg(not(any(unix, windows)))]
pub(super) fn write_fake_git_rev_parse_tool(_tool_path: &Path, name: &str) {
    panic!("fake git `{name}` is not supported on this platform");
}

pub(super) fn find_host_tool(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        for candidate_name in host_tool_names(name) {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

#[cfg(windows)]
pub(super) fn host_tool_names(name: &str) -> Vec<String> {
    vec![
        format!("{name}.exe"),
        format!("{name}.cmd"),
        format!("{name}.bat"),
        name.to_string(),
    ]
}

#[cfg(not(windows))]
pub(super) fn host_tool_names(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(unix)]
pub(super) fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    std::os::unix::fs::symlink(host_tool, tool_path.join(name))
        .expect("real tool symlink should be created");
}

#[cfg(windows)]
pub(super) fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    let extension = host_tool
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("exe");
    fs::copy(host_tool, tool_path.join(format!("{name}.{extension}")))
        .expect("real tool should be copied");
}

#[cfg(not(any(unix, windows)))]
pub(super) fn install_real_tool(_tool_path: &Path, name: &str, _host_tool: &Path) {
    panic!("real tool `{name}` is not supported on this platform");
}
