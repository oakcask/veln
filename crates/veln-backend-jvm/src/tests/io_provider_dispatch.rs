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

fn filesystem_host_program(callback: &str) -> JvmProgram {
    let analysis = analyze_project(
        Project {
            root: std::path::PathBuf::new(),
            manifest: None,
            files: vec![
                SourceFile::new(
                    "main.veln",
                    include_str!("../../test-support/filesystem_host.veln").replace(
                        "api::with_system(root, exercise)",
                        &format!("api::with_system(root, {callback})"),
                    ),
                ),
                SourceFile::new(
                    "io_provider_test_support_test.veln",
                    app_support(include_str!(
                        "../../../veln-stdlib/veln/io_provider_test_support_test.veln"
                    )),
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
    generate_classfiles_with_entry(&lowered.ir.expect("host fixture should lower"), "main")
}

#[test]
fn io_provider_system_filesystem_reaches_real_selected_host_resources() {
    let program = filesystem_host_program("exercise");
    let root = temp_dir("io-provider-real-host");
    fs::create_dir_all(&root).expect("host directory");
    fs::write(root.join("data"), b"abcd").expect("native input");
    for name in ["😀", "é", ""] {
        fs::write(root.join(name), b"").expect("exact native Unicode name");
    }
    // Observe the host's spelling independently, including filesystems that
    // normalize names. UTF-8 string order agrees with Unicode scalar order.
    let mut names = fs::read_dir(&root)
        .expect("native entries")
        .map(|entry| {
            entry
                .expect("native entry")
                .file_name()
                .into_string()
                .expect("representable fixture name")
        })
        .collect::<Vec<_>>();
    names.push("new".to_owned());
    names.sort();
    let expected_entries = format!("{};", names.join(";"));
    let Some(output) = run_jvm_program_when_java_is_available(
        "io-provider-host-program",
        &program,
        &[root.to_str().expect("fixture root text"), &expected_entries],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "real filesystem boundary verified\n"
    );
    assert_eq!(
        fs::read(root.join("data")).expect("native input after positional write and truncate"),
        b"aX"
    );
    assert_eq!(
        fs::read(root.join("new")).expect("native created file"),
        b"hello"
    );
    assert!(
        !root.join("destination").exists(),
        "unsupported rename mutated the namespace"
    );
    fs::remove_dir_all(&root).expect("remove host fixture");
}

#[test]
fn io_provider_system_copy_routes_real_file_bytes_to_a_veln_memory_provider() {
    // Mixed host/source integration: a native FileChannel reader and a Veln
    // writer must each retain their provider under the system scope handlers.
    let program = filesystem_host_program("exercise_memory_copy");
    let root = temp_dir("io-provider-host-memory-copy");
    fs::create_dir_all(&root).expect("host directory");
    fs::write(root.join("data"), b"abcd").expect("native input");
    let Some(output) = run_jvm_program_when_java_is_available(
        "io-provider-native-memory-copy",
        &program,
        &[root.to_str().expect("fixture root text")],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(root.join("data")).expect("native source retained"),
        b"abcd"
    );
    fs::remove_dir_all(&root).expect("remove host fixture");
}

#[cfg(unix)]
#[test]
fn io_provider_system_unlink_keeps_the_original_native_file_identity() {
    // Native inode/descriptor contract: unlink and path reuse must not redirect
    // an existing FileChannel. The callback removes both native bindings.
    let program = filesystem_host_program("exercise_removals");
    let root = temp_dir("io-provider-unlink-identity");
    fs::create_dir_all(&root).expect("host directory");
    let Some(output) = run_jvm_program_when_java_is_available(
        "io-provider-native-unlink",
        &program,
        &[root.to_str().expect("fixture root text")],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !root.join("ephemeral").exists(),
        "native unlink/directory removal did not commit"
    );
    fs::remove_dir_all(&root).expect("remove host fixture");
}

#[cfg(unix)]
#[test]
fn io_provider_system_entries_reject_lossy_native_names() {
    use std::os::unix::ffi::OsStringExt;

    // Host integration: this native byte name cannot be constructed as a
    // portable Unicode source path. Enumeration must fail as a whole.
    let program = filesystem_host_program("exercise");
    let root = temp_dir("io-provider-native-name");
    fs::create_dir_all(&root).expect("host directory");
    fs::write(root.join("data"), b"abcd").expect("native input");
    let invalid = std::ffi::OsString::from_vec(vec![b'b', b'a', b'd', 0xff]);
    fs::write(root.join(invalid), b"native").expect("unrepresentable native name");
    let Some(output) = run_jvm_program_when_java_is_available(
        "io-provider-native-enumeration",
        &program,
        &[root.to_str().expect("fixture root text")],
    ) else {
        return;
    };
    assert!(
        !output.status.success(),
        "lossy native enumeration must fail"
    );
    assert!(
        output.stdout.is_empty(),
        "failed enumeration must not produce a partial result"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("UnrepresentablePath") && stderr.contains("entries"),
        "{stderr}"
    );
    fs::remove_dir_all(&root).expect("remove host fixture");
}

#[cfg(unix)]
#[test]
fn io_provider_system_root_is_a_namespace_view_with_real_link_traversal() {
    // Host boundary: a real symlink leaves the selected namespace. The root
    // must not be presented as confinement, and CREATE_NEW preserves links.
    let program = filesystem_host_program("exercise_links");
    let root = temp_dir("io-provider-root-link");
    let selected = root.join("selected");
    fs::create_dir_all(&selected).expect("host directory");
    fs::write(root.join("outside"), b"escape").expect("outside native target");
    std::os::unix::fs::symlink("../outside", selected.join("link")).expect("native escaping link");
    std::os::unix::fs::symlink("missing", selected.join("dangling")).expect("native dangling link");
    let Some(output) = run_jvm_program_when_java_is_available(
        "io-provider-native-link",
        &program,
        &[selected.to_str().expect("fixture root text")],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(root.join("outside")).expect("unchanged outside target"),
        b"escape"
    );
    assert_eq!(
        fs::read_link(selected.join("dangling")).expect("preserved dangling link"),
        std::path::PathBuf::from("missing")
    );
    fs::remove_dir_all(&root).expect("remove host fixture");
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
        (
            "contract",
            "reject(false)\n\tOk(())",
            "contract failure: require `value` in `reject` blame caller",
        ),
        (
            "ordinary-error",
            "Err(\"original body error\")",
            "provider cleanup could not confirm resource closure",
        ),
        ("cancelled", "", "Err(cancelled)"),
        (
            "propagated-error",
            "let _ = Err(\"original body error\")?\n\tOk(())",
            "provider cleanup could not confirm resource closure",
        ),
    ] {
        let mut source =
            include_str!("../../test-support/filesystem_cleanup.veln").replace("__BODY__", body);
        if name == "cancelled" {
            source =
                include_str!("../../test-support/filesystem_cleanup_cancelled.veln").to_string();
        }
        if name == "propagated-error" {
            source = source.replace(
                "handle body(cell) with io_provider::scope()",
                "handle begin\n\t\tlet _ = body(cell)?\n\t\tOk(())\n\tend with io_provider::scope()",
            );
        }
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
        let trace_root = temp_dir(&format!("filesystem-cleanup-trace-{name}"));
        let trace_path = trace_root.join("cleanup-errors.tsv");
        let result_trace_path = trace_root.join("result-errors.tsv");
        let Some(output) = run_jvm_program_with_env_when_java_is_available(
            &format!("filesystem-cleanup-{name}"),
            &program,
            &[
                (
                    "VELN_CLEANUP_ERRORS",
                    trace_path.to_str().expect("trace path"),
                ),
                (
                    "VELN_RESULT_ERRORS",
                    result_trace_path.to_str().expect("result trace path"),
                ),
            ],
            &[],
        ) else {
            let _ = fs::remove_dir_all(&trace_root);
            return;
        };
        let trace = fs::read_to_string(&trace_path).unwrap_or_else(|error| {
            panic!(
                "{name}: missing cleanup diagnostic trace: {error}; stdout={}; stderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
        let _ = fs::remove_dir_all(&trace_root);
        let encoded = |text: &str| {
            text.bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let third_trace = trace
            .find(&encoded("third release"))
            .expect("first related release diagnostic");
        let second_trace = trace
            .find(&encoded("second release"))
            .expect("second related release diagnostic");
        assert!(third_trace < second_trace, "{name}: {trace}");
        assert!(
            !output.status.success(),
            "{name}: cleanup failure must be abrupt"
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "321\n", "{name}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(primary), "{name}: {stderr}");
        if name != "cancelled" {
            let third = stderr
                .find("third release")
                .expect("first release failure retained");
            let second = stderr
                .find("second release")
                .expect("second release failure retained");
            assert!(third < second, "{name}: {stderr}");
        }
        if matches!(name, "ordinary-error" | "propagated-error") {
            assert!(
                stderr.contains("original body error"),
                "{name}: original ordinary error must remain related: {stderr}"
            );
        }
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
