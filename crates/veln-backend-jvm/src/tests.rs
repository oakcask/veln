use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::java::{
    concurrency_method, java_string, java_type_identifier, prelude_method,
    sanitize_identifier_text, standard_library_method, stdio_method, unique_java_identifier,
    veln_string_literal_value,
};
use crate::*;
use veln_ast::lower_surface_ast;
use veln_ir::{IrExprKind, IrStmtKind, TypedProgram};
use veln_sema::lower_checked_surface_module;
use veln_source::SourceFile;
use veln_syntax::parse;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn generates_program_and_runtime_sources_for_result_try_and_stdio() {
    let ir = lower_to_ir(concat!(
        "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
        "  Ok(1)\n",
        "end\n",
        "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  stdio::println(\"ok\")\n",
        "  Ok(())\n",
        "end\n",
    ));

    let java = generate_java_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("static Object fn_parse(Object p_raw)"));
    assert!(program.contains("static Object fn_main(Object p_raw)"));
    assert!(program.contains("Object __try0 = fn_parse(p_raw);"));
    assert!(program.contains("if (VelnRuntime.isErr(__try0))"));
    assert!(program.contains("Object v_value = VelnRuntime.unwrapOk(__try0);"));
    assert!(program.contains("VelnRuntime.stdioPrintln(\"ok\", \"call-"));
    assert!(program.contains("return VelnRuntime.ok(VelnRuntime.UNIT);"));
    assert!(runtime.contains("public static final class Result"));
    assert!(runtime.contains("public static final class Option"));
    assert!(runtime.contains("public static java.util.Map<String, Object> record"));
    assert!(runtime.contains("public static java.util.List<Object> list"));
    assert!(runtime.contains("public static java.util.Map<Object, Object> dict"));
    assert!(runtime.contains("private static java.util.List<Object> freezeList"));
    assert!(runtime.contains("private static <K, V> java.util.Map<K, V> freezeMap"));
}

#[test]
fn generates_runtime_values_for_records_lists_and_options() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result({message: String, values: Vec(String), maybe: Option(String), empty: Option(String)}, AppError) effects []\n",
        "  Ok({message: \"ok\", values: [\"a\", \"b\"], maybe: Some(\"x\"), empty: None})\n",
        "end\n",
    ));

    let java = generate_java_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(program.contains("VelnRuntime.record("));
    assert!(program.contains("\"message\", \"ok\""));
    assert!(program.contains("\"values\", VelnRuntime.list(\"a\", \"b\")"));
    assert!(program.contains("\"maybe\", VelnRuntime.some(\"x\")"));
    assert!(program.contains("\"empty\", VelnRuntime.none()"));
}

#[test]
fn generates_runtime_values_for_dictionary_literals() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result(Dict(String, Int), AppError) effects []\n",
        "  Ok({\"one\": 1, \"two\": 2})\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(
        program.contains("VelnRuntime.dict(\"one\", Long.valueOf(1L), \"two\", Long.valueOf(2L))")
    );
}

#[test]
fn generates_runtime_calls_for_bounded_channels() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> String effects [concurrency]\n",
        "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let producer = channel::clone(pair.tx)\n",
        "  let _ = channel::send(producer, \"hello\")\n",
        "  match channel::recv(pair.rx)\n",
        "    Some(value) => value\n",
        "    None => \"missing\"\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.channelBounded(Long.valueOf(1L))"));
    assert!(program.contains("VelnRuntime.channelClone("));
    assert!(program.contains("VelnRuntime.channelSend("));
    assert!(program.contains("VelnRuntime.channelRecv("));
    assert!(runtime.contains("public static final class Channel"));
    assert!(runtime.contains("private final long capacity;"));
    assert!(runtime.contains("public static Object channelBounded"));
    assert!(runtime.contains("public static Object channelClone"));
}

#[test]
fn generates_runtime_calls_for_channel_select() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> String effects [concurrency]\n",
        "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let _ = channel::send(right.tx, \"hello\")\n",
        "  match channel::select(left.rx, right.rx)\n",
        "    Some(selected) => selected.value\n",
        "    None => \"missing\"\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.channelSelect("));
    assert!(runtime.contains("public static Object channelSelect"));
    assert!(runtime.contains("private static Object channelSelectPoll"));
}

#[test]
fn generates_runtime_calls_for_channel_select_timeout() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> String effects [concurrency]\n",
        "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  match channel::select_timeout(left.rx, right.rx, 0)\n",
        "    Some(selected) => selected.value\n",
        "    None => \"missing\"\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.channelSelectTimeout("));
    assert!(runtime.contains("public static Object channelSelectTimeout"));
    assert!(runtime.contains("private static Object channelSelectWithTimeout"));
}

#[test]
fn generates_runtime_calls_for_channel_select_priority() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> String effects [concurrency]\n",
        "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  match channel::select_priority(left.rx, right.rx)\n",
        "    Some(selected) => selected.value\n",
        "    None => \"missing\"\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.channelSelectPriority("));
    assert!(runtime.contains("public static Object channelSelectPriority"));
}

#[test]
fn generates_runtime_calls_for_channel_select_result() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result(Option({index: Int, value: String}), SelectError) effects [concurrency]\n",
        "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  channel::select_result(left.rx, right.rx)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.channelSelectResult("));
    assert!(runtime.contains("public static Object channelSelectResult"));
    assert!(runtime.contains("return reportInterrupt ? err(\"interrupted\") : none();"));
}

#[test]
fn generated_runtime_rotates_select_tie_breaking() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects []\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");
    assert!(runtime.contains("SELECT_CURSOR"));

    let root = temp_dir("select-tie-breaking");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("SelectProbe.java"),
        r#"public final class SelectProbe {
    private SelectProbe() {}

    public static void main(String[] args) {
        Object leftPair = VelnRuntime.channelBounded(Long.valueOf(4L));
        Object rightPair = VelnRuntime.channelBounded(Long.valueOf(4L));
        java.util.Map<?, ?> left = (java.util.Map<?, ?>) leftPair;
        java.util.Map<?, ?> right = (java.util.Map<?, ?>) rightPair;
        Object leftTx = left.get("tx");
        Object leftRx = left.get("rx");
        Object rightTx = right.get("tx");
        Object rightRx = right.get("rx");

        VelnRuntime.channelSend(leftTx, "left-1");
        VelnRuntime.channelSend(rightTx, "right-1");
        long first = selectedIndex(VelnRuntime.channelSelect(leftRx, rightRx));

        VelnRuntime.channelSend(leftTx, "left-2");
        VelnRuntime.channelSend(rightTx, "right-2");
        long second = selectedIndex(VelnRuntime.channelSelect(leftRx, rightRx));

        if (first == second) {
            throw new AssertionError("select should rotate ready-receiver tie breaking");
        }
    }

    private static long selectedIndex(Object selected) {
        if (!VelnRuntime.isSome(selected)) {
            throw new AssertionError("select should produce a value");
        }
        java.util.Map<?, ?> record = (java.util.Map<?, ?>) VelnRuntime.optionValue(selected);
        return ((Long) record.get("index")).longValue();
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("SelectProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed: {}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("SelectProbe")
        .current_dir(&root)
        .output()
        .expect("java should run");
    assert!(
        java.status.success(),
        "java failed: {}",
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_prioritizes_left_select_receiver() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects []\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);

    let root = temp_dir("select-priority");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("SelectPriorityProbe.java"),
        r#"public final class SelectPriorityProbe {
    private SelectPriorityProbe() {}

    public static void main(String[] args) {
        Object leftPair = VelnRuntime.channelBounded(Long.valueOf(4L));
        Object rightPair = VelnRuntime.channelBounded(Long.valueOf(4L));
        java.util.Map<?, ?> left = (java.util.Map<?, ?>) leftPair;
        java.util.Map<?, ?> right = (java.util.Map<?, ?>) rightPair;
        Object leftTx = left.get("tx");
        Object leftRx = left.get("rx");
        Object rightTx = right.get("tx");
        Object rightRx = right.get("rx");

        VelnRuntime.channelSend(leftTx, "left-1");
        VelnRuntime.channelSend(rightTx, "right-1");
        long first = selectedIndex(VelnRuntime.channelSelectPriority(leftRx, rightRx));

        VelnRuntime.channelSend(leftTx, "left-2");
        VelnRuntime.channelSend(rightTx, "right-2");
        long second = selectedIndex(VelnRuntime.channelSelectPriority(leftRx, rightRx));

        if (first != 0L || second != 0L) {
            throw new AssertionError("priority select should prefer the left receiver");
        }
    }

    private static long selectedIndex(Object selected) {
        if (!VelnRuntime.isSome(selected)) {
            throw new AssertionError("select should produce a value");
        }
        java.util.Map<?, ?> record = (java.util.Map<?, ?>) VelnRuntime.optionValue(selected);
        return ((Long) record.get("index")).longValue();
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("SelectPriorityProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed: {}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("SelectPriorityProbe")
        .current_dir(&root)
        .output()
        .expect("java should run");
    assert!(
        java.status.success(),
        "java failed: {}",
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_reports_interrupted_channel_select_result() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects []\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);

    let root = temp_dir("select-result-interrupt");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("SelectResultInterruptProbe.java"),
        r#"public final class SelectResultInterruptProbe {
    private SelectResultInterruptProbe() {}

    public static void main(String[] args) throws Exception {
        Object leftPair = VelnRuntime.channelBounded(Long.valueOf(1L));
        Object rightPair = VelnRuntime.channelBounded(Long.valueOf(1L));
        java.util.Map<?, ?> left = (java.util.Map<?, ?>) leftPair;
        java.util.Map<?, ?> right = (java.util.Map<?, ?>) rightPair;
        Object leftRx = left.get("rx");
        Object rightRx = right.get("rx");
        final Object[] selected = new Object[1];

        Thread worker = new Thread(() -> {
            selected[0] = VelnRuntime.channelSelectResult(leftRx, rightRx);
        });
        worker.start();
        Thread.sleep(25L);
        worker.interrupt();
        worker.join(1000L);

        if (worker.isAlive()) {
            throw new AssertionError("select result should stop after interruption");
        }
        if (!VelnRuntime.isErr(selected[0])) {
            throw new AssertionError("select result should report interruption as Err");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("SelectResultInterruptProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed: {}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("SelectResultInterruptProbe")
        .current_dir(&root)
        .output()
        .expect("java should run");
    assert!(
        java.status.success(),
        "java failed: {}",
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generates_runtime_calls_for_tasks() {
    let ir = lower_to_ir(concat!(
        "fn produce() -> String effects []\n",
        "  \"hello\"\n",
        "end\n",
        "pub fn main() -> Result(String, JoinError) effects [concurrency]\n",
        "  let task = task::spawn(produce)\n",
        "  task::join(task)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.taskSpawn("));
    assert!(program.contains("VelnRuntime.taskJoin("));
    assert!(runtime.contains("public static final class Task"));
    assert!(runtime.contains("public static Object taskSpawn"));
    assert!(runtime.contains("public static Object taskJoin"));
    assert!(runtime.contains("public static Object taskCancel"));
}

#[test]
fn generated_runtime_serializes_concurrent_stdio_events() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(\"ok\")\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");
    assert!(runtime.contains("private static final Object stdioLock"));
    assert!(runtime.contains("synchronized (stdioLock)"));

    let root = temp_dir("concurrent-stdio");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("StdioProbe.java"),
        r#"public final class StdioProbe {
    private StdioProbe() {}

    public static void main(String[] args) throws Exception {
        Thread first = new Thread(() -> {
            for (int index = 0; index < 8; index += 1) {
                VelnRuntime.stdioPrintln("out-" + index, "call-out", "main.veln");
            }
        });
        Thread second = new Thread(() -> {
            for (int index = 0; index < 8; index += 1) {
                VelnRuntime.stdioEprintln("err-" + index, "call-err", "main.veln");
            }
        });
        first.start();
        second.start();
        first.join();
        second.join();
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("StdioProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let event_file = root.join("stdio-events.tsv");
    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("StdioProbe")
        .env("VELN_STDIO_EVENTS", &event_file)
        .output()
        .expect("java should run");

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
    let trace = fs::read_to_string(&event_file).expect("stdio trace should be written");
    let _ = fs::remove_dir_all(&root);
    let lines = trace.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 16, "{trace}");
    for (index, line) in lines.iter().enumerate() {
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 7, "{line}");
        assert_eq!(fields[0], (index + 1).to_string(), "{trace}");
    }
}

#[test]
fn generated_runtime_treats_zero_capacity_channel_as_rendezvous() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [concurrency]\n",
        "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(0)\n",
        "  let _ = channel::send(pair.tx, \"hello\")\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("zero-capacity-channel");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("ChannelProbe.java"),
        r#"public final class ChannelProbe {
    private ChannelProbe() {}

    public static void main(String[] args) throws Exception {
        Object pair = VelnRuntime.channelBounded(Long.valueOf(0L));
        Object tx = VelnRuntime.recordField(pair, "tx");
        Object rx = VelnRuntime.recordField(pair, "rx");
        final Object[] send = new Object[1];
        Thread sender = new Thread(() -> {
            send[0] = VelnRuntime.channelSend(tx, "hello");
        });
        sender.start();
        Thread.sleep(100L);
        if (!sender.isAlive()) {
            throw new AssertionError("zero-capacity send should wait for receiver");
        }
        Object received = VelnRuntime.channelRecv(rx);
        sender.join(1000L);
        if (sender.isAlive()) {
            throw new AssertionError("zero-capacity send should complete after receive");
        }
        if (!VelnRuntime.isSome(received)) {
            throw new AssertionError("zero-capacity receive should accept rendezvous value");
        }
        if (!"hello".equals(VelnRuntime.optionValue(received))) {
            throw new AssertionError("received unexpected value");
        }
        if (!VelnRuntime.isOk(send[0])) {
            throw new AssertionError("zero-capacity send should succeed after rendezvous");
        }
        VelnRuntime.channelClose(tx);
        Object closedSend = VelnRuntime.channelSend(tx, "again");
        if (!VelnRuntime.isErr(closedSend)) {
            throw new AssertionError("closed zero-capacity send should fail");
        }
        Object closedRecv = VelnRuntime.channelRecv(rx);
        if (!VelnRuntime.isNone(closedRecv)) {
            throw new AssertionError("closed zero-capacity channel should drain as none");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("ChannelProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("ChannelProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_rejects_zero_capacity_send_after_close() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [concurrency]\n",
        "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(0)\n",
        "  let _ = channel::close(pair.tx)\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("closed-zero-capacity-channel");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("ClosedChannelProbe.java"),
        r#"public final class ClosedChannelProbe {
    private ClosedChannelProbe() {}

    public static void main(String[] args) {
        Object pair = VelnRuntime.channelBounded(Long.valueOf(0L));
        Object tx = VelnRuntime.recordField(pair, "tx");
        Object rx = VelnRuntime.recordField(pair, "rx");
        VelnRuntime.channelClose(tx);
        Object send = VelnRuntime.channelSend(tx, "hello");
        if (!VelnRuntime.isErr(send)) {
            throw new AssertionError("closed zero-capacity send should fail");
        }
        if (!VelnRuntime.isNone(VelnRuntime.channelRecv(rx))) {
            throw new AssertionError("closed zero-capacity channel should drain as none");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("ClosedChannelProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("ClosedChannelProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generates_runtime_calls_for_fs_and_process_intrinsics() {
    let ir = lower_to_ir(concat!(
        "pub fn main(path: Path, key: String, status: Int) -> Result(String, FsError) effects [fs, process]\n",
        "  let args: Vec(String) = process::args()\n",
        "  let cwd: Result(Path, ProcessError) = process::cwd()\n",
        "  let value: Option(String) = process::env(key)\n",
        "  let exists: Result(Bool, FsError) = fs::exists(path)\n",
        "  let listed: Result(Vec(Path), FsError) = fs::read_dir(path)\n",
        "  let written: Result((), FsError) = fs::write_string(path, key)\n",
        "  process::exit(status)\n",
        "  fs::read_to_string(path)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.processArgs()"));
    assert!(program.contains("VelnRuntime.processCwd()"));
    assert!(program.contains("VelnRuntime.processEnv("));
    assert!(program.contains("VelnRuntime.fsExists("));
    assert!(program.contains("VelnRuntime.fsReadDir("));
    assert!(program.contains("VelnRuntime.fsWriteString("));
    assert!(program.contains("VelnRuntime.processExit("));
    assert!(program.contains("return VelnRuntime.fsReadToString("));
    assert!(runtime.contains("public static Object fsReadToString"));
    assert!(runtime.contains("public static Object fsWriteString"));
    assert!(runtime.contains("public static Object fsReadDir"));
    assert!(runtime.contains("public static Object processArgs"));
    assert!(runtime.contains("public static void setProcessArgs"));
}

#[test]
fn generated_entry_reads_file_with_fs_intrinsic() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main(path: Path) -> Result((), FsError) effects [fs, stdio]\n",
        "  let text: String = fs::read_to_string(path)?\n",
        "  stdio::println(text)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let java = generate_java_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let root = temp_dir("fs-read-runtime");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    let input = root.join("input.txt");
    fs::write(&input, "standard library fs").expect("input file should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .arg(input.to_string_lossy().as_ref())
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "standard library fs\n"
    );
}

#[test]
fn generated_runtime_writes_lists_and_reads_files_with_fs_intrinsics() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects []\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("fs-runtime-probe");
    let data_dir = root.join("data");
    fs::create_dir_all(&data_dir).expect("data directory should be created");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("FsProbe.java"),
        r#"public final class FsProbe {
    private FsProbe() {}

    public static void main(String[] args) {
        String dir = args[0];
        String target = dir + java.io.File.separator + "created.txt";

        Object write = VelnRuntime.fsWriteString(target, "standard library write");
        if (!VelnRuntime.isOk(write)) {
            throw new AssertionError("write_string should return Ok");
        }

        Object exists = VelnRuntime.fsExists(target);
        if (!VelnRuntime.isOk(exists) || !Boolean.TRUE.equals(VelnRuntime.unwrapOk(exists))) {
            throw new AssertionError("exists should report the written file");
        }

        Object read = VelnRuntime.fsReadToString(target);
        if (!VelnRuntime.isOk(read) || !"standard library write".equals(VelnRuntime.unwrapOk(read))) {
            throw new AssertionError("read_to_string should return written text");
        }

        Object listed = VelnRuntime.fsReadDir(dir);
        if (!VelnRuntime.isOk(listed) || !VelnRuntime.format(VelnRuntime.unwrapOk(listed)).contains("created.txt")) {
            throw new AssertionError("read_dir should include the written file");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("FsProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("FsProbe")
        .arg(&data_dir)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_compiler_support_source_loader_reads_text_with_fs_intrinsic() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let source = format!(
        "{}\n\npub fn main(path: Path) -> Result(String, FsError) effects [fs]\n  load_source_text(path)\nend\n",
        veln_stdlib::COMPILER_SUPPORT.text
    );
    let ir = lower_to_ir(&source);
    let java = generate_java(&ir);
    let root = temp_dir("compiler-support-source-loader");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    let input = root.join("input.veln");
    fs::write(&input, "fn parsed_by_compiler_support()\n  ()\nend\n")
        .expect("input source should be written");
    fs::write(
        root.join("SourceLoaderProbe.java"),
        r#"public final class SourceLoaderProbe {
    private SourceLoaderProbe() {}

    public static void main(String[] args) {
        Object result = VelnProgram.fn_main(args[0]);
        if (!VelnRuntime.isOk(result)) {
            throw new AssertionError("source load should return Ok");
        }
        Object text = VelnRuntime.unwrapOk(result);
        if (!"fn parsed_by_compiler_support()\n  ()\nend\n".equals(text)) {
            throw new AssertionError("source load should return file contents");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("SourceLoaderProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("SourceLoaderProbe")
        .arg(&input)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_reports_process_environment_and_cwd() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects []\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("process-runtime");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("ProcessProbe.java"),
        r#"public final class ProcessProbe {
    private ProcessProbe() {}

    public static void main(String[] args) {
        VelnRuntime.setProcessArgs(new String[] {"first", "second"});
        Object processArgs = VelnRuntime.processArgs();
        if (!"[first, second]".equals(VelnRuntime.format(processArgs))) {
            throw new AssertionError("process args should preserve entry arguments");
        }
        if (!VelnRuntime.isNone(VelnRuntime.processEnv("VELN_BACKEND_JVM_MISSING_ENV"))) {
            throw new AssertionError("missing environment key should return None");
        }
        Object cwd = VelnRuntime.processCwd();
        if (VelnRuntime.isErr(cwd)) {
            throw new AssertionError("cwd should return Ok for the host current directory");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("ProcessProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("ProcessProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_blocks_receive_until_value_is_sent() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [concurrency]\n",
        "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let _ = channel::send(pair.tx, \"hello\")\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("blocking-channel-recv");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("BlockingRecvProbe.java"),
        r#"public final class BlockingRecvProbe {
    private BlockingRecvProbe() {}

    public static void main(String[] args) throws Exception {
        Object pair = VelnRuntime.channelBounded(Long.valueOf(1L));
        Object tx = VelnRuntime.recordField(pair, "tx");
        Object rx = VelnRuntime.recordField(pair, "rx");
        Thread sender = new Thread(() -> {
            try {
                Thread.sleep(100L);
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
            }
            VelnRuntime.channelSend(tx, "hello");
        });
        sender.start();
        long before = System.currentTimeMillis();
        Object received = VelnRuntime.channelRecv(rx);
        long elapsed = System.currentTimeMillis() - before;
        sender.join(1000L);
        if (!VelnRuntime.isSome(received)) {
            throw new AssertionError("receive should wait for the sent value");
        }
        if (!"hello".equals(VelnRuntime.optionValue(received))) {
            throw new AssertionError("received unexpected value");
        }
        if (elapsed < 50L) {
            throw new AssertionError("receive returned before the delayed send");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("BlockingRecvProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("BlockingRecvProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generated_runtime_blocks_send_until_capacity_is_available() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [concurrency]\n",
        "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
        "  let _ = channel::send(pair.tx, \"hello\")\n",
        "  ()\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("blocking-channel-send");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("BlockingSendProbe.java"),
        r#"public final class BlockingSendProbe {
    private BlockingSendProbe() {}

    public static void main(String[] args) throws Exception {
        Object pair = VelnRuntime.channelBounded(Long.valueOf(1L));
        Object tx = VelnRuntime.recordField(pair, "tx");
        Object rx = VelnRuntime.recordField(pair, "rx");
        Object first = VelnRuntime.channelSend(tx, "first");
        if (!VelnRuntime.isOk(first)) {
            throw new AssertionError("initial send should fill channel");
        }
        final Object[] second = new Object[1];
        Thread sender = new Thread(() -> {
            second[0] = VelnRuntime.channelSend(tx, "second");
        });
        sender.start();
        Thread.sleep(100L);
        if (!sender.isAlive()) {
            throw new AssertionError("send should wait while channel is full");
        }
        Object received = VelnRuntime.channelRecv(rx);
        sender.join(1000L);
        if (sender.isAlive()) {
            throw new AssertionError("send should complete after receive frees capacity");
        }
        if (!VelnRuntime.isSome(received)) {
            throw new AssertionError("receive should return the queued value");
        }
        if (!"first".equals(VelnRuntime.optionValue(received))) {
            throw new AssertionError("received unexpected queued value");
        }
        if (!VelnRuntime.isOk(second[0])) {
            throw new AssertionError("blocked send should succeed after capacity is available");
        }
        Object later = VelnRuntime.channelRecv(rx);
        if (!VelnRuntime.isSome(later)) {
            throw new AssertionError("second send should enqueue its value");
        }
        if (!"second".equals(VelnRuntime.optionValue(later))) {
            throw new AssertionError("received unexpected second value");
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("BlockingSendProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("BlockingSendProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generates_record_pattern_matching() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: {count: Int, label: String}) -> String effects []\n",
        "  match value\n",
        "    {count: 0, label: name} => name\n",
        "    {count: count, label: _} => \"many\"\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.recordHasField("));
    assert!(program.contains("VelnRuntime.recordField("));
    assert!(runtime.contains("public static boolean recordHasField"));
}

#[test]
fn generated_runtime_freezes_container_values_at_public_boundaries() {
    let ir = lower_to_ir(concat!(
        "pub fn main(items: Vec(Int), table: Dict(String, Int)) -> Result({pushed: Vec(Int), inserted: Dict(String, Int)}, AppError) effects []\n",
        "  Ok({pushed: vec_push(items, 1), inserted: dict_insert(table, \"one\", 1)})\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(runtime.contains("return freezeMap(map);"));
    assert!(runtime.contains(
        "return freezeList(new java.util.ArrayList<Object>(java.util.Arrays.asList(values)));"
    ));
    assert!(runtime.contains("return freezeList(copy);"));
    assert!(runtime.contains("return freezeMap(copy);"));
    assert!(runtime.contains("return ok(freezeList(mapped));"));
    assert!(runtime.contains("private static Object freezeValue(Object value)"));
    assert!(runtime.contains("frozen.add(freezeValue(value));"));
    assert!(runtime.contains("V value = (V) freezeValue(entry.getValue());"));
}

#[test]
fn generated_runtime_transitively_freezes_nested_container_values() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result(Vec(Vec(String)), AppError) effects []\n",
        "  Ok([[\"a\"]])\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("freeze-runtime");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }
    fs::write(
        root.join("FreezeProbe.java"),
        r#"public final class FreezeProbe {
    private FreezeProbe() {}

    public static void main(String[] args) {
        java.util.ArrayList<Object> inner = new java.util.ArrayList<Object>();
        inner.add("x");
        java.util.List<Object> outer = VelnRuntime.list(inner);
        java.util.List<Object> frozenInner = (java.util.List<Object>) outer.get(0);
        expectFrozenList(outer);
        expectFrozenList(frozenInner);

        java.util.LinkedHashMap<Object, Object> mutable = new java.util.LinkedHashMap<Object, Object>();
        mutable.put("items", inner);
        VelnRuntime.Result result = VelnRuntime.ok(mutable);
        java.util.Map<Object, Object> frozenMap = (java.util.Map<Object, Object>) result.value();
        expectFrozenMap(frozenMap);
        expectFrozenList((java.util.List<Object>) frozenMap.get("items"));
    }

    private static void expectFrozenList(java.util.List<Object> value) {
        try {
            value.add("mutated");
            throw new AssertionError("list mutation succeeded");
        } catch (UnsupportedOperationException expected) {
        }
    }

    private static void expectFrozenMap(java.util.Map<Object, Object> value) {
        try {
            value.put("mutated", "value");
            throw new AssertionError("map mutation succeeded");
        } catch (UnsupportedOperationException expected) {
        }
    }
}
"#,
    )
    .expect("probe source should be written");

    let javac = Command::new("javac")
        .arg("VelnRuntime.java")
        .arg("FreezeProbe.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("FreezeProbe")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        java.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&java.stdout),
        String::from_utf8_lossy(&java.stderr)
    );
}

#[test]
fn generates_runtime_lookup_for_record_field_access() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Int effects []\n",
        "  {count: 1}.count\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains(
        "return VelnRuntime.recordField(VelnRuntime.record(\"count\", Long.valueOf(1L)), \"count\");"
    ));
    assert!(runtime.contains("public static Object recordField(Object record, String field)"));
}

#[test]
fn generates_entry_runner_for_selected_function() {
    let ir = lower_to_ir(concat!(
        "pub fn other() -> () effects []\n",
        "  ()\n",
        "end\n",
        "pub fn chosen() -> Result((), AppError) effects []\n",
        "  Ok(())\n",
        "end\n",
    ));

    let java = generate_java_with_entry(&ir, "chosen");
    let runner = java
        .source("VelnEntry.java")
        .expect("entry source should exist");

    assert!(runner.contains("Object result = VelnProgram.fn_chosen();"));
    assert!(runner.contains("if (VelnRuntime.isErr(result))"));
    assert!(runner.contains("System.exit(1);"));
}

#[test]
fn generates_entry_runner_argument_conversions() {
    let ir = lower_to_ir(concat!(
        "pub fn main(count: Int, ratio: Float, enabled: Bool) -> () effects []\n",
        "  ()\n",
        "end\n",
    ));

    let java = generate_java_with_entry_arg_types(
        &ir,
        "main",
        &[EntryArgType::Int, EntryArgType::Float, EntryArgType::Bool],
    );
    let runner = java
        .source("VelnEntry.java")
        .expect("entry source should exist");

    assert!(runner.contains("VelnProgram.fn_main(argInt(args[0], \"0\"), argFloat(args[1], \"1\"), argBool(args[2], \"2\"));"));
    assert!(runner.contains("Long.valueOf(Long.parseLong(text))"));
    assert!(runner.contains("Double.valueOf(Double.parseDouble(text))"));
    assert!(runner.contains("\"true\".equals(text)"));
}

#[test]
fn sanitizes_custom_class_names_and_entry_references() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result((), AppError) effects []\n",
        "  Ok(())\n",
        "end\n",
    ));

    let java = generate_java_with_entry_options(
        &ir,
        "main",
        &JavaBackendOptions {
            program_class: "9 bad-name".to_string(),
            runtime_class: "class".to_string(),
        },
    );
    let program = java
        .source("_9_bad_name.java")
        .expect("sanitized program source should exist");
    let runtime = java
        .source("VelnGenerated.java")
        .expect("fallback runtime source should exist");
    let runner = java
        .source("VelnEntry.java")
        .expect("entry source should exist");

    assert!(program.contains("public final class _9_bad_name"));
    assert!(program.contains("return VelnGenerated.ok(VelnGenerated.UNIT);"));
    assert!(runtime.contains("public final class VelnGenerated"));
    assert!(runner.contains("Object result = _9_bad_name.fn_main();"));
    assert!(runner.contains("if (VelnGenerated.isErr(result))"));
}

#[test]
fn sanitizes_java_keywords_and_colliding_identifiers() {
    let mut ir = lower_to_ir(concat!(
        "fn add(left: Int, right: Int) -> Int effects []\n",
        "  let total: Int = left + right\n",
        "  total\n",
        "end\n",
    ));
    let function = &mut ir.functions[0];
    function.name = "class".to_string();
    function.params[0].name = "a-b".to_string();
    function.params[1].name = "a_b".to_string();
    if let IrStmtKind::Let { name, value, .. } = &mut function.body[0].kind {
        *name = "return".to_string();
        if let IrExprKind::Binary { left, right, .. } = &mut value.kind {
            left.kind = IrExprKind::Local("a-b".to_string());
            right.kind = IrExprKind::Local("a_b".to_string());
        }
    }
    if let IrStmtKind::Return { value } = &mut function.body[1].kind {
        value.kind = IrExprKind::Local("return".to_string());
    }

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(program.contains("static Object fn_class(Object p_a_b, Object p_a_b_1)"));
    assert!(program.contains("Object v_return = VelnRuntime.add(p_a_b, p_a_b_1);"));
    assert!(program.contains("return v_return;"));
}

#[test]
fn generates_runtime_calls_for_value_call_prefix_and_binary_ops() {
    let ir = lower_to_ir(concat!(
        "fn inc(value: Int) -> Int\n",
        "  value + 1\n",
        "end\n",
        "pub fn main(callback: fn(Int) -> Int, a: Int, b: Int, flag: Bool) -> {",
        "called: Int, negated: Int, inverted: Bool, add: Int, sub: Int, mul: Int, div: Int, ",
        "eq: Bool, ne: Bool, lt: Bool, le: Bool, gt: Bool, ge: Bool, anded: Bool, ored: Bool, piped: Int",
        "} effects []\n",
        "  {called: callback(1), negated: -a, inverted: not flag, add: a + b, sub: a - b, ",
        "mul: a * b, div: a / b, eq: a == b, ne: a != b, lt: a < b, le: a <= b, ",
        "gt: a > b, ge: a >= b, anded: flag and false, ored: flag or true, piped: a |> inc()}\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(program.contains("\"called\", VelnRuntime.call(p_callback, Long.valueOf(1L))"));
    assert!(program.contains("\"negated\", VelnRuntime.negate(p_a)"));
    assert!(program.contains("\"inverted\", VelnRuntime.not(p_flag)"));
    assert!(program.contains("\"add\", VelnRuntime.add(p_a, p_b)"));
    assert!(program.contains("\"sub\", VelnRuntime.subtract(p_a, p_b)"));
    assert!(program.contains("\"mul\", VelnRuntime.multiply(p_a, p_b)"));
    assert!(program.contains("\"div\", VelnRuntime.divide(p_a, p_b)"));
    assert!(program.contains("\"eq\", VelnRuntime.equal(p_a, p_b)"));
    assert!(program.contains("\"ne\", VelnRuntime.notEqual(p_a, p_b)"));
    assert!(program.contains("\"lt\", VelnRuntime.less(p_a, p_b)"));
    assert!(program.contains("\"le\", VelnRuntime.lessEqual(p_a, p_b)"));
    assert!(program.contains("\"gt\", VelnRuntime.greater(p_a, p_b)"));
    assert!(program.contains("\"ge\", VelnRuntime.greaterEqual(p_a, p_b)"));
    assert!(program.contains("\"anded\", VelnRuntime.and(p_flag, Boolean.FALSE)"));
    assert!(program.contains("\"ored\", VelnRuntime.or(p_flag, Boolean.TRUE)"));
    assert!(program.contains("\"piped\", fn_inc(p_a)"));
}

#[test]
fn generates_runtime_support_for_float_numeric_ops() {
    let ir = lower_to_ir(concat!(
        "pub fn main(left: Float, right: Float) -> {negated: Float, sum: Float, ordered: Bool} effects []\n",
        "  {negated: -left, sum: left + right, ordered: left <= right}\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("\"negated\", VelnRuntime.floatNegate(p_left)"));
    assert!(program.contains("\"sum\", VelnRuntime.floatAdd(p_left, p_right)"));
    assert!(program.contains("\"ordered\", VelnRuntime.floatLessEqual(p_left, p_right)"));
    assert!(runtime.contains("public static Object floatAdd(Object left, Object right)"));
    assert!(runtime.contains("public static Object floatLessEqual(Object left, Object right)"));
    assert!(!runtime.contains("requireFloatOperands"));
    assert!(runtime.contains("return Double.valueOf(asDouble(left) + asDouble(right));"));
    assert!(runtime.contains("return Boolean.valueOf(asDouble(left) <= asDouble(right));"));
}

#[test]
fn generated_float_comparison_allows_nan_operands() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Bool effects []\n",
        "  0.0 / 0.0 < 1.0\n",
        "end\n",
    ));
    let java = generate_java_with_entry(&ir, "main");
    let root = temp_dir("float-comparison-nan");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(java.status.success());
}

#[test]
fn generated_float_arithmetic_allows_nan_operands() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Float effects []\n",
        "  0.0 / 0.0 + 1.0\n",
        "end\n",
    ));
    let java = generate_java_with_entry(&ir, "main");
    let root = temp_dir("float-arithmetic-nan");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let java = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(java.status.success());
}

#[test]
fn generates_runtime_calls_for_prelude_helpers() {
    let ir = lower_to_ir(concat!(
        "pub fn main(items: Vec(Int), table: Dict(String, Int), mapper: fn(Int) -> String) -> {",
        "count: Int, pushed: Vec(Int), mapped: Vec(String), found: Option(Int), inserted: Dict(String, Int)",
        "} effects []\n",
        "  {count: vec_len(items), pushed: vec_push(items, 1), mapped: vec_map(items, mapper), ",
        "found: dict_get(table, \"a\"), inserted: dict_insert(table, \"b\", 2)}\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("\"count\", VelnRuntime.vecLen(p_items)"));
    assert!(program.contains("\"pushed\", VelnRuntime.vecPush(p_items, Long.valueOf(1L))"));
    assert!(program.contains("\"mapped\", VelnRuntime.vecMap(p_items, p_mapper)"));
    assert!(program.contains("\"found\", VelnRuntime.dictGet(p_table, \"a\")"));
    assert!(
        program.contains("\"inserted\", VelnRuntime.dictInsert(p_table, \"b\", Long.valueOf(2L))")
    );
    assert!(runtime.contains("public static Object vecTryMap"));
    assert!(runtime.contains("public static Object resultAndThen"));
}

#[test]
fn generates_function_values_for_declared_functions() {
    let ir = lower_to_ir(concat!(
        "fn stringify(value: Int) -> String effects []\n",
        "  \"ok\"\n",
        "end\n",
        "pub fn main(items: Vec(Int)) -> Vec(String) effects []\n",
        "  vec_map(items, stringify)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(
        program.contains(
            "VelnRuntime.vecMap(p_items, (VelnRuntime.Fn) ((Object... fnArgs) -> fn_stringify(fnArgs[0])))"
        )
    );

    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let root = temp_dir("function-value");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let output = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn escapes_string_literals_and_emits_result_errors() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> Result(String, String) effects []\n",
        "  Err(\"line\\n\\\"quoted\\\"\\\\tail\")\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(program.contains("return VelnRuntime.err(\"line\\n\\\"quoted\\\"\\\\tail\");"));
}

#[test]
fn emits_match_expression_branches() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: Option(Int)) -> Int effects []\n",
        "  match value\n",
        "    Some(count) => count + 1\n",
        "    None => 0\n",
        "  end\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("if (VelnRuntime.isSome("));
    assert!(program.contains("VelnRuntime.optionValue("));
    assert!(program.contains("else if (VelnRuntime.isNone("));
    assert!(runtime.contains("public static boolean isSome"));
    assert!(runtime.contains("public static Object optionValue"));
}

#[test]
fn emits_runtime_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn clamp(value: Int) -> result: Int effects []\n",
        "  require value >= 0\n",
        "  invariant value >= 0\n",
        "  ensure result >= value\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");
    let runtime = java
        .source("VelnRuntime.java")
        .expect("runtime source should exist");

    assert!(program.contains("VelnRuntime.checkContract("));
    assert!(program.contains("\"require\", \"value >= 0\", \"clamp\", \"caller\""));
    assert!(program.contains("\"invariant\", \"value >= 0\", \"clamp\", \"caller\""));
    assert!(program.contains("\"invariant\", \"value >= 0\", \"clamp\", \"implementation\""));
    assert!(program.contains("\"ensure\", \"result >= value\", \"clamp\", \"implementation\""));
    assert!(runtime.contains("public static final class ContractFailure"));
}

#[test]
fn omits_statically_proven_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn constant() -> output: Int effects []\n",
        "  require true\n",
        "  invariant true\n",
        "  ensure not false\n",
        "  1\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"true\", \"constant\", \"caller\""));
    assert!(!program.contains("\"invariant\", \"true\", \"constant\", \"caller\""));
    assert!(!program.contains("\"ensure\", \"not false\", \"constant\", \"implementation\""));
    assert!(program.contains("return Long.valueOf(1L);"));
}

#[test]
fn omits_boolean_identity_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn identity(value: Int) -> output: Int effects []\n",
        "  require true or value > 0\n",
        "  ensure (output >= value or true) and not false\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"true or value > 0\""));
    assert!(!program.contains("\"ensure\", \"(output >= value or true) and not false\""));
    assert!(program.contains("return p_value;"));
}

#[test]
fn omits_negated_complementary_and_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool} effects []\n",
        "  require not (value.ready and not value.ready)\n",
        "  ensure not((output.ready) and not(output.ready))\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"not (value.ready and not value.ready)\""));
    assert!(!program.contains("\"ensure\", \"not((output.ready) and not(output.ready))\""));
    assert!(program.contains("return p_value;"));
}

#[test]
fn omits_multi_branch_complementary_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool} effects []\n",
        "  require value.ready or extra or not value.ready\n",
        "  ensure not((output.ready) and extra and not(output.ready))\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"value.ready or extra or not value.ready\""));
    assert!(
        !program.contains("\"ensure\", \"not((output.ready) and extra and not(output.ready))\"")
    );
    assert!(program.contains("return p_value;"));
}

#[test]
fn omits_statically_proven_literal_comparison_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn constant() -> output: Int effects []\n",
        "  require 1 < 2 and \"ready\" != \"pending\"\n",
        "  ensure true == true and 0.5 <= 1.50\n",
        "  1\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"1 < 2 and"));
    assert!(!program.contains("\"ensure\", \"true == true and"));
    assert!(program.contains("return Long.valueOf(1L);"));
}

#[test]
fn omits_statically_proven_same_shape_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn identity(value: Int, label: String) -> output: Int effects []\n",
        "  require value + 1 == value + 1\n",
        "  require not(value < value)\n",
        "  ensure label == label and output >= output\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"value + 1 == value + 1\""));
    assert!(!program.contains("\"require\", \"not(value < value)\""));
    assert!(!program.contains("\"ensure\", \"label == label and output >= output\""));
    assert!(program.contains("return p_value;"));
}

#[test]
fn omits_transitive_strict_order_cycle_contract_checks() {
    let ir = lower_to_ir(concat!(
        "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
        "  require not (low < mid and mid <= high and high <= low)\n",
        "  ensure not (output <= mid and mid < high and high <= output)\n",
        "  low\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\", \"not (low < mid"));
    assert!(!program.contains("\"ensure\", \"not (output <= mid"));
    assert!(program.contains("return p_low;"));
}

#[test]
fn omits_wide_partial_case_split_contract_checks() {
    let fields = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let record_type = bool_record_type(&fields);
    let predicate = partial_case_split_chain_predicate("value", &fields);
    let source = format!(
        "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}} effects []\n  require {predicate}\n  value\nend\n"
    );
    let ir = lower_to_ir(&source);

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(!program.contains("VelnRuntime.checkContract("));
    assert!(!program.contains("\"require\""));
    assert!(program.contains("return p_value;"));
}

#[test]
fn emits_ensure_checks_before_try_early_return() {
    let ir = lower_to_ir(concat!(
        "fn fail() -> Result(Int, String) effects []\n",
        "  Err(\"bad\")\n",
        "end\n",
        "pub fn main(flag: Bool) -> output: Result(Int, String) effects []\n",
        "  ensure flag and output == output\n",
        "  let value: Int = fail()?\n",
        "  Ok(value)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    let early_return = program
        .find("if (VelnRuntime.isErr(__try0))")
        .expect("try early return should be generated");
    let ensure = program[early_return..]
        .find("\"ensure\", \"flag and output == output\", \"main\", \"implementation\"")
        .expect("ensure check should be generated for the early return");
    let return_err = program[early_return..]
        .find("return __try0;")
        .expect("try error should still return");

    assert!(ensure < return_err);
    assert!(program[early_return..].contains("VelnRuntime.equal(__try0, __try0)"));
}

#[test]
fn emits_qualified_runtime_contract_calls() {
    let ir = lower_to_ir(concat!(
        "mod app.main\n",
        "use app.main\n",
        "fn positive(value: Int) -> Bool effects []\n",
        "  value > 0\n",
        "end\n",
        "pub fn main(value: Int) -> Int effects []\n",
        "  require main::positive(value)\n",
        "  value\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(program.contains("fn_positive("));
    assert!(program.contains("\"require\", \"main::positive(value)\", \"main\", \"caller\""));
}

#[test]
fn generated_entry_reports_contract_failures() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main(value: Int) -> Int effects []\n",
        "  require value > 0\n",
        "  value\n",
        "end\n",
    ));
    let java = generate_java_with_entry_arg_types(&ir, "main", &[EntryArgType::Int]);
    let root = temp_dir("contract-runtime");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .arg("0")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: require `value > 0`"));
    assert!(stderr.contains("blame caller"));
}

#[test]
fn generated_entry_reports_ensure_contract_failures() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "pub fn main(value: Int) -> output: Int effects []\n",
        "  ensure output > 0\n",
        "  value\n",
        "end\n",
    ));
    let java = generate_java_with_entry_arg_types(&ir, "main", &[EntryArgType::Int]);
    let root = temp_dir("ensure-contract-runtime");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let javac = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .arg("VelnEntry.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .arg("0")
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: ensure `output > 0`"));
    assert!(stderr.contains("blame implementation"));
}

#[test]
fn generated_sources_compile_when_javac_is_available() {
    if Command::new("javac").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir(concat!(
        "fn parse(raw: String) -> Result(Int, AppError) effects []\n",
        "  Ok(1)\n",
        "end\n",
        "pub fn main(raw: String) -> Result((), AppError) effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  stdio::println(\"ok\")\n",
        "  Ok(())\n",
        "end\n",
    ));
    let java = generate_java(&ir);
    let root = temp_dir("javac");
    for source in &java.sources {
        fs::write(root.join(&source.path), &source.contents)
            .expect("java source should be written");
    }

    let output = Command::new("javac")
        .arg("VelnProgram.java")
        .arg("VelnRuntime.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn bytecode_backend_emits_classfiles_without_java_sources() {
    let ir = lower_to_ir("pub fn main() -> () effects []\n  ()\nend\n");

    let program = generate_classfiles_with_entry(&ir, "main");

    assert!(program.class("VelnEntry.class").is_some());
    assert!(program.class("VelnProgram.class").is_some());
    assert!(program.class("VelnRuntime.class").is_some());
    assert!(
        program
            .classes
            .iter()
            .all(|class| class.path.ends_with(".class"))
    );
}

#[test]
fn bytecode_backend_classfiles_run_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> () effects [stdio]\n  stdio::println(\"ok\")\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("bytecode-run");
    write_jvm_program(&root, &program);

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn bytecode_backend_javap_reports_target_version_and_entry_descriptor_when_available() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir("pub fn main(value: String) -> String effects []\n  value\nend\n");
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

#[test]
fn java_identifier_helpers_sanitize_keywords_and_collisions() {
    let mut used_names = std::collections::BTreeSet::new();

    assert_eq!(sanitize_identifier_text("1-value!"), "_1_value_");
    assert_eq!(java_type_identifier("class"), "VelnGenerated");
    assert_eq!(java_type_identifier("app.Main"), "app_Main");
    assert_eq!(unique_java_identifier("", &mut used_names), "_value");
    assert_eq!(unique_java_identifier("return", &mut used_names), "_return");
    assert_eq!(unique_java_identifier("value", &mut used_names), "value");
    assert_eq!(unique_java_identifier("value", &mut used_names), "value_1");
}

#[test]
fn java_method_name_helpers_map_builtin_surface_names() {
    for (surface, method) in [
        ("stdio::print", "stdioPrint"),
        ("stdio::println", "stdioPrintln"),
        ("stdio::eprint", "stdioEprint"),
        ("stdio::eprintln", "stdioEprintln"),
    ] {
        assert_eq!(stdio_method(surface), method);
    }

    for (surface, method) in [
        ("float_negate", "floatNegate"),
        ("float_add", "floatAdd"),
        ("float_subtract", "floatSubtract"),
        ("float_multiply", "floatMultiply"),
        ("float_divide", "floatDivide"),
        ("float_less", "floatLess"),
        ("float_less_equal", "floatLessEqual"),
        ("float_greater", "floatGreater"),
        ("float_greater_equal", "floatGreaterEqual"),
        ("string_split_once", "stringSplitOnce"),
        ("string_parse_int", "stringParseInt"),
        ("int_to_string", "intToString"),
        ("vec_len", "vecLen"),
        ("vec_is_empty", "vecIsEmpty"),
        ("vec_push", "vecPush"),
        ("vec_concat", "vecConcat"),
        ("vec_map", "vecMap"),
        ("vec_filter", "vecFilter"),
        ("vec_fold", "vecFold"),
        ("vec_try_map", "vecTryMap"),
        ("vec_try_map_with", "vecTryMapWith"),
        ("dict_get", "dictGet"),
        ("dict_contains", "dictContains"),
        ("dict_insert", "dictInsert"),
        ("dict_remove", "dictRemove"),
        ("option_map", "optionMap"),
        ("option_and_then", "optionAndThen"),
        ("option_unwrap_or", "optionUnwrapOr"),
        ("result_map", "resultMap"),
        ("result_map_err", "resultMapErr"),
        ("result_and_then", "resultAndThen"),
    ] {
        assert_eq!(prelude_method(surface), method);
    }

    for (surface, method) in [
        ("channel::bounded", "channelBounded"),
        ("channel::clone", "channelClone"),
        ("channel::send", "channelSend"),
        ("channel::recv", "channelRecv"),
        ("channel::select", "channelSelect"),
        ("channel::select_priority", "channelSelectPriority"),
        ("channel::select_timeout", "channelSelectTimeout"),
        ("channel::select_result", "channelSelectResult"),
        (
            "channel::select_priority_result",
            "channelSelectPriorityResult",
        ),
        (
            "channel::select_timeout_result",
            "channelSelectTimeoutResult",
        ),
        ("channel::close", "channelClose"),
        ("task::spawn", "taskSpawn"),
        ("task::join", "taskJoin"),
        ("task::cancel", "taskCancel"),
    ] {
        assert_eq!(concurrency_method(surface), method);
    }

    for (surface, method) in [
        ("fs::read_to_string", "fsReadToString"),
        ("fs::write_string", "fsWriteString"),
        ("fs::exists", "fsExists"),
        ("fs::read_dir", "fsReadDir"),
        ("process::args", "processArgs"),
        ("process::env", "processEnv"),
        ("process::cwd", "processCwd"),
        ("process::exit", "processExit"),
    ] {
        assert_eq!(standard_library_method(surface), method);
    }

    let panic = std::panic::catch_unwind(|| standard_library_method("fs::unknown"));
    assert!(panic.is_err());
}

#[test]
fn java_string_escapes_special_characters_and_control_codes() {
    assert_eq!(
        java_string("quote \" slash \\ newline\n tab\t nul\0"),
        "\"quote \\\" slash \\\\ newline\\n tab\\t nul\\u0000\""
    );
}

#[test]
fn veln_string_literal_value_decodes_known_escapes_and_preserves_unknown_ones() {
    assert_eq!(
        veln_string_literal_value("\"line\\nquote\\\"slash\\\\tab\\t\""),
        "line\nquote\"slash\\tab\t"
    );
    assert_eq!(veln_string_literal_value("\"unknown\\q\""), "unknown\\q");
    assert_eq!(veln_string_literal_value("\"trailing\\\""), "trailing\\");
    assert_eq!(veln_string_literal_value("raw"), "raw");
}

fn lower_to_ir(text: &str) -> TypedProgram {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse diagnostics: {:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.is_empty(),
        "semantic diagnostics: {:#?}",
        lowered.diagnostics
    );
    lowered.ir.expect("source should lower to typed IR")
}

fn bool_record_type(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("{field}: Bool"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn partial_case_split_chain_predicate(subject: &str, fields: &[&str]) -> String {
    let mut disjuncts = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let mut conjuncts = fields[..index]
            .iter()
            .map(|field| format!("not {subject}.{field}"))
            .collect::<Vec<_>>();
        conjuncts.push(format!("{subject}.{field}"));
        disjuncts.push(format!("({})", conjuncts.join(" and ")));
    }
    disjuncts.push(format!(
        "({})",
        fields
            .iter()
            .map(|field| format!("not {subject}.{field}"))
            .collect::<Vec<_>>()
            .join(" and ")
    ));
    disjuncts.join(" or ")
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-backend-jvm-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test directory should be created");
    root
}

fn write_jvm_program(root: &std::path::Path, program: &JvmProgram) {
    for class in &program.classes {
        fs::write(root.join(&class.path), &class.contents).expect("classfile should be written");
    }
}
