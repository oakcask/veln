use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

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

    let java = generate_java(&ir);
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
        "pub fn main() -> Result({message: String, values: List(String), maybe: Option(String), empty: Option(String)}, AppError) effects []\n",
        "  Ok({message: \"ok\", values: [\"a\", \"b\"], maybe: Some(\"x\"), empty: None})\n",
        "end\n",
    ));

    let java = generate_java(&ir);
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
        "pub fn main(items: List(Int), table: Dict(String, Int)) -> Result({pushed: List(Int), inserted: Dict(String, Int)}, AppError) effects []\n",
        "  Ok({pushed: list_push(items, 1), inserted: dict_insert(table, \"one\", 1)})\n",
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
        "pub fn main() -> Result(List(List(String)), AppError) effects []\n",
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
        "pub fn main(items: List(Int), table: Dict(String, Int), mapper: fn(Int) -> String) -> {",
        "count: Int, pushed: List(Int), mapped: List(String), found: Option(Int), inserted: Dict(String, Int)",
        "} effects []\n",
        "  {count: list_len(items), pushed: list_push(items, 1), mapped: list_map(items, mapper), ",
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

    assert!(program.contains("\"count\", VelnRuntime.listLen(p_items)"));
    assert!(program.contains("\"pushed\", VelnRuntime.listPush(p_items, Long.valueOf(1L))"));
    assert!(program.contains("\"mapped\", VelnRuntime.listMap(p_items, p_mapper)"));
    assert!(program.contains("\"found\", VelnRuntime.dictGet(p_table, \"a\")"));
    assert!(
        program.contains("\"inserted\", VelnRuntime.dictInsert(p_table, \"b\", Long.valueOf(2L))")
    );
    assert!(runtime.contains("public static Object listTryMap"));
    assert!(runtime.contains("public static Object resultAndThen"));
}

#[test]
fn generates_function_values_for_declared_functions() {
    let ir = lower_to_ir(concat!(
        "fn stringify(value: Int) -> String effects []\n",
        "  \"ok\"\n",
        "end\n",
        "pub fn main(items: List(Int)) -> List(String) effects []\n",
        "  list_map(items, stringify)\n",
        "end\n",
    ));

    let java = generate_java(&ir);
    let program = java
        .source("VelnProgram.java")
        .expect("program source should exist");

    assert!(
        program.contains(
            "VelnRuntime.listMap(p_items, (VelnRuntime.Fn) ((Object... fnArgs) -> fn_stringify(fnArgs[0])))"
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
    assert!(program.contains("\"ensure\", \"result >= value\", \"clamp\", \"implementation\""));
    assert!(runtime.contains("public static final class ContractFailure"));
}

#[test]
fn emits_ensure_checks_before_try_early_return() {
    let ir = lower_to_ir(concat!(
        "fn fail() -> Result(Int, String) effects []\n",
        "  Err(\"bad\")\n",
        "end\n",
        "pub fn main() -> output: Result(Int, String) effects []\n",
        "  ensure output == output\n",
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
        .find("\"ensure\", \"output == output\", \"main\", \"implementation\"")
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
