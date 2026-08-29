use super::*;

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

    let records = run_test_case_jobs(named_cases(["alpha", "beta", "gamma"]), 1, Ok::<_, ()>, {
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
