use super::*;

#[test]
fn normal_completion_runs_registered_cleanup_once_in_reverse_order_with_snapshots() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  let captured = \"first\"\n",
        "  defer\n",
        "    stdio::println(captured)\n",
        "  end\n",
        "  let captured = \"shadowed\"\n",
        "  defer\n",
        "    stdio::println(\"second\")\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"third\")\n",
        "  end\n",
        "  stdio::println(\"body\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-normal-order", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "body\nthird\nsecond\nfirst\n"
    );
}

#[test]
fn cleanup_failure_keeps_running_remaining_cleanups_and_preserves_first_failure() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"remaining cleanup\")\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"first cleanup\")\n",
        "    let invalid_count = 64\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  stdio::println(\"body\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-first-cleanup-failure",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "body\nfirst cleanup\nremaining cleanup\n"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with("invalid shift count 64 for operator `<<`")
    );
}

#[test]
fn existing_runtime_failure_stays_primary_with_ordered_cleanup_failures() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"second cleanup\")\n",
        "    let invalid_count = 65\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"first cleanup\")\n",
        "    let invalid_count = 64\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  let invalid_count = 66\n",
        "  let ignored = 1 << invalid_count\n",
        "  ()\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-existing-runtime-failure",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "first cleanup\nsecond cleanup\n"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        concat!(
            "invalid shift count 66 for operator `<<`; expected a value between 0 and 63\n",
            "related cleanup failure: invalid shift count 64 for operator `<<`; expected a value between 0 and 63\n",
            "related cleanup failure: invalid shift count 65 for operator `<<`; expected a value between 0 and 63\n",
        )
    );
}

#[test]
fn existing_contract_failure_stays_primary_with_cleanup_failure_related() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn reject(value: Bool) -> ()\n",
        "require value\n",
        "  ()\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup\")\n",
        "    let invalid_count = 64\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  reject(false)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-existing-contract-failure-related",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cleanup\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let primary = stderr
        .find("contract failure: require `value` in `reject` blame caller")
        .expect("contract failure should remain visible");
    let related = stderr
        .find("related cleanup failure: invalid shift count 64 for operator `<<`; expected a value between 0 and 63")
        .expect("cleanup failure should be related");
    assert!(primary < related, "{stderr}");
}

#[test]
fn task_join_reports_cancellation_only_after_registered_cleanup_finishes() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn worker(context: { ready : Sender<String>, gate : Receiver<String> }) -> () effects [concurrency, stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup finished\")\n",
        "  end\n",
        "  let _ = channel::send(context.ready, \"ready\")\n",
        "  let _ = channel::recv(context.gate)\n",
        "  stdio::println(\"unexpected continuation\")\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, stdio]\n",
        "  let ready = channel::bounded<String>(1)\n",
        "  let gate = channel::bounded<String>(0)\n",
        "  let worker = task::spawn_with<(), { ready : Sender<String>, gate : Receiver<String> }>(worker, { ready: ready.tx, gate: gate.rx })\n",
        "  let _ = channel::recv(ready.rx)\n",
        "  task::cancel(worker)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"cancelled\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-task-cancellation", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup finished\ncancelled\n"
    );
}

#[test]
fn task_cancellation_interrupts_pure_tail_recursive_computation_before_join() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn spin(value: Int) -> ()\n",
        "  spin(value + 1)\n",
        "end\n",
        "fn worker(ready: Sender<String>) -> () effects [concurrency, stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup finished\")\n",
        "  end\n",
        "  let _ = channel::send(ready, \"ready\")\n",
        "  spin(0)\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, stdio]\n",
        "  let ready = channel::bounded<String>(1)\n",
        "  let worker = task::spawn_with<(), Sender<String>>(worker, ready.tx)\n",
        "  let _ = channel::recv(ready.rx)\n",
        "  task::cancel(worker)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"cancelled\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-tail-recursion-cancellation",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup finished\ncancelled\n"
    );
}

#[test]
fn task_cancellation_interrupts_host_time_wait_before_join() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn worker(ready: Sender<String>) -> () effects [concurrency, stdio, time]\n",
        "  defer\n",
        "    stdio::println(\"cleanup finished\")\n",
        "  end\n",
        "  let _ = channel::send(ready, \"ready\")\n",
        "  time::timeout_ms(60000)\n",
        "  stdio::println(\"unexpected continuation\")\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, stdio, time]\n",
        "  let ready = channel::bounded<String>(1)\n",
        "  let worker = task::spawn_with<(), Sender<String>>(worker, ready.tx)\n",
        "  let _ = channel::recv(ready.rx)\n",
        "  task::cancel(worker)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"cancelled\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-time-wait-cancellation",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup finished\ncancelled\n"
    );
}

#[test]
fn task_cancellation_unblocks_host_accept_before_join() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn worker(context: { ready : Sender<String>, listener : NetListener }) -> () effects [concurrency, net, stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup finished\")\n",
        "  end\n",
        "  let _ = channel::send(context.ready, \"ready\")\n",
        "  let _ = net::accept(context.listener)\n",
        "  stdio::println(\"unexpected continuation\")\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, net, stdio]\n",
        "  let listener = net::listen(\"127.0.0.1:0\")\n",
        "  let ready = channel::bounded<String>(1)\n",
        "  let worker = task::spawn_with<(), { ready : Sender<String>, listener : NetListener }>(worker, { ready: ready.tx, listener: listener })\n",
        "  let _ = channel::recv(ready.rx)\n",
        "  task::cancel(worker)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"cancelled\")\n",
        "  end\n",
        "  let address = net::listener_local_addr(listener)\n",
        "  let client = net::connect(address)\n",
        "  let server = net::accept(listener)\n",
        "  net::close_stream(client)\n",
        "  net::close_stream(server)\n",
        "  net::close_listener(listener)\n",
        "  stdio::println(\"listener reused\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-host-accept-cancellation",
        &program,
        &[],
    ) else {
        return;
    };

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Operation not permitted") {
        return;
    }
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup finished\ncancelled\nlistener reused\n"
    );
}

#[test]
fn nested_cleanup_failures_are_reported_once_with_linear_growth() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn fail_at(count: Int) -> ()\n",
        "  let ignored = 1 << count\n",
        "  ()\n",
        "end\n",
        "fn fail_nested(depth: Int) -> ()\n",
        "  defer\n",
        "    if depth > 0\n",
        "      fail_nested(depth - 1)\n",
        "    else\n",
        "      fail_at(64)\n",
        "    end\n",
        "  end\n",
        "  fail_at(65 + depth)\n",
        "end\n",
        "pub fn main() -> ()\n",
        "  fail_nested(24)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-nested-failure-linear-growth",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(stderr.lines().count(), 26, "{stderr}");
    for count in (64..=89).rev() {
        let message = format!("invalid shift count {count} for operator `<<`");
        assert_eq!(stderr.matches(&message).count(), 1, "{stderr}");
    }
}

#[test]
fn task_cancellation_with_cleanup_failures_returns_cancelled_join_error() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn worker(context: { ready : Sender<String>, gate : Receiver<String> }) -> () effects [concurrency, stdio]\n",
        "  defer\n",
        "    stdio::println(\"second cleanup\")\n",
        "    let invalid_count = 65\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"first cleanup\")\n",
        "    let invalid_count = 64\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  let _ = channel::send(context.ready, \"ready\")\n",
        "  let _ = channel::recv(context.gate)\n",
        "  stdio::println(\"unexpected continuation\")\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, stdio]\n",
        "  let ready = channel::bounded<String>(1)\n",
        "  let gate = channel::bounded<String>(0)\n",
        "  let worker = task::spawn_with<(), { ready : Sender<String>, gate : Receiver<String> }>(worker, { ready: ready.tx, gate: gate.rx })\n",
        "  let _ = channel::recv(ready.rx)\n",
        "  task::cancel(worker)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(error) => if task::join_error_is_cancelled(error)\n",
        "      stdio::println(\"cancelled\")\n",
        "    else\n",
        "      stdio::println(\"unexpected task error\")\n",
        "    end\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-cancellation-failure-related",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "first cleanup\nsecond cleanup\ncancelled\n"
    );
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn cancellation_requested_during_cleanup_preserves_the_body_failure() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn worker(context: { cleanup_started : Sender<String>, gate : Receiver<String> }) -> () effects [concurrency]\n",
        "  defer\n",
        "    let _ = channel::send(context.cleanup_started, \"started\")\n",
        "    let _ = channel::recv(context.gate)\n",
        "    ()\n",
        "  end\n",
        "  let invalid_count = 64\n",
        "  let ignored = 1 << invalid_count\n",
        "  ()\n",
        "end\n",
        "pub fn main() -> () effects [concurrency]\n",
        "  let cleanup_started = channel::bounded<String>(1)\n",
        "  let gate = channel::bounded<String>(0)\n",
        "  let worker = task::spawn_with<(), { cleanup_started : Sender<String>, gate : Receiver<String> }>(worker, { cleanup_started: cleanup_started.tx, gate: gate.rx })\n",
        "  let _ = channel::recv(cleanup_started.rx)\n",
        "  task::cancel(worker)\n",
        "  let _ = task::join(worker)\n",
        "  ()\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-late-cancellation", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        concat!(
            "invalid shift count 64 for operator `<<`; expected a value between 0 and 63\n",
            "related cleanup failure: task cancelled\n",
        )
    );
}

#[test]
fn begin_cleanup_finishes_before_its_value_is_transferred() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  let value = begin\n",
        "    defer\n",
        "      stdio::println(\"cleanup\")\n",
        "    end\n",
        "    \"value\"\n",
        "  end\n",
        "  stdio::println(value)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-begin-value", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cleanup\nvalue\n");
}

#[test]
fn acquisition_failure_before_registration_does_not_run_cleanup() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn acquire() -> Result<Int, String>\n",
        "  Err(\"not acquired\")\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let resource = acquire()?\n",
        "  defer\n",
        "    stdio::println(int_to_string(resource))\n",
        "  end\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-acquisition-failure",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert!(String::from_utf8_lossy(&output.stderr).contains("not acquired"));
}

#[test]
fn bytecode_backend_result_propagation_unwinds_registered_function_cleanups_only() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"function failure\")\n",
        "end\n",
        "fn worker() -> Result<(), String> effects [stdio]\n",
        "  let captured = \"captured before propagation\"\n",
        "  defer\n",
        "    stdio::println(captured)\n",
        "  end\n",
        "  let captured = \"shadowed before propagation\"\n",
        "  let value = fail()?\n",
        "  defer\n",
        "    stdio::println(\"registered too late\")\n",
        "  end\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  match worker()\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"caller observed error\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-function-propagation",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "captured before propagation\ncaller observed error\n"
    );
}

#[test]
fn result_propagation_keeps_running_cleanups_after_cleanup_failure() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"body failure\")\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"remaining cleanup\")\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"failing cleanup\")\n",
        "    let invalid_count = 64\n",
        "    let ignored = 1 << invalid_count\n",
        "    ()\n",
        "  end\n",
        "  let ignored = fail()?\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-result-propagation-cleanup-failure",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "failing cleanup\nremaining cleanup\n"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .starts_with("invalid shift count 64 for operator `<<`")
    );
}

#[test]
fn bytecode_backend_result_propagation_unwinds_nested_regions_inside_out_in_reverse_order() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"nested failure\")\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let captured = \"outer captured\"\n",
        "  defer\n",
        "    stdio::println(captured)\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"outer second\")\n",
        "  end\n",
        "  let captured = \"outer shadowed\"\n",
        "  let value = begin\n",
        "    let inner_captured = \"inner captured\"\n",
        "    defer\n",
        "      stdio::println(inner_captured)\n",
        "    end\n",
        "    defer\n",
        "      stdio::println(\"inner second\")\n",
        "    end\n",
        "    let inner_captured = \"inner shadowed\"\n",
        "    let ignored = fail()?\n",
        "    defer\n",
        "      stdio::println(\"registered too late\")\n",
        "    end\n",
        "    ()\n",
        "  end\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-nested-propagation",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "inner second\ninner captured\nouter second\nouter captured\n"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("nested failure"));
}

#[test]
fn unwind_result_slot_does_not_overwrite_a_later_cleanup_capture() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn succeed() -> Result<Int, String>\n",
        "  Ok(41)\n",
        "end\n",
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"expected failure\")\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let value = begin\n",
        "    let succeeded = succeed()?\n",
        "    succeeded + 1\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(int_to_string(value))\n",
        "  end\n",
        "  let ignored = fail()?\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-unwind-result-slot",
        &program,
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    assert!(String::from_utf8_lossy(&output.stderr).contains("expected failure"));
}

#[test]
fn result_propagation_restores_inner_handler_before_outer_cleanup() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "effect CleanupProbe\n",
        "  owner() -> String\n",
        "end\n",
        "handler cleanup_probe(label: String) handles CleanupProbe\n",
        "  owner() => label\n",
        "end\n",
        "fn fail() -> Result<(), String>\n",
        "  Err(\"expected\")\n",
        "end\n",
        "fn worker() -> Result<(), String> effects [CleanupProbe, stdio]\n",
        "  defer\n",
        "    stdio::println(perform CleanupProbe::owner())\n",
        "  end\n",
        "  let ignored = handle fail()? with cleanup_probe(\"inner\")\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let result = handle worker() with cleanup_probe(\"outer\")\n",
        "  match result\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"caller observed error\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-handler-unwind-order",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "outer\ncaller observed error\n"
    );
}

#[test]
fn result_propagation_clears_expression_operands_before_shared_cleanup() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn succeed(value: Int) -> Result<Int, String>\n",
        "  Ok(value)\n",
        "end\n",
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"expected failure\")\n",
        "end\n",
        "fn worker() -> Result<(), String> effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup\")\n",
        "  end\n",
        "  let first = succeed(succeed(1)? + 2)\n",
        "  let second = succeed(3)? + fail()?\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  match worker()\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"caller observed error\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "deferred-cleanup-expression-operands",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup\ncaller observed error\n"
    );
}

#[test]
fn bytecode_backend_contract_failure_unwinds_nested_regions_inside_out_with_snapshots() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "fn reject(value: Bool) -> ()\n",
        "require value\n",
        "  ()\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let captured = \"outer captured\"\n",
        "  defer\n",
        "    stdio::println(captured)\n",
        "  end\n",
        "  defer\n",
        "    stdio::println(\"outer second\")\n",
        "  end\n",
        "  let captured = \"outer shadowed\"\n",
        "  begin\n",
        "    let inner_captured = \"inner captured\"\n",
        "    defer\n",
        "      stdio::println(inner_captured)\n",
        "    end\n",
        "    defer\n",
        "      stdio::println(\"inner second\")\n",
        "    end\n",
        "    let inner_captured = \"inner shadowed\"\n",
        "    reject(false)\n",
        "    defer\n",
        "      stdio::println(\"registered too late\")\n",
        "    end\n",
        "    ()\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-contract-failure", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "inner second\ninner captured\nouter second\nouter captured\n"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("contract failure: require `value` in `reject` blame caller")
    );
}

#[test]
fn bytecode_backend_runtime_failure_unwinds_cleanup_and_handler_frames_in_lexical_order() {
    let ir = lower_deferred_cleanup_foundation_to_ir(concat!(
        "effect CleanupProbe\n",
        "  owner() -> String\n",
        "end\n",
        "handler cleanup_probe(label: String) handles CleanupProbe\n",
        "  owner() => label\n",
        "end\n",
        "fn worker() -> () effects [CleanupProbe, stdio]\n",
        "  defer\n",
        "    stdio::println(perform CleanupProbe::owner())\n",
        "  end\n",
        "  let ignored = handle begin\n",
        "    defer\n",
        "      stdio::println(perform CleanupProbe::owner())\n",
        "    end\n",
        "    let invalid_count = 64\n",
        "    1 << invalid_count\n",
        "  end with cleanup_probe(\"inner\")\n",
        "  ()\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  handle worker() with cleanup_probe(\"outer\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("deferred-cleanup-runtime-failure", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "inner\nouter\n");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("invalid shift count 64 for operator `<<`")
    );
}

#[test]
fn sequential_cleanup_regions_reuse_jvm_local_slots() {
    let mut source = String::from("pub fn main() -> ()\n");
    for index in 0..128 {
        source.push_str("  begin\n");
        source.push_str(&format!("    let value_{index}: Int = {index}\n"));
        source.push_str("    defer\n");
        source.push_str(&format!("      let copy: Int = value_{index}\n"));
        source.push_str("      ()\n");
        source.push_str("    end\n");
        source.push_str("    ()\n");
        source.push_str("  end\n");
    }
    source.push_str("  ()\nend\n");

    let ir = lower_deferred_cleanup_foundation_to_ir(&source);
    generate_classfiles_with_entry(&ir, "main");
}

#[test]
fn cleanup_free_function_preserves_the_supported_jvm_local_limit() {
    let mut source = String::from("pub fn main() -> ()\n");
    for index in 0..254 {
        source.push_str(&format!("  let value_{index}: Int = {index}\n"));
    }
    source.push_str("  ()\nend\n");

    let ir = lower_to_ir(&source);
    generate_classfiles_with_entry(&ir, "main");
}

#[test]
fn nested_cleanup_region_local_binding_retention_grows_linearly() {
    fn retention_at_depth(depth: usize) -> usize {
        let mut source = String::from("pub fn main() -> ()\n  defer\n    ()\n  end\n");
        for level in 0..depth {
            source.push_str(&"  ".repeat(level + 1));
            source.push_str("begin\n");
            source.push_str(&"  ".repeat(level + 2));
            source.push_str(&format!("let value_{level}: Int = {level}\n"));
        }
        source.push_str(&"  ".repeat(depth + 1));
        source.push_str("()\n");
        for level in (0..depth).rev() {
            source.push_str(&"  ".repeat(level + 1));
            source.push_str("end\n");
        }
        source.push_str("end\n");

        let ir = lower_deferred_cleanup_foundation_to_ir(&source);
        crate::classfile::local_binding_retention(&ir, "main")
    }

    assert_eq!(retention_at_depth(16), 32);
    assert_eq!(retention_at_depth(32), 64);
    assert_eq!(retention_at_depth(64), 128);
}

#[test]
fn doubling_cleanups_and_try_sites_keeps_bytecode_growth_below_threefold() {
    fn code_len(scale: usize) -> usize {
        let mut source = String::from(
            "fn fail() -> Result<Int, String>\n  Err(\"failure\")\nend\n\
             pub fn main() -> Result<(), String>\n",
        );
        for index in 0..scale {
            source.push_str(&format!("  let captured_{index}: Int = {index}\n"));
            source.push_str("  defer\n");
            source.push_str(&format!("    let copy: Int = captured_{index}\n"));
            source.push_str("    ()\n");
            source.push_str("  end\n");
        }
        for index in 0..scale {
            source.push_str(&format!("  let value_{index}: Int = fail()?\n"));
        }
        source.push_str("  Ok(())\nend\n");

        let ir = lower_deferred_cleanup_foundation_to_ir(&source);
        crate::classfile::function_code_footprint(&ir, "main")
    }

    let small = code_len(16);
    let large = code_len(32);
    assert!(
        large < small * 3,
        "doubling registrations and propagation sites grew bytecode from {small} to {large} bytes"
    );
}
