use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_ast::{FunctionKind, SurfaceModule};
use veln_backend_jvm::generate_java_with_entry;
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::Project;
use veln_sema::{analyze_surface_module, lower_checked_surface_module};
use veln_test::{
    SuiteError, TestCase, TestCaseStatus, TestFailure, TestReport, TestRunStatus, TestSelection,
    discover_test_cases, selected_test_files, stdio_call_spans, stdio_events_from_output,
    stdio_events_from_trace,
};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{JavaRunResult, compile_and_run_java_capture_with_env, create_build_dir};
use crate::surface::{load_surface_module, reachable_entry_module};

pub(crate) fn test(json: bool, targets: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let target_expansion = expand_test_targets(&root, &targets);
    let project =
        Project::discover(root, &target_expansion.targets).map_err(|error| error.to_string())?;
    let (module, mut diagnostics) = load_surface_module(&project);
    let test_files = selected_test_files(&project, explicit);
    let mut cases = discover_test_cases(&module, &test_files);
    let mut suite_errors = Vec::new();

    if !has_error(&diagnostics) {
        diagnostics.extend(analyze_surface_module(&module));
    }

    if cases.is_empty() && !has_error(&diagnostics) {
        suite_errors.push(SuiteError::discovery(
            "no test declarations were discovered",
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
        TestSelection::new(&project, &test_files, explicit)
            .source_to_test_convention(target_expansion.source_to_test_added_count),
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

struct TestTargetExpansion {
    targets: Vec<PathBuf>,
    source_to_test_added_count: usize,
}

fn expand_test_targets(root: &Path, targets: &[PathBuf]) -> TestTargetExpansion {
    if targets.is_empty() {
        return TestTargetExpansion {
            targets: Vec::new(),
            source_to_test_added_count: 0,
        };
    }

    let mut original_targets = targets.to_vec();
    original_targets.sort();
    original_targets.dedup();
    let original_count = original_targets.len();
    let mut expanded = targets.to_vec();
    for target in targets {
        if let Some(test_target) = paired_test_target(root, target) {
            expanded.push(test_target);
        }
    }
    expanded.sort();
    expanded.dedup();
    let source_to_test_added_count = expanded.len().saturating_sub(original_count);
    TestTargetExpansion {
        targets: expanded,
        source_to_test_added_count,
    }
}

fn paired_test_target(root: &Path, target: &Path) -> Option<PathBuf> {
    let absolute = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    if absolute.is_dir()
        || absolute
            .extension()
            .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    let file_name = absolute.file_name()?.to_str()?;
    if file_name.ends_with("_test.veln") {
        return None;
    }
    let stem = absolute.file_stem()?.to_str()?;
    let candidate = absolute.with_file_name(format!("{stem}_test.veln"));
    if !candidate.is_file() {
        return None;
    }
    if target.is_absolute() {
        Some(candidate)
    } else {
        candidate.strip_prefix(root).map_or_else(
            |_| Some(candidate.clone()),
            |relative| Some(relative.to_path_buf()),
        )
    }
}

fn run_test_case(module: &SurfaceModule, case: &mut TestCase) -> Result<(), String> {
    let reachable_module = reachable_entry_module(module, &case.name, FunctionKind::Test);
    let lowered = lower_checked_surface_module(&reachable_module);
    let Some(ir) = lowered.ir else {
        case.status = TestCaseStatus::Blocked;
        case.reason = Some("static_gate".to_string());
        case.diagnostics = lowered.diagnostics;
        return Ok(());
    };

    let java = generate_java_with_entry(&ir, &case.name);
    let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
    let event_file = build_dir.join("stdio-events.tsv");
    let event_env = [("VELN_STDIO_EVENTS", event_file.as_os_str())];
    let result = compile_and_run_java_capture_with_env(&build_dir, &java, "veln test", &event_env);
    let event_trace = fs::read_to_string(&event_file).unwrap_or_default();
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

    let call_spans = stdio_call_spans(&reachable_module);
    case.events = if event_trace.is_empty() {
        stdio_events_from_output(&output, &case.source)
    } else {
        stdio_events_from_trace(&event_trace, &call_spans, &case.source)
    };
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
    for note in &report.selection.notes {
        eprintln!("veln: test selection: {note}");
    }
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
