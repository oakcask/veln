use super::*;

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

#[derive(Clone, Copy)]
struct DynamicZeroRemainderCase {
    name: &'static str,
    contract: &'static str,
    operation: &'static str,
    divisor_column: usize,
    operation_column: usize,
}

#[test]
fn bytecode_backend_reports_dynamic_zero_remainder_in_each_contract_position() {
    for case in [
        DynamicZeroRemainderCase {
            name: "require",
            contract: "require value % divisor    == 0",
            operation: "value % divisor",
            divisor_column: 17,
            operation_column: 9,
        },
        DynamicZeroRemainderCase {
            name: "ensure",
            contract: "ensure output % divisor    == 0",
            operation: "output % divisor",
            divisor_column: 17,
            operation_column: 8,
        },
        DynamicZeroRemainderCase {
            name: "invariant",
            contract: "invariant value % divisor    == 0",
            operation: "value % divisor",
            divisor_column: 19,
            operation_column: 11,
        },
    ] {
        check_dynamic_zero_remainder(case);
    }
}

fn check_dynamic_zero_remainder(case: DynamicZeroRemainderCase) {
    let return_type = if case.name == "ensure" {
        "output: Int"
    } else {
        "Int"
    };
    let source = format!(
        "fn checked(value: Int, divisor: Int) -> {return_type}\n{}\n  value\nend\npub fn main() -> ()\n  checked(7, 0)\n  ()\nend\n",
        case.contract
    );
    let program = generate_classfiles_with_entry(&lower_to_ir(&source), "main");
    let trace_root = temp_dir(&format!(
        "bytecode-contract-{}-remainder-zero-trace",
        case.name
    ));
    let trace_path = trace_root.join("runtime-errors.tsv");
    let trace_text = trace_path.to_string_lossy().to_string();
    let output = run_jvm_program_with_env_when_java_is_available(
        &format!("bytecode-contract-{}-remainder-zero", case.name),
        &program,
        &[("VELN_RUNTIME_ERRORS", trace_text.as_str())],
        &[],
    );
    let Some(output) = output else {
        let _ = fs::remove_dir_all(&trace_root);
        return;
    };

    assert_dynamic_zero_remainder(case, &source, &trace_path, &output);
    let _ = fs::remove_dir_all(&trace_root);
}

fn assert_dynamic_zero_remainder(
    case: DynamicZeroRemainderCase,
    source: &str,
    trace_path: &std::path::Path,
    output: &std::process::Output,
) {
    assert!(!output.status.success(), "{}", case.name);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error[runtime.remainder_by_zero]: remainder divisor is zero"),
        "{}: {stderr}",
        case.name
    );
    assert!(
        !stderr.contains("ArithmeticException"),
        "{}: {stderr}",
        case.name
    );
    let trace = fs::read_to_string(trace_path).expect("remainder runtime trace");
    let divisor_start = source.find("divisor    ==").expect("divisor source offset");
    let operation_start = source
        .find(case.operation)
        .expect("operation source offset");
    let divisor_end = divisor_start + "divisor".len();
    let operation_end_column = case.divisor_column + "divisor".len();
    let expected_coordinates = format!(
        "\t2\t{}\t{divisor_start}\t2\t{operation_end_column}\t{divisor_end}\t2\t{}\t{operation_start}\t2\t{operation_end_column}\t{divisor_end}",
        case.divisor_column, case.operation_column
    );
    assert!(
        trace.contains(&expected_coordinates),
        "{}: {trace}",
        case.name
    );
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
