use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};
use std::sync::{Arc, Mutex};
#[cfg(test)]
use std::sync::{MutexGuard, OnceLock};

use veln_analysis::{DoctestMode, ProjectAnalysis, analyze_project};
use veln_ast::{FunctionKind, SurfaceModule};
use veln_backend_jvm::{
    JvmProgram, generate_classfiles_with_entry, generate_classfiles_with_test_entries,
};
use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{Project, discover_source_paths};
use veln_test::{
    SuiteError, TestCase, TestCaseStatus, TestFailure, TestReport, TestRunStatus, TestSelection,
    TestSelectionPlan, apply_runtime_result, attach_doctest_expectations, compare_expected_output,
    contract_failure_from_trace, dependency_aware_selection_plan, discover_test_cases,
    expand_test_targets, result_failure_from_trace, selected_test_files, stdio_call_spans,
    stdio_events_from_output, stdio_events_from_trace,
};

use crate::commands::test_scheduler::{SchedulerError, run_ordered_bounded};
use crate::diagnostics::{
    harness_source_diagnostic_artifact_requested, has_error, print_human_stderr, tool_info,
    write_harness_source_diagnostic_artifact,
};
use crate::java::{
    JvmExecution, JvmExecutionPreparation, JvmRunResult, create_build_dir,
    prepare_and_run_jvm_capture_with_env, prepare_and_run_jvm_capture_with_execution,
    prepare_jvm_execution,
};

pub(crate) fn test(
    start: super::CommandAnalysisStart,
    json: bool,
    jobs: Option<usize>,
    targets: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let targets = start.resolve_inputs(targets);
    let root = start.package_root;
    let suite = prepare_test_suite(root, &targets)?;
    let report = run_test_suite(suite, jobs)?;

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

struct PreparedTestSuite {
    explicit: bool,
    selection_plan: TestSelectionPlan,
    analysis: ProjectAnalysis,
    test_files: BTreeSet<String>,
    cases: Vec<TestCase>,
    diagnostics: Vec<Diagnostic>,
    suite_errors: Vec<SuiteError>,
}

fn prepare_test_suite(root: PathBuf, targets: &[PathBuf]) -> Result<PreparedTestSuite, String> {
    let explicit = !targets.is_empty();
    let target_expansion = expand_test_targets(&root, targets);
    let selection_plan = selection_plan(&root, targets, explicit, &target_expansion)?;
    let analysis_targets = if harness_source_diagnostic_artifact_requested() {
        &[]
    } else {
        selection_plan.analysis_targets.as_slice()
    };
    let project = Project::discover(root, analysis_targets).map_err(|error| error.to_string())?;
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
    write_harness_source_diagnostic_artifact(&analysis.checked_diagnostics())?;
    let diagnostics_have_errors = has_error(&diagnostics);

    if cases.is_empty() && !diagnostics_have_errors {
        suite_errors.push(SuiteError::discovery(
            "no test declarations were discovered",
        ));
    }

    Ok(PreparedTestSuite {
        explicit,
        selection_plan,
        analysis,
        test_files,
        cases,
        diagnostics,
        suite_errors,
    })
}

fn run_test_suite(suite: PreparedTestSuite, jobs: Option<usize>) -> Result<TestReport, String> {
    let PreparedTestSuite {
        explicit,
        selection_plan,
        mut analysis,
        test_files,
        mut cases,
        diagnostics,
        suite_errors,
    } = suite;
    let diagnostics_have_errors = has_error(&diagnostics);

    let mut analysis_slot = Some(analysis);
    process_discovered_test_cases(
        &mut cases,
        diagnostics_have_errors,
        suite_errors.is_empty(),
        |runnable_cases| {
            let analysis = analysis_slot
                .take()
                .expect("analysis state should be available before test execution");
            let (analysis, cases) = execute_runnable_test_cases(analysis, runnable_cases, jobs)?;
            analysis_slot = Some(analysis);
            Ok::<_, String>(cases)
        },
    )?;
    analysis = analysis_slot.expect("analysis state should be available after test execution");

    Ok(TestReport::new(
        TestSelection::new(&analysis.project, &test_files, explicit)
            .apply_metadata(selection_plan.metadata),
        diagnostics,
        suite_errors,
        cases,
    ))
}

fn execute_runnable_test_cases(
    analysis: ProjectAnalysis,
    runnable_cases: Vec<TestCase>,
    jobs: Option<usize>,
) -> Result<(ProjectAnalysis, Vec<TestCase>), String> {
    let reusable_program = analysis.reusable_standard_ir().map(|ir| {
        generate_classfiles_with_test_entries(
            ir,
            &runnable_cases
                .iter()
                .map(|case| case.name.clone())
                .collect::<Vec<_>>(),
        )
    });
    let execution = Arc::new(TestJvmExecution::new());
    let has_ready_job =
        preflight_runnable_test_case_jobs(&analysis, reusable_program.as_ref(), &runnable_cases);
    if has_ready_job {
        execution.get()?;
    }
    let active_jobs = resolve_test_jobs(jobs, runnable_cases.len(), || {
        std::thread::available_parallelism().ok()
    });
    let analysis_mutex = Mutex::new(analysis);
    let cases = run_test_case_jobs(
        runnable_cases,
        active_jobs,
        |case| {
            let analysis = analysis_mutex
                .lock()
                .map_err(|_| "test analysis state was poisoned".to_string())?;
            let mut job = prepare_test_case_job(&analysis, reusable_program.as_ref(), case);
            job.set_execution(Arc::clone(&execution));
            Ok::<_, String>(job)
        },
        execute_test_case_job,
    )
    .map_err(test_scheduler_error)?;
    let analysis = analysis_mutex
        .into_inner()
        .map_err(|_| "test analysis state was poisoned".to_string())?;
    Ok((analysis, cases))
}

fn preflight_runnable_test_case_jobs(
    analysis: &ProjectAnalysis,
    reusable_program: Option<&JvmProgram>,
    runnable_cases: &[TestCase],
) -> bool {
    let mut has_ready_job = false;
    for case in runnable_cases {
        if preflight_test_case_job(analysis, reusable_program, &case.name) {
            has_ready_job = true;
        }
    }
    has_ready_job
}

fn preflight_test_case_job(
    analysis: &ProjectAnalysis,
    reusable_program: Option<&JvmProgram>,
    case_name: &str,
) -> bool {
    let Some(reachable) = analysis
        .reusable_standard_ir()
        .is_none()
        .then(|| analysis.lower_reachable_entry(case_name, FunctionKind::Test))
    else {
        return reusable_program.is_some();
    };

    if !reachable.lowered.diagnostics.is_empty() {
        return false;
    }
    if retained_user_effect_diagnostic(
        &reachable.module,
        reachable.lowered.core.as_ref(),
        case_name,
    )
    .is_some()
    {
        return false;
    }
    let Some(ir) = &reachable.lowered.ir else {
        return false;
    };
    let _program = generate_classfiles_with_entry(ir, case_name);
    true
}

pub(crate) fn resolve_test_jobs(
    explicit: Option<usize>,
    runnable_cases: usize,
    available_parallelism: impl FnOnce() -> Option<std::num::NonZeroUsize>,
) -> usize {
    if runnable_cases == 0 {
        return 0;
    }
    let requested = explicit.unwrap_or_else(|| {
        available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
    });
    requested.min(runnable_cases)
}

fn test_scheduler_error(error: SchedulerError<String>) -> String {
    match error {
        SchedulerError::InvalidBound => {
            "test scheduler concurrency bound must be positive".to_string()
        }
        SchedulerError::Job(message) => message,
        SchedulerError::WorkerPanicked => "test scheduler worker panicked".to_string(),
    }
}

fn process_discovered_test_cases<E, Run>(
    cases: &mut Vec<TestCase>,
    diagnostics_have_errors: bool,
    suite_can_run: bool,
    run: Run,
) -> Result<(), E>
where
    Run: FnOnce(Vec<TestCase>) -> Result<Vec<TestCase>, E>,
{
    if diagnostics_have_errors {
        for case in cases {
            case.status = TestCaseStatus::Blocked;
            case.reason = Some("static_gate".to_string());
        }
    } else if suite_can_run {
        let runnable_cases = std::mem::take(cases);
        *cases = run(runnable_cases)?;
    }
    Ok(())
}

fn run_test_case_jobs<C, P, E, Prepare, Execute>(
    cases: Vec<C>,
    active_jobs: usize,
    prepare: Prepare,
    execute: Execute,
) -> Result<Vec<TestCase>, SchedulerError<E>>
where
    C: Send,
    P: Send,
    E: Send,
    Prepare: Fn(C) -> Result<P, E> + Sync,
    Execute: Fn(P) -> Result<TestCase, E> + Sync,
{
    run_ordered_bounded(cases, active_jobs, |case| execute(prepare(case)?))
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

mod execution;
use execution::*;

#[cfg(test)]
#[path = "test/tests.rs"]
mod tests;
