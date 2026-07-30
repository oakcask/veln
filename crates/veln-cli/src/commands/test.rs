use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{ExitCode, Output};
use std::sync::Mutex;

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

use crate::commands::test_scheduler::{SchedulerError, run_ordered_bounded};
use crate::diagnostics::{has_error, print_human_stderr, tool_info};
use crate::java::{JvmRunResult, create_build_dir, prepare_and_run_jvm_capture_with_env};

pub(crate) fn test(
    json: bool,
    jobs: Option<usize>,
    targets: Vec<PathBuf>,
) -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let explicit = !targets.is_empty();
    let target_expansion = expand_test_targets(&root, &targets);
    let selection_plan = selection_plan(&root, &targets, explicit, &target_expansion)?;
    let project = Project::discover(root, &selection_plan.analysis_targets)
        .map_err(|error| error.to_string())?;
    let mut analysis = analyze_project(project, DoctestMode::Include);
    let test_files = selected_test_files(
        &analysis.project,
        &analysis.module,
        selection_plan.selected_roots.as_ref(),
    );
    let mut cases = discover_test_cases(&analysis.module, &test_files);
    attach_doctest_expectations(&mut cases, &analysis.doctest_expectations);
    let mut suite_errors = Vec::new();
    let diagnostics = analysis.semantic_diagnostics();
    let diagnostics_have_errors = has_error(&diagnostics);

    if cases.is_empty() && !diagnostics_have_errors {
        suite_errors.push(SuiteError::discovery(
            "no test declarations were discovered",
        ));
    }

    let mut analysis_slot = Some(analysis);
    process_discovered_test_cases(
        &mut cases,
        diagnostics_have_errors,
        suite_errors.is_empty(),
        |runnable_cases| {
            let analysis = analysis_slot
                .take()
                .expect("analysis state should be available before test execution");
            let reusable_program = analysis.reusable_standard_ir().map(|ir| {
                generate_classfiles_with_test_entries(
                    ir,
                    &runnable_cases
                        .iter()
                        .map(|case| case.name.clone())
                        .collect::<Vec<_>>(),
                )
            });
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
                    Ok(prepare_test_case_job(
                        &analysis,
                        reusable_program.as_ref(),
                        case,
                    ))
                },
                execute_test_case_job,
            )
            .map_err(test_scheduler_error)?;
            analysis_slot = Some(
                analysis_mutex
                    .into_inner()
                    .map_err(|_| "test analysis state was poisoned".to_string())?,
            );
            Ok::<_, String>(cases)
        },
    )?;
    analysis = analysis_slot.expect("analysis state should be available after test execution");

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

fn run_test_case_jobs<P, E, Prepare, Execute>(
    cases: Vec<TestCase>,
    active_jobs: usize,
    prepare: Prepare,
    execute: Execute,
) -> Result<Vec<TestCase>, SchedulerError<E>>
where
    P: Send,
    E: Send,
    Prepare: Fn(TestCase) -> Result<P, E> + Sync,
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

#[cfg(test)]
mod tests {
    use super::process_discovered_test_cases;
    use super::{SchedulerError, resolve_test_jobs, run_test_case_jobs};
    use std::collections::BTreeSet;
    use std::num::NonZeroUsize;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use veln_diagnostics::JsonValue;
    use veln_source::{SourceFile, TextRange};
    use veln_test::{TestCase, TestCaseSource, TestCaseStatus, TestFailure};

    #[test]
    fn resolves_worker_count_from_explicit_automatic_fallback_and_case_count() {
        let cases = [
            (Some(3), 5, Some(8), 3),
            (None, 5, Some(4), 4),
            (None, 5, None, 1),
            (Some(8), 3, Some(8), 3),
            (None, 0, Some(8), 0),
        ];

        for (explicit, runnable_cases, available, expected) in cases {
            let actual = resolve_test_jobs(explicit, runnable_cases, || {
                available.and_then(NonZeroUsize::new)
            });
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn production_case_orchestration_obeys_selected_bound_and_preserves_order() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(BTreeSet::new()), Condvar::new()));

        let cases = named_cases(["alpha", "beta", "gamma"]);
        let records = run_test_case_jobs(cases, 2, Ok::<_, ()>, {
            let active = Arc::clone(&active);
            let max_active = Arc::clone(&max_active);
            let gate = Arc::clone(&gate);
            move |mut case: TestCase| {
                let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now_active, Ordering::SeqCst);

                let (lock, cvar) = &*gate;
                let mut started = lock.lock().expect("started set should lock");
                started.insert(case.name.clone());
                cvar.notify_all();
                while started.len() < 2 {
                    started = cvar.wait(started).expect("started set should lock");
                }
                drop(started);

                case.events
                    .push(JsonValue::string(format!("{} out", case.name)));
                active.fetch_sub(1, Ordering::SeqCst);
                Ok::<_, ()>(case)
            }
        })
        .expect("case orchestration should complete");

        assert_eq!(case_names(&records), ["alpha", "beta", "gamma"]);
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
        assert_eq!(records[0].events, [JsonValue::string("alpha out")]);
        assert_eq!(records[1].events, [JsonValue::string("beta out")]);
        assert_eq!(records[2].events, [JsonValue::string("gamma out")]);
    }

    #[test]
    fn production_case_orchestration_keeps_jobs_serial_when_bound_is_one() {
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));

        let records =
            run_test_case_jobs(named_cases(["alpha", "beta", "gamma"]), 1, Ok::<_, ()>, {
                let active = Arc::clone(&active);
                let max_active = Arc::clone(&max_active);
                move |case: TestCase| {
                    let now_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                    max_active.fetch_max(now_active, Ordering::SeqCst);
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, ()>(case)
                }
            })
            .expect("case orchestration should complete");

        assert_eq!(case_names(&records), ["alpha", "beta", "gamma"]);
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn production_case_orchestration_reports_mixed_results_in_discovered_order() {
        let records = run_test_case_jobs(
            named_cases(["pass", "fail", "blocked", "doctest", "runner"]),
            3,
            Ok::<_, ()>,
            |mut case| {
                match case.name.as_str() {
                    "pass" => {}
                    "fail" => {
                        case.status = TestCaseStatus::Failed;
                        case.reason = Some("runtime_failure".to_string());
                        case.failure = Some(TestFailure::result("bad".to_string(), None));
                    }
                    "blocked" => {
                        case.status = TestCaseStatus::Blocked;
                        case.reason = Some("static_gate".to_string());
                    }
                    "doctest" => {
                        case.kind = "doctest".to_string();
                    }
                    "runner" => {
                        case.status = TestCaseStatus::Error;
                        case.reason = Some("runner_error".to_string());
                        case.failure = Some(TestFailure::runtime("java not found"));
                    }
                    _ => panic!("unexpected case"),
                }
                Ok::<_, ()>(case)
            },
        )
        .expect("case orchestration should complete");

        assert_eq!(
            case_names(&records),
            ["pass", "fail", "blocked", "doctest", "runner"]
        );
        assert_eq!(records[0].status, TestCaseStatus::Passed);
        assert_eq!(records[1].status, TestCaseStatus::Failed);
        assert_eq!(
            records[1]
                .failure
                .as_ref()
                .map(|failure| failure.kind.as_str()),
            Some("result")
        );
        assert_eq!(records[2].status, TestCaseStatus::Blocked);
        assert_eq!(records[3].kind, "doctest");
        assert_eq!(records[3].status, TestCaseStatus::Passed);
        assert_eq!(records[4].status, TestCaseStatus::Error);
        assert_eq!(records[4].reason.as_deref(), Some("runner_error"));
    }

    #[test]
    fn static_gate_blocks_cases_without_starting_runnable_workers() {
        let worker_starts = AtomicUsize::new(0);
        let mut cases = named_cases(["alpha", "beta"]);
        process_discovered_test_cases(&mut cases, true, true, |runnable_cases| {
            worker_starts.fetch_add(runnable_cases.len(), Ordering::SeqCst);
            Ok::<_, ()>(runnable_cases)
        })
        .expect("static gate should not fail");

        assert_eq!(worker_starts.load(Ordering::SeqCst), 0);
        assert!(
            cases
                .iter()
                .all(|case| case.status == TestCaseStatus::Blocked
                    && case.reason.as_deref() == Some("static_gate"))
        );
    }

    #[test]
    fn production_case_orchestration_joins_all_workers_after_error() {
        let completed = Arc::new(AtomicUsize::new(0));
        let result = run_test_case_jobs(named_cases(["alpha", "beta", "gamma", "delta"]), 2, Ok, {
            let completed = Arc::clone(&completed);
            move |case: TestCase| {
                completed.fetch_add(1, Ordering::SeqCst);
                if case.name == "beta" {
                    Err("injected orchestration failure")
                } else {
                    Ok(case)
                }
            }
        });

        match result {
            Err(SchedulerError::Job("injected orchestration failure")) => {}
            _ => panic!("expected injected orchestration failure"),
        }
        assert_eq!(completed.load(Ordering::SeqCst), 4);
    }

    fn named_cases<const N: usize>(names: [&str; N]) -> Vec<TestCase> {
        names
            .into_iter()
            .enumerate()
            .map(|(index, name)| test_case(index + 1, name))
            .collect()
    }

    fn test_case(index: usize, name: &str) -> TestCase {
        let source = SourceFile::new("main_test.veln", format!("test {name}() -> ()\nend\n"));
        let span = source.span(TextRange::new(0, source.len()));
        TestCase {
            id: format!("case-{index}"),
            name: name.to_string(),
            kind: "test".to_string(),
            status: TestCaseStatus::Passed,
            source: TestCaseSource {
                file: "main_test.veln".to_string(),
                node_id: format!("test-{index}"),
                span,
            },
            reason: None,
            failure: None,
            expected_output: None,
            expected_runtime_failure: None,
            events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn case_names<const N: usize>(cases: &[TestCase]) -> [&str; N] {
        cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>()
            .try_into()
            .expect("case count should match expected names")
    }
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

enum TestCaseJob {
    Ready(Box<ReadyTestCaseJob>),
    Completed(Box<TestCase>),
}

struct ReadyTestCaseJob {
    case: TestCase,
    module: SurfaceModule,
    program: JvmProgram,
    java_args: Vec<String>,
}

fn prepare_test_case_job(
    analysis: &ProjectAnalysis,
    reusable_program: Option<&JvmProgram>,
    mut case: TestCase,
) -> TestCaseJob {
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
                return TestCaseJob::Completed(Box::new(case));
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

    let (program, java_args) = if let Some(program) = reusable_program {
        (program.clone(), vec![case.name.clone()])
    } else {
        (generate_classfiles_with_entry(ir, &case.name), Vec::new())
    };

    TestCaseJob::Ready(Box::new(ReadyTestCaseJob {
        case,
        module: module.clone(),
        program,
        java_args,
    }))
}

fn execute_test_case_job(job: TestCaseJob) -> Result<TestCase, String> {
    let (mut case, module, program, java_args) = match job {
        TestCaseJob::Ready(job) => {
            let ReadyTestCaseJob {
                case,
                module,
                program,
                java_args,
            } = *job;
            (case, module, program, java_args)
        }
        TestCaseJob::Completed(case) => return Ok(*case),
    };

    let TestRunArtifacts {
        output,
        event_trace,
        contract_error_trace,
        result_error_trace,
    } = match execute_test_program(&program, &java_args)? {
        TestExecution::Ran(artifacts) => artifacts,
        TestExecution::ToolError(message) => {
            case.status = TestCaseStatus::Error;
            case.reason = Some("runner_error".to_string());
            case.failure = Some(TestFailure::runtime(message));
            return Ok(case);
        }
    };

    collect_test_events(&mut case, &module, &output, &event_trace);
    apply_test_process_result(
        &mut case,
        &output,
        &contract_error_trace,
        &result_error_trace,
    );
    if case.status == TestCaseStatus::Passed {
        compare_expected_output(&mut case);
    }
    Ok(case)
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
