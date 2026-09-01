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
fn contract_binary_splitting_prefers_longest_tokens_and_left_associativity() {
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
