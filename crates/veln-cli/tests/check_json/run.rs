use super::support::*;

#[test]
fn run_forwards_stdout_and_stderr_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-stdio");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprintln(\"err\")\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "out\n");
    assert_eq!(stderr(&output), "err\n");
}

#[test]
fn run_passes_string_entry_arguments_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-entry-args");
    project.write(
        "main.veln",
        concat!(
            "pub fn greet(name: String) -> () effects [stdio]\n",
            "  stdio::println(name)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["greet", "main.veln", "--", "Ada"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Ada\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_treats_flag_like_values_after_separator_as_entry_arguments() {
    let project = TestProject::new("run-flag-like-entry-arg");
    project.write(
        "main.veln",
        "pub fn main(value: String) -> String\n  value\nend\n",
    );

    let output = project.run_with_path(&["main", "main.veln", "--", "--wat"], "");

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr(&output).contains("unknown run flag"),
        "post-separator entry argument should not be parsed as a flag: {}",
        stderr(&output)
    );
}

#[test]
fn run_converts_primitive_entry_arguments_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-primitive-entry-args");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(count: Int, ratio: Float, enabled: Bool) -> {count: Int, ratio: Float, enabled: Bool}\n",
            "  {count: count + 1, ratio: ratio + 0.5, enabled: not enabled}\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln", "--", "41", "1.5", "false"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_function_typed_value_calls_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-function-typed-value-call");
    project.write(
        "main.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "pub fn main() -> output: Int\n",
            "  ensure output == 2\n",
            "  let callback: fn(Int) -> Int effects [] = increment\n",
            "  callback(1)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_reports_byteview_read_truncation_human_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-byteview-read-truncation-human");
    project.write(
        "main.veln",
        concat!(
            "use stdio\n",
            "pub fn main() -> Result<(), String> effects [stdio]\n",
            "  stdio::eprintln(\"before truncation\")\n",
            "  let chunk: ByteChunk = byte_chunk_from_hex(\"0001\")?\n",
            "  let offset: ByteOffset = byte_offset(0)?\n",
            "  let count: ByteCount = byte_count(2)?\n",
            "  let view: ByteView = byte_view(chunk, offset, count)?\n",
            "  let ignored: Int = byte_read_u24_be(view)?\n",
            "  Ok(())\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "before truncation\n",
            "error[codec.incomplete_input]: missing byte at byte offset 2",
            "note: pending readiness is `need_bytes` because input is closed.",
            "note: Fixed-width read expected 3 byte(s); 2 byte(s) were available.",
        ],
    );
}

#[test]
fn run_executes_bounded_channel_send_and_receive_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-bounded-channel");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let pair: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  let output: String = match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_explicit_type_argument_bounded_channel_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-bounded-channel-type-arg");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let pair = channel::bounded<String>(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  let output: String = match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_channel_select_timeout_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-channel-select-timeout");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let left: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let right: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(right.tx, \"hello\")\n",
            "  let output: String = match channel::select_timeout(left.rx, right.rx, 10)\n",
            "    Some(selected) => selected.value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_channel_select_result_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-channel-select-result");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let left: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let right: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(right.tx, \"hello\")\n",
            "  let output: String = match channel::select_result(left.rx, right.rx)\n",
            "    Ok(Some(selected)) => selected.value\n",
            "    Ok(None) => \"missing\"\n",
            "    Err(_) => \"interrupted\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_channel_select_many_timeout_result_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-channel-select-many-timeout-result");
    project.write(
        "main.veln",
        concat!(
            "fn output_score(selected: Result<Option<{index: Int, value: Int}>, SelectError>) -> Int\n",
            "  match selected\n",
            "    Ok(Some(value)) => value.index * 100 + value.value\n",
            "    Ok(None) => 0\n",
            "    Err(_) => -1\n",
            "  end\n",
            "end\n",
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let first: {tx: Sender<Int>, rx: Receiver<Int>} = channel::bounded(1)\n",
            "  let second: {tx: Sender<Int>, rx: Receiver<Int>} = channel::bounded(1)\n",
            "  let third: {tx: Sender<Int>, rx: Receiver<Int>} = channel::bounded(1)\n",
            "  let _ = channel::send(second.tx, 21)\n",
            "  let _ = channel::send(third.tx, 34)\n",
            "  let ready: List<Receiver<Int>> = list_cons(first.rx, list_cons(second.rx, list_cons(third.rx, list_nil())))\n",
            "  stdio::println(int_to_string(output_score(channel::select_many_timeout_result(ready, 10))))\n",
            "  let timeout_first: {tx: Sender<Int>, rx: Receiver<Int>} = channel::bounded(1)\n",
            "  let timeout_second: {tx: Sender<Int>, rx: Receiver<Int>} = channel::bounded(1)\n",
            "  let waiting: List<Receiver<Int>> = list_cons(timeout_first.rx, list_cons(timeout_second.rx, list_nil()))\n",
            "  stdio::println(int_to_string(output_score(channel::select_many_timeout_result(waiting, 0))))\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "121\n0\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_task_spawn_and_join_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-task-spawn-join");
    project.write(
        "main.veln",
        concat!(
            "fn produce() -> String\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let task = task::spawn(produce)\n",
            "  let output: String = match task::join(task)\n",
            "    Ok(value) => value\n",
            "    Err(_) => \"failed\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_blocks_reachable_holes_before_jdk_execution() {
    let project = TestProject::new("run-hole");
    project.write(
        "main.veln",
        "pub fn main() -> Result<(), AppError>\n  _\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Result<(), AppError>` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_function_values_before_jdk_execution() {
    let project = TestProject::new("run-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], stringify)\n",
            "end\n",
            "fn stringify(value: Int) -> String\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `String` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_qualified_function_values_before_jdk_execution() {
    let project = TestProject::new("run-qualified-function-value-hole");
    project.write(
        "app/text.veln",
        concat!("fn stringify(value: Int) -> String\n", "  _\n", "end\n",),
    );
    project.write(
        "app/main.veln",
        concat!(
            "use app::text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], app::text::stringify)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "app/main.veln", "app/text.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `String` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_opaque_function_value_calls_before_jdk_execution() {
    let project = TestProject::new("run-opaque-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn invoke(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_opaque_local_function_value_calls_before_jdk_execution() {
    let project = TestProject::new("run-opaque-local-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn invoke() -> Bool\n",
            "  let job: fn() -> Bool = ready\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Bool\n",
            "  invoke()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_contract_helpers_before_jdk_execution() {
    let project = TestProject::new("run-contract-helper-hole");
    project.write(
        "main.veln",
        concat!(
            "fn positive(value: Int) -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> output: Int\n",
            "  ensure positive(output)\n",
            "  1\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_contract_function_values_before_jdk_execution() {
    let project = TestProject::new("run-contract-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  require accepts(ready)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_imported_calls_before_jdk_execution() {
    let project = TestProject::new("run-imported-call-hole");
    project.write(
        "app/util.veln",
        concat!("fn value() -> Int\n", "  _\n", "end\n",),
    );
    project.write(
        "app/main.veln",
        concat!(
            "use app::util\n",
            "pub fn main() -> Int\n",
            "  app::util::value()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "app/main.veln", "app/util.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Int` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_reports_parse_diagnostics_before_semantic_analysis() {
    let project = TestProject::new("run-parse-diagnostics");
    project.write("main.veln", "fn main() -> ()\n  @\nend\n");

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["main.veln:2:3: error[parse.invalid_token]: invalid token in expression"],
    );
}

#[test]
fn run_reports_semantic_diagnostics_before_lowering() {
    let project = TestProject::new("run-semantic-diagnostics");
    project.write("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["main.veln:2:3: error[type.mismatch]: expected `Int`, but found `String`"],
    );
}

#[test]
fn run_ignores_unreachable_semantic_diagnostics() {
    let project = TestProject::new("run-unreachable-semantic-diagnostics");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
            "fn later() -> Int\n",
            "  \"no\"\n",
            "end\n",
        ),
    );

    let output = project.run_with_path(&["main", "main.veln"], "");
    let stderr = stderr(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr,
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr.contains("type.mismatch"),
        "unreachable diagnostic should not block run: {stderr}"
    );
}

#[test]
fn run_ignores_function_shadowed_by_local_binding() {
    let project = TestProject::new("run-local-shadowed-function");
    project.write(
        "main.veln",
        concat!(
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  let helper = 1\n",
            "  helper\n",
            "end\n",
        ),
    );

    let output = project.run_with_path(&["main", "main.veln"], "");
    let stderr = stderr(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr,
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr.contains("hole.unfilled"),
        "shadowed function should not be reachable: {stderr}"
    );
}

#[test]
fn run_does_not_block_unreachable_holes_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-unreachable-hole");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"ran\")\n",
            "  ()\n",
            "end\n",
            "fn later() -> ()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ran\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_reports_missing_entry_before_jdk_execution() {
    let project = TestProject::new("run-missing-entry");
    project.write("main.veln", "pub fn main() -> ()\n  ()\nend\n");

    let output = project.run(&["missing", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: run entry `missing` was not found"],
    );
}

#[test]
fn run_rejects_wrong_entry_argument_count_before_jdk_execution() {
    let project = TestProject::new("run-entry-params");
    project.write(
        "main.veln",
        "pub fn main(value: String) -> String\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        concat!(
            "veln: run entry `main` expects 1 argument(s), got 0\n",
            "veln: note: pass entry arguments after `--`\n",
        )
    );
}

#[test]
fn run_rejects_unsupported_entry_parameters_before_jdk_execution() {
    let project = TestProject::new("run-entry-unsupported-param");
    project.write(
        "main.veln",
        "pub fn main(value: Vec<Int>) -> Vec<Int>\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "1"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        concat!(
            "veln: run entry parameter `value` cannot be supplied from a command-line argument\n",
            "veln: note: supported entry argument types are String, Int, Float, and Bool\n",
        )
    );
}

#[test]
fn run_rejects_invalid_typed_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-arg");
    project.write(
        "main.veln",
        "pub fn main(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "not-int"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Int argument for parameter `value`: `not-int`\n"
    );
}

#[test]
fn run_rejects_invalid_float_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-float");
    project.write(
        "main.veln",
        "pub fn main(value: Float) -> Float\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "not-float"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Float argument for parameter `value`: `not-float`\n"
    );
}

#[test]
fn run_rejects_invalid_bool_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-bool");
    project.write(
        "main.veln",
        "pub fn main(value: Bool) -> Bool\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "yes"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Bool argument for parameter `value`: `yes`\n"
    );
}

#[test]
fn run_json_reports_runtime_contract_failures_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-json-contract-failure");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "require false\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["--json", "main", "main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-run-json/v0\"",
            "\"command\":\"run\"",
            "\"status\":\"failed\"",
            "\"error\":{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `main` blame caller\"",
            "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",\"clause\":\"require\",\"predicate\":\"false\"",
            "\"function\":\"main\",\"blame\":\"caller\",\"node_id\":\"contract-",
            "\"span\":{\"file\":\"main.veln\"",
        ],
    );
}

#[test]
fn run_json_reports_success_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-json-success");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"ready\")\n",
            "end\n",
        ),
    );

    let output = project.run(&["--json", "main", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-run-json/v0\"",
            "\"command\":\"run\"",
            "\"status\":\"passed\"",
            "\"exit_code\":0",
            "\"stdout\":\"ready\\n\"",
            "\"stderr\":\"\"",
            "\"error\":null",
        ],
    );
}
