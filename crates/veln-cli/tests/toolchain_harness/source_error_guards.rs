use super::*;

pub(super) fn test_discovery_root(
    id: &'static str,
    relative: &'static str,
) -> toolchain_case_inventory::DiscoveryRoot {
    toolchain_case_inventory::DiscoveryRoot { id, relative }
}

#[test]
pub(super) fn example_source_error_guard_requires_explicit_intent() {
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
pub(super) fn example_source_error_guard_accepts_declared_diagnostic_case() {
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
pub(super) fn normal_check_run_and_test_cases_use_command_source_diagnostic_artifacts() {
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
pub(super) fn declared_and_intended_source_error_cases_keep_independent_guard() {
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
pub(super) fn command_source_error_artifacts_are_read_per_run() {
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
pub(super) fn command_source_error_guard_rejects_unexpected_command_diagnostics() {
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
pub(super) fn command_artifact_guard_rejects_unselected_check_source_errors() {
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
pub(super) fn command_artifact_guard_does_not_reuse_between_copied_projects() {
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
pub(super) fn repeated_command_artifact_guard_uses_each_invocation_artifact() {
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
pub(super) fn command_artifact_guard_rejects_unselected_run_source_errors() {
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
pub(super) fn command_artifact_guard_keeps_runtime_failure_distinct_from_source_errors() {
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
