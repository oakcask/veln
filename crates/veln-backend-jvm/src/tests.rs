use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::classfile::{
    TailRecursionEligibility, classify_tail_recursion, contract_call_metadata_work,
    contract_remainder_emission_work, contract_remainder_scan_work, function_max_stack,
    split_contract_binary,
};
use crate::java::{
    java_type_identifier, sanitize_identifier_text, unique_java_identifier,
    veln_string_literal_value,
};
use crate::runtime::{concurrency_method, prelude_method, standard_library_method, stdio_method};
use crate::*;
use veln_analysis::{DoctestMode, analyze_project};
use veln_ast::{FunctionKind, lower_surface_ast_with_module_identity};
use veln_ir::{IrCallTarget, IrExpr, IrExprKind, IrStmtKind, TypedProgram};
use veln_project::Project;
use veln_sema::lower_checked_surface_module;
use veln_source::{SourceFile, TextRange};
use veln_syntax::parse;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

mod basic_backend;
mod collections_and_tail_recursion;
mod concurrency;
mod deferred_cleanup;
mod effect_boundary_policy;
mod effect_injection;
mod harness_constants;
mod java_helpers_and_mapping;
mod result_diagnostic_harness;
mod runtime_integration;
mod schema_refinement_policy;
mod tasks_and_contracts;

use harness_constants::*;

fn lower_to_ir(text: &str) -> TypedProgram {
    let source = SourceFile::new("main.veln", text);
    lower_source_to_ir(&source)
}

fn lower_source_to_ir(source: &SourceFile) -> TypedProgram {
    let parsed = parse(source);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse diagnostics: {:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast_with_module_identity(
        &parsed.tree,
        "main".to_string(),
        source.span(TextRange::at(0)),
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.is_empty(),
        "semantic diagnostics: {:#?}",
        lowered.diagnostics
    );
    lowered.ir.expect("source should lower to typed IR")
}

fn lower_with_standard_modules_to_ir(text: &str) -> TypedProgram {
    lower_project_entry_to_ir(text, "main")
}

fn lower_network_facade_entry_to_ir(text: &str, entry: &str) -> TypedProgram {
    lower_project_entry_to_ir(
        &format!(
            "{}\n{}\n{text}",
            include_str!("../test-support/network.veln"),
            include_str!("../test-support/network_boundary.veln")
        ),
        entry,
    )
}

fn lower_project_entry_to_ir(text: &str, entry: &str) -> TypedProgram {
    let source = SourceFile::new("main.veln", text);
    let analysis = analyze_project(
        Project {
            root: std::path::PathBuf::new(),
            files: vec![source],
            manifest: None,
        },
        DoctestMode::Exclude,
    );
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let lowered = analysis
        .lower_reachable_entry(entry, FunctionKind::Function)
        .lowered;
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    lowered.ir.expect("project entry should lower")
}

fn lower_deferred_cleanup_foundation_to_ir(text: &str) -> TypedProgram {
    lower_to_ir(text)
}

#[test]
fn deferred_cleanup_foundation_lowering_only_keeps_the_fixture_program() {
    let ir = lower_deferred_cleanup_foundation_to_ir(
        "pub fn main() -> ()\n  defer\n    ()\n  end\n  ()\nend\n",
    );

    assert_eq!(ir.functions.len(), 1);
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-backend-jvm-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test directory should be created");
    root
}

fn bind_loopback_listener_when_available() -> Option<TcpListener> {
    match TcpListener::bind("127.0.0.1:0") {
        Ok(listener) => Some(listener),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => None,
        Err(error) => panic!("loopback listener should bind: {error}"),
    }
}

fn run_jvm_program_when_java_is_available(
    name: &str,
    program: &JvmProgram,
    args: &[&str],
) -> Option<std::process::Output> {
    run_jvm_program_with_env_when_java_is_available(name, program, &[], args)
}

fn run_jvm_program_with_env_when_java_is_available(
    name: &str,
    program: &JvmProgram,
    env: &[(&str, &str)],
    args: &[&str],
) -> Option<std::process::Output> {
    if Command::new("java").arg("-version").output().is_err() {
        return None;
    }

    let root = temp_dir(name);
    write_jvm_program(&root, program);

    let mut command = Command::new("java");
    command
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    for arg in args {
        command.arg(arg);
    }
    let child = command.spawn().expect("java should run");
    let output = wait_for_test_child(child, name, std::time::Duration::from_secs(30));
    let _ = fs::remove_dir_all(&root);
    Some(output)
}

fn wait_for_test_child(
    mut child: std::process::Child,
    name: &str,
    timeout: std::time::Duration,
) -> std::process::Output {
    let mut stdout = child
        .stdout
        .take()
        .expect("test child stdout should be piped");
    let mut stderr = child
        .stderr
        .take()
        .expect("test child stderr should be piped");
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .read_to_end(&mut bytes)
            .expect("test child stdout should be readable");
        bytes
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .read_to_end(&mut bytes)
            .expect("test child stderr should be readable");
        bytes
    });
    let started = std::time::Instant::now();
    let (status, timed_out) = loop {
        if let Some(status) = child
            .try_wait()
            .expect("test child status should be readable")
        {
            break (status, false);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            break (
                child.wait().expect("timed out test child should be reaped"),
                true,
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    let stdout = stdout_reader.join().expect("stdout reader should finish");
    let stderr = stderr_reader.join().expect("stderr reader should finish");
    if timed_out {
        panic!(
            "{name}: JVM test child exceeded {timeout:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr),
        );
    }
    std::process::Output {
        status,
        stdout,
        stderr,
    }
}

#[cfg(unix)]
#[test]
fn jvm_test_child_timeout_kills_and_reports_the_case() {
    let mut command = Command::new("sleep");
    command
        .arg("60")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().expect("blocking test child should spawn");
    let failure = std::panic::catch_unwind(|| {
        wait_for_test_child(
            child,
            "bounded-jvm-case",
            std::time::Duration::from_millis(50),
        )
    })
    .expect_err("blocking test child should time out");
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap_or("");
    assert!(message.contains("bounded-jvm-case"), "{message}");
    assert!(message.contains("exceeded"), "{message}");
}

fn write_jvm_program(root: &std::path::Path, program: &JvmProgram) {
    for class in &program.classes {
        fs::write(root.join(&class.path), &class.contents).expect("classfile should be written");
    }
}
