use super::*;

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
