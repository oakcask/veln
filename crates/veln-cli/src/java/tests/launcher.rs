use super::*;

#[test]
fn jvm_class_cache_key_tracks_class_path_contents_and_order() {
    let base = jvm_program(&[
        ("VelnProgram.class", b"class VelnProgram {}"),
        ("VelnRuntime.class", b"class VelnRuntime {}"),
    ]);
    let changed_contents = jvm_program(&[
        ("VelnProgram.class", b"class VelnProgram { int value; }"),
        ("VelnRuntime.class", b"class VelnRuntime {}"),
    ]);
    let changed_path = jvm_program(&[
        ("Entry.class", b"class VelnProgram {}"),
        ("VelnRuntime.class", b"class VelnRuntime {}"),
    ]);
    let changed_order = jvm_program(&[
        ("VelnRuntime.class", b"class VelnRuntime {}"),
        ("VelnProgram.class", b"class VelnProgram {}"),
    ]);

    let base_key = jvm_class_cache_key(&base);

    assert_eq!(
        base_key,
        "e3468afa5195f57975f7020f2137e12175936e3bee539ca7c781f7ff7a4d289e"
    );
    assert_eq!(base_key.len(), 64);
    assert!(base_key.chars().all(|ch| ch.is_ascii_hexdigit()));
    assert_ne!(base_key, jvm_class_cache_key(&changed_contents));
    assert_ne!(base_key, jvm_class_cache_key(&changed_path));
    assert_ne!(base_key, jvm_class_cache_key(&changed_order));
}

#[test]
fn jvm_runner_reports_missing_java_launcher() {
    let root = temp_root("missing-java");
    let result = run_jvm_class_dir(
        root.join("missing-java").as_os_str(),
        &root,
        "veln run",
        &[],
        &[],
    )
    .expect("runner should handle missing launcher");

    match result {
        JvmRunResult::ToolError(message) => {
            assert_eq!(
                message,
                "veln: `java` was not found; install a JDK to use `veln run`"
            );
        }
        JvmRunResult::Ran(_) => panic!("missing launcher should not run"),
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn jvm_runner_accepts_harness_owned_success_launcher() {
    let root = temp_root("fake-java");
    let fake_java = write_fake_java(&root);
    let result = run_jvm_class_dir(
        fake_java.as_os_str(),
        &root,
        "veln test",
        &[],
        &["arg".to_string()],
    )
    .expect("fake launcher should run");

    match result {
        JvmRunResult::Ran(output) => {
            assert!(output.status.success());
            assert_eq!(output.stdout, b"");
            assert_eq!(output.stderr, b"");
        }
        JvmRunResult::ToolError(message) => panic!("unexpected tool error: {message}"),
    }

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[cfg(unix)]
fn current_process_is_root() -> bool {
    ProcessCommand::new("/bin/sh")
        .arg("-c")
        .arg("test \"$(id -u)\" = 0")
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
#[test]
fn java_launcher_discovery_skips_candidate_current_process_cannot_execute() {
    use std::os::unix::fs::PermissionsExt;

    if current_process_is_root() {
        return;
    }
    let root = temp_root("java-discovery-skip-unusable");
    let unusable_dir = root.join("unusable");
    let usable_dir = root.join("usable");
    fs::create_dir_all(&unusable_dir).expect("unusable directory should be created");
    let unusable_java = unusable_dir.join("java");
    fs::write(&unusable_java, "#!/bin/sh\nexit 7\n").expect("unusable java should be written");
    let mut permissions = fs::metadata(&unusable_java)
        .expect("unusable java metadata should be available")
        .permissions();
    permissions.set_mode(0o001);
    fs::set_permissions(&unusable_java, permissions).expect("unusable java mode should be set");
    let usable_java = write_fake_java(&usable_dir);
    let path = env::join_paths([unusable_dir.as_path(), usable_dir.as_path()])
        .expect("test PATH should join");

    let selected = find_java_launcher_in_path(&path).expect("usable launcher should be found");

    assert_eq!(selected, usable_java);
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[cfg(unix)]
#[test]
fn java_launcher_discovery_rejects_only_unusable_candidates() {
    use std::os::unix::fs::PermissionsExt;

    if current_process_is_root() {
        return;
    }
    let root = temp_root("java-discovery-only-unusable");
    let unusable_java = root.join("java");
    fs::write(&unusable_java, "#!/bin/sh\nexit 7\n").expect("unusable java should be written");
    let mut permissions = fs::metadata(&unusable_java)
        .expect("unusable java metadata should be available")
        .permissions();
    permissions.set_mode(0o001);
    fs::set_permissions(&unusable_java, permissions).expect("unusable java mode should be set");
    let path = env::join_paths([root.as_path()]).expect("test PATH should join");

    assert_eq!(find_java_launcher_in_path(&path), None);
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn sha256_matches_known_digest_and_incremental_writes() {
    assert_eq!(
        format!("{:x}", LowerHexBytes(&Sha256::digest(b""))),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        format!("{:x}", LowerHexBytes(&Sha256::digest(b"abc"))),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );

    let mut all_at_once = Sha256::new();
    all_at_once.update(b"left-right");

    let mut incremental = Sha256::new();
    incremental.update(b"left");
    incremental.update(b"-");
    incremental.update(b"right");

    assert_eq!(all_at_once.finalize(), incremental.finalize());
}
