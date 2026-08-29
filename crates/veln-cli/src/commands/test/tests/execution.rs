use super::*;

#[test]
fn test_program_hook_tool_error_cleans_case_artifacts() {
    let observed_build_dir = Arc::new(Mutex::new(None));
    let _hook = TestProgramHookGuard::install({
        let observed_build_dir = Arc::clone(&observed_build_dir);
        Arc::new(
            move |_case_name,
                  build_dir,
                  _event_file,
                  _contract_error_file,
                  _result_error_file,
                  _java_args| {
                *observed_build_dir
                    .lock()
                    .expect("observed build directory should lock") = Some(build_dir.to_path_buf());
                Some(TestExecution::ToolError(
                    "injected runner tool error".to_string(),
                ))
            },
        )
    });

    let execution = execute_test_program(
        "runner",
        &JvmProgram {
            classes: Vec::new(),
        },
        &[],
        None,
    )
    .expect("hooked execution should complete");

    assert!(matches!(execution, TestExecution::ToolError(_)));
    let build_dir = observed_build_dir
        .lock()
        .expect("observed build directory should lock")
        .clone()
        .expect("hook should observe the build directory");
    assert!(
        !build_dir.exists(),
        "hooked tool errors should clean per-case artifacts"
    );
}

#[test]
fn test_run_files_isolate_case_artifacts() {
    let first = TestRunFiles::create().expect("first case artifacts should be created");
    let second = TestRunFiles::create().expect("second case artifacts should be created");
    let build_dirs = [first.build_dir.clone(), second.build_dir.clone()];

    assert_ne!(first.build_dir, second.build_dir);
    assert_ne!(first.event_file, second.event_file);
    assert_ne!(first.contract_error_file, second.contract_error_file);
    assert_ne!(first.result_error_file, second.result_error_file);

    drop(first);
    drop(second);
    assert!(build_dirs.iter().all(|path| !path.exists()));
}

#[test]
fn production_jvm_path_overlaps_and_keeps_case_artifacts_isolated() {
    if !jdk_is_available() {
        return;
    }

    let mut scenario = ProductionJvmScenario::new();
    let _hook = scenario.install_path_observer();
    let records = scenario.run_cases();

    assert_production_jvm_outputs(&records);
    scenario.assert_isolated_artifacts();
}

#[test]
fn production_paths_report_mixed_case_outcomes_in_discovered_order() {
    if !jdk_is_available() {
        return;
    }

    let project = TempProject::new("production-mixed-results");
    project.write(
        "main.veln",
        concat!(
            "# Mixed doctest fixture.\n",
            "## ```veln\n",
            "## stdio::println(\"doctest out\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## doctest out\n",
            "## ```\n",
            "pub fn documented() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    project.write(
        "main_test.veln",
        concat!(
            "test pass() -> () effects [stdio]\n",
            "  stdio::println(\"pass out\")\n",
            "  ()\n",
            "end\n",
            "test fail() -> Result<(), String>\n",
            "  Err(\"bad\")\n",
            "end\n",
            "test blocked() -> Result<(), String>\n",
            "  _\n",
            "end\n",
            "test runner() -> () effects [stdio]\n",
            "  stdio::println(\"runner should not stream\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let (analysis, cases) = analyzed_cases(&project.root);
    let analysis_mutex = Mutex::new(analysis);
    let execution = Arc::new(TestJvmExecution::ready(JvmExecution::for_test(
        project.root.join("cache/jvm"),
    )));
    let _hook = TestProgramHookGuard::install(Arc::new(
        |case_name,
         _build_dir,
         _event_file,
         _contract_error_file,
         _result_error_file,
         _java_args| {
            (case_name == "runner")
                .then(|| TestExecution::ToolError("injected runner tool error".to_string()))
        },
    ));

    let records = run_test_case_jobs(
        cases,
        3,
        |case| {
            let analysis = analysis_mutex.lock().expect("analysis state should lock");
            let mut job = prepare_test_case_job(&analysis, None, case);
            job.set_execution(Arc::clone(&execution));
            Ok::<_, String>(job)
        },
        execute_test_case_job,
    )
    .expect("mixed production path should complete");

    assert_eq!(
        case_names_of(&records),
        ["pass", "fail", "blocked", "runner", "doctest_1"]
    );
    assert_eq!(records[0].status, TestCaseStatus::Passed);
    assert_stdout_event(&records[0], "pass out");
    assert_eq!(records[1].status, TestCaseStatus::Failed);
    assert_eq!(
        records[1]
            .failure
            .as_ref()
            .map(|failure| failure.kind.as_str()),
        Some("result")
    );
    assert_eq!(records[2].status, TestCaseStatus::Blocked);
    assert_eq!(records[2].reason.as_deref(), Some("static_gate"));
    assert!(
        records[2]
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "hole.unfilled"),
        "blocked case should carry per-case lowering diagnostics"
    );
    assert_eq!(records[3].status, TestCaseStatus::Error);
    assert_eq!(records[3].reason.as_deref(), Some("runner_error"));
    assert_eq!(
        records[3]
            .failure
            .as_ref()
            .map(|failure| failure.message.as_str()),
        Some("injected runner tool error")
    );
    assert_eq!(records[4].kind, "doctest");
    assert_eq!(records[4].status, TestCaseStatus::Passed);
    assert_stdio_event(&records[4], "stdout", "doctest out");
}

#[test]
fn non_reusable_jobs_preflight_jvm_programs_before_cache_validation() {
    let project = TempProject::new("non-reusable-prepares-before-cache");
    project.write(
        "main_test.veln",
        concat!(
            "test pass() -> ()\n",
            "  ()\n",
            "end\n",
            "test blocked() -> Result<(), String>\n",
            "  _\n",
            "end\n",
            "test runner() -> () effects [stdio]\n",
            "  stdio::println(\"runner\")\n",
            "  ()\n",
            "end\n",
        ),
    );
    let (analysis, cases) = analyzed_cases(&project.root);
    assert!(
        analysis.reusable_standard_ir().is_none(),
        "fixture should force per-case JVM generation"
    );

    let has_ready_job = preflight_runnable_test_case_jobs(&analysis, None, &cases);

    assert!(has_ready_job);
    let jobs = cases
        .into_iter()
        .map(|case| prepare_test_case_job(&analysis, None, case))
        .collect::<Vec<_>>();
    assert_eq!(jobs.len(), 3);
    assert!(
        jobs.iter()
            .any(|job| matches!(job, TestCaseJob::Completed(case) if case.name == "blocked"))
    );
    let ready_names = jobs
        .iter()
        .filter_map(|job| match job {
            TestCaseJob::Ready(job) => {
                assert!(
                    job.java_args.is_empty(),
                    "per-case JVM programs should not need a reusable test-entry argument"
                );
                Some(job.case.name.as_str())
            }
            TestCaseJob::Completed(_) => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(ready_names, ["pass", "runner"]);
}

#[derive(Default)]
struct ProductionPathObservation {
    build_dirs: Vec<PathBuf>,
    event_files: Vec<PathBuf>,
    contract_error_files: Vec<PathBuf>,
    result_error_files: Vec<PathBuf>,
}

struct ProductionJvmScenario {
    _project: TempProject,
    analysis: Mutex<ProjectAnalysis>,
    cases: Vec<TestCase>,
    reusable_program: Option<JvmProgram>,
    execution: Arc<TestJvmExecution>,
    observed: Arc<(Mutex<ProductionPathObservation>, Condvar)>,
}

impl ProductionJvmScenario {
    fn new() -> Self {
        let project = TempProject::new("production-jvm-overlap");
        project.write("main_test.veln", production_jvm_fixture_source());
        let (analysis, cases) = analyzed_cases(&project.root);
        let case_names = cases
            .iter()
            .map(|case| case.name.clone())
            .collect::<Vec<_>>();
        let reusable_program = analysis
            .reusable_standard_ir()
            .map(|ir| generate_classfiles_with_test_entries(ir, &case_names));
        let execution = Arc::new(TestJvmExecution::ready(JvmExecution::for_test(
            project.root.join("cache/jvm"),
        )));
        Self {
            _project: project,
            analysis: Mutex::new(analysis),
            cases,
            reusable_program,
            execution,
            observed: Arc::new((
                Mutex::new(ProductionPathObservation::default()),
                Condvar::new(),
            )),
        }
    }

    fn install_path_observer(&self) -> TestProgramHookGuard {
        let observed = Arc::clone(&self.observed);
        TestProgramHookGuard::install(Arc::new(
            move |_case_name,
                  build_dir,
                  event_file,
                  contract_error_file,
                  result_error_file,
                  _java_args| {
                observe_overlapping_case_paths(
                    &observed,
                    build_dir,
                    event_file,
                    contract_error_file,
                    result_error_file,
                );
                None
            },
        ))
    }

    fn run_cases(&mut self) -> Vec<TestCase> {
        let cases = std::mem::take(&mut self.cases);
        let analysis = &self.analysis;
        let reusable_program = self.reusable_program.as_ref();
        let execution = Arc::clone(&self.execution);
        run_test_case_jobs(
            cases,
            2,
            |case| {
                let analysis = analysis.lock().expect("analysis state should lock");
                let mut job = prepare_test_case_job(&analysis, reusable_program, case);
                job.set_execution(Arc::clone(&execution));
                Ok::<_, String>(job)
            },
            execute_test_case_job,
        )
        .expect("production JVM path should complete")
    }

    fn assert_isolated_artifacts(&self) {
        let observation = self.observed.0.lock().expect("observation should lock");
        assert_eq!(observation.build_dirs.len(), 3);
        assert_unique_paths(&observation.build_dirs);
        assert_unique_paths(&observation.event_files);
        assert_unique_paths(&observation.contract_error_files);
        assert_unique_paths(&observation.result_error_files);
        assert!(
            observation.build_dirs.iter().all(|path| !path.exists()),
            "per-case build directories should be cleaned up after execution"
        );
    }
}

fn production_jvm_fixture_source() -> &'static str {
    concat!(
        "test alpha() -> () effects [stdio]\n",
        "  stdio::println(\"alpha out\")\n",
        "  stdio::eprintln(\"alpha err\")\n",
        "  ()\n",
        "end\n",
        "test beta() -> () effects [stdio]\n",
        "  stdio::println(\"beta out\")\n",
        "  stdio::eprintln(\"beta err\")\n",
        "  ()\n",
        "end\n",
        "test gamma() -> () effects [stdio]\n",
        "  stdio::println(\"gamma out\")\n",
        "  stdio::eprintln(\"gamma err\")\n",
        "  ()\n",
        "end\n",
    )
}

fn observe_overlapping_case_paths(
    observed: &(Mutex<ProductionPathObservation>, Condvar),
    build_dir: &Path,
    event_file: &Path,
    contract_error_file: &Path,
    result_error_file: &Path,
) {
    let (lock, cvar) = observed;
    let mut observation = lock.lock().expect("observation should lock");
    observation.build_dirs.push(build_dir.to_path_buf());
    observation.event_files.push(event_file.to_path_buf());
    observation
        .contract_error_files
        .push(contract_error_file.to_path_buf());
    observation
        .result_error_files
        .push(result_error_file.to_path_buf());
    cvar.notify_all();
    while observation.build_dirs.len() < 2 {
        observation = cvar
            .wait(observation)
            .expect("observation should lock after wait");
    }
}

fn assert_production_jvm_outputs(records: &[TestCase]) {
    assert_eq!(case_names_of(records), ["alpha", "beta", "gamma"]);
    assert_stdio_event(&records[0], "stdout", "alpha out");
    assert_stdio_event(&records[0], "stderr", "alpha err");
    assert_stdio_event(&records[1], "stdout", "beta out");
    assert_stdio_event(&records[1], "stderr", "beta err");
    assert_stdio_event(&records[2], "stdout", "gamma out");
    assert_stdio_event(&records[2], "stderr", "gamma err");
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("veln-cli-test-{name}-{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("test project directory should be created");
        Self { root }
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, text).expect("fixture should be written");
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn analyzed_cases(root: &Path) -> (ProjectAnalysis, Vec<TestCase>) {
    let project =
        Project::discover(root.to_path_buf(), &[]).expect("test project should be discovered");
    let analysis = analyze_project(project, DoctestMode::Include);
    let test_files = selected_test_files(&analysis.project, &analysis.module, None);
    let mut cases = discover_test_cases(&analysis.module, &test_files);
    attach_doctest_expectations(&mut cases, &analysis.doctest_expectations);
    (analysis, cases)
}

fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}
