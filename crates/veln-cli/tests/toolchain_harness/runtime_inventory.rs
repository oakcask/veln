use super::*;

pub(super) fn toolchain_case_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

pub(super) fn run_case(case_dir: &Path) {
    run_case_with_after_invocation(case_dir, |_, _| {});
}

pub(super) fn run_case_with_after_invocation(
    case_dir: &Path,
    mut after_invocation: impl FnMut(&CaseRunContext<'_>, &Path),
) {
    run_case_with_guard_and_after_invocation(
        case_dir,
        guard_generated_or_synthetic_case,
        &mut after_invocation,
    );
}

pub(super) fn run_case_with_guard_and_after_invocation(
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
        collect_panic_failure(&mut run_failures, || {
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

pub(super) fn collect_panic_failure(failures: &mut Vec<String>, action: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)) {
        failures.push(panic_message(panic));
    }
}

pub(super) fn guard_generated_or_synthetic_case(case_dir: &Path) {
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

pub(super) fn is_generated_inventory_member(case_dir: &Path) -> bool {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(relative) = case_dir.strip_prefix(manifest_dir) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    generated_toolchain_cases_contains(&relative)
}

pub(super) fn runtime_generated_inventory_barrier() {
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

pub(super) fn generated_toolchain_cases_contains(relative: &str) -> bool {
    TEST_GENERATED_TOOLCHAIN_CASES.with(|cases| {
        cases.borrow().as_ref().map_or_else(
            || GENERATED_TOOLCHAIN_CASES.contains(&relative),
            |generated| generated.iter().any(|case| case == relative),
        )
    })
}

pub(super) fn with_test_generated_toolchain_cases(generated: Vec<String>, test: impl FnOnce()) {
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

pub(super) struct RuntimeInventoryBarrier {
    pub(super) result: OnceLock<Result<(), String>>,
}

impl RuntimeInventoryBarrier {
    pub(super) const fn new() -> Self {
        Self {
            result: OnceLock::new(),
        }
    }

    pub(super) fn check_with(&self, scan: impl FnOnce() -> Result<(), String>) {
        match self.result.get_or_init(scan) {
            Ok(()) => {}
            Err(message) => panic!("{message}"),
        }
    }
}

#[cfg(unix)]
pub(super) fn write_cache_test_java(tool_dir: &Path) {
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
pub(super) fn write_unusable_cache_test_java(tool_dir: &Path) {
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
pub(super) fn write_other_execute_only_cache_test_java(tool_dir: &Path) {
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
pub(super) fn process_runs_as_root() -> bool {
    Command::new("/bin/sh")
        .arg("-c")
        .arg("test \"$(id -u)\" = 0")
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(unix)]
pub(super) fn cache_test_command(
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
