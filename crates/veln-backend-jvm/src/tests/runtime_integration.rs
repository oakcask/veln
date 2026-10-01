use super::*;

fn run_java_runtime_harness(
    name: &str,
    source_name: &str,
    source: &str,
    program: &JvmProgram,
) -> std::process::Output {
    let root = temp_dir(name);
    write_jvm_program(&root, program);
    fs::write(root.join(source_name), source).expect("Java harness should be written");
    let compiled = Command::new("javac")
        .arg(source_name)
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        compiled.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let class_name = source_name
        .strip_suffix(".java")
        .expect("Java harness source should end in .java");
    let mut command = Command::new("java");
    command
        .arg("-cp")
        .arg(&root)
        .arg(class_name)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().expect("Java harness should run");
    let output = wait_for_test_child(child, name, std::time::Duration::from_secs(30));
    let _ = fs::remove_dir_all(&root);
    output
}

#[test]
fn jvm_runtime_preserves_cancellation_entering_host_connect_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-connect-cancellation",
        "ConnectCancellationHarness.java",
        include_str!("../../test-support/ConnectCancellationHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "connect cancellation remained primary\n"
    );
}

#[test]
fn jvm_runtime_join_blocks_until_cancelled_task_cleanup_finishes_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir(include_str!("../../test-support/join_blocking.veln"));
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-join-cleanup-blocking",
        "JoinBlockingHarness.java",
        include_str!("../../test-support/JoinBlockingHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "join waited for cleanup completion\n"
    );
}

#[test]
fn jvm_runtime_bounds_standard_input_read_ahead_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-stdin-backpressure");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeStdinBackpressureHarness.java"),
        RUNTIME_STDIN_BACKPRESSURE_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeStdinBackpressureHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeStdinBackpressureHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reads = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .expect("harness should print its input read count");
    assert!(reads <= 3, "stdin pump read ahead {reads} chunks");
}

#[test]
fn jvm_runtime_rechecks_partial_socket_write_deadline_and_cancellation_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-socket-partial-write");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeSocketPartialWriteHarness.java"),
        RUNTIME_SOCKET_PARTIAL_WRITE_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeSocketPartialWriteHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeSocketPartialWriteHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Operation not permitted") {
        return;
    }
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "WriteDeadlineExpired\nWriteCancelled\n"
    );
}

#[test]
fn jvm_runtime_preserves_path_values_across_standard_calls_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-path-values");
    write_jvm_program(&root, &program);
    fs::write(root.join("RuntimePathHarness.java"), RUNTIME_PATH_HARNESS)
        .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimePathHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimePathHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Ok(true)\nOk(true)\n"
    );
}

#[test]
fn jvm_runtime_reports_receiver_list_timeout_result_outcomes_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-channel-select-many-timeout-result");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeChannelSelectManyTimeoutResultHarness.java"),
        RUNTIME_CHANNEL_SELECT_MANY_TIMEOUT_RESULT_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeChannelSelectManyTimeoutResultHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeChannelSelectManyTimeoutResultHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "Ok(Some({index=1, value=21}))\n",
            "Ok(None)\n",
            "Err(interrupted)\n",
            "Err(cancelled)\n",
            "Err(cancelled)\n",
        )
    );
}

#[test]
fn jvm_runtime_list_helpers_traverse_large_lists_iteratively_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-list-helpers");
    write_jvm_program(&root, &program);
    fs::write(root.join("RuntimeListHarness.java"), RUNTIME_LIST_HARNESS)
        .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeListHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeListHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "20000:20000:true:false:2:Err(stop)\n"
    );
}

#[test]
fn jvm_runtime_reads_and_writes_byte_views_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-byte-view");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeByteViewHarness.java"),
        RUNTIME_BYTE_VIEW_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeByteViewHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeByteViewHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "Ok(1)\n",
            "Ok(258)\n",
            "Ok(66051)\n",
            "Ok(513)\n",
            "Ok(197121)\n",
            "ByteCount(3)\n",
            "Ok(258)\n",
            "Ok(3)\n",
            "Ok(515)\n",
            "Cons(ByteChunk([Byte(1), Byte(2)]), Cons(ByteChunk([Byte(3)]), Nil))\n",
            "Err(byte view count exceeds view length)\n",
            "Ok(16909311)\n",
            "Ok(2147483647)\n",
            "Ok(4294967295)\n",
            "Ok(1099511627775)\n",
            "Ok(281474976710655)\n",
            "Ok(2147483647)\n",
            "Ok(4294967295)\n",
            "Ok(1099511627775)\n",
            "Ok(281474976710655)\n",
            "Ok(9223372036854775807)\n",
            "Ok(9223372036854775807)\n",
            "Ok(ByteChunk([Byte(52), Byte(18)]))\n",
            "Ok(ByteChunk([Byte(3), Byte(2), Byte(1)]))\n",
            "Ok(ByteChunk([Byte(4), Byte(3), Byte(2), Byte(1)]))\n",
            "Err(byte read requires 3 bytes but view has 2)\n",
            "Err(byte read requires 3 bytes but view has 2)\n",
            "Err(byte view range exceeds chunk length)\n",
            "Err(byte_write_u8_be value must be between 0 and 255)\n",
            "Err(byte_write_u32_le value must be between 0 and 4294967295)\n",
            "Err(byte_read_u31_be value exceeds maximum 2147483647)\n",
            "Err(byte_read_u31_le value exceeds maximum 2147483647)\n",
            "Err(byte_read_u64_be value exceeds maximum 9223372036854775807)\n",
            "Err(byte_read_u64_le value exceeds maximum 9223372036854775807)\n",
            "Ok({length=5, kind=1, flags=4, stream_id=1})\n",
            "Ok({short_value=4660, wide_value=3735928559})\n",
            "Ok({length=5, padding_length=4})\n",
            "Err(schema validation failed at byte offset 3)\n",
            "Err(reserved bits mismatch at byte offset 5)\n",
        )
    );
}

#[test]
fn bytecode_backend_public_list_helpers_traverse_large_lists_iteratively_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn add(total: Int, value: Int) -> Int\n",
        "  total + value\n",
        "end\n",
        "fn double(value: Int) -> Int\n",
        "  value * 2\n",
        "end\n",
        "fn keep_one(value: Int) -> Bool\n",
        "  value == 1\n",
        "end\n",
        "fn ok_next(value: Int) -> Result<Int, String>\n",
        "  Ok(value + 1)\n",
        "end\n",
        "fn stop_at_two(value: Int) -> Result<Int, String>\n",
        "  match value == 2\n",
        "    true => Err(\"stop\")\n",
        "    false => Ok(value)\n",
        "  end\n",
        "end\n",
        "pub fn consume(values: List<Int>) -> () effects [stdio]\n",
        "  let mapped: List<Int> = list_map(values, double)\n",
        "  let tried: Result<List<Int>, String> = list_try_map(values, ok_next)\n",
        "  stdio::println(int_to_string(list_fold(values, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(mapped, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_filter(values, keep_one), 0, add)))\n",
        "  match tried\n",
        "    Ok(items) => stdio::println(int_to_string(list_fold(items, 0, add)))\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "  stdio::println(match list_try_map(list_cons(1, list_cons(2, values)), stop_at_two)\n",
        "    Ok(_) => \"unexpected\"\n",
        "    Err(error) => error\n",
        "  end)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "consume");
    let root = temp_dir("public-list-helpers");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("PublicListHelperHarness.java"),
        PUBLIC_LIST_HELPER_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("PublicListHelperHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("PublicListHelperHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "20000\n40000\n20000\n40000\nstop\n"
    );
}
