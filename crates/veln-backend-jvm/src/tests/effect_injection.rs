use super::*;

fn lower_with_host_effects(text: String) -> TypedProgram {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast_with_module_identity(
        &parsed.tree,
        "main".to_string(),
        source.span(TextRange::at(0)),
    );
    let host = SourceFile::new(
        "host_effects.veln",
        include_str!("../../../veln-stdlib/veln/host_effects.veln"),
    );
    let parsed = parse(&host);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let host_module = lower_surface_ast_with_module_identity(
        &parsed.tree,
        "std::host_effects".to_string(),
        host.span(TextRange::at(0)),
    );
    module.effects.extend(host_module.effects);
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.is_empty(),
        "{:#?}",
        lowered
            .diagnostics
            .iter()
            .map(|diagnostic| (&diagnostic.message, &diagnostic.span))
            .collect::<Vec<_>>()
    );
    lowered.ir.expect("clock fixture should lower")
}

pub(super) fn lower_with_clock(text: &str) -> TypedProgram {
    lower_with_host_effects(format!(
        "{}\n{text}",
        include_str!("../../test-support/clock.veln")
    ))
}

fn lower_with_network_facade(text: &str) -> TypedProgram {
    let facade = include_str!("../../../veln-stdlib/veln/net.veln")
        .strip_prefix("use prelude\n")
        .expect("network facade should import the standard prelude")
        .replace("pub type ByteCount = prelude::ByteCount\n", "");
    lower_with_host_effects(format!(
        "use host_effects from \"std\"\n{}\n{}\n{text}",
        facade,
        include_str!("../../test-support/network.veln"),
    ))
}

#[test]
fn wall_clock_values_are_normalized_and_may_move_backwards() {
    let ir = lower_with_clock(
        r#"
fn emit(value: WallTime) -> () effects [stdio]
    stdio::println(int_to_string(value.unix_seconds))
    stdio::println(int_to_string(value.nanosecond))
end
fn observe_backwards_clock() -> String effects [time]
    let first = time::wall_time()
    let second = time::wall_time()
    if first.unix_seconds == 2 and first.nanosecond == 0 and second.unix_seconds == 1 and second.nanosecond == 0
        "backwards"
    else
        "unexpected"
    end
end
pub fn main() -> () effects [time, concurrency, stdio]
    emit(handle time::wall_time() with wall_clock(0, 0))
    emit(handle time::wall_time() with wall_clock(0, -1))
    emit(handle time::wall_time() with wall_clock(0, 1000000000))
    let state = channel::bounded<Int>(1)
    let _ = channel::send(state.tx, 2)
    stdio::println(handle observe_backwards_clock() with backwards_wall_clock(state))
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let paths = program
        .classes
        .iter()
        .map(|class| class.path.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        paths.len(),
        paths
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "duplicate JVM class paths: {paths:#?}"
    );
    let Some(output) = run_jvm_program_when_java_is_available("wall-clock-values", &program, &[])
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
        "0\n0\n-1\n999999999\n1\n0\nbackwards\n"
    );
}

#[test]
fn unhandled_wall_clock_reads_the_real_host_clock() {
    let ir = lower_with_clock(
        r#"
pub fn main() -> () effects [time, stdio]
    let value = time::wall_time()
    stdio::println(if value.nanosecond >= 0 and value.nanosecond < 1000000000
        "host wall clock normalized"
    else
        "invalid host wall clock"
    end)
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available("host-wall-clock", &program, &[])
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
        "host wall clock normalized\n"
    );
}

#[test]
fn injected_clock_restores_host_default_after_exception() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_with_clock(
        r#"
fn fail() -> () effects [time]
    handle time::timeout_ms(0) with failing_clock("sleep", "injected clock failure")
end
pub fn main() -> () effects [time]
    handle fail() with fixed_clock(11)
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("clock-exception-restoration");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("ClockExceptionHarness.java"),
        include_str!("../../test-support/ClockExceptionHarness.java"),
    )
    .expect("exception catcher should be written");
    let compiled = Command::new("javac")
        .arg("ClockExceptionHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new("java")
        .args(["-cp", ".", "ClockExceptionHarness"])
        .current_dir(&root)
        .output()
        .expect("exception catcher should run");
    let _ = fs::remove_dir_all(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "exceptional clock scopes restored\n"
    );
}

#[test]
fn injected_clock_restores_nested_and_propagating_scopes() {
    let ir = lower_with_clock(
        r#"
fn fail() -> Result<Int, String> effects [time]
    let observed = time::monotonic_ms()
    let error: Result<(), String> = Err("expected")
    let _ = handle error? with fixed_clock(55)
    Ok(observed)
end
fn nested() -> () effects [time, stdio]
    stdio::println(int_to_string(time::monotonic_ms()))
    let inner = handle time::monotonic_ms() with fixed_clock(22)
    stdio::println(int_to_string(inner))
    let failed = handle fail() with fixed_clock(33)
    match failed
        Err(message) => stdio::println(message)
        Ok(_) => stdio::println("unexpected success")
    end
    stdio::println(int_to_string(time::monotonic_ms()))
end
pub fn main() -> () effects [time, stdio]
    handle nested() with fixed_clock(11)
    let fresh = handle time::monotonic_ms() with fixed_clock(44)
    stdio::println(int_to_string(fresh))
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available("clock-restoration", &program, &[])
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
        "11\n22\nexpected\n11\n44\n"
    );
}

#[test]
fn injected_clock_is_inherited_by_tasks_and_isolated_between_scopes() {
    let ir = lower_with_clock(
        r#"
fn read_clock(gate: Receiver<Int>) -> Int effects [time, concurrency]
    let _ = channel::recv(gate)
    let inner = handle time::monotonic_ms() with fixed_clock(99)
    time::monotonic_ms() + inner
end
fn read_without_argument() -> Int effects [time]
    time::monotonic_ms()
end
pub fn main() -> Result<(), JoinError> effects [time, concurrency, stdio]
    let first_gate = channel::bounded<Int>(1)
    let second_gate = channel::bounded<Int>(1)
    let first = handle task::spawn_with(read_clock, first_gate.rx) with fixed_clock(11)
    let second = handle task::spawn_with(read_clock, second_gate.rx) with fixed_clock(22)
    let third = handle task::spawn(read_without_argument) with fixed_clock(33)
    let _ = channel::send(first_gate.tx, 0)
    let _ = channel::send(second_gate.tx, 0)
    stdio::println(int_to_string(task::join(first)?))
    stdio::println(int_to_string(task::join(second)?))
    stdio::println(int_to_string(task::join(third)?))
    Ok(())
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available("clock-task-scopes", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "110\n121\n33\n");
}

#[test]
fn injected_network_handles_retain_their_creating_scope() {
    let ir = lower_with_network_facade(
        r#"
pub fn main() -> Result<(), String> effects [net, stdio]
    let first_chunk = byte_chunk_from_hex("01")?
    let second_chunk = byte_chunk_from_hex("0203")?
    let first = handle net::connect("fixture") with fixed_network("first", first_chunk)
    let second = handle net::connect("fixture") with fixed_network("second", second_chunk)
    let first_read = handle net::read_chunk(first) with fixed_network("shadow", second_chunk)
    let second_read = net::read_chunk(second)
    stdio::println(int_to_string(byte_count_to_int(byte_chunk_count(first_read))))
    stdio::println(int_to_string(byte_count_to_int(byte_chunk_count(second_read))))
    stdio::println(net::stream_peer_addr(first))
    stdio::println(net::stream_peer_addr(second))
    net::close_stream(first)
    net::close_stream(second)
    Ok(())
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("network-retained-scopes", &program, &[])
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
        "1\n2\nfirst\nsecond\n"
    );
}

#[test]
fn standard_network_facade_forwards_every_operation() {
    let ir = lower_with_network_facade(
        r#"
pub fn main() -> Result<(), String> effects [net, time]
    verify_network_facade()
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("standard-network-facade", &program, &[])
    else {
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
fn standard_network_write_all_preserves_the_effect_boundary() {
    let ir = lower_with_network_facade(
        r#"
pub fn main() -> Result<(), String> effects [net, time]
    verify_write_all()
end
"#,
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("standard-network-write-all", &program, &[])
    else {
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
fn standard_network_write_all_handles_long_one_byte_progress_in_linear_ranges() {
    let byte_count = 512;
    let input = "a".repeat(byte_count);
    let ir = lower_with_network_facade(&format!(
        r#"
pub fn main() -> Result<(), String> effects [net, time, concurrency]
    verify_write_all_one_byte_progress(byte_chunk_from_visible_ascii_string("{input}")?, {byte_count})
end
"#,
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "standard-network-write-all-one-byte-progress",
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
