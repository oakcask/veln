use super::effect_injection::lower_with_clock;
use super::*;

#[test]
fn bytecode_backend_reports_forced_timeout_expiry_when_java_is_available() {
    let ir = lower_with_clock(concat!(
        "fn run_test() -> () effects [time]\n  time::timeout_ms(5)\nend\n",
        "pub fn main() -> () effects [time, stdio]\n handle run_test() with failing_clock(\"sleep\", \"transport timeout expired: injected clock\")\nend\n"
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-timeout-expiry", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport timeout expired: injected clock\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_deadline_expiry_when_java_is_available() {
    let ir = lower_with_clock(concat!(
        "fn run_test() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  time::wait_until(deadline)\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n handle run_test() with failing_clock(\"wait\", \"transport deadline expired: injected clock\")\nend\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-deadline-expiry", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport deadline expired: injected clock\n"
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
fn entry_stack_overflow_exits_with_a_waiting_worker_when_java_is_available() {
    let ir = lower_to_ir(
        r#"fn wait_for_release(context: { entered : Sender<()>, release : Receiver<()> }) -> () effects [concurrency]
    let _ = channel::send(context.entered, ())
    let _ = channel::recv(context.release)
    ()
end
fn overflow(remaining: Int) -> Int
    if remaining <= 0
        0
    else
        1 + overflow(remaining - 1)
    end
end
fn fail_with_waiting_worker() -> () effects [concurrency]
    let entered = channel::bounded<()>(1)
    let release = channel::bounded<()>(1)
    let _ = task::spawn_with(wait_for_release, { entered: entered.tx, release: release.rx })
    let _ = channel::recv(entered.rx)
    let _ = overflow(1000000)
    ()
end
pub fn main() -> () effects [concurrency]
    fail_with_waiting_worker()
end
test overflow_with_waiting_worker() -> Result<(), String> effects [concurrency]
    fail_with_waiting_worker()
    Ok(())
end
"#,
    );
    let programs = [
        (
            "run-entry-stack-overflow",
            generate_classfiles_with_entry(&ir, "main"),
            Vec::new(),
        ),
        (
            "test-entry-stack-overflow",
            try_generate_classfiles_with_test_entries(
                &ir,
                &["overflow_with_waiting_worker".to_string()],
            )
            .expect("test entry should generate"),
            vec!["overflow_with_waiting_worker"],
        ),
    ];
    for (name, program, args) in programs {
        let Some(output) = run_jvm_program_with_java_args_when_java_is_available(
            name,
            &program,
            &["-Xss256k"],
            &args,
        ) else {
            return;
        };
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("java.lang.StackOverflowError"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
