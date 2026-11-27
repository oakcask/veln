use super::*;

#[test]
fn folded_scalars_execute_with_full_width_integer_results() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(1 + 2 * 3))\n",
        "  stdio::println(int_to_string(9223372036854775807 + 1))\n",
        "  stdio::println(int_to_string(-(9223372036854775807 + 1)))\n",
        "  stdio::println(int_to_string(-7 / 3))\n",
        "  stdio::println(int_to_string(-7 % 3))\n",
        "  stdio::println(int_to_string((9223372036854775807 + 1) / -1))\n",
        "  stdio::println(int_to_string((9223372036854775807 + 1) % -1))\n",
        "  stdio::println(int_to_string(-1 >>> 63))\n",
        "  stdio::println(int_to_string(~0))\n",
        "  if (1 + 2 == 3) and not false\n",
        "    stdio::println(\"true\")\n",
        "  else\n",
        "    stdio::println(\"false\")\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("constant-folding-results", &program, &[])
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
        concat!(
            "7\n",
            "-9223372036854775808\n",
            "-9223372036854775808\n",
            "-2\n",
            "-1\n",
            "-9223372036854775808\n",
            "0\n",
            "1\n",
            "-1\n",
            "true\n",
        )
    );
}

#[test]
fn folded_bitwise_not_method_omits_runtime_helper() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> Int\n  ~0\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("constant-folding-bitwise-not-javap");
    write_jvm_program(&root, &program);
    let output = Command::new("javap")
        .arg("-c")
        .arg("-p")
        .arg("-classpath")
        .arg(&root)
        .arg("VelnProgram")
        .output()
        .expect("javap should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let instructions = String::from_utf8_lossy(&output.stdout);
    assert!(instructions.contains("long -1l"), "{instructions}");
    assert!(
        !instructions.contains("VelnRuntime.bitwiseNot"),
        "{instructions}"
    );
}

#[test]
fn folded_method_materializes_minimum_int_without_operator_helpers() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Int\n",
        "  -(1 + 9223372036854775807)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("constant-folding-javap");
    write_jvm_program(&root, &program);
    let output = Command::new("javap")
        .arg("-c")
        .arg("-p")
        .arg("-classpath")
        .arg(&root)
        .arg("VelnProgram")
        .output()
        .expect("javap should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let instructions = String::from_utf8_lossy(&output.stdout);
    assert!(
        instructions.contains("long -9223372036854775808l"),
        "{instructions}"
    );
    for helper in ["VelnRuntime.add", "VelnRuntime.negate"] {
        assert!(!instructions.contains(helper), "{helper}: {instructions}");
    }
}

#[test]
fn dynamic_parent_retains_only_its_runtime_operator_helper() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }
    let ir = lower_to_ir("pub fn main(x: Int) -> Int\n  x + (2 * 3)\nend\n");
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Int]);
    let root = temp_dir("constant-folding-dynamic-javap");
    write_jvm_program(&root, &program);
    let output = Command::new("javap")
        .arg("-c")
        .arg("-p")
        .arg("-classpath")
        .arg(&root)
        .arg("VelnProgram")
        .output()
        .expect("javap should run");
    let _ = fs::remove_dir_all(&root);

    assert!(output.status.success());
    let instructions = String::from_utf8_lossy(&output.stdout);
    assert!(instructions.contains("VelnRuntime.add"), "{instructions}");
    assert!(instructions.contains("long 6l"), "{instructions}");
    assert!(
        !instructions.contains("VelnRuntime.multiply"),
        "{instructions}"
    );
}

#[test]
fn excluded_float_and_mixed_equality_operations_retain_runtime_helpers() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }
    for (name, source, helper) in [
        (
            "float",
            "pub fn main() -> Float\n  1 + 2.0\nend\n",
            "VelnRuntime.floatAdd",
        ),
        (
            "mixed-equality",
            "pub fn main() -> Bool\n  1 == true\nend\n",
            "VelnRuntime.equal",
        ),
    ] {
        let ir = lower_to_ir(source);
        let program = generate_classfiles_with_entry(&ir, "main");
        let root = temp_dir(&format!("constant-folding-excluded-{name}"));
        write_jvm_program(&root, &program);
        let output = Command::new("javap")
            .arg("-c")
            .arg("-p")
            .arg("-classpath")
            .arg(&root)
            .arg("VelnProgram")
            .output()
            .expect("javap should run");
        let _ = fs::remove_dir_all(&root);
        assert!(output.status.success(), "{name}");
        let instructions = String::from_utf8_lossy(&output.stdout);
        assert!(instructions.contains(helper), "{name}: {instructions}");
    }
}

#[test]
fn computed_zero_remainder_preserves_output_cleanup_and_source_context() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Int effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup\")\n",
        "  end\n",
        "  stdio::println(\"before\")\n",
        "  let result = 7 % (1 - 1)\n",
        "  stdio::println(\"after\")\n",
        "  result\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("constant-folding-remainder-failure", &program, &[])
    else {
        return;
    };

    assert!(!output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), "before\ncleanup\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("main.veln:6:"), "{stderr}");
    assert!(
        stderr.contains("error[runtime.remainder_by_zero]"),
        "{stderr}"
    );
    assert!(stderr.contains("Remainder operation is here."), "{stderr}");
}

#[test]
fn dynamic_operands_are_not_removed_or_duplicated() {
    for (name, expression) in [
        ("multiply", "0 * fail_int()"),
        ("and", "false and fail_bool()"),
        ("or", "true or fail_bool()"),
    ] {
        let source = format!(
            concat!(
                "fn fail_int() -> Int effects [stdio]\n",
                "  stdio::println(\"called\")\n",
                "  1 << (32 + 32)\n",
                "end\n",
                "fn fail_bool() -> Bool effects [stdio]\n",
                "  stdio::println(\"called\")\n",
                "  let _ = 1 << (32 + 32)\n",
                "  false\n",
                "end\n",
                "pub fn main() -> () effects [stdio]\n",
                "  let _ = {}\n",
                "  ()\n",
                "end\n",
            ),
            expression
        );
        let ir = lower_to_ir(&source);
        let program = generate_classfiles_with_entry(&ir, "main");
        let Some(output) = run_jvm_program_when_java_is_available(
            &format!("constant-folding-dynamic-{name}"),
            &program,
            &[],
        ) else {
            return;
        };
        assert!(!output.status.success(), "{name}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "called\n",
            "{name}"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("invalid shift count 64"),
            "{name}"
        );
    }
}

#[test]
fn computed_division_and_shift_failures_retain_source_context() {
    for (name, expression, id, message) in [
        (
            "divide",
            "7 / (1 - 1)",
            "runtime.division_by_zero",
            "division divisor is zero",
        ),
        (
            "shift-negative",
            "1 << (1 - 2)",
            "runtime.invalid_shift_count",
            "invalid shift count -1",
        ),
        (
            "shift-high",
            "1 >>> (32 + 32)",
            "runtime.invalid_shift_count",
            "invalid shift count 64",
        ),
    ] {
        let ir = lower_to_ir(&format!(
            "pub fn main() -> Int effects [stdio]\n  stdio::println(\"before\")\n  {expression}\nend\n"
        ));
        let program = generate_classfiles_with_entry(&ir, "main");
        let Some(output) = run_jvm_program_when_java_is_available(
            &format!("constant-folding-{name}-failure"),
            &program,
            &[],
        ) else {
            return;
        };
        assert!(!output.status.success(), "{name}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "before\n",
            "{name}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("main.veln:3:"), "{name}: {stderr}");
        assert!(stderr.contains(&format!("error[{id}]")), "{name}: {stderr}");
        assert!(stderr.contains(message), "{name}: {stderr}");
        assert!(stderr.contains("Operator `"), "{name}: {stderr}");
    }
}

#[test]
fn computed_division_failure_retains_task_join_error_boundary() {
    let ir = lower_to_ir(concat!(
        "fn divide() -> Int\n",
        "  7 / (1 - 1)\n",
        "end\n",
        "pub fn main() -> () effects [concurrency, stdio]\n",
        "  let worker = task::spawn(divide)\n",
        "  match task::join(worker)\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(_) => stdio::println(\"join error\")\n",
        "  end\n",
        "  stdio::println(\"after\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("constant-folding-division-task", &program, &[])
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
        "join error\nafter\n"
    );
}
