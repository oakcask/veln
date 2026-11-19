use super::*;

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
