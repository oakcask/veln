use super::*;

fn app_support(text: &str) -> String {
    let mut qualified = String::new();
    for line in text.lines() {
        if line == "use prelude" {
            continue;
        }
        qualified.push_str(line);
        if matches!(
            line.strip_prefix("use ")
                .and_then(|rest| rest.split_whitespace().next()),
            Some("path" | "fs" | "io" | "io_provider")
        ) {
            qualified.push_str(" from \"std\"");
        }
        qualified.push('\n');
    }
    qualified
}

#[test]
fn io_provider_dispatch_preserves_captured_owner_through_task_freezing() {
    // Reuse the scoped Veln fake as application modules. Only import ownership
    // changes here; no fake filesystem decisions move into Rust or Java.
    let analysis = analyze_project(
        Project {
            root: std::path::PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new(
                    "main.veln",
                    include_str!("../../test-support/filesystem_dispatch.veln"),
                ),
                SourceFile::new(
                    "fs_memory_state_test_support_test.veln",
                    app_support(include_str!(
                        "../../../veln-stdlib/veln/fs_memory_state_test_support_test.veln"
                    )),
                ),
                SourceFile::new(
                    "fs_memory_test_support_test.veln",
                    app_support(include_str!(
                        "../../../veln-stdlib/veln/fs_memory_test_support_test.veln"
                    )),
                ),
            ],
        },
        DoctestMode::Exclude,
    );
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let lowered = analysis
        .lower_reachable_entry("main", FunctionKind::Function)
        .lowered;
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let program =
        generate_classfiles_with_entry(&lowered.ir.expect("fixture should lower"), "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("filesystem-captured-dispatch", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "captured filesystem owner and task token preserved\n"
    );
}

#[test]
fn io_provider_cleanup_attempts_all_resources_and_projects_release_failures() {
    for (name, body, primary) in [
        (
            "success",
            "Ok(())",
            "provider cleanup could not confirm resource closure",
        ),
        (
            "runtime",
            "let invalid_count = 66\n\tlet ignored = 1 << invalid_count\n\tOk(())",
            "invalid shift count 66",
        ),
    ] {
        let source =
            include_str!("../../test-support/filesystem_cleanup.veln").replace("__BODY__", body);
        let analysis = analyze_project(
            Project {
                root: std::path::PathBuf::new(),
                manifest: None,
                files: vec![
                    SourceFile::new("main.veln", source),
                    SourceFile::new(
                        "io_lifecycle_test_support_test.veln",
                        app_support(include_str!(
                            "../../../veln-stdlib/veln/io_lifecycle_test_support_test.veln"
                        )),
                    ),
                ],
            },
            DoctestMode::Exclude,
        );
        assert!(
            analysis.checked_diagnostics().is_empty(),
            "{name}: {:#?}",
            analysis.checked_diagnostics()
        );
        let lowered = analysis
            .lower_reachable_entry("main", FunctionKind::Function)
            .lowered;
        assert!(
            lowered.diagnostics.is_empty(),
            "{name}: {:#?}",
            lowered.diagnostics
        );
        let program =
            generate_classfiles_with_entry(&lowered.ir.expect("fixture should lower"), "main");
        let Some(output) = run_jvm_program_when_java_is_available(
            &format!("filesystem-cleanup-{name}"),
            &program,
            &[],
        ) else {
            return;
        };
        assert!(
            !output.status.success(),
            "{name}: cleanup failure must be abrupt"
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "321\n", "{name}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(primary), "{name}: {stderr}");
        let third = stderr
            .find("third release")
            .expect("first release failure retained");
        let second = stderr
            .find("second release")
            .expect("second release failure retained");
        assert!(third < second, "{name}: {stderr}");
    }
}

#[test]
fn io_provider_import_alias_lowers_to_its_selected_storage_boundary() {
    // Compiler/backend contract: standard storage operations retain their
    // selected package under an alias, including explicit type application.
    // A same-spelled application module remains an ordinary source module.
    let analysis = analyze_project(
        Project {
            root: std::path::PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new(
                    "main.veln",
                    r#"
use io_provider as cells from "std"
use io_provider as local

fn change(value: Int, context: {amount: Int, label: String}) -> {state: Int, result: String}
    {state: value + context.amount, result: context.label}
end

fn snapshot(value: Int, ignored: ()) -> {state: Int, result: Int}
    {state: value, result: value}
end

pub fn main() -> () effects [concurrency, stdio]
    let cell: cells::State<Int> = cells::state<Int>(40)
    let message = cells::update(cell, {amount: 2, label: "aliased"}, change)
    let value: Int = cells::update<Int, (), Int>(cell, (), snapshot)
    stdio::println(message)
    stdio::println(int_to_string(value))
    stdio::println(int_to_string(local::state(7)))
end
"#,
                ),
                SourceFile::new(
                    "io_provider.veln",
                    "pub fn state(value: Int) -> Int\n  value + 1\nend\n",
                ),
            ],
        },
        DoctestMode::Exclude,
    );
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "{:#?}",
        analysis.checked_diagnostics()
    );
    let lowered = analysis
        .lower_reachable_entry("main", FunctionKind::Function)
        .lowered;
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let program =
        generate_classfiles_with_entry(&lowered.ir.expect("fixture should lower"), "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("provider-storage-import-alias", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "aliased\n42\n8\n");
}
