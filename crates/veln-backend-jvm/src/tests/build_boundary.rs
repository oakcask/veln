use super::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Output;

#[cfg(unix)]
const SUCCESSFUL_JAVAC: &str = concat!(
    "#!/bin/sh\n",
    "printf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\n",
    "if [ \"$1\" = '-version' ]; then printf '%s\\n' 'javac 25.0.1' >&2; exit 0; fi\n",
    "printf fake-class > VelnRuntime.class\n",
    "exit 0\n",
);

#[cfg(unix)]
fn run_fake_javac_build(name: &str, script: &str) -> (PathBuf, Output) {
    let root = temp_dir(name);
    let tools = root.join("tools");
    write_executable(&tools.join("javac"), script);
    let output = check_jvm_backend(&root, [tools]);
    (root, output)
}

#[cfg(unix)]
fn write_executable(path: &Path, script: impl AsRef<[u8]>) {
    fs::create_dir_all(
        path.parent()
            .expect("fake tool should have a parent directory"),
    )
    .expect("fake tool directory should be created");
    fs::write(path, script).expect("fake tool should be written");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("fake tool should be executable");
}

#[cfg(unix)]
fn check_jvm_backend(root: &Path, path_entries: impl IntoIterator<Item = PathBuf>) -> Output {
    let process_path = std::env::var_os("PATH").expect("test process should have PATH");
    let path_entries = path_entries
        .into_iter()
        .chain(std::env::split_paths(&process_path));
    let selected_path = std::env::join_paths(path_entries).expect("test PATH should be valid");
    Command::new(env!("CARGO"))
        .arg("check")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .arg("--package")
        .arg("veln-backend-jvm")
        .env("CARGO_TARGET_DIR", root.join("target"))
        .env("PATH", selected_path)
        .env("VELN_FAKE_JAVAC_LOG", root.join("javac-invocations"))
        .output()
        .expect("cargo should run the JVM backend build script")
}

#[cfg(unix)]
fn read_javac_invocations(root: &Path) -> String {
    fs::read_to_string(root.join("javac-invocations"))
        .expect("fake javac should record its invocations")
}

#[cfg(unix)]
fn relative_path(from: &Path, to: &Path) -> PathBuf {
    let from = from.components().collect::<Vec<_>>();
    let to = to.components().collect::<Vec<_>>();
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut path = PathBuf::new();
    for _ in common..from.len() {
        path.push("..");
    }
    for component in &to[common..] {
        path.push(component.as_os_str());
    }
    path
}

#[cfg(unix)]
#[test]
fn backend_build_rejects_older_selected_javac_before_runtime_compilation() {
    let script = concat!(
        "#!/bin/sh\n",
        "printf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\n",
        "if [ \"$#\" -eq 1 ] && [ \"$1\" = '-version' ]; then\n",
        "  printf '%s\\n' 'javac 24.0.2' >&2\n",
        "  exit 0\n",
        "fi\n",
        "printf '%s\\n' 'unexpected javac compilation invocation' >&2\n",
        "exit 97\n",
    );
    let (root, output) = run_fake_javac_build("older-javac-build-boundary", script);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "build unexpectedly succeeded");
    assert!(
        stderr.contains(
            "Java 25 or newer is required to build the JVM runtime; selected javac has major version 24"
        ),
        "build failure should report the required and detected Java majors:\n{stderr}"
    );

    let invocations = read_javac_invocations(&root);
    assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
    assert!(!invocations.contains("--release 25"));

    let _ = fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn backend_build_rejects_wrong_tool_and_split_stream_version_responses() {
    for (name, response) in [
        (
            "wrong-tool-javac",
            "printf '%s\\n' 'openjdk version \"25\"' >&2",
        ),
        (
            "split-stream-javac",
            "printf '%s' 'javac 25'\nprintf '%s\\n' 'javac 24' >&2",
        ),
    ] {
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\nif [ \"$1\" = '-version' ]; then\n  {response}\n  exit 0\nfi\nexit 97\n"
        );
        let (root, output) = run_fake_javac_build(name, &script);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "build unexpectedly succeeded");
        assert!(
            stderr.contains("could not determine selected javac major version"),
            "unexpected build failure:\n{stderr}"
        );
        let invocations = read_javac_invocations(&root);
        assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(unix)]
#[test]
fn backend_build_rejects_malformed_modern_and_suffixed_javac_banners() {
    for (name, banner) in [
        ("modern-underscore-javac", "javac 25_0"),
        ("suffixed-javac", "javac 25.0.1 trailing-garbage"),
    ] {
        let script = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\nif [ \"$1\" = '-version' ]; then printf '%s\\n' '{banner}' >&2; exit 0; fi\nprintf fake-class > VelnRuntime.class\nexit 0\n"
        );
        let (root, output) = run_fake_javac_build(name, &script);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "build unexpectedly succeeded");
        assert!(
            stderr.contains("could not determine selected javac major version"),
            "unexpected build failure:\n{stderr}"
        );
        let invocations = read_javac_invocations(&root);
        assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
        assert!(!invocations.contains("--release 25"));
        let _ = fs::remove_dir_all(root);
    }
}

#[cfg(unix)]
#[test]
fn backend_build_ignores_javac_exe_and_selects_the_later_unix_javac() {
    let root = temp_dir("unix-javac-name-selection");
    let exe_tools = root.join("exe-tools");
    let unix_tools = root.join("unix-tools");
    let exe_marker = root.join("javac-exe-started");
    write_executable(
        &exe_tools.join("javac.exe"),
        format!(
            "#!/bin/sh\nprintf started > '{}'\nprintf '%s\\n' 'javac 24' >&2\nexit 0\n",
            exe_marker.display()
        ),
    );
    write_executable(&unix_tools.join("javac"), SUCCESSFUL_JAVAC);
    let output = check_jvm_backend(&root, [exe_tools, unix_tools]);

    assert!(
        output.status.success(),
        "Unix javac build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!exe_marker.exists(), "Unix lookup must ignore javac.exe");
    let invocations = read_javac_invocations(&root);
    assert_eq!(
        invocations.lines().collect::<Vec<_>>(),
        ["-version", "--release 25 VelnRuntime.java"]
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn backend_build_rejects_unsuccessful_version_probe_before_compilation() {
    let script = concat!(
        "#!/bin/sh\n",
        "printf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\n",
        "printf '%s\\n' 'javac 25.0.1' >&2\n",
        "exit 9\n",
    );
    let (root, output) = run_fake_javac_build("failed-javac-probe", script);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "build unexpectedly succeeded");
    assert!(
        stderr.contains("selected javac could not report its major version"),
        "unexpected build failure:\n{stderr}"
    );
    let invocations = read_javac_invocations(&root);
    assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn backend_build_uses_the_probed_relative_path_javac_for_compilation() {
    let root = temp_dir("relative-selected-javac");
    let tools = root.join("tools");
    let fallback = root.join("fallback");
    let fallback_marker = root.join("fallback-started");
    write_executable(
        &fallback.join("javac"),
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexit 91\n",
            fallback_marker.display()
        ),
    );
    write_executable(&tools.join("javac"), SUCCESSFUL_JAVAC);
    let selected = relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &tools);
    let output = check_jvm_backend(&root, [selected, fallback]);
    assert!(
        output.status.success(),
        "retained javac build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invocations = read_javac_invocations(&root);
    assert_eq!(
        invocations.lines().collect::<Vec<_>>(),
        ["-version", "--release 25 VelnRuntime.java"]
    );
    assert!(!fallback_marker.exists(), "fallback javac must not run");
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn backend_build_skips_an_unlaunchable_javac_path_candidate() {
    let root = temp_dir("skip-unlaunchable-javac");
    let unusable = root.join("unusable");
    let usable = root.join("usable");
    let interpreter = unusable.join("interpreter-directory");
    fs::create_dir_all(&interpreter).expect("interpreter directory should be created");
    write_executable(
        &unusable.join("javac"),
        format!("#!{}\n", interpreter.display()),
    );
    write_executable(&usable.join("javac"), SUCCESSFUL_JAVAC);
    let output = check_jvm_backend(&root, [unusable, usable]);

    assert!(
        output.status.success(),
        "fallback javac build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invocations = read_javac_invocations(&root);
    assert_eq!(
        invocations.lines().collect::<Vec<_>>(),
        ["-version", "--release 25 VelnRuntime.java"]
    );
    let _ = fs::remove_dir_all(root);
}
