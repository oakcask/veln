use std::fs;
use std::path::PathBuf;
use std::process::{ExitStatus, Output};

use veln_ast::lower_surface_ast;
use veln_source::{SourceFile, TextRange};
use veln_syntax::parse;

use super::*;

fn module(text: &str) -> SurfaceModule {
    let source = SourceFile::new("main_test.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    lower_surface_ast(&parsed.tree)
}

fn test_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("veln-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

#[cfg(unix)]
fn exit_status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;

    ExitStatus::from_raw(code)
}

#[cfg(windows)]
fn exit_status(code: u32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;

    ExitStatus::from_raw(code)
}

mod discovery_and_runtime_metadata;
mod doctest_fences_and_failures;
mod doctest_types_and_inference;
mod output_and_trace;
mod protocol_diagnostics;
mod runtime_expectations_and_status;
