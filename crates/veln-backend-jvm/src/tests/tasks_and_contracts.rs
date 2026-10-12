use super::*;

#[test]
fn bytecode_backend_runs_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn produce() -> String\n",
        "  \"hello\"\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn(produce)\n",
        "  let value: String = task::join(task)?\n",
        "  stdio::println(value)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn bytecode_backend_binds_callsite_callbacks_for_tasks() {
    let ir = lower_to_ir(concat!(
        "fn produce(value: Int) -> {value: Int, location: SourceLocation} effects [concurrency] callsite\n",
        "  {value: value, location: callsite}\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let stored: fn(Int) -> {value: Int, location: SourceLocation} effects [concurrency] = produce\n",
        "  let task = task::spawn_with(stored, 41)\n",
        "  let result: {value: Int, location: SourceLocation} = task::join(task)?\n",
        "  stdio::println(int_to_string(result.value))\n",
        "  stdio::println(int_to_string(result.location.start_line))\n",
        "  stdio::println(int_to_string(result.location.start_column))\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-callsite-task", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "41\n6\n14\n");
}

#[test]
fn bytecode_backend_runs_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn produce(input: String) -> String effects [concurrency]\n",
        "  input\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with(produce, \"hello\")\n",
        "  let value: String = task::join(task)?\n",
        "  stdio::println(value)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn bytecode_backend_runs_record_context_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(context: {left: String, right: Int, marker: String}) -> {left: String, right: Int, marker: String} effects [concurrency]\n",
        "  { left: context.left, right: context.right, marker: context.marker }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let context = {left: \"hello\", right: 42, marker: \"done\"}\n",
        "  let task = task::spawn_with<{left: String, right: Int, marker: String}, {left: String, right: Int, marker: String}>(combine, context)\n",
        "  let value: {left: String, right: Int, marker: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.right))\n",
        "  stdio::println(value.marker)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-task-context", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n42\ndone\n");
}

#[test]
fn bytecode_backend_entry_reports_contract_failures_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: Int) -> output: Int\n",
        "  ensure output > 0\n",
        "  value\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Int]);

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-contract-failure", &program, &["0"])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: ensure `output > 0`"));
    assert!(stderr.contains("blame implementation"));
}

#[test]
fn bytecode_backend_evaluates_prefixed_contract_integers_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn bounded(value: Int) -> Int\n",
        "require value >= -0x0A\n",
        "require value <= 0b1010\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(bounded(0)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-prefixed-contract", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "0\n");
}

#[test]
fn bytecode_backend_executes_remainder_in_all_contract_positions() {
    let ir = lower_to_ir(concat!(
        "fn checked(value: Int, divisor: Int) -> output: Int\n",
        "require value % -divisor == 0\n",
        "require (-value) % 2 == 0\n",
        "ensure output % -divisor == 0\n",
        "invariant (-value) % divisor == 0\n",
        "  value\n",
        "end\n",
        "fn signed(value: Int, divisor: Int, expected: Int) -> output: Int\n",
        "require value % divisor == expected\n",
        "ensure output == expected\n",
        "invariant value % divisor == expected\n",
        "  value % divisor\n",
        "end\n",
        "fn endpoint(value: Int) -> Int\n",
        "require value % -1 == 0\n",
        "  value % -1\n",
        "end\n",
        "fn large_divisor(value: Int) -> Int\n",
        "require value % 9223372036854775807 == value\n",
        "  value % 9223372036854775807\n",
        "end\n",
        "fn nested_left(value: Int) -> Int\n",
        "require ((value % 6) * 2) % 4 == 0\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(checked(6, 3)))\n",
        "  stdio::println(int_to_string(checked(-6, -3)))\n",
        "  stdio::println(int_to_string(signed(-7, 3, -1)))\n",
        "  stdio::println(int_to_string(signed(7, -3, 1)))\n",
        "  stdio::println(int_to_string(signed(-7, -3, -1)))\n",
        "  stdio::println(int_to_string(endpoint(-9223372036854775807 - 1)))\n",
        "  stdio::println(int_to_string(large_divisor(1)))\n",
        "  stdio::println(int_to_string(nested_left(8)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-contract-remainder", &program, &[])
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
        "6\n-6\n-1\n1\n-1\n0\n1\n8\n"
    );
}

#[test]
fn bytecode_backend_reports_dynamic_zero_remainder_in_each_contract_position() {
    for (name, contract, operation, divisor_column, operation_column) in [
        (
            "require",
            "require value % divisor    == 0",
            "value % divisor",
            17,
            9,
        ),
        (
            "ensure",
            "ensure output % divisor    == 0",
            "output % divisor",
            17,
            8,
        ),
        (
            "invariant",
            "invariant value % divisor    == 0",
            "value % divisor",
            19,
            11,
        ),
    ] {
        let return_type = if name == "ensure" {
            "output: Int"
        } else {
            "Int"
        };
        let source = format!(
            "fn checked(value: Int, divisor: Int) -> {return_type}\n{contract}\n  value\nend\npub fn main() -> ()\n  checked(7, 0)\n  ()\nend\n"
        );
        let ir = lower_to_ir(&source);
        let program = generate_classfiles_with_entry(&ir, "main");

        let trace_root = temp_dir(&format!("bytecode-contract-{name}-remainder-zero-trace"));
        let trace_path = trace_root.join("runtime-errors.tsv");
        let trace_text = trace_path.to_string_lossy().to_string();

        let Some(output) = run_jvm_program_with_env_when_java_is_available(
            &format!("bytecode-contract-{name}-remainder-zero"),
            &program,
            &[("VELN_RUNTIME_ERRORS", trace_text.as_str())],
            &[],
        ) else {
            let _ = fs::remove_dir_all(&trace_root);
            return;
        };

        assert!(!output.status.success(), "{name}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("error[runtime.remainder_by_zero]: remainder divisor is zero"),
            "{name}: {stderr}"
        );
        assert!(!stderr.contains("ArithmeticException"), "{name}: {stderr}");
        let trace = fs::read_to_string(&trace_path).expect("remainder runtime trace");
        let divisor_start = source.find("divisor    ==").expect("divisor source offset");
        let operation_start = source.find(operation).expect("operation source offset");
        let divisor_end = divisor_start + "divisor".len();
        let operation_end_column = divisor_column + "divisor".len();
        let expected_coordinates = format!(
            "\t2\t{divisor_column}\t{divisor_start}\t2\t{operation_end_column}\t{divisor_end}\t2\t{operation_column}\t{operation_start}\t2\t{operation_end_column}\t{divisor_end}"
        );
        assert!(trace.contains(&expected_coordinates), "{name}: {trace}");
        let _ = fs::remove_dir_all(&trace_root);
    }
}

#[test]
fn return_contract_remainder_failure_preserves_output_and_cleanup_state() {
    let source = concat!(
        "fn zero() -> Int\n",
        "  let value = 1 - 1\n",
        "  value\n",
        "end\n",
        "fn checked() -> output: Int effects [stdio]\n",
        "ensure output % zero() == 0\n",
        "  defer\n",
        "    stdio::println(\"cleanup\")\n",
        "  end\n",
        "  stdio::println(\"body output\")\n",
        "  7\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  checked()\n",
        "  stdio::println(\"later work\")\n",
        "end\n",
    );
    let ir = lower_to_ir(source);
    let program = generate_classfiles_with_entry(&ir, "main");
    let trace_root = temp_dir("return-contract-remainder-state-trace");
    let trace_path = trace_root.join("runtime-errors.tsv");
    let trace_text = trace_path.to_string_lossy().to_string();
    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "return-contract-remainder-state",
        &program,
        &[("VELN_RUNTIME_ERRORS", trace_text.as_str())],
        &[],
    ) else {
        let _ = fs::remove_dir_all(&trace_root);
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "body output\ncleanup\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error[runtime.remainder_by_zero]: remainder divisor is zero"));
    assert!(!stderr.contains("Exception"), "{stderr}");
    let trace = fs::read_to_string(&trace_path).expect("return contract remainder trace");
    let divisor_start = source.find("zero() ==").expect("divisor offset");
    let operation_start = source.find("output % zero()").expect("operation offset");
    let divisor_end = divisor_start + "zero()".len();
    assert!(
        trace.contains(&format!(
            "\t6\t17\t{divisor_start}\t6\t23\t{divisor_end}\t6\t8\t{operation_start}\t6\t23\t{divisor_end}"
        )),
        "{trace}"
    );
    let _ = fs::remove_dir_all(&trace_root);
}

#[test]
fn bytecode_backend_reports_negated_dynamic_zero_remainder_in_contract() {
    let source = concat!(
        "fn checked(value: Int, divisor: Int) -> Int\n",
        "require value % -divisor == 0\n",
        "  value\n",
        "end\n",
        "pub fn main() -> ()\n",
        "  checked(7, 0)\n",
        "  ()\n",
        "end\n",
    );
    let ir = lower_to_ir(source);
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-contract-negated-remainder-zero",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error[runtime.remainder_by_zero]: remainder divisor is zero"),
        "{stderr}"
    );
    assert!(!stderr.contains("ClassCastException"), "{stderr}");
    assert!(!stderr.contains("ArithmeticException"), "{stderr}");
}

#[test]
fn bytecode_backend_executes_mixed_remainder_contracts_with_int_semantics() {
    for predicate in [
        "5 % 3 * 2 == 5",
        "5 % (3 * 2) == 4",
        "1 % 2 / 2 == 0.5",
        "5 % (3 / 2) == 1",
        "20 % 6 % 4 == 0",
        "20 % (6 % 4) == 1",
    ] {
        let ir = lower_to_ir(&format!(
            "fn checked() -> Int\nrequire {predicate}\n  0\nend\npub fn main() -> ()\n  checked()\n  ()\nend\n"
        ));
        let normalized_predicate = ir.functions[0].contracts[0].predicate.clone();
        let program = generate_classfiles_with_entry(&ir, "main");
        let Some(output) = run_jvm_program_when_java_is_available(
            "bytecode-contract-remainder-mixed",
            &program,
            &[],
        ) else {
            return;
        };
        assert!(!output.status.success(), "{predicate}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!(
                "contract failure: require `{normalized_predicate}`"
            )),
            "{predicate}: {stderr}"
        );
    }
}

#[test]
fn bytecode_backend_accounts_for_deep_remainder_helper_stack() {
    let mut predicate = "divisor".to_string();
    for value in (1..=120).rev() {
        predicate = format!("{value} % ({predicate})");
    }
    let ir = lower_to_ir(&format!(
        "fn checked(divisor: Int) -> Int\nrequire {predicate} == 1\n  divisor\nend\npub fn main() -> () effects [stdio]\n  stdio::println(int_to_string(checked(121)))\nend\n"
    ));
    assert_eq!(function_max_stack(&ir, "checked"), 134);
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-contract-remainder-deep", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "121\n");
}

#[test]
fn bytecode_backend_evaluates_chained_and_mixed_bitwise_contracts_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn shifted_quarter(value: Int) -> Int\n",
        "require value >> 1 >> 1 == 2\n",
        "  value >> 2\n",
        "end\n",
        "fn mixed_bits(value: Int) -> Int\n",
        "require (value >>> 1 & 3 ^ 1) == 2\n",
        "require ((~value & 15) | (1 << 4)) == 25\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(shifted_quarter(8)))\n",
        "  stdio::println(int_to_string(mixed_bits(6)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-bitwise-contract", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "2\n6\n");
}

#[test]
fn bytecode_backend_evaluates_contract_calls_and_fields_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn positive(value: Int) -> Bool\n",
        "  value > 0\n",
        "end\n",
        "fn summary(value: Int) -> {ready: Bool, next: Int}\n",
        "  {ready: positive(value), next: value + 1}\n",
        "end\n",
        "fn checked(value: Int) -> output: Int\n",
        "require positive(value)\n",
        "ensure summary(output).ready\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(checked(1)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-contract-call-field", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}

#[test]
fn bytecode_backend_evaluates_logical_contract_calls_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn positive(value: Int) -> Bool callsite\n",
        "  value > 0 and callsite.start_line > 0\n",
        "end\n",
        "fn available(value: Int) -> Bool callsite\n",
        "  value > 0 and callsite.start_column > 0\n",
        "end\n",
        "fn checked(value: Int) -> Int callsite\n",
        "require positive(value) and true\n",
        "require false or positive(value)\n",
        "require not positive(value) or available(value)\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(checked(1)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-logical-contract-call", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "1\n");
}

fn high_cardinality_contract(call_count: usize) -> TypedProgram {
    let mut text = String::new();
    for index in 0..call_count {
        text.push_str(&format!(
            "fn predicate_{index}() -> Int callsite\n  callsite.start_line\nend\n"
        ));
    }
    text.push_str("pub fn guarded() -> Bool callsite\nrequire ");
    for index in 0..call_count {
        if index > 0 {
            text.push_str(" + ");
        }
        text.push_str(&format!("predicate_{index}()"));
    }
    text.push_str(" > 0\n  true\nend\n");

    lower_to_ir(&text)
}

#[test]
fn contract_callsite_metadata_work_grows_linearly() {
    let smaller_count = 32;
    let larger_count = 64;
    let smaller = contract_call_metadata_work(&high_cardinality_contract(smaller_count), "guarded");
    let larger = contract_call_metadata_work(&high_cardinality_contract(larger_count), "guarded");

    assert_eq!(smaller, smaller_count * 2);
    assert_eq!(larger, larger_count * 2);
    assert_eq!(larger, smaller * 2);
}

fn flat_remainder_contract(operator_count: usize) -> TypedProgram {
    let mut predicate = String::from("value");
    for _ in 0..operator_count {
        predicate.push_str(" % 2");
    }
    lower_to_ir(&format!(
        "pub fn guarded(value: Int) -> Int\nrequire {predicate} == 0\n  value\nend\n"
    ))
}

fn independent_remainder_contract(operator_count: usize) -> TypedProgram {
    let mut left = String::from("left");
    let mut right = String::from("right");
    for _ in 0..operator_count {
        left.push_str(" % 2");
        right.push_str(" % 2");
    }
    lower_to_ir(&format!(
        "pub fn guarded(left: Int, right: Int) -> Int\nrequire ({left} == 0) and ({right} == 0)\n  left\nend\n"
    ))
}

fn right_nested_remainder_contract(operator_count: usize) -> TypedProgram {
    let mut predicate = String::from("divisor");
    for _ in 0..operator_count {
        predicate = format!("value % ({predicate})");
    }
    lower_to_ir(&format!(
        "pub fn guarded(value: Int, divisor: Int) -> Int\nrequire {predicate} == 0\n  value\nend\n"
    ))
}

#[test]
fn flat_remainder_contract_compacts_repeated_nonzero_literal_divisors() {
    assert_eq!(
        contract_remainder_emission_work(&flat_remainder_contract(4_097), "guarded"),
        1
    );
}

#[test]
fn independent_remainder_contract_scan_work_grows_linearly() {
    let smaller = contract_remainder_scan_work(&independent_remainder_contract(500), "guarded");
    let larger = contract_remainder_scan_work(&independent_remainder_contract(1_000), "guarded");

    assert!(
        smaller > 0,
        "the scan metric must observe predicate text visits"
    );
    assert!(
        larger <= smaller * 3,
        "doubling both chains must retain linear scan work: {smaller} -> {larger}"
    );
}

#[test]
fn right_nested_remainder_contract_work_grows_linearly() {
    let smaller = contract_remainder_scan_work(&right_nested_remainder_contract(120), "guarded");
    let larger = contract_remainder_scan_work(&right_nested_remainder_contract(121), "guarded");
    assert!(smaller > 0);
    assert!(
        larger <= smaller + 2,
        "adjacent nesting must add bounded lookup work: {smaller} -> {larger}"
    );
    assert_eq!(
        contract_remainder_emission_work(&right_nested_remainder_contract(121), "guarded"),
        121
    );
}

#[test]
fn independent_long_remainder_contract_emission_is_stack_safe() {
    let program = independent_remainder_contract(4_097);
    assert!(function_max_stack(&program, "guarded") < 128);
    assert_eq!(contract_remainder_emission_work(&program, "guarded"), 2);
}

#[test]
fn independent_remainder_contract_executes_without_recursive_emission() {
    let program = generate_classfiles_with_entry(
        &lower_to_ir(&format!(
            "{}pub fn main() -> Int\n  guarded(8, 6)\nend\n",
            {
                let mut left = String::from("left");
                let mut right = String::from("right");
                for _ in 0..4_097 {
                    left.push_str(" % 2");
                    right.push_str(" % 2");
                }
                format!(
                    "fn guarded(left: Int, right: Int) -> Int\nrequire ({left} == 0) or ({right} == 0)\n  left\nend\n"
                )
            }
        )),
        "main",
    );
    let Some(output) = run_jvm_program_when_java_is_available(
        "independent-remainder-contract-execution",
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
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}

#[test]
fn variadic_contract_calls_reuse_temporary_local_slots() {
    let call_count = 300;
    let mut text = String::from(concat!(
        "fn accepts(values: ...Int) -> Bool callsite\n",
        "  true\n",
        "end\n",
        "fn guarded() -> Bool callsite\n",
        "require ",
    ));
    for index in 0..call_count {
        if index > 0 {
            text.push_str(" and ");
        }
        text.push_str("accepts()");
    }
    text.push_str(concat!(
        "\n",
        "  true\n",
        "end\n",
        "pub fn main() -> Bool\n",
        "  guarded()\n",
        "end\n",
    ));

    let ir = lower_to_ir(&text);
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("variadic-contract-local-reuse", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn contract_binary_splitting_prefers_longest_tokens_and_left_associativity() {
    assert_eq!(
        split_contract_binary("left or right and tail", "or"),
        Some(("left", "right and tail"))
    );
    assert_eq!(
        split_contract_binary("candy and true", "and"),
        Some(("candy", "true"))
    );
    assert_eq!(split_contract_binary("candy", "and"), None);
    assert_eq!(
        split_contract_binary("value >> 1 >> 1", ">>"),
        Some(("value >> 1", "1"))
    );
    assert_eq!(split_contract_binary("value >>> 1", ">"), None);
    assert_eq!(split_contract_binary("value >>> 1", ">>"), None);
    assert_eq!(
        split_contract_binary("value >>> 1", ">>>"),
        Some(("value", "1"))
    );
    assert_eq!(split_contract_binary("value |> helper", "|"), None);
}

#[test]
fn bytecode_backend_entry_invariant_failure_blames_caller_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: Bool) -> Bool\n",
        "invariant value\n",
        "  value\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Bool]);

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-entry-invariant-failure",
        &program,
        &["false"],
    ) else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: invariant `value`"));
    assert!(stderr.contains("blame caller"));
}

#[test]
fn bytecode_backend_return_invariant_failure_blames_implementation_when_java_is_available() {
    let mut ir = lower_to_ir(concat!(
        "pub fn main(value: Bool) -> Bool\n",
        "invariant value\n",
        "  false\n",
        "end\n",
    ));
    // Exercise the return-position bytecode path directly; surface analysis
    // rejects result bindings that duplicate parameter names.
    ir.functions[0].return_binding = Some("value".to_string());
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Bool]);

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-return-invariant-failure",
        &program,
        &["true"],
    ) else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: invariant `value`"));
    assert!(stderr.contains("blame implementation"));
}

#[test]
fn bytecode_backend_javap_reports_target_version_and_entry_descriptor_when_available() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir("pub fn main(value: String) -> String\n  value\nend\n");
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let root = temp_dir("bytecode-javap");
    write_jvm_program(&root, &program);

    let output = Command::new("javap")
        .arg("-verbose")
        .arg("-classpath")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("javap should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("major version: 49"));
    assert!(stdout.contains("public static void main(java.lang.String[]);"));
    assert!(stdout.contains("descriptor: ([Ljava/lang/String;)V"));
}
