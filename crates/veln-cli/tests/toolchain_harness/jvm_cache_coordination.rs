use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::{TestProject, ToolSetup, assert_success, cache_test_command, write_cache_test_java};

struct AbandonedCoordinationFixture {
    project: TestProject,
    tool_dir: PathBuf,
    cache_root: PathBuf,
    coordination_marker: PathBuf,
    java_marker: PathBuf,
    published_entry: PathBuf,
    published_snapshot: Vec<(PathBuf, Vec<u8>)>,
}

impl AbandonedCoordinationFixture {
    fn new() -> Self {
        let project = TestProject::new(
            "abandoned-jvm-cache-coordination".to_string(),
            &ToolSetup::default(),
        );
        fs::write(
            project.root.join("main.veln"),
            "fn main() -> ()\n  ()\nend\n",
        )
        .expect("source should be written");
        let tool_dir = project.root.join("tools");
        write_cache_test_java(&tool_dir);
        let cache_root = project.root.join("cache-root");
        let coordination_marker = project.root.join("cache-lock-ready");
        let java_marker = project.root.join("java-started");

        let warm = cache_test_command(
            &project.root,
            &["run", "main", "main.veln"],
            &tool_dir,
            &[
                ("VELN_CACHE_DIR", &cache_root),
                ("JAVA_MARKER", &java_marker),
            ],
        );
        assert_success("initial cache publication", &warm);
        fs::remove_file(&java_marker).expect("initial Java marker should be removed");
        let published_entry = find_published_entry(&cache_root);
        let published_snapshot = directory_file_snapshot(&published_entry);

        Self {
            project,
            tool_dir,
            cache_root,
            coordination_marker,
            java_marker,
            published_entry,
            published_snapshot,
        }
    }

    fn abandon_writer_after_coordination(&self) {
        let mut writer = self.cache_command();
        writer.env(
            "VELN_INTERNAL_TEST_CACHE_LOCK_READY",
            &self.coordination_marker,
        );
        let mut writer = writer.spawn().expect("cache writer should start");
        wait_for_coordination(&mut writer, &self.coordination_marker);
        writer.kill().expect("cache writer should be stopped");
        writer
            .wait()
            .expect("stopped cache writer should be reaped");
    }

    fn run_waiter(&self) -> Output {
        let mut waiter = self.cache_command();
        waiter.env("VELN_INTERNAL_TEST_CACHE_LOCK_WAIT_MS", "2000");
        waiter.stdout(Stdio::piped());
        waiter.stderr(Stdio::piped());
        wait_for_bounded_output(
            waiter.spawn().expect("later cache command should start"),
            Instant::now() + Duration::from_secs(10),
            "later cache command",
        )
    }

    fn assert_waiter_preserved_published_entry(&self, output: &Output) {
        assert_eq!(output.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("timed out waiting for JVM cache coordination")
        );
        assert!(
            !self.java_marker.exists(),
            "Java must not start after timeout"
        );
        assert_eq!(
            directory_file_snapshot(&self.published_entry),
            self.published_snapshot,
            "abandoned coordination must not alter a complete published entry"
        );
    }

    fn cache_command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.project.root);
        command.args(["run", "main", "main.veln"]);
        command.env("PATH", &self.tool_dir);
        command.env("VELN_CACHE_DIR", &self.cache_root);
        command.env("JAVA_MARKER", &self.java_marker);
        command
    }
}

#[test]
fn abandoned_jvm_cache_coordination_reaches_bounded_error_without_starting_java() {
    let fixture = AbandonedCoordinationFixture::new();

    fixture.abandon_writer_after_coordination();
    let output = fixture.run_waiter();

    fixture.assert_waiter_preserved_published_entry(&output);
}

fn find_published_entry(cache_root: &Path) -> PathBuf {
    fs::read_dir(cache_root.join("jvm"))
        .expect("JVM cache root should be readable")
        .map(|entry| entry.expect("cache entry should be readable").path())
        .find(|path| path.join(".veln-cache-ok").is_file())
        .expect("initial command should publish a complete entry")
}

fn wait_for_coordination(writer: &mut Child, marker: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !marker.is_file() {
        if let Some(status) = writer.try_wait().expect("writer status should be readable") {
            panic!("cache writer exited before reaching coordination: {status}");
        }
        assert!(
            Instant::now() < deadline,
            "cache writer should reach coordination within the harness bound"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_for_bounded_output(mut child: Child, deadline: Instant, label: &str) -> Output {
    loop {
        if child
            .try_wait()
            .expect("child status should be readable")
            .is_some()
        {
            return child
                .wait_with_output()
                .expect("child output should be read");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child
                .wait_with_output()
                .expect("timed-out child output should be read");
            panic!(
                "{label} exceeded the harness bound\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn directory_file_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn collect(root: &Path, directory: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(directory).expect("snapshot directory should be readable") {
            let entry = entry.expect("snapshot entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, files);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("snapshot path should be below its root")
                    .to_path_buf();
                files.push((
                    relative,
                    fs::read(path).expect("snapshot file should be readable"),
                ));
            }
        }
    }

    let mut files = Vec::new();
    collect(root, root, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}
