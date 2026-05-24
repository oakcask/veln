use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_ast::SurfaceModule;
use veln_backend_jvm::generate_java_with_entry;
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::{analyze_surface_module, lower_checked_surface_module};
use veln_test::{
    SuiteError, TestCase, TestCaseStatus, TestFailure, TestReport, TestRunStatus, TestSelection,
    discover_test_cases, selected_test_files, stdio_events_from_output,
};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{JavaRunResult, compile_and_run_java_capture, create_build_dir};
use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) fn test(json: bool, targets: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let project = Project::discover(root, &targets).map_err(|error| error.to_string())?;
    let (module, mut diagnostics) = load_surface_module(&project);
    let test_files = selected_test_files(&project, explicit);
    let mut cases = discover_test_cases(&module, &test_files);
    let mut suite_errors = Vec::new();

    if !has_error(&diagnostics) {
        diagnostics.extend(analyze_surface_module(&module));
    }

    if cases.is_empty() && !has_error(&diagnostics) {
        suite_errors.push(SuiteError::discovery(
            "no zero-argument test functions were discovered",
        ));
    }

    if has_error(&diagnostics) {
        for case in &mut cases {
            case.status = TestCaseStatus::Blocked;
            case.reason = Some("static_gate".to_string());
        }
    } else if suite_errors.is_empty() {
        for case in &mut cases {
            run_test_case(&module, case)?;
        }
    }

    let report = TestReport::new(
        TestSelection::new(&project, &test_files, explicit),
        diagnostics,
        suite_errors,
        cases,
    );

    if json {
        println!("{}", report.to_json());
    } else {
        print_test_human(&report)?;
    }

    Ok(if report.status == TestRunStatus::Passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn run_test_case(module: &SurfaceModule, case: &mut TestCase) -> Result<(), String> {
    let reachable_module = reachable_entry_module(module, &case.name);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        case.status = TestCaseStatus::Blocked;
        case.reason = Some("static_gate".to_string());
        case.diagnostics = lowered.diagnostics;
        return Ok(());
    };

    let java = generate_java_with_entry(&ir, &case.name);
    let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
    let result = compile_and_run_java_capture(&build_dir, &java, "veln test");
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }

    let output = match result? {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            case.status = TestCaseStatus::Error;
            case.reason = Some("runner_error".to_string());
            case.failure = Some(TestFailure {
                kind: "runtime".to_string(),
                message,
            });
            return Ok(());
        }
    };

    case.events = stdio_events_from_output(&output, &case.source);
    if output.status.success() {
        case.status = TestCaseStatus::Passed;
    } else {
        case.status = TestCaseStatus::Failed;
        case.failure = Some(TestFailure {
            kind: "runtime".to_string(),
            message: format!("test process exited with status {}", output.status),
        });
    }
    Ok(())
}

fn print_test_human(report: &TestReport) -> Result<(), String> {
    if !report.diagnostics.is_empty() {
        print_human_stderr(&DiagnosticEnvelope::new(
            tool_info(),
            report.diagnostics.clone(),
        ))?;
    }
    for suite_error in &report.suite_errors {
        eprintln!("veln: test {}: {}", suite_error.kind, suite_error.message);
    }
    for case in &report.cases {
        match case.status {
            TestCaseStatus::Passed => println!("ok {}", case.name),
            TestCaseStatus::Failed => println!("not ok {}", case.name),
            TestCaseStatus::Blocked => println!("blocked {}", case.name),
            TestCaseStatus::Error => println!("error {}", case.name),
        }
        for diagnostic in &case.diagnostics {
            print_human_stderr(&DiagnosticEnvelope::new(
                tool_info(),
                vec![diagnostic.clone()],
            ))?;
        }
        if let Some(failure) = &case.failure {
            eprintln!("veln: test `{}` failed: {}", case.name, failure.message);
        }
    }
    Ok(())
}
