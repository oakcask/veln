use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};

use veln_analysis::{DoctestMode, ProjectAnalysis, analyze_project};
use veln_ast::{FunctionKind, SurfaceModule};
use veln_backend_jvm::{
    JvmProgram, generate_classfiles_with_entry, generate_classfiles_with_test_entries,
};
use veln_diagnostics::DiagnosticEnvelope;
use veln_project::{Project, discover_source_paths};
use veln_test::{
    SuiteError, TestCase, TestCaseStatus, TestFailure, TestReport, TestRunStatus, TestSelection,
    TestSelectionPlan, apply_runtime_result, attach_doctest_expectations, compare_expected_output,
    contract_failure_from_trace, dependency_aware_selection_plan, discover_test_cases,
    expand_test_targets, result_failure_from_trace, selected_test_files, stdio_call_spans,
    stdio_events_from_output, stdio_events_from_trace,
};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{JvmRunResult, create_build_dir, prepare_and_run_jvm_capture_with_env};

pub(crate) fn test(json: bool, targets: Vec<PathBuf>) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let target_expansion = expand_test_targets(&root, &targets);
    let selection_plan = selection_plan(&root, &targets, explicit, &target_expansion)?;
    let project = Project::discover(root, &selection_plan.analysis_targets)
        .map_err(|error| error.to_string())?;
    let analysis = analyze_project(project, DoctestMode::Include);
    let test_files = selected_test_files(
        &analysis.project,
        &analysis.module,
        selection_plan.selected_roots.as_ref(),
    );
    let mut cases = discover_test_cases(&analysis.module, &test_files);
    attach_doctest_expectations(&mut cases, &analysis.doctest_expectations);
    let mut suite_errors = Vec::new();
    let diagnostics = analysis.semantic_diagnostics();

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
        let reusable_program = analysis.reusable_standard_ir().map(|ir| {
            generate_classfiles_with_test_entries(
                ir,
                &cases
                    .iter()
                    .map(|case| case.name.clone())
                    .collect::<Vec<_>>(),
            )
        });
        for case in &mut cases {
            run_test_case(&analysis, reusable_program.as_ref(), case)?;
        }
    }

    let report = TestReport::new(
        TestSelection::new(&analysis.project, &test_files, explicit)
            .apply_metadata(selection_plan.metadata),
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

fn selection_plan(
    root: &Path,
    targets: &[PathBuf],
    explicit: bool,
    target_expansion: &veln_test::TestTargetExpansion,
) -> Result<TestSelectionPlan, String> {
    if !explicit {
        return Ok(TestSelectionPlan::discovered());
    }

    let explicit_roots = discovered_source_set(root, &target_expansion.targets)?;
    if targets
        .iter()
        .any(|target| absolute_path(root, target).is_dir())
    {
        let mut plan = TestSelectionPlan::explicit(
            explicit_roots,
            target_expansion.source_to_test_added_count,
        );
        preserve_standard_package_analysis(root, &mut plan)?;
        return Ok(plan);
    }

    let source_roots = discovered_source_set(root, targets)?
        .into_iter()
        .filter(|path| !path.ends_with("_test.veln"))
        .collect::<std::collections::BTreeSet<_>>();

    if source_roots.is_empty() {
        let mut plan = TestSelectionPlan::explicit(
            explicit_roots,
            target_expansion.source_to_test_added_count,
        );
        preserve_standard_package_analysis(root, &mut plan)?;
        return Ok(plan);
    }

    let graph_project = Project::discover(root, &[]).map_err(|error| error.to_string())?;
    let graph_analysis = analyze_project(graph_project, DoctestMode::Exclude);
    Ok(dependency_aware_selection_plan(
        &graph_analysis.project,
        &graph_analysis.module,
        &explicit_roots,
        &source_roots,
        target_expansion.source_to_test_added_count,
    ))
}

fn preserve_standard_package_analysis(
    root: &Path,
    plan: &mut TestSelectionPlan,
) -> Result<(), String> {
    let manifest = veln_project::read_manifest(root).map_err(|error| error.to_string())?;
    let is_standard = manifest.is_some_and(|manifest| {
        manifest
            .package
            .fields
            .iter()
            .any(|field| field.key == "name" && field.value == veln_stdlib::PACKAGE_NAME)
    });
    if is_standard {
        plan.analysis_targets.clear();
    }
    Ok(())
}

fn discovered_source_set(
    root: &Path,
    inputs: &[PathBuf],
) -> Result<std::collections::BTreeSet<String>, String> {
    discover_source_paths(root, inputs)
        .map_err(|error| error.to_string())
        .map(|paths| {
            paths
                .into_iter()
                .map(|path| source_path(root, &path))
                .collect()
        })
}

fn source_path(root: &Path, path: &Path) -> String {
    let absolute = absolute_path(root, path);
    absolute
        .strip_prefix(root)
        .map_or_else(|_| absolute.clone(), PathBuf::from)
        .to_string_lossy()
        .replace('\\', "/")
}

fn absolute_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn run_test_case(
    analysis: &ProjectAnalysis,
    reusable_program: Option<&JvmProgram>,
    case: &mut TestCase,
) -> Result<(), String> {
    let reachable = analysis
        .reusable_standard_ir()
        .is_none()
        .then(|| analysis.lower_reachable_entry(&case.name, FunctionKind::Test));
    let (module, ir) = match &reachable {
        Some(reachable) => {
            let Some(ir) = &reachable.lowered.ir else {
                case.status = TestCaseStatus::Blocked;
                case.reason = Some("static_gate".to_string());
                case.diagnostics = reachable.lowered.diagnostics.clone();
                return Ok(());
            };
            (&reachable.module, ir)
        }
        None => (
            &analysis.module,
            analysis
                .reusable_standard_ir()
                .expect("fully lowered project IR was checked before test execution"),
        ),
    };

    let generated;
    let (program, java_args) = if let Some(program) = reusable_program {
        (program, vec![case.name.clone()])
    } else {
        generated = generate_classfiles_with_entry(ir, &case.name);
        (&generated, Vec::new())
    };
    let TestRunArtifacts {
        output,
        event_trace,
        contract_error_trace,
        result_error_trace,
    } = match execute_test_program(program, &java_args)? {
        TestExecution::Ran(artifacts) => artifacts,
        TestExecution::ToolError(message) => {
            case.status = TestCaseStatus::Error;
            case.reason = Some("runner_error".to_string());
            case.failure = Some(TestFailure::runtime(message));
            return Ok(());
        }
    };

    collect_test_events(case, module, &output, &event_trace);
    apply_test_process_result(case, &output, &contract_error_trace, &result_error_trace);
    if case.status == TestCaseStatus::Passed {
        compare_expected_output(case);
    }
    Ok(())
}

struct TestRunArtifacts {
    output: Output,
    event_trace: String,
    contract_error_trace: String,
    result_error_trace: String,
}

enum TestExecution {
    Ran(TestRunArtifacts),
    ToolError(String),
}

fn execute_test_program(jvm: &JvmProgram, java_args: &[String]) -> Result<TestExecution, String> {
    let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
    let event_file = build_dir.join("stdio-events.tsv");
    let contract_error_file = build_dir.join("contract-errors.tsv");
    let result_error_file = build_dir.join("result-errors.tsv");
    let event_env = [
        ("VELN_STDIO_EVENTS", event_file.as_os_str()),
        ("VELN_CONTRACT_ERRORS", contract_error_file.as_os_str()),
        ("VELN_RESULT_ERRORS", result_error_file.as_os_str()),
    ];
    let result =
        prepare_and_run_jvm_capture_with_env(&build_dir, jvm, "veln test", &event_env, java_args);
    let event_trace = fs::read_to_string(&event_file).unwrap_or_default();
    let contract_error_trace = fs::read_to_string(&contract_error_file).unwrap_or_default();
    let result_error_trace = fs::read_to_string(&result_error_file).unwrap_or_default();
    let cleanup_result = fs::remove_dir_all(&build_dir);
    if let Err(error) = cleanup_result {
        eprintln!(
            "veln: warning: failed to remove build directory `{}`: {error}",
            build_dir.display()
        );
    }

    match result? {
        JvmRunResult::Ran(output) => Ok(TestExecution::Ran(TestRunArtifacts {
            output,
            event_trace,
            contract_error_trace,
            result_error_trace,
        })),
        JvmRunResult::ToolError(message) => Ok(TestExecution::ToolError(message)),
    }
}

fn collect_test_events(
    case: &mut TestCase,
    module: &SurfaceModule,
    output: &Output,
    event_trace: &str,
) {
    let call_spans = stdio_call_spans(module);
    case.events = if event_trace.is_empty() {
        stdio_events_from_output(output, &case.source)
    } else {
        stdio_events_from_trace(event_trace, &call_spans, &case.source)
    };
}

fn apply_test_process_result(
    case: &mut TestCase,
    output: &Output,
    contract_error_trace: &str,
    result_error_trace: &str,
) {
    if output.status.success() {
        apply_runtime_result(case, None);
    } else {
        let message = format!("test process exited with status {}", output.status);
        let actual_failure = contract_failure_from_trace(contract_error_trace)
            .or_else(|| result_failure_from_trace(result_error_trace))
            .or_else(|| Some(TestFailure::runtime(message)));
        apply_runtime_result(case, actual_failure);
    }
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
