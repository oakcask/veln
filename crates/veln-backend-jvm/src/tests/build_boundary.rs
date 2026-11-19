use super::*;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Output;

#[cfg(unix)]
fn run_fake_javac_build(
    name: &str,
    script: &str,
    path_entries: Vec<PathBuf>,
    relative_selected: bool,
) -> (PathBuf, Output) {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_dir(name);
    let tools = root.join("tools");
    let target = root.join("target");
    fs::create_dir_all(&tools).expect("fake tool directory should be created");
    let javac = tools.join("javac");
    fs::write(&javac, script).expect("fake javac should be written");
    let mut permissions = fs::metadata(&javac)
        .expect("fake javac metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&javac, permissions).expect("fake javac should be executable");

    let selected_tool_entry = if relative_selected {
        relative_path(Path::new(env!("CARGO_MANIFEST_DIR")), &tools)
    } else {
        tools.clone()
    };
    let mut selected_entries = vec![selected_tool_entry];
    selected_entries.extend(path_entries);
    selected_entries.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("test process should have PATH"),
    ));
    let selected_path = std::env::join_paths(selected_entries).expect("test PATH should be valid");
    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .arg("--package")
        .arg("veln-backend-jvm")
        .env("CARGO_TARGET_DIR", target)
        .env("PATH", selected_path)
        .env("VELN_FAKE_JAVAC_LOG", root.join("javac-invocations"))
        .output()
        .expect("cargo should run the JVM backend build script");
    (root, output)
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
    use std::os::unix::fs::PermissionsExt;

    let root = temp_dir("older-javac-build-boundary");
    let tools = root.join("tools");
    let target = root.join("target");
    let invocation_log = root.join("javac-invocations");
    fs::create_dir_all(&tools).expect("fake tool directory should be created");

    let javac = tools.join("javac");
    fs::write(
        &javac,
        concat!(
            "#!/bin/sh\n",
            "printf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\n",
            "if [ \"$#\" -eq 1 ] && [ \"$1\" = '-version' ]; then\n",
            "  printf '%s\\n' 'javac 24.0.2' >&2\n",
            "  exit 0\n",
            "fi\n",
            "printf '%s\\n' 'unexpected javac compilation invocation' >&2\n",
            "exit 97\n",
        ),
    )
    .expect("fake javac should be written");
    let mut permissions = fs::metadata(&javac)
        .expect("fake javac metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&javac, permissions).expect("fake javac should be executable");

    let mut path_entries = vec![tools];
    path_entries.extend(std::env::split_paths(
        &std::env::var_os("PATH").expect("test process should have PATH"),
    ));
    let selected_path = std::env::join_paths(path_entries).expect("test PATH should be valid");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .arg("--package")
        .arg("veln-backend-jvm")
        .env("CARGO_TARGET_DIR", &target)
        .env("PATH", selected_path)
        .env("VELN_FAKE_JAVAC_LOG", &invocation_log)
        .output()
        .expect("cargo should run the JVM backend build script");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "build unexpectedly succeeded");
    assert!(
        stderr.contains(
            "Java 25 or newer is required to build the JVM runtime; selected javac has major version 24"
        ),
        "build failure should report the required and detected Java majors:\n{stderr}"
    );

    let invocations =
        fs::read_to_string(&invocation_log).expect("fake javac should record its version probe");
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
        let (root, output) = run_fake_javac_build(name, &script, Vec::new(), false);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "build unexpectedly succeeded");
        assert!(
            stderr.contains("could not determine selected javac major version"),
            "unexpected build failure:\n{stderr}"
        );
        let invocations = fs::read_to_string(root.join("javac-invocations"))
            .expect("fake javac should record its probe");
        assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
        let _ = fs::remove_dir_all(root);
    }
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
    let (root, output) = run_fake_javac_build("failed-javac-probe", script, Vec::new(), false);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "build unexpectedly succeeded");
    assert!(
        stderr.contains("selected javac could not report its major version"),
        "unexpected build failure:\n{stderr}"
    );
    let invocations = fs::read_to_string(root.join("javac-invocations"))
        .expect("fake javac should record its probe");
    assert_eq!(invocations.lines().collect::<Vec<_>>(), ["-version"]);
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn backend_build_uses_the_probed_relative_path_javac_for_compilation() {
    use std::os::unix::fs::PermissionsExt;

    let placeholder = temp_dir("relative-javac-path");
    let fallback = placeholder.join("fallback");
    fs::create_dir_all(&fallback).expect("fallback tool directory should be created");
    let fallback_marker = placeholder.join("fallback-started");
    let fallback_javac = fallback.join("javac");
    fs::write(
        &fallback_javac,
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexit 91\n",
            fallback_marker.display()
        ),
    )
    .expect("fallback javac should be written");
    let mut permissions = fs::metadata(&fallback_javac)
        .expect("fallback metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fallback_javac, permissions).expect("fallback javac should be executable");

    let script = concat!(
        "#!/bin/sh\n",
        "printf '%s\\n' \"$*\" >> \"$VELN_FAKE_JAVAC_LOG\"\n",
        "if [ \"$1\" = '-version' ]; then printf '%s\\n' 'javac 25.0.1' >&2; exit 0; fi\n",
        "printf fake-class > VelnRuntime.class\n",
        "exit 0\n",
    );
    let (root, output) =
        run_fake_javac_build("relative-selected-javac", script, vec![fallback], true);
    assert!(
        output.status.success(),
        "retained javac build failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let invocations = fs::read_to_string(root.join("javac-invocations"))
        .expect("selected javac should record both invocations");
    assert_eq!(
        invocations.lines().collect::<Vec<_>>(),
        ["-version", "--release 25 VelnRuntime.java"]
    );
    assert!(!fallback_marker.exists(), "fallback javac must not run");
    let _ = fs::remove_dir_all(root);
    let _ = fs::remove_dir_all(placeholder);
}
