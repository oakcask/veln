use super::*;

pub(super) fn preserve_standard_package_analysis(
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

pub(super) fn discovered_source_set(
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

pub(super) fn absolute_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

pub(super) enum TestCaseJob {
    Ready(Box<ReadyTestCaseJob>),
    Completed(Box<TestCase>),
}

pub(super) struct ReadyTestCaseJob {
    pub(super) case: TestCase,
    module: SurfaceModule,
    program: JvmProgram,
    pub(super) java_args: Vec<String>,
    execution: Option<Arc<TestJvmExecution>>,
}

impl TestCaseJob {
    pub(super) fn set_execution(&mut self, execution: Arc<TestJvmExecution>) {
        if let Self::Ready(job) = self {
            job.execution = Some(execution);
        }
    }
}

#[derive(Clone)]
pub(super) enum TestJvmExecutionResult {
    Ready(Arc<JvmExecution>),
    ToolError(String),
}

pub(super) struct TestJvmExecution {
    result: Mutex<Option<Result<TestJvmExecutionResult, String>>>,
}

impl TestJvmExecution {
    pub(super) fn new() -> Self {
        Self {
            result: Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub(super) fn ready(execution: JvmExecution) -> Self {
        Self {
            result: Mutex::new(Some(Ok(TestJvmExecutionResult::Ready(Arc::new(execution))))),
        }
    }

    pub(super) fn get(&self) -> Result<TestJvmExecutionResult, String> {
        let mut result = self
            .result
            .lock()
            .map_err(|_| "test JVM execution state was poisoned".to_string())?;
        if let Some(result) = result.as_ref() {
            return result.clone();
        }
        let prepared = match prepare_jvm_execution("veln test") {
            Ok(JvmExecutionPreparation::Ready(execution)) => {
                Ok(TestJvmExecutionResult::Ready(Arc::new(execution)))
            }
            Ok(JvmExecutionPreparation::ToolError(message)) => {
                Ok(TestJvmExecutionResult::ToolError(message))
            }
            Err(message) => Err(message),
        };
        *result = Some(prepared.clone());
        prepared
    }
}

pub(super) fn prepare_test_case_job(
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
            if !reachable.lowered.diagnostics.is_empty() {
                case.status = TestCaseStatus::Blocked;
                case.reason = Some("static_gate".to_string());
                case.diagnostics = reachable.lowered.diagnostics.clone();
                return TestCaseJob::Completed(Box::new(case));
            };
            if let Some(diagnostic) = retained_user_effect_diagnostic(
                &reachable.module,
                reachable.lowered.core.as_ref(),
                &case.name,
            ) {
                case.status = TestCaseStatus::Blocked;
                case.reason = Some("static_gate".to_string());
                case.diagnostics = vec![diagnostic];
                return TestCaseJob::Completed(Box::new(case));
            }
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
        execution: None,
    }))
}

pub(super) fn retained_user_effect_diagnostic(
    module: &SurfaceModule,
    core: Option<&veln_core::CheckedProgram>,
    test_name: &str,
) -> Option<Diagnostic> {
    crate::commands::retained_user_effect_diagnostic(
        module,
        core,
        test_name,
        crate::commands::RunnableEntryDiagnostic {
            kind: FunctionKind::Test,
            subject: "test",
            node_kind: "test",
            boundary: "test_entry",
        },
    )
}

pub(super) fn execute_test_case_job(job: TestCaseJob) -> Result<TestCase, String> {
    let (mut case, module, program, java_args, execution) = match job {
        TestCaseJob::Ready(job) => {
            let ReadyTestCaseJob {
                case,
                module,
                program,
                java_args,
                execution,
            } = *job;
            (case, module, program, java_args, execution)
        }
        TestCaseJob::Completed(case) => return Ok(*case),
    };

    let TestRunArtifacts {
        output,
        event_trace,
        contract_error_trace,
        result_error_trace,
    } = match execute_test_program(&case.name, &program, &java_args, execution.as_deref())? {
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

pub(super) struct TestRunArtifacts {
    output: Output,
    event_trace: String,
    contract_error_trace: String,
    result_error_trace: String,
}

pub(super) struct TestRunFiles {
    pub(super) build_dir: PathBuf,
    pub(super) event_file: PathBuf,
    pub(super) contract_error_file: PathBuf,
    pub(super) result_error_file: PathBuf,
}

impl TestRunFiles {
    pub(super) fn create() -> Result<Self, String> {
        let build_dir = create_build_dir("veln-test").map_err(|error| error.to_string())?;
        Ok(Self {
            event_file: build_dir.join("stdio-events.tsv"),
            contract_error_file: build_dir.join("contract-errors.tsv"),
            result_error_file: build_dir.join("result-errors.tsv"),
            build_dir,
        })
    }

    fn read_traces(&self) -> (String, String, String) {
        (
            fs::read_to_string(&self.event_file).unwrap_or_default(),
            fs::read_to_string(&self.contract_error_file).unwrap_or_default(),
            fs::read_to_string(&self.result_error_file).unwrap_or_default(),
        )
    }
}

impl Drop for TestRunFiles {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.build_dir)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "veln: warning: failed to remove build directory `{}`: {error}",
                self.build_dir.display()
            );
        }
    }
}

pub(super) enum TestExecution {
    Ran(TestRunArtifacts),
    ToolError(String),
}

pub(super) fn execute_test_program(
    case_name: &str,
    jvm: &JvmProgram,
    java_args: &[String],
    execution: Option<&TestJvmExecution>,
) -> Result<TestExecution, String> {
    #[cfg(not(test))]
    let _ = case_name;
    let prepared_execution = if let Some(execution) = execution {
        Some(execution.get()?)
    } else {
        None
    };
    if let Some(TestJvmExecutionResult::ToolError(message)) = prepared_execution.as_ref() {
        return Ok(TestExecution::ToolError(message.clone()));
    }
    let files = TestRunFiles::create()?;
    #[cfg(test)]
    if let Some(test_execution) = run_test_program_hook(
        case_name,
        &files.build_dir,
        &files.event_file,
        &files.contract_error_file,
        &files.result_error_file,
        java_args,
    ) {
        return Ok(test_execution);
    }
    let event_env = [
        ("VELN_STDIO_EVENTS", files.event_file.as_os_str()),
        (
            "VELN_CONTRACT_ERRORS",
            files.contract_error_file.as_os_str(),
        ),
        ("VELN_RESULT_ERRORS", files.result_error_file.as_os_str()),
    ];
    let result = if let Some(prepared_execution) = prepared_execution {
        match prepared_execution {
            TestJvmExecutionResult::Ready(execution) => prepare_and_run_jvm_capture_with_execution(
                &execution,
                jvm,
                "veln test",
                &event_env,
                java_args,
            ),
            TestJvmExecutionResult::ToolError(message) => Ok(JvmRunResult::ToolError(message)),
        }
    } else {
        prepare_and_run_jvm_capture_with_env(
            &files.build_dir,
            jvm,
            "veln test",
            &event_env,
            java_args,
        )
    };
    let (event_trace, contract_error_trace, result_error_trace) = files.read_traces();

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

#[cfg(test)]
type TestProgramHook =
    dyn Fn(&str, &Path, &Path, &Path, &Path, &[String]) -> Option<TestExecution> + Send + Sync;

#[cfg(test)]
fn test_program_hook_slot() -> &'static Mutex<Option<Arc<TestProgramHook>>> {
    static HOOK: OnceLock<Mutex<Option<Arc<TestProgramHook>>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn test_program_hook_serial_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
fn run_test_program_hook(
    case_name: &str,
    build_dir: &Path,
    event_file: &Path,
    contract_error_file: &Path,
    result_error_file: &Path,
    java_args: &[String],
) -> Option<TestExecution> {
    let hook = test_program_hook_slot()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    hook.and_then(|hook| {
        hook(
            case_name,
            build_dir,
            event_file,
            contract_error_file,
            result_error_file,
            java_args,
        )
    })
}

#[cfg(test)]
pub(super) struct TestProgramHookGuard {
    _serial: MutexGuard<'static, ()>,
}

#[cfg(test)]
impl TestProgramHookGuard {
    pub(super) fn install(hook: Arc<TestProgramHook>) -> Self {
        let serial = test_program_hook_serial_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut slot = test_program_hook_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(
            slot.is_none(),
            "test program hook should not already be set"
        );
        *slot = Some(hook);
        Self { _serial: serial }
    }
}

#[cfg(test)]
impl Drop for TestProgramHookGuard {
    fn drop(&mut self) {
        *test_program_hook_slot()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
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

pub(super) fn print_test_human(report: &TestReport) -> Result<(), String> {
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
