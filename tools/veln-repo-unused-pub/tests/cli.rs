use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "veln-unused-pub-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, file: &str, text: &str) {
        let path = self.0.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_veln-repo-unused-pub"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn findings_are_advisory_and_json_is_deterministic_with_relative_locations() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs", "pub fn candidate() {}\n");
    fixture.write("src/target/broken.rs", "not rust");
    let args = ["--format", "json", "src", "src/lib.rs"];
    let result = fixture.run(&args);
    assert!(result.status.success());
    assert_eq!(result.stdout, fixture.run(&args).stdout);
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["status"], "advisory");
    assert_eq!(report["files"], 1);
    assert_eq!(report["findings"][0]["file"], "src/lib.rs");
    assert_eq!(report["findings"][0]["line"], 1);
    assert_eq!(report["findings"][0]["evidence"], "no-references");
    let human = fixture.run(&["src"]);
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("src/lib.rs:1: fn candidate [no-references]"));
    assert!(text.contains("before removing code"));
}

#[test]
fn invalid_input_fails_and_does_not_emit_a_success_report() {
    let fixture = Fixture::new();
    fixture.write("broken.rs", "pub fn {");
    for args in [
        vec!["--format", "yaml"],
        vec!["missing"],
        vec!["broken.rs"],
        vec!["."],
    ] {
        let result = fixture.run(&args);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
    }
    let empty = Fixture::new();
    assert_eq!(empty.run(&["."]).status.code(), Some(2));
}

#[test]
fn test_consumers_are_information_not_dead_code_in_human_and_json_reports() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs", "pub fn helper() {}\n");
    fixture.write("tests/contract.rs", "#[test] fn contract() { helper(); }\n");
    let human = fixture.run(&["."]);
    assert!(human.status.success());
    let text = String::from_utf8(human.stdout).unwrap();
    assert!(text.contains("Suspected dead public items: 0"));
    assert!(text.contains("informational, not dead-code candidates): 1"));
    assert!(text.contains("name-matched test reference: ./tests/contract.rs:1"));
    let json = fixture.run(&["--format", "json", "."]);
    assert!(json.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(
        report["findings"][0]["references"][0]["file"],
        "./tests/contract.rs"
    );
    assert_eq!(report["findings"][0]["references"][0]["line"], 1);
    assert_eq!(report["findings"][0]["references_truncated"], false);
}

#[cfg(unix)]
#[test]
fn directory_symlinks_do_not_duplicate_sources_or_recurse() {
    let fixture = Fixture::new();
    fixture.write("src/lib.rs", "pub fn orphan() {}");
    std::os::unix::fs::symlink(".", fixture.0.join("src/loop")).unwrap();
    let result = fixture.run(&["--format", "json", "src"]);
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["files"], 1);
}
