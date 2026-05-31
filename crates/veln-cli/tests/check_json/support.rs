use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

pub(super) struct TestProject {
    pub(super) root: PathBuf,
}

impl TestProject {
    pub(super) fn new(name: &str) -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-cli-check-json-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test project directory should be created");
        Self { root }
    }

    pub(super) fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, text).expect("fixture should be written");
    }

    pub(super) fn check_json(&self, args: &[&str]) -> Output {
        self.veln(&["check", "--json"], args)
    }

    pub(super) fn fmt(&self, args: &[&str]) -> Output {
        self.veln(&["fmt"], args)
    }

    pub(super) fn assert_fmt_idempotent(&self, args: &[&str], expected_files: &[(&str, &str)]) {
        let output = self.fmt(args);

        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), "");
        self.assert_files(expected_files);

        let second_output = self.fmt(args);

        assert!(second_output.status.success(), "{}", stderr(&second_output));
        assert_eq!(stdout(&second_output), "");
        self.assert_files(expected_files);
    }

    pub(super) fn assert_files(&self, expected_files: &[(&str, &str)]) {
        for (path, expected) in expected_files {
            assert_eq!(self.read(path), *expected);
        }
    }

    pub(super) fn run(&self, args: &[&str]) -> Output {
        self.veln(&["run"], args)
    }

    pub(super) fn test(&self, args: &[&str]) -> Output {
        self.veln(&["test"], args)
    }

    pub(super) fn repair(&self, args: &[&str]) -> Output {
        self.veln(&["repair"], args)
    }

    pub(super) fn run_with_path(&self, args: &[&str], path: &str) -> Output {
        self.veln_with_path("run", args, path)
    }

    pub(super) fn veln_with_path(&self, subcommand: &str, args: &[&str], path: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        command.env("PATH", path);
        command.arg(subcommand);
        for arg in args {
            command.arg(arg);
        }
        command.output().expect("veln should run")
    }

    pub(super) fn read(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).expect("fixture should be read")
    }

    pub(super) fn veln(&self, command_args: &[&str], args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        for arg in command_args {
            command.arg(arg);
        }
        for arg in args {
            command.arg(arg);
        }
        command.output().expect("veln should run")
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn repo_file(path: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under repository root")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

pub(super) fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8")
}

pub(super) fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr should be UTF-8")
}

pub(super) fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "missing substring `{needle}` in {haystack}"
        );
    }
}

pub(super) fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}
