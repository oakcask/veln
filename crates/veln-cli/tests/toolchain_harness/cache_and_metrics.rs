use super::*;

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
pub(super) fn run_uses_isolated_host_cache_without_local_fallback() {
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
pub(super) fn equivalent_working_directories_share_the_default_cache_entry() {
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
pub(super) fn unavailable_unix_host_base_has_no_local_or_temporary_fallback() {
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
pub(super) fn absolute_override_is_complete_and_keeps_lexical_components_usable() {
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
pub(super) fn invalid_overrides_fail_before_test_bodies_without_host_fallback() {
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
pub(super) fn analysis_and_no_test_results_precede_invalid_cache_configuration() {
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
pub(super) fn missing_java_precedes_invalid_cache_configuration() {
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
pub(super) fn unusable_java_precedes_invalid_cache_configuration() {
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
pub(super) fn inaccessible_tmpdir_does_not_make_other_execute_only_java_available() {
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
pub(super) fn non_executing_commands_ignore_invalid_cache_configuration() {
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
pub(super) fn cache_root_file_is_preserved_and_user_code_does_not_start() {
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
pub(super) fn cache_root_creation_failure_preserves_existing_parent_and_user_code_does_not_start() {
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
pub(super) fn metrics_baseline_check_preserves_report_fields() {
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
pub(super) fn metrics_cli_output_is_stable_for_reversed_input_order() {
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
pub(super) fn analysis_commands_select_the_manifest_package_above_the_invocation_directory() {
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
