use super::*;

#[cfg(unix)]
fn write_recording_java(
    tool_dir: &Path,
    version_commands: &str,
    probe_log: &Path,
    entry_marker: &Path,
) {
    fs::create_dir_all(tool_dir).expect("tool directory should be created");
    let java = tool_dir.join("java");
    fs::write(
        &java,
        format!(
            "#!/bin/sh\nif [ \"$1\" = '-version' ]; then\n  printf '%s\\n' \"$*\" >> '{}'\n  {version_commands}\n  exit 0\nfi\nprintf started > '{}'\nexit 0\n",
            probe_log.display(),
            entry_marker.display(),
        ),
    )
    .expect("fake java should be written");
    let mut permissions = fs::metadata(&java)
        .expect("fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(java, permissions).expect("fake java should be executable");
}

#[cfg(unix)]
fn write_unlaunchable_java(tool_dir: &Path) {
    fs::create_dir_all(tool_dir).expect("tool directory should be created");
    let interpreter = tool_dir.join("interpreter-directory");
    fs::create_dir(&interpreter).expect("interpreter directory should be created");
    let java = tool_dir.join("java");
    fs::write(&java, format!("#!{}\n", interpreter.display()))
        .expect("unlaunchable Java should be written");
    let mut permissions = fs::metadata(&java)
        .expect("unlaunchable Java metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(java, permissions).expect("unlaunchable Java should be executable");
}

fn json_string_field<'a>(value: &'a JsonValue, name: &str) -> &'a str {
    value
        .object_field(name)
        .and_then(JsonValue::as_str)
        .unwrap_or_else(|| panic!("`{name}` should be a JSON string"))
}

#[cfg(unix)]
fn java_boundary_project(name: &str) -> TestProject {
    let project = TestProject::new(name.to_string(), &ToolSetup::default());
    fs::write(
        project.root.join("main.veln"),
        "fn main() -> ()\n  ()\nend\n",
    )
    .expect("run source should be written");
    fs::write(
        project.root.join("main_test.veln"),
        "test baseline() -> ()\n  ()\nend\n",
    )
    .expect("test source should be written");
    project
}

#[cfg(unix)]
#[test]
fn public_run_and_test_reject_invalid_runtime_version_responses_without_launch() {
    let cases = [
        (
            "split-stream",
            "printf '%s' 'openjdk version \"25\"'; printf '%s\\n' 'openjdk version \"24\"' >&2",
            "could not determine the selected `java` major version",
        ),
        (
            "wrong-tool",
            "printf '%s\\n' 'javac 25.0.1' >&2",
            "could not determine the selected `java` major version",
        ),
        (
            "malformed-component",
            "printf '%s\\n' 'openjdk version \"25.not-a-version\"' >&2",
            "could not determine the selected `java` major version",
        ),
        (
            "modern-underscore",
            "printf '%s\\n' 'openjdk version \"25_0\"' >&2",
            "could not determine the selected `java` major version",
        ),
        (
            "older-major",
            "printf '%s\\n' 'openjdk version \"24.0.2\"' >&2",
            "Java 25 or newer",
        ),
    ];
    for (case_name, response, expected) in cases {
        for (command_name, args) in [
            ("run", &["run", "--json", "main", "main.veln"][..]),
            ("test", &["test", "--json", "main_test.veln"][..]),
        ] {
            let project = java_boundary_project(&format!("{case_name}-{command_name}"));
            let tools = project.root.join("tools");
            let probe_log = project.root.join("version-probes");
            let entry_marker = project.root.join("entry-started");
            let cache_root = project.root.join("cache");
            write_recording_java(&tools, response, &probe_log, &entry_marker);

            let output = cache_test_command(
                &project.root,
                args,
                &tools,
                &[("VELN_CACHE_DIR", &cache_root)],
            );

            assert_eq!(output.status.code(), Some(1), "{case_name} {command_name}");
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            assert!(
                combined.contains(expected),
                "{case_name} {command_name} did not report `{expected}`:\n{combined}"
            );
            if case_name == "modern-underscore" {
                assert!(
                    output.stderr.is_empty(),
                    "owned JSON error must not use stderr"
                );
                let report = parse_json(
                    std::str::from_utf8(&output.stdout).expect("owned error should be UTF-8 JSON"),
                )
                .expect("owned error should parse as JSON");
                if command_name == "run" {
                    assert_eq!(json_string_field(&report, "status"), "error");
                    let error = report
                        .object_field("error")
                        .expect("run error should exist");
                    assert_eq!(json_string_field(error, "kind"), "runner");
                    assert_eq!(
                        json_string_field(
                            error
                                .object_field("details")
                                .expect("run details should exist"),
                            "phase",
                        ),
                        "tool"
                    );
                } else {
                    let cases = report
                        .object_field("cases")
                        .and_then(JsonValue::as_array)
                        .expect("test cases should exist");
                    assert_eq!(cases.len(), 1);
                    assert_eq!(json_string_field(&cases[0], "status"), "error");
                    assert_eq!(json_string_field(&cases[0], "reason"), "runner_error");
                    assert_eq!(
                        json_string_field(
                            cases[0]
                                .object_field("failure")
                                .expect("test failure should exist"),
                            "kind",
                        ),
                        "runtime"
                    );
                }
            }
            if case_name == "older-major" {
                assert!(combined.contains("major version 24"));
            }
            assert_eq!(
                fs::read_to_string(&probe_log).expect("version probe should be recorded"),
                "-version\n"
            );
            assert!(
                !cache_root.exists(),
                "rejection must precede cache creation"
            );
            assert!(!entry_marker.exists(), "VelnEntry must not launch");
        }
    }
}

#[cfg(unix)]
#[test]
fn public_run_and_test_report_an_unlaunchable_version_probe_as_an_owned_error() {
    for (command_name, args) in [
        ("run", &["run", "--json", "main", "main.veln"][..]),
        ("test", &["test", "--json", "main_test.veln"][..]),
    ] {
        let project = java_boundary_project(&format!("unlaunchable-probe-{command_name}"));
        let tools = project.root.join("tools");
        let cache_root = project.root.join("cache");
        write_unlaunchable_java(&tools);

        let output = cache_test_command(
            &project.root,
            args,
            &tools,
            &[("VELN_CACHE_DIR", &cache_root)],
        );

        assert_eq!(output.status.code(), Some(1), "{command_name}");
        assert!(
            output.stderr.is_empty(),
            "owned JSON error must not use stderr"
        );
        let report = parse_json(
            std::str::from_utf8(&output.stdout).expect("owned error should be UTF-8 JSON"),
        )
        .expect("owned error should parse as JSON");
        if command_name == "run" {
            assert_eq!(json_string_field(&report, "status"), "error");
            let error = report
                .object_field("error")
                .expect("run error should exist");
            assert_eq!(json_string_field(error, "kind"), "runner");
            assert_eq!(
                json_string_field(
                    error
                        .object_field("details")
                        .expect("run details should exist"),
                    "phase",
                ),
                "tool"
            );
            assert!(
                json_string_field(error, "message")
                    .contains("selected `java` could not report its major version")
            );
        } else {
            let cases = report
                .object_field("cases")
                .and_then(JsonValue::as_array)
                .expect("test cases should exist");
            assert_eq!(cases.len(), 1);
            assert_eq!(json_string_field(&cases[0], "status"), "error");
            assert_eq!(json_string_field(&cases[0], "reason"), "runner_error");
            let failure = cases[0]
                .object_field("failure")
                .expect("test failure should exist");
            assert_eq!(json_string_field(failure, "kind"), "runtime");
            assert!(
                json_string_field(failure, "message")
                    .contains("selected `java` could not report its major version")
            );
        }
        assert!(
            !cache_root.exists(),
            "probe rejection must precede cache creation"
        );
    }
}

#[cfg(unix)]
#[test]
fn run_cache_resolution_precedes_the_java_minimum_major_check() {
    let project = java_boundary_project("run-cache-version-precedence");
    let tools = project.root.join("tools");
    let probe_log = project.root.join("version-probes");
    let entry_marker = project.root.join("entry-started");
    write_recording_java(
        &tools,
        "printf '%s\\n' 'openjdk version \"24.0.2\"' >&2",
        &probe_log,
        &entry_marker,
    );

    let relative = Path::new("relative-cache");
    let invalid = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tools,
        &[("VELN_CACHE_DIR", relative)],
    );
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("invalid VELN_CACHE_DIR"));
    assert!(
        !probe_log.exists(),
        "invalid cache root must precede probing"
    );
    assert!(!entry_marker.exists());

    let cache_root = project.root.join("valid-new-cache");
    let older = cache_test_command(
        &project.root,
        &["run", "main", "main.veln"],
        &tools,
        &[("VELN_CACHE_DIR", &cache_root)],
    );
    assert_eq!(older.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&older.stderr);
    assert!(stderr.contains("Java 25 or newer"));
    assert!(stderr.contains("major version 24"));
    assert_eq!(
        fs::read_to_string(&probe_log).expect("valid root should permit one probe"),
        "-version\n"
    );
    assert!(
        !cache_root.exists(),
        "older Java must precede cache creation"
    );
    assert!(!entry_marker.exists(), "VelnEntry must not launch");
}
