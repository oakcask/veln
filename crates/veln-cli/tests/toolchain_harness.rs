use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
#[cfg(all(unix, debug_assertions))]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use veln_analysis::{
    DoctestMode, checked_project_diagnostics, derive_source_module_path, load_surface_module,
};
use veln_ast::{FunctionKind, PublicAliasKind, SurfaceModule, UseDecl, Visibility};
use veln_diagnostics::{Diagnostic, Severity};
use veln_project::Project;

#[path = "toolchain_harness/manifest_syntax.rs"]
mod manifest_syntax;
#[path = "../toolchain_case_inventory.rs"]
mod toolchain_case_inventory;

use manifest_syntax::{Statement as ManifestStatement, Value as ManifestValue};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);
const SOURCE_DIAGNOSTIC_ARTIFACT_ENV: &str = "VELN_HARNESS_SOURCE_DIAGNOSTICS";

thread_local! {
    static TEST_GENERATED_TOOLCHAIN_CASES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

include!(concat!(env!("OUT_DIR"), "/toolchain_cases.rs"));

fn toolchain_case_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn run_case(case_dir: &Path) {
    run_case_with_after_invocation(case_dir, |_, _| {});
}

fn run_case_with_after_invocation(
    case_dir: &Path,
    mut after_invocation: impl FnMut(&CaseRunContext<'_>, &Path),
) {
    run_case_with_guard_and_after_invocation(
        case_dir,
        guard_generated_or_synthetic_case,
        &mut after_invocation,
    );
}

fn run_case_with_guard_and_after_invocation(
    case_dir: &Path,
    guard_case: impl FnOnce(&Path),
    mut after_invocation: impl FnMut(&CaseRunContext<'_>, &Path),
) {
    guard_case(case_dir);
    let manifest = CaseManifest::read(&case_dir.join("case.toml"));
    if let Some(reason) = manifest.skip_reason() {
        eprintln!("skipping {}: {reason}", case_dir.display());
        return;
    }

    let project = TestProject::new(case_name(case_dir), &manifest.tools);
    project.copy_fixtures(case_dir);
    project.setup_tools(&manifest.tools);
    if let Some(expected_error) = &manifest.manifest_error {
        let panic = std::panic::catch_unwind(|| {
            manifest.validate_fixture_schema_references(&project.root);
        })
        .expect_err("case should fail manifest validation");
        let message = panic_message(panic);
        expected_error.assert_matches(case_dir, &message);
        return;
    }

    if manifest.needs_pre_command_source_error_guard(case_dir) {
        manifest.assert_no_unexpected_example_source_errors(case_dir, &project.root);
    }
    manifest.validate_fixture_schema_references(&project.root);

    let mut run_failures = Vec::new();
    for run_index in 0..manifest.invocation.repeat {
        let context = CaseRunContext {
            case_dir,
            run_number: run_index + 1,
        };
        let artifact_path = manifest
            .needs_command_source_error_guard(case_dir)
            .then(|| project.source_diagnostic_artifact_path(run_index));
        let stdin = manifest.invocation.materialized_stdin(&project.root);
        let output = CapturedOutput::read(
            &context,
            project.veln_with_artifact(
                &manifest.invocation.command,
                manifest.invocation.cwd.as_deref(),
                &manifest.invocation.env,
                stdin.as_deref(),
                artifact_path.as_deref(),
            ),
        );
        collect_run_failure(&mut run_failures, || {
            if let Some(artifact_path) = artifact_path.as_deref() {
                let evidence = CommandSourceDiagnosticEvidence::read(&context, artifact_path);
                manifest.assert_no_unexpected_command_source_errors(&context, &evidence);
            }
            manifest
                .expectations
                .assert_matches(&context, &output, &project.root);
            manifest
                .expectations
                .assert_files_match(&context, &project.root);
            assert_no_metrics_baseline_temp_file(&context, &project.root);
            after_invocation(&context, &project.root);
        });
    }
    if !run_failures.is_empty() {
        panic!("toolchain case failures:\n{}", run_failures.join("\n"));
    }
}

fn collect_run_failure(failures: &mut Vec<String>, action: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        failures.push(panic_message(panic));
    }
}

fn guard_generated_or_synthetic_case(case_dir: &Path) {
    if is_generated_inventory_member(case_dir) {
        runtime_generated_inventory_barrier();
    } else {
        let manifest = case_dir.join("case.toml");
        let text = fs::read_to_string(&manifest).unwrap_or_else(|error| {
            panic!("{}: failed to read manifest: {error}", manifest.display())
        });
        let findings = manifest_syntax::manifest_policy_findings(&manifest, &text);
        if !findings.is_empty() {
            let mut message = format!(
                "{}: synthetic toolchain case violates manifest line-break policy before loading resources",
                manifest.display()
            );
            for finding in findings {
                message.push_str(&format!(
                    "\n- line {} field `{}` contains {} `{}`; use physical multiline text or a sidecar so line structure remains reviewable",
                    finding.line,
                    finding.field,
                    finding.category,
                    finding.spelling.escape_debug()
                ));
            }
            panic!("{message}");
        }
    }
}

fn is_generated_inventory_member(case_dir: &Path) -> bool {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(relative) = case_dir.strip_prefix(manifest_dir) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    generated_toolchain_cases_contains(&relative)
}

fn runtime_generated_inventory_barrier() {
    if let Some(error) = TEST_GENERATED_TOOLCHAIN_CASES.with(|cases| {
        let borrowed = cases.borrow();
        let generated = borrowed.as_ref()?;
        let generated = generated.iter().map(String::as_str).collect::<Vec<_>>();
        toolchain_case_inventory::compare_generated_inventory(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &generated,
        )
        .err()
    }) {
        panic!("{error}");
    }

    static BARRIER: RuntimeInventoryBarrier = RuntimeInventoryBarrier::new();
    BARRIER.check_with(|| {
        toolchain_case_inventory::compare_generated_inventory(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            GENERATED_TOOLCHAIN_CASES,
        )
        .map(|_| ())
    });
}

fn generated_toolchain_cases_contains(relative: &str) -> bool {
    TEST_GENERATED_TOOLCHAIN_CASES.with(|cases| {
        cases.borrow().as_ref().map_or_else(
            || GENERATED_TOOLCHAIN_CASES.contains(&relative),
            |generated| generated.iter().any(|case| case == relative),
        )
    })
}

fn with_test_generated_toolchain_cases(generated: Vec<String>, test: impl FnOnce()) {
    struct Reset;

    impl Drop for Reset {
        fn drop(&mut self) {
            TEST_GENERATED_TOOLCHAIN_CASES.with(|cases| {
                *cases.borrow_mut() = None;
            });
        }
    }

    TEST_GENERATED_TOOLCHAIN_CASES.with(|cases| {
        assert!(
            cases.borrow().is_none(),
            "test generated inventory override should not be nested"
        );
        *cases.borrow_mut() = Some(generated);
    });
    let _reset = Reset;
    test();
}

struct RuntimeInventoryBarrier {
    result: OnceLock<Result<(), String>>,
}

impl RuntimeInventoryBarrier {
    const fn new() -> Self {
        Self {
            result: OnceLock::new(),
        }
    }

    fn check_with(&self, scan: impl FnOnce() -> Result<(), String>) {
        match self.result.get_or_init(scan) {
            Ok(()) => {}
            Err(message) => panic!("{message}"),
        }
    }
}

#[cfg(unix)]
fn write_cache_test_java(tool_dir: &Path) {
    fs::create_dir_all(tool_dir).expect("tool directory should be created");
    let java = tool_dir.join("java");
    fs::write(
        &java,
        "#!/bin/sh\nif [ -n \"${JAVA_MARKER:-}\" ]; then printf started > \"$JAVA_MARKER\"; fi\nexit 0\n",
    )
    .expect("fake java should be written");
    let mut permissions = fs::metadata(&java)
        .expect("fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(java, permissions).expect("fake java should be executable");
}

#[cfg(unix)]
fn write_unusable_cache_test_java(tool_dir: &Path) {
    fs::create_dir_all(tool_dir).expect("tool directory should be created");
    let java = tool_dir.join("java");
    fs::write(&java, "#!/bin/sh\nexit 7\n").expect("fake java should be written");
    let mut permissions = fs::metadata(&java)
        .expect("fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(java, permissions).expect("fake java should be unusable");
}

#[cfg(unix)]
fn write_other_execute_only_cache_test_java(tool_dir: &Path) {
    fs::create_dir_all(tool_dir).expect("tool directory should be created");
    let java = tool_dir.join("java");
    fs::write(&java, "#!/bin/sh\nexit 7\n").expect("fake java should be written");
    let mut permissions = fs::metadata(&java)
        .expect("fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o001);
    fs::set_permissions(java, permissions).expect("fake java mode should be set");
}

#[cfg(unix)]
fn process_runs_as_root() -> bool {
    Command::new("/bin/sh")
        .arg("-c")
        .arg("test \"$(id -u)\" = 0")
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
fn cache_test_command(
    project_root: &Path,
    args: &[&str],
    tool_dir: &Path,
    environment: &[(&str, &Path)],
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
    command.current_dir(project_root);
    command.args(args);
    command.env("PATH", tool_dir);
    for name in [
        "VELN_CACHE_DIR",
        "XDG_CACHE_HOME",
        "HOME",
        "LOCALAPPDATA",
        "JAVA_MARKER",
        "VELN_INTERNAL_TEST_CACHE_LOCK_READY",
        "VELN_INTERNAL_TEST_CACHE_LOCK_WAIT_MS",
    ] {
        command.env_remove(name);
    }
    for (name, value) in environment {
        command.env(name, value);
    }
    command.output().expect("veln should run")
}

#[cfg(all(unix, debug_assertions))]
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

#[cfg(all(unix, debug_assertions))]
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

#[cfg(all(unix, debug_assertions))]
#[test]
fn abandoned_jvm_cache_coordination_reaches_bounded_error_without_starting_java() {
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
    let jvm_cache = cache_root.join("jvm");
    let published_entry = fs::read_dir(&jvm_cache)
        .expect("JVM cache root should be readable")
        .map(|entry| entry.expect("cache entry should be readable").path())
        .find(|path| path.join(".veln-cache-ok").is_file())
        .expect("initial command should publish a complete entry");
    let published_snapshot = directory_file_snapshot(&published_entry);

    let mut writer = Command::new(env!("CARGO_BIN_EXE_veln"));
    writer.current_dir(&project.root);
    writer.args(["run", "main", "main.veln"]);
    writer.env("PATH", &tool_dir);
    writer.env("VELN_CACHE_DIR", &cache_root);
    writer.env("VELN_INTERNAL_TEST_CACHE_LOCK_READY", &coordination_marker);
    writer.env("JAVA_MARKER", &java_marker);
    let mut writer = writer.spawn().expect("cache writer should start");

    let marker_deadline = Instant::now() + Duration::from_secs(5);
    while !coordination_marker.is_file() {
        if let Some(status) = writer.try_wait().expect("writer status should be readable") {
            panic!("cache writer exited before reaching coordination: {status}");
        }
        assert!(
            Instant::now() < marker_deadline,
            "cache writer should reach coordination within the harness bound"
        );
        thread::sleep(Duration::from_millis(10));
    }
    writer.kill().expect("cache writer should be stopped");
    writer
        .wait()
        .expect("stopped cache writer should be reaped");

    let later_deadline = Instant::now() + Duration::from_secs(10);
    let mut later = Command::new(env!("CARGO_BIN_EXE_veln"));
    later.current_dir(&project.root);
    later.args(["run", "main", "main.veln"]);
    later.env("PATH", &tool_dir);
    later.env("VELN_CACHE_DIR", &cache_root);
    later.env("VELN_INTERNAL_TEST_CACHE_LOCK_WAIT_MS", "2000");
    later.env("JAVA_MARKER", &java_marker);
    later.stdout(Stdio::piped());
    later.stderr(Stdio::piped());
    let output = wait_for_bounded_output(
        later.spawn().expect("later cache command should start"),
        later_deadline,
        "later cache command",
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("timed out waiting for JVM cache coordination")
    );
    assert!(!java_marker.exists(), "Java must not start after timeout");
    assert_eq!(
        directory_file_snapshot(&published_entry),
        published_snapshot,
        "abandoned coordination must not alter a complete published entry"
    );
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn run_uses_isolated_host_cache_without_local_fallback() {
    let project = TestProject::new("run-host-cache".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let xdg = test_temp_root("run-host-cache-xdg");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("XDG_CACHE_HOME", &xdg)],
    );

    assert_success("default host cache run", &output);
    assert!(xdg.join("veln/jvm").is_dir());
    assert!(!project.root.join("target/veln-cache").exists());
    assert!(!project.root.join("veln/jvm").exists());
    fs::remove_dir_all(xdg).expect("isolated XDG root should be removed");
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn equivalent_working_directories_share_the_default_cache_entry() {
    let root = test_temp_root("working-directory-cache");
    let first = root.join("first");
    let second = root.join("second");
    fs::create_dir_all(&first).expect("first working directory should be created");
    fs::create_dir_all(&second).expect("second working directory should be created");
    for directory in [&first, &second] {
        fs::write(directory.join("main.veln"), "fn main() -> ()\n  ()\nend\n")
            .expect("source should be written");
    }
    let tool_dir = root.join("tools");
    write_cache_test_java(&tool_dir);
    let xdg = root.join("host-cache");

    for directory in [&first, &second] {
        let output = cache_test_command(
            directory,
            &["run", "main", "main.veln"],
            &tool_dir,
            &[("XDG_CACHE_HOME", &xdg)],
        );
        assert_success("working-directory-independent run", &output);
        assert!(!directory.join("target/veln-cache").exists());
    }

    let entries = fs::read_dir(xdg.join("veln/jvm"))
        .expect("shared JVM cache should be readable")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join(".veln-cache-ok").is_file())
        .count();
    assert_eq!(entries, 1, "equivalent programs should share one entry");
    fs::remove_dir_all(root).expect("working-directory fixture should be removed");
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn unavailable_unix_host_base_has_no_local_or_temporary_fallback() {
    let project = TestProject::new("unavailable-host-cache".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let marker = project.root.join("java-started");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("JAVA_MARKER", &marker)],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("user cache directory is unavailable")
    );
    assert!(!project.root.join("target/veln-cache").exists());
    assert!(
        !marker.exists(),
        "Java should not start without a cache root"
    );
}

#[cfg(unix)]
#[test]
fn absolute_override_is_complete_and_keeps_lexical_components_usable() {
    let project = TestProject::new("run-cache-override".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let parent = project.root.join("cache-parent");
    fs::create_dir_all(&parent).expect("lexical parent should be created");
    let cache_override = parent.join(".").join("..").join("selected-cache");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", &cache_override)],
    );

    assert_success("explicit cache override run", &output);
    assert!(cache_override.join("jvm").is_dir());
    assert!(!cache_override.join("veln").exists());
}

#[cfg(unix)]
#[test]
fn invalid_overrides_fail_before_test_bodies_without_host_fallback() {
    let project = TestProject::new("test-invalid-cache".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main_test.veln"),
        "test alpha() -> ()\n  ()\nend\n\ntest beta() -> ()\n  ()\nend\n",
    )
    .expect("tests should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let marker = project.root.join("java-started");
    let host_cache = project.root.join("valid-host-cache");
    let tmp_root = project.root.join("tmp");
    fs::create_dir_all(&tmp_root).expect("isolated tmp root should be created");

    for cache_override in [
        project.root.join("empty-placeholder"),
        PathBuf::from("relative-cache"),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&project.root);
        command.args(["test", "main_test.veln"]);
        command.env("PATH", &tool_dir);
        command.env("XDG_CACHE_HOME", &host_cache);
        command.env("JAVA_MARKER", &marker);
        command.env("TMPDIR", &tmp_root);
        if cache_override.is_absolute() {
            command.env("VELN_CACHE_DIR", "");
        } else {
            command.env("VELN_CACHE_DIR", &cache_override);
        }
        let output = command.output().expect("veln test should run");
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid VELN_CACHE_DIR"));
        assert!(!marker.exists(), "no test JVM should start");
        assert!(!host_cache.exists(), "invalid override must not fall back");
        assert_no_entries_with_prefix(&tmp_root, "veln-test-");
    }
}

#[cfg(unix)]
#[test]
fn analysis_and_no_test_results_precede_invalid_cache_configuration() {
    let project = TestProject::new("cache-validation-gates".to_string(), &ToolSetup::default());
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    fs::write(project.root.join("broken.veln"), "fn broken(\n")
        .expect("broken source should be written");
    fs::write(
        project.root.join("no_tests.veln"),
        "fn helper() -> ()\n  ()\nend\n",
    )
    .expect("non-test source should be written");
    let relative = Path::new("relative-cache");

    let analysis = cache_test_command(
        &project.root,
        &["run", "main", "broken.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", relative)],
    );
    assert_eq!(analysis.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&analysis.stderr).contains("VELN_CACHE_DIR"));
    assert!(!project.root.join(relative).exists());

    let no_tests = cache_test_command(
        &project.root,
        &["test", "no_tests.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", relative)],
    );
    assert_eq!(no_tests.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&no_tests.stderr).contains("VELN_CACHE_DIR"));
    assert!(!project.root.join(relative).exists());
}

#[cfg(unix)]
#[test]
fn missing_java_precedes_invalid_cache_configuration() {
    let project = TestProject::new("missing-java-cache-gate".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let empty_tools = project.root.join("empty-tools");
    fs::create_dir_all(&empty_tools).expect("empty tool path should be created");
    let relative = Path::new("relative-cache");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &empty_tools,
        &[("VELN_CACHE_DIR", relative)],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`java` was not found"));
    assert!(!stderr.contains("VELN_CACHE_DIR"));
}

#[cfg(unix)]
#[test]
fn unusable_java_precedes_invalid_cache_configuration() {
    let project = TestProject::new(
        "unusable-java-cache-gate".to_string(),
        &ToolSetup::default(),
    );
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_unusable_cache_test_java(&tool_dir);
    let relative = Path::new("relative-cache");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", relative)],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`java` was not found"));
    assert!(!stderr.contains("VELN_CACHE_DIR"));
    assert!(!project.root.join(relative).exists());
}

#[cfg(unix)]
#[test]
fn inaccessible_tmpdir_does_not_make_other_execute_only_java_available() {
    if process_runs_as_root() {
        return;
    }
    let project = TestProject::new(
        "inaccessible-tmpdir-java-cache-gate".to_string(),
        &ToolSetup::default(),
    );
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_other_execute_only_cache_test_java(&tool_dir);
    let blocked_tmpdir = project.root.join("blocked-tmpdir");
    fs::write(&blocked_tmpdir, "not a directory").expect("blocked TMPDIR should be written");
    let relative = Path::new("relative-cache");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("TMPDIR", &blocked_tmpdir), ("VELN_CACHE_DIR", relative)],
    );

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("`java` was not found"));
    assert!(!stderr.contains("VELN_CACHE_DIR"));
    assert!(!project.root.join(relative).exists());
}

#[test]
fn non_executing_commands_ignore_invalid_cache_configuration() {
    for args in [["--help"].as_slice(), ["--version"].as_slice()] {
        let output = Command::new(env!("CARGO_BIN_EXE_veln"))
            .args(args)
            .env("VELN_CACHE_DIR", "relative-cache")
            .output()
            .expect("non-executing command should run");
        assert_success("non-executing command", &output);
        assert!(!String::from_utf8_lossy(&output.stderr).contains("VELN_CACHE_DIR"));
    }
}

#[cfg(unix)]
#[test]
fn cache_root_file_is_preserved_and_user_code_does_not_start() {
    let project = TestProject::new("cache-root-file".to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let cache_root = project.root.join("cache-root");
    fs::write(&cache_root, "preserve me").expect("cache-root file should be written");
    let marker = project.root.join("java-started");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", &cache_root), ("JAVA_MARKER", &marker)],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Veln cache root"));
    assert_eq!(
        fs::read_to_string(cache_root).expect("file should remain"),
        "preserve me"
    );
    assert!(!marker.exists());
}

#[cfg(unix)]
#[test]
fn cache_root_creation_failure_preserves_existing_parent_and_user_code_does_not_start() {
    let project = TestProject::new(
        "cache-root-creation-failure".to_string(),
        &ToolSetup::default(),
    );
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("source should be written");
    let tool_dir = project.root.join("tools");
    write_cache_test_java(&tool_dir);
    let blocked_parent = project.root.join("blocked-parent");
    fs::write(&blocked_parent, "preserve parent").expect("blocking file should be written");
    let cache_root = blocked_parent.join("selected-cache");
    let marker = project.root.join("java-started");

    let output = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tool_dir,
        &[("VELN_CACHE_DIR", &cache_root), ("JAVA_MARKER", &marker)],
    );

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Veln cache root"));
    assert_eq!(
        fs::read_to_string(&blocked_parent).expect("blocking file should remain"),
        "preserve parent"
    );
    assert!(!cache_root.exists());
    assert!(!project.root.join("target/veln-cache").exists());
    assert!(!marker.exists());
}

#[test]
fn metrics_baseline_check_preserves_report_fields() {
    let project = TestProject::new(
        "metrics-baseline-check-preserves-report-fields".to_string(),
        &ToolSetup::default(),
    );
    fs::write(
        project.root.join("veln.toml"),
        "[tool.metrics]\ndeny_cycles = \"true\"\n",
    )
    .expect("manifest should be written");
    fs::write(
        project.root.join("app.veln"),
        "use util\nfn main() -> Int\n  value()\nend\n",
    )
    .expect("app source should be written");
    fs::write(
        project.root.join("util.veln"),
        "use app\nfn value() -> Int\n  1\nend\n",
    )
    .expect("util source should be written");

    let write_output = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "--write-baseline".to_string(),
            "metrics.baseline.json".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );
    assert!(
        write_output.status.success(),
        "baseline write failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&write_output.stdout),
        String::from_utf8_lossy(&write_output.stderr)
    );

    let report_output = project.veln_with_artifact(
        &["metrics".to_string(), "--json".to_string()],
        None,
        &[],
        None,
        None,
    );
    let check_output = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "--check".to_string(),
            "--baseline".to_string(),
            "metrics.baseline.json".to_string(),
            "--json".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );
    assert!(
        report_output.status.success(),
        "report failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&report_output.stdout),
        String::from_utf8_lossy(&report_output.stderr)
    );
    assert!(
        check_output.status.success(),
        "baseline check failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );

    let report = parse_json(std::str::from_utf8(&report_output.stdout).expect("report JSON"))
        .expect("report should parse");
    let check = parse_json(std::str::from_utf8(&check_output.stdout).expect("check JSON"))
        .expect("check report should parse");

    for field in [
        "project",
        "modules",
        "edges",
        "cycles",
        "abc_subjects",
        "summary",
    ] {
        assert_eq!(
            json_path(&check, field),
            json_path(&report, field),
            "baseline check changed `{field}` report field"
        );
    }
}

#[test]
fn metrics_cli_output_is_stable_for_reversed_input_order() {
    let project = TestProject::new(
        "metrics-cli-output-is-stable-for-reversed-input-order".to_string(),
        &ToolSetup::default(),
    );
    fs::create_dir_all(project.root.join("src")).expect("source directory should be created");
    fs::write(
        project.root.join("veln.toml"),
        "[tool.metrics]\nsimilarity_min_tokens = \"8\"\nmax_findings = \"3\"\n",
    )
    .expect("manifest should be written");
    fs::write(
        project.root.join("src/app.veln"),
        "use src::util\n\nfn add(left: Int, right: Int) -> Int\n  left + right\nend\n\nfn duplicate_app() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n",
    )
    .expect("app source should be written");
    fs::write(
        project.root.join("src/util.veln"),
        "use src::app\n\nfn add(left: Int, right: Int) -> Int\n  left + right\nend\n\nfn duplicate_util() -> Int\n  let value = add(1, 2)\n  let other = add(value, 3)\n  other\nend\n",
    )
    .expect("util source should be written");

    let forward_json = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "--json".to_string(),
            "src/app.veln".to_string(),
            "src/util.veln".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );
    let reversed_json = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "--json".to_string(),
            "src/util.veln".to_string(),
            "src/app.veln".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );
    let forward_human = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "src/app.veln".to_string(),
            "src/util.veln".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );
    let reversed_human = project.veln_with_artifact(
        &[
            "metrics".to_string(),
            "src/util.veln".to_string(),
            "src/app.veln".to_string(),
        ],
        None,
        &[],
        None,
        None,
    );

    assert_success("forward JSON metrics", &forward_json);
    assert_success("reversed JSON metrics", &reversed_json);
    assert_success("forward human metrics", &forward_human);
    assert_success("reversed human metrics", &reversed_human);
    assert_eq!(forward_json.stdout, reversed_json.stdout);
    assert_eq!(forward_human.stdout, reversed_human.stdout);
    assert!(
        String::from_utf8_lossy(&forward_human.stdout).contains(
            "Detailed findings omitted: 1; use veln metrics --json for complete evidence."
        ),
        "reversed input comparison should exercise stable human truncation"
    );
    assert!(
        !String::from_utf8_lossy(&forward_json.stdout).contains('\\'),
        "JSON output should use canonical separators"
    );
    assert!(
        !String::from_utf8_lossy(&forward_human.stdout).contains('\\'),
        "human output should use canonical separators"
    );
}

#[test]
fn analysis_commands_select_the_manifest_package_above_the_invocation_directory() {
    let project = TestProject::new(
        "analysis-commands-select-package-root".to_string(),
        &ToolSetup::default(),
    );
    fs::create_dir_all(project.root.join("work/deep"))
        .expect("nested invocation directory should be created");
    fs::write(
        project.root.join("veln.toml"),
        "[package]\nname = \"command-root\"\n",
    )
    .expect("manifest should be written");
    fs::write(project.root.join("main.veln"), "fn broken(\n")
        .expect("invalid root source should be written");

    for args in [
        &["check", "--json"][..],
        &["doc"][..],
        &["fmt"][..],
        &["metrics", "--json"][..],
        &["run", "--json", "main"][..],
        &["test", "--json"][..],
    ] {
        let args = args
            .iter()
            .map(|arg| (*arg).to_string())
            .collect::<Vec<_>>();
        let output =
            project.veln_with_artifact(&args, Some(Path::new("work/deep")), &[], None, None);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "`{}` did not analyze the invalid package source\n{combined}",
            args.join(" ")
        );
        assert!(
            combined.contains("main.veln"),
            "`{}` did not report the package source\n{combined}",
            args.join(" ")
        );
    }

    let repair_project = TestProject::new(
        "repair-selects-package-root".to_string(),
        &ToolSetup::default(),
    );
    fs::create_dir_all(repair_project.root.join("work/deep"))
        .expect("nested repair invocation directory should be created");
    fs::write(
        repair_project.root.join("veln.toml"),
        "[package]\nname = \"repair-command-root\"\n",
    )
    .expect("repair manifest should be written");
    fs::write(
        repair_project.root.join("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    )
    .expect("repair source should be written");
    let repair_output = repair_project.veln_with_artifact(
        &["repair".to_string(), "--json".to_string()],
        Some(Path::new("work/deep")),
        &[],
        None,
        None,
    );
    assert_success("repair below manifest root", &repair_output);
    let repair_stdout = String::from_utf8_lossy(&repair_output.stdout);
    for expected in [
        "\"repair_id\":\"repair-1\"",
        "\"file\":\"main.veln\"",
        "\"summary\":{\"candidate_count\":1,\"applicable_count\":1",
    ] {
        assert!(
            repair_stdout.contains(expected),
            "repair below manifest root did not report `{expected}`\n{repair_stdout}",
        );
    }

    let lock_output = project.veln_with_artifact(
        &["package".to_string(), "lock".to_string()],
        Some(Path::new("work/deep")),
        &[],
        None,
        None,
    );
    assert_success("package lock below manifest root", &lock_output);
    assert!(project.root.join("veln.lock").is_file());
    assert!(!project.root.join("work/deep/veln.lock").exists());
}

fn case_name(case_dir: &Path) -> String {
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

fn assert_no_metrics_baseline_temp_file(context: &CaseRunContext<'_>, project_root: &Path) {
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

fn assert_no_entries_with_prefix(root: &Path, prefix: &str) {
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

fn assert_success(label: &str, output: &Output) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct TestProject {
    root: PathBuf,
    tool_path: Option<PathBuf>,
}

impl TestProject {
    fn new(name: String, tools: &ToolSetup) -> Self {
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

    fn copy_fixtures(&self, case_dir: &Path) {
        copy_fixture_dir(case_dir, case_dir, &self.root);
    }

    fn source_diagnostic_artifact_path(&self, run_index: usize) -> PathBuf {
        self.root
            .join(format!(".veln-source-diagnostics-{}.json", run_index + 1))
    }

    fn veln_with_artifact(
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

    fn setup_tools(&self, tools: &ToolSetup) {
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

fn copy_fixture_dir(base: &Path, dir: &Path, target_root: &Path) {
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

fn setup_tool(tool_path: &Path, name: &str, availability: ToolAvailability) {
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

const FAKE_GIT_RESOLVED_REV: &str = "0123456789abcdef0123456789abcdef01234567";

#[cfg(unix)]
fn write_fake_success_tool(tool_path: &Path, name: &str) {
    let tool = tool_path.join(name);
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("fake tool should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake tool metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake tool should be executable");
}

#[cfg(windows)]
fn write_fake_success_tool(tool_path: &Path, name: &str) {
    for extension in ["bat", "cmd"] {
        let tool = tool_path.join(format!("{name}.{extension}"));
        fs::write(&tool, "@echo off\r\nexit /b 0\r\n").expect("fake tool should be written");
    }
}

#[cfg(not(any(unix, windows)))]
fn write_fake_success_tool(_tool_path: &Path, name: &str) {
    panic!("fake tool `{name}` is not supported on this platform");
}

#[cfg(unix)]
fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
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
fn write_fake_git_rev_parse_tool(tool_path: &Path, name: &str) {
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
fn write_fake_git_rev_parse_tool(_tool_path: &Path, name: &str) {
    panic!("fake git `{name}` is not supported on this platform");
}

fn find_host_tool(name: &str) -> Option<PathBuf> {
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
fn host_tool_names(name: &str) -> Vec<String> {
    vec![
        format!("{name}.exe"),
        format!("{name}.cmd"),
        format!("{name}.bat"),
        name.to_string(),
    ]
}

#[cfg(not(windows))]
fn host_tool_names(name: &str) -> Vec<String> {
    vec![name.to_string()]
}

#[cfg(unix)]
fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    std::os::unix::fs::symlink(host_tool, tool_path.join(name))
        .expect("real tool symlink should be created");
}

#[cfg(windows)]
fn install_real_tool(tool_path: &Path, name: &str, host_tool: &Path) {
    let extension = host_tool
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("exe");
    fs::copy(host_tool, tool_path.join(format!("{name}.{extension}")))
        .expect("real tool should be copied");
}

#[cfg(not(any(unix, windows)))]
fn install_real_tool(_tool_path: &Path, name: &str, _host_tool: &Path) {
    panic!("real tool `{name}` is not supported on this platform");
}

#[derive(Debug)]
struct CaseInvocation {
    command: Vec<String>,
    cwd: Option<PathBuf>,
    stdin: Option<String>,
    stdin_jsonrpc_file: Option<String>,
    stdin_jsonrpc_workspace_file_uri_directives: Vec<WorkspaceFileUriDirective>,
    repeat: usize,
    env: Vec<(String, String)>,
}

impl CaseInvocation {
    fn materialized_stdin(&self, project_root: &Path) -> Option<String> {
        let input = self.stdin.as_deref()?;
        if self.stdin_jsonrpc_file.is_some() {
            Some(materialize_jsonrpc_workspace_file_uri_directives(
                input,
                &self.stdin_jsonrpc_workspace_file_uri_directives,
                project_root,
            ))
        } else {
            Some(input.to_string())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkspaceFileUriDirective {
    message_index: usize,
    pointer_route: Vec<JsonPointerRouteSegment>,
    relative: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonPointerRouteSegment {
    ArrayIndex(usize),
    ObjectMember { key: String, occurrence: usize },
}

#[derive(Debug)]
struct CaseExpectations {
    exit: i32,
    stdout: StreamExpectation,
    stderr: StreamExpectation,
    help: Option<HelpExpectation>,
    json_assertions: Vec<JsonAssertion>,
    result_value_assertions: Vec<ResultValueAssertion>,
    lsp_assertions: Vec<LspAssertion>,
    mcp_assertions: Vec<McpAssertion>,
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
    binary_fixtures: Vec<BinaryFixtureExpectation>,
    output_chunk_lists: Vec<OutputChunkListExpectation>,
}

#[derive(Debug)]
struct CaseManifest {
    invocation: CaseInvocation,
    expectations: CaseExpectations,
    source_errors: SourceErrorExpectation,
    manifest_error: Option<ManifestErrorExpectation>,
    tools: ToolSetup,
    requires: Requirements,
    skip: SkipRules,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SourceErrorExpectation {
    #[default]
    Forbidden,
    Expected,
}

impl CaseManifest {
    fn read(path: &Path) -> Self {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("{}: failed to read manifest: {error}", path.display()));
        parse_manifest(path, &text)
    }

    fn validate(&self, path: &Path) {
        self.expectations.validate(path);
        if !self.expectations.lsp_assertions.is_empty()
            && self.invocation.command.first().map(String::as_str) != Some("lsp")
        {
            manifest_error(path, 0, "lsp_assert requires `command = [\"lsp\", ...]`");
        }
        if !self.expectations.mcp_assertions.is_empty()
            && self.invocation.command.first().map(String::as_str) != Some("mcp")
        {
            manifest_error(path, 0, "mcp_assert requires `command = [\"mcp\", ...]`");
        }
        if let Some(expectation) = &self.manifest_error
            && !expectation.has_assertion()
        {
            manifest_error(path, 0, "manifest_error section has no assertion");
        }
    }

    fn skip_reason(&self) -> Option<String> {
        if self.requires_jdk() && !jdk_is_available() {
            return Some("requires a real JDK".to_string());
        }
        if self
            .skip
            .platforms
            .iter()
            .any(|platform| platform.matches())
        {
            let reason = self
                .skip
                .reason
                .as_deref()
                .unwrap_or("case is skipped on this platform");
            return Some(reason.to_string());
        }
        None
    }

    fn requires_jdk(&self) -> bool {
        self.requires.jdk || self.tools.requires_jdk()
    }

    fn assert_no_unexpected_example_source_errors(&self, case_dir: &Path, project_root: &Path) {
        if !is_specification_example(case_dir) || !self.needs_independent_source_error_guard() {
            return;
        }
        if self.command_explicitly_expects_source_errors()
            && self.source_errors == SourceErrorExpectation::Forbidden
        {
            return;
        }

        let project = Project::discover(project_root.to_path_buf(), &[]).unwrap_or_else(|error| {
            panic!(
                "{}: inspect the example project inputs; source-error guard discovery failed: {error}",
                case_dir.display()
            )
        });
        let errors = checked_project_diagnostics(project, DoctestMode::Include)
            .into_iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .collect::<Vec<_>>();

        match (self.source_errors, errors.is_empty()) {
            (SourceErrorExpectation::Forbidden, false) => {
                panic!(
                    "{}: remove unexpected source error diagnostics, or set `source_errors = \"expected\"` in case.toml when this example exists to exercise them; clean examples prevent unrelated editor errors.\n{}",
                    case_dir.display(),
                    source_error_evidence(&errors)
                );
            }
            (SourceErrorExpectation::Expected, true) => {
                panic!(
                    "{}: remove stale `source_errors = \"expected\"` from case.toml; the example no longer produces a source error diagnostic",
                    case_dir.display()
                );
            }
            _ => {}
        }
    }

    fn assert_no_unexpected_command_source_errors(
        &self,
        context: &CaseRunContext<'_>,
        evidence: &CommandSourceDiagnosticEvidence,
    ) {
        if evidence.error_count == 0 {
            return;
        }
        panic!(
            "{}: remove unexpected source error diagnostics, or set `source_errors = \"expected\"` in case.toml when this example exists to exercise them; clean examples prevent unrelated editor errors.\n{}",
            context.label(),
            evidence.message
        );
    }

    fn needs_pre_command_source_error_guard(&self, case_dir: &Path) -> bool {
        is_specification_example(case_dir)
            && self.needs_independent_source_error_guard()
            && !self.needs_command_source_error_guard(case_dir)
            && !(self.command_explicitly_expects_source_errors()
                && self.source_errors == SourceErrorExpectation::Forbidden)
    }

    fn needs_command_source_error_guard(&self, case_dir: &Path) -> bool {
        is_specification_example(case_dir)
            && self.source_errors == SourceErrorExpectation::Forbidden
            && !self.command_explicitly_expects_source_errors()
            && matches!(
                self.invocation.command.first().map(String::as_str),
                Some("check" | "run" | "test")
            )
    }

    fn needs_independent_source_error_guard(&self) -> bool {
        matches!(
            self.invocation.command.first().map(String::as_str),
            Some("check" | "doc" | "fmt" | "lsp" | "metrics" | "repair" | "run" | "test")
        )
    }

    fn command_explicitly_expects_source_errors(&self) -> bool {
        self.expectations.exit != 0
            && matches!(
                self.invocation.command.first().map(String::as_str),
                Some("check" | "doc" | "fmt" | "repair")
            )
            || self.invocation.command.first().map(String::as_str) == Some("run")
                && self.expectations.exit != 0
                && self
                    .expectations
                    .stderr
                    .contains
                    .iter()
                    .any(|fragment| fragment.contains("runnable entry retains user-defined effect"))
    }

    fn validate_fixture_schema_references(&self, project_root: &Path) {
        if self
            .expectations
            .binary_fixtures
            .iter()
            .all(|fixture| fixture.schema.is_none())
        {
            return;
        }

        let inputs = command_source_inputs(&self.invocation.command);
        let project = Project::discover(project_root.to_path_buf(), &inputs)
            .unwrap_or_else(|error| manifest_error(project_root, 0, error));
        let current_module = fixture_reference_module(&project, inputs.first());
        let (module, _) = load_surface_module(&project);
        validate_binary_fixture_schema_references(
            project_root,
            &module,
            current_module.as_deref(),
            &self.expectations.binary_fixtures,
        );
    }
}

fn is_specification_example(case_dir: &Path) -> bool {
    let components = case_dir.components().collect::<Vec<_>>();
    components
        .windows(2)
        .any(|pair| pair[0].as_os_str() == "examples" && pair[1].as_os_str() == "specification")
}

fn source_error_evidence(errors: &[Diagnostic]) -> String {
    errors
        .iter()
        .map(|diagnostic| {
            let location = diagnostic.span.as_ref().map_or_else(
                || "<unknown>".to_string(),
                |span| {
                    format!(
                        "{}:{}:{}",
                        span.file.as_str(),
                        span.start.line,
                        span.start.column
                    )
                },
            );
            format!(
                "{location}: error[{}]: {}",
                diagnostic.id, diagnostic.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

struct CommandSourceDiagnosticEvidence {
    error_count: usize,
    message: String,
}

impl CommandSourceDiagnosticEvidence {
    fn read(context: &CaseRunContext<'_>, path: &Path) -> Self {
        let text = fs::read_to_string(path).unwrap_or_else(|error| {
            panic!(
                "{}: command did not write source diagnostic artifact `{}`: {error}",
                context.label(),
                path.display()
            )
        });
        let json = parse_json(&text).unwrap_or_else(|error| {
            panic!(
                "{}: source diagnostic artifact JSON parse failed: {error}\n{}",
                context.label(),
                text
            )
        });
        let diagnostics = json
            .object_field("diagnostics")
            .and_then(JsonValue::as_array)
            .unwrap_or_else(|| {
                panic!(
                    "{}: source diagnostic artifact is missing diagnostics array",
                    context.label()
                )
            });
        let errors = diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .object_field("severity")
                    .and_then(JsonValue::as_str)
                    == Some("error")
            })
            .collect::<Vec<_>>();
        Self {
            error_count: errors.len(),
            message: command_source_error_evidence(&errors),
        }
    }
}

fn command_source_error_evidence(errors: &[&JsonValue]) -> String {
    errors
        .iter()
        .map(|diagnostic| {
            let location = diagnostic
                .object_field("span")
                .and_then(command_span_evidence)
                .unwrap_or_else(|| "<unknown>".to_string());
            let id = diagnostic
                .object_field("id")
                .and_then(JsonValue::as_str)
                .unwrap_or("<unknown>");
            let message = diagnostic
                .object_field("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("<missing message>");
            format!("{location}: error[{id}]: {message}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn command_span_evidence(span: &JsonValue) -> Option<String> {
    if matches!(span, JsonValue::Null) {
        return None;
    }
    let file = span.object_field("file")?.as_str()?;
    let start = span.object_field("start")?;
    let line = start.object_field("line")?.as_i64()?;
    let column = start.object_field("column")?.as_i64()?;
    Some(format!("{file}:{line}:{column}"))
}

impl CaseExpectations {
    fn validate(&self, path: &Path) {
        for (index, assertion) in self.json_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.result_value_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.lsp_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, assertion) in self.mcp_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        if let Some(help) = &self.help
            && !help.has_assertion()
        {
            manifest_error(path, 0, "help section has no assertion");
        }
        for (index, assertion) in self.file_assertions.iter().enumerate() {
            assertion.validate(path, index);
        }
        for (index, diagnostic) in self.diagnostics.iter().enumerate() {
            diagnostic.validate(path, index);
        }
        for (index, fixture) in self.binary_fixtures.iter().enumerate() {
            fixture.validate(path, index);
        }
        for (index, chunks) in self.output_chunk_lists.iter().enumerate() {
            chunks.validate(path, index);
        }
    }

    fn assert_matches(
        &self,
        context: &CaseRunContext<'_>,
        output: &CapturedOutput,
        project_root: &Path,
    ) {
        let mut independent_failures = Vec::new();
        if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(
                output.exit,
                Some(self.exit),
                "{}: expected exit {}, got {:?}\nstdout:\n{}\nstderr:\n{}",
                context.label(),
                self.exit,
                output.exit,
                output.stdout,
                output.stderr
            );
        })) {
            independent_failures.push(panic_message(panic));
        }
        if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_stream(context, "stdout", &self.stdout, &output.stdout)
        })) {
            independent_failures.push(panic_message(panic));
        }
        if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_stream(context, "stderr", &self.stderr, &output.stderr)
        })) {
            independent_failures.push(panic_message(panic));
        }
        if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_lsp_assertions_in_workspace(
                context,
                &output.stdout,
                &self.lsp_assertions,
                project_root,
            )
        })) {
            independent_failures.push(panic_message(panic));
        }
        if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_mcp_assertions(context, &output.stdout, &self.mcp_assertions, project_root)
        })) {
            independent_failures.push(panic_message(panic));
        }
        if !independent_failures.is_empty() {
            panic!("{}", independent_failures.join("\n"));
        }
        if let Some(help) = &self.help {
            help.assert_matches(context, output);
        }

        let json = if self.needs_stdout_json() {
            Some(parse_json(&output.stdout).unwrap_or_else(|error| {
                panic!(
                    "{}: stdout JSON parse failed: {error}\n{}",
                    context.label(),
                    output.stdout
                )
            }))
        } else {
            None
        };

        if let Some(json) = json.as_ref() {
            for (index, assertion) in self.json_assertions.iter().enumerate() {
                assert_json_path_in_workspace(context, json, assertion, index, project_root);
            }
            for (index, assertion) in self.result_value_assertions.iter().enumerate() {
                assert_result_value_path_in_workspace(
                    context,
                    json,
                    assertion,
                    index,
                    project_root,
                );
            }
            for diagnostic in &self.diagnostics {
                assert_diagnostic(context, json, diagnostic);
            }
        }

        if !self.binary_fixtures.is_empty() || !self.output_chunk_lists.is_empty() {
            let program_stdout = json
                .as_ref()
                .and_then(|json| json_path(json, "stdout"))
                .and_then(JsonValue::as_str)
                .unwrap_or(&output.stdout);
            for fixture in &self.binary_fixtures {
                assert_binary_fixture(context, program_stdout, fixture);
            }
            for chunks in &self.output_chunk_lists {
                assert_output_chunk_list(context, program_stdout, chunks);
            }
        }
    }

    fn needs_stdout_json(&self) -> bool {
        self.stdout.format == Some(StreamFormat::Json)
            || !self.json_assertions.is_empty()
            || !self.result_value_assertions.is_empty()
            || !self.diagnostics.is_empty()
            || !self.binary_fixtures.is_empty()
            || !self.output_chunk_lists.is_empty()
    }

    fn assert_files_match(&self, context: &CaseRunContext<'_>, project_root: &Path) {
        for assertion in &self.file_assertions {
            let path = project_root.join(&assertion.path);
            let actual = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!(
                    "{}: failed to read asserted file `{}`: {error}",
                    context.label(),
                    assertion.path
                )
            });
            assert_eq!(
                actual,
                assertion
                    .equals
                    .as_ref()
                    .expect("file assertion should have expected text")
                    .as_str(),
                "{}: file `{}` contents mismatch",
                context.label(),
                assertion.path
            );
        }
    }
}

#[derive(Debug, Default)]
struct ManifestErrorExpectation {
    contains: Vec<String>,
}

impl ManifestErrorExpectation {
    fn assert_matches(&self, case_dir: &Path, message: &str) {
        for expected in &self.contains {
            assert!(
                message.contains(expected),
                "{}: manifest error should contain `{expected}`, got `{message}`",
                case_dir.display()
            );
        }
    }

    fn has_assertion(&self) -> bool {
        !self.contains.is_empty()
    }
}

struct CaseRunContext<'a> {
    case_dir: &'a Path,
    run_number: usize,
}

impl CaseRunContext<'_> {
    fn label(&self) -> String {
        format!("{} run {}", self.case_dir.display(), self.run_number)
    }
}

struct CapturedOutput {
    exit: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CapturedOutput {
    fn read(context: &CaseRunContext<'_>, output: Output) -> Self {
        Self {
            exit: output.status.code(),
            stdout: stream_text(output.stdout, context, "stdout"),
            stderr: stream_text(output.stderr, context, "stderr"),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct StreamExpectation {
    format: Option<StreamFormat>,
    contains: Vec<String>,
    not_contains: Vec<String>,
    equals: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamFormat {
    Empty,
    Text,
    Json,
}

#[derive(Debug)]
struct HelpExpectation {
    stream: OutputStream,
    summary: Option<String>,
    usage: Option<String>,
    commands: Vec<String>,
    arguments: Vec<String>,
    options: Vec<String>,
    contains: Vec<String>,
}

impl Default for HelpExpectation {
    fn default() -> Self {
        Self {
            stream: OutputStream::Stdout,
            summary: None,
            usage: None,
            commands: Vec::new(),
            arguments: Vec::new(),
            options: Vec::new(),
            contains: Vec::new(),
        }
    }
}

impl HelpExpectation {
    fn assert_matches(&self, context: &CaseRunContext<'_>, output: &CapturedOutput) {
        let stream = self.stream.text(output);
        let stream_name = self.stream.name();
        let help_surface = format!("help {stream_name}");
        if let Some(summary) = &self.summary {
            assert_eq!(
                stream.lines().next(),
                Some(summary.as_str()),
                "{}: help summary mismatch on {}",
                context.label(),
                stream_name
            );
        }
        if let Some(usage) = &self.usage {
            assert_contains_fragment(context, &help_surface, stream, &format!("Usage: {usage}\n"));
        }
        assert_help_section(context, &help_surface, stream, "Commands", &self.commands);
        assert_help_section(context, &help_surface, stream, "Arguments", &self.arguments);
        assert_help_section(context, &help_surface, stream, "Options", &self.options);
        for fragment in &self.contains {
            assert_contains_fragment(context, &help_surface, stream, fragment);
        }
    }

    fn has_assertion(&self) -> bool {
        self.summary.is_some()
            || self.usage.is_some()
            || !self.commands.is_empty()
            || !self.arguments.is_empty()
            || !self.options.is_empty()
            || !self.contains.is_empty()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputStream {
    Stdout,
    Stderr,
}

impl OutputStream {
    fn name(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }

    fn text(self, output: &CapturedOutput) -> &str {
        match self {
            Self::Stdout => &output.stdout,
            Self::Stderr => &output.stderr,
        }
    }

    fn parse(path: &Path, value: &ManifestValue<'_>) -> Self {
        let line_number = value.line();
        let value = parse_string(path, value);
        match value.as_str() {
            "stdout" => Self::Stdout,
            "stderr" => Self::Stderr,
            _ => manifest_error(
                path,
                line_number,
                format!("unknown output stream `{value}`"),
            ),
        }
    }
}

#[derive(Debug)]
struct JsonAssertion {
    path: String,
    operation: Option<ValueAssertionOperation>,
}

#[derive(Debug, PartialEq, Eq)]
enum ValueAssertionOperation {
    Equals(JsonValue),
    EqualsFile(JsonValue),
    EqualsJsonFile(JsonValue),
    Contains(String),
    Length(usize),
    Missing,
    WorkspaceFileUri(String),
}

#[derive(Debug)]
struct LspAssertion {
    id: Option<JsonValue>,
    method: Option<String>,
    occurrence: Option<usize>,
    path: String,
    path_present: bool,
    pointer_tokens: Vec<String>,
    operation: Option<RpcAssertionOperation>,
    operation_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
enum RpcAssertionOperation {
    Equals(JsonValue),
    EqualsFile(String),
    EqualsFileRef(CaseTextReference),
    EqualsJsonFile(JsonValue),
    EqualsJsonFileRef(CaseTextReference),
    Contains(String),
    Length(usize),
    Missing(bool),
    WorkspaceFileUri(String),
}

#[derive(Debug)]
struct McpAssertion {
    id: Option<JsonValue>,
    path: String,
    path_present: bool,
    pointer_tokens: Vec<String>,
    operation: Option<RpcAssertionOperation>,
    operation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaseTextReference {
    line_number: usize,
    relative: String,
}

impl LspAssertion {
    fn validate(&self, path: &Path, index: usize) {
        if self.id.is_some() == self.method.is_some() {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} needs exactly one of `id` or `method`"),
            );
        }
        if !self.path_present {
            manifest_error(path, 0, format!("lsp_assert {index} is missing `path`"));
        }
        if self.occurrence.is_some() && self.method.is_none() {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} `occurrence` is valid only with `method`"),
            );
        }
        if matches!(self.operation, Some(RpcAssertionOperation::Missing(false))) {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} `missing` must be true when present"),
            );
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "lsp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
    }

    fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else {
            format!(
                "notification method {:?} occurrence {}",
                self.method.as_deref().expect("validated method selector"),
                self.occurrence.unwrap_or(0)
            )
        }
    }
}

impl JsonAssertion {
    fn validate(&self, path: &Path, index: usize) {
        if self.path.is_empty() {
            manifest_error(path, 0, format!("json_assert {index} is missing `path`"));
        }
    }
}

impl McpAssertion {
    fn validate(&self, path: &Path, index: usize) {
        if self.id.is_none() {
            manifest_error(path, 0, format!("mcp_assert {index} is missing `id`"));
        }
        if !self.path_present {
            manifest_error(path, 0, format!("mcp_assert {index} is missing `path`"));
        }
        if matches!(self.operation, Some(RpcAssertionOperation::Missing(false))) {
            manifest_error(
                path,
                0,
                format!("mcp_assert {index} `missing` must be true when present"),
            );
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "mcp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
    }

    fn selector(&self) -> String {
        format!(
            "response id {}",
            self.id
                .as_ref()
                .expect("validated MCP id")
                .to_compact_string()
        )
    }
}

#[derive(Debug)]
struct ResultValueAssertion {
    value_path: String,
    path: String,
    operation: Option<ValueAssertionOperation>,
}

impl ResultValueAssertion {
    fn validate(&self, path: &Path, index: usize) {
        if self.value_path.is_empty() {
            manifest_error(
                path,
                0,
                format!("result_value_assert {index} is missing `value_path`"),
            );
        }
        if self.path.is_empty() {
            manifest_error(
                path,
                0,
                format!("result_value_assert {index} is missing `path`"),
            );
        }
    }
}

#[derive(Debug)]
struct FileAssertion {
    path: String,
    equals: Option<String>,
    operation_count: usize,
}

impl FileAssertion {
    fn validate(&self, path: &Path, index: usize) {
        if self.path.is_empty() {
            manifest_error(path, 0, format!("file_assert {index} is missing `path`"));
        }
        if self.operation_count != 1 {
            manifest_error(
                path,
                0,
                format!("file_assert {index} needs exactly one of `equals` or `equals_file`"),
            );
        }
    }
}

#[derive(Debug)]
struct DiagnosticExpectation {
    id: String,
    severity: Option<String>,
    kind: Option<String>,
    message: Option<String>,
    span: Option<SpanExpectation>,
}

impl DiagnosticExpectation {
    fn validate(&self, path: &Path, index: usize) {
        if self.id.is_empty() {
            manifest_error(path, 0, format!("diagnostics {index} is missing `id`"));
        }
    }
}

#[derive(Debug)]
struct BinaryFixtureExpectation {
    name: String,
    schema: Option<String>,
    bytes: Option<BinaryFixtureBytes>,
    consumed: Option<usize>,
    error: Option<String>,
    byte_diagnostic: Option<BinaryFixtureByteDiagnostic>,
}

impl BinaryFixtureExpectation {
    fn validate(&self, path: &Path, index: usize) {
        if self.name.is_empty() {
            manifest_error(path, 0, format!("binary_fixture {index} is missing `name`"));
        }
        match (&self.bytes, &self.error) {
            (Some(_), None) => {}
            (None, Some(_)) if self.consumed.is_none() => {}
            (Some(_), Some(_)) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} cannot specify both `hex` and `error`"),
            ),
            (None, Some(_)) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} with `error` cannot specify `consumed`"),
            ),
            (None, None) => manifest_error(
                path,
                0,
                format!("binary_fixture {index} needs `hex` or `error`"),
            ),
        }
        if let (Some(bytes), Some(consumed)) = (&self.bytes, self.consumed)
            && consumed > bytes.bytes.len()
        {
            manifest_error(
                path,
                0,
                format!("binary_fixture {index} `consumed` exceeds decoded byte count"),
            );
        }
        if let Some(diagnostic) = &self.byte_diagnostic {
            diagnostic.validate(path, index, self.bytes.is_some());
        }
    }
}

#[derive(Debug)]
struct BinaryFixtureBytes {
    hex: String,
    bytes: Vec<u8>,
}

#[derive(Debug)]
struct OutputChunkListExpectation {
    name: String,
    chunks: Option<Vec<BinaryFixtureBytes>>,
}

impl OutputChunkListExpectation {
    fn validate(&self, path: &Path, index: usize) {
        if self.name.is_empty() {
            manifest_error(
                path,
                0,
                format!("output_chunk_list {index} is missing `name`"),
            );
        }
        if self.chunks.is_none() {
            manifest_error(
                path,
                0,
                format!("output_chunk_list {index} is missing `chunks`"),
            );
        }
    }
}

#[derive(Debug, Default)]
struct BinaryFixtureByteDiagnostic {
    diagnostic_id: Option<String>,
    byte_offset: Option<usize>,
    expected_count: Option<usize>,
    available_count: Option<usize>,
    readiness: Option<String>,
    field_path: Option<JsonValue>,
}

impl BinaryFixtureByteDiagnostic {
    fn validate(&self, path: &Path, fixture_index: usize, fixture_has_bytes: bool) {
        if !fixture_has_bytes {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} byte diagnostic metadata needs `hex`"),
            );
        }
        if self.byte_offset.is_none() || self.field_path.is_none() {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} has incomplete byte diagnostic metadata"),
            );
        }
        validate_binary_fixture_field_path(path, fixture_index, self.field_path.as_ref());

        let has_count_metadata = self.expected_count.is_some()
            || self.available_count.is_some()
            || self.readiness.is_some();
        if has_count_metadata
            && (self.expected_count.is_none()
                || self.available_count.is_none()
                || self.readiness.is_none())
        {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} has incomplete byte count metadata"),
            );
        }
        if self.diagnostic_id.is_none() && !has_count_metadata {
            manifest_error(
                path,
                0,
                format!("binary_fixture {fixture_index} needs `diagnostic_id` for field metadata"),
            );
        }
    }
}

#[derive(Debug, Default)]
struct SpanExpectation {
    file: Option<String>,
    line: Option<i64>,
    column: Option<i64>,
}

#[derive(Debug, Default)]
struct Requirements {
    jdk: bool,
}

#[derive(Debug, Default)]
struct ToolSetup {
    java: Option<ToolAvailability>,
    git: Option<ToolAvailability>,
}

impl ToolSetup {
    fn needs_path(&self) -> bool {
        self.configured().next().is_some()
    }

    fn requires_jdk(&self) -> bool {
        self.configured().any(ToolConfig::requires_jdk)
    }

    fn configured(&self) -> impl Iterator<Item = ToolConfig> {
        [
            self.java
                .map(|availability| ToolName::Java.config(availability)),
            self.git
                .map(|availability| ToolName::Git.config(availability)),
        ]
        .into_iter()
        .flatten()
    }

    fn set(&mut self, name: ToolName, availability: ToolAvailability) {
        match name {
            ToolName::Java => self.java = Some(availability),
            ToolName::Git => self.git = Some(availability),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ToolConfig {
    name: ToolName,
    availability: ToolAvailability,
}

impl ToolConfig {
    fn requires_jdk(self) -> bool {
        self.name == ToolName::Java && self.availability == ToolAvailability::Real
    }

    fn setup(self, tool_path: &Path) {
        setup_tool(tool_path, self.name.as_str(), self.availability);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolName {
    Java,
    Git,
}

impl ToolName {
    fn as_str(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Git => "git",
        }
    }

    fn config(self, availability: ToolAvailability) -> ToolConfig {
        ToolConfig {
            name: self,
            availability,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ToolAvailability {
    Missing,
    FakeSuccess,
    FakeGitRevParse,
    Real,
}

#[derive(Debug, Default)]
struct SkipRules {
    platforms: Vec<SkipPlatform>,
    reason: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkipPlatform {
    Unix,
    Windows,
    Macos,
    Linux,
}

impl SkipPlatform {
    fn matches(self) -> bool {
        match self {
            Self::Unix => cfg!(unix),
            Self::Windows => cfg!(windows),
            Self::Macos => cfg!(target_os = "macos"),
            Self::Linux => cfg!(target_os = "linux"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Section {
    Root,
    Stdout,
    Stderr,
    Help,
    JsonAssert(usize),
    ResultValueAssert(usize),
    LspAssert(usize),
    McpAssert(usize),
    FileAssert(usize),
    Diagnostic(usize),
    DiagnosticSpan(usize),
    ManifestError,
    BinaryFixture(usize),
    OutputChunkList(usize),
    Requires,
    Skip,
    Env,
    Tools,
}

fn parse_manifest(path: &Path, text: &str) -> CaseManifest {
    let statements = manifest_syntax::parse_document(path, text);
    validate_manifest_assignment_preflight(path, &statements);
    let mut parser = ManifestParser::new(path);
    for statement in statements {
        match statement {
            ManifestStatement::Section { name, line } => {
                parser.parse_section_header(&name, line);
            }
            ManifestStatement::Assignment { key, line, value } => {
                parser.parse_section_key(line, key, &value);
            }
        }
    }
    parser.finish()
}

fn validate_manifest_assignment_preflight(path: &Path, statements: &[ManifestStatement<'_>]) {
    let mut section = Section::Root;
    let mut seen = BTreeSet::new();
    let mut root_stdin_operands = 0;
    let mut json_operations = Vec::<ValueAssertionPreflight>::new();
    let mut result_value_operations = Vec::<ValueAssertionPreflight>::new();
    let mut lsp_operations = Vec::<LspAssertionPreflight>::new();
    let mut mcp_operations = Vec::<McpAssertionPreflight>::new();
    let mut file_assert_operations = Vec::<usize>::new();

    for statement in statements {
        match statement {
            ManifestStatement::Section { name, line } => {
                section = match name.as_str() {
                    "[stdout]" => Section::Stdout,
                    "[stderr]" => Section::Stderr,
                    "[help]" => Section::Help,
                    "[requires]" => Section::Requires,
                    "[skip]" => Section::Skip,
                    "[env]" => Section::Env,
                    "[tools]" => Section::Tools,
                    "[[json_assert]]" => {
                        json_operations.push(ValueAssertionPreflight::default());
                        Section::JsonAssert(json_operations.len() - 1)
                    }
                    "[[result_value_assert]]" => {
                        result_value_operations.push(ValueAssertionPreflight::default());
                        Section::ResultValueAssert(result_value_operations.len() - 1)
                    }
                    "[[lsp_assert]]" => {
                        lsp_operations.push(LspAssertionPreflight::default());
                        Section::LspAssert(lsp_operations.len() - 1)
                    }
                    "[[mcp_assert]]" => {
                        mcp_operations.push(McpAssertionPreflight::default());
                        Section::McpAssert(mcp_operations.len() - 1)
                    }
                    "[[file_assert]]" => {
                        file_assert_operations.push(0);
                        Section::FileAssert(file_assert_operations.len() - 1)
                    }
                    "[[diagnostics]]" => Section::Diagnostic(0),
                    "[diagnostics.span]" => Section::DiagnosticSpan(0),
                    "[manifest_error]" => Section::ManifestError,
                    "[[binary_fixture]]" => Section::BinaryFixture(0),
                    "[[output_chunk_list]]" => Section::OutputChunkList(0),
                    _ => continue,
                };
                if matches!(
                    section,
                    Section::Diagnostic(0)
                        | Section::DiagnosticSpan(0)
                        | Section::BinaryFixture(0)
                        | Section::OutputChunkList(0)
                ) {
                    seen.clear();
                } else {
                    let _ = line;
                }
            }
            ManifestStatement::Assignment { key, line, value } => {
                let assignment = format!("{section:?}:{key}");
                if !is_accumulating_manifest_key(section, key) && !seen.insert(assignment) {
                    manifest_error(path, *line, format!("duplicate key `{key}`"));
                }
                match section {
                    Section::Root
                        if matches!(*key, "stdin" | "stdin_file" | "stdin_jsonrpc_file") =>
                    {
                        root_stdin_operands += 1;
                    }
                    Section::JsonAssert(index) => {
                        json_operations[index].record(path, key, value);
                    }
                    Section::ResultValueAssert(index) => {
                        result_value_operations[index].record(path, key, value);
                    }
                    Section::LspAssert(index)
                        if matches!(
                            *key,
                            "equals"
                                | "equals_file"
                                | "equals_json_file"
                                | "contains"
                                | "length"
                                | "workspace_file_uri"
                                | "missing"
                        ) =>
                    {
                        lsp_operations[index].operation.record(path, key, value);
                    }
                    Section::McpAssert(index)
                        if matches!(
                            *key,
                            "equals"
                                | "equals_file"
                                | "equals_json_file"
                                | "contains"
                                | "length"
                                | "workspace_file_uri"
                                | "missing"
                        ) =>
                    {
                        mcp_operations[index].operation.record(path, key, value);
                    }
                    Section::LspAssert(index) => {
                        lsp_operations[index].record_selector_or_path(path, key, value);
                    }
                    Section::McpAssert(index) => {
                        mcp_operations[index].record_selector_or_path(path, key, value);
                    }
                    Section::FileAssert(index) if matches!(*key, "equals" | "equals_file") => {
                        file_assert_operations[index] += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    if root_stdin_operands > 1 {
        manifest_error(
            path,
            0,
            "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
        );
    }
    for (index, assertion) in json_operations.iter().enumerate() {
        if assertion.operation.missing_false {
            manifest_error(
                path,
                0,
                format!("json_assert {index} `missing` must be true when present"),
            );
        }
        if assertion.operation.count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "json_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
        assertion.operation.validate_operands(
            path,
            &value_assertion_base_context(
                "json_assert",
                index,
                assertion.selected_path.as_deref().unwrap_or(""),
            ),
        );
    }
    for (index, assertion) in result_value_operations.iter().enumerate() {
        if assertion.operation.missing_false {
            manifest_error(
                path,
                0,
                format!("result_value_assert {index} `missing` must be true when present"),
            );
        }
        if assertion.operation.count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "result_value_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
        assertion.operation.validate_operands(
            path,
            &value_assertion_base_context(
                "result_value_assert",
                index,
                assertion.selected_path.as_deref().unwrap_or(""),
            ),
        );
    }
    for (index, assertion) in lsp_operations.iter().enumerate() {
        if assertion.operation.missing_false {
            manifest_error(
                path,
                0,
                format!("lsp_assert {index} `missing` must be true when present"),
            );
        }
        if assertion.operation.count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "lsp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
        assertion.operation.validate_operands(
            path,
            &assertion_base_context(
                "lsp_assert",
                index,
                &assertion.selector(),
                assertion.path.as_deref().unwrap_or(""),
            ),
        );
    }
    for (index, assertion) in mcp_operations.iter().enumerate() {
        if assertion.operation.missing_false {
            manifest_error(
                path,
                0,
                format!("mcp_assert {index} `missing` must be true when present"),
            );
        }
        if assertion.operation.count != 1 {
            manifest_error(
                path,
                0,
                format!(
                    "mcp_assert {index} needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
                ),
            );
        }
        assertion.operation.validate_operands(
            path,
            &assertion_base_context(
                "mcp_assert",
                index,
                &assertion.selector(),
                assertion.path.as_deref().unwrap_or(""),
            ),
        );
    }
    for (index, count) in file_assert_operations.iter().enumerate() {
        if *count != 1 {
            manifest_error(
                path,
                0,
                format!("file_assert {index} needs exactly one of `equals` or `equals_file`"),
            );
        }
    }
}

#[derive(Default)]
struct ValueAssertionPreflight {
    selected_path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl ValueAssertionPreflight {
    fn record(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "path" => self.selected_path = Some(parse_string(path, value)),
            "equals" | "equals_file" | "equals_json_file" | "contains" | "length"
            | "workspace_file_uri" | "missing" => self.operation.record(path, key, value),
            _ => {}
        }
    }
}

#[derive(Default)]
struct AssertionOperationPreflight {
    count: usize,
    missing_false: bool,
    length: Option<PreflightLengthOperand>,
    workspace_file_uri: Option<PreflightWorkspaceFileUriOperand>,
}

impl AssertionOperationPreflight {
    fn record(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        self.count += 1;
        if key == "missing" && !parse_bool(path, value) {
            self.missing_false = true;
        }
        if key == "length" {
            self.length = Some(PreflightLengthOperand {
                line_number: value.line(),
                raw: value.raw().to_string(),
            });
        }
        if key == "workspace_file_uri" {
            self.workspace_file_uri = Some(PreflightWorkspaceFileUriOperand {
                line_number: value.line(),
                relative: value.is_string().then(|| parse_string(path, value)),
                string_operand: value.is_string(),
            });
        }
    }

    fn validate_operands(&self, path: &Path, base_context: &str) {
        if let Some(operand) = &self.length {
            let context = format!("{base_context} length");
            parse_nonnegative_usize_raw_with_context(
                path,
                operand.line_number,
                &operand.raw,
                Some(&context),
            );
        }
        if let Some(operand) = &self.workspace_file_uri {
            let context = format!("{base_context} workspace_file_uri");
            if !operand.string_operand {
                manifest_error(
                    path,
                    operand.line_number,
                    format!("{context}: expected string"),
                );
            }
            let relative = operand
                .relative
                .as_deref()
                .expect("validated workspace_file_uri string operand");
            validate_workspace_file_uri_operand_with_context(
                path,
                operand.line_number,
                relative,
                Some(&context),
            );
        }
    }
}

#[derive(Default)]
struct LspAssertionPreflight {
    id: Option<JsonValue>,
    method: Option<String>,
    occurrence: Option<usize>,
    path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl LspAssertionPreflight {
    fn record_selector_or_path(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "id" => self.id = Some(parse_manifest_json_value(path, value)),
            "method" => self.method = Some(parse_string(path, value)),
            "occurrence" => self.occurrence = Some(parse_nonnegative_usize(path, value)),
            "path" => self.path = Some(parse_string(path, value)),
            _ => {}
        }
    }

    fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else if let Some(method) = &self.method {
            format!(
                "notification method {method:?} occurrence {}",
                self.occurrence.unwrap_or(0)
            )
        } else {
            "unresolved selector".to_string()
        }
    }
}

#[derive(Default)]
struct McpAssertionPreflight {
    id: Option<JsonValue>,
    path: Option<String>,
    operation: AssertionOperationPreflight,
}

impl McpAssertionPreflight {
    fn record_selector_or_path(&mut self, path: &Path, key: &str, value: &ManifestValue<'_>) {
        match key {
            "id" => self.id = Some(parse_manifest_json_value_allow_decimal(path, value)),
            "path" => self.path = Some(parse_string(path, value)),
            _ => {}
        }
    }

    fn selector(&self) -> String {
        if let Some(id) = &self.id {
            format!("response id {}", id.to_compact_string())
        } else {
            "unresolved selector".to_string()
        }
    }
}

#[derive(Default)]
struct PreflightLengthOperand {
    line_number: usize,
    raw: String,
}

#[derive(Default)]
struct PreflightWorkspaceFileUriOperand {
    line_number: usize,
    relative: Option<String>,
    string_operand: bool,
}

struct ManifestParser<'a> {
    path: &'a Path,
    command: Option<Vec<String>>,
    cwd: Option<PathBuf>,
    stdin: Option<String>,
    stdin_jsonrpc_file: Option<String>,
    stdin_jsonrpc_workspace_file_uri_directives: Vec<WorkspaceFileUriDirective>,
    exit: Option<i32>,
    repeat: usize,
    env: Vec<(String, String)>,
    source_errors: SourceErrorExpectation,
    stdout: StreamExpectation,
    stderr: StreamExpectation,
    help: Option<HelpExpectation>,
    json_assertions: Vec<JsonAssertion>,
    result_value_assertions: Vec<ResultValueAssertion>,
    lsp_assertions: Vec<LspAssertion>,
    mcp_assertions: Vec<McpAssertion>,
    file_assertions: Vec<FileAssertion>,
    diagnostics: Vec<DiagnosticExpectation>,
    manifest_error: Option<ManifestErrorExpectation>,
    binary_fixtures: Vec<BinaryFixtureExpectation>,
    output_chunk_lists: Vec<OutputChunkListExpectation>,
    tools: ToolSetup,
    requires: Requirements,
    skip: SkipRules,
    section: Section,
    seen_assignments: BTreeSet<String>,
    stdin_operand_count: usize,
    case_text_cache: CaseTextCache,
}

impl<'a> ManifestParser<'a> {
    fn new(path: &'a Path) -> Self {
        Self {
            path,
            command: None,
            cwd: None,
            stdin: None,
            stdin_jsonrpc_file: None,
            stdin_jsonrpc_workspace_file_uri_directives: Vec::new(),
            exit: None,
            repeat: 1,
            env: Vec::new(),
            source_errors: SourceErrorExpectation::Forbidden,
            stdout: StreamExpectation::default(),
            stderr: StreamExpectation::default(),
            help: None,
            json_assertions: Vec::new(),
            result_value_assertions: Vec::new(),
            lsp_assertions: Vec::new(),
            mcp_assertions: Vec::new(),
            file_assertions: Vec::new(),
            diagnostics: Vec::new(),
            manifest_error: None,
            binary_fixtures: Vec::new(),
            output_chunk_lists: Vec::new(),
            tools: ToolSetup::default(),
            requires: Requirements::default(),
            skip: SkipRules::default(),
            section: Section::Root,
            seen_assignments: BTreeSet::new(),
            stdin_operand_count: 0,
            case_text_cache: CaseTextCache::default(),
        }
    }

    fn parse_section_header(&mut self, line: &str, line_number: usize) {
        self.section = match line {
            "[stdout]" => Section::Stdout,
            "[stderr]" => Section::Stderr,
            "[help]" => self.parse_help_header(line_number),
            "[requires]" => Section::Requires,
            "[skip]" => Section::Skip,
            "[env]" => Section::Env,
            "[tools]" => Section::Tools,
            "[[json_assert]]" => self.parse_json_assert_header(),
            "[[result_value_assert]]" => self.parse_result_value_assert_header(),
            "[[lsp_assert]]" => self.parse_lsp_assert_header(),
            "[[mcp_assert]]" => self.parse_mcp_assert_header(),
            "[[file_assert]]" => self.parse_file_assert_header(),
            "[[diagnostics]]" => self.parse_diagnostic_header(),
            "[diagnostics.span]" => self.parse_diagnostic_span_header(line_number),
            "[manifest_error]" => self.parse_manifest_error_header(line_number),
            "[[binary_fixture]]" => self.parse_binary_fixture_header(),
            "[[output_chunk_list]]" => self.parse_output_chunk_list_header(),
            _ => manifest_error(self.path, line_number, format!("unknown section `{line}`")),
        };
    }

    fn parse_help_header(&mut self, line_number: usize) -> Section {
        if self.help.is_some() {
            manifest_error(self.path, line_number, "duplicate help section");
        }
        self.help = Some(HelpExpectation::default());
        Section::Help
    }

    fn parse_json_assert_header(&mut self) -> Section {
        self.json_assertions.push(JsonAssertion {
            path: String::new(),
            operation: None,
        });
        Section::JsonAssert(self.json_assertions.len() - 1)
    }

    fn parse_file_assert_header(&mut self) -> Section {
        self.file_assertions.push(FileAssertion {
            path: String::new(),
            equals: None,
            operation_count: 0,
        });
        Section::FileAssert(self.file_assertions.len() - 1)
    }

    fn parse_result_value_assert_header(&mut self) -> Section {
        self.result_value_assertions.push(ResultValueAssertion {
            value_path: String::new(),
            path: String::new(),
            operation: None,
        });
        Section::ResultValueAssert(self.result_value_assertions.len() - 1)
    }

    fn parse_lsp_assert_header(&mut self) -> Section {
        self.lsp_assertions.push(LspAssertion {
            id: None,
            method: None,
            occurrence: None,
            path: String::new(),
            path_present: false,
            pointer_tokens: Vec::new(),
            operation: None,
            operation_count: 0,
        });
        Section::LspAssert(self.lsp_assertions.len() - 1)
    }

    fn parse_mcp_assert_header(&mut self) -> Section {
        self.mcp_assertions.push(McpAssertion {
            id: None,
            path: String::new(),
            path_present: false,
            pointer_tokens: Vec::new(),
            operation: None,
            operation_count: 0,
        });
        Section::McpAssert(self.mcp_assertions.len() - 1)
    }

    fn parse_diagnostic_header(&mut self) -> Section {
        self.diagnostics.push(DiagnosticExpectation {
            id: String::new(),
            severity: None,
            kind: None,
            message: None,
            span: None,
        });
        Section::Diagnostic(self.diagnostics.len() - 1)
    }

    fn parse_diagnostic_span_header(&mut self, line_number: usize) -> Section {
        let Some(index) = self.diagnostics.len().checked_sub(1) else {
            manifest_error(
                self.path,
                line_number,
                "diagnostics.span needs a diagnostic",
            );
        };
        if self.diagnostics[index].span.is_none() {
            self.diagnostics[index].span = Some(SpanExpectation::default());
        }
        Section::DiagnosticSpan(index)
    }

    fn parse_manifest_error_header(&mut self, line_number: usize) -> Section {
        if self.manifest_error.is_some() {
            manifest_error(self.path, line_number, "duplicate manifest_error section");
        }
        self.manifest_error = Some(ManifestErrorExpectation::default());
        Section::ManifestError
    }

    fn parse_binary_fixture_header(&mut self) -> Section {
        self.binary_fixtures.push(BinaryFixtureExpectation {
            name: String::new(),
            schema: None,
            bytes: None,
            consumed: None,
            error: None,
            byte_diagnostic: None,
        });
        Section::BinaryFixture(self.binary_fixtures.len() - 1)
    }

    fn parse_output_chunk_list_header(&mut self) -> Section {
        self.output_chunk_lists.push(OutputChunkListExpectation {
            name: String::new(),
            chunks: None,
        });
        Section::OutputChunkList(self.output_chunk_lists.len() - 1)
    }

    fn parse_section_key(&mut self, line_number: usize, key: &str, value: &ManifestValue<'_>) {
        if !is_accumulating_manifest_key(self.section, key) {
            self.reject_duplicate_assignment(line_number, key);
        }
        match self.section {
            Section::Root => self.parse_root_key(line_number, key, value),
            Section::Stdout => parse_stream_key(
                self.path,
                line_number,
                &mut self.stdout,
                key,
                value,
                true,
                &mut self.case_text_cache,
            ),
            Section::Stderr => parse_stream_key(
                self.path,
                line_number,
                &mut self.stderr,
                key,
                value,
                false,
                &mut self.case_text_cache,
            ),
            Section::Help => parse_help_key(
                self.path,
                line_number,
                self.help.as_mut().expect("help section should exist"),
                key,
                value,
                &mut self.case_text_cache,
            ),
            Section::Requires => self.parse_requires_key(line_number, key, value),
            Section::Skip => self.parse_skip_key(line_number, key, value),
            Section::Env => self
                .env
                .push((key.to_string(), parse_string(self.path, value))),
            Section::Tools => self.parse_tools_key(line_number, key, value),
            Section::JsonAssert(index) => {
                self.parse_json_assert_key(index, line_number, key, value)
            }
            Section::ResultValueAssert(index) => {
                self.parse_result_value_assert_key(index, line_number, key, value)
            }
            Section::LspAssert(index) => self.parse_lsp_assert_key(index, line_number, key, value),
            Section::McpAssert(index) => self.parse_mcp_assert_key(index, line_number, key, value),
            Section::FileAssert(index) => {
                self.parse_file_assert_key(index, line_number, key, value)
            }
            Section::Diagnostic(index) => self.parse_diagnostic_key(index, line_number, key, value),
            Section::DiagnosticSpan(index) => {
                self.parse_diagnostic_span_key(index, line_number, key, value);
            }
            Section::ManifestError => self.parse_manifest_error_key(line_number, key, value),
            Section::BinaryFixture(index) => {
                self.parse_binary_fixture_key(index, line_number, key, value)
            }
            Section::OutputChunkList(index) => {
                self.parse_output_chunk_list_key(index, line_number, key, value)
            }
        }
    }

    fn reject_duplicate_assignment(&mut self, line_number: usize, key: &str) {
        let assignment = format!("{:?}:{key}", self.section);
        if !self.seen_assignments.insert(assignment) {
            manifest_error(self.path, line_number, format!("duplicate key `{key}`"));
        }
    }

    fn parse_root_key(&mut self, line_number: usize, key: &str, value: &ManifestValue<'_>) {
        match key {
            "command" => self.command = Some(parse_string_array(self.path, value)),
            "cwd" => self.cwd = Some(PathBuf::from(parse_string(self.path, value))),
            "stdin" => {
                self.stdin_operand_count += 1;
                self.stdin = Some(parse_string(self.path, value));
            }
            "stdin_file" => {
                self.stdin_operand_count += 1;
                self.stdin = Some(self.case_text_cache.read(self.path, value));
            }
            "stdin_jsonrpc_file" => {
                self.stdin_operand_count += 1;
                let relative = parse_string(self.path, value);
                let mut directives = Vec::new();
                self.stdin = Some(load_jsonrpc_stdin_snapshot(
                    self.path,
                    value.line(),
                    &relative,
                    &mut self.case_text_cache,
                    &mut directives,
                ));
                self.stdin_jsonrpc_file = Some(relative);
                self.stdin_jsonrpc_workspace_file_uri_directives = directives;
            }
            "exit" => self.exit = Some(parse_i32(self.path, value)),
            "repeat" => self.repeat = parse_positive_usize(self.path, value),
            "source_errors" => {
                self.source_errors = parse_source_error_expectation(self.path, value)
            }
            _ => manifest_error(self.path, line_number, format!("unknown root key `{key}`")),
        }
    }

    fn parse_requires_key(&mut self, line_number: usize, key: &str, value: &ManifestValue<'_>) {
        match key {
            "jdk" => self.requires.jdk = parse_bool(self.path, value),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown requires key `{key}`"),
            ),
        }
    }

    fn parse_skip_key(&mut self, line_number: usize, key: &str, value: &ManifestValue<'_>) {
        match key {
            "platforms" => {
                self.skip.platforms = parse_string_array(self.path, value)
                    .into_iter()
                    .map(|platform| parse_skip_platform(self.path, line_number, &platform))
                    .collect();
            }
            "reason" => self.skip.reason = Some(parse_string(self.path, value)),
            _ => manifest_error(self.path, line_number, format!("unknown skip key `{key}`")),
        }
    }

    fn parse_tools_key(&mut self, line_number: usize, key: &str, value: &ManifestValue<'_>) {
        match key {
            "java" => {
                self.tools
                    .set(ToolName::Java, parse_tool_availability(self.path, value));
            }
            "git" => {
                self.tools
                    .set(ToolName::Git, parse_tool_availability(self.path, value));
            }
            _ => manifest_error(self.path, line_number, format!("unknown tools key `{key}`")),
        }
    }

    fn parse_json_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "path" => self.json_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Equals(
                    parse_manifest_json_value(self.path, value),
                ))
            }
            "equals_file" => {
                self.json_assertions[index].operation = Some(ValueAssertionOperation::EqualsFile(
                    JsonValue::String(self.case_text_cache.read(self.path, value)),
                ))
            }
            "equals_json_file" => {
                let text = self.case_text_cache.read(self.path, value);
                self.json_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsJsonFile(
                        parse_json(&text).unwrap_or_else(|error| {
                            manifest_error(
                                self.path,
                                line_number,
                                format!("invalid json_assert equals_json_file value: {error}"),
                            )
                        }),
                    ))
            }
            "contains" => {
                self.json_assertions[index].operation =
                    Some(parse_value_contains_operation(self.path, value));
            }
            "length" => {
                let context = value_assertion_context(
                    "json_assert",
                    index,
                    &self.json_assertions[index].path,
                    "length",
                );
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                let context = value_assertion_context(
                    "json_assert",
                    index,
                    &self.json_assertions[index].path,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                self.json_assertions[index].operation =
                    Some(ValueAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                let missing = parse_bool(self.path, value);
                debug_assert!(missing, "preflight rejects missing = false");
                self.json_assertions[index].operation = Some(ValueAssertionOperation::Missing);
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown json_assert key `{key}`"),
            ),
        }
    }

    fn parse_result_value_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "value_path" => {
                self.result_value_assertions[index].value_path = parse_string(self.path, value)
            }
            "path" => self.result_value_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.result_value_assertions[index].operation = Some(
                    ValueAssertionOperation::Equals(parse_manifest_json_value(self.path, value)),
                )
            }
            "equals_file" => {
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsFile(JsonValue::String(
                        self.case_text_cache.read(self.path, value),
                    )))
            }
            "equals_json_file" => {
                let text = self.case_text_cache.read(self.path, value);
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::EqualsJsonFile(
                        parse_json(&text).unwrap_or_else(|error| {
                            manifest_error(
                                self.path,
                                line_number,
                                format!(
                                    "invalid result_value_assert equals_json_file value: {error}"
                                ),
                            )
                        }),
                    ))
            }
            "contains" => {
                self.result_value_assertions[index].operation =
                    Some(parse_value_contains_operation(self.path, value));
            }
            "length" => {
                let context = value_assertion_context(
                    "result_value_assert",
                    index,
                    &self.result_value_assertions[index].path,
                    "length",
                );
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::Length(
                        parse_nonnegative_usize_with_context(self.path, value, &context),
                    ));
            }
            "workspace_file_uri" => {
                let context = value_assertion_context(
                    "result_value_assert",
                    index,
                    &self.result_value_assertions[index].path,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                let missing = parse_bool(self.path, value);
                debug_assert!(missing, "preflight rejects missing = false");
                self.result_value_assertions[index].operation =
                    Some(ValueAssertionOperation::Missing);
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown result_value_assert key `{key}`"),
            ),
        }
    }

    fn parse_lsp_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let assertion = &mut self.lsp_assertions[index];
        match key {
            "id" => {
                let id = parse_manifest_json_value(self.path, value);
                if !matches!(
                    id,
                    JsonValue::Null | JsonValue::Number(_) | JsonValue::String(_)
                ) && !matches!(
                    &id,
                    JsonValue::Decimal(raw) if is_json_integer_token(raw)
                ) {
                    manifest_error(
                        self.path,
                        line_number,
                        "lsp_assert `id` must be a JSON string, integer, or null",
                    );
                }
                assertion.id = Some(id);
            }
            "method" => assertion.method = Some(parse_string(self.path, value)),
            "occurrence" => assertion.occurrence = Some(parse_nonnegative_usize(self.path, value)),
            "path" => {
                assertion.path = parse_string(self.path, value);
                assertion.path_present = true;
                assertion.pointer_tokens = parse_json_pointer(
                    self.path,
                    line_number,
                    "lsp_assert",
                    index,
                    &assertion.path,
                );
            }
            "equals" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Equals(
                    parse_manifest_json_value(self.path, value),
                ));
            }
            "equals_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsFileRef(
                    parse_case_text_reference(self.path, value, "lsp_assert", "equals_file"),
                ));
            }
            "equals_json_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsJsonFileRef(
                    parse_case_text_reference(self.path, value, "lsp_assert", "equals_json_file"),
                ));
            }
            "contains" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Contains(parse_string(
                    self.path, value,
                )));
            }
            "length" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context("lsp_assert", index, "length");
                assertion.operation = Some(RpcAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context(
                    "lsp_assert",
                    index,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                assertion.operation = Some(RpcAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                assertion.operation_count += 1;
                assertion.operation =
                    Some(RpcAssertionOperation::Missing(parse_bool(self.path, value)));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown lsp_assert key `{key}`"),
            ),
        }
    }

    fn parse_mcp_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let assertion = &mut self.mcp_assertions[index];
        match key {
            "id" => {
                let id = parse_manifest_json_value_allow_decimal(self.path, value);
                if !matches!(id, JsonValue::Number(_) | JsonValue::String(_))
                    && !matches!(
                        &id,
                        JsonValue::Decimal(raw) if is_json_integer_token(raw)
                    )
                {
                    manifest_error(
                        self.path,
                        line_number,
                        "mcp_assert `id` must be a JSON string or integer",
                    );
                }
                assertion.id = Some(id);
            }
            "path" => {
                assertion.path = parse_string(self.path, value);
                assertion.path_present = true;
                assertion.pointer_tokens = parse_json_pointer(
                    self.path,
                    line_number,
                    "mcp_assert",
                    index,
                    &assertion.path,
                );
            }
            "equals" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::Equals(
                    parse_manifest_mcp_json_value(self.path, value),
                ));
            }
            "equals_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsFileRef(
                    parse_case_text_reference(self.path, value, "mcp_assert", "equals_file"),
                ));
            }
            "equals_json_file" => {
                assertion.operation_count += 1;
                assertion.operation = Some(RpcAssertionOperation::EqualsJsonFileRef(
                    parse_case_text_reference(self.path, value, "mcp_assert", "equals_json_file"),
                ));
            }
            "contains" => {
                record_mcp_contains_assertion(assertion, self.path, value);
            }
            "length" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context("mcp_assert", index, "length");
                assertion.operation = Some(RpcAssertionOperation::Length(
                    parse_nonnegative_usize_with_context(self.path, value, &context),
                ));
            }
            "workspace_file_uri" => {
                assertion.operation_count += 1;
                let context = unresolved_assertion_operation_context(
                    "mcp_assert",
                    index,
                    "workspace_file_uri",
                );
                let relative = parse_string_with_context(self.path, value, &context);
                validate_workspace_file_uri_operand_with_context(
                    self.path,
                    line_number,
                    &relative,
                    Some(&context),
                );
                assertion.operation = Some(RpcAssertionOperation::WorkspaceFileUri(relative));
            }
            "missing" => {
                assertion.operation_count += 1;
                assertion.operation =
                    Some(RpcAssertionOperation::Missing(parse_bool(self.path, value)));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown mcp_assert key `{key}`"),
            ),
        }
    }

    fn parse_file_assert_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "path" => self.file_assertions[index].path = parse_string(self.path, value),
            "equals" => {
                self.file_assertions[index].operation_count += 1;
                self.file_assertions[index].equals = Some(parse_string(self.path, value));
            }
            "equals_file" => {
                self.file_assertions[index].operation_count += 1;
                self.file_assertions[index].equals =
                    Some(self.case_text_cache.read(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown file_assert key `{key}`"),
            ),
        }
    }

    fn parse_diagnostic_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        match key {
            "id" => self.diagnostics[index].id = parse_string(self.path, value),
            "severity" => {
                self.diagnostics[index].severity = Some(parse_string(self.path, value));
            }
            "kind" => self.diagnostics[index].kind = Some(parse_string(self.path, value)),
            "message" => {
                self.diagnostics[index].message = Some(parse_string(self.path, value));
            }
            "message_file" => {
                self.diagnostics[index].message = Some(self.case_text_cache.read(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics key `{key}`"),
            ),
        }
    }

    fn parse_diagnostic_span_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let span = self.diagnostics[index]
            .span
            .as_mut()
            .expect("diagnostic span should exist");
        match key {
            "file" => span.file = Some(parse_string(self.path, value)),
            "line" => span.line = Some(parse_i64(self.path, value)),
            "column" => span.column = Some(parse_i64(self.path, value)),
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown diagnostics.span key `{key}`"),
            ),
        }
    }

    fn parse_manifest_error_key(
        &mut self,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let expectation = self
            .manifest_error
            .as_mut()
            .expect("manifest_error section should exist");
        match key {
            "contains" => {
                expectation
                    .contains
                    .extend(parse_string_array(self.path, value));
            }
            "contains_file" => {
                expectation
                    .contains
                    .push(self.case_text_cache.read(self.path, value));
            }
            "contains_files" => {
                expectation
                    .contains
                    .extend(self.case_text_cache.read_many(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown manifest_error key `{key}`"),
            ),
        }
    }

    fn parse_binary_fixture_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let fixture = &mut self.binary_fixtures[index];
        match key {
            "name" => fixture.name = parse_string(self.path, value),
            "schema" => fixture.schema = Some(parse_string(self.path, value)),
            "hex" => {
                fixture.bytes = Some(parse_binary_fixture_hex(self.path, value));
            }
            "consumed" => {
                fixture.consumed = Some(parse_nonnegative_usize(self.path, value));
            }
            "error" => fixture.error = Some(parse_string(self.path, value)),
            "diagnostic_id" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .diagnostic_id = Some(parse_string(self.path, value));
            }
            "byte_offset" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .byte_offset = Some(parse_nonnegative_usize(self.path, value));
            }
            "expected_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .expected_count = Some(parse_nonnegative_usize(self.path, value));
            }
            "available_count" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .available_count = Some(parse_nonnegative_usize(self.path, value));
            }
            "readiness" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .readiness = Some(parse_string(self.path, value));
            }
            "field_path" => {
                fixture
                    .byte_diagnostic
                    .get_or_insert_with(BinaryFixtureByteDiagnostic::default)
                    .field_path = Some(parse_manifest_json_value(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown binary_fixture key `{key}`"),
            ),
        }
    }

    fn parse_output_chunk_list_key(
        &mut self,
        index: usize,
        line_number: usize,
        key: &str,
        value: &ManifestValue<'_>,
    ) {
        let chunks = &mut self.output_chunk_lists[index];
        match key {
            "name" => chunks.name = parse_string(self.path, value),
            "chunks" => {
                chunks.chunks = Some(parse_binary_fixture_hex_array(self.path, value));
            }
            _ => manifest_error(
                self.path,
                line_number,
                format!("unknown output_chunk_list key `{key}`"),
            ),
        }
    }

    fn finish(mut self) -> CaseManifest {
        let path = self.path;
        let mut case_text_cache = std::mem::take(&mut self.case_text_cache);
        if self.stdin_operand_count > 1 {
            manifest_error(
                path,
                0,
                "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
            );
        }
        let mut manifest = CaseManifest {
            invocation: CaseInvocation {
                command: self
                    .command
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `command`")),
                cwd: self.cwd,
                stdin: self.stdin,
                stdin_jsonrpc_file: self.stdin_jsonrpc_file,
                stdin_jsonrpc_workspace_file_uri_directives: self
                    .stdin_jsonrpc_workspace_file_uri_directives,
                repeat: self.repeat,
                env: self.env,
            },
            expectations: CaseExpectations {
                exit: self
                    .exit
                    .unwrap_or_else(|| manifest_error(self.path, 0, "missing `exit`")),
                stdout: self.stdout,
                stderr: self.stderr,
                help: self.help,
                json_assertions: self.json_assertions,
                result_value_assertions: self.result_value_assertions,
                lsp_assertions: self.lsp_assertions,
                mcp_assertions: self.mcp_assertions,
                file_assertions: self.file_assertions,
                diagnostics: self.diagnostics,
                binary_fixtures: self.binary_fixtures,
                output_chunk_lists: self.output_chunk_lists,
            },
            source_errors: self.source_errors,
            manifest_error: self.manifest_error,
            tools: self.tools,
            requires: self.requires,
            skip: self.skip,
        };

        manifest.validate(path);
        resolve_lsp_mcp_file_backed_assertions(
            path,
            &mut manifest.expectations,
            &mut case_text_cache,
        );
        manifest
    }
}

fn resolve_lsp_mcp_file_backed_assertions(
    path: &Path,
    expectations: &mut CaseExpectations,
    case_text_cache: &mut CaseTextCache,
) {
    for (index, assertion) in expectations.lsp_assertions.iter_mut().enumerate() {
        let selector = assertion.selector();
        resolve_protocol_file_backed_operation(
            path,
            "lsp_assert",
            index,
            &selector,
            &assertion.path,
            &mut assertion.operation,
            case_text_cache,
        );
    }
    for (index, assertion) in expectations.mcp_assertions.iter_mut().enumerate() {
        let selector = assertion.selector();
        resolve_protocol_file_backed_operation(
            path,
            "mcp_assert",
            index,
            &selector,
            &assertion.path,
            &mut assertion.operation,
            case_text_cache,
        );
    }
}

fn resolve_protocol_file_backed_operation(
    path: &Path,
    section: &str,
    index: usize,
    selector: &str,
    pointer: &str,
    operation: &mut Option<RpcAssertionOperation>,
    case_text_cache: &mut CaseTextCache,
) {
    let Some(current) = operation.take() else {
        return;
    };
    *operation = Some(match current {
        RpcAssertionOperation::EqualsFileRef(reference) => {
            let context = assertion_context(section, index, selector, pointer, "equals_file");
            let text = case_text_cache.read_path_with_context(
                path,
                reference.line_number,
                &reference.relative,
                Some(&context),
            );
            RpcAssertionOperation::EqualsFile(text)
        }
        RpcAssertionOperation::EqualsJsonFileRef(reference) => {
            let context = assertion_context(section, index, selector, pointer, "equals_json_file");
            let text = case_text_cache.read_path_with_context(
                path,
                reference.line_number,
                &reference.relative,
                Some(&context),
            );
            RpcAssertionOperation::EqualsJsonFile(parse_json(&text).unwrap_or_else(|error| {
                manifest_error(
                    path,
                    reference.line_number,
                    format!("invalid {context} value: {error}"),
                )
            }))
        }
        operation => operation,
    });
}

fn assertion_context(
    section: &str,
    index: usize,
    selector: &str,
    pointer: &str,
    operation: &str,
) -> String {
    format!(
        "{} {operation}",
        assertion_base_context(section, index, selector, pointer)
    )
}

fn assertion_base_context(section: &str, index: usize, selector: &str, pointer: &str) -> String {
    format!("{section} {index} {selector} path `{pointer}`")
}

fn value_assertion_context(section: &str, index: usize, path: &str, operation: &str) -> String {
    format!(
        "{} {operation}",
        value_assertion_base_context(section, index, path)
    )
}

fn value_assertion_base_context(section: &str, index: usize, path: &str) -> String {
    format!("{section} {index} path `{path}`")
}

fn unresolved_assertion_operation_context(section: &str, index: usize, operation: &str) -> String {
    format!("{section} {index} {operation}")
}

fn validate_binary_fixture_field_path(
    path: &Path,
    fixture_index: usize,
    field_path: Option<&JsonValue>,
) {
    let Some(JsonValue::Array(segments)) = field_path else {
        manifest_error(
            path,
            0,
            format!("binary_fixture {fixture_index} `field_path` must be a JSON array"),
        );
    };
    for (segment_index, segment) in segments.iter().enumerate() {
        let JsonValue::Object(_) = segment else {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} must be an object"
                ),
            );
        };
        if segment
            .object_field("kind")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `kind`"
                ),
            );
        }
        if segment
            .object_field("name")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `name`"
                ),
            );
        }
    }
}

fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    let Some(command_name) = command.first().map(String::as_str) else {
        return Vec::new();
    };
    match command_name {
        "run" => run_command_source_inputs(&command[1..]),
        "check" | "doc" | "fmt" | "metrics" | "test" => source_inputs_after_flags(&command[1..]),
        _ => Vec::new(),
    }
}

fn run_command_source_inputs(args: &[String]) -> Vec<PathBuf> {
    let mut saw_entry = false;
    let mut inputs = Vec::new();
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        if !saw_entry {
            saw_entry = true;
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

fn source_inputs_after_flags(args: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        if matches!(
            arg.as_str(),
            "--baseline" | "--write-baseline" | "--jobs" | "-j"
        ) {
            let _ = args.next();
            continue;
        }
        if arg.starts_with("--baseline=")
            || arg.starts_with("--write-baseline=")
            || arg.starts_with("--jobs=")
        {
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

fn fixture_reference_module(project: &Project, first_input: Option<&PathBuf>) -> Option<String> {
    if let Some(first_input) = first_input {
        let source_path = if first_input.is_absolute() {
            first_input.clone()
        } else {
            project.root.join(first_input)
        };
        if source_path.is_file()
            && let Ok(source) = veln_source::SourceFile::read(&project.root, &source_path)
            && let Ok(module) = derive_source_module_path(&source)
        {
            return Some(module);
        }
    }
    project
        .files
        .first()
        .and_then(|source| derive_source_module_path(source).ok())
}

fn validate_binary_fixture_schema_references(
    path: &Path,
    module: &SurfaceModule,
    current_module: Option<&str>,
    fixtures: &[BinaryFixtureExpectation],
) {
    let mut errors = Vec::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let Some(schema) = &fixture.schema else {
            continue;
        };
        match resolve_fixture_schema_reference(module, schema, current_module) {
            FixtureSchemaResolution::Resolved { name } => {
                if let Some(error) =
                    validate_binary_fixture_schema_field_path(index, &name, fixture)
                {
                    errors.push(error);
                }
            }
            FixtureSchemaResolution::Private => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is private"
            )),
            FixtureSchemaResolution::WrongKind(kind) => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is a {kind}, not a schema"
            )),
            FixtureSchemaResolution::Unresolved => errors.push(format!(
                "unresolved binary_fixture {index} schema reference `{schema}`"
            )),
        }
    }
    if !errors.is_empty() {
        manifest_error(path, 0, errors.join("\n"));
    }
}

fn validate_binary_fixture_schema_field_path(
    fixture_index: usize,
    schema_name: &str,
    fixture: &BinaryFixtureExpectation,
) -> Option<String> {
    let field_path = fixture
        .byte_diagnostic
        .as_ref()
        .and_then(|diagnostic| diagnostic.field_path.as_ref())?;
    let segments = field_path.as_array()?;
    let first_schema = segments
        .first()
        .and_then(|segment| match segment.object_field("kind") {
            Some(kind) if kind.as_str() == Some("schema") => segment.object_field("name"),
            _ => None,
        })
        .and_then(JsonValue::as_str);
    if first_schema != Some(schema_name) {
        return Some(format!(
            "binary_fixture {fixture_index} `field_path` first segment must name schema `{schema_name}`"
        ));
    }
    None
}

enum FixtureSchemaResolution {
    Resolved { name: String },
    Private,
    WrongKind(&'static str),
    Unresolved,
}

fn resolve_fixture_schema_reference(
    module: &SurfaceModule,
    target: &str,
    current_module: Option<&str>,
) -> FixtureSchemaResolution {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    resolve_fixture_schema_segments(module, &segments, current_module, true, &mut Vec::new())
}

fn resolve_fixture_schema_segments(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    match segments {
        [name] => resolve_fixture_schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return FixtureSchemaResolution::Unresolved;
            };
            resolve_fixture_schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
            )
        }
        _ => FixtureSchemaResolution::Unresolved,
    }
}

fn resolve_fixture_schema_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            FixtureSchemaResolution::Resolved {
                name: schema.name.clone().expect("schema should have a name"),
            }
        } else {
            FixtureSchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_fixture_schema_alias_target(module, alias, visited_aliases);
    }
    fixture_schema_wrong_kind(module, module_name, name).map_or(
        FixtureSchemaResolution::Unresolved,
        FixtureSchemaResolution::WrongKind,
    )
}

fn resolve_fixture_schema_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    let Some(name) = &alias.name else {
        return FixtureSchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return FixtureSchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_fixture_schema_segments(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    resolution
}

fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn is_accumulating_manifest_key(section: Section, key: &str) -> bool {
    matches!(
        (section, key),
        (
            Section::Stdout | Section::Stderr,
            "contains"
                | "contains_file"
                | "contains_files"
                | "not_contains"
                | "not_contains_file"
                | "not_contains_files"
        ) | (
            Section::Help | Section::ManifestError,
            "contains" | "contains_file" | "contains_files"
        )
    )
}

fn fixture_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if module.codecs.iter().any(|codec| {
        codec.name.as_deref() == Some(name) && codec.module_name.as_deref() == module_name
    }) {
        return Some("codec");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

fn parse_help_key(
    path: &Path,
    line_number: usize,
    help: &mut HelpExpectation,
    key: &str,
    value: &ManifestValue<'_>,
    case_text_cache: &mut CaseTextCache,
) {
    match key {
        "stream" => help.stream = OutputStream::parse(path, value),
        "summary" => help.summary = Some(parse_string(path, value)),
        "usage" => help.usage = Some(parse_string(path, value)),
        "commands" => help.commands = parse_string_array(path, value),
        "arguments" => help.arguments = parse_string_array(path, value),
        "options" => help.options = parse_string_array(path, value),
        "contains" => help.contains.extend(parse_string_array(path, value)),
        "contains_file" => help.contains.push(case_text_cache.read(path, value)),
        "contains_files" => help.contains.extend(case_text_cache.read_many(path, value)),
        _ => manifest_error(path, line_number, format!("unknown help key `{key}`")),
    }
}

fn parse_stream_key(
    path: &Path,
    line_number: usize,
    stream: &mut StreamExpectation,
    key: &str,
    value: &ManifestValue<'_>,
    allow_json: bool,
    case_text_cache: &mut CaseTextCache,
) {
    match key {
        "format" => {
            let format = parse_string(path, value);
            stream.format = Some(match format.as_str() {
                "empty" => StreamFormat::Empty,
                "text" => StreamFormat::Text,
                "json" if allow_json => StreamFormat::Json,
                _ => manifest_error(
                    path,
                    line_number,
                    format!("unknown stream format `{format}`"),
                ),
            });
        }
        "equals_file" => stream.equals = Some(case_text_cache.read(path, value)),
        "contains" => stream.contains.extend(parse_string_array(path, value)),
        "contains_file" => stream.contains.push(case_text_cache.read(path, value)),
        "contains_files" => stream
            .contains
            .extend(case_text_cache.read_many(path, value)),
        "not_contains" => stream.not_contains.extend(parse_string_array(path, value)),
        "not_contains_file" => stream.not_contains.push(case_text_cache.read(path, value)),
        "not_contains_files" => stream
            .not_contains
            .extend(case_text_cache.read_many(path, value)),
        _ => manifest_error(path, line_number, format!("unknown stream key `{key}`")),
    }
}

fn parse_value_contains_operation(
    path: &Path,
    value: &ManifestValue<'_>,
) -> ValueAssertionOperation {
    ValueAssertionOperation::Contains(parse_string(path, value))
}

fn record_mcp_contains_assertion(
    assertion: &mut McpAssertion,
    path: &Path,
    value: &ManifestValue<'_>,
) {
    assertion.operation_count += 1;
    assertion.operation = Some(RpcAssertionOperation::Contains(parse_string(path, value)));
}

#[derive(Debug, Default)]
struct CaseTextCache {
    snapshots: BTreeMap<PathBuf, String>,
}

impl CaseTextCache {
    fn read_many(&mut self, path: &Path, value: &ManifestValue<'_>) -> Vec<String> {
        parse_string_array(path, value)
            .into_iter()
            .map(|relative| self.read_path(path, value.line(), &relative))
            .collect()
    }

    fn read(&mut self, path: &Path, value: &ManifestValue<'_>) -> String {
        let relative = parse_string(path, value);
        self.read_path(path, value.line(), &relative)
    }

    fn read_path(&mut self, path: &Path, line_number: usize, relative: &str) -> String {
        self.read_path_with_context(path, line_number, relative, None)
    }

    fn read_path_with_context(
        &mut self,
        path: &Path,
        line_number: usize,
        relative: &str,
        context: Option<&str>,
    ) -> String {
        let relative_path = validate_case_file_reference(path, line_number, relative, context);
        if let Some(snapshot) = self.snapshots.get(&relative_path) {
            return snapshot.clone();
        }
        let text = read_case_text_file_path(path, line_number, relative, &relative_path, context);
        self.snapshots.insert(relative_path, text.clone());
        text
    }
}

fn parse_case_text_reference(
    path: &Path,
    value: &ManifestValue<'_>,
    section: &str,
    operation: &str,
) -> CaseTextReference {
    if !value.is_string() {
        manifest_error(
            path,
            value.line(),
            format!("{section} `{operation}` must be a string case file reference"),
        );
    }
    CaseTextReference {
        line_number: value.line(),
        relative: parse_string(path, value),
    }
}

fn load_jsonrpc_stdin_snapshot(
    manifest_path: &Path,
    line_number: usize,
    relative: &str,
    case_text_cache: &mut CaseTextCache,
    workspace_file_uri_directives: &mut Vec<WorkspaceFileUriDirective>,
) -> String {
    load_jsonrpc_stdin(
        manifest_path,
        line_number,
        relative,
        case_text_cache,
        workspace_file_uri_directives,
    )
}

fn load_jsonrpc_stdin(
    manifest_path: &Path,
    line_number: usize,
    relative: &str,
    case_text_cache: &mut CaseTextCache,
    workspace_file_uri_directives: &mut Vec<WorkspaceFileUriDirective>,
) -> String {
    let text = case_text_cache.read_path(manifest_path, line_number, relative);
    let fixture = parse_json(&text).unwrap_or_else(|error| {
        let message_context = jsonrpc_parse_error_message_context(&text, error.offset)
            .map(|index| format!(" message {index}"))
            .unwrap_or_default();
        manifest_error(
            manifest_path,
            line_number,
            format!("invalid JSON-RPC fixture `{relative}`{message_context}: {error}"),
        )
    });
    let JsonValue::Array(messages) = fixture else {
        manifest_error(
            manifest_path,
            line_number,
            format!("JSON-RPC fixture `{relative}` root must be an array"),
        );
    };

    let mut framed = String::new();
    for (index, mut message) in messages.into_iter().enumerate() {
        let position = format!("$[{index}]");
        let mut context = JsonrpcDirectiveExpansion {
            manifest_path,
            line_number,
            message_index: index,
            case_text_cache,
            workspace_file_uri_directives,
        };
        expand_case_text_directives(
            &mut context,
            &position,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut message,
        );
        validate_jsonrpc_input_message(manifest_path, line_number, index, &message);
        let body = message.to_compact_string();
        let length = body.len();
        framed.push_str(&format!("Content-Length: {length}\r\n\r\n{body}"));
    }
    framed
}

struct JsonrpcDirectiveExpansion<'a> {
    manifest_path: &'a Path,
    line_number: usize,
    message_index: usize,
    case_text_cache: &'a mut CaseTextCache,
    workspace_file_uri_directives: &'a mut Vec<WorkspaceFileUriDirective>,
}

fn expand_case_text_directives(
    context: &mut JsonrpcDirectiveExpansion<'_>,
    position: &str,
    pointer_tokens: &mut Vec<String>,
    pointer_route: &mut Vec<JsonPointerRouteSegment>,
    value: &mut JsonValue,
) {
    match value {
        JsonValue::Array(values) => {
            for (index, value) in values.iter_mut().enumerate() {
                pointer_tokens.push(index.to_string());
                pointer_route.push(JsonPointerRouteSegment::ArrayIndex(index));
                expand_case_text_directives(
                    context,
                    &format!("{position}[{index}]"),
                    pointer_tokens,
                    pointer_route,
                    value,
                );
                pointer_tokens.pop();
                pointer_route.pop();
            }
        }
        JsonValue::Object(entries) => {
            if let Some((_, directive)) =
                entries.iter().find(|(key, _)| key == "$workspace_file_uri")
            {
                if entries.len() != 1 {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$workspace_file_uri` directive object must contain no other members",
                    );
                }
                let JsonValue::String(relative) = directive else {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$workspace_file_uri` directive value must be a string",
                    );
                };
                validate_workspace_file_uri_operand(
                    context.manifest_path,
                    context.line_number,
                    relative,
                );
                context
                    .workspace_file_uri_directives
                    .push(WorkspaceFileUriDirective {
                        message_index: context.message_index,
                        pointer_route: pointer_route.clone(),
                        relative: relative.clone(),
                    });
                *value = JsonValue::String(workspace_file_uri_marker(relative));
                return;
            }
            if let Some((_, directive)) = entries.iter().find(|(key, _)| key == "$case_text") {
                if entries.len() != 1 {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$case_text` directive object must contain no other members",
                    );
                }
                let JsonValue::String(relative) = directive else {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        "`$case_text` directive value must be a string",
                    );
                };
                let replacement = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    context.case_text_cache.read_path(
                        context.manifest_path,
                        context.line_number,
                        relative,
                    )
                }))
                .unwrap_or_else(|panic| {
                    jsonrpc_fixture_error(
                        context.manifest_path,
                        context.line_number,
                        context.message_index,
                        position,
                        &format!("case-text reference failed: {}", panic_message(panic)),
                    )
                });
                *value = JsonValue::String(replacement);
                return;
            }
            let mut seen_keys = BTreeMap::<String, usize>::new();
            for (key, value) in entries {
                let occurrence = *seen_keys.get(key).unwrap_or(&0);
                seen_keys.insert(key.clone(), occurrence + 1);
                pointer_tokens.push(key.clone());
                pointer_route.push(JsonPointerRouteSegment::ObjectMember {
                    key: key.clone(),
                    occurrence,
                });
                expand_case_text_directives(
                    context,
                    &format!("{position}.{}", escape_json_position_key(key)),
                    pointer_tokens,
                    pointer_route,
                    value,
                );
                pointer_tokens.pop();
                pointer_route.pop();
            }
        }
        _ => {}
    }
}

const WORKSPACE_FILE_URI_MARKER: &str = "veln-harness-workspace-file-uri:";

fn workspace_file_uri_marker(relative: &str) -> String {
    let mut marker = WORKSPACE_FILE_URI_MARKER.to_string();
    for byte in relative.bytes() {
        marker.push_str(&format!("{byte:02x}"));
    }
    marker
}

fn materialize_jsonrpc_workspace_file_uri_directives(
    input: &str,
    directives: &[WorkspaceFileUriDirective],
    project_root: &Path,
) -> String {
    if directives.is_empty() {
        return input.to_string();
    }
    let mut messages = decode_lsp_stdout(input)
        .unwrap_or_else(|error| panic!("workspace URI directive input failed to decode: {error}"));
    for directive in directives {
        let Some(message) = messages.get_mut(directive.message_index) else {
            panic!(
                "workspace URI directive references missing message {}",
                directive.message_index
            );
        };
        let target = json_pointer_route_mut(message, &directive.pointer_route)
            .unwrap_or_else(|| panic!("workspace URI directive path was not found"));
        let uri = workspace_file_uri(project_root, &directive.relative)
            .unwrap_or_else(|error| panic!("workspace URI directive failed: {error}"));
        *target = JsonValue::String(uri);
    }
    messages
        .iter()
        .map(|message| lsp_frame(&message.to_compact_string()))
        .collect()
}

fn escape_json_position_key(key: &str) -> String {
    if key
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        key.to_string()
    } else {
        format!(
            "[{}]",
            JsonValue::String(key.to_string()).to_compact_string()
        )
    }
}

fn validate_jsonrpc_input_message(
    manifest_path: &Path,
    line_number: usize,
    index: usize,
    message: &JsonValue,
) {
    let JsonValue::Object(entries) = message else {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}]"),
            "message must be an object",
        );
    };
    let field = |name: &str| {
        entries
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value)
    };
    if field("result").is_some() || field("error").is_some() {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}]"),
            "request or notification must not contain `result` or `error`",
        );
    }
    for name in ["jsonrpc", "method", "id", "params"] {
        let count = entries.iter().filter(|(key, _)| key == name).count();
        if count > 1 {
            jsonrpc_fixture_error(
                manifest_path,
                line_number,
                index,
                &format!("$[{index}].{name}"),
                &format!("`{name}` must not appear more than once"),
            );
        }
    }
    if field("jsonrpc") != Some(&JsonValue::String("2.0".to_string())) {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].jsonrpc"),
            "`jsonrpc` must be the string `2.0`",
        );
    }
    if !matches!(field("method"), Some(JsonValue::String(_))) {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].method"),
            "`method` must be a string",
        );
    }
    if let Some(id) = field("id")
        && !matches!(
            id,
            JsonValue::Null | JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Decimal(_)
        )
    {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].id"),
            "`id` must be a string, number, or null",
        );
    }
    if let Some(params) = field("params")
        && !matches!(
            params,
            JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_)
        )
    {
        jsonrpc_fixture_error(
            manifest_path,
            line_number,
            index,
            &format!("$[{index}].params"),
            "`params` must be an object, array, or null",
        );
    }
}

fn jsonrpc_parse_error_message_context(text: &str, error_offset: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut offset = skip_json_ws(bytes, 0);
    if bytes.get(offset) != Some(&b'[') {
        return None;
    }
    offset += 1;
    let mut index = 0;
    loop {
        offset = skip_json_ws(bytes, offset);
        match bytes.get(offset) {
            Some(b']') => return None,
            Some(_) if error_offset <= offset => return Some(index),
            Some(_) => {}
            None => return Some(index),
        }
        match skip_json_value(bytes, offset, error_offset) {
            Some(next) => offset = skip_json_ws(bytes, next),
            None => return Some(index),
        }
        match bytes.get(offset) {
            Some(b',') => {
                offset += 1;
                index += 1;
            }
            Some(b']') => return None,
            Some(_) | None => return Some(index),
        }
    }
}

fn skip_json_ws(bytes: &[u8], mut offset: usize) -> usize {
    while matches!(bytes.get(offset), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        offset += 1;
    }
    offset
}

fn skip_json_value(bytes: &[u8], offset: usize, stop: usize) -> Option<usize> {
    let offset = skip_json_ws(bytes, offset);
    match bytes.get(offset)? {
        b'"' => skip_json_string(bytes, offset, stop),
        b'{' => skip_json_container(bytes, offset, stop, b'{', b'}'),
        b'[' => skip_json_container(bytes, offset, stop, b'[', b']'),
        b'-' | b'0'..=b'9' => skip_json_number(bytes, offset),
        b'n' if bytes.get(offset..offset + 4) == Some(b"null") => Some(offset + 4),
        b't' if bytes.get(offset..offset + 4) == Some(b"true") => Some(offset + 4),
        b'f' if bytes.get(offset..offset + 5) == Some(b"false") => Some(offset + 5),
        _ => None,
    }
}

fn skip_json_container(
    bytes: &[u8],
    mut offset: usize,
    stop: usize,
    open: u8,
    close: u8,
) -> Option<usize> {
    if bytes.get(offset) != Some(&open) {
        return None;
    }
    let mut depth = 0usize;
    while offset < bytes.len() {
        if offset >= stop {
            return None;
        }
        match bytes[offset] {
            byte if byte == open => {
                depth += 1;
                offset += 1;
            }
            byte if byte == close => {
                depth -= 1;
                offset += 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            b'"' => offset = skip_json_string(bytes, offset, stop)?,
            _ => offset += 1,
        }
    }
    None
}

fn skip_json_string(bytes: &[u8], mut offset: usize, stop: usize) -> Option<usize> {
    if bytes.get(offset) != Some(&b'"') {
        return None;
    }
    offset += 1;
    while offset < bytes.len() {
        if offset >= stop {
            return None;
        }
        match bytes[offset] {
            b'"' => return Some(offset + 1),
            b'\\' => offset += 2,
            _ => offset += 1,
        }
    }
    None
}

fn skip_json_number(bytes: &[u8], mut offset: usize) -> Option<usize> {
    if bytes.get(offset) == Some(&b'-') {
        offset += 1;
    }
    match bytes.get(offset)? {
        b'0' => offset += 1,
        b'1'..=b'9' => {
            offset += 1;
            while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
                offset += 1;
            }
        }
        _ => return None,
    }
    if bytes.get(offset) == Some(&b'.') {
        offset += 1;
        if !matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            offset += 1;
        }
    }
    if matches!(bytes.get(offset), Some(b'e' | b'E')) {
        offset += 1;
        if matches!(bytes.get(offset), Some(b'+' | b'-')) {
            offset += 1;
        }
        if !matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(bytes.get(offset), Some(b'0'..=b'9')) {
            offset += 1;
        }
    }
    Some(offset)
}

fn jsonrpc_fixture_error(
    manifest_path: &Path,
    line_number: usize,
    message_index: usize,
    position: &str,
    fact: &str,
) -> ! {
    manifest_error(
        manifest_path,
        line_number,
        format!("JSON-RPC fixture message {message_index} at {position}: {fact}"),
    )
}

fn read_case_text_file_path(
    path: &Path,
    line_number: usize,
    relative: &str,
    relative_path: &Path,
    context: Option<&str>,
) -> String {
    let base = path.parent().unwrap_or_else(|| Path::new(""));
    let resolved =
        resolve_case_file_reference(path, line_number, base, relative, relative_path, context);
    fs::read_to_string(&resolved).unwrap_or_else(|error| {
        manifest_error(
            path,
            line_number,
            format!(
                "failed to read case file `{relative}`{} as UTF-8: {error}",
                case_file_error_context(context)
            ),
        )
    })
}

fn validate_case_file_reference(
    path: &Path,
    line_number: usize,
    relative: &str,
    context: Option<&str>,
) -> PathBuf {
    if relative.is_empty()
        || relative.starts_with('/')
        || relative.starts_with('\\')
        || relative.contains('\\')
        || relative
            .split('/')
            .any(|component| !is_portable_case_file_component(component))
    {
        manifest_error(
            path,
            line_number,
            format!(
                "case file reference `{relative}`{} must use portable relative components",
                case_file_error_context(context)
            ),
        );
    }
    PathBuf::from(relative)
}

fn case_file_error_context(context: Option<&str>) -> String {
    context
        .map(|context| format!(" for {context}"))
        .unwrap_or_default()
}

fn is_portable_case_file_component(component: &str) -> bool {
    if component.is_empty() || component == "." || component == ".." || component.ends_with('.') {
        return false;
    }
    if !component
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return false;
    }
    let stem = component.split('.').next().unwrap_or(component);
    !matches_reserved_windows_stem(stem)
}

fn matches_reserved_windows_stem(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || upper.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || upper.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
}

fn resolve_case_file_reference(
    manifest_path: &Path,
    line_number: usize,
    base: &Path,
    relative: &str,
    relative_path: &Path,
    context: Option<&str>,
) -> PathBuf {
    let mut current = base.to_path_buf();
    let mut traversed = PathBuf::new();
    let component_count = relative_path.components().count();
    for (index, component) in relative_path.components().enumerate() {
        let name = component.as_os_str();
        if !directory_contains_exact_entry(&current, name) {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} must match fixture entry spelling exactly",
                    case_file_error_context(context)
                ),
            );
        }
        current.push(name);
        traversed.push(name);
        let metadata = fs::symlink_metadata(&current).unwrap_or_else(|error| {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "failed to inspect case file `{relative}`{}: {error}",
                    case_file_error_context(context)
                ),
            )
        });
        if is_link_like_metadata(&metadata) {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} must not traverse a link or reparse point",
                    case_file_error_context(context)
                ),
            );
        }
        let final_component = index + 1 == component_count;
        if final_component {
            if !metadata.is_file() {
                manifest_error(
                    manifest_path,
                    line_number,
                    format!(
                        "case file `{relative}`{} must be a regular file",
                        case_file_error_context(context)
                    ),
                );
            }
        } else if !metadata.is_dir() {
            manifest_error(
                manifest_path,
                line_number,
                format!(
                    "case file `{relative}`{} component `{}` must be a directory",
                    case_file_error_context(context),
                    traversed.display()
                ),
            );
        }
    }
    current
}

fn directory_contains_exact_entry(dir: &Path, name: &std::ffi::OsStr) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name() == name)
}

#[cfg(unix)]
fn is_link_like_metadata(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn is_link_like_metadata(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(any(unix, windows)))]
fn is_link_like_metadata(_metadata: &fs::Metadata) -> bool {
    false
}

fn parse_manifest_json_value(path: &Path, value: &ManifestValue<'_>) -> JsonValue {
    parse_manifest_json_value_allow_decimal(path, value)
}

fn parse_manifest_mcp_json_value(path: &Path, value: &ManifestValue<'_>) -> JsonValue {
    parse_manifest_json_value_allow_decimal(path, value)
}

fn parse_manifest_json_value_allow_decimal(path: &Path, value: &ManifestValue<'_>) -> JsonValue {
    if value.is_string() {
        JsonValue::String(parse_string(path, value))
    } else if value.raw() == "true" {
        JsonValue::Bool(true)
    } else if value.raw() == "false" {
        JsonValue::Bool(false)
    } else if value.raw() == "null" {
        JsonValue::Null
    } else if value.raw().starts_with('[') || value.raw().starts_with('{') {
        parse_json(value.raw()).unwrap_or_else(|error| {
            if value.is_unterminated() && error.missing_closing_delimiter {
                value.report_unterminated(path);
            }
            manifest_error(
                path,
                value.json_error_line(error.offset),
                format!("invalid json assertion value: {error}"),
            )
        })
    } else {
        parse_json(value.raw())
            .unwrap_or_else(|_| manifest_error(path, value.line(), "expected JSON value"))
    }
}

fn is_json_integer_token(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let mut index = 0;
    if matches!(bytes.first(), Some(b'-')) {
        index = 1;
    }
    let Some(first) = bytes.get(index) else {
        return false;
    };
    match first {
        b'0' => index += 1,
        b'1'..=b'9' => {
            index += 1;
            while matches!(bytes.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }
        _ => return false,
    }
    index == bytes.len()
}

fn parse_binary_fixture_hex(path: &Path, value: &ManifestValue<'_>) -> BinaryFixtureBytes {
    let line_number = value.line();
    let hex = parse_string(path, value);
    let bytes = decode_lowercase_hex(path, line_number, &hex);
    BinaryFixtureBytes { hex, bytes }
}

fn parse_binary_fixture_hex_array(
    path: &Path,
    value: &ManifestValue<'_>,
) -> Vec<BinaryFixtureBytes> {
    let line_number = value.line();
    parse_string_array(path, value)
        .into_iter()
        .map(|hex| {
            let bytes = decode_lowercase_hex(path, line_number, &hex);
            BinaryFixtureBytes { hex, bytes }
        })
        .collect()
}

fn decode_lowercase_hex(path: &Path, line_number: usize, hex: &str) -> Vec<u8> {
    let bytes = hex.as_bytes();
    if !bytes.len().is_multiple_of(2) {
        manifest_error(
            path,
            line_number,
            "expected complete lowercase hex byte pairs",
        );
    }

    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = lowercase_hex_nibble(pair[0])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        let low = lowercase_hex_nibble(pair[1])
            .unwrap_or_else(|| manifest_error(path, line_number, "expected lowercase hex"));
        decoded.push((high << 4) | low);
    }
    decoded
}

fn lowercase_hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_bool(path: &Path, value: &ManifestValue<'_>) -> bool {
    match value.raw() {
        "true" => true,
        "false" => false,
        _ => manifest_error(path, value.line(), "expected bool"),
    }
}

fn parse_source_error_expectation(
    path: &Path,
    value: &ManifestValue<'_>,
) -> SourceErrorExpectation {
    let line_number = value.line();
    let value = parse_string(path, value);
    match value.as_str() {
        "expected" => SourceErrorExpectation::Expected,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown source error expectation `{value}`"),
        ),
    }
}

fn parse_skip_platform(path: &Path, line_number: usize, value: &str) -> SkipPlatform {
    match value {
        "unix" => SkipPlatform::Unix,
        "windows" => SkipPlatform::Windows,
        "macos" => SkipPlatform::Macos,
        "linux" => SkipPlatform::Linux,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown skip platform `{value}`"),
        ),
    }
}

fn parse_tool_availability(path: &Path, value: &ManifestValue<'_>) -> ToolAvailability {
    let line_number = value.line();
    let value = parse_string(path, value);
    match value.as_str() {
        "missing" => ToolAvailability::Missing,
        "fake-success" => ToolAvailability::FakeSuccess,
        "fake-git-rev-parse" => ToolAvailability::FakeGitRevParse,
        "real" => ToolAvailability::Real,
        _ => manifest_error(
            path,
            line_number,
            format!("unknown tool availability `{value}`"),
        ),
    }
}

fn parse_string_array(path: &Path, value: &ManifestValue<'_>) -> Vec<String> {
    value.parse_string_array(path)
}

fn parse_string(path: &Path, value: &ManifestValue<'_>) -> String {
    value.parse_string(path)
}

fn parse_string_with_context(path: &Path, value: &ManifestValue<'_>, context: &str) -> String {
    if !value.is_string() {
        manifest_error(path, value.line(), format!("{context}: expected string"));
    }
    parse_string(path, value)
}

fn parse_i32(path: &Path, value: &ManifestValue<'_>) -> i32 {
    value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected i32"))
}

fn parse_i64(path: &Path, value: &ManifestValue<'_>) -> i64 {
    value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected integer"))
}

fn parse_positive_usize(path: &Path, value: &ManifestValue<'_>) -> usize {
    let parsed = value
        .raw()
        .parse()
        .unwrap_or_else(|_| manifest_error(path, value.line(), "expected positive integer"));
    if parsed == 0 {
        manifest_error(path, value.line(), "expected positive integer");
    }
    parsed
}

fn parse_nonnegative_usize(path: &Path, value: &ManifestValue<'_>) -> usize {
    parse_nonnegative_usize_raw_with_context(path, value.line(), value.raw(), None)
}

fn parse_nonnegative_usize_with_context(
    path: &Path,
    value: &ManifestValue<'_>,
    context: &str,
) -> usize {
    parse_nonnegative_usize_raw_with_context(path, value.line(), value.raw(), Some(context))
}

fn parse_nonnegative_usize_raw_with_context(
    path: &Path,
    line_number: usize,
    raw: &str,
    context: Option<&str>,
) -> usize {
    if raw.starts_with('-') && is_json_integer_token(raw) {
        let message = "expected non-negative integer";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message);
    }
    if !is_json_integer_token(raw) {
        let message = "expected integer";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message);
    }
    raw.parse::<usize>().unwrap_or_else(|_| {
        let message = "expected non-negative integer within range";
        let message = match context {
            Some(context) => format!("{context}: {message}"),
            None => message.to_string(),
        };
        manifest_error(path, line_number, message)
    })
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}

fn stream_text(bytes: Vec<u8>, context: &CaseRunContext<'_>, stream: &str) -> String {
    String::from_utf8(bytes)
        .unwrap_or_else(|error| panic!("{}: {stream} should be UTF-8: {error}", context.label()))
}

fn assert_stream(
    context: &CaseRunContext<'_>,
    name: &str,
    expectation: &StreamExpectation,
    actual: &str,
) {
    match expectation.format {
        Some(StreamFormat::Empty) => assert_eq!(
            actual,
            "",
            "{}: expected {name} to be empty, got:\n{actual}",
            context.label()
        ),
        Some(StreamFormat::Text) | Some(StreamFormat::Json) | None => {}
    }

    if let Some(expected) = &expectation.equals {
        assert_eq!(
            actual,
            expected,
            "{}: expected {name} to equal configured text",
            context.label()
        );
    }
    for fragment in &expectation.contains {
        assert_contains_fragment(context, name, actual, fragment);
    }
    for fragment in &expectation.not_contains {
        assert!(
            !actual.contains(fragment),
            "{}: expected {name} not to contain `{fragment}`, got:\n{actual}",
            context.label()
        );
    }
}

fn assert_help_section(
    context: &CaseRunContext<'_>,
    surface: &str,
    stream: &str,
    section: &str,
    fragments: &[String],
) {
    if fragments.is_empty() {
        return;
    }
    assert_contains_fragment(context, surface, stream, &format!("{section}:\n"));
    for fragment in fragments {
        assert_contains_fragment(context, surface, stream, fragment);
    }
}

fn assert_contains_fragment(
    context: &CaseRunContext<'_>,
    surface: &str,
    actual: &str,
    fragment: &str,
) {
    assert!(
        actual.contains(fragment),
        "{}: expected {surface} to contain `{fragment}`, got:\n{actual}",
        context.label()
    );
}

fn assert_binary_fixture(
    context: &CaseRunContext<'_>,
    stdout: &str,
    fixture: &BinaryFixtureExpectation,
) {
    let expected = expected_binary_fixture_line(fixture);
    assert!(
        stdout.lines().any(|line| line == expected),
        "{}: expected binary fixture line `{expected}`, got:\n{stdout}",
        context.label()
    );
}

fn assert_output_chunk_list(
    context: &CaseRunContext<'_>,
    stdout: &str,
    chunks: &OutputChunkListExpectation,
) {
    let expected = expected_output_chunk_list_lines(chunks);
    let actual = stdout.lines().collect::<Vec<_>>();
    let matches = actual.windows(expected.len()).any(|window| {
        window
            .iter()
            .zip(&expected)
            .all(|(actual, expected)| *actual == expected.as_str())
    });
    assert!(
        matches,
        "{}: expected output chunk list:\n{}\ngot:\n{stdout}",
        context.label(),
        expected.join("\n")
    );
}

fn expected_binary_fixture_line(fixture: &BinaryFixtureExpectation) -> String {
    if let Some(bytes) = &fixture.bytes {
        let consumed = fixture
            .consumed
            .map_or_else(|| "none".to_string(), |value| value.to_string());
        let mut line = format!(
            "fixture {} hex {} count {} consumed {}",
            fixture.name,
            bytes.hex,
            bytes.bytes.len(),
            consumed
        );
        if let Some(byte_diagnostic) = &fixture.byte_diagnostic {
            if let Some(diagnostic_id) = &byte_diagnostic.diagnostic_id {
                line.push_str(&format!(" diagnostic {diagnostic_id}"));
            }
            if let Some(byte_offset) = byte_diagnostic.byte_offset {
                line.push_str(&format!(" offset {byte_offset}"));
            }
            if let Some(expected_count) = byte_diagnostic.expected_count {
                line.push_str(&format!(" expected {expected_count}"));
            }
            if let Some(available_count) = byte_diagnostic.available_count {
                line.push_str(&format!(" available {available_count}"));
            }
            if let Some(readiness) = &byte_diagnostic.readiness {
                line.push_str(&format!(" readiness {readiness}"));
            }
            if let Some(field_path) = &byte_diagnostic.field_path {
                line.push_str(&format!(" field_path {}", field_path.to_compact_string()));
            }
        }
        return line;
    }

    format!(
        "fixture {} error {}",
        fixture.name,
        fixture
            .error
            .as_deref()
            .expect("binary fixture error should be present")
    )
}

fn expected_output_chunk_list_lines(chunks: &OutputChunkListExpectation) -> Vec<String> {
    let chunk_values = chunks
        .chunks
        .as_deref()
        .expect("output chunk list chunks should be present");
    let mut lines = vec![format!(
        "output_chunk_list {} count {}",
        chunks.name,
        chunk_values.len()
    )];
    for (index, chunk) in chunk_values.iter().enumerate() {
        lines.push(format!(
            "output_chunk {} index {} hex \"{}\" count {}",
            chunks.name,
            index,
            chunk.hex,
            chunk.bytes.len()
        ));
    }
    lines
}

fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}

#[test]
fn manifest_tools_parse_controlled_java_availability() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "fake-success"
git = "fake-git-rev-parse"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(!manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::FakeSuccess));
    assert_eq!(manifest.tools.git, Some(ToolAvailability::FakeGitRevParse));

    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "real"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::Real));
}

#[test]
fn toolchain_inventory_discovers_both_roots_with_stable_root_qualified_order() {
    let root = test_temp_root("inventory-order");
    for case_dir in ["a/zeta", "b/alpha", "a/alpha"] {
        fs::create_dir_all(root.join(case_dir)).expect("case directory should be created");
        fs::write(
            root.join(case_dir).join("case.toml"),
            "command = [\"check\"]\nexit = 0\n",
        )
        .expect("case manifest should be written");
    }

    let preflight = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[
            test_discovery_root("root-a", "a"),
            test_discovery_root("root-b", "b"),
        ],
    )
    .expect("inventory should be discovered");

    assert_eq!(
        preflight
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        ["root-a/alpha", "root-a/zeta", "root-b/alpha"]
    );
    assert_eq!(
        preflight
            .cases
            .iter()
            .map(|case| case.manifest_relative.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>(),
        ["a/alpha", "a/zeta", "b/alpha"]
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
fn toolchain_inventory_rejects_overlapping_root_and_manifest_boundaries() {
    let root = test_temp_root("inventory-boundaries");
    fs::create_dir_all(root.join("outer/nested")).expect("directories should be created");
    fs::write(
        root.join("outer/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("outer manifest should be written");
    fs::write(
        root.join("outer/nested/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("nested manifest should be written");

    let overlap = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[
            test_discovery_root("outer", "outer"),
            test_discovery_root("nested", "outer/nested"),
        ],
    )
    .expect_err("overlapping roots should fail");
    assert!(overlap.contains("configured discovery roots overlap"));

    let nested = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("outer", "outer")],
    )
    .expect_err("root and nested manifests should fail");
    assert!(nested.contains("root-level case.toml"));
    assert!(nested.contains("nested case.toml"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
fn toolchain_inventory_reports_mixed_entry_failures_in_stable_path_order() {
    use std::os::unix::fs::symlink;

    fn write_inventory(root: &Path, entries: &[&str]) {
        fs::create_dir_all(root.join("cases/nested"))
            .expect("inventory directories should be created");
        for entry in entries {
            match *entry {
                "case.toml" => fs::write(
                    root.join("cases/case.toml"),
                    "command = [\"check\"]\nexit = 0\n",
                )
                .expect("root manifest should be written"),
                "link" => symlink("nested", root.join("cases/link"))
                    .expect("directory link should be created"),
                "nested/case.toml" => fs::write(
                    root.join("cases/nested/case.toml"),
                    "command = [\"check\"]\nexit = 0\n",
                )
                .expect("nested manifest should be written"),
                entry => panic!("unexpected fixture entry {entry}"),
            }
        }
    }

    let first = test_temp_root("inventory-stable-entry-errors-a");
    let second = test_temp_root("inventory-stable-entry-errors-b");
    write_inventory(&first, &["link", "nested/case.toml", "case.toml"]);
    write_inventory(&second, &["case.toml", "nested/case.toml", "link"]);

    let first_error = toolchain_case_inventory::run_preflight_with_roots(
        &first,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("mixed invalid entries should fail discovery");
    let second_error = toolchain_case_inventory::run_preflight_with_roots(
        &second,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("mixed invalid entries should fail discovery");

    assert_eq!(first_error, second_error);
    assert!(first_error.contains("toolchain case preflight found 4 problem(s)"));
    assert!(first_error.contains("cases/link: replace the link or reparse point"));
    assert!(first_error.contains("cases: remove root-level case.toml"));
    assert!(
        first_error.contains("cases/nested/case.toml: nested case.toml is below cases/case.toml")
    );

    fs::remove_dir_all(first).expect("first inventory root should be removed");
    fs::remove_dir_all(second).expect("second inventory root should be removed");
}

#[cfg(unix)]
#[test]
fn toolchain_inventory_rejects_links_without_following_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-links");
    fs::create_dir_all(root.join("cases/ordinary")).expect("directories should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    symlink("ordinary", root.join("cases/link")).expect("symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("link should fail discovery");
    assert!(error.contains("replace the link or reparse point"));
    assert!(error.contains("cases/link"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
fn toolchain_inventory_rejects_root_links_without_following_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-root-links");
    fs::create_dir_all(root.join("external/hidden")).expect("target case directory should exist");
    fs::write(
        root.join("external/hidden/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("target case manifest should be written");
    symlink("external", root.join("linked-root")).expect("root symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("root symlink should fail discovery before traversal");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("hidden"),
        "discovery should not resolve root symlink targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
fn toolchain_inventory_rejects_broken_root_links_without_resolving_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-broken-root-link");
    symlink("missing-target", root.join("linked-root"))
        .expect("broken root symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("broken root symlink should fail as a link-like root");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("missing-target"),
        "discovery should not resolve broken root link targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
fn toolchain_inventory_rejects_broken_file_links_and_link_cycles() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-link-kinds");
    fs::create_dir_all(root.join("cases/ordinary")).expect("case directory should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    symlink("missing-target", root.join("cases/broken-link"))
        .expect("broken symlink should be created");
    fs::write(root.join("target-file"), "fixture").expect("target file should be written");
    symlink("../target-file", root.join("cases/file-link"))
        .expect("file symlink should be created");
    symlink("cycle-b", root.join("cases/cycle-a")).expect("first cycle link should be created");
    symlink("cycle-a", root.join("cases/cycle-b")).expect("second cycle link should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("link-like entries should fail discovery before following them");

    assert!(error.contains("cases/broken-link: replace the link or reparse point"));
    assert!(error.contains("cases/file-link: replace the link or reparse point"));
    assert!(error.contains("cases/cycle-a: replace the link or reparse point"));
    assert!(error.contains("cases/cycle-b: replace the link or reparse point"));
    assert!(
        !error.contains("missing-target"),
        "discovery should not resolve broken link targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(windows)]
#[test]
fn toolchain_inventory_rejects_windows_reparse_point_roots() {
    use std::os::windows::fs::symlink_dir;

    let root = test_temp_root("inventory-windows-root-reparse");
    fs::create_dir_all(root.join("external/hidden")).expect("target case directory should exist");
    fs::write(
        root.join("external/hidden/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("target case manifest should be written");
    symlink_dir(root.join("external"), root.join("linked-root"))
        .expect("root reparse point should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("root reparse point should fail discovery before traversal");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("hidden"),
        "discovery should not resolve root reparse point targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(windows)]
#[test]
fn toolchain_inventory_rejects_windows_reparse_point_files_and_directories() {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    let root = test_temp_root("inventory-windows-reparse");
    fs::create_dir_all(root.join("cases/ordinary")).expect("case directory should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    fs::write(root.join("target-file"), "fixture").expect("target file should be written");
    symlink_file(root.join("target-file"), root.join("cases/file-link"))
        .expect("file reparse point should be created");
    symlink_dir(
        root.join("cases/ordinary"),
        root.join("cases/directory-link"),
    )
    .expect("directory reparse point should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("Windows reparse points should fail discovery before traversal");

    assert!(error.contains("cases/directory-link: replace the link or reparse point"));
    assert!(error.contains("cases/file-link: replace the link or reparse point"));
    assert!(
        !error.contains("target-file"),
        "discovery should not resolve reparse point targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
fn toolchain_inventory_parity_reports_stale_generated_cases() {
    let error = toolchain_case_inventory::compare_generated_inventory_with_policy(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &[],
        false,
    )
    .expect_err("empty generated inventory should be stale");

    assert!(error.contains("rebuild the toolchain harness"));
    assert!(error.contains("case manifest was added after test generation"));
}

#[test]
fn policy_preflight_failure_prevents_generated_test_module_creation() {
    let root = test_temp_root("policy-generation-block");
    fs::create_dir_all(root.join("cases/blocked")).expect("case directory should be created");
    fs::write(
        root.join("cases/blocked/case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
    )
    .expect("blocked manifest should be written");

    let error = toolchain_case_inventory::generated_toolchain_tests_from_preflight(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail before generating tests");

    assert!(error.contains("toolchain case preflight found 1 problem(s)"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(!error.contains("generated_toolchain_cases"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
fn manifest_policy_reports_encoded_line_breaks_in_toml_and_json_strings() {
    let source = r#"
command = ["check"]
stdin = ["\n", '\n', {"json":"\u000A", "nested":["\\r"]}]
physical = """
line
break"""
# "\n"
exit = 0
"#;

    let findings = manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source);
    assert_eq!(
        findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("stdin", "escape-produced-line-break", "escape-produced LF"),
            ("stdin", "decoded-line-break-spelling", "\\n"),
            ("stdin", "escape-produced-line-break", "escape-produced LF"),
            ("stdin", "decoded-line-break-spelling", "\\r"),
        ]
    );
}

#[test]
fn manifest_policy_reports_cr_unicode_and_obfuscated_spellings_in_order() {
    let source = r#"
command = ["check"]
cr = "\r"
unicode_lf = "\u000a"
unicode_cr = "\U0000000D"
literal = '\u000A'
assembled = "\u005Cn"
even_backslashes = "\\n"
odd_backslashes = "\\\n"
json = {"\u005Cr":"\\U0000000a"}
exit = 0
"#;

    let findings = manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source);

    assert_eq!(
        findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("cr", "escape-produced-line-break", "escape-produced CR"),
            (
                "unicode_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "unicode_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            ("literal", "decoded-line-break-spelling", "\\u000A"),
            ("assembled", "decoded-line-break-spelling", "\\n"),
            ("even_backslashes", "decoded-line-break-spelling", "\\n"),
            (
                "odd_backslashes",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            ("json", "decoded-line-break-spelling", "\\r"),
            ("json", "decoded-line-break-spelling", "\\U0000000a"),
        ]
    );
}

#[test]
fn manifest_policy_accepts_json_solidus_escape_in_keys_and_values() {
    let object_source = r#"
command = ["check"]
json = {"https:\/\/x":"https:\/\/v", "nested":{"slash\/key":"value\/ok", "line":"\u000A", "spelled":"\\r"}}
exit = 0
"#;
    let array_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = ["https:\/\/x", {"slash\/key":"value\/ok"}, "\u000D", "\\n"]
"#;

    let manifest = parse_manifest(Path::new("case.toml"), array_source);
    let Some(ValueAssertionOperation::Equals(expected)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(
        expected.to_compact_string(),
        r#"["https://x",{"slash/key":"value/ok"},"\r","\\n"]"#
    );

    let object_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), object_source);
    assert_eq!(object_scan.error, None);
    assert_eq!(
        object_scan
            .findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("json", "escape-produced-line-break", "escape-produced LF"),
            ("json", "decoded-line-break-spelling", "\\r"),
        ]
    );

    let array_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), array_source);
    assert_eq!(array_scan.error, None);
    assert_eq!(
        array_scan
            .findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (
                "[[json_assert]].equals",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "[[json_assert]].equals",
                "decoded-line-break-spelling",
                "\\n"
            ),
        ]
    );
}

#[test]
fn manifest_policy_scan_reports_invalid_json_escapes_in_object_and_array_roots() {
    let object_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = {"bad":"\q"}
"#;
    let array_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = ["valid\/solidus", {"bad":"\q"}]
"#;

    let object_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), object_source);
    assert_eq!(
        object_scan.error.as_deref(),
        Some("unsupported JSON string escape `q`")
    );
    assert!(object_scan.findings.is_empty());

    let array_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), array_source);
    assert_eq!(
        array_scan.error.as_deref(),
        Some("unsupported JSON string escape `q`")
    );
    assert!(array_scan.findings.is_empty());
}

#[test]
fn policy_preflight_invalid_json_escape_prevents_generated_test_module_creation() {
    let root = test_temp_root("policy-invalid-json-escape-generation-block");
    fs::create_dir_all(root.join("cases/blocked")).expect("case directory should be created");
    fs::write(
        root.join("cases/blocked/case.toml"),
        r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = {"bad":"\q"}
"#,
    )
    .expect("blocked manifest should be written");

    let error = toolchain_case_inventory::generated_toolchain_tests_from_preflight(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail before generating tests");

    assert!(error.contains("toolchain case preflight found 1 problem(s)"));
    assert!(error.contains("cases/blocked: manifest policy scan failed"));
    assert!(error.contains("unsupported JSON string escape `q`"));
    assert!(!error.contains("generated_toolchain_cases"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
fn manifest_policy_reports_lowercase_and_uppercase_unicode_line_break_matrix() {
    let source = r#"
command = ["check"]
lower_lf = "\u000a"
upper_lf = "\u000A"
lower_cr = "\u000d"
upper_cr = "\u000D"
wide_lower_lf = "\U0000000a"
wide_upper_lf = "\U0000000A"
wide_lower_cr = "\U0000000d"
wide_upper_cr = "\U0000000D"
literal_lower_lf = '\u000a'
literal_upper_lf = '\u000A'
literal_lower_cr = '\u000d'
literal_upper_cr = '\u000D'
literal_wide_lower_lf = '\U0000000a'
literal_wide_upper_lf = '\U0000000A'
literal_wide_lower_cr = '\U0000000d'
literal_wide_upper_cr = '\U0000000D'
exit = 0
"#;

    let findings = manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source);

    assert_eq!(
        findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (
                "lower_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "upper_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "lower_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "upper_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "wide_lower_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "wide_upper_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "wide_lower_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "wide_upper_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            ("literal_lower_lf", "decoded-line-break-spelling", "\\u000a"),
            ("literal_upper_lf", "decoded-line-break-spelling", "\\u000A"),
            ("literal_lower_cr", "decoded-line-break-spelling", "\\u000d"),
            ("literal_upper_cr", "decoded-line-break-spelling", "\\u000D"),
            (
                "literal_wide_lower_lf",
                "decoded-line-break-spelling",
                "\\U0000000a"
            ),
            (
                "literal_wide_upper_lf",
                "decoded-line-break-spelling",
                "\\U0000000A"
            ),
            (
                "literal_wide_lower_cr",
                "decoded-line-break-spelling",
                "\\U0000000d"
            ),
            (
                "literal_wide_upper_cr",
                "decoded-line-break-spelling",
                "\\U0000000D"
            ),
        ]
    );
}

#[test]
fn manifest_policy_accepts_physical_comments_and_token_boundaries() {
    let source = r#"
command = ["check"]
stdin = """
first
second"""
split = ['\', 'n']
# "\r"
exit = 0
"#;

    assert!(manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source).is_empty());
}

#[test]
fn manifest_policy_scan_keeps_findings_before_unterminated_string_boundary() {
    let source = "command = [\"check\"]\nstdin = \"\\n\"\nlate = \"unterminated";
    let scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), source);

    assert_eq!(
        scan.findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [("stdin", "escape-produced-line-break", "escape-produced LF")]
    );
    assert_eq!(scan.error.as_deref(), Some("unterminated manifest string"));
}

#[test]
fn manifest_policy_scan_keeps_findings_before_lone_cr_boundary() {
    let source = "command = [\"check\"]\nstdin = \"\\r\"\rbad = true\n";
    let scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), source);

    assert_eq!(
        scan.findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [("stdin", "escape-produced-line-break", "escape-produced CR")]
    );
    assert_eq!(
        scan.error.as_deref(),
        Some("lone carriage return in manifest")
    );
}

#[test]
fn toolchain_policy_preflight_aggregates_skipped_unavailable_and_lexical_cases() {
    let root = test_temp_root("policy-aggregation");
    for case_dir in [
        "cases/lone-cr",
        "cases/malformed",
        "cases/skipped",
        "cases/unavailable",
    ] {
        fs::create_dir_all(root.join(case_dir)).expect("case directory should be created");
    }
    fs::write(
        root.join("cases/skipped/case.toml"),
        r#"
command = ["check"]
stdin = "\n"
exit = 0

[skip]
platforms = ["linux"]
reason = "lifecycle sentinel"
"#,
    )
    .expect("skipped manifest should be written");
    fs::write(
        root.join("cases/unavailable/case.toml"),
        r#"
command = ["check"]
exit = 0

[tools]
java = "missing"

[stdout]
contains = ["\\r"]
"#,
    )
    .expect("unavailable-tool manifest should be written");
    fs::write(
        root.join("cases/malformed/case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nlate = \"unterminated",
    )
    .expect("malformed manifest should be written");
    fs::write(
        root.join("cases/lone-cr/case.toml"),
        b"command = [\"check\"]\nstdin = \"\\r\"\rbad = true\n",
    )
    .expect("lone CR manifest should be written");

    let error = toolchain_case_inventory::run_preflight_with_roots_and_policy(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail");

    assert!(error.contains("toolchain case preflight found 6 problem(s) affecting 4 manifest(s)"));
    assert!(error.contains("cases/lone-cr:"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(error.contains("lone carriage return in manifest"));
    assert!(error.contains("cases/skipped:"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(error.contains("cases/unavailable:"));
    assert!(error.contains("field `[stdout].contains` contains decoded-line-break-spelling"));
    assert!(error.contains("cases/malformed: manifest policy scan failed"));
    assert!(error.contains("cases/malformed:2:"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(error.contains("unterminated manifest string"));
    let lone_cr_index = error
        .find("cases/lone-cr")
        .expect("lone CR case should be reported");
    let malformed_index = error
        .find("cases/malformed")
        .expect("malformed case should be reported");
    let skipped_index = error
        .find("cases/skipped")
        .expect("skipped case should be reported");
    let unavailable_index = error
        .find("cases/unavailable")
        .expect("unavailable-tool case should be reported");
    assert!(lone_cr_index < malformed_index);
    assert!(malformed_index < skipped_index);
    assert!(skipped_index < unavailable_index);

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
fn toolchain_policy_preflight_reports_stable_detailed_aggregate_findings() {
    fn write_cases(root: &Path, order: &[&str]) {
        fs::create_dir_all(root.join("cases")).expect("case root should be created");
        for name in order {
            let case_dir = root.join("cases").join(name);
            fs::create_dir_all(&case_dir).expect("case directory should be created");
            let manifest = match *name {
                "alpha" => {
                    "command = [\"check\"]\nstdin = [\"\\n\", \"\\r\"]\nexit = 0\n".to_string()
                }
                "beta" => {
                    "command = [\"check\"]\n[stdout]\ncontains = ['\\u000A', '\\U0000000d']\nexit = 0\n".to_string()
                }
                _ => unreachable!(),
            };
            fs::write(case_dir.join("case.toml"), manifest)
                .expect("case manifest should be written");
        }
    }

    let first = test_temp_root("policy-stable-aggregate-a");
    let second = test_temp_root("policy-stable-aggregate-b");
    write_cases(&first, &["beta", "alpha"]);
    write_cases(&second, &["alpha", "beta"]);

    let first_error = toolchain_case_inventory::run_preflight_with_roots(
        &first,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("first policy preflight should fail");
    let second_error = toolchain_case_inventory::run_preflight_with_roots(
        &second,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("second policy preflight should fail");

    assert_eq!(
        first_error, second_error,
        "aggregate policy output should not depend on filesystem entry order"
    );
    assert!(
        first_error.contains("toolchain case preflight found 4 problem(s) affecting 2 manifest(s)")
    );
    assert!(first_error.contains("cases/alpha:2:29-33 field `stdin` contains escape-produced-line-break `escape-produced LF`; use physical multiline text or a sidecar so line structure remains reviewable"));
    assert!(first_error.contains("cases/alpha:2:35-39 field `stdin` contains escape-produced-line-break `escape-produced CR`; use physical multiline text or a sidecar so line structure remains reviewable"));
    assert!(first_error.contains("cases/beta:3:"));
    assert!(
        first_error
            .contains("field `[stdout].contains` contains decoded-line-break-spelling `\\\\u000A`")
    );
    assert!(first_error.contains(
        "field `[stdout].contains` contains decoded-line-break-spelling `\\\\U0000000d`"
    ));
    assert!(
        first_error.contains(
            "use physical multiline text or a sidecar so line structure remains reviewable"
        )
    );

    fs::remove_dir_all(first).expect("first inventory root should be removed");
    fs::remove_dir_all(second).expect("second inventory root should be removed");
}

#[test]
fn synthetic_policy_guard_runs_before_manifest_loading_skip_and_fixtures() {
    let root = test_temp_root("synthetic-policy-guard");
    let case_dir = root.join("synthetic");
    fs::create_dir_all(&case_dir).expect("synthetic case directory should be created");
    fs::write(case_dir.join("fixture.txt"), "fixture").expect("fixture should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check"]
stdin = "\n"
stdin_file = "case-text/missing-sidecar.txt"
exit = 0

[skip]
platforms = ["linux", "macos", "windows"]
reason = "skip evaluation must not run before policy"
"#,
    )
    .expect("synthetic manifest should be written");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("synthetic policy violation should stop the case before setup");
    let message = panic_message(panic);

    assert!(message.contains(
        "synthetic toolchain case violates manifest line-break policy before loading resources"
    ));
    assert!(message.contains("field `stdin` contains escape-produced-line-break"));
    assert!(
        !message.contains("skip evaluation must not run"),
        "skip evaluation should be bypassed by the synthetic policy guard"
    );
    assert!(
        !message.contains("missing-sidecar"),
        "sidecar resource loading should be bypassed by the synthetic policy guard"
    );
    assert!(
        !root.join("command-started").exists(),
        "command execution should be bypassed by the synthetic policy guard"
    );

    fs::remove_dir_all(root).expect("synthetic root should be removed");
}

#[test]
fn synthetic_policy_guard_also_applies_inside_crate_for_non_inventory_cases() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("target/non-inventory-synthetic-policy");
    let _ = fs::remove_dir_all(&case_dir);
    fs::create_dir_all(&case_dir).expect("synthetic crate-local case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
    )
    .expect("synthetic manifest should be written");

    assert!(!is_generated_inventory_member(&case_dir));
    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("crate-local non-inventory case should use the synthetic guard");
    let message = panic_message(panic);
    assert!(message.contains(
        "synthetic toolchain case violates manifest line-break policy before loading resources"
    ));

    fs::remove_dir_all(case_dir).expect("synthetic case should be removed");
}

#[test]
fn generated_inventory_membership_is_exact() {
    let generated = toolchain_case_path(
        GENERATED_TOOLCHAIN_CASES
            .first()
            .expect("generated inventory should not be empty"),
    );
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sibling = manifest_dir.join("tests/toolchain_cases/not-generated");

    assert!(is_generated_inventory_member(&generated));
    assert!(!is_generated_inventory_member(&sibling));
}

#[cfg(unix)]
#[test]
fn fixture_copy_rejects_links_before_command_execution() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("copy-link-reject");
    let case_dir = root.join("synthetic");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        "command = [\"run-command-that-must-not-start\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    fs::write(root.join("target.txt"), "target").expect("link target should be written");
    symlink("../target.txt", case_dir.join("fixture-link.txt")).expect("symlink should be created");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("fixture copy should reject symlinks");
    let message = panic_message(panic);
    assert!(message.contains("replace the link or reparse point with a regular fixture entry"));
    assert!(!message.contains("run-command-that-must-not-start"));

    fs::remove_dir_all(root).expect("copy link root should be removed");
}

#[cfg(unix)]
#[test]
fn runtime_inventory_barrier_failure_blocks_generated_case_lifecycle() {
    use std::os::unix::fs::symlink;

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("target/runtime-barrier-block/stale-generated");
    let _ = fs::remove_dir_all(&case_dir);
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        manifest_dir.join("target/runtime-barrier-target.txt"),
        "target",
    )
    .expect("link target should be written");
    symlink(
        "../runtime-barrier-target.txt",
        case_dir.join("fixture-link.txt"),
    )
    .expect("fixture sentinel link should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run-command-that-must-not-start"]
stdin_file = "case-text/missing-sidecar.txt"
exit = 0

[skip]
platforms = ["linux", "macos", "windows"]
reason = "skip evaluation must not run after stale inventory"
"#,
    )
    .expect("case manifest should be written");

    let relative = case_dir
        .strip_prefix(manifest_dir)
        .expect("stale generated case should be under manifest dir")
        .to_string_lossy()
        .replace('\\', "/");
    assert!(!GENERATED_TOOLCHAIN_CASES.contains(&relative.as_str()));
    let mut stale_generated = GENERATED_TOOLCHAIN_CASES
        .iter()
        .map(|case| (*case).to_string())
        .collect::<Vec<_>>();
    stale_generated.push(relative.clone());
    with_test_generated_toolchain_cases(stale_generated, || {
        assert!(is_generated_inventory_member(&case_dir));
        let panic = std::panic::catch_unwind(|| run_case(&case_dir))
            .expect_err("stale inventory should stop generated case execution");
        let message = panic_message(panic);

        assert!(message.contains("toolchain case preflight found"));
        assert!(message.contains(&format!(
            "{relative}: rebuild the toolchain harness because this generated case manifest is no longer discovered"
        )));
        assert!(
            !message.contains("skip evaluation must not run"),
            "skip evaluation should be bypassed by the runtime barrier"
        );
        assert!(
            !message.contains("replace the link or reparse point"),
            "fixture copying should be bypassed by the runtime barrier"
        );
        assert!(
            !message.contains("missing-sidecar"),
            "resource loading should be bypassed by the runtime barrier"
        );
        assert!(
            !message.contains("run-command-that-must-not-start"),
            "command execution should be bypassed by the runtime barrier"
        );
    });

    fs::remove_dir_all(manifest_dir.join("target/runtime-barrier-block"))
        .expect("runtime root should be removed");
    fs::remove_file(manifest_dir.join("target/runtime-barrier-target.txt"))
        .expect("link target should be removed");
}

#[test]
fn runtime_inventory_barrier_shares_one_concurrent_scan_result() {
    use std::sync::{Arc, Barrier};

    let barrier = Arc::new(RuntimeInventoryBarrier::new());
    let ready = Arc::new(Barrier::new(8));
    let scans = Arc::new(AtomicUsize::new(0));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let barrier = Arc::clone(&barrier);
        let ready = Arc::clone(&ready);
        let scans = Arc::clone(&scans);
        threads.push(thread::spawn(move || {
            ready.wait();
            let panic = std::panic::catch_unwind(|| {
                barrier.check_with(|| {
                    scans.fetch_add(1, Ordering::SeqCst);
                    Err("shared stale inventory result".to_string())
                });
            })
            .expect_err("shared failing scan should panic in every caller");
            panic_message(panic)
        }));
    }

    let messages = threads
        .into_iter()
        .map(|thread| thread.join().expect("barrier thread should complete"))
        .collect::<Vec<_>>();

    assert_eq!(
        scans.load(Ordering::SeqCst),
        1,
        "concurrent generated tests should share one runtime scan"
    );
    assert!(
        messages
            .iter()
            .all(|message| message.contains("shared stale inventory result"))
    );
}

#[test]
fn toolchain_policy_preflight_keeps_reliable_cases_when_discovery_has_errors() {
    let root = test_temp_root("policy-partial-inventory");
    fs::create_dir_all(root.join("primary/rooted/nested"))
        .expect("primary directories should be created");
    fs::create_dir_all(root.join("secondary/readable"))
        .expect("secondary directory should be created");
    fs::write(
        root.join("primary/rooted/case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
    )
    .expect("rooted manifest should be written");
    fs::write(
        root.join("primary/rooted/nested/case.toml"),
        "command = [\"check\"]\nstdin = \"\\r\"\nexit = 0\n",
    )
    .expect("nested manifest should be written");
    fs::write(
        root.join("secondary/readable/case.toml"),
        "command = [\"check\"]\n[stdout]\ncontains = [\"\\\\r\"]\nexit = 0\n",
    )
    .expect("secondary manifest should be written");

    let error = toolchain_case_inventory::run_preflight_with_roots_and_policy(
        &root,
        &[
            test_discovery_root("missing", "missing"),
            test_discovery_root("primary", "primary"),
            test_discovery_root("secondary", "secondary"),
        ],
        true,
    )
    .expect_err("mixed discovery and policy failures should aggregate");

    assert!(error.contains("toolchain case preflight found 4 problem(s)"));
    assert!(error.contains("missing: discovery root must be readable"));
    assert!(error.contains("nested case.toml"));
    assert!(error.contains("primary/rooted:"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(error.contains("secondary/readable:"));
    assert!(error.contains("field `[stdout].contains` contains decoded-line-break-spelling"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

fn test_discovery_root(
    id: &'static str,
    relative: &'static str,
) -> toolchain_case_inventory::DiscoveryRoot {
    toolchain_case_inventory::DiscoveryRoot { id, relative }
}

#[test]
fn example_source_error_guard_requires_explicit_intent() {
    let root = test_temp_root("source-error-guard");
    fs::write(
        root.join("main.veln"),
        "fn main() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("guard source should be written");
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 1
"#,
    );

    let panic = std::panic::catch_unwind(|| {
        manifest.assert_no_unexpected_example_source_errors(
            Path::new("examples/specification/fmt/source-error-guard"),
            &root,
        );
    })
    .expect_err("unexpected source error should fail the guard");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("error[type.mismatch]"));
    assert!(message.contains("prevent unrelated editor errors"));

    fs::remove_dir_all(root).expect("guard root should be removed");
}

#[test]
fn example_source_error_guard_accepts_declared_diagnostic_case() {
    let root = test_temp_root("expected-source-error-guard");
    fs::write(
        root.join("main.veln"),
        "fn main() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("guard source should be written");
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["lsp"]
source_errors = "expected"
exit = 0
"#,
    );

    manifest.assert_no_unexpected_example_source_errors(
        Path::new("examples/specification/lsp/source-error-guard"),
        &root,
    );
    assert_eq!(manifest.source_errors, SourceErrorExpectation::Expected);

    fs::remove_dir_all(root).expect("guard root should be removed");
}

#[test]
fn normal_check_run_and_test_cases_use_command_source_diagnostic_artifacts() {
    for command in ["check", "run", "test"] {
        let manifest = parse_manifest(
            Path::new("case.toml"),
            &format!(
                r#"
command = ["{command}", "main.veln"]
exit = 0
"#
            ),
        );

        assert!(manifest.needs_command_source_error_guard(Path::new(
            "examples/specification/run/artifact-guard"
        )));
        assert!(!manifest.needs_pre_command_source_error_guard(Path::new(
            "examples/specification/run/artifact-guard"
        )));
    }
}

#[test]
fn declared_and_intended_source_error_cases_keep_independent_guard() {
    let expected_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
source_errors = "expected"
exit = 0
"#,
    );
    assert!(
        !expected_manifest.needs_command_source_error_guard(Path::new(
            "examples/specification/run/declared-source-error"
        ))
    );
    assert!(
        expected_manifest.needs_pre_command_source_error_guard(Path::new(
            "examples/specification/run/declared-source-error"
        ))
    );

    let intended_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["check", "main.veln"]
exit = 1
"#,
    );
    assert!(
        !intended_manifest.needs_command_source_error_guard(Path::new(
            "examples/specification/check/intended-source-error"
        ))
    );
    assert!(
        !intended_manifest.needs_pre_command_source_error_guard(Path::new(
            "examples/specification/check/intended-source-error"
        ))
    );

    let run_static_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 1

[stderr]
contains = ["runnable entry retains user-defined effect `main::Audit`"]
"#,
    );
    assert!(
        !run_static_manifest.needs_command_source_error_guard(Path::new(
            "examples/specification/run/intended-static-gate"
        ))
    );
    assert!(
        !run_static_manifest.needs_pre_command_source_error_guard(Path::new(
            "examples/specification/run/intended-static-gate"
        ))
    );
}

#[test]
fn command_source_error_artifacts_are_read_per_run() {
    let root = test_temp_root("source-artifact-runs");
    let first = root.join("first.json");
    let second = root.join("second.json");
    fs::write(
        &first,
        r#"{"diagnostics":[{"id":"type.mismatch","severity":"error","message":"first project","span":{"file":"main.veln","start":{"line":1,"column":2},"end":{"line":1,"column":3}}}]}"#,
    )
    .expect("first artifact should be written");
    fs::write(
        &second,
        r#"{"diagnostics":[{"id":"parse.expected_item","severity":"error","message":"second project","span":{"file":"other.veln","start":{"line":4,"column":5},"end":{"line":4,"column":6}}}]}"#,
    )
    .expect("second artifact should be written");

    let first_context = CaseRunContext {
        case_dir: Path::new("examples/specification/run/artifact-guard"),
        run_number: 1,
    };
    let second_context = CaseRunContext {
        case_dir: Path::new("examples/specification/run/artifact-guard"),
        run_number: 2,
    };
    let first_evidence = CommandSourceDiagnosticEvidence::read(&first_context, &first);
    let second_evidence = CommandSourceDiagnosticEvidence::read(&second_context, &second);

    assert_eq!(first_evidence.error_count, 1);
    assert!(first_evidence.message.contains("main.veln:1:2"));
    assert!(first_evidence.message.contains("first project"));
    assert_eq!(second_evidence.error_count, 1);
    assert!(second_evidence.message.contains("other.veln:4:5"));
    assert!(second_evidence.message.contains("second project"));

    fs::remove_dir_all(root).expect("artifact root should be removed");
}

#[test]
fn command_source_error_guard_rejects_unexpected_command_diagnostics() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["test", "--json", "main.veln"]
exit = 1
"#,
    );
    let evidence = CommandSourceDiagnosticEvidence {
        error_count: 1,
        message: "main.veln:1:1: error[type.mismatch]: wrong type".to_string(),
    };
    let context = CaseRunContext {
        case_dir: Path::new("examples/specification/test/static-gate"),
        run_number: 1,
    };

    let panic = std::panic::catch_unwind(|| {
        manifest.assert_no_unexpected_command_source_errors(&context, &evidence);
    })
    .expect_err("unexpected source error should fail command guard");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("error[type.mismatch]"));
}

#[test]
fn command_artifact_guard_rejects_unselected_check_source_errors() {
    let root = test_temp_root("artifact-unselected-check");
    let case_dir = root.join("examples/specification/check/artifact-unselected-check");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check", "main.veln"]
exit = 0
"#,
    )
    .expect("case manifest should be written");
    fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
        .expect("selected source should be written");
    fs::write(
        case_dir.join("unselected.veln"),
        "fn broken() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("unselected source should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case(&case_dir);
    })
    .expect_err("unselected source error should fail command artifact guard");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("unselected.veln"));
    assert!(message.contains("error[type.mismatch]"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn command_artifact_guard_does_not_reuse_between_copied_projects() {
    let root = test_temp_root("artifact-copied-projects");
    let clean_case_dir = root.join("examples/specification/check/artifact-clean-project");
    let dirty_case_dir = root.join("examples/specification/check/artifact-dirty-project");
    fs::create_dir_all(&clean_case_dir).expect("clean case directory should be created");
    fs::create_dir_all(&dirty_case_dir).expect("dirty case directory should be created");
    for case_dir in [&clean_case_dir, &dirty_case_dir] {
        fs::write(
            case_dir.join("case.toml"),
            r#"
command = ["check", "main.veln"]
exit = 0

[stdout]
contains = ["ok"]
"#,
        )
        .expect("case manifest should be written");
        fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
            .expect("selected source should be written");
    }
    fs::write(
        dirty_case_dir.join("unselected.veln"),
        "fn broken() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("unselected source should be written");

    run_case(&clean_case_dir);
    let panic = std::panic::catch_unwind(|| {
        run_case(&dirty_case_dir);
    })
    .expect_err("dirty copied project should not reuse a clean project's artifact");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("unselected.veln"));
    assert!(message.contains("error[type.mismatch]"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn repeated_command_artifact_guard_uses_each_invocation_artifact() {
    let root = test_temp_root("artifact-repeat-run-case");
    let case_dir = root.join("examples/specification/check/artifact-repeat-run-case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check", "main.veln"]
repeat = 2
exit = 0

[stdout]
contains = ["ok"]
"#,
    )
    .expect("case manifest should be written");
    fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
        .expect("selected source should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case_with_after_invocation(&case_dir, |context, project_root| {
            if context.run_number == 1 {
                fs::write(
                    project_root.join("unselected.veln"),
                    "fn broken() -> Int\n\t\"wrong\"\nend\n",
                )
                .expect("second-run source error should be injected");
            }
        });
    })
    .expect_err("second invocation should read its own command artifact");
    let message = panic_message(panic);
    assert!(message.contains("run 2"));
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("unselected.veln"));
    assert!(message.contains("error[type.mismatch]"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn command_artifact_guard_rejects_unselected_run_source_errors() {
    if !jdk_is_available() {
        eprintln!("skipping command artifact run guard test: requires a real JDK");
        return;
    }

    let root = test_temp_root("artifact-unselected-run");
    let case_dir = root.join("examples/specification/run/artifact-unselected-run");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0
"#,
    )
    .expect("case manifest should be written");
    fs::write(
        case_dir.join("main.veln"),
        "pub fn main() -> ()\n\t()\nend\n",
    )
    .expect("selected source should be written");
    fs::write(
        case_dir.join("unselected.veln"),
        "fn broken() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("unselected source should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case(&case_dir);
    })
    .expect_err("unselected source error should fail command artifact guard");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("unselected.veln"));
    assert!(message.contains("error[type.mismatch]"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn command_artifact_guard_keeps_runtime_failure_distinct_from_source_errors() {
    if !jdk_is_available() {
        eprintln!("skipping command artifact runtime guard test: requires a real JDK");
        return;
    }

    let root = test_temp_root("artifact-runtime-source");
    let case_dir = root.join("examples/specification/test/artifact-runtime-source");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["test", "--json", "main_test.veln"]
exit = 1
"#,
    )
    .expect("case manifest should be written");
    fs::write(
        case_dir.join("main_test.veln"),
        "test rejects() -> ()\n\trequire false\n\t()\nend\n",
    )
    .expect("selected test source should be written");
    fs::write(
        case_dir.join("unselected.veln"),
        "fn broken() -> Int\n\t\"wrong\"\nend\n",
    )
    .expect("unselected source should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case(&case_dir);
    })
    .expect_err("source error should not be accepted as the runtime failure");
    let message = panic_message(panic);
    assert!(message.contains("remove unexpected source error diagnostics"));
    assert!(message.contains("unselected.veln"));
    assert!(message.contains("error[type.mismatch]"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_json_assertions_support_missing_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "error.details.byte_diagnostic.byte_preview"
missing = true
"#,
    );

    let assertion = &manifest.expectations.json_assertions[0];
    assert_eq!(assertion.path, "error.details.byte_diagnostic.byte_preview");
    assert_eq!(assertion.operation, Some(ValueAssertionOperation::Missing));
}

#[test]
fn manifest_value_assertions_keep_operation_and_operand_together() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "message"
contains = "needle"

[[result_value_assert]]
value_path = "error.details.value"
path = "value.count"
equals = 3
"#,
    );

    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "3".to_string()
        )))
    );
}

#[test]
#[should_panic(
    expected = "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`"
)]
fn manifest_json_assertions_reject_mixed_equals_and_missing() {
    parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[json_assert]]
path = "status"
equals = "failed"
missing = true
"#,
    );
}

#[test]
fn manifest_json_assertions_parse_equals_json_file() {
    let root = test_temp_root("json-assert-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(
        text_dir.join("expected.json"),
        "{\"nested\":[1,true,null]}\n",
    )
    .expect("expected JSON sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::EqualsJsonFile(JsonValue::Object(
            vec![(
                "nested".to_string(),
                JsonValue::Array(vec![
                    JsonValue::Decimal("1".to_string()),
                    JsonValue::Bool(true),
                    JsonValue::Null
                ])
            )]
        )))
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn manifest_result_value_assertions_parse_equals_json_file() {
    let root = test_temp_root("result-value-assert-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("expected.json"), "[\"ok\",2]\n")
        .expect("expected JSON sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::EqualsJsonFile(JsonValue::Array(
            vec![
                JsonValue::String("ok".to_string()),
                JsonValue::Decimal("2".to_string())
            ]
        )))
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn manifest_equals_json_file_rejects_invalid_json() {
    let root = test_temp_root("invalid-equals-json-file");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");

    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/invalid.json"
"#,
        )
    })
    .expect_err("invalid equals_json_file JSON should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid json_assert equals_json_file value"),
        "expected invalid JSON error, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn manifest_equals_json_file_cardinality_is_checked_before_file_io() {
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals = "inline"
equals_json_file = "case-text/missing-sidecar.json"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
equals = "inline"
equals_json_file = "case-text/missing-sidecar.json"
"#,
        "result_value_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
}

#[test]
fn manifest_assertion_missing_false_is_checked_before_file_io() {
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "status"
missing = false

[[json_assert]]
path = "stdout"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 `missing` must be true when present",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"
missing = false

[[result_value_assert]]
value_path = "error.other"
path = "value"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "result_value_assert 0 `missing` must be true when present",
        "missing-sidecar",
    );
}

#[test]
fn manifest_assertion_operation_omission_is_checked_before_file_io() {
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "status"

[[json_assert]]
path = "stdout"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.value"
path = "value"

[[result_value_assert]]
value_path = "error.other"
path = "value"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "result_value_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
        "missing-sidecar",
    );
    assert_manifest_parse_error_without(
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[[file_assert]]
path = "out.txt"

[[file_assert]]
path = "other.txt"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "file_assert 0 needs exactly one of `equals` or `equals_file`",
        "missing-sidecar",
    );
}

#[test]
fn manifest_equals_json_file_loads_before_skip_evaluation() {
    let root = test_temp_root("equals-json-file-skip-lifecycle");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[skip]
reason = "would skip after manifest loading"

[[json_assert]]
path = "stdout"
equals_json_file = "case-text/invalid.json"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case_with_guard_and_after_invocation(
            &case_dir,
            |_| {},
            |_, _| panic!("command lifecycle should not reach invocation"),
        );
    })
    .expect_err("invalid equals_json_file should be rejected before skip evaluation");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid json_assert equals_json_file value"),
        "expected invalid JSON error, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn manifest_jsonrpc_fixture_frames_envelope_matrix_and_exact_case_text_bytes() {
    let root = test_temp_root("jsonrpc-fixture-framing");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(
        text_dir.join("exact.raw"),
        b"\xef\xbb\xbfalpha\r\n\xe6\x97\xa5\xe6\x9c\xac\xe8\xaa\x9e\r\n",
    )
    .expect("exact case text should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","id":"request","method":"unknown/string","params":{"nested":[{"$case_text":"case-text/exact.raw"}],"emoji":"\uD83D\uDE42"},"extension":true},
  {"jsonrpc":"2.0","id":1.5,"method":"unknown/number","params":[1,null]},
  {"jsonrpc":"2.0","id":null,"method":"unknown/null","params":null},
  {"jsonrpc":"2.0","method":"unknown/notification"},
  {"jsonrpc":"2.0","id":"request","method":"unknown/duplicate","params":{"methodSpecific":"unchecked"}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let bodies = [
        "{\"jsonrpc\":\"2.0\",\"id\":\"request\",\"method\":\"unknown/string\",\"params\":{\"nested\":[\"\u{feff}alpha\\r\\n日本語\\r\\n\"],\"emoji\":\"🙂\"},\"extension\":true}",
        "{\"jsonrpc\":\"2.0\",\"id\":1.5,\"method\":\"unknown/number\",\"params\":[1,null]}",
        "{\"jsonrpc\":\"2.0\",\"id\":null,\"method\":\"unknown/null\",\"params\":null}",
        "{\"jsonrpc\":\"2.0\",\"method\":\"unknown/notification\"}",
        "{\"jsonrpc\":\"2.0\",\"id\":\"request\",\"method\":\"unknown/duplicate\",\"params\":{\"methodSpecific\":\"unchecked\"}}",
    ];
    let expected = bodies
        .iter()
        .map(|body| format!("Content-Length: {}\r\n\r\n{body}", body.len()))
        .collect::<String>();
    assert_eq!(
        manifest.invocation.stdin.as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        manifest.invocation.stdin_jsonrpc_file.as_deref(),
        Some("requests.json")
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_jsonrpc_fixture_rejects_root_message_and_envelope_failures() {
    let root = test_temp_root("jsonrpc-fixture-envelope-failures");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    let cases = [
        ("{", "invalid JSON-RPC fixture"),
        ("{}", "root must be an array"),
        ("[null]", "message 0 at $[0]: message must be an object"),
        ("[{\"method\":\"m\"}]", "message 0 at $[0].jsonrpc"),
        (
            "[{\"jsonrpc\":\"2.0\"}]",
            "message 0 at $[0].method: `method` must be a string",
        ),
        (
            "[{\"jsonrpc\":\"1.0\",\"method\":\"m\"}]",
            "`jsonrpc` must be the string `2.0`",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"jsonrpc\":\"1.0\",\"method\":\"m\"}]",
            "message 0 at $[0].jsonrpc: `jsonrpc` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"1.0\",\"jsonrpc\":\"2.0\",\"method\":\"m\"}]",
            "message 0 at $[0].jsonrpc: `jsonrpc` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"method\":false}]",
            "message 0 at $[0].method: `method` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":false,\"method\":\"m\"}]",
            "message 0 at $[0].method: `method` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":1,\"id\":true}]",
            "message 0 at $[0].id: `id` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":true,\"id\":1}]",
            "message 0 at $[0].id: `id` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":null,\"params\":\"bad\"}]",
            "message 0 at $[0].params: `params` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":\"bad\",\"params\":null}]",
            "message 0 at $[0].params: `params` must not appear more than once",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":false}]",
            "message 0 at $[0].method: `method` must be a string",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"id\":true}]",
            "message 0 at $[0].id: `id` must be a string, number, or null",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"params\":\"bad\"}]",
            "message 0 at $[0].params: `params` must be an object, array, or null",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"result\":null}]",
            "request or notification must not contain `result` or `error`",
        ),
        (
            "[{\"jsonrpc\":\"2.0\",\"method\":\"m\",\"error\":null}]",
            "request or notification must not contain `result` or `error`",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"emoji":"\uD83D"}}]"#,
            "unpaired high surrogate",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"emoji":"\uDE42"}}]"#,
            "unpaired low surrogate",
        ),
    ];
    for (fixture, expected) in cases {
        fs::write(case_dir.join("requests.json"), fixture)
            .expect("JSON-RPC fixture should be written");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
            );
        })
        .expect_err("invalid JSON-RPC fixture should fail");
        let message = panic_message(panic);
        assert!(
            message.contains(expected),
            "expected `{expected}` in `{message}`"
        );
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_jsonrpc_fixture_reports_malformed_following_message_index() {
    let root = test_temp_root("jsonrpc-fixture-malformed-index");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"first"},{"jsonrpc":"2.0","method":}]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
        );
    })
    .expect_err("malformed following message should fail");
    let message = panic_message(panic);
    assert!(
        message.contains("invalid JSON-RPC fixture `requests.json` message 1"),
        "expected message index in `{message}`"
    );
    assert!(
        message.contains("unexpected byte `}`"),
        "expected parse position detail in `{message}`"
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_jsonrpc_fixture_rejects_reserved_directive_shapes_and_paths() {
    let root = test_temp_root("jsonrpc-fixture-directive-failures");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    let cases = [
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"deep":[{"$case_text":1}]}}]"#,
            "message 0 at $[0].params.deep[0]: `$case_text` directive value must be a string",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"missing.txt","extra":null}}]"#,
            "`$case_text` directive object must contain no other members",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"../escape.txt"}}]"#,
            "must use portable relative components",
        ),
        (
            r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"missing.txt"}}]"#,
            "case file `missing.txt` must match fixture entry spelling exactly",
        ),
    ];
    for (fixture, expected) in cases {
        fs::write(case_dir.join("requests.json"), fixture)
            .expect("JSON-RPC fixture should be written");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
            );
        })
        .expect_err("invalid directive should fail");
        let message = panic_message(panic);
        assert!(
            message.contains(expected),
            "expected `{expected}` in `{message}`"
        );
    }
    fs::write(case_dir.join("invalid.raw"), [0xff]).expect("non-UTF-8 sidecar should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"invalid.raw"}}]"#,
    )
    .expect("JSON-RPC fixture should be written");
    let panic = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
        );
    })
    .expect_err("non-UTF-8 directive sidecar should fail");
    assert!(panic_message(panic).contains("failed to read case file `invalid.raw` as UTF-8"));
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_jsonrpc_workspace_uri_directives_do_not_rewrite_marker_like_case_text() {
    let root = test_temp_root("jsonrpc-workspace-uri-case-text-collision");
    let case_dir = root.join("case");
    fs::create_dir_all(case_dir.join("case-text")).expect("case text directory should be created");
    fs::write(case_dir.join("main.veln"), "").expect("workspace file should be written");
    let marker_like_text = format!("{WORKSPACE_FILE_URI_MARKER}not-hex\nordinary text");
    fs::write(case_dir.join("case-text/marker.raw"), &marker_like_text)
        .expect("marker-like case text should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","method":"text","params":{"text":{"$case_text":"case-text/marker.raw"}}},
  {"jsonrpc":"2.0","method":"uri","params":{"uri":{"$workspace_file_uri":"main.veln"}}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    assert!(
        manifest
            .invocation
            .stdin
            .as_deref()
            .is_some_and(|stdin| stdin.contains(WORKSPACE_FILE_URI_MARKER))
    );
    let framed = manifest
        .invocation
        .materialized_stdin(&case_dir)
        .expect("JSON-RPC stdin should materialize");
    let messages = decode_lsp_stdout(&framed).expect("framed JSON-RPC input should decode");
    assert_eq!(
        json_path(&messages[0], "params.text"),
        Some(&JsonValue::String(marker_like_text))
    );
    assert_eq!(
        json_path(&messages[1], "params.uri"),
        Some(&JsonValue::String(
            workspace_file_uri(&case_dir, "main.veln").expect("workspace URI should resolve")
        ))
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_jsonrpc_workspace_uri_directive_materializes_later_duplicate_member() {
    let root = test_temp_root("jsonrpc-workspace-uri-duplicate-member");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(case_dir.join("main.veln"), "").expect("workspace file should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","method":"uri","params":{"uri":"preserve-first","uri":{"$workspace_file_uri":"main.veln"}}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let framed = manifest
        .invocation
        .materialized_stdin(&case_dir)
        .expect("JSON-RPC stdin should materialize");
    assert!(
        !framed.contains(WORKSPACE_FILE_URI_MARKER),
        "workspace URI marker must not reach command input"
    );
    let messages = decode_lsp_stdout(&framed).expect("framed JSON-RPC input should decode");
    let JsonValue::Object(message_entries) = &messages[0] else {
        panic!("message should be an object");
    };
    let params = message_entries
        .iter()
        .find_map(|(key, value)| (key == "params").then_some(value))
        .expect("params should exist");
    let JsonValue::Object(params_entries) = params else {
        panic!("params should be an object");
    };
    let uri_values = params_entries
        .iter()
        .filter_map(|(key, value)| (key == "uri").then_some(value))
        .collect::<Vec<_>>();
    assert_eq!(uri_values.len(), 2);
    assert_eq!(
        uri_values[0],
        &JsonValue::String("preserve-first".to_string())
    );
    let expected_uri =
        workspace_file_uri(&case_dir, "main.veln").expect("workspace URI should resolve");
    assert_eq!(uri_values[1], &JsonValue::String(expected_uri));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[cfg(unix)]
#[test]
fn manifest_jsonrpc_resources_fail_before_skip_fixture_copy_and_command_start() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("jsonrpc-fixture-lifecycle");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(case_dir.join("requests.json"), "{").expect("invalid fixture should be written");
    symlink("missing-target", case_dir.join("copy-must-not-start"))
        .expect("fixture-copy sentinel link should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["command-must-not-start"]
stdin_jsonrpc_file = "requests.json"
exit = 0

[skip]
platforms = ["linux", "macos"]
reason = "skip evaluation must not run"
"#,
    )
    .expect("case manifest should be written");
    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("resource failure should precede the case lifecycle");
    let message = panic_message(panic);
    assert!(message.contains("invalid JSON-RPC fixture"));
    assert!(!message.contains("skip evaluation must not run"));
    assert!(!message.contains("fixture entries must not be links"));
    assert!(!message.contains("command-must-not-start"));

    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"copy-must-not-start"}}]"#,
    )
    .expect("link directive fixture should be written");
    let panic = std::panic::catch_unwind(|| CaseManifest::read(&case_dir.join("case.toml")))
        .expect_err("link-like directive sidecar should fail");
    assert!(panic_message(panic).contains("must not traverse a link or reparse point"));
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_sidecar_choice_cardinality_is_checked_before_file_io() {
    assert_manifest_parse_error(
        r#"
command = ["check"]
stdin = "inline"
stdin_file = "case-text/missing-sidecar.txt"
stdin_jsonrpc_file = "missing.json"
exit = 0
"#,
        "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals = "inline"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[[file_assert]]
path = "out.txt"
equals = "inline"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "file_assert 0 needs exactly one of `equals` or `equals_file`",
    );
}

#[test]
fn manifest_stream_fragments_accumulate_in_manifest_order() {
    let root = test_temp_root("manifest-fragment-order");
    let case_dir = root.join("examples/specification/check/manifest-fragment-order");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("middle.txt"), "middle").expect("sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["check"]
exit = 0

[stdout]
contains = ["before"]
contains_file = "case-text/middle.txt"
contains = ["after"]
not_contains = ["forbidden before"]
not_contains = ["forbidden after"]
"#,
    );

    assert_eq!(
        manifest.expectations.stdout.contains,
        ["before", "middle", "after"]
    );
    assert_eq!(
        manifest.expectations.stdout.not_contains,
        ["forbidden before", "forbidden after"]
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn case_text_git_attributes_cover_text_and_raw_sidecars() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("manifest directory should be under the repository");
    let migrated_lsp_raw_sidecars = [
        "examples/specification/lsp/ambiguous-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/companion-private-function-identity/case-text/root-stdin-1.raw",
        "examples/specification/lsp/companion-private-function-rename-overlay/case-text/root-stdin-1.raw",
        "examples/specification/lsp/direct-dependency-virtual-document-boundary/case-text/root-stdin-1.raw",
        "examples/specification/lsp/direct-git-dependency-virtual-document/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-context-callable-binding/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-context-operation-heading-isolation/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-operation-editor/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-satisfy-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/imported-constructor-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/open-document-nested-boundary/case-text/root-stdin-1.raw",
        "examples/specification/lsp/private-import-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/reexported-constructor-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/schema-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/standard-library-virtual-document/case-text/root-stdin-1.raw",
        "examples/specification/lsp/unopened-missing-file/case-text/root-stdin-1.raw",
        "examples/specification/lsp/workspace-diagnostics/case-text/root-stdin-1.raw",
        "examples/specification/lsp/workspace-package-root-selection/case-text/root-stdin-1.raw",
    ];
    let ordinary_sidecars = [
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/json-assert-equals-1.txt",
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/nested/json-assert-equals-1.txt",
    ];
    let pattern_raw_sidecars = [
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/protocol.raw",
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/nested/protocol.raw",
        "examples/specification/lsp/semantic-tokens/case-text/protocol.raw",
        "examples/specification/lsp/semantic-tokens/case-text/nested/protocol.raw",
    ];
    let mut check_attr_args = vec!["check-attr", "text", "eol", "diff", "whitespace", "--"];
    check_attr_args.extend(ordinary_sidecars);
    check_attr_args.extend(pattern_raw_sidecars);
    check_attr_args.extend(migrated_lsp_raw_sidecars);
    let output = Command::new("git")
        .current_dir(repo)
        .args(check_attr_args)
        .output()
        .expect("git check-attr should run");
    assert!(
        output.status.success(),
        "git check-attr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("attribute output should be utf-8");
    for ordinary in ordinary_sidecars {
        assert!(stdout.contains(&format!("{ordinary}: text: set")));
        assert!(stdout.contains(&format!("{ordinary}: eol: lf")));
        assert!(stdout.contains(&format!("{ordinary}: whitespace: unset")));
    }
    for raw in pattern_raw_sidecars
        .into_iter()
        .chain(migrated_lsp_raw_sidecars)
    {
        assert!(stdout.contains(&format!("{raw}: text: unset")));
        assert!(stdout.contains(&format!("{raw}: eol: unset")));
        assert!(stdout.contains(&format!("{raw}: diff: unset")));
    }
    let mut ls_files_args = vec!["ls-files", "--eol", "--"];
    ls_files_args.extend(migrated_lsp_raw_sidecars);
    let output = Command::new("git")
        .current_dir(repo)
        .args(ls_files_args)
        .output()
        .expect("git ls-files --eol should run");
    assert!(
        output.status.success(),
        "git ls-files --eol failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("eol output should be utf-8");
    for raw in migrated_lsp_raw_sidecars {
        assert!(
            stdout.contains(&format!("i/crlf  w/crlf  attr/-text            \t{raw}")),
            "{raw} should be checked out without text normalization:\n{stdout}"
        );
    }
}

#[test]
fn manifest_sidecar_paths_reject_nonportable_components_before_io() {
    for relative in [
        "case-text/CON.txt",
        "case-text/lpt9.log",
        "case-text/trailing.",
        "case-text/space name.txt",
        "../escape.txt",
    ] {
        let source = format!("command = [\"check\"]\nstdin_file = {relative:?}\nexit = 0\n");
        assert_manifest_parse_error(&source, "must use portable relative components");
    }
}

#[cfg(unix)]
#[test]
fn manifest_sidecar_paths_reject_links_without_following_targets() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("manifest-sidecar-link");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(root.join("outside.txt"), "target").expect("link target should be written");
    symlink("../../outside.txt", text_dir.join("linked.txt")).expect("file symlink should exist");
    symlink("../../outside-dir", text_dir.join("linked-dir")).expect("dir symlink should exist");

    for relative in ["case-text/linked.txt", "case-text/linked-dir/file.txt"] {
        let source = format!("command = [\"check\"]\nstdin_file = {relative:?}\nexit = 0\n");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(&case_dir.join("case.toml"), &source);
        })
        .expect_err("sidecar link traversal should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("must not traverse a link or reparse point"),
            "unexpected sidecar link error: {message}"
        );
        assert!(
            !message.contains("outside"),
            "sidecar diagnostics should not expose followed targets: {message}"
        );
    }

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_sidecar_files_must_be_utf8_before_skip_evaluation() {
    let root = test_temp_root("manifest-sidecar-utf8-skip");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.raw"), [0xff, b'a']).expect("invalid sidecar should exist");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run-command-that-must-not-start"]
stdin_file = "case-text/invalid.raw"
exit = 0

[skip]
platforms = ["linux", "macos", "windows"]
reason = "skip evaluation must not hide sidecar loading"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("invalid UTF-8 sidecar should fail before skip evaluation");
    let message = panic_message(panic);
    assert!(message.contains("failed to read case file `case-text/invalid.raw` as UTF-8"));
    assert!(
        !message.contains("skip evaluation must not hide sidecar loading"),
        "skip evaluation should be bypassed by sidecar loading failure"
    );
    assert!(
        !message.contains("run-command-that-must-not-start"),
        "command execution should be bypassed by sidecar loading failure"
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_sidecar_files_preserve_bom_crlf_and_final_line_breaks() {
    let root = test_temp_root("manifest-sidecar-exact-text");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("exact.raw"), b"\xef\xbb\xbfalpha\r\nbeta\r\n")
        .expect("exact text sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["check"]
stdin_file = "case-text/exact.raw"
exit = 0
"#,
    );

    assert_eq!(
        manifest.invocation.stdin.as_deref(),
        Some("\u{feff}alpha\r\nbeta\r\n")
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_sidecar_snapshots_are_immutable_across_repeated_invocations() {
    let root = test_temp_root("manifest-sidecar-repeat-snapshot");
    let case_dir = root.join("examples/specification/check/manifest-sidecar-repeat-snapshot");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("out.txt"), "original").expect("sidecar should be written");
    fs::write(case_dir.join("out.txt"), "original").expect("asserted file should be written");
    fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
        .expect("source file should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check", "main.veln"]
repeat = 2
exit = 0

[[file_assert]]
path = "out.txt"
equals_file = "case-text/out.txt"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case_with_after_invocation(&case_dir, |context, project_root| {
            if context.run_number == 1 {
                fs::write(project_root.join("out.txt"), "mutated")
                    .expect("copied fixture should be mutable after first run");
            }
        });
    })
    .expect_err("second run should compare against the original sidecar snapshot");
    let message = panic_message(panic);
    assert!(message.contains("run 2"));
    assert!(message.contains("file `out.txt` contents mismatch"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_validation_rejects_incomplete_section_expectations() {
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[binary_fixture]]
name = "short-u24"
hex = "0001"
byte_offset = 2
field_path = []
expected_count = 3
"#,
        "binary_fixture 0 has incomplete byte count metadata",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
"#,
        "output_chunk_list 0 is missing `chunks`",
    );
}

fn lsp_frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

fn parsed_lsp_assertions(source: &str) -> Vec<LspAssertion> {
    parse_manifest(Path::new("case.toml"), source)
        .expectations
        .lsp_assertions
}

fn parsed_mcp_assertions(source: &str) -> Vec<McpAssertion> {
    parse_manifest(Path::new("case.toml"), source)
        .expectations
        .mcp_assertions
}

#[test]
fn manifest_lsp_and_mcp_file_backed_equality_loads_immutable_case_operands() {
    let root = test_temp_root("lsp-mcp-file-backed-equality");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    let expected_text = text_dir.join("expected.txt");
    let expected_json = text_dir.join("expected.json");
    fs::write(&expected_text, "expected text\n").expect("text sidecar should be written");
    fs::write(&expected_json, r#"{"b":[2],"a":1}"#).expect("JSON sidecar should be written");

    let lsp_manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals_json_file = "case-text/expected.json"
"#,
    );
    let mcp_manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
equals_file = "case-text/expected.txt"
[[mcp_assert]]
id = 1
path = "/result/value"
equals_json_file = "case-text/expected.json"
"#,
    );

    assert_eq!(
        lsp_manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::EqualsJsonFile(
            parse_json(r#"{"b":[2],"a":1}"#).expect("expected JSON should parse")
        ))
    );
    assert_eq!(
        mcp_manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::EqualsFile(
            "expected text\n".to_string()
        ))
    );
    assert!(matches!(
        mcp_manifest.expectations.mcp_assertions[1].operation,
        Some(RpcAssertionOperation::EqualsJsonFile(_))
    ));

    fs::write(&expected_text, "modified workspace text\n")
        .expect("text sidecar should be changed after manifest loading");
    fs::write(&expected_json, r#"{"modified":true}"#)
        .expect("JSON sidecar should be changed after manifest loading");

    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"a":1,"b":[2]}}"#,
    ))
    .expect("LSP response should decode");
    evaluate_lsp_assertion(&lsp_messages, &lsp_manifest.expectations.lsp_assertions[0])
        .expect("LSP JSON file equality should use the discovered case snapshot");

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"expected text\n","value":{"a":1,"b":[2]}}}
"#,
    )
    .expect("MCP response should decode");
    for assertion in &mcp_manifest.expectations.mcp_assertions {
        evaluate_mcp_assertion(&mcp_messages, assertion, &case_dir)
            .expect("MCP file equality should use the discovered case snapshot");
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn manifest_lsp_and_mcp_file_equality_rejects_invalid_missing_and_duplicate_operands() {
    let root = test_temp_root("lsp-mcp-invalid-json-sidecar");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.json"), "{").expect("invalid JSON sidecar should be written");

    for (command, section) in [("lsp", "lsp_assert"), ("mcp", "mcp_assert")] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals_json_file = \"case-text/invalid.json\"\n"
                ),
            )
        })
        .expect_err("invalid JSON sidecar should fail manifest loading");
        let message = panic_message(error);
        assert!(message.contains(section), "{message}");
        assert!(message.contains("0"), "{message}");
        assert!(message.contains("response id 1"), "{message}");
        assert!(message.contains("path `/result`"), "{message}");
        assert!(message.contains("equals_json_file"), "{message}");
        assert!(message.contains("invalid"), "{message}");

        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &case_dir.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals_json_file = \"case-text/missing.json\"\n"
                ),
            )
        })
        .expect_err("missing JSON sidecar should fail manifest loading");
        let message = panic_message(error);
        assert!(message.contains("case file `case-text/missing.json`"));
        assert!(message.contains(section), "{message}");
        assert!(message.contains("0"), "{message}");
        assert!(message.contains("response id 1"), "{message}");
        assert!(message.contains("path `/result`"), "{message}");
        assert!(message.contains("equals_json_file"), "{message}");

        assert_manifest_parse_error(
            &format!(
                "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\nequals = null\nequals_json_file = \"case-text/missing.json\"\n"
            ),
            "needs exactly one",
        );
    }

    let error = std::panic::catch_unwind(|| {
        parse_manifest(
            &case_dir.join("case.toml"),
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals_file = \"case-text/missing.txt\"\n",
        )
    })
    .expect_err("missing MCP text sidecar should fail manifest loading");
    let message = panic_message(error);
    assert!(message.contains("case file `case-text/missing.txt`"));
    assert!(message.contains("mcp_assert 0 response id 1 path `/result` equals_file"));

    for (command, section, operation, expected) in [
        (
            "lsp",
            "lsp_assert",
            "equals_json_file",
            "lsp_assert `equals_json_file` must be a string case file reference",
        ),
        (
            "mcp",
            "mcp_assert",
            "equals_file",
            "mcp_assert `equals_file` must be a string case file reference",
        ),
        (
            "mcp",
            "mcp_assert",
            "equals_json_file",
            "mcp_assert `equals_json_file` must be a string case file reference",
        ),
    ] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"{command}\"]\nexit = 0\n[[{section}]]\nid = 1\npath = \"/result\"\n{operation} = 1\n"
            ),
            expected,
        );
    }
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
fn lsp_and_mcp_file_backed_equality_report_operation_specific_failures() {
    let expected_object = parse_json(r#"{"a":2}"#).expect("expected object should parse");
    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":"one","result":{"a":1}}"#,
    ))
    .expect("LSP response should decode");
    let lsp_assertion = LspAssertion {
        id: Some(JsonValue::String("one".to_string())),
        method: None,
        occurrence: None,
        path: "/result".to_string(),
        path_present: true,
        pointer_tokens: vec!["result".to_string()],
        operation: Some(RpcAssertionOperation::EqualsJsonFile(
            expected_object.clone(),
        )),
        operation_count: 1,
    };
    assert_eq!(
        evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
            .expect_err("different LSP JSON should fail"),
        "value mismatch: expected {\"a\":2}, got {\"a\":1}"
    );

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":"one","result":{"text":1,"value":{"a":1}}}
"#,
    )
    .expect("MCP response should decode");
    let mut mcp_assertion = McpAssertion {
        id: Some(JsonValue::String("one".to_string())),
        path: "/result/text".to_string(),
        path_present: true,
        pointer_tokens: vec!["result".to_string(), "text".to_string()],
        operation: Some(RpcAssertionOperation::EqualsFile("1".to_string())),
        operation_count: 1,
    };
    assert_eq!(
        evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
            .expect_err("non-string MCP value should fail"),
        "equals_file requires a selected JSON string"
    );

    mcp_assertion.path = "/result/value".to_string();
    mcp_assertion.pointer_tokens = vec!["result".to_string(), "value".to_string()];
    mcp_assertion.operation = Some(RpcAssertionOperation::EqualsJsonFile(expected_object));
    assert_eq!(
        evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
            .expect_err("different MCP JSON should fail"),
        "value mismatch: expected {\"a\":2}, got {\"a\":1}"
    );

    let context = CaseRunContext {
        case_dir: Path::new("file-backed-runtime-context"),
        run_number: 1,
    };
    let lsp_panic = std::panic::catch_unwind(|| {
        assert_lsp_assertions(
            &context,
            &lsp_frame(r#"{"jsonrpc":"2.0","id":"one","result":{"a":1}}"#),
            &[lsp_assertion],
        );
    })
    .expect_err("aggregated LSP assertion should fail");
    let message = panic_message(lsp_panic);
    assert!(message.contains("file-backed-runtime-context run 1"));
    assert!(message.contains("lsp_assert 0"));
    assert!(message.contains("response id \"one\""));
    assert!(message.contains("path \"/result\""));
    assert!(message.contains("value mismatch"));

    let mcp_panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(
            &context,
            r#"{"jsonrpc":"2.0","id":"one","result":{"text":1,"value":{"a":1}}}
"#,
            &[mcp_assertion],
            Path::new("."),
        );
    })
    .expect_err("aggregated MCP assertion should fail");
    let message = panic_message(mcp_panic);
    assert!(message.contains("file-backed-runtime-context run 1"));
    assert!(message.contains("mcp_assert 0"));
    assert!(message.contains("response id \"one\""));
    assert!(message.contains("path \"/result/value\""));
    assert!(message.contains("value mismatch"));
}

#[test]
fn lsp_and_mcp_shared_operations_produce_the_same_result() {
    let expected_object = parse_json(r#"{"a":2}"#).expect("expected object should parse");
    let cases = [
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":{"a":1}}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Equals(expected_object.clone()),
            RpcAssertionOperation::Equals(expected_object.clone()),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":"haystack"}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Contains("needle".to_string()),
            RpcAssertionOperation::Contains("needle".to_string()),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{"value":[1]}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Length(2),
            RpcAssertionOperation::Length(2),
        ),
        (
            r#"{"jsonrpc":"2.0","id":"one","result":{}}"#,
            vec!["result".to_string(), "value".to_string()],
            RpcAssertionOperation::Missing(true),
            RpcAssertionOperation::Missing(true),
        ),
    ];

    for (message, pointer_tokens, lsp_operation, mcp_operation) in cases {
        let messages = vec![parse_json(message).expect("response should parse")];
        let lsp_assertion = LspAssertion {
            id: Some(JsonValue::String("one".to_string())),
            method: None,
            occurrence: None,
            path: "/result/value".to_string(),
            path_present: true,
            pointer_tokens: pointer_tokens.clone(),
            operation: Some(lsp_operation),
            operation_count: 1,
        };
        let mcp_assertion = McpAssertion {
            id: Some(JsonValue::String("one".to_string())),
            path: "/result/value".to_string(),
            path_present: true,
            pointer_tokens,
            operation: Some(mcp_operation),
            operation_count: 1,
        };

        assert_eq!(
            evaluate_lsp_assertion(&messages, &lsp_assertion),
            evaluate_mcp_assertion(&messages, &mcp_assertion, Path::new("."))
        );
    }
}

#[test]
fn manifest_contains_operations_parse_and_reject_invalid_forms_through_every_adapter() {
    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "value"
contains = "needle"
"#,
    );
    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value"
contains = "needle"
"#,
    );
    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
contains = "needle"
"#,
    );
    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
contains = "needle"
"#,
    );

    assert_eq!(
        json_manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        result_manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        lsp_manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Contains("needle".to_string()))
    );
    assert_eq!(
        mcp_manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::Contains("needle".to_string()))
    );

    for (command, section, fields) in [
        ("check", "json_assert", "path = \"value\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result\"\n"),
        ("mcp", "mcp_assert", "id = 1\npath = \"/result\"\n"),
    ] {
        for (operations, expected) in [
            ("", "needs exactly one"),
            (
                "contains = \"needle\"\nequals = \"needle\"\n",
                "needs exactly one",
            ),
            (
                "contains = \"first\"\ncontains = \"second\"\n",
                "duplicate key `contains`",
            ),
            ("missing = false\n", "missing` must be true"),
            ("contains = 1\n", "expected string"),
        ] {
            assert_manifest_parse_error(
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{fields}{operations}"
                ),
                expected,
            );
        }
    }
}

#[test]
fn common_length_and_workspace_uri_operations_cover_every_json_adapter() {
    let root = test_temp_root("common-json-operations");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");
    let uri = workspace_file_uri(&root, "main.veln").expect("workspace URI should resolve");

    let json_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "items"
length = 2
[[json_assert]]
path = "uri"
workspace_file_uri = "main.veln"
"#,
    );
    let json = parse_json(&format!(r#"{{"items":[1,2],"uri":{uri:?}}}"#))
        .expect("JSON adapter input should parse");
    let context = CaseRunContext {
        case_dir: Path::new("common-json-operations"),
        run_number: 1,
    };
    for (index, assertion) in json_manifest
        .expectations
        .json_assertions
        .iter()
        .enumerate()
    {
        assert_json_path_in_workspace(&context, &json, assertion, index, &root);
    }

    let result_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value.field_path"
length = 2
[[result_value_assert]]
value_path = "uri_rendered"
path = "value.id"
workspace_file_uri = "main.veln"
"#,
    );
    let rendered = parse_json(&format!(
        r#"{{"rendered":"RuntimeByteDiagnostic(offset, Cons(first, Cons(second, Nil)), facts, preview)","uri_rendered":"RuntimeDiagnostic({uri}, message, detail)"}}"#
    ))
    .expect("result-value adapter input should parse");
    for (index, assertion) in result_manifest
        .expectations
        .result_value_assertions
        .iter()
        .enumerate()
    {
        assert_result_value_path_in_workspace(&context, &rendered, assertion, index, &root);
    }

    let lsp_manifest = parse_manifest(
        &manifest_path,
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/items"
length = 2
[[lsp_assert]]
method = "note"
path = "/params/uri"
workspace_file_uri = "main.veln"
"#,
    );
    let lsp_stdout = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":[1,2]}}"#),
        lsp_frame(&format!(
            r#"{{"jsonrpc":"2.0","method":"note","params":{{"uri":{uri:?}}}}}"#
        ))
    );
    assert_lsp_assertions_in_workspace(
        &context,
        &lsp_stdout,
        &lsp_manifest.expectations.lsp_assertions,
        &root,
    );

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"value\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result\"\n"),
    ] {
        for (operation, expected) in [
            ("length = \"2\"\n", "expected integer"),
            ("workspace_file_uri = 1\n", "expected string"),
            (
                "length = 1\nworkspace_file_uri = \"main.veln\"\n",
                "needs exactly one",
            ),
        ] {
            assert_manifest_parse_error(
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}{operation}"
                ),
                expected,
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn common_length_and_workspace_uri_operand_errors_report_assertion_context() {
    let root = test_temp_root("common-json-operation-operand-context");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, selectors, selector_fragment, path_fragment) in [
        (
            "check",
            "json_assert",
            "path = \"value\"\n",
            "",
            "path `value`",
        ),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value\"\n",
            "",
            "path `value`",
        ),
        (
            "lsp",
            "lsp_assert",
            "id = 1\npath = \"/result\"\n",
            "response id 1",
            "path `/result`",
        ),
        (
            "mcp",
            "mcp_assert",
            "id = 1\npath = \"/result\"\n",
            "response id 1",
            "path `/result`",
        ),
    ] {
        for (operation, operation_fragment, failed_fact) in [
            ("length = \"2\"\n", "length", "expected integer"),
            (
                "workspace_file_uri = 1\n",
                "workspace_file_uri",
                "expected string",
            ),
            (
                "workspace_file_uri = \"../main.veln\"\n",
                "workspace_file_uri",
                "must not contain",
            ),
        ] {
            let panic = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}{operation}"
                    ),
                )
            })
            .expect_err("invalid assertion operand should fail");
            let message = panic_message(panic);
            for expected in [
                section,
                "0",
                selector_fragment,
                path_fragment,
                operation_fragment,
                failed_fact,
            ] {
                assert!(
                    expected.is_empty() || message.contains(expected),
                    "expected `{expected}` in `{message}`"
                );
            }
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn json_and_result_value_operation_before_path_errors_keep_resolved_context() {
    let root = test_temp_root("json-result-operation-before-path-error");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, expected_fragments) in [
        (
            "check",
            "json_assert",
            "length = \"2\"\n",
            "path = \"items\"\n",
            vec![
                "json_assert",
                "0",
                "path `items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "check",
            "json_assert",
            "workspace_file_uri = 1\n",
            "path = \"uri\"\n",
            vec![
                "json_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "check",
            "json_assert",
            "workspace_file_uri = \"../main.veln\"\n",
            "path = \"uri\"\n",
            vec![
                "json_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "must not contain",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "length = \"2\"\n",
            "value_path = \"rendered\"\npath = \"items\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "workspace_file_uri = 1\n",
            "value_path = \"rendered\"\npath = \"uri\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "run",
            "result_value_assert",
            "workspace_file_uri = \"../main.veln\"\n",
            "value_path = \"rendered\"\npath = \"uri\"\n",
            vec![
                "result_value_assert",
                "0",
                "path `uri`",
                "workspace_file_uri",
                "must not contain",
            ],
        ),
    ] {
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"
                ),
            )
        })
        .expect_err("invalid assertion operand should fail");
        let message = panic_message(panic);
        for expected in expected_fragments {
            assert!(
                message.contains(expected),
                "expected `{expected}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn lsp_and_mcp_length_and_workspace_uri_accept_operation_before_selector_path() {
    let root = test_temp_root("lsp-mcp-operation-before-selector");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, operation_fragment) in [
        (
            "lsp",
            "lsp_assert",
            "length = 2\n",
            "id = 1\npath = \"/result/items\"\n",
            "length",
        ),
        (
            "lsp",
            "lsp_assert",
            "workspace_file_uri = \"main.veln\"\n",
            "method = \"note\"\npath = \"/params/uri\"\n",
            "workspace_file_uri",
        ),
        (
            "mcp",
            "mcp_assert",
            "length = 2\n",
            "id = 1\npath = \"/result/items\"\n",
            "length",
        ),
        (
            "mcp",
            "mcp_assert",
            "workspace_file_uri = \"main.veln\"\n",
            "id = 1\npath = \"/result/uri\"\n",
            "workspace_file_uri",
        ),
    ] {
        let manifest = parse_manifest(
            &manifest_path,
            &format!("command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"),
        );
        let parsed_operation = match section {
            "lsp_assert" => match manifest.expectations.lsp_assertions[0]
                .operation
                .as_ref()
                .expect("operation should parse")
            {
                RpcAssertionOperation::Length(_) => "length",
                RpcAssertionOperation::WorkspaceFileUri(_) => "workspace_file_uri",
                operation => panic!("unexpected lsp operation: {operation:?}"),
            },
            "mcp_assert" => match manifest.expectations.mcp_assertions[0]
                .operation
                .as_ref()
                .expect("operation should parse")
            {
                RpcAssertionOperation::Length(_) => "length",
                RpcAssertionOperation::WorkspaceFileUri(_) => "workspace_file_uri",
                operation => panic!("unexpected mcp operation: {operation:?}"),
            },
            _ => unreachable!("test section should be covered"),
        };
        assert_eq!(parsed_operation, operation_fragment);
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn lsp_and_mcp_operation_before_selector_path_errors_keep_resolved_context() {
    let root = test_temp_root("lsp-mcp-operation-before-selector-error");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let manifest_path = root.join("case.toml");

    for (command, section, operation, selectors, expected_fragments) in [
        (
            "lsp",
            "lsp_assert",
            "length = \"2\"\n",
            "id = 1\npath = \"/result/items\"\n",
            vec![
                "lsp_assert",
                "0",
                "response id 1",
                "path `/result/items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "lsp",
            "lsp_assert",
            "workspace_file_uri = 1\n",
            "method = \"note\"\npath = \"/params/uri\"\n",
            vec![
                "lsp_assert",
                "0",
                "notification method \"note\" occurrence 0",
                "path `/params/uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
        (
            "mcp",
            "mcp_assert",
            "length = \"2\"\n",
            "id = 1\npath = \"/result/items\"\n",
            vec![
                "mcp_assert",
                "0",
                "response id 1",
                "path `/result/items`",
                "length",
                "expected integer",
            ],
        ),
        (
            "mcp",
            "mcp_assert",
            "workspace_file_uri = 1\n",
            "id = 1\npath = \"/result/uri\"\n",
            vec![
                "mcp_assert",
                "0",
                "response id 1",
                "path `/result/uri`",
                "workspace_file_uri",
                "expected string",
            ],
        ),
    ] {
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{operation}{selectors}"
                ),
            )
        })
        .expect_err("invalid assertion operand should fail");
        let message = panic_message(panic);
        for expected in expected_fragments {
            assert!(
                message.contains(expected),
                "expected `{expected}` in `{message}`"
            );
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn common_length_and_workspace_uri_failures_keep_section_context() {
    let root = test_temp_root("common-json-operation-failures");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let context = CaseRunContext {
        case_dir: Path::new("common-json-operation-failures"),
        run_number: 1,
    };

    for (actual, operation, expected) in [
        (
            JsonValue::String("not-an-array".to_string()),
            ValueAssertionOperation::Length(1),
            "length requires a selected JSON array",
        ),
        (
            JsonValue::Array(vec![]),
            ValueAssertionOperation::Length(1),
            "array length mismatch: expected 1, got 0",
        ),
        (
            JsonValue::Number(1),
            ValueAssertionOperation::WorkspaceFileUri("main.veln".to_string()),
            "workspace_file_uri requires a selected JSON string",
        ),
        (
            JsonValue::String("file:///wrong".to_string()),
            ValueAssertionOperation::WorkspaceFileUri("main.veln".to_string()),
            "workspace URI mismatch",
        ),
    ] {
        let error = expect_value_assertion(&actual, &operation, &root)
            .expect_err("common operation should report its failed fact");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }

    let json_manifest = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "missing"
length = 0
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_json_path_in_workspace(
            &context,
            &JsonValue::Object(vec![]),
            &json_manifest.expectations.json_assertions[0],
            0,
            &root,
        )
    })
    .expect_err("missing JSON path should fail");
    let message = panic_message(panic);
    assert!(message.contains("json_assert 0"));
    assert!(message.contains("path `missing`"));
    assert!(message.contains("was not found"));

    let result_manifest = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "value.missing"
length = 0
"#,
    );
    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path_in_workspace(
            &context,
            &rendered,
            &result_manifest.expectations.result_value_assertions[0],
            0,
            &root,
        )
    })
    .expect_err("missing result-value path should fail");
    let message = panic_message(panic);
    assert!(message.contains("result_value_assert 0"));
    assert!(message.contains("path `value.missing`"));
    assert!(message.contains("was not found"));

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"uri\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"value.id\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result/uri\"\n"),
    ] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &root.join("case.toml"),
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}workspace_file_uri = \"../main.veln\"\n"
                ),
            )
        })
        .expect_err("unsafe workspace URI operand should fail");
        let message = panic_message(error);
        assert!(message.contains("workspace_file_uri"));
        assert!(message.contains("must not contain"));
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn common_length_parser_accepts_full_usize_range_without_signed_intermediary() {
    let root = test_temp_root("common-length-usize-range");
    let manifest_path = root.join("case.toml");
    let max = usize::MAX.to_string();
    let over_max = ((usize::MAX as u128) + 1).to_string();

    for (command, section, selectors) in [
        ("check", "json_assert", "path = \"items\"\n"),
        (
            "run",
            "result_value_assert",
            "value_path = \"rendered\"\npath = \"items\"\n",
        ),
        ("lsp", "lsp_assert", "id = 1\npath = \"/result/items\"\n"),
        ("mcp", "mcp_assert", "id = 1\npath = \"/result/items\"\n"),
    ] {
        for accepted in ["9223372036854775807", max.as_str()] {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = {accepted}\n"
                ),
            );
        }
        if usize::BITS > 63 {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = 9223372036854775808\n"
                ),
            );
        }

        for (rejected, expected) in [
            ("-1", "expected non-negative integer"),
            ("1.0", "expected integer"),
            (
                over_max.as_str(),
                "expected non-negative integer within range",
            ),
        ] {
            let panic = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"{command}\"]\nexit = 0\n[[{section}]]\n{selectors}length = {rejected}\n"
                    ),
                )
            })
            .expect_err("invalid length operand should fail");
            let message = panic_message(panic);
            for fragment in [section, "0", "length", expected] {
                assert!(
                    message.contains(fragment),
                    "expected `{fragment}` in `{message}`"
                );
            }
        }
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn common_operation_wrapper_failures_keep_full_context_matrix() {
    let root = test_temp_root("common-operation-wrapper-context");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let context = CaseRunContext {
        case_dir: Path::new("common-operation-wrapper-context"),
        run_number: 1,
    };
    let actual_uri = workspace_file_uri(&root, "main.veln").expect("workspace URI should resolve");
    let wrong_uri = "file:///wrong";

    for (operation, actual, fragments) in [
        (
            "length = 1",
            r#"{"items":"not-array"}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `items`",
                "length requires a selected JSON array",
            ],
        ),
        (
            "length = 2",
            r#"{"items":[1]}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `items`",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            r#"{"uri":1}"#.to_string(),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `uri`",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            format!(r#"{{"uri":"{wrong_uri}"}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "json_assert 0",
                "JSON path `uri`",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let manifest = parse_manifest(
            &root.join("case.toml"),
            &format!(
                "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"{}\"\n{operation}\n",
                if operation.starts_with("length") {
                    "items"
                } else {
                    "uri"
                }
            ),
        );
        let json = parse_json(&actual).expect("JSON wrapper matrix input should parse");
        let panic = std::panic::catch_unwind(|| {
            assert_json_path_in_workspace(
                &context,
                &json,
                &manifest.expectations.json_assertions[0],
                0,
                &root,
            )
        })
        .expect_err("JSON assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    for (operation, rendered_value, selected_path, fragments) in [
        (
            "length = 1",
            "RuntimeDiagnostic(id, msg, RuntimeValueDiagnostic(Nil, reason))",
            "value.id",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.id`",
                "length requires a selected JSON array",
            ],
        ),
        (
            "length = 2",
            "RuntimeDiagnostic(id, msg, RuntimeByteDiagnostic(ByteOffset(1), Cons(RuntimeDiagnosticFieldPathSegment(schema, body), Nil), RuntimeByteCountFacts(ByteCount(2), ByteCount(1), partial), NoRuntimeBytePreview))",
            "value.detail.field_path",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.detail.field_path`",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            "ByteOffset(2)",
            "value",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value`",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "workspace_file_uri = \"main.veln\"",
            "RuntimeDiagnostic(id, msg, RuntimeValueDiagnostic(Nil, reason))",
            "value.id",
            vec![
                "common-operation-wrapper-context run 1",
                "result_value_assert 0",
                "result value path `value.id`",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let manifest = parse_manifest(
            &root.join("case.toml"),
            &format!(
                "command = [\"run\", \"--json\", \"main\", \"main.veln\"]\nexit = 0\n[[result_value_assert]]\nvalue_path = \"rendered\"\npath = \"{selected_path}\"\n{operation}\n"
            ),
        );
        let rendered = JsonValue::Object(vec![(
            "rendered".to_string(),
            JsonValue::String(rendered_value.to_string()),
        )]);
        let panic = std::panic::catch_unwind(|| {
            assert_result_value_path_in_workspace(
                &context,
                &rendered,
                &manifest.expectations.result_value_assertions[0],
                0,
                &root,
            )
        })
        .expect_err("result-value assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    for (manifest_fields, lsp_stdout, fragments) in [
        (
            "id = 1\npath = \"/result/items\"\nlength = 1\n",
            lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":"not-array"}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "response id 1",
                "path \"/result/items\"",
                "length requires a selected JSON array",
            ],
        ),
        (
            "id = 1\npath = \"/result/items\"\nlength = 2\n",
            lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":[1]}}"#),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "response id 1",
                "path \"/result/items\"",
                "array length mismatch: expected 2, got 1",
            ],
        ),
        (
            "method = \"textDocument/publishDiagnostics\"\npath = \"/params/uri\"\nworkspace_file_uri = \"main.veln\"\n",
            lsp_frame(
                r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":1,"diagnostics":[]}}"#,
            ),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "notification method \"textDocument/publishDiagnostics\" occurrence 0",
                "path \"/params/uri\"",
                "workspace_file_uri requires a selected JSON string",
            ],
        ),
        (
            "method = \"textDocument/publishDiagnostics\"\npath = \"/params/uri\"\nworkspace_file_uri = \"main.veln\"\n",
            lsp_frame(&format!(
                r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":"{wrong_uri}","diagnostics":[]}}}}"#
            )),
            vec![
                "common-operation-wrapper-context run 1",
                "lsp_assert 0",
                "notification method \"textDocument/publishDiagnostics\" occurrence 0",
                "path \"/params/uri\"",
                "workspace URI mismatch",
                "main.veln",
            ],
        ),
    ] {
        let lsp_failures = parse_manifest(
            &root.join("case.toml"),
            &format!("command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{manifest_fields}"),
        )
        .expectations
        .lsp_assertions;
        let panic = std::panic::catch_unwind(|| {
            assert_lsp_assertions_in_workspace(&context, &lsp_stdout, &lsp_failures, &root)
        })
        .expect_err("LSP assertion should fail");
        let message = panic_message(panic);
        for fragment in fragments {
            assert!(
                message.contains(fragment),
                "expected `{fragment}` in `{message}`"
            );
        }
    }

    let lsp_failures = parse_manifest(
        &root.join("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/items"
length = 1
[[lsp_assert]]
id = 2
path = "/result/items"
length = 2
"#,
    )
    .expectations
    .lsp_assertions;
    let lsp_stdout = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"items":"not-array"}}"#),
        lsp_frame(r#"{"jsonrpc":"2.0","id":2,"result":{"items":[1]}}"#)
    );
    let panic = std::panic::catch_unwind(|| {
        assert_lsp_assertions_in_workspace(&context, &lsp_stdout, &lsp_failures, &root)
    })
    .expect_err("LSP assertions should aggregate failures");
    let message = panic_message(panic);
    for fragment in [
        "lsp_assert 0",
        "length requires a selected JSON array",
        "lsp_assert 1",
        "array length mismatch: expected 2, got 1",
    ] {
        assert!(
            message.contains(fragment),
            "expected `{fragment}` in `{message}`"
        );
    }

    assert!(
        actual_uri.starts_with("file://"),
        "test should create a real file URI"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

fn contains_adapter_context() -> CaseRunContext<'static> {
    CaseRunContext {
        case_dir: Path::new("contains-adapters"),
        run_number: 1,
    }
}

#[test]
fn json_contains_evaluation_covers_success_and_failure_classes() {
    let context = contains_adapter_context();
    let json =
        parse_json(r#"{"text":"alpha beta","number":1}"#).expect("JSON adapter input should parse");
    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check"]
exit = 0
[[json_assert]]
path = "text"
contains = "ha be"
[[json_assert]]
path = "text"
contains = "missing"
[[json_assert]]
path = "number"
contains = "1"
[[json_assert]]
path = "absent"
contains = "x"
"#,
    );
    assert_json_path(
        &context,
        &json,
        &json_manifest.expectations.json_assertions[0],
    );
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "was not found"),
    ] {
        let panic = std::panic::catch_unwind(|| {
            assert_json_path(
                &context,
                &json,
                &json_manifest.expectations.json_assertions[index],
            )
        })
        .expect_err("JSON contains assertion should fail");
        assert!(panic_message(panic).contains(expected));
    }
}

#[test]
fn result_value_contains_evaluation_covers_success_and_failure_classes() {
    let context = contains_adapter_context();
    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "rr"
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "missing"
[[result_value_assert]]
value_path = "rendered"
path = "value"
contains = "2"
[[result_value_assert]]
value_path = "rendered"
path = "absent"
contains = "x"
"#,
    );
    assert_result_value_path(
        &context,
        &rendered,
        &result_manifest.expectations.result_value_assertions[0],
    );
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "was not found"),
    ] {
        let panic = std::panic::catch_unwind(|| {
            assert_result_value_path(
                &context,
                &rendered,
                &result_manifest.expectations.result_value_assertions[index],
            )
        })
        .expect_err("result-value contains assertion should fail");
        assert!(panic_message(panic).contains(expected));
    }
}

#[test]
fn lsp_contains_evaluation_covers_success_and_failure_classes() {
    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":1}}"#,
    ))
    .expect("LSP adapter input should decode");
    let lsp_assertions = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "ha be"
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "missing"
[[lsp_assert]]
id = 1
path = "/result/number"
contains = "1"
[[lsp_assert]]
id = 1
path = "/result/absent"
contains = "x"
"#,
    );
    evaluate_lsp_assertion(&lsp_messages, &lsp_assertions[0])
        .expect("LSP contains assertion should pass");
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "selected JSON path was not found"),
    ] {
        let error = evaluate_lsp_assertion(&lsp_messages, &lsp_assertions[index])
            .expect_err("LSP contains assertion should fail");
        assert!(error.contains(expected));
    }
}

#[test]
fn mcp_contains_evaluation_covers_success_and_failure_classes() {
    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":1}}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertions = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "ha be"
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "missing"
[[mcp_assert]]
id = 1
path = "/result/number"
contains = "1"
[[mcp_assert]]
id = 1
path = "/result/absent"
contains = "x"
"#,
    );
    evaluate_mcp_assertion(&mcp_messages, &mcp_assertions[0], Path::new("."))
        .expect("MCP contains assertion should pass");
    for (index, expected) in [
        (1, "string does not contain"),
        (2, "contains requires a selected JSON string"),
        (3, "selected JSON path was not found"),
    ] {
        let error = evaluate_mcp_assertion(&mcp_messages, &mcp_assertions[index], Path::new("."))
            .expect_err("MCP contains assertion should fail");
        assert!(error.contains(expected));
    }
}

#[test]
fn contains_failures_retain_wrapper_context_through_every_adapter() {
    let context = contains_adapter_context();

    let json_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["check", "--json", "main.veln"]
exit = 0
[[json_assert]]
path = "text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        json_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"text":"alpha beta"}"#.to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("JSON contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("JSON path `text` mismatch"));
    assert!(message.contains("string does not contain \"missing\""));

    let result_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["run", "--json", "main", "main.veln"]
exit = 0
[[result_value_assert]]
value_path = "rendered"
path = "constructor"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        result_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"rendered":"ByteOffset(2)"}"#.to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("result-value contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("result value path `constructor` mismatch"));
    assert!(message.contains("string does not contain \"missing\""));

    let lsp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        lsp_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta"}}"#),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("LSP contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("response id 1 path \"/result/text\""));
    assert!(message.contains("string does not contain \"missing\""));

    let mcp_manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/text"
contains = "missing"
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        mcp_manifest.expectations.assert_matches(
            &context,
            &CapturedOutput {
                exit: Some(0),
                stdout: r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta"}}
"#
                .to_string(),
                stderr: String::new(),
            },
            Path::new("."),
        )
    })
    .expect_err("MCP contains failure should include wrapper context");
    let message = panic_message(panic);
    assert!(message.contains("contains-adapters run 1"));
    assert!(message.contains("response id 1 path \"/result/text\""));
    assert!(message.contains("string does not contain \"missing\""));
}

#[test]
fn manifest_lsp_assertions_validate_selector_operation_and_pointer_contracts() {
    for (fields, expected) in [
        (
            "path = \"\"\nequals = null\n",
            "exactly one of `id` or `method`",
        ),
        (
            "id = 1\nmethod = \"note\"\npath = \"\"\nequals = null\n",
            "exactly one of `id` or `method`",
        ),
        (
            "id = 1\noccurrence = 0\npath = \"\"\nequals = null\n",
            "occurrence` is valid only with `method`",
        ),
        ("id = 1\nequals = null\n", "is missing `path`"),
        ("id = 1\npath = \"\"\n", "needs exactly one"),
        (
            "id = 1\npath = \"\"\nequals = null\ncontains = \"x\"\n",
            "needs exactly one",
        ),
        (
            "id = 1\npath = \"\"\nmissing = false\n",
            "missing` must be true",
        ),
    ] {
        assert_manifest_parse_error(
            &format!("command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{fields}"),
            expected,
        );
    }

    for pointer in ["value", "#fragment", "/~", "/~2", "/a~x"] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = {pointer:?}\nequals = null\n"
            ),
            "JSON Pointer",
        );
    }
}

#[test]
fn manifest_mcp_assertions_validate_selector_operation_pointer_and_uri_contracts() {
    for (fields, expected) in [
        ("path = \"\"\nequals = null\n", "is missing `id`"),
        (
            "id = null\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = []\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = 1.0\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        (
            "id = 1e0\npath = \"\"\nequals = null\n",
            "JSON string or integer",
        ),
        ("id = 1\nequals = null\n", "is missing `path`"),
        ("id = 1\npath = \"\"\n", "needs exactly one"),
        (
            "id = 1\npath = \"\"\nequals = null\nlength = 0\n",
            "needs exactly one",
        ),
        (
            "id = 1\npath = \"\"\nmissing = false\n",
            "missing` must be true",
        ),
    ] {
        assert_manifest_parse_error(
            &format!("command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\n{fields}"),
            expected,
        );
    }

    for pointer in ["value", "#fragment", "/~", "/~2", "/a~x"] {
        assert_manifest_parse_error(
            &format!(
                "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = {pointer:?}\nequals = null\n"
            ),
            "JSON Pointer",
        );
    }

    let root = test_temp_root("mcp-uri-manifest");
    let manifest_path = root.join("case.toml");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    parse_manifest(
        &manifest_path,
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"main.veln\"\n",
    );
    parse_manifest(
        &manifest_path,
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 9223372036854775808\npath = \"/result/uri\"\nworkspace_file_uri = \"main.veln\"\n",
    );
    for relative in [
        "",
        "/abs.veln",
        "nested/../main.veln",
        "./main.veln",
        "nested//main.veln",
        "nested/./main.veln",
        "bad\\path",
        "missing.veln",
    ] {
        let error = std::panic::catch_unwind(|| {
            parse_manifest(
                &manifest_path,
                &format!(
                    "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = {relative:?}\n"
                ),
            );
        })
        .expect_err("invalid workspace_file_uri should fail");
        assert!(
            panic_message(error).contains("workspace_file_uri"),
            "unexpected error for {relative:?}"
        );
    }
    fs::create_dir(root.join("directory.veln")).expect("directory should be created");
    let error = std::panic::catch_unwind(|| {
        parse_manifest(
            &manifest_path,
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"directory.veln\"\n",
        );
    })
    .expect_err("directory workspace_file_uri should fail");
    assert!(panic_message(error).contains("existing regular file"));

    #[cfg(unix)]
    {
        fs::create_dir(root.join("target-dir")).expect("target dir should be created");
        fs::write(root.join("target-dir").join("linked.veln"), "")
            .expect("target file should be written");
        std::os::unix::fs::symlink(
            root.join("target-dir").join("linked.veln"),
            root.join("link.veln"),
        )
        .expect("symlink file should be created");
        std::os::unix::fs::symlink(root.join("target-dir"), root.join("link-dir"))
            .expect("symlink directory should be created");

        for relative in ["link.veln", "link-dir/linked.veln"] {
            let error = std::panic::catch_unwind(|| {
                parse_manifest(
                    &manifest_path,
                    &format!(
                        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = {relative:?}\n"
                    ),
                );
            })
            .expect_err("link-like workspace_file_uri traversal should fail");
            assert!(
                panic_message(error).contains("link-like"),
                "unexpected error for {relative:?}"
            );
        }
    }
    #[cfg(windows)]
    {
        fs::write(root.join("target-file.veln"), "").expect("target file should be written");
        match std::os::windows::fs::symlink_file(
            root.join("target-file.veln"),
            root.join("link.veln"),
        ) {
            Ok(()) => {
                let error = std::panic::catch_unwind(|| {
                    parse_manifest(
                        &manifest_path,
                        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result/uri\"\nworkspace_file_uri = \"link.veln\"\n",
                    );
                })
                .expect_err("Windows link-like workspace_file_uri traversal should fail");
                assert!(panic_message(error).contains("link-like"));
            }
            Err(error) => {
                eprintln!("skipping Windows link-like workspace_file_uri evidence: {error}");
            }
        }
    }
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[cfg(not(unix))]
#[test]
fn workspace_file_uri_percent_encodes_native_non_unix_separators() {
    assert_eq!(
        path_to_file_uri(Path::new("workspace\\main.veln")),
        "file://workspace%5Cmain.veln"
    );
}

#[test]
fn manifest_json_assertions_preserve_scalar_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals = 1.0
[[result_value_assert]]
value_path = "value"
path = "value"
equals = 1e0
[[lsp_assert]]
id = 1
path = "/value"
equals = 1.0
"#,
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Decimal(
            "1e0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
}

#[test]
fn manifest_non_mcp_assertions_preserve_container_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[json_assert]]
path = "value"
equals = [1.0, {"nested": 1e0}]
[[result_value_assert]]
value_path = "value"
path = "value"
equals = {"nested": [1.0]}
[[lsp_assert]]
id = 1
path = "/value"
equals = {"nested": 1e0}
"#,
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Array(vec![
            JsonValue::Decimal("1.0".to_string()),
            JsonValue::Object(vec![(
                "nested".to_string(),
                JsonValue::Decimal("1e0".to_string())
            )])
        ])))
    );
    assert_eq!(
        manifest.expectations.result_value_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Array(vec![JsonValue::Decimal("1.0".to_string())])
        )])))
    );
    assert_eq!(
        manifest.expectations.lsp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Decimal("1e0".to_string())
        )])))
    );
}

#[test]
fn manifest_mcp_assertions_preserve_decimal_json_spelling() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result/decimal"
equals = 1.0
[[mcp_assert]]
id = 1
path = "/result/exponent"
equals = 1e0
[[mcp_assert]]
id = 1
path = "/result/nested"
equals = {"nested": [1.0, 1e0]}
"#,
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[0].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1.0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[1].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Decimal(
            "1e0".to_string()
        )))
    );
    assert_eq!(
        manifest.expectations.mcp_assertions[2].operation,
        Some(RpcAssertionOperation::Equals(JsonValue::Object(vec![(
            "nested".to_string(),
            JsonValue::Array(vec![
                JsonValue::Decimal("1.0".to_string()),
                JsonValue::Decimal("1e0".to_string())
            ])
        )])))
    );
}

#[test]
fn json_equality_preserves_object_array_kind_and_number_boundaries() {
    for (left, right, equal) in [
        (
            r#"{"outer":{"a":1,"b":[true,null]}}"#,
            r#"{"outer":{"b":[true,null],"a":1}}"#,
            true,
        ),
        (r#"[1,2]"#, r#"[1,2]"#, true),
        (r#"[1,2]"#, r#"[2,1]"#, false),
        (r#"[1,2]"#, r#"[1,2,3]"#, false),
        ("1", "1.0", false),
        ("1", "1e0", false),
        ("0", "-0", false),
        ("1.0", "1e0", false),
        (r#"{"nested":1}"#, r#"{"nested":2}"#, false),
        ("true", r#""true""#, false),
    ] {
        let left = parse_json(left).expect("left matrix value should parse");
        let right = parse_json(right).expect("right matrix value should parse");
        assert_eq!(json_values_equal(&left, &right), equal);
    }
}

fn assert_number_spelling_adapter_result(
    result: Result<(), Box<dyn std::any::Any + Send>>,
    should_match: bool,
    adapter: &str,
    mismatch_context: &str,
) {
    if should_match {
        if let Err(panic) = result {
            std::panic::resume_unwind(panic);
        }
        return;
    }

    let panic = match result {
        Ok(()) => panic!("{adapter} adapter should preserve number spelling"),
        Err(panic) => panic,
    };
    let message = panic_message(panic);
    assert!(message.contains(mismatch_context));
    assert!(message.contains("value mismatch"));
}

fn assert_json_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let actual = parse_json(&format!(r#"{{"selected":{actual}}}"#))
        .expect("JSON adapter input should parse");
    let assertion = JsonAssertion {
        path: "selected".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json(expected).expect("JSON expectation should parse"),
        )),
    };
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_json_path(context, &actual, &assertion)),
        should_match,
        "JSON",
        "JSON path `selected` mismatch",
    );
}

fn assert_lsp_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let stdout = lsp_frame(&format!(r#"{{"jsonrpc":"2.0","id":1,"result":{actual}}}"#));
    let assertions = parsed_lsp_assertions(&format!(
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result\"\nequals = {expected}\n"
    ));
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_lsp_assertions(context, &stdout, &assertions)),
        should_match,
        "LSP",
        "response id 1 path \"/result\"",
    );
}

fn assert_mcp_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let stdout = format!(r#"{{"jsonrpc":"2.0","id":1,"result":{actual}}}"#) + "\n";
    let assertions = parsed_mcp_assertions(&format!(
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals = {expected}\n"
    ));
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| {
            assert_mcp_assertions(context, &stdout, &assertions, Path::new("."))
        }),
        should_match,
        "MCP",
        "response id 1 path \"/result\"",
    );
}

fn assert_result_value_number_spelling_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    let rendered = parse_json(&format!(r#"{{"rendered":"{actual}"}}"#))
        .expect("result-value adapter input should parse");
    let assertion = ResultValueAssertion {
        value_path: "rendered".to_string(),
        path: "value".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json(expected).expect("result-value expectation should parse"),
        )),
    };
    assert_number_spelling_adapter_result(
        std::panic::catch_unwind(|| assert_result_value_path(context, &rendered, &assertion)),
        should_match,
        "result-value",
        "result value path `value` mismatch",
    );
}

fn assert_number_spelling_through_every_adapter(
    context: &CaseRunContext<'_>,
    actual: &str,
    expected: &str,
    should_match: bool,
) {
    assert_json_number_spelling_adapter(context, actual, expected, should_match);
    assert_lsp_number_spelling_adapter(context, actual, expected, should_match);
    assert_mcp_number_spelling_adapter(context, actual, expected, should_match);
    assert_result_value_number_spelling_adapter(context, actual, expected, should_match);
}

#[test]
fn json_number_spelling_matrix_runs_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-number-spelling-adapters"),
        run_number: 1,
    };

    for (actual, expected, should_match) in [
        ("1", "1", true),
        ("1.0", "1.0", true),
        ("1e0", "1e0", true),
        ("1", "1.0", false),
        ("1", "1e0", false),
        ("1.0", "1", false),
        ("1.0", "1e0", false),
        ("1e0", "1", false),
        ("1e0", "1.0", false),
    ] {
        assert_number_spelling_through_every_adapter(&context, actual, expected, should_match);
    }
}

#[test]
fn diagnostic_assertions_use_common_json_equality_for_integer_tokens() {
    let context = CaseRunContext {
        case_dir: Path::new("diagnostic-json-equality"),
        run_number: 1,
    };
    let json = parse_json(
        r#"{"diagnostics":[{"id":"type.mismatch","severity":"error","kind":"type","message":"expected `Int`, but found `String`","span":{"file":"main.veln","start":{"line":2,"column":3,"offset":23},"end":{"line":2,"column":7,"offset":27}}}]}"#,
    )
    .expect("diagnostic JSON should parse");

    assert_diagnostic(
        &context,
        &json,
        &DiagnosticExpectation {
            id: "type.mismatch".to_string(),
            severity: Some("error".to_string()),
            kind: Some("type".to_string()),
            message: Some("expected `Int`, but found `String`".to_string()),
            span: Some(SpanExpectation {
                file: Some("main.veln".to_string()),
                line: Some(2),
                column: Some(3),
            }),
        },
    );
}

#[test]
fn reordered_json_objects_compare_equal_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-equality-adapters"),
        run_number: 1,
    };
    let actual = parse_json(r#"{"selected":{"outer":{"a":1,"b":2}}}"#)
        .expect("JSON adapter input should parse");
    assert_json_path(
        &context,
        &actual,
        &JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(r#"{"outer":{"b":2,"a":1}}"#)
                    .expect("JSON adapter expectation should parse"),
            )),
        },
    );

    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    assert_result_value_path(
        &context,
        &rendered,
        &ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(r#"{"value":2,"constructor":"ByteOffset"}"#)
                    .expect("result-value adapter expectation should parse"),
            )),
        },
    );

    let lsp_messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"a":1,"b":2}}}"#,
    ))
    .expect("LSP adapter input should decode");
    let lsp_assertion = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = {"outer":{"b":2,"a":1}}
"#,
    )
    .remove(0);
    evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
        .expect("LSP adapter should ignore object member order");

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"a":1,"b":2}}}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertion = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = {"outer":{"b":2,"a":1}}
"#,
    )
    .remove(0);
    evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
        .expect("MCP adapter should ignore object member order");
}

#[test]
fn reordered_json_arrays_fail_through_every_assertion_adapter() {
    let context = CaseRunContext {
        case_dir: Path::new("json-array-order-adapters"),
        run_number: 1,
    };
    let actual = parse_json(r#"{"selected":[1,2]}"#).expect("JSON adapter input should parse");
    let json_assertion = JsonAssertion {
        path: "selected".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json("[2,1]").expect("JSON adapter expectation should parse"),
        )),
    };
    let panic = std::panic::catch_unwind(|| assert_json_path(&context, &actual, &json_assertion))
        .expect_err("JSON adapter should retain array order");
    assert!(panic_message(panic).contains("JSON path `selected` mismatch"));

    let rendered = parse_json(r#"{"rendered":"Cons(1, Cons(2, Nil))"}"#)
        .expect("result-value adapter input should parse");
    let result_assertion = ResultValueAssertion {
        value_path: "rendered".to_string(),
        path: "value".to_string(),
        operation: Some(ValueAssertionOperation::Equals(
            parse_json("[2,1]").expect("result-value expectation should parse"),
        )),
    };
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path(&context, &rendered, &result_assertion)
    })
    .expect_err("result-value adapter should retain array order");
    assert!(panic_message(panic).contains("result value path `value` mismatch"));

    let lsp_messages = decode_lsp_stdout(&lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}"#))
        .expect("LSP adapter input should decode");
    let lsp_assertion = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [2,1]
"#,
    )
    .remove(0);
    let error = evaluate_lsp_assertion(&lsp_messages, &lsp_assertion)
        .expect_err("LSP adapter should retain array order");
    assert!(error.contains("value mismatch"));

    let mcp_messages = decode_mcp_stdout(
        r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}
"#,
    )
    .expect("MCP adapter input should decode");
    let mcp_assertion = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [2,1]
"#,
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&mcp_messages, &mcp_assertion, Path::new("."))
        .expect_err("MCP adapter should retain array order");
    assert!(error.contains("value mismatch"));
}

#[test]
fn ordered_json_arrays_succeed_and_length_mismatches_retain_adapter_context() {
    let context = CaseRunContext {
        case_dir: Path::new("json-array-length-adapters"),
        run_number: 1,
    };

    let actual = parse_json(r#"{"selected":[1,2]}"#).expect("JSON adapter input should parse");
    assert_json_path(
        &context,
        &actual,
        &JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json("[1,2]").expect("JSON success expectation should parse"),
            )),
        },
    );
    let panic = std::panic::catch_unwind(|| {
        assert_json_path(
            &context,
            &actual,
            &JsonAssertion {
                path: "selected".to_string(),
                operation: Some(ValueAssertionOperation::Equals(
                    parse_json("[1,2,3]").expect("JSON failure expectation should parse"),
                )),
            },
        )
    })
    .expect_err("JSON adapter should reject array length mismatch");
    assert!(panic_message(panic).contains("JSON path `selected` mismatch"));

    let rendered = parse_json(r#"{"rendered":"Cons(1, Cons(2, Nil))"}"#)
        .expect("result-value adapter input should parse");
    assert_result_value_path(
        &context,
        &rendered,
        &ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json("[1,2]").expect("result-value success expectation should parse"),
            )),
        },
    );
    let panic = std::panic::catch_unwind(|| {
        assert_result_value_path(
            &context,
            &rendered,
            &ResultValueAssertion {
                value_path: "rendered".to_string(),
                path: "value".to_string(),
                operation: Some(ValueAssertionOperation::Equals(
                    parse_json("[1,2,3]").expect("result-value failure expectation should parse"),
                )),
            },
        )
    })
    .expect_err("result-value adapter should reject array length mismatch");
    assert!(panic_message(panic).contains("result value path `value` mismatch"));

    let lsp_stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}"#);
    let lsp_success = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [1,2]
"#,
    );
    assert_lsp_assertions(&context, &lsp_stdout, &lsp_success);
    let lsp_failure = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = [1,2,3]
"#,
    );
    let panic =
        std::panic::catch_unwind(|| assert_lsp_assertions(&context, &lsp_stdout, &lsp_failure))
            .expect_err("LSP adapter should reject array length mismatch");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.contains("value mismatch"));

    let mcp_stdout = r#"{"jsonrpc":"2.0","id":1,"result":[1,2]}
"#;
    let mcp_success = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [1,2]
"#,
    );
    assert_mcp_assertions(&context, mcp_stdout, &mcp_success, Path::new("."));
    let mcp_failure = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = [1,2,3]
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(&context, mcp_stdout, &mcp_failure, Path::new("."))
    })
    .expect_err("MCP adapter should reject array length mismatch");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.contains("value mismatch"));
}

#[test]
fn kind_and_nested_json_mismatches_retain_adapter_context() {
    let context = CaseRunContext {
        case_dir: Path::new("json-kind-nested-adapters"),
        run_number: 1,
    };

    let actual = parse_json(r#"{"selected":{"outer":{"nested":1}}}"#)
        .expect("JSON adapter input should parse");
    for expected in [r#""object""#, r#"{"outer":{"nested":2}}"#] {
        let assertion = JsonAssertion {
            path: "selected".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(expected).expect("JSON expectation should parse"),
            )),
        };
        let panic = std::panic::catch_unwind(|| assert_json_path(&context, &actual, &assertion))
            .expect_err("JSON adapter should reject mismatched value");
        let message = panic_message(panic);
        assert!(message.contains("JSON path `selected` mismatch"));
        assert!(message.contains("value mismatch"));
    }

    let rendered = parse_json(r#"{"rendered":"ByteOffset(2)"}"#)
        .expect("result-value adapter input should parse");
    for expected in [
        r#""ByteOffset""#,
        r#"{"constructor":"ByteOffset","value":3}"#,
    ] {
        let assertion = ResultValueAssertion {
            value_path: "rendered".to_string(),
            path: "value".to_string(),
            operation: Some(ValueAssertionOperation::Equals(
                parse_json(expected).expect("result-value expectation should parse"),
            )),
        };
        let panic =
            std::panic::catch_unwind(|| assert_result_value_path(&context, &rendered, &assertion))
                .expect_err("result-value adapter should reject mismatched value");
        let message = panic_message(panic);
        assert!(message.contains("result value path `value` mismatch"));
        assert!(message.contains("value mismatch"));
    }

    let lsp_stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"nested":1}}}"#);
    let lsp_failures = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result"
equals = "object"
[[lsp_assert]]
id = 1
path = "/result"
equals = {"outer":{"nested":2}}
"#,
    );
    let panic =
        std::panic::catch_unwind(|| assert_lsp_assertions(&context, &lsp_stdout, &lsp_failures))
            .expect_err("LSP adapter should reject mismatched values");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.matches("value mismatch").count() >= 2);

    let mcp_stdout = r#"{"jsonrpc":"2.0","id":1,"result":{"outer":{"nested":1}}}
"#;
    let mcp_failures = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = 1
path = "/result"
equals = "object"
[[mcp_assert]]
id = 1
path = "/result"
equals = {"outer":{"nested":2}}
"#,
    );
    let panic = std::panic::catch_unwind(|| {
        assert_mcp_assertions(&context, mcp_stdout, &mcp_failures, Path::new("."))
    })
    .expect_err("MCP adapter should reject mismatched values");
    let message = panic_message(panic);
    assert!(message.contains("response id 1 path \"/result\""));
    assert!(message.matches("value mismatch").count() >= 2);
}

#[test]
fn decoded_mcp_jsonl_assertions_cover_success_matrix() {
    let root = test_temp_root("mcp-jsonl");
    fs::write(root.join("main file.veln"), "").expect("workspace file should be written");
    let uri = path_to_file_uri(&root.join("main file.veln").canonicalize().unwrap());
    let stdout = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":\"alpha\",\"result\":{{\"value\":{{\"a/b\":\"slash\",\"m~n\":\"tilde\"}},\"items\":[\"first\",\"second\"],\"uri\":\"{uri}\",\"decimal\":1.0,\"exponent\":1e0}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{{\"object\":{{\"z\":1,\"a\":2}}}}}}\n{{\"jsonrpc\":\"2.0\",\"id\":9223372036854775808,\"result\":{{\"selected\":\"wide\"}}}}\n"
    );
    let source = r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/value/a~1b"
equals = "slash"
[[mcp_assert]]
id = "alpha"
path = "/result/value/m~0n"
equals = "tilde"
[[mcp_assert]]
id = "alpha"
path = "/result/items"
equals = ["first", "second"]
[[mcp_assert]]
id = "alpha"
path = "/result/items"
length = 2
[[mcp_assert]]
id = "alpha"
path = "/result/absent"
missing = true
[[mcp_assert]]
id = "alpha"
path = "/result/uri"
workspace_file_uri = "main file.veln"
[[mcp_assert]]
id = "alpha"
path = "/result/decimal"
equals = 1.0
[[mcp_assert]]
id = "alpha"
path = "/result/exponent"
equals = 1e0
[[mcp_assert]]
id = 2
path = "/result/object"
equals = {"a":2,"z":1}
[[mcp_assert]]
id = 9223372036854775808
path = "/result/selected"
equals = "wide"
"#;
    let assertions = parse_manifest(&root.join("case.toml"), source)
        .expectations
        .mcp_assertions;
    let context = CaseRunContext {
        case_dir: Path::new("decoded-mcp"),
        run_number: 1,
    };
    assert_mcp_assertions(&context, &stdout, &assertions, &root);

    let reversed = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/items"
equals = ["second", "first"]
"#,
    )
    .remove(0);
    let messages = decode_mcp_stdout(&stdout).expect("stream should decode");
    let error = evaluate_mcp_assertion(&messages, &reversed, &root)
        .expect_err("reversed array should fail equality");
    assert!(error.contains("value mismatch"));

    let integer_decimal = parsed_mcp_assertions(
        r#"command = ["mcp"]
exit = 0
[[mcp_assert]]
id = "alpha"
path = "/result/decimal"
equals = 1
"#,
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&messages, &integer_decimal, &root)
        .expect_err("integer spelling should not equal decimal spelling");
    assert!(error.contains("value mismatch"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn decoded_mcp_jsonl_rejection_matrix_reports_actionable_failures() {
    for (stdout, expected) in [
        ("{".to_string(), "invalid JSON"),
        ("[]\n".to_string(), "not a JSON-RPC object"),
    ] {
        let error = decode_mcp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }

    let root = test_temp_root("mcp-jsonl-reject");
    fs::write(root.join("main.veln"), "").expect("workspace file should be written");
    let duplicate_messages = decode_mcp_stdout(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[\"first\",\"second\"],\"text\":\"value\"}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":null}\n",
    )
    .expect("stream should decode");
    let messages = decode_mcp_stdout(
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"items\":[\"first\",\"second\"],\"text\":\"value\"}}\n",
    )
    .expect("stream should decode");

    for (fields, expected) in [
        (
            "id = 9\npath = \"/result\"\nmissing = true",
            "selected response was not found",
        ),
        (
            "id = 1\npath = \"/result/missing\"\nequals = null",
            "selected JSON path was not found",
        ),
        (
            "id = 1\npath = \"/result/missing\"\nlength = 0",
            "selected JSON path was not found",
        ),
        (
            "id = 1\npath = \"/result/text/value\"\nmissing = true",
            "invalid traversal",
        ),
        (
            "id = 1\npath = \"/result/text\"\nlength = 1",
            "requires a selected JSON array",
        ),
        (
            "id = 1\npath = \"/result/items\"\nlength = 1",
            "array length mismatch",
        ),
        (
            "id = 1\npath = \"/result/text\"\nmissing = true",
            "exists but should be missing",
        ),
    ] {
        let assertion = parsed_mcp_assertions(&format!(
            "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\n{fields}\n"
        ))
        .remove(0);
        let error = evaluate_mcp_assertion(&messages, &assertion, &root)
            .expect_err("assertion should report its failure");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
    let duplicate_assertion = parsed_mcp_assertions(
        "command = [\"mcp\"]\nexit = 0\n[[mcp_assert]]\nid = 1\npath = \"/result\"\nequals = null\n",
    )
    .remove(0);
    let error = evaluate_mcp_assertion(&duplicate_messages, &duplicate_assertion, &root)
        .expect_err("duplicate selected id should fail");
    assert!(error.contains("matched 2 responses"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn decoded_lsp_stream_selectors_and_json_pointer_object_matrix_succeed() {
    let response = r#"{"jsonrpc":"2.0","id":2,"result":{"":"empty","a.b":"dot","0":"numeric","a/b":"slash","m~n":"tilde","a b":"space","日本語":"unicode","~1":"escape-order","adj~/":"adjacent"}}"#;
    let first = r#"{"jsonrpc":"2.0","method":"note","params":{"value":"first"}}"#;
    let second = r#"{"jsonrpc":"2.0","method":"note","params":{"value":"second"}}"#;
    let stdout = format!(
        "{}{}{}",
        lsp_frame(response),
        lsp_frame(first),
        lsp_frame(second)
    );
    let source = r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 2
path = "/result/"
equals = "empty"
[[lsp_assert]]
id = 2
path = "/result/a.b"
equals = "dot"
[[lsp_assert]]
id = 2
path = "/result/0"
equals = "numeric"
[[lsp_assert]]
id = 2
path = "/result/a~1b"
equals = "slash"
[[lsp_assert]]
id = 2
path = "/result/m~0n"
equals = "tilde"
[[lsp_assert]]
id = 2
path = "/result/a b"
equals = "space"
[[lsp_assert]]
id = 2
path = "/result/日本語"
equals = "unicode"
[[lsp_assert]]
id = 2
path = "/result/~01"
equals = "escape-order"
[[lsp_assert]]
id = 2
path = "/result/adj~0~1"
equals = "adjacent"
[[lsp_assert]]
method = "note"
occurrence = 1
path = "/params/value"
equals = "second"
[[lsp_assert]]
method = "note"
path = "/params/value"
equals = "first"
[[lsp_assert]]
id = 2
path = ""
equals = {"jsonrpc":"2.0","id":2,"result":{"":"empty","a.b":"dot","0":"numeric","a/b":"slash","m~n":"tilde","a b":"space","日本語":"unicode","~1":"escape-order","adj~/":"adjacent"}}
"#;
    let assertions = parsed_lsp_assertions(source);
    let context = CaseRunContext {
        case_dir: Path::new("decoded-lsp"),
        run_number: 1,
    };
    assert_lsp_assertions(&context, &stdout, &assertions);
}

#[test]
fn decoded_lsp_transport_failure_matrix_rejects_invalid_complete_streams() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":null}"#;
    let valid = lsp_frame(response);
    for (stdout, expected) in [
        ("garbage".to_string(), "malformed or partial framing"),
        (
            "Content-Length: 10\r\n\r\n{}".to_string(),
            "partial frame body",
        ),
        (format!("{valid}garbage"), "trailing bytes"),
        (lsp_frame("{"), "invalid JSON"),
        (
            format!("{valid}{}", lsp_frame(response)),
            "duplicate response identifier 1",
        ),
    ] {
        let error = decode_lsp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
fn decoded_lsp_transport_preserves_header_and_body_failure_boundaries() {
    for (stdout, expected) in [
        (
            "Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}".to_string(),
            "missing Content-Length header",
        ),
        (
            "Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_string(),
            "duplicate Content-Length header",
        ),
        (
            "Content-Length: many\r\n\r\n{}".to_string(),
            "invalid Content-Length `many`",
        ),
        (lsp_frame("[]"), "is not a JSON-RPC object"),
        (
            "Content-Length: 1\r\n\r\né".to_string(),
            "frame body at byte offset 21 is not UTF-8",
        ),
    ] {
        let error = decode_lsp_stdout(&stdout).expect_err("stream should fail decoding");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
fn decoded_lsp_duplicate_ids_are_rejected_only_for_responses() {
    let requests = format!(
        "{}{}",
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"method":"first"}"#),
        lsp_frame(r#"{"jsonrpc":"2.0","id":1,"method":"second"}"#),
    );
    assert_eq!(
        decode_lsp_stdout(&requests)
            .expect("request identifiers may repeat in decoded server output")
            .len(),
        2
    );
}

#[test]
fn decoded_lsp_array_pointer_boundary_matrix_distinguishes_missing_and_invalid() {
    let value = parse_json(r#"["first","last"]"#).expect("array should parse");
    for (token, expected) in [("0", "first"), ("1", "last")] {
        match json_pointer(&value, &[token.to_string()]) {
            JsonPointerResult::Found(JsonValue::String(actual)) => assert_eq!(actual, expected),
            _ => panic!("array token {token:?} should resolve"),
        }
    }
    assert!(matches!(
        json_pointer(&value, &["2".to_string()]),
        JsonPointerResult::Missing
    ));
    for token in [
        "184467440737095516160",
        "01",
        "-1",
        "+1",
        " 1",
        "١",
        "-",
        "",
    ] {
        assert!(
            matches!(
                json_pointer(&value, &[token.to_string()]),
                JsonPointerResult::Invalid(_)
            ),
            "array token {token:?} should be invalid"
        );
    }
}

#[test]
fn decoded_lsp_operations_cover_string_kinds_missing_paths_and_selectors() {
    let stdout = lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha beta","number":2}}"#);
    let passing = parsed_lsp_assertions(
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/text"
contains = "beta"
[[lsp_assert]]
id = 1
path = "/result/absent"
missing = true
"#,
    );
    let messages = decode_lsp_stdout(&stdout).expect("stream should decode");
    for assertion in &passing {
        evaluate_lsp_assertion(&messages, assertion).expect("assertion should pass");
    }

    for (fields, expected) in [
        (
            "id = 1\npath = \"/result/number\"\ncontains = \"2\"",
            "requires a selected JSON string",
        ),
        (
            "id = 1\npath = \"/result/text/value\"\nmissing = true",
            "invalid traversal",
        ),
        (
            "id = 1\npath = \"/result/text\"\nmissing = true",
            "exists but should be missing",
        ),
        (
            "id = 9\npath = \"/result\"\nmissing = true",
            "selected response was not found",
        ),
        (
            "method = \"absent\"\npath = \"/params\"\nmissing = true",
            "selected notification was not found",
        ),
    ] {
        let assertion = parsed_lsp_assertions(&format!(
            "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\n{fields}\n"
        ))
        .remove(0);
        let error = evaluate_lsp_assertion(&messages, &assertion)
            .expect_err("assertion should report its failure");
        assert!(
            error.contains(expected),
            "expected `{expected}` in `{error}`"
        );
    }
}

#[test]
fn decoded_lsp_equals_file_uses_an_immutable_string_operand() {
    let root = test_temp_root("lsp-equals-file");
    let manifest_path = root.join("case.toml");
    fs::write(root.join("expected.txt"), "alpha\r\nbeta\n")
        .expect("expected text should be written");
    let manifest = parse_manifest(
        &manifest_path,
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result/text\"\nequals_file = \"expected.txt\"\n",
    );
    fs::write(root.join("expected.txt"), "changed")
        .expect("expected text should be changed after loading");
    let messages = decode_lsp_stdout(&lsp_frame(
        r#"{"jsonrpc":"2.0","id":1,"result":{"text":"alpha\r\nbeta\n"}}"#,
    ))
    .expect("stream should decode");
    evaluate_lsp_assertion(&messages, &manifest.expectations.lsp_assertions[0])
        .expect("snapshot should retain exact original text");

    let wrong_kind = parse_manifest(
        &manifest_path,
        "command = [\"lsp\"]\nexit = 0\n[[lsp_assert]]\nid = 1\npath = \"/result\"\nequals_file = \"expected.txt\"\n",
    );
    let error = evaluate_lsp_assertion(&messages, &wrong_kind.expectations.lsp_assertions[0])
        .expect_err("equals_file should reject a non-string value");
    assert!(error.contains("requires a selected JSON string"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn raw_stdout_and_decoded_lsp_failures_are_reported_independently() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[stdout]
contains = ["raw marker"]
[[lsp_assert]]
id = 1
path = "/result"
equals = "expected"
"#,
    );
    let context = CaseRunContext {
        case_dir: Path::new("independence"),
        run_number: 1,
    };
    let output = CapturedOutput {
        exit: Some(0),
        stdout: lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":"actual"}"#),
        stderr: String::new(),
    };
    let root = test_temp_root("lsp-independent-assertions");
    let panic = std::panic::catch_unwind(|| {
        manifest
            .expectations
            .assert_matches(&context, &output, &root)
    })
    .expect_err("both assertions should fail");
    let message = panic_message(panic);
    assert!(message.contains("raw marker"));
    assert!(message.contains("value mismatch"));

    let transport_output = CapturedOutput {
        exit: Some(0),
        stdout: "trailing transport bytes".to_string(),
        stderr: String::new(),
    };
    let panic = std::panic::catch_unwind(|| {
        manifest
            .expectations
            .assert_matches(&context, &transport_output, &root)
    })
    .expect_err("raw and transport assertions should fail");
    let message = panic_message(panic);
    assert!(message.contains("raw marker"));
    assert!(message.contains("malformed or partial framing"));
    assert!(!message.contains("value mismatch"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn repeated_run_failures_are_grouped_by_run_and_manifest_assertion_order() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"command = ["lsp"]
exit = 0
[[lsp_assert]]
id = 1
path = "/result/value"
equals = "first expected"
[[lsp_assert]]
id = 1
path = "/result/other"
equals = "second expected"
"#,
    );
    let outputs = [
        CapturedOutput {
            exit: Some(0),
            stdout: lsp_frame(
                r#"{"jsonrpc":"2.0","id":1,"result":{"value":"run one","other":"one"}}"#,
            ),
            stderr: String::new(),
        },
        CapturedOutput {
            exit: Some(7),
            stdout: format!(
                "{}trailing",
                lsp_frame(r#"{"jsonrpc":"2.0","id":1,"result":{"value":"run two"}}"#)
            ),
            stderr: String::new(),
        },
    ];
    let mut failures = Vec::new();
    let root = test_temp_root("lsp-repeated-failures");
    for (index, output) in outputs.iter().enumerate() {
        let context = CaseRunContext {
            case_dir: Path::new("repeat-lsp"),
            run_number: index + 1,
        };
        collect_run_failure(&mut failures, || {
            manifest
                .expectations
                .assert_matches(&context, output, &root)
        });
    }
    assert_eq!(failures.len(), 2);
    assert!(failures[0].contains("repeat-lsp run 1"));
    let first_position = failures[0]
        .find("/result/value")
        .expect("first assertion should be reported");
    let second_position = failures[0]
        .find("/result/other")
        .expect("second assertion should be reported");
    assert!(first_position < second_position);
    assert!(failures[1].contains("repeat-lsp run 2"));
    assert!(failures[1].contains("expected exit 0, got Some(7)"));
    assert!(failures[1].contains("trailing bytes"));
    fs::remove_dir_all(root).expect("test root should be removed");
}

fn assert_manifest_parse_error(source: &str, expected: &str) {
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("incomplete manifest section should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
}

fn assert_manifest_parse_error_without(source: &str, expected: &str, forbidden: &str) {
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("manifest should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
    assert!(
        !message.contains(forbidden),
        "expected panic to avoid `{forbidden}`, got `{message}`"
    );
}

#[test]
fn manifest_string_forms_decode_in_scalar_and_array_fields() {
    for (spelling, expected) in [
        (r#""basic\nvalue""#, "basic\nvalue"),
        (r#"'literal\nvalue'"#, r#"literal\nvalue"#),
        ("\"\"\"\nmultiline basic\"\"\"", "multiline basic"),
        ("'''\nmultiline literal'''", "multiline literal"),
        ("\"\"\"\"\"\"", ""),
        ("''''''", ""),
    ] {
        let manifest = parse_manifest(
            Path::new("case.toml"),
            &format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n"),
        );
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"basic\",\n  'literal', # inter-element comment\n  \"\"\"\nmultiline basic\"\"\",\n  '''\nmultiline literal''',\n]\nexit = 0\n",
    );
    assert_eq!(
        manifest.invocation.command,
        ["basic", "literal", "multiline basic", "multiline literal"]
    );
}

#[test]
fn manifest_basic_string_escape_matrix_decodes_unicode_scalars() {
    for (escape, expected) in [
        (r#"\b"#, "\u{08}"),
        (r#"\t"#, "\t"),
        (r#"\n"#, "\n"),
        (r#"\f"#, "\u{0c}"),
        (r#"\r"#, "\r"),
        (r#"\""#, "\""),
        (r#"\\"#, "\\"),
        (r#"\u03B1"#, "α"),
        (r#"\U0001F642"#, "🙂"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = \"{escape}\"\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
fn manifest_invalid_string_token_matrix_rejects_toml_boundaries() {
    for (spelling, fact) in [
        (r#""\x""#, "unsupported manifest string escape"),
        (r#""\u12""#, "incomplete Unicode escape"),
        (r#""\u12x4""#, "invalid hexadecimal digit"),
        (r#""\uD800""#, "not a scalar value"),
        (r#""\U00110000""#, "not a scalar value"),
        ("\"control \u{1}\"", "prohibited control character"),
        ("'control \u{7f}'", "prohibited control character"),
        (
            "\"\"\"invalid\"\"\"\"\"\"",
            "invalid multiline string quote run",
        ),
        ("'''invalid''''''", "invalid multiline string quote run"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        assert_manifest_parse_error(&source, fact);
    }
}

#[test]
fn manifest_multiline_strings_preserve_layout_folding_and_quote_runs() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\n \t beta\\  \n\n\t gamma\"\"\"\nexit = 0\n",
    );
    assert_eq!(manifest.invocation.stdin.as_deref(), Some("alphabetagamma"));

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\\\nbeta\"\"\"\nexit = 0\n",
    );
    assert_eq!(manifest.invocation.stdin.as_deref(), Some("alpha\\\nbeta"));

    for (spelling, expected) in [
        ("\"\"\"one\"\"two\"\"\"\"", "one\"\"two\""),
        ("'''one''two''''", "one''two'"),
        (
            "'''\n  [section]\n\tkey = #,\n '''",
            "  [section]\n\tkey = #,\n ",
        ),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
fn manifest_multiline_indentation_and_closing_delimiters_are_value_bytes() {
    for (spelling, expected) in [
        (
            "\"\"\"\n\tleft\n  middle\n\t \nright\"\"\"",
            "\tleft\n  middle\n\t \nright",
        ),
        (
            "'''\n\tleft\n  middle\n\t \nright'''",
            "\tleft\n  middle\n\t \nright",
        ),
        ("\"\"\"\nvalue\n\"\"\"", "value\n"),
        ("\"\"\"\nvalue\n  \"\"\"", "value\n  "),
        ("\"\"\"\nvalue\"\"\"", "value"),
        ("'''\nvalue\n'''", "value\n"),
        ("'''\nvalue\n\t'''", "value\n\t"),
        ("'''\nvalue'''", "value"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
fn manifest_multiline_array_placement_does_not_indent_values() {
    let scalar = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nvalue\n\"\"\"\nexit = 0\n",
    );
    let shallow_array = parse_manifest(
        Path::new("case.toml"),
        "command = [\n\"\"\"\nvalue\n\"\"\"\n]\nexit = 0\n",
    );
    let deep_array = parse_manifest(
        Path::new("case.toml"),
        "command = [\n        \"\"\"\nvalue\n\"\"\"\n]\nexit = 0\n",
    );

    assert_eq!(scalar.invocation.stdin.as_deref(), Some("value\n"));
    assert_eq!(shallow_array.invocation.command, ["value\n"]);
    assert_eq!(deep_array.invocation.command, ["value\n"]);
}

#[test]
fn manifest_multiline_quote_run_matrix_preserves_terminal_quotes() {
    for (spelling, expected) in [
        ("\"\"\"one\"two\"\"\"", "one\"two"),
        ("\"\"\"one\"\"two\"\"\"", "one\"\"two"),
        ("\"\"\"tail\"\"\"", "tail"),
        ("\"\"\"tail\"\"\"\"", "tail\""),
        ("\"\"\"tail\"\"\"\"\"", "tail\"\""),
        ("'''one'two'''", "one'two"),
        ("'''one''two'''", "one''two"),
        ("'''tail'''", "tail"),
        ("'''tail''''", "tail'"),
        ("'''tail'''''", "tail''"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
fn manifest_physical_newline_matrix_normalizes_multiline_values() {
    for delimiters in [("\"\"\"", "\"\"\""), ("'''", "'''")] {
        let lf = format!(
            "command = [\"check\"]\nstdin = {}\nfirst\nsecond{}\nexit = 0\n",
            delimiters.0, delimiters.1
        );
        let crlf = lf.replace('\n', "\r\n");
        let mixed = lf.replacen('\n', "\r\n", 2);
        for source in [lf, crlf, mixed] {
            let manifest = parse_manifest(Path::new("case.toml"), &source);
            assert_eq!(manifest.invocation.stdin.as_deref(), Some("first\nsecond"));
        }
    }

    for opening in ["\n", "\r\n"] {
        let source = format!(
            "command = [\"check\"]\nstdin = \"\"\"{opening}{opening}value\"\"\"\nexit = 0\n"
        );
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some("\nvalue"));
    }
}

#[test]
fn manifest_multiline_basic_folding_accepts_lf_crlf_and_mixed_lines() {
    let lf = "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\n \t beta\\  \n\n\t gamma\"\"\"\nexit = 0\n";
    let crlf = lf.replace('\n', "\r\n");
    let mixed = lf.replacen('\n', "\r\n", 4);
    for source in [lf.to_string(), crlf, mixed] {
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some("alphabetagamma"));
    }
}

#[test]
fn manifest_physical_newlines_match_escaped_lf_not_escaped_crlf() {
    let escaped_lf = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"first\\n  second\\n\"\nexit = 0\n",
    );
    for source in [
        "command = [\"check\"]\nstdin = \"\"\"\nfirst\n  second\n\"\"\"\nexit = 0\n".to_string(),
        "command = [\"check\"]\r\nstdin = \"\"\"\r\nfirst\r\n  second\r\n\"\"\"\r\nexit = 0\r\n"
            .to_string(),
    ] {
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin, escaped_lf.invocation.stdin);
    }

    let escaped_crlf = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"first\\r\\n  second\\r\\n\"\nexit = 0\n",
    );
    assert_ne!(escaped_crlf.invocation.stdin, escaped_lf.invocation.stdin);
}

#[test]
fn manifest_field_directed_containers_keep_array_and_json_grammars_distinct() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"check\",\n  'main.veln',\n]\nexit = 0\n\n[stdout]\ncontains = [\n  \"a,#[]{}\",\n  '''b''',\n]\n\n[[json_assert]]\npath = \"payload\"\nequals = {\n  \"array\": [1, {\"ok\": true}],\n  \"text\": \"a,#[]{}\"\n}\n",
    );
    assert_eq!(manifest.invocation.command, ["check", "main.veln"]);
    assert_eq!(manifest.expectations.stdout.contains, ["a,#[]{}", "b"]);
    let Some(ValueAssertionOperation::Equals(expected)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(
        expected.to_compact_string(),
        r#"{"array":[1,{"ok":true}],"text":"a,#[]{}"}"#
    );

    for invalid in [
        "command = [\"check\", 1]\nexit = 0\n",
        "command = [\"check\", true]\nexit = 0\n",
        "command = [\"check\", null]\nexit = 0\n",
        "command = [\"check\", [\"nested\"]]\nexit = 0\n",
        "command = [\"check\", {\"nested\":\"object\"}]\nexit = 0\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [1, # no comments\n2]\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [1,]\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = ['literal']\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\"\"\"multi\"\"\"]\n",
    ] {
        assert_manifest_parse_error(invalid, "case.toml:");
    }

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = '''\ntext\n'''",
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::String(
            "text\n".to_string()
        )))
    );
}

#[test]
fn manifest_string_array_layout_matrix_accepts_schema_selected_fields() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"check\",\n  'main.veln',\n]\nexit = 0\n[stdout]\ncontains = []\nnot_contains = [\n  \"basic\",\n  'literal',\n  \"\"\"\nmultiline basic\"\"\",\n  '''\nmultiline literal''',\n  # trailing comment\n]\n[help]\ncommands = [\"check\",]\narguments = []\noptions = [\n  \"--json\",\n]\ncontains = [\n  \"done\",\n]\n",
    );

    assert_eq!(manifest.invocation.command, ["check", "main.veln"]);
    assert!(manifest.expectations.stdout.contains.is_empty());
    assert_eq!(
        manifest.expectations.stdout.not_contains,
        ["basic", "literal", "multiline basic", "multiline literal"]
    );
    let help = manifest.expectations.help.as_ref().expect("help section");
    assert_eq!(help.commands, ["check"]);
    assert!(help.arguments.is_empty());
    assert_eq!(help.options, ["--json"]);
    assert_eq!(help.contains, ["done"]);
}

#[test]
fn manifest_array_boundaries_keep_punctuation_inside_string_tokens() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nexit = 0\n[stdout]\ncontains = [\n  \"brackets [] braces {} comma , hash # quote \\\"\",\n]\n[[json_assert]]\npath = \"x\"\nequals = [\n  \"brackets [] braces {} comma , hash # quote \\\"\"\n]\n",
    );

    let expected = "brackets [] braces {} comma , hash # quote \"";
    assert_eq!(manifest.expectations.stdout.contains, [expected]);
    let Some(ValueAssertionOperation::Equals(value)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(value.to_compact_string(), format!("[{expected:?}]"));
}

#[test]
fn manifest_container_trailing_tokens_and_local_errors_take_precedence() {
    for (value, line) in [
        ("\"\"\"\ntext\n\"\"\" trailing", 4),
        ("[\n  \"check\"\n] trailing", 3),
        ("{\n  \"ok\": true\n} trailing", 7),
    ] {
        let source = if value.starts_with('{') {
            format!(
                "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {value}\n"
            )
        } else if value.starts_with('[') {
            format!("command = {value}\nexit = 0\n")
        } else {
            format!("command = [\"check\"]\nstdin = {value}\nexit = 0\n")
        };
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), &source))
            .expect_err("trailing token should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("unexpected token after completed manifest value"),
            "unexpected failure: {message}"
        );
        assert!(
            message.contains(&format!("case.toml:{line}:")),
            "unexpected error line: {message}"
        );
    }

    let source = "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\": # local JSON error\n";
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("local JSON error should be rejected");
    let message = panic_message(panic);
    assert!(message.contains("case.toml:6: invalid json assertion value"));
    assert!(!message.contains("unterminated container"));

    for (source, line, fact) in [
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\"\n",
            7,
            "expected `:`",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\":\n",
            7,
            "unexpected end of input",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\n  1,\n",
            7,
            "unexpected end of input",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"nested\": [\n    1\n",
            6,
            "unterminated container; expected `]`",
        ),
    ] {
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
            .expect_err("incomplete JSON container should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains(&format!("case.toml:{line}:")),
            "unexpected error line: {message}"
        );
        assert!(
            message.contains(fact),
            "expected `{fact}` in error, got `{message}`"
        );
        if fact != "unterminated container; expected `]`" {
            assert!(
                !message.contains("unterminated container"),
                "local JSON error was replaced by outer delimiter error: {message}"
            );
        }
    }
}

#[test]
fn manifest_syntax_errors_report_exact_physical_lines() {
    for (source, line, fact) in [
        (
            "command = [\"check\"]\nstdin = \"\"\"\nok\nbad \\q\n\"\"\"\nexit = 0\n",
            4,
            "unsupported manifest string escape",
        ),
        (
            "command = [\n  \"check\"\n  \"main.veln\"\n]\nexit = 0\n",
            3,
            "expected `,` before string array element",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\n  1,\n  # forbidden\n  2\n]\n",
            7,
            "invalid json assertion value",
        ),
        (
            "command = [\"check\"] trailing\nexit = 0\n",
            1,
            "unexpected token after completed manifest value",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"nested\": [\n    1\n",
            6,
            "unterminated container; expected `]`",
        ),
        (
            "command = [\"check\"]\nstdin = \"\\u12\nexit = 0\n",
            2,
            "incomplete Unicode escape",
        ),
        (
            "command = [\"check\"]\r\nstdin = \"\\u12\r\nexit = 0\r\n",
            2,
            "incomplete Unicode escape",
        ),
    ] {
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
            .expect_err("invalid manifest should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains(&format!("case.toml:{line}: {fact}")),
            "expected line {line} and `{fact}`, got `{message}`"
        );
    }

    for source in [
        "command = [\"check\"]\rstdin = \"x\"\nexit = 0\n",
        "command = [\"check\"] # comment\rstill comment\nexit = 0\n",
        "command = [\"check\"]\nstdin = '''a\rb'''\nexit = 0\n",
        "command = [\"check\"]\nstdin = \"\"\"a\\\rb\"\"\"\nexit = 0\n",
    ] {
        assert_manifest_parse_error(source, "lone carriage return");
    }
}

#[test]
fn manifest_syntax_errors_report_equivalent_lines_with_lf_crlf_and_mixed_prefixes() {
    let lf_prefix = "command = [\"check\"]\nexit = 0\n[stdout]\n";
    let crlf_prefix = lf_prefix.replace('\n', "\r\n");
    let mixed_prefix = lf_prefix.replacen('\n', "\r\n", 2);
    for prefix in [lf_prefix.to_string(), crlf_prefix, mixed_prefix] {
        let source = format!("{prefix}contains = [\n  \"ok\"\n  \"missing comma\"\n]\n");
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), &source))
            .expect_err("missing comma should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("case.toml:6: expected `,` before string array element"),
            "unexpected error line: {message}"
        );
    }
}

#[test]
fn manifest_binary_fixtures_parse_named_bytes_and_errors() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[binary_fixture]]
name = "short-u24"
hex = "0001"
consumed = 2
byte_offset = 2
expected_count = 3
available_count = 2
readiness = "need_bytes"
field_path = []

[[binary_fixture]]
name = "invalid-frame-kind"
schema = "DemoPacket"
hex = "ff0001"
consumed = 1
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"DemoPacket"},{"kind":"field","name":"kind"}]

[[binary_fixture]]
name = "bad-separator"
error = "fixture.hex.invalid_character"
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let fixtures = &manifest.expectations.binary_fixtures;
    assert_eq!(fixtures.len(), 3);
    assert_eq!(fixtures[0].name, "short-u24");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().hex, "0001");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().bytes, [0, 1]);
    assert_eq!(fixtures[0].consumed, Some(2));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[0]),
        "fixture short-u24 hex 0001 count 2 consumed 2 offset 2 expected 3 available 2 readiness need_bytes field_path []"
    );
    assert_eq!(fixtures[1].name, "invalid-frame-kind");
    assert_eq!(fixtures[1].schema.as_deref(), Some("DemoPacket"));
    assert_eq!(fixtures[1].bytes.as_ref().unwrap().hex, "ff0001");
    assert_eq!(fixtures[1].consumed, Some(1));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[1]),
        "fixture invalid-frame-kind hex ff0001 count 3 consumed 1 diagnostic schema.invalid_field_value offset 0 field_path [{\"kind\":\"schema\",\"name\":\"DemoPacket\"},{\"kind\":\"field\",\"name\":\"kind\"}]"
    );
    assert_eq!(fixtures[2].name, "bad-separator");
    assert_eq!(
        fixtures[2].error.as_deref(),
        Some("fixture.hex.invalid_character")
    );
    assert_eq!(
        expected_binary_fixture_line(&fixtures[2]),
        "fixture bad-separator error fixture.hex.invalid_character"
    );
}

#[test]
fn binary_fixture_schema_references_resolve_from_command_sources() {
    let root = test_temp_root("fixture-schema-references");
    write_fixture_schema_sources(&root);
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "local-private"
schema = "LocalPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"LocalPacket"}]

[[binary_fixture]]
name = "imported-public"
schema = "wire::PublicPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]

[[binary_fixture]]
name = "imported-alias"
schema = "facade::AliasPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]
"#,
    );

    manifest.validate_fixture_schema_references(&root);
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn binary_fixture_schema_references_reject_wrong_targets() {
    assert_fixture_schema_error(
        "MissingPacket",
        Some(r#"[{"kind":"schema","name":"MissingPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `MissingPacket`",
    );
    assert_fixture_schema_error(
        "PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "unresolved binary_fixture 0 schema reference `PrivatePacket`",
    );
    assert_fixture_schema_error(
        "wire::PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "binary_fixture 0 schema reference `wire::PrivatePacket` is private",
    );
    assert_fixture_schema_error(
        "wire::make_packet",
        Some(r#"[{"kind":"schema","name":"make_packet"}]"#),
        "binary_fixture 0 schema reference `wire::make_packet` is a function, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketShape",
        Some(r#"[{"kind":"schema","name":"PacketShape"}]"#),
        "binary_fixture 0 schema reference `wire::PacketShape` is a type, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketCodec",
        Some(r#"[{"kind":"schema","name":"PacketCodec"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::PacketCodec`",
    );
    assert_fixture_schema_error(
        "wire::byte_decode_public_packet",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::byte_decode_public_packet`",
    );
    assert_fixture_schema_error(
        "other::PublicPacket",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `other::PublicPacket`",
    );
    assert_fixture_schema_error(
        "wire::PublicPacket",
        Some(r#"[{"kind":"schema","name":"OtherPacket"}]"#),
        "binary_fixture 0 `field_path` first segment must name schema `PublicPacket`",
    );
}

#[test]
fn manifest_result_value_assertions_parse_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.details.value"
path = "value.id"
equals = "codec.incomplete_input"

[[result_value_assert]]
value_path = "error.details.value"
path = "value.detail.preview"
missing = true
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let assertions = &manifest.expectations.result_value_assertions;
    assert_eq!(assertions.len(), 2);
    assert_eq!(assertions[0].value_path, "error.details.value");
    assert_eq!(assertions[0].path, "value.id");
    assert_eq!(
        assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::String(
            "codec.incomplete_input".to_string()
        )))
    );
    assert_eq!(
        assertions[1].operation,
        Some(ValueAssertionOperation::Missing)
    );
}

#[test]
fn result_value_parser_exposes_runtime_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(codec.incomplete_input, byte read requires 3 bytes but view has 2, RuntimeByteDiagnostic(ByteOffset(2), Cons(RuntimeDiagnosticFieldPathSegment(schema, Payload), Cons(RuntimeDiagnosticFieldPathSegment(field, body), Nil)), RuntimeByteCountFacts(ByteCount(3), ByteCount(2), need_bytes), RuntimeBytePreview(0001, ByteCount(2), ByteCount(2), false)))",
    )
    .expect("runtime diagnostic value should parse");

    assert_eq!(
        json_path(&parsed, "constructor"),
        Some(&JsonValue::String("Err".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.constructor"),
        Some(&JsonValue::String("RuntimeDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("body".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.facts.expected_count.value"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.preview.truncated"),
        Some(&JsonValue::Bool(false))
    );
}

#[test]
fn result_value_parser_exposes_runtime_value_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(schema.encode_value_unrepresentable, encode value is unrepresentable, RuntimeValueDiagnostic(Cons(RuntimeDiagnosticFieldPathSegment(schema, RuntimeValuePacket), Cons(RuntimeDiagnosticFieldPathSegment(field, value), Nil)), value must be between 0 and 255))",
    )
    .expect("runtime value diagnostic should parse");

    assert_eq!(
        json_path(&parsed, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeValueDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("value".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.reason"),
        Some(&JsonValue::String(
            "value must be between 0 and 255".to_string()
        ))
    );
}

#[test]
fn veln_value_parser_preserves_constructor_field_kinds() {
    let parsed = parse_veln_value(
        "RuntimeByteDiagnostic(ByteOffset(4), Cons(RuntimeDiagnosticFieldPathSegment(schema, Packet), Nil), RuntimeByteReasonFacts(invalid byte), RuntimeBytePreview(ff, 1, 3, true))",
    )
    .expect("runtime byte diagnostic should parse");

    assert_eq!(
        json_path(&parsed, "byte_offset.value"),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&parsed, "field_path.0.name"),
        Some(&JsonValue::String("Packet".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "facts.reason"),
        Some(&JsonValue::String("invalid byte".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "preview.encoding"),
        Some(&JsonValue::String("hex".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "preview.truncated"),
        Some(&JsonValue::Bool(true))
    );
}

#[test]
fn result_value_parser_exposes_hpack_fixture_runtime_diagnostics() {
    let fixture = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.malformed_raw_string_value, HPACK fixture malformed raw string value at byte offset 9, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(9, 5, 8, fixture HPACK raw string value, hpack_fixture, ByteChunk([Byte(8), Byte(3), Byte(50), Byte(31), Byte(48)]))))",
    )
    .expect("HPACK fixture runtime diagnostic value should parse");
    let dynamic_index = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_index_out_of_range, HPACK dynamic index out of range at byte offset 27, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(27, 1, 190, 0, 0, fixture dynamic indexed header, hpack_fixture, ByteChunk([Byte(190)]))))",
    )
    .expect("HPACK dynamic-index runtime diagnostic value should parse");
    let dynamic_name = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_name_continuation_out_of_range, HPACK dynamic-name continuation out of range at byte offset 98, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(98, 8, 127, 3, 3, fixture dynamic-name continuation range, hpack_fixture, ByteChunk([Byte(127), Byte(2), Byte(5), Byte(80), Byte(65), Byte(84), Byte(67), Byte(72)]))))",
    )
    .expect("HPACK dynamic-name runtime diagnostic value should parse");
    let table_size = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_not_at_start, HPACK fixture table-size update after header field at byte offset 10, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(10, 2, 62, 30, 1, 1, hpack-fixture, fixture HPACK table-size update at header block start, hpack_fixture, ByteChunk([Byte(130), Byte(62)]))))",
    )
    .expect("HPACK table-size runtime diagnostic value should parse");
    let table_size_malformed = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_malformed, HPACK fixture malformed table-size update integer at byte offset 77, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(77, 2, 63, fixture HPACK malformed table-size update integer, hpack_fixture, ByteChunk([Byte(63), Byte(128)]))))",
    )
    .expect("HPACK table-size malformed runtime diagnostic value should parse");

    assert_eq!(
        json_path(&fixture, "value.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2HpackDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.expected_fixture"),
        Some(&JsonValue::String(
            "fixture HPACK raw string value".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.preview.bytes.2.value"),
        Some(&JsonValue::Number(50))
    );
    assert_eq!(
        json_path(
            &dynamic_index,
            "value.detail.detail.requested_dynamic_index"
        ),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&dynamic_index, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(190))
    );
    assert_eq!(
        json_path(&dynamic_name, "value.detail.detail.requested_dynamic_index"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &dynamic_name,
            "value.detail.detail.dynamic_table_entry_count"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &table_size,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(30))
    );
    assert_eq!(
        json_path(&table_size, "value.detail.detail.active_state"),
        Some(&JsonValue::String("hpack-fixture".to_string()))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.expected_fixture"
        ),
        Some(&JsonValue::String(
            "fixture HPACK malformed table-size update integer".to_string()
        ))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.preview.bytes.1.value"
        ),
        Some(&JsonValue::Number(128))
    );
}

#[test]
fn result_value_parser_exposes_http2_peer_limit_runtime_diagnostics() {
    let header_table = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.header_table_size_exceeded, HTTP/2 header table size exceeds receive maximum at byte offset 35, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(35, 289, 160, 9, 1, local_configuration, hpack_dynamic_table_size_update, ByteChunk([Byte(63), Byte(129), Byte(1)]))))",
    )
    .expect("header-table runtime diagnostic value should parse");
    let concurrent_streams = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.concurrent_streams_exceeded, HTTP/2 concurrent stream receive limit exceeded at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(9, 3, 2, 1, server, open-stream, local_configuration, peer_created_stream_receive_limit, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(3)]))))",
    )
    .expect("concurrent-stream runtime diagnostic value should parse");

    assert_eq!(
        json_path(&header_table, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeHttp2Diagnostic".to_string()))
    );
    assert_eq!(
        json_path(
            &header_table,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(289))
    );
    assert_eq!(
        json_path(&header_table, "value.detail.detail.preview.bytes.1.value"),
        Some(&JsonValue::Number(129))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.attempted_concurrent_stream_count"
        ),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.receive_limit_provenance"
        ),
        Some(&JsonValue::String("local_configuration".to_string()))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.preview.bytes.8.value"
        ),
        Some(&JsonValue::Number(3))
    );
}

#[test]
fn result_value_parser_exposes_http2_data_flow_content_length_diagnostics() {
    let data_padding = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_data_padding, HTTP/2 invalid DATA padding at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(9, 1, 2, 0, open-stream, rfc9113_data_padding, ByteChunk([Byte(2)]))))",
    )
    .expect("DATA padding runtime diagnostic value should parse");
    let flow_control = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.flow_control_window_exceeded, HTTP/2 flow-control window exceeded at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(0, 4, 3, 0, 1, open-stream, stream_receive_window, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("flow-control runtime diagnostic value should parse");
    let content_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.content_length_mismatch, HTTP/2 content-length body length mismatch at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(9, 0, 1, 5, 3, open-stream, rfc9113_content_length_body, ByteChunk([Byte(170), Byte(187), Byte(204)]))))",
    )
    .expect("content-length runtime diagnostic value should parse");

    assert_eq!(
        json_path(&data_padding, "value.detail.detail.pad_length"),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(&data_padding, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.allowed_window_credit"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("stream_receive_window".to_string()))
    );
    assert_eq!(
        json_path(
            &content_length,
            "value.detail.detail.expected_content_length"
        ),
        Some(&JsonValue::Number(5))
    );
    assert_eq!(
        json_path(&content_length, "value.detail.detail.observed_body_length"),
        Some(&JsonValue::Number(3))
    );
}

#[test]
fn result_value_parser_exposes_http2_header_list_runtime_diagnostics() {
    let request = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_request_header_list, HTTP/2 request header list is missing :method at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :method, headers, request-headers, rfc9113_request_pseudo_headers, ByteChunk([Byte(130), Byte(132), Byte(134)]))))",
    )
    .expect("request header-list runtime diagnostic value should parse");
    let response = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_response_header_list, HTTP/2 response header list is missing :status at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :status, server, response-headers, rfc9113_response_pseudo_headers, ByteChunk([Byte(136)]))))",
    )
    .expect("response header-list runtime diagnostic value should parse");

    assert_eq!(
        json_path(&request, "value.detail.detail.failed_header_fact"),
        Some(&JsonValue::String(
            "missing_required_pseudo_header".to_string()
        ))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.decoded_header_names"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.header_name"),
        Some(&JsonValue::String(":status".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(136))
    );
}

#[test]
fn result_value_parser_exposes_http2_preface_runtime_diagnostics() {
    let partial = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.partial_preface, HTTP/2 input ended with partial client connection preface at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(0, 12, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(42), Byte(32)]))))",
    )
    .expect("partial preface runtime diagnostic value should parse");
    let invalid = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_preface, HTTP/2 invalid client connection preface at byte offset 4, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(4, 42, 43, 4, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(43)]))))",
    )
    .expect("invalid preface runtime diagnostic value should parse");

    assert_eq!(
        json_path(&partial, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(12))
    );
    assert_eq!(
        json_path(&partial, "value.detail.detail.active_state"),
        Some(&JsonValue::String("connection-preface".to_string()))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.expected_byte"),
        Some(&JsonValue::Number(42))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.actual_byte"),
        Some(&JsonValue::Number(43))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
fn result_value_parser_exposes_http2_control_runtime_diagnostics() {
    let closed = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.closed_with_pending, HTTP/2 input ended with 4 pending byte(s) at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(0, 4, none, 0, 0, 0, 0, none, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("closed-input runtime diagnostic value should parse");
    let continuation = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.continuation_expected, HTTP/2 expected CONTINUATION frame at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(9, 0, 1, 1, 1, 0, headers, 3, rfc9113_continuation_sequence, ByteChunk([Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("continuation runtime diagnostic value should parse");
    let frame_kind = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_frame_kind, HTTP/2 invalid frame kind at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(0, 0, 1, 1, idle-stream, idle_streams_require_headers, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid frame-kind runtime diagnostic value should parse");
    let stream_id = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_stream_id, HTTP/2 invalid stream id at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(0, 1, 2, nonzero client-initiated stream id, server, stream-id-domain, server_receives_client_initiated_streams, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid stream-id runtime diagnostic value should parse");

    assert_eq!(
        json_path(&closed, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(
            &continuation,
            "value.detail.detail.accumulated_header_block_bytes"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "rfc9113_continuation_sequence".to_string()
        ))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.expected_frame_kind"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.active_state"),
        Some(&JsonValue::String("idle-stream".to_string()))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.required_stream_id_domain"),
        Some(&JsonValue::String(
            "nonzero client-initiated stream id".to_string()
        ))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "server_receives_client_initiated_streams".to_string()
        ))
    );
}

#[test]
fn result_value_parser_exposes_http2_limit_and_shutdown_runtime_diagnostics() {
    let payload_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_payload_length, HTTP/2 invalid payload length at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(9, 8, 0, 3, 4, connection-flow-control, rfc9113_window_update_payload_length, ByteChunk([Byte(1), Byte(2), Byte(3)]))))",
    )
    .expect("invalid payload-length runtime diagnostic value should parse");
    let settings_value = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.settings_value_out_of_range, HTTP/2 SETTINGS value outside accepted range at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitSettingsValueDiagnostic(9, 5, SETTINGS_MAX_FRAME_SIZE, 16383, 16384, 16777215, peer_settings, ByteChunk([Byte(0), Byte(5)]))))",
    )
    .expect("settings value runtime diagnostic value should parse");
    let window_update = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_window_update_increment, HTTP/2 invalid WINDOW_UPDATE increment at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(0, 0, 0, 1, 2147483647, connection-flow-control, window_update_increment_nonzero, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("window-update runtime diagnostic value should parse");
    let priority = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_priority_dependency, HTTP/2 invalid PRIORITY dependency at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(0, 1, 1, stream-control, rfc9113_priority_dependency, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(15)]))))",
    )
    .expect("priority runtime diagnostic value should parse");
    let goaway = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.stream_after_goaway, HTTP/2 stream opened after graceful shutdown at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(9, 7, 5, graceful_shutdown, server, goaway_last_stream_id, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(7)]))))",
    )
    .expect("stream-after-GOAWAY runtime diagnostic value should parse");

    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.observed_payload_length"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.expected_payload_length"
        ),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.setting_name"),
        Some(&JsonValue::String("SETTINGS_MAX_FRAME_SIZE".to_string()))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.peer_limit_provenance"),
        Some(&JsonValue::String("peer_settings".to_string()))
    );
    assert_eq!(
        json_path(
            &window_update,
            "value.detail.detail.accepted_max_window_increment"
        ),
        Some(&JsonValue::Number(2147483647))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.dependency_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.shutdown_state"),
        Some(&JsonValue::String("graceful_shutdown".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("goaway_last_stream_id".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
fn manifest_output_chunk_lists_parse_ordered_hex_chunks() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["0001ff", "00040000000f000001"]

[[output_chunk_list]]
name = "empty-chunk"
chunks = [""]

[[output_chunk_list]]
name = "no-output"
chunks = []
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let chunk_lists = &manifest.expectations.output_chunk_lists;
    assert_eq!(chunk_lists.len(), 3);
    assert_eq!(chunk_lists[0].name, "protocol-output");
    assert_eq!(
        chunk_lists[0].chunks.as_ref().unwrap()[0].bytes,
        [0, 1, 255]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[0]),
        [
            "output_chunk_list protocol-output count 2",
            "output_chunk protocol-output index 0 hex \"0001ff\" count 3",
            "output_chunk protocol-output index 1 hex \"00040000000f000001\" count 9",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[1]),
        [
            "output_chunk_list empty-chunk count 1",
            "output_chunk empty-chunk index 0 hex \"\" count 0",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[2]),
        ["output_chunk_list no-output count 0"]
    );
}

#[test]
#[should_panic(expected = "expected lowercase hex")]
fn manifest_output_chunk_lists_reject_uppercase_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["00FF"]
"#,
    );
}

#[test]
#[should_panic(expected = "expected complete lowercase hex byte pairs")]
fn manifest_output_chunk_lists_reject_odd_length_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["001"]
"#,
    );
}

#[test]
fn missing_tool_setup_leaves_isolated_tool_path_empty() {
    let root = test_temp_root("missing-tool");
    setup_tool(&root, "java", ToolAvailability::Missing);

    let entries = fs::read_dir(&root)
        .expect("tool root should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("tool entries should be readable");
    assert!(entries.is_empty());

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn fake_success_tool_setup_installs_success_launcher() {
    let root = test_temp_root("fake-tool");
    setup_tool(&root, "java", ToolAvailability::FakeSuccess);

    let output = Command::new(fake_tool_path(&root, "java"))
        .output()
        .expect("fake tool should run");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");

    fs::remove_dir_all(root).expect("test root should be removed");
}

fn assert_fixture_schema_error(schema: &str, field_path: Option<&str>, expected: &str) {
    let root = test_temp_root("fixture-schema-error");
    write_fixture_schema_sources(&root);
    let field_path = field_path
        .map(|value| format!("field_path = {value}"))
        .unwrap_or_default();
    let manifest = parse_manifest(
        Path::new("case.toml"),
        &format!(
            r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "schema-reference"
schema = "{schema}"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
{field_path}
"#
        ),
    );
    let panic = std::panic::catch_unwind(|| manifest.validate_fixture_schema_references(&root))
        .expect_err("schema reference should be rejected");
    let message = panic_message(panic);
    assert!(
        message.contains(expected),
        "expected panic to contain `{expected}`, got `{message}`"
    );
    fs::remove_dir_all(root).expect("test root should be removed");
}

fn write_fixture_schema_sources(root: &Path) {
    fs::write(
        root.join("main.veln"),
        r#"
use wire
use facade

schema LocalPacket
	format binary

	length: UInt8
end
"#,
    )
    .expect("main source should be written");
    fs::write(
        root.join("wire.veln"),
        r#"
pub schema PublicPacket
	format binary

	length: UInt8
end

schema PrivatePacket
	format binary

	length: UInt8
end

pub fn make_packet() -> Int
	1
end

pub type PacketShape
	pub Packet(Int)
end

"#,
    )
    .expect("wire source should be written");
    fs::write(
        root.join("facade.veln"),
        r#"
use wire

pub schema AliasPacket = wire::PublicPacket
"#,
    )
    .expect("facade source should be written");
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return message.to_string();
    }
    "non-string panic".to_string()
}

fn test_temp_root(name: &str) -> PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-toolchain-harness-test-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test root should be created");
    root
}

#[cfg(windows)]
fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(format!("{name}.cmd"))
}

#[cfg(not(windows))]
fn fake_tool_path(root: &Path, name: &str) -> PathBuf {
    root.join(name)
}

fn parse_json_pointer(
    path: &Path,
    line_number: usize,
    assertion_name: &str,
    assertion_index: usize,
    pointer: &str,
) -> Vec<String> {
    if pointer.is_empty() {
        return Vec::new();
    }
    if !pointer.starts_with('/') {
        manifest_error(
            path,
            line_number,
            format!(
                "{assertion_name} {assertion_index} path `{pointer}` is not a JSON Pointer; nonempty pointers must start with `/`"
            ),
        );
    }
    pointer[1..]
        .split('/')
        .map(|token| {
            let mut decoded = String::new();
            let mut chars = token.chars();
            while let Some(ch) = chars.next() {
                if ch != '~' {
                    decoded.push(ch);
                    continue;
                }
                match chars.next() {
                    Some('0') => decoded.push('~'),
                    Some('1') => decoded.push('/'),
                    Some(escape) => manifest_error(
                        path,
                        line_number,
                        format!(
                            "{assertion_name} {assertion_index} path `{pointer}` has invalid JSON Pointer escape `~{escape}`"
                        ),
                    ),
                    None => manifest_error(
                        path,
                        line_number,
                        format!(
                            "{assertion_name} {assertion_index} path `{pointer}` has an incomplete JSON Pointer escape"
                        ),
                    ),
                }
            }
            decoded
        })
        .collect()
}

fn assert_lsp_assertions(context: &CaseRunContext<'_>, stdout: &str, assertions: &[LspAssertion]) {
    assert_lsp_assertions_in_workspace(context, stdout, assertions, Path::new("."));
}

fn assert_lsp_assertions_in_workspace(
    context: &CaseRunContext<'_>,
    stdout: &str,
    assertions: &[LspAssertion],
    project_root: &Path,
) {
    if assertions.is_empty() {
        return;
    }
    let messages = decode_lsp_stdout(stdout).unwrap_or_else(|error| {
        let selectors = assertions
            .iter()
            .map(LspAssertion::selector)
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "{}: decoded LSP stream failed for {selectors}: {error}",
            context.label()
        )
    });
    let mut failures = Vec::new();
    for (index, assertion) in assertions.iter().enumerate() {
        if let Err(error) = evaluate_lsp_assertion_in_workspace(&messages, assertion, project_root)
        {
            failures.push(format!(
                "{}: lsp_assert {index} {} path {:?}: {error}",
                context.label(),
                assertion.selector(),
                assertion.path
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}

fn assert_mcp_assertions(
    context: &CaseRunContext<'_>,
    stdout: &str,
    assertions: &[McpAssertion],
    project_root: &Path,
) {
    if assertions.is_empty() {
        return;
    }
    let messages = decode_mcp_stdout(stdout).unwrap_or_else(|error| {
        let selectors = assertions
            .iter()
            .map(McpAssertion::selector)
            .collect::<Vec<_>>()
            .join(", ");
        panic!(
            "{}: decoded MCP JSONL stream failed for {selectors}: {error}",
            context.label()
        )
    });
    let mut failures = Vec::new();
    for (index, assertion) in assertions.iter().enumerate() {
        if let Err(error) = evaluate_mcp_assertion(&messages, assertion, project_root) {
            failures.push(format!(
                "{}: mcp_assert {index} {} path {:?}: {error}",
                context.label(),
                assertion.selector(),
                assertion.path
            ));
        }
    }
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}

fn decode_mcp_stdout(stdout: &str) -> Result<Vec<JsonValue>, String> {
    let mut messages = Vec::new();
    for (index, line) in stdout.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let message = parse_json(line)
            .map_err(|error| format!("line {} is invalid JSON: {error}", index + 1))?;
        if !matches!(message, JsonValue::Object(_)) {
            return Err(format!("line {} is not a JSON-RPC object", index + 1));
        }
        messages.push(message);
    }
    Ok(messages)
}

fn decode_lsp_stdout(stdout: &str) -> Result<Vec<JsonValue>, String> {
    let bytes = stdout.as_bytes();
    let mut offset = 0usize;
    let mut messages = Vec::new();
    while offset < bytes.len() {
        let missing_delimiter_error = if messages.is_empty() {
            "malformed or partial framing"
        } else {
            "trailing bytes"
        };
        let (message, next_offset) = decode_lsp_frame(bytes, offset, missing_delimiter_error)?;
        messages.push(message);
        offset = next_offset;
    }

    validate_unique_lsp_response_ids(&messages)?;
    Ok(messages)
}

fn decode_lsp_frame(
    bytes: &[u8],
    offset: usize,
    missing_delimiter_error: &str,
) -> Result<(JsonValue, usize), String> {
    let (body_start, content_length) = decode_lsp_header(bytes, offset, missing_delimiter_error)?;
    let body_end = body_start
        .checked_add(content_length)
        .ok_or_else(|| format!("Content-Length overflow at byte offset {offset}"))?;
    if body_end > bytes.len() {
        return Err(format!(
            "partial frame body at byte offset {body_start}: expected {content_length} bytes, found {}",
            bytes.len() - body_start
        ));
    }
    let body = std::str::from_utf8(&bytes[body_start..body_end])
        .map_err(|_| format!("frame body at byte offset {body_start} is not UTF-8"))?;
    let message = parse_json(body).map_err(|error| {
        format!("frame body at byte offset {body_start} is invalid JSON: {error}")
    })?;
    if !matches!(message, JsonValue::Object(_)) {
        return Err(format!(
            "frame body at byte offset {body_start} is not a JSON-RPC object"
        ));
    }
    Ok((message, body_end))
}

fn decode_lsp_header(
    bytes: &[u8],
    offset: usize,
    missing_delimiter_error: &str,
) -> Result<(usize, usize), String> {
    let Some(header_end_relative) = find_bytes(&bytes[offset..], b"\r\n\r\n") else {
        return Err(format!("{missing_delimiter_error} at byte offset {offset}"));
    };
    let header_end = offset + header_end_relative;
    let header = std::str::from_utf8(&bytes[offset..header_end])
        .map_err(|_| format!("malformed frame header at byte offset {offset}"))?;
    let mut content_length = None;
    for line in header.split("\r\n") {
        if let Some(raw) = line.strip_prefix("Content-Length:") {
            if content_length.is_some() {
                return Err(format!(
                    "duplicate Content-Length header at byte offset {offset}"
                ));
            }
            let raw = raw.trim();
            content_length =
                Some(raw.parse::<usize>().map_err(|_| {
                    format!("invalid Content-Length `{raw}` at byte offset {offset}")
                })?);
        }
    }
    let content_length = content_length
        .ok_or_else(|| format!("missing Content-Length header at byte offset {offset}"))?;
    Ok((header_end + 4, content_length))
}

fn validate_unique_lsp_response_ids(messages: &[JsonValue]) -> Result<(), String> {
    let mut response_ids = Vec::<&JsonValue>::new();
    for message in messages {
        if is_lsp_response(message) {
            let id = message
                .object_field("id")
                .expect("response classification requires id");
            if response_ids.contains(&id) {
                return Err(format!(
                    "duplicate response identifier {}",
                    id.to_compact_string()
                ));
            }
            response_ids.push(id);
        }
    }
    Ok(())
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn is_lsp_response(message: &JsonValue) -> bool {
    message.object_field("id").is_some()
        && message.object_field("method").is_none()
        && (message.object_field("result").is_some() || message.object_field("error").is_some())
}

fn evaluate_lsp_assertion(messages: &[JsonValue], assertion: &LspAssertion) -> Result<(), String> {
    evaluate_lsp_assertion_in_workspace(messages, assertion, Path::new("."))
}

fn evaluate_lsp_assertion_in_workspace(
    messages: &[JsonValue],
    assertion: &LspAssertion,
    project_root: &Path,
) -> Result<(), String> {
    let selected = if let Some(id) = &assertion.id {
        messages
            .iter()
            .find(|message| is_lsp_response(message) && message.object_field("id") == Some(id))
            .ok_or_else(|| "selected response was not found".to_string())?
    } else {
        let method = assertion
            .method
            .as_deref()
            .expect("validated method selector");
        messages
            .iter()
            .filter(|message| {
                message.object_field("id").is_none()
                    && message.object_field("method").and_then(JsonValue::as_str) == Some(method)
            })
            .nth(assertion.occurrence.unwrap_or(0))
            .ok_or_else(|| "selected notification was not found".to_string())?
    };

    evaluate_protocol_pointer_result(
        json_pointer(selected, &assertion.pointer_tokens),
        assertion
            .operation
            .as_ref()
            .expect("validated LSP assertion operation"),
        project_root,
    )
}

fn evaluate_protocol_pointer_result(
    result: JsonPointerResult<'_>,
    operation: &RpcAssertionOperation,
    project_root: &Path,
) -> Result<(), String> {
    match result {
        JsonPointerResult::Missing => {
            if matches!(operation, RpcAssertionOperation::Missing(true)) {
                Ok(())
            } else {
                Err("selected JSON path was not found".to_string())
            }
        }
        JsonPointerResult::Invalid(reason) => Err(format!("invalid traversal: {reason}")),
        JsonPointerResult::Found(actual) => match operation {
            RpcAssertionOperation::Equals(expected) => expect_json_value(actual, expected),
            RpcAssertionOperation::EqualsFile(expected) => {
                expect_string_equals_file(actual, expected)
            }
            RpcAssertionOperation::EqualsFileRef(_) => {
                unreachable!("manifest finish resolves protocol equals_file operands")
            }
            RpcAssertionOperation::EqualsJsonFile(expected) => expect_json_value(actual, expected),
            RpcAssertionOperation::EqualsJsonFileRef(_) => {
                unreachable!("manifest finish resolves protocol equals_json_file operands")
            }
            RpcAssertionOperation::Contains(expected) => expect_string_contains(actual, expected),
            RpcAssertionOperation::Length(expected) => expect_array_length(actual, *expected),
            RpcAssertionOperation::Missing(true) => {
                Err("selected JSON path exists but should be missing".to_string())
            }
            RpcAssertionOperation::Missing(false) => {
                unreachable!("validated missing operation")
            }
            RpcAssertionOperation::WorkspaceFileUri(relative) => {
                expect_workspace_file_uri(actual, project_root, relative)
            }
        },
    }
}

fn evaluate_mcp_assertion(
    messages: &[JsonValue],
    assertion: &McpAssertion,
    project_root: &Path,
) -> Result<(), String> {
    let id = assertion.id.as_ref().expect("validated MCP id");
    let selected = select_mcp_response(messages, id)?;
    evaluate_protocol_pointer_result(
        json_pointer(selected, &assertion.pointer_tokens),
        assertion
            .operation
            .as_ref()
            .expect("validated MCP assertion operation"),
        project_root,
    )
}

fn select_mcp_response<'a>(
    messages: &'a [JsonValue],
    id: &JsonValue,
) -> Result<&'a JsonValue, String> {
    let matches = messages
        .iter()
        .filter(|message| {
            message.object_field("id") == Some(id)
                && message.object_field("method").is_none()
                && (message.object_field("result").is_some()
                    || message.object_field("error").is_some())
        })
        .collect::<Vec<_>>();
    let selected = match matches.as_slice() {
        [selected] => *selected,
        [] => return Err("selected response was not found".to_string()),
        _ => {
            return Err(format!(
                "selected response id {} matched {} responses",
                id.to_compact_string(),
                matches.len()
            ));
        }
    };
    Ok(selected)
}

fn expect_json_value(actual: &JsonValue, expected: &JsonValue) -> Result<(), String> {
    if json_values_equal(actual, expected) {
        Ok(())
    } else {
        Err(format!(
            "value mismatch: expected {}, got {}",
            expected.to_compact_string(),
            actual.to_compact_string()
        ))
    }
}

fn expect_string_equals_file(actual: &JsonValue, expected: &str) -> Result<(), String> {
    let actual = actual
        .as_str()
        .ok_or_else(|| "equals_file requires a selected JSON string".to_string())?;
    if actual == expected {
        Ok(())
    } else {
        Err("string does not equal the expected file contents".to_string())
    }
}

fn expect_string_contains(actual: &JsonValue, expected: &str) -> Result<(), String> {
    let actual = actual
        .as_str()
        .ok_or_else(|| "contains requires a selected JSON string".to_string())?;
    if actual.contains(expected) {
        Ok(())
    } else {
        Err(format!("string does not contain {expected:?}"))
    }
}

fn expect_array_length(actual: &JsonValue, expected: usize) -> Result<(), String> {
    let actual = actual
        .as_array()
        .ok_or_else(|| "length requires a selected JSON array".to_string())?;
    if actual.len() == expected {
        Ok(())
    } else {
        Err(format!(
            "array length mismatch: expected {expected}, got {}",
            actual.len()
        ))
    }
}

fn expect_workspace_file_uri(
    actual: &JsonValue,
    project_root: &Path,
    relative: &str,
) -> Result<(), String> {
    let expected = workspace_file_uri(project_root, relative)?;
    let actual = actual
        .as_str()
        .ok_or_else(|| "workspace_file_uri requires a selected JSON string".to_string())?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "workspace URI mismatch: expected {expected}, got {actual}"
        ))
    }
}

fn json_values_equal(left: &JsonValue, right: &JsonValue) -> bool {
    match (left, right) {
        (JsonValue::Null, JsonValue::Null) => true,
        (JsonValue::Bool(left), JsonValue::Bool(right)) => left == right,
        (JsonValue::Number(left), JsonValue::Number(right)) => left == right,
        (JsonValue::Number(left), JsonValue::Decimal(right))
        | (JsonValue::Decimal(right), JsonValue::Number(left)) => left.to_string() == *right,
        (JsonValue::Decimal(left), JsonValue::Decimal(right)) => left == right,
        (JsonValue::String(left), JsonValue::String(right)) => left == right,
        (JsonValue::Array(left), JsonValue::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(left, right)| json_values_equal(left, right))
        }
        (JsonValue::Object(left), JsonValue::Object(right)) => {
            if left.len() != right.len() {
                return false;
            }
            let mut matched = vec![false; right.len()];
            left.iter().all(|(key, left_value)| {
                let Some(index) =
                    right
                        .iter()
                        .enumerate()
                        .position(|(index, (right_key, right_value))| {
                            !matched[index]
                                && right_key == key
                                && json_values_equal(left_value, right_value)
                        })
                else {
                    return false;
                };
                matched[index] = true;
                true
            })
        }
        _ => false,
    }
}

fn validate_workspace_file_uri_operand(path: &Path, line_number: usize, relative: &str) {
    validate_workspace_file_uri_operand_with_context(path, line_number, relative, None);
}

fn validate_workspace_file_uri_operand_with_context(
    path: &Path,
    line_number: usize,
    relative: &str,
    context: Option<&str>,
) {
    let case_dir = path.parent().unwrap_or_else(|| Path::new("."));
    if let Err(error) = validate_workspace_relative_file(case_dir, relative) {
        manifest_error(
            path,
            line_number,
            match context {
                Some(context) => format!("{context}: {error}"),
                None => error,
            },
        );
    }
}

fn workspace_file_uri(project_root: &Path, relative: &str) -> Result<String, String> {
    validate_workspace_relative_file(project_root, relative)?;
    let path = project_root.join(relative);
    if is_link_like_metadata(
        &path
            .symlink_metadata()
            .map_err(|error| format!("workspace file `{relative}` is not available: {error}"))?,
    ) {
        return Err(format!(
            "workspace file `{relative}` must not be a link-like entry"
        ));
    }
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("workspace file `{relative}` is not available: {error}"))?;
    if is_link_like_metadata(&metadata) {
        return Err(format!(
            "workspace file `{relative}` must not be a link-like entry"
        ));
    }
    if !metadata.is_file() {
        return Err(format!(
            "workspace file `{relative}` must be a regular file"
        ));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("workspace file `{relative}` cannot be canonicalized: {error}"))?;
    let canonical_root = project_root
        .canonicalize()
        .map_err(|error| format!("workspace root cannot be canonicalized: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!(
            "workspace file `{relative}` must stay inside the canonical workspace root"
        ));
    }
    Ok(path_to_file_uri(&canonical))
}

fn validate_workspace_relative_file(base: &Path, relative: &str) -> Result<(), String> {
    let path = Path::new(relative);
    if relative.is_empty() || path.is_absolute() || relative.contains('\\') {
        return Err(format!(
            "workspace_file_uri `{relative}` must be a nonempty workspace-relative path"
        ));
    }
    if relative
        .split('/')
        .any(|segment| segment.is_empty() || segment == "." || segment == "..")
    {
        return Err(format!(
            "workspace_file_uri `{relative}` must not contain `.`, `..`, empty, root, or prefix segments"
        ));
    }
    for component in path.components() {
        match component {
            std::path::Component::Normal(segment) if !segment.is_empty() => {}
            _ => {
                return Err(format!(
                    "workspace_file_uri `{relative}` must not contain `.`, `..`, empty, root, or prefix segments"
                ));
            }
        }
    }
    let mut full = base.to_path_buf();
    for component in path.components() {
        full.push(component);
        let metadata = full.symlink_metadata().map_err(|error| {
            format!("workspace_file_uri `{relative}` must name an existing regular file: {error}")
        })?;
        if is_link_like_metadata(&metadata) {
            return Err(format!(
                "workspace_file_uri `{relative}` must not name a link-like entry"
            ));
        }
    }
    if !full
        .metadata()
        .map_err(|error| {
            format!("workspace_file_uri `{relative}` must name an existing regular file: {error}")
        })?
        .is_file()
    {
        return Err(format!(
            "workspace_file_uri `{relative}` must name an existing regular file"
        ));
    }
    Ok(())
}

fn path_to_file_uri(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let path_text = path.to_string_lossy();
    #[cfg(not(unix))]
    let bytes = path_text.as_bytes();
    let mut encoded = String::new();
    for &byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("file://{encoded}")
}

enum JsonPointerResult<'a> {
    Found(&'a JsonValue),
    Missing,
    Invalid(String),
}

fn json_pointer<'a>(value: &'a JsonValue, tokens: &[String]) -> JsonPointerResult<'a> {
    let mut current = value;
    for token in tokens {
        match current {
            JsonValue::Object(entries) => {
                let Some((_, child)) = entries.iter().find(|(key, _)| key == token) else {
                    return JsonPointerResult::Missing;
                };
                current = child;
            }
            JsonValue::Array(values) => {
                if token != "0"
                    && (token.starts_with('0')
                        || token.is_empty()
                        || !token.bytes().all(|byte| byte.is_ascii_digit()))
                {
                    return JsonPointerResult::Invalid(format!(
                        "array token {token:?} is not a canonical non-negative index"
                    ));
                }
                let Ok(index) = token.parse::<usize>() else {
                    return JsonPointerResult::Invalid(format!(
                        "array token {token:?} exceeds the supported index range"
                    ));
                };
                let Some(child) = values.get(index) else {
                    return JsonPointerResult::Missing;
                };
                current = child;
            }
            _ => {
                return JsonPointerResult::Invalid(format!(
                    "token {token:?} cannot traverse {}",
                    current.to_compact_string()
                ));
            }
        }
    }
    JsonPointerResult::Found(current)
}

fn json_pointer_route_mut<'a>(
    value: &'a mut JsonValue,
    route: &[JsonPointerRouteSegment],
) -> Option<&'a mut JsonValue> {
    let mut current = value;
    for segment in route {
        current = match (current, segment) {
            (JsonValue::Array(values), JsonPointerRouteSegment::ArrayIndex(index)) => {
                values.get_mut(*index)?
            }
            (
                JsonValue::Object(entries),
                JsonPointerRouteSegment::ObjectMember { key, occurrence },
            ) => entries
                .iter_mut()
                .filter(|(candidate, _)| candidate == key)
                .nth(*occurrence)
                .map(|(_, child)| child)?,
            _ => return None,
        };
    }
    Some(current)
}

fn assert_json_path(context: &CaseRunContext<'_>, json: &JsonValue, assertion: &JsonAssertion) {
    assert_json_path_in_workspace(context, json, assertion, 0, Path::new("."));
}

fn assert_json_path_in_workspace(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &JsonAssertion,
    index: usize,
    project_root: &Path,
) {
    let operation = assertion
        .operation
        .as_ref()
        .expect("preflight requires one JSON assertion operation");
    if matches!(operation, ValueAssertionOperation::Missing) {
        assert!(
            json_path(json, &assertion.path).is_none(),
            "{}: json_assert {index}: JSON path `{}` exists but should be missing in {:?}",
            context.label(),
            assertion.path,
            json
        );
        return;
    }

    let actual = json_path(json, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: json_assert {index}: JSON path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            json
        )
    });
    let result = expect_value_assertion(actual, operation, project_root);
    result.unwrap_or_else(|error| {
        panic!(
            "{}: json_assert {index}: JSON path `{}` mismatch: {error}",
            context.label(),
            assertion.path
        )
    });
}

fn assert_result_value_path(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &ResultValueAssertion,
) {
    assert_result_value_path_in_workspace(context, json, assertion, 0, Path::new("."));
}

fn assert_result_value_path_in_workspace(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    assertion: &ResultValueAssertion,
    index: usize,
    project_root: &Path,
) {
    let rendered = json_path(json, &assertion.value_path)
        .and_then(JsonValue::as_str)
        .unwrap_or_else(|| {
            panic!(
                "{}: result_value_assert {index}: result value source path `{}` was not found as a string in {:?}",
                context.label(),
                assertion.value_path,
                json
            )
        });
    let parsed = parse_result_value(rendered).unwrap_or_else(|error| {
        panic!(
            "{}: result_value_assert {index}: result value at `{}` could not be parsed: {error}\nvalue: {rendered}",
            context.label(),
            assertion.value_path
        )
    });

    let operation = assertion
        .operation
        .as_ref()
        .expect("preflight requires one result_value assertion operation");
    if matches!(operation, ValueAssertionOperation::Missing) {
        assert!(
            json_path(&parsed, &assertion.path).is_none(),
            "{}: result_value_assert {index}: result value path `{}` exists but should be missing in {:?}",
            context.label(),
            assertion.path,
            parsed
        );
        return;
    }

    let actual = json_path(&parsed, &assertion.path).unwrap_or_else(|| {
        panic!(
            "{}: result_value_assert {index}: result value path `{}` was not found in {:?}",
            context.label(),
            assertion.path,
            parsed
        )
    });
    let result = expect_value_assertion(actual, operation, project_root);
    result.unwrap_or_else(|error| {
        panic!(
            "{}: result_value_assert {index}: result value path `{}` mismatch: {error}",
            context.label(),
            assertion.path
        )
    });
}

fn expect_value_assertion(
    actual: &JsonValue,
    operation: &ValueAssertionOperation,
    project_root: &Path,
) -> Result<(), String> {
    match operation {
        ValueAssertionOperation::Equals(expected)
        | ValueAssertionOperation::EqualsFile(expected)
        | ValueAssertionOperation::EqualsJsonFile(expected) => expect_json_value(actual, expected),
        ValueAssertionOperation::Contains(expected) => expect_string_contains(actual, expected),
        ValueAssertionOperation::Length(expected) => expect_array_length(actual, *expected),
        ValueAssertionOperation::Missing => unreachable!("handled missing operation"),
        ValueAssertionOperation::WorkspaceFileUri(relative) => {
            expect_workspace_file_uri(actual, project_root, relative)
        }
    }
}

fn assert_diagnostic(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    expected: &DiagnosticExpectation,
) {
    let diagnostics = json_path(json, "diagnostics")
        .and_then(JsonValue::as_array)
        .unwrap_or_else(|| panic!("{}: JSON diagnostics array missing", context.label()));

    let mut matches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic_field(diagnostic, "id") == Some(expected.id.as_str()))
        .filter(|diagnostic| {
            expected
                .message
                .as_deref()
                .is_none_or(|message| diagnostic_field(diagnostic, "message") == Some(message))
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.file.as_deref())
                .is_none_or(|file| {
                    json_path(diagnostic, "span.file") == Some(&JsonValue::String(file.to_string()))
                })
        })
        .filter(|diagnostic| {
            expected
                .span
                .as_ref()
                .and_then(|span| span.line)
                .is_none_or(|line| {
                    json_path_equals(diagnostic, "span.start.line", &JsonValue::Number(line))
                })
        });

    let diagnostic = matches.next().unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{}` was not found in {:?}",
            context.label(),
            expected.id,
            diagnostics
        )
    });
    assert!(
        matches.next().is_none(),
        "{}: diagnostic `{}` matched more than one JSON diagnostic",
        context.label(),
        expected.id
    );

    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "severity",
        &expected.severity,
    );
    assert_diagnostic_field(context, diagnostic, &expected.id, "kind", &expected.kind);
    assert_diagnostic_field(
        context,
        diagnostic,
        &expected.id,
        "message",
        &expected.message,
    );
    if let Some(span) = &expected.span {
        if let Some(file) = &span.file {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.file",
                &JsonValue::String(file.clone()),
            );
        }
        if let Some(line) = span.line {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.line",
                &JsonValue::Number(line),
            );
        }
        if let Some(column) = span.column {
            assert_json_equals(
                context,
                diagnostic,
                &expected.id,
                "span.start.column",
                &JsonValue::Number(column),
            );
        }
    }
}

fn assert_diagnostic_field(
    context: &CaseRunContext<'_>,
    diagnostic: &JsonValue,
    id: &str,
    field: &str,
    expected: &Option<String>,
) {
    if let Some(expected) = expected {
        assert_json_equals(
            context,
            diagnostic,
            id,
            field,
            &JsonValue::String(expected.clone()),
        );
    }
}

fn assert_json_equals(
    context: &CaseRunContext<'_>,
    json: &JsonValue,
    id: &str,
    path: &str,
    expected: &JsonValue,
) {
    let actual = json_path(json, path).unwrap_or_else(|| {
        panic!(
            "{}: diagnostic `{id}` JSON path `{path}` missing in {:?}",
            context.label(),
            json
        )
    });
    expect_json_value(actual, expected).unwrap_or_else(|error| {
        panic!(
            "{}: diagnostic `{id}` JSON path `{path}` mismatch: {error}",
            context.label()
        )
    });
}

fn diagnostic_field<'a>(diagnostic: &'a JsonValue, field: &str) -> Option<&'a str> {
    json_path(diagnostic, field).and_then(JsonValue::as_str)
}

fn json_path_equals(json: &JsonValue, path: &str, expected: &JsonValue) -> bool {
    json_path(json, path).is_some_and(|actual| json_values_equal(actual, expected))
}

fn json_path<'a>(mut value: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    for segment in path.split('.') {
        value = if let Ok(index) = segment.parse::<usize>() {
            value.as_array()?.get(index)?
        } else {
            value.object_field(segment)?
        };
    }
    Some(value)
}

fn parse_result_value(rendered_value: &str) -> Result<JsonValue, String> {
    let trimmed = rendered_value.trim();
    if let Some(inner) = constructor_arg(trimmed, "Err") {
        return parse_veln_value(trimmed).or_else(|_| {
            Ok(result_value_object(
                "Err",
                vec![("value", parse_veln_value(inner)?)],
            ))
        });
    }
    Ok(result_value_object(
        "Err",
        vec![("value", parse_veln_value(trimmed)?)],
    ))
}

#[derive(Clone, Copy)]
enum VelnFieldKind {
    List,
    Text,
    Value,
}

type VelnConstructorField = (&'static str, VelnFieldKind);
type VelnConstructorSchema = (&'static str, &'static [VelnConstructorField]);

use VelnFieldKind::{List, Text, Value};

const VELN_CONSTRUCTOR_SCHEMAS: &[VelnConstructorSchema] = &[
    ("Err", &[("value", Value)]),
    (
        "RuntimeDiagnostic",
        &[("id", Text), ("message", Text), ("detail", Value)],
    ),
    (
        "RuntimeByteDiagnostic",
        &[
            ("byte_offset", Value),
            ("field_path", List),
            ("facts", Value),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeValueDiagnostic",
        &[("field_path", List), ("reason", Text)],
    ),
    ("RuntimeHttp2Diagnostic", &[("detail", Value)]),
    ("RuntimeHttp2HpackDiagnostic", &[("detail", Value)]),
    (
        "RuntimeHpackFixtureDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureDynamicIndexDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("requested_dynamic_index", Value),
            ("dynamic_table_entry_count", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureDynamicNameDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("requested_dynamic_index", Value),
            ("dynamic_table_entry_count", Value),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHpackFixtureTableSizeUpdateDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_block_size", Value),
            ("observed_first_byte", Value),
            ("observed_header_table_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("active_state", Text),
            ("expected_fixture", Text),
            ("codec_module", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPartialPrefaceDiagnostic",
        &[
            ("byte_offset", Value),
            ("pending_count", Value),
            ("expected_count", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidPrefaceDiagnostic",
        &[
            ("byte_offset", Value),
            ("expected_byte", Value),
            ("actual_byte", Value),
            ("matched_prefix_count", Value),
            ("expected_count", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolClosedWithPendingDiagnostic",
        &[
            ("byte_offset", Value),
            ("pending_count", Value),
            ("active_continuation", Text),
            ("expected_stream_id", Value),
            ("started_frame_kind", Value),
            ("started_byte_offset", Value),
            ("accumulated_header_block_bytes", Value),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolContinuationExpectedDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("actual_stream_id", Value),
            ("expected_stream_id", Value),
            ("started_frame_kind", Value),
            ("started_byte_offset", Value),
            ("active_continuation", Text),
            ("accumulated_header_block_bytes", Value),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidFrameKindDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("stream_id", Value),
            ("expected_frame_kind", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidStreamIdDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("required_stream_id_domain", Text),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPeerStreamIdNotIncreasingDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("previous_peer_stream_id", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitFrameSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_payload_length", Value),
            ("allowed_max_frame_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitHeaderListSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_list_size", Value),
            ("allowed_header_list_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitSettingsValueDiagnostic",
        &[
            ("byte_offset", Value),
            ("setting_identifier", Value),
            ("setting_name", Text),
            ("observed_value", Value),
            ("accepted_min_value", Value),
            ("accepted_max_value", Value),
            ("peer_limit_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("observed_payload_length", Value),
            ("expected_payload_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("pad_length", Value),
            ("remaining_payload_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitFlowControlWindowDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_payload_length", Value),
            ("allowed_window_credit", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolContentLengthMismatchDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("expected_content_length", Value),
            ("observed_body_length", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic",
        &[
            ("byte_offset", Value),
            ("observed_header_table_size", Value),
            ("allowed_header_table_size", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("attempted_concurrent_stream_count", Value),
            ("allowed_concurrent_stream_count", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("receive_limit_provenance", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("failed_header_fact", Text),
            ("header_name", Text),
            ("decoded_header_names", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic",
        &[
            ("byte_offset", Value),
            ("frame_kind", Value),
            ("stream_id", Value),
            ("failed_header_fact", Text),
            ("header_name", Text),
            ("decoded_header_names", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("observed_window_increment", Value),
            ("accepted_min_window_increment", Value),
            ("accepted_max_window_increment", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolUnexpectedSettingsAckDiagnostic",
        &[
            ("byte_offset", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolInitialPeerSettingsRequiredDiagnostic",
        &[
            ("byte_offset", Value),
            ("actual_frame_kind", Value),
            ("actual_flags", Value),
            ("stream_id", Value),
            ("endpoint_role", Text),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolSettingsNotAllowedForEndpointDiagnostic",
        &[
            ("byte_offset", Value),
            ("setting_identifier", Value),
            ("setting_name", Text),
            ("endpoint_role", Text),
            ("frame_kind", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolPriorityDependencyDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("dependency_stream_id", Value),
            ("active_state", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic",
        &[
            ("byte_offset", Value),
            ("stream_id", Value),
            ("last_stream_id", Value),
            ("shutdown_state", Text),
            ("endpoint_role", Text),
            ("rule_provenance", Text),
            ("preview", Value),
        ],
    ),
    (
        "RuntimeDiagnosticFieldPathSegment",
        &[("kind", Text), ("name", Text)],
    ),
    (
        "RuntimeByteCountFacts",
        &[
            ("expected_count", Value),
            ("available_count", Value),
            ("readiness", Text),
        ],
    ),
    (
        "RuntimeByteRangeFacts",
        &[("requested_count", Value), ("available_count", Value)],
    ),
    (
        "RuntimeByteFixedValueFacts",
        &[("expected_value", Value), ("actual_value", Value)],
    ),
    ("RuntimeByteReasonFacts", &[("reason", Text)]),
];

fn parse_veln_value(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(JsonValue::Array(Vec::new()));
    }
    if text == "NoRuntimeBytePreview" {
        return Ok(result_value_object("NoRuntimeBytePreview", Vec::new()));
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Ok(parse_veln_atom(text));
    };

    if let Some(schema) = VELN_CONSTRUCTOR_SCHEMAS
        .iter()
        .find(|(constructor, _)| *constructor == name)
    {
        return parse_veln_constructor(name, args, schema.1);
    }

    match name {
        "RuntimeBytePreview" => parse_runtime_byte_preview(name, args),
        "ByteChunk" => parse_byte_chunk(name, args),
        "Byte" | "ByteOffset" | "ByteCount" => parse_byte_measure(name, args),
        "Cons" => Ok(JsonValue::Array(parse_veln_list_items(text)?)),
        _ => parse_unknown_veln_constructor(name, args),
    }
}

fn parse_veln_constructor(
    name: &str,
    args: Vec<&str>,
    fields: &[VelnConstructorField],
) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, fields.len())?;
    let fields = fields
        .iter()
        .zip(args)
        .map(|((field, kind), value)| Ok((*field, parse_veln_field(*kind, value)?)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(result_value_object(name, fields))
}

fn parse_veln_field(kind: VelnFieldKind, text: &str) -> Result<JsonValue, String> {
    match kind {
        List => parse_veln_list(text),
        Text => Ok(JsonValue::String(text.trim().to_string())),
        Value => parse_veln_value(text),
    }
}

fn parse_runtime_byte_preview(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 4)?;
    Ok(result_value_object(
        name,
        vec![
            ("encoding", JsonValue::String("hex".to_string())),
            ("data", JsonValue::String(args[0].trim().to_string())),
            ("preview_byte_count", parse_veln_value(args[1])?),
            ("total_byte_count", parse_veln_value(args[2])?),
            ("truncated", parse_veln_value(args[3])?),
        ],
    ))
}

fn parse_byte_chunk(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 1)?;
    Ok(result_value_object(
        name,
        vec![("bytes", parse_veln_bracketed_list(args[0])?)],
    ))
}

fn parse_byte_measure(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    let args = expect_arity(name, args, 1)?;
    Ok(result_value_object(
        name,
        vec![("value", parse_veln_nonnegative_integer(name, args[0])?)],
    ))
}

fn parse_unknown_veln_constructor(name: &str, args: Vec<&str>) -> Result<JsonValue, String> {
    Ok(result_value_object(
        name,
        vec![(
            "fields",
            JsonValue::Array(
                args.into_iter()
                    .map(parse_veln_value)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        )],
    ))
}

fn parse_veln_list(text: &str) -> Result<JsonValue, String> {
    Ok(JsonValue::Array(parse_veln_list_items(text)?))
}

fn parse_veln_list_items(text: &str) -> Result<Vec<JsonValue>, String> {
    let text = text.trim();
    if text == "Nil" {
        return Ok(Vec::new());
    }
    let Some((name, args)) = split_constructor_call(text) else {
        return Err(format!("expected list value, got `{text}`"));
    };
    if name != "Cons" {
        return Err(format!("expected `Cons` or `Nil`, got `{name}`"));
    }
    let args = expect_arity(name, args, 2)?;
    let mut values = vec![parse_veln_value(args[0])?];
    values.extend(parse_veln_list_items(args[1])?);
    Ok(values)
}

fn parse_veln_bracketed_list(text: &str) -> Result<JsonValue, String> {
    let text = text.trim();
    let inner = text
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("expected bracketed list, got `{text}`"))?;
    if inner.trim().is_empty() {
        return Ok(JsonValue::Array(Vec::new()));
    }
    Ok(JsonValue::Array(
        split_top_level_args(inner)
            .into_iter()
            .map(parse_veln_value)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn parse_veln_atom(text: &str) -> JsonValue {
    match text {
        "true" => JsonValue::Bool(true),
        "false" => JsonValue::Bool(false),
        _ => parse_veln_number_atom(text).unwrap_or_else(|| JsonValue::String(text.to_string())),
    }
}

fn parse_veln_number_atom(text: &str) -> Option<JsonValue> {
    let JsonValue::Decimal(raw) = parse_json(text).ok()? else {
        return None;
    };
    if let Ok(value) = raw.parse::<i64>()
        && value.to_string() == raw
    {
        return Some(JsonValue::Number(value));
    }
    Some(JsonValue::Decimal(raw))
}

fn parse_veln_nonnegative_integer(name: &str, text: &str) -> Result<JsonValue, String> {
    let value = text
        .trim()
        .parse::<i64>()
        .map_err(|_| format!("`{name}` expects an integer payload, got `{}`", text.trim()))?;
    Ok(JsonValue::Number(value))
}

fn split_constructor_call(text: &str) -> Option<(&str, Vec<&str>)> {
    let open = text.find('(')?;
    if !text.ends_with(')') {
        return None;
    }
    let name = text[..open].trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let inner = &text[open + 1..text.len() - 1];
    Some((name, split_top_level_args(inner)))
}

fn constructor_arg<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}(");
    text.strip_prefix(&prefix)?.strip_suffix(')')
}

fn split_top_level_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if paren_depth == 0 && bracket_depth == 0 => {
                args.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        args.push(tail);
    }
    args
}

fn expect_arity<'a>(
    name: &str,
    args: Vec<&'a str>,
    expected: usize,
) -> Result<Vec<&'a str>, String> {
    if args.len() == expected {
        Ok(args)
    } else {
        Err(format!(
            "`{name}` expects {expected} argument(s), got {}",
            args.len()
        ))
    }
}

fn result_value_object(constructor: &str, fields: Vec<(&str, JsonValue)>) -> JsonValue {
    let mut entries = vec![(
        "constructor".to_string(),
        JsonValue::String(constructor.to_string()),
    )];
    entries.extend(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value)),
    );
    JsonValue::Object(entries)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(i64),
    Decimal(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn to_compact_string(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Decimal(value) => value.clone(),
            Self::String(value) => format!("\"{}\"", escape_json_string(value)),
            Self::Array(values) => {
                let values = values
                    .iter()
                    .map(JsonValue::to_compact_string)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("[{values}]")
            }
            Self::Object(entries) => {
                let entries = entries
                    .iter()
                    .map(|(key, value)| {
                        format!(
                            "\"{}\":{}",
                            escape_json_string(key),
                            value.to_compact_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("{{{entries}}}")
            }
        }
    }

    fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Decimal(value) if is_json_integer_token(value) => value.parse().ok(),
            _ => None,
        }
    }

    fn object_field(&self, name: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(entries) => entries
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            ch if ch.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

#[derive(Debug)]
struct JsonParseError {
    message: String,
    offset: usize,
    missing_closing_delimiter: bool,
}

impl JsonParseError {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            offset,
            missing_closing_delimiter: false,
        }
    }

    fn missing_closing_delimiter(offset: usize, delimiter: u8) -> Self {
        Self {
            message: format!("expected `{}` at byte {offset}", delimiter as char),
            offset,
            missing_closing_delimiter: true,
        }
    }
}

impl std::fmt::Display for JsonParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

fn parse_json(text: &str) -> Result<JsonValue, JsonParseError> {
    let mut parser = JsonParser { text, offset: 0 };
    let value = parser.parse_value()?;
    parser.skip_ws();
    if parser.offset == text.len() {
        Ok(value)
    } else {
        Err(JsonParseError::new(
            parser.offset,
            format!("unexpected trailing input at byte {}", parser.offset),
        ))
    }
}

struct JsonParser<'a> {
    text: &'a str,
    offset: usize,
}

impl JsonParser<'_> {
    fn parse_value(&mut self) -> Result<JsonValue, JsonParseError> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => {
                self.expect_literal("null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.expect_literal("true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.expect_literal("false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            Some(byte) => Err(JsonParseError::new(
                self.offset,
                format!("unexpected byte `{}` at byte {}", byte as char, self.offset),
            )),
            None => Err(JsonParseError::new(
                self.offset,
                format!("unexpected end of input at byte {}", self.offset),
            )),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume(b'[')?;
        let mut values = Vec::new();
        self.skip_ws();
        if self.consume_if(b']') {
            return Ok(JsonValue::Array(values));
        }
        loop {
            values.push(self.parse_value()?);
            self.skip_ws();
            if self.consume_if(b']') {
                break;
            }
            if self.peek().is_none() {
                return Err(JsonParseError::missing_closing_delimiter(self.offset, b']'));
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonParseError> {
        self.consume(b'{')?;
        let mut entries = Vec::new();
        self.skip_ws();
        if self.consume_if(b'}') {
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.consume(b':')?;
            let value = self.parse_value()?;
            entries.push((key, value));
            self.skip_ws();
            if self.consume_if(b'}') {
                break;
            }
            if self.peek().is_none() {
                return Err(JsonParseError::missing_closing_delimiter(self.offset, b'}'));
            }
            self.consume(b',')?;
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, JsonParseError> {
        self.consume(b'"')?;
        let mut parsed = String::new();
        while let Some(ch) = self.next_char() {
            match ch {
                '"' => return Ok(parsed),
                '\\' => parsed.push(self.parse_escape()?),
                ch if ch.is_control() => {
                    return Err(JsonParseError::new(
                        self.offset,
                        format!("control character in string at byte {}", self.offset),
                    ));
                }
                ch => parsed.push(ch),
            }
        }
        Err(JsonParseError::new(
            self.offset,
            format!("unterminated string at byte {}", self.offset),
        ))
    }

    fn parse_escape(&mut self) -> Result<char, JsonParseError> {
        let Some(ch) = self.next_char() else {
            return Err(JsonParseError::new(
                self.offset,
                format!("unterminated escape at byte {}", self.offset),
            ));
        };
        match ch {
            '"' | '\\' | '/' => Ok(ch),
            'b' => Ok('\u{08}'),
            'f' => Ok('\u{0c}'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'u' => self.parse_unicode_escape(),
            _ => Err(JsonParseError::new(
                self.offset,
                format!("unsupported escape `{ch}` at byte {}", self.offset),
            )),
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<char, JsonParseError> {
        let start = self.offset;
        let end = start + 4;
        let Some(hex) = self.text.get(start..end) else {
            return Err(JsonParseError::new(
                start,
                format!("short unicode escape at byte {start}"),
            ));
        };
        if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(JsonParseError::new(
                start,
                format!("invalid unicode escape `{hex}` at byte {start}"),
            ));
        }
        self.offset = end;
        let codepoint = u16::from_str_radix(hex, 16).expect("hex was validated");
        if (0xd800..=0xdbff).contains(&codepoint) {
            if !self.text[self.offset..].starts_with("\\u") {
                return Err(JsonParseError::new(
                    start,
                    format!("unpaired high surrogate `{hex}` at byte {start}"),
                ));
            }
            self.offset += 2;
            let (low, low_hex, low_start) = self.parse_unicode_unit()?;
            if !(0xdc00..=0xdfff).contains(&low) {
                return Err(JsonParseError::new(
                    low_start,
                    format!("invalid low surrogate `{low_hex}` at byte {low_start}"),
                ));
            }
            let high_value = u32::from(codepoint - 0xd800);
            let low_value = u32::from(low - 0xdc00);
            let scalar = 0x10000 + ((high_value << 10) | low_value);
            return char::from_u32(scalar).ok_or_else(|| {
                JsonParseError::new(start, format!("invalid surrogate pair at byte {start}"))
            });
        }
        if (0xdc00..=0xdfff).contains(&codepoint) {
            return Err(JsonParseError::new(
                start,
                format!("unpaired low surrogate `{hex}` at byte {start}"),
            ));
        }
        char::from_u32(u32::from(codepoint)).ok_or_else(|| {
            JsonParseError::new(
                start,
                format!("invalid unicode codepoint `{hex}` at byte {start}"),
            )
        })
    }

    fn parse_unicode_unit(&mut self) -> Result<(u16, String, usize), JsonParseError> {
        let start = self.offset;
        let end = start + 4;
        let Some(hex) = self.text.get(start..end) else {
            return Err(JsonParseError::new(
                start,
                format!("short unicode escape at byte {start}"),
            ));
        };
        if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(JsonParseError::new(
                start,
                format!("invalid unicode escape `{hex}` at byte {start}"),
            ));
        }
        self.offset = end;
        let codepoint = u16::from_str_radix(hex, 16).expect("hex was validated");
        Ok((codepoint, hex.to_string(), start))
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonParseError> {
        let start = self.offset;
        self.consume_if(b'-');
        match self.peek() {
            Some(b'0') => {
                self.offset += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(JsonParseError::new(
                        self.offset,
                        format!("leading zero in number at byte {}", self.offset),
                    ));
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected digit at byte {}", self.offset),
                ));
            }
        }
        if self.consume_if(b'.') {
            let fraction_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == fraction_start {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected fraction digit at byte {}", self.offset),
                ));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
            if self.offset == exponent_start {
                return Err(JsonParseError::new(
                    self.offset,
                    format!("expected exponent digit at byte {}", self.offset),
                ));
            }
        }
        let raw = &self.text[start..self.offset];
        Ok(JsonValue::Decimal(raw.to_string()))
    }

    fn expect_literal(&mut self, literal: &str) -> Result<(), JsonParseError> {
        if self.text[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(JsonParseError::new(
                self.offset,
                format!("expected `{literal}` at byte {}", self.offset),
            ))
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.offset += 1;
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), JsonParseError> {
        if self.consume_if(expected) {
            Ok(())
        } else {
            Err(JsonParseError::new(
                self.offset,
                format!("expected `{}` at byte {}", expected as char, self.offset),
            ))
        }
    }

    fn consume_if(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.offset).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.text[self.offset..].chars().next()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }
}
