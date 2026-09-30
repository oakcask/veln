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
