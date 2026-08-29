use super::*;

#[test]
fn bytecode_backend_reports_forced_timeout_expiry_when_java_is_available() {
    let ir = lower_to_ir("pub fn main() -> () effects [time]\n  time::timeout_ms(5)\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-timeout-expiry",
        &program,
        &[("VELN_TIME_TIMEOUT_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport timeout expired: VELN_TIME_TIMEOUT_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_deadline_expiry_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  time::wait_until(deadline)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-deadline-expiry",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport deadline expired: VELN_TIME_DEADLINE_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_waits_until_deadline_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  time::wait_until(deadline)\n",
        "  stdio::println(\"deadline\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-deadline", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "deadline\n");
}

#[test]
fn bytecode_backend_waits_until_cancellable_deadline_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "  stdio::println(\"cancellable\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancellable-deadline", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancellable\n");
}

#[test]
fn bytecode_backend_returns_cancellable_wait_outcomes_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    prelude::CancellableWaitOutcome::WaitCompleted => \"completed\"\n",
        "    prelude::CancellableWaitOutcome::WaitDeadlineExpired => \"deadline\"\n",
        "    prelude::CancellableWaitOutcome::WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let completed_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let completed_token: CancelToken = time::cancel_token()\n",
        "  let completed: CancellableWaitOutcome = time::wait_until_cancellable_outcome(completed_deadline, completed_token)\n",
        "  stdio::println(outcome_text(completed))\n",
        "  let cancelled_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let cancelled_token: CancelToken = time::cancel_token()\n",
        "  time::cancel(cancelled_token)\n",
        "  let cancelled: CancellableWaitOutcome = time::wait_until_cancellable_outcome(cancelled_deadline, cancelled_token)\n",
        "  stdio::println(outcome_text(cancelled))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancellable-wait-outcomes", &program, &[])
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
        "completed\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_observes_cancel_token_status_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn status_text(cancelled: Bool) -> String\n",
        "  match cancelled\n",
        "    true => \"cancelled\"\n",
        "    false => \"active\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  stdio::println(status_text(time::is_cancelled(token)))\n",
        "  time::cancel(token)\n",
        "  stdio::println(status_text(time::is_cancelled(token)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancel-token-status", &program, &[])
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
        "active\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_observes_cancel_owner_status_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn status_text(cancelled: Bool) -> String\n",
        "  match cancelled\n",
        "    true => \"cancelled\"\n",
        "    false => \"active\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let owner: CancelOwner = time::cancel_owner()\n",
        "  stdio::println(status_text(time::is_cancelled_owner(owner)))\n",
        "  time::cancel_owned(owner)\n",
        "  stdio::println(status_text(time::is_cancelled_owner(owner)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancel-owner-status", &program, &[])
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
        "active\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_returns_forced_cancellable_wait_expiry_outcome_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    prelude::CancellableWaitOutcome::WaitCompleted => \"completed\"\n",
        "    prelude::CancellableWaitOutcome::WaitDeadlineExpired => \"deadline\"\n",
        "    prelude::CancellableWaitOutcome::WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  let outcome: CancellableWaitOutcome = time::wait_until_cancellable_outcome(deadline, token)\n",
        "  stdio::println(outcome_text(outcome))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-expiry-outcome",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "deadline\n");
}

#[test]
fn bytecode_backend_returns_forced_cancellable_wait_outcome_sequence_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    prelude::CancellableWaitOutcome::WaitCompleted => \"completed\"\n",
        "    prelude::CancellableWaitOutcome::WaitDeadlineExpired => \"deadline\"\n",
        "    prelude::CancellableWaitOutcome::WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let first_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let first_token: CancelToken = time::cancel_token()\n",
        "  let first: CancellableWaitOutcome = time::wait_until_cancellable_outcome(first_deadline, first_token)\n",
        "  stdio::println(outcome_text(first))\n",
        "  let second_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let second_token: CancelToken = time::cancel_token()\n",
        "  let second: CancellableWaitOutcome = time::wait_until_cancellable_outcome(second_deadline, second_token)\n",
        "  stdio::println(outcome_text(second))\n",
        "  let third_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let third_token: CancelToken = time::cancel_token()\n",
        "  let third: CancellableWaitOutcome = time::wait_until_cancellable_outcome(third_deadline, third_token)\n",
        "  stdio::println(outcome_text(third))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-outcome-sequence",
        &program,
        &[(
            "VELN_TIME_CANCELLABLE_OUTCOMES",
            "completed,deadline-expired,cancelled",
        )],
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
        "completed\ndeadline\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_cancellable_wait_expiry_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-deadline-expiry",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport deadline expired: VELN_TIME_DEADLINE_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_cancellable_wait_cancellation_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-cancelled",
        &program,
        &[("VELN_TIME_WAIT_CANCELLED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport wait cancelled: VELN_TIME_WAIT_CANCELLED\n"
    );
}

#[test]
fn bytecode_backend_reports_source_cancelled_wait_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::cancel(token)\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-source-cancelled-wait", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport wait cancelled: cancellation token\n"
    );
}
