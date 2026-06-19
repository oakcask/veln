use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::classfile::{TailRecursionEligibility, classify_tail_recursion};
use crate::java::{
    java_type_identifier, sanitize_identifier_text, unique_java_identifier,
    veln_string_literal_value,
};
use crate::runtime::{concurrency_method, prelude_method, standard_library_method, stdio_method};
use crate::*;
use veln_ast::lower_surface_ast_with_module_identity;
use veln_ir::TypedProgram;
use veln_sema::lower_checked_surface_module;
use veln_source::{SourceFile, TextRange};
use veln_syntax::parse;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

const RUNTIME_LIST_HARNESS: &str = r#"
public final class RuntimeListHarness {
    private static int foldCalls = 0;
    private static int tryCalls = 0;

    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        Object reversed = VelnRuntime.listReverse(values);
        Object total = VelnRuntime.listFold(reversed, Long.valueOf(0), new VelnRuntime.Fn() {
            public Object call(Object... args) {
                foldCalls += 1;
                return Long.valueOf(((Long) args[0]).longValue() + ((Long) args[1]).longValue());
            }
        });
        Object kept = VelnRuntime.listFilter(reversed, new VelnRuntime.Fn() {
            public Object call(Object... args) {
                return Boolean.TRUE;
            }
        });
        Object tried = VelnRuntime.listTryMap(
            VelnRuntime.listCons(
                Long.valueOf(1),
                VelnRuntime.listCons(Long.valueOf(2), VelnRuntime.listCons(Long.valueOf(3), VelnRuntime.listNil()))
            ),
            new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    tryCalls += 1;
                    if (((Long) args[0]).longValue() == 2L) {
                        return VelnRuntime.err("stop");
                    }
                    return VelnRuntime.ok(args[0]);
                }
            }
        );
        System.out.println(
            total
                + ":"
                + foldCalls
                + ":"
                + VelnRuntime.listIsEmpty(VelnRuntime.listNil())
                + ":"
                + VelnRuntime.listIsEmpty(kept)
                + ":"
                + tryCalls
                + ":"
                + tried
        );
    }
}
"#;

const RUNTIME_BYTE_HEX_HARNESS: &str = r#"
public final class RuntimeByteHexHarness {
    public static void main(String[] args) {
        System.out.println(VelnRuntime.byteChunkFromHex("0001ff"));
        System.out.println(VelnRuntime.byteChunkFromHex("00 01\nff\t10"));
        System.out.println(VelnRuntime.byteChunkFromHex("0x00"));
        System.out.println(VelnRuntime.byteChunkFromHex("00_1"));
        System.out.println(VelnRuntime.byteChunkFromHex("0 0"));
        System.out.println(VelnRuntime.byteChunkFromHex("00#comment"));
        System.out.println(VelnRuntime.byteChunkFromHex("00:01"));
        System.out.println(VelnRuntime.byteChunkFromHex("001"));
    }
}
"#;

const RUNTIME_BYTE_VIEW_HARNESS: &str = r#"
public final class RuntimeByteViewHarness {
    public static void main(String[] args) {
        Object chunk = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00010203ff")).value();
        Object view = ((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(1))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(3))).value()
        )).value();
        Object wideView = ((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(1))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        Object two = ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value();
        Object viewPrefix = ((VelnRuntime.Result) VelnRuntime.byteViewTake(view, two)).value();
        Object viewSuffix = ((VelnRuntime.Result) VelnRuntime.byteViewDrop(view, two)).value();
        Object viewSlice = ((VelnRuntime.Result) VelnRuntime.byteViewSlice(
            wideView,
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(1))).value(),
            two
        )).value();
        Object outputChunks = VelnRuntime.byteChunksAppend(
            VelnRuntime.byteChunksOne(VelnRuntime.byteViewToChunk(viewPrefix)),
            VelnRuntime.byteChunksOne(VelnRuntime.byteViewToChunk(viewSuffix))
        );
        System.out.println(VelnRuntime.byteReadU8Be(view));
        System.out.println(VelnRuntime.byteReadU16Be(view));
        System.out.println(VelnRuntime.byteReadU24Be(view));
        System.out.println(VelnRuntime.byteReadU16Le(view));
        System.out.println(VelnRuntime.byteReadU24Le(view));
        System.out.println(VelnRuntime.byteViewCount(view));
        System.out.println(VelnRuntime.byteReadU16Be(viewPrefix));
        System.out.println(VelnRuntime.byteReadU8Be(viewSuffix));
        System.out.println(VelnRuntime.byteReadU16Be(viewSlice));
        System.out.println(outputChunks);
        System.out.println(VelnRuntime.byteViewDrop(view, ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()));
        System.out.println(VelnRuntime.byteReadU31Be(wideView));
        Object maxU31 = ((VelnRuntime.Result) VelnRuntime.byteWriteU31Be(Long.valueOf(2147483647))).value();
        Object maxU31View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU31,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU31Be(maxU31View));
        Object maxU32 = ((VelnRuntime.Result) VelnRuntime.byteWriteU32Be(Long.valueOf(4294967295L))).value();
        Object maxU32View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU32,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU32Be(maxU32View));
        Object maxU40 = ((VelnRuntime.Result) VelnRuntime.byteWriteU40Be(Long.valueOf(1099511627775L))).value();
        Object maxU40View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU40,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(5))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU40Be(maxU40View));
        Object maxU48 = ((VelnRuntime.Result) VelnRuntime.byteWriteU48Be(Long.valueOf(281474976710655L))).value();
        Object maxU48View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU48,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU48Be(maxU48View));
        Object maxU31Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU31Le(Long.valueOf(2147483647))).value();
        Object maxU31LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU31Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU31Le(maxU31LeView));
        Object maxU32Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU32Le(Long.valueOf(4294967295L))).value();
        Object maxU32LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU32Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU32Le(maxU32LeView));
        Object maxU40Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU40Le(Long.valueOf(1099511627775L))).value();
        Object maxU40LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU40Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(5))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU40Le(maxU40LeView));
        Object maxU48Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU48Le(Long.valueOf(281474976710655L))).value();
        Object maxU48LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU48Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU48Le(maxU48LeView));
        Object maxU64 = ((VelnRuntime.Result) VelnRuntime.byteWriteU64Be(Long.MAX_VALUE)).value();
        Object maxU64View = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU64,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Be(maxU64View));
        Object maxU64Le = ((VelnRuntime.Result) VelnRuntime.byteWriteU64Le(Long.MAX_VALUE)).value();
        Object maxU64LeView = ((VelnRuntime.Result) VelnRuntime.byteView(
            maxU64Le,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Le(maxU64LeView));
        System.out.println(VelnRuntime.byteWriteU16Le(Long.valueOf(4660)));
        System.out.println(VelnRuntime.byteWriteU24Le(Long.valueOf(66051)));
        System.out.println(VelnRuntime.byteWriteU32Le(Long.valueOf(16909060)));
        System.out.println(VelnRuntime.byteReadU24Be(((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        )).value()));
        System.out.println(VelnRuntime.byteReadU24Le(((VelnRuntime.Result) VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        )).value()));
        System.out.println(VelnRuntime.byteView(
            chunk,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(4))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(2))).value()
        ));
        System.out.println(VelnRuntime.byteWriteU8Be(Long.valueOf(256)));
        System.out.println(VelnRuntime.byteWriteU32Le(Long.valueOf(4294967296L)));
        System.out.println(VelnRuntime.byteReadU31Be(maxU32View));
        System.out.println(VelnRuntime.byteReadU31Le(maxU32LeView));
        Object overflowU64View = ((VelnRuntime.Result) VelnRuntime.byteView(
            ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("ffffffffffffffff")).value(),
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(8))).value()
        )).value();
        System.out.println(VelnRuntime.byteReadU64Be(overflowU64View));
        System.out.println(VelnRuntime.byteReadU64Le(overflowU64View));
        Object frame = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("000005010400000001")).value();
        Object frameView = ((VelnRuntime.Result) VelnRuntime.byteView(
            frame,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(9))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeHttp2FrameHeader(frameView));
        Object widthSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("1234deadbeef")).value();
        Object widthSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            widthSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(6))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaWidthSample(widthSampleView));
        Object validationSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00000504")).value();
        Object validationSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            validationSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaValidationSample(validationSampleView));
        Object invalidValidationSample = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("00000506")).value();
        Object invalidValidationSampleView = ((VelnRuntime.Result) VelnRuntime.byteView(
            invalidValidationSample,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(4))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeSchemaValidationSample(invalidValidationSampleView));
        Object reservedFrame = ((VelnRuntime.Result) VelnRuntime.byteChunkFromHex("000005010480000001")).value();
        Object reservedView = ((VelnRuntime.Result) VelnRuntime.byteView(
            reservedFrame,
            ((VelnRuntime.Result) VelnRuntime.byteOffset(Long.valueOf(0))).value(),
            ((VelnRuntime.Result) VelnRuntime.byteCount(Long.valueOf(9))).value()
        )).value();
        System.out.println(VelnRuntime.byteDecodeHttp2FrameHeader(reservedView));
    }
}
"#;

const PUBLIC_LIST_HELPER_HARNESS: &str = r#"
public final class PublicListHelperHarness {
    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        VelnProgram.fn_consume(values);
    }
}
"#;

const RUNTIME_PATH_HARNESS: &str = r#"
public final class RuntimePathHarness {
    public static void main(String[] args) {
        Object cwd = ((VelnRuntime.Result) VelnRuntime.processCwd()).value();
        System.out.println(VelnRuntime.fsExists(cwd));

        Object entries = ((VelnRuntime.Result) VelnRuntime.fsReadDir(cwd)).value();
        Object first = ((java.util.List<?>) entries).get(0);
        System.out.println(VelnRuntime.fsExists(first));
    }
}
"#;

const RUNTIME_CHANNEL_SELECT_MANY_TIMEOUT_RESULT_HARNESS: &str = r#"
public final class RuntimeChannelSelectManyTimeoutResultHarness {
    public static void main(String[] args) {
        Object first = VelnRuntime.channelBounded(Long.valueOf(1));
        Object second = VelnRuntime.channelBounded(Long.valueOf(1));
        Object third = VelnRuntime.channelBounded(Long.valueOf(1));
        VelnRuntime.channelSend(VelnRuntime.recordField(second, "tx"), Long.valueOf(21));
        VelnRuntime.channelSend(VelnRuntime.recordField(third, "tx"), Long.valueOf(34));
        Object readyReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(first, "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(second, "rx"),
                VelnRuntime.listCons(VelnRuntime.recordField(third, "rx"), VelnRuntime.listNil())
            )
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(readyReceivers, Long.valueOf(10)));

        Object timeoutFirst = VelnRuntime.channelBounded(Long.valueOf(1));
        Object timeoutSecond = VelnRuntime.channelBounded(Long.valueOf(1));
        Object timeoutReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(timeoutFirst, "rx"),
            VelnRuntime.listCons(VelnRuntime.recordField(timeoutSecond, "rx"), VelnRuntime.listNil())
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(timeoutReceivers, Long.valueOf(0)));

        Object interruptFirst = VelnRuntime.channelBounded(Long.valueOf(1));
        Object interruptSecond = VelnRuntime.channelBounded(Long.valueOf(1));
        Object interruptReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(interruptFirst, "rx"),
            VelnRuntime.listCons(VelnRuntime.recordField(interruptSecond, "rx"), VelnRuntime.listNil())
        );
        Thread.currentThread().interrupt();
        System.out.println(VelnRuntime.channelSelectManyTimeoutResult(interruptReceivers, Long.valueOf(10000)));
        Thread.interrupted();

        Object cancelledToken = VelnRuntime.timeCancelToken();
        VelnRuntime.timeCancel(cancelledToken);
        Object cancelledReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
                VelnRuntime.listNil()
            )
        );
        System.out.println(VelnRuntime.channelSelectManyTimeoutCancellable(
            cancelledReceivers,
            Long.valueOf(10000),
            cancelledToken
        ));

        final Object waitToken = VelnRuntime.timeCancelToken();
        Thread canceller = new Thread(new Runnable() {
            public void run() {
                try {
                    Thread.sleep(10L);
                } catch (InterruptedException error) {
                    Thread.currentThread().interrupt();
                }
                VelnRuntime.timeCancel(waitToken);
            }
        });
        Object waitingReceivers = VelnRuntime.listCons(
            VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
            VelnRuntime.listCons(
                VelnRuntime.recordField(VelnRuntime.channelBounded(Long.valueOf(1)), "rx"),
                VelnRuntime.listNil()
            )
        );
        canceller.start();
        System.out.println(VelnRuntime.channelSelectManyTimeoutCancellable(
            waitingReceivers,
            Long.valueOf(10000),
            waitToken
        ));
    }
}
"#;

#[test]
fn bytecode_backend_emits_classfiles_without_java_sources() {
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");

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
fn bytecode_backend_sanitizes_custom_program_class_name() {
    let ir = lower_to_ir("pub fn main() -> String\n  \"ok\"\nend\n");
    let program = generate_classfiles_with_entry_arg_types_options(
        &ir,
        "main",
        &[],
        &JvmBackendOptions {
            program_class: "9 bad-name".to_string(),
        },
    );

    assert!(program.class("_9_bad_name.class").is_some());
    assert!(program.class("_9_bad_name$fn_main.class").is_some());
    assert!(program.class("VelnEntry.class").is_some());
}

#[test]
fn bytecode_backend_classfiles_run_when_java_is_available() {
    let ir = lower_to_ir("pub fn main() -> () effects [stdio]\n  stdio::println(\"ok\")\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-run", &program, &[]) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn bytecode_backend_reports_forced_timeout_expiry_when_java_is_available() {
    let ir = lower_to_ir("pub fn main() -> () effects [time]\n  time::timeout_ms(5)\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-timeout-expiry",
        &program,
        &[("VELN_TIME_TIMEOUT_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport timeout expired: VELN_TIME_TIMEOUT_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_deadline_expiry_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  time::wait_until(deadline)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-deadline-expiry",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport deadline expired: VELN_TIME_DEADLINE_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_waits_until_deadline_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  time::wait_until(deadline)\n",
        "  stdio::println(\"deadline\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-deadline", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "deadline\n");
}

#[test]
fn bytecode_backend_waits_until_cancellable_deadline_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "  stdio::println(\"cancellable\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancellable-deadline", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancellable\n");
}

#[test]
fn bytecode_backend_returns_cancellable_wait_outcomes_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    WaitCompleted => \"completed\"\n",
        "    WaitDeadlineExpired => \"deadline\"\n",
        "    WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let completed_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let completed_token: CancelToken = time::cancel_token()\n",
        "  let completed: CancellableWaitOutcome = time::wait_until_cancellable_outcome(completed_deadline, completed_token)\n",
        "  stdio::println(outcome_text(completed))\n",
        "  let cancelled_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let cancelled_token: CancelToken = time::cancel_token()\n",
        "  time::cancel(cancelled_token)\n",
        "  let cancelled: CancellableWaitOutcome = time::wait_until_cancellable_outcome(cancelled_deadline, cancelled_token)\n",
        "  stdio::println(outcome_text(cancelled))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancellable-wait-outcomes", &program, &[])
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
        "completed\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_observes_cancel_token_status_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn status_text(cancelled: Bool) -> String\n",
        "  match cancelled\n",
        "    true => \"cancelled\"\n",
        "    false => \"active\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  stdio::println(status_text(time::is_cancelled(token)))\n",
        "  time::cancel(token)\n",
        "  stdio::println(status_text(time::is_cancelled(token)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-cancel-token-status", &program, &[])
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
        "active\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_returns_forced_cancellable_wait_expiry_outcome_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    WaitCompleted => \"completed\"\n",
        "    WaitDeadlineExpired => \"deadline\"\n",
        "    WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  let outcome: CancellableWaitOutcome = time::wait_until_cancellable_outcome(deadline, token)\n",
        "  stdio::println(outcome_text(outcome))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-expiry-outcome",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "deadline\n");
}

#[test]
fn bytecode_backend_returns_forced_cancellable_wait_outcome_sequence_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn outcome_text(outcome: CancellableWaitOutcome) -> String\n",
        "  match outcome\n",
        "    WaitCompleted => \"completed\"\n",
        "    WaitDeadlineExpired => \"deadline\"\n",
        "    WaitCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [time, stdio]\n",
        "  let first_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let first_token: CancelToken = time::cancel_token()\n",
        "  let first: CancellableWaitOutcome = time::wait_until_cancellable_outcome(first_deadline, first_token)\n",
        "  stdio::println(outcome_text(first))\n",
        "  let second_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let second_token: CancelToken = time::cancel_token()\n",
        "  let second: CancellableWaitOutcome = time::wait_until_cancellable_outcome(second_deadline, second_token)\n",
        "  stdio::println(outcome_text(second))\n",
        "  let third_deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let third_token: CancelToken = time::cancel_token()\n",
        "  let third: CancellableWaitOutcome = time::wait_until_cancellable_outcome(third_deadline, third_token)\n",
        "  stdio::println(outcome_text(third))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-outcome-sequence",
        &program,
        &[(
            "VELN_TIME_CANCELLABLE_OUTCOMES",
            "completed,deadline-expired,cancelled",
        )],
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "completed\ndeadline\ncancelled\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_cancellable_wait_expiry_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-deadline-expiry",
        &program,
        &[("VELN_TIME_DEADLINE_EXPIRED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport deadline expired: VELN_TIME_DEADLINE_EXPIRED\n"
    );
}

#[test]
fn bytecode_backend_reports_forced_cancellable_wait_cancellation_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(5)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_with_env_when_java_is_available(
        "bytecode-cancellable-wait-cancelled",
        &program,
        &[("VELN_TIME_WAIT_CANCELLED", "1")],
        &[],
    ) else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport wait cancelled: VELN_TIME_WAIT_CANCELLED\n"
    );
}

#[test]
fn bytecode_backend_reports_source_cancelled_wait_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [time]\n",
        "  let deadline: Deadline = time::deadline_after_ms(0)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::cancel(token)\n",
        "  time::wait_until_cancellable(deadline, token)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-source-cancelled-wait", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "transport wait cancelled: cancellation token\n"
    );
}

#[test]
fn bytecode_backend_runs_result_try_collections_and_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn parse(raw: String) -> Result<Int, {message: String}>\n",
        "  Ok(1)\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  \"ok\"\n",
        "end\n",
        "pub fn main(raw: String) -> Result<(), {message: String}> effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  let mapped: Vec<String> = vec_map([value], stringify)\n",
        "  let message: String = match dict_get({\"first\": \"bad\", \"second\": \"ok\"}, \"second\")\n",
        "    Some(found) => found\n",
        "    None => \"missing\"\n",
        "  end\n",
        "  stdio::println(message)\n",
        "  match vec_len(mapped) == 1\n",
        "    true => Ok(())\n",
        "    false => Err({message: \"bad\"})\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-result-collections", &program, &["raw"])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn bytecode_backend_runs_minimal_list_adt_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn sum(values: List<Int>) -> Int\n",
        "  match values\n",
        "    Nil => 0\n",
        "    Cons(head, tail) => head + sum(tail)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(sum(Cons(1, Cons(2, Nil)))))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-list-adt", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\n");
}

#[test]
fn bytecode_backend_runs_vec_try_map_with_context_and_error_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn attach(context: String, value: Int) -> Result<{prefix: String, value: Int}, String>\n",
        "  Ok({prefix: context, value: value})\n",
        "end\n",
        "fn stop_at_two(context: String, value: Int) -> Result<{prefix: String, value: Int}, String>\n",
        "  match value == 2\n",
        "    true => Err(context)\n",
        "    false => match value == 3\n",
        "      true => Err(\"later\")\n",
        "      false => Ok({prefix: context, value: value})\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn add_value(total: Int, item: {prefix: String, value: Int}) -> Int\n",
        "  total + item.value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let mapped: Result<Vec<{prefix: String, value: Int}>, String> = vec_try_map_with(\"ctx\", [1, 2], attach)\n",
        "  let stopped: Result<Vec<{prefix: String, value: Int}>, String> = vec_try_map_with(\"ctx\", [1, 2, 3], stop_at_two)\n",
        "  match mapped\n",
        "    Ok(items) => stdio::println(int_to_string(vec_fold(items, 0, add_value)))\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "  match stopped\n",
        "    Ok(_) => stdio::println(\"unexpected\")\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-vec-try-map-with", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\nctx\n");
}

#[test]
fn bytecode_backend_runs_list_helpers_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn add(total: Int, value: Int) -> Int\n",
        "  total + value\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  int_to_string(value)\n",
        "end\n",
        "fn keep_large(value: Int) -> Bool\n",
        "  value > 1\n",
        "end\n",
        "fn stop_at_two(value: Int) -> Result<String, String>\n",
        "  match value == 2\n",
        "    true => Err(\"stop\")\n",
        "    false => match value == 3\n",
        "      true => Err(\"later\")\n",
        "      false => Ok(int_to_string(value))\n",
        "    end\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let values: List<Int> = list_cons(1, list_cons(2, list_cons(3, list_nil())))\n",
        "  stdio::println(int_to_string(list_fold(values, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_reverse(values), 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_filter(values, keep_large), 0, add)))\n",
        "  stdio::println(match list_try_map(values, stop_at_two)\n",
        "    Ok(_) => \"unexpected\"\n",
        "    Err(error) => error\n",
        "  end)\n",
        "  stdio::println(match list_map(values, stringify)\n",
        "    Nil => \"empty\"\n",
        "    Cons(head, _) => head\n",
        "  end)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-list-helpers", &program, &[])
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
        "6\n6\n5\nstop\n1\n"
    );
}

#[test]
fn bytecode_backend_runs_deep_tail_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn countdown(value: Int) -> Int\n",
        "require value >= 0\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "fn nested_countdown(value: Int, active: Bool) -> Int\n",
        "  match active\n",
        "    true => match value\n",
        "      0 => 0\n",
        "      _ => nested_countdown(value - 1, true)\n",
        "    end\n",
        "    false => 0\n",
        "  end\n",
        "end\n",
        "fn pair_step(first: Int, second: Int, steps: Int) -> Int\n",
        "  match steps\n",
        "    0 => first\n",
        "    _ => pair_step(second, first + second, steps - 1)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(countdown(30000)))\n",
        "  stdio::println(int_to_string(nested_countdown(30000, true)))\n",
        "  stdio::println(int_to_string(pair_step(0, 1, 10)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "0\n0\n55\n");
}

#[test]
fn bytecode_backend_rechecks_require_contracts_inside_tail_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn countdown(value: Int) -> Int\n",
        "require value != 2\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  countdown(4)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion-require", &program, &[])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: require `value != 2`"));
    assert!(stderr.contains("blame caller"));
}

#[test]
fn bytecode_backend_verifies_all_tail_match_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn reject(value: Int) -> Int\n",
        "require false\n",
        "  match value\n",
        "    _ => reject(value)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  reject(0)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion-all-tail", &program, &[])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: require `false`"));
}

#[test]
fn bytecode_backend_classifies_tail_recursion_conservatively() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn countdown(value: Int) -> Int\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "fn length(items: List<Int>) -> Int\n",
        "  match items\n",
        "    Nil => 0\n",
        "    Cons(_, tail) => 1 + length(tail)\n",
        "  end\n",
        "end\n",
        "fn checked(value: Int) -> result: Int\n",
        "ensure result >= 0\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => checked(value - 1)\n",
        "  end\n",
        "end\n",
        "fn through_value(callback: fn(Int) -> Int, value: Int) -> Int\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => through_value(callback, callback(value - 1))\n",
        "  end\n",
        "end\n",
    ));

    let function = |name: &str| {
        ir.functions
            .iter()
            .find(|function| function.name == name)
            .expect("function should exist")
    };

    assert_eq!(
        classify_tail_recursion(function("countdown")),
        TailRecursionEligibility::Eligible
    );
    assert_eq!(
        classify_tail_recursion(function("length")),
        TailRecursionEligibility::NonTailSelfCall
    );
    assert_eq!(
        classify_tail_recursion(function("checked")),
        TailRecursionEligibility::RuntimeReturnContract
    );
    assert_eq!(
        classify_tail_recursion(function("through_value")),
        TailRecursionEligibility::IndirectValueCall
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
fn jvm_runtime_decodes_compact_hex_fixtures_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-byte-hex");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeByteHexHarness.java"),
        RUNTIME_BYTE_HEX_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeByteHexHarness.java")
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
        .arg("RuntimeByteHexHarness")
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
            "Ok(ByteChunk([Byte(0), Byte(1), Byte(255)]))\n",
            "Ok(ByteChunk([Byte(0), Byte(1), Byte(255), Byte(16)]))\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 0 low nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 0 low nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.odd_length: dangling hex nibble at byte offset 1 high nibble)\n",
        )
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
fn bytecode_backend_runs_two_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, right: Int) -> {left: String, right: Int} effects [concurrency]\n",
        "  { left: left, right: right }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with2(combine, \"hello\", 42)\n",
        "  let value: {left: String, right: Int} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.right))\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg2", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n42\n");
}

#[test]
fn bytecode_backend_runs_three_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String) -> {left: String, count: Int, marker: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with3(combine, \"hello\", 42, \"done\")\n",
        "  let value: {left: String, count: Int, marker: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg3", &program, &[])
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
fn bytecode_backend_runs_four_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String) -> {left: String, count: Int, marker: String, suffix: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with4(combine, \"hello\", 42, \"done\", \"extra\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg4", &program, &[])
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
        "hello\n42\ndone\nextra\n"
    );
}

#[test]
fn bytecode_backend_runs_five_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with5(combine, \"hello\", 42, \"done\", \"extra\", \"tail\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg5", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\n"
    );
}

#[test]
fn bytecode_backend_runs_six_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with6(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg6", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\n"
    );
}

#[test]
fn bytecode_backend_runs_seven_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with7(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg7", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\n"
    );
}

#[test]
fn bytecode_backend_runs_eight_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with8(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg8", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\n"
    );
}

#[test]
fn bytecode_backend_runs_nine_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with9(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg9", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\n"
    );
}

#[test]
fn bytecode_backend_runs_ten_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with10(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg10", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\n"
    );
}

#[test]
fn bytecode_backend_runs_eleven_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with11(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg11", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\n"
    );
}

#[test]
fn bytecode_backend_runs_twelve_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with12(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg12", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\n"
    );
}

#[test]
fn bytecode_backend_runs_thirteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with13(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg13", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\n"
    );
}

#[test]
fn bytecode_backend_runs_fourteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with14(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg14", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\n"
    );
}

#[test]
fn bytecode_backend_runs_fifteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with15(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg15", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\n"
    );
}

#[test]
fn bytecode_backend_runs_sixteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin, slot: slot }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with16(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\", \"slot\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  stdio::println(value.slot)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg16", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\nslot\n"
    );
}

#[test]
fn bytecode_backend_runs_seventeen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin, slot: slot, lane: lane }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with17(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\", \"slot\", \"lane\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  stdio::println(value.slot)\n",
        "  stdio::println(value.lane)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg17", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\nslot\nlane\n"
    );
}

#[test]
fn bytecode_backend_runs_eighteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin, slot: slot, lane: lane, row: row }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with18(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\", \"slot\", \"lane\", \"row\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  stdio::println(value.slot)\n",
        "  stdio::println(value.lane)\n",
        "  stdio::println(value.row)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg18", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\nslot\nlane\nrow\n"
    );
}

#[test]
fn bytecode_backend_runs_nineteen_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin, slot: slot, lane: lane, row: row, section: section }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with19(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\", \"slot\", \"lane\", \"row\", \"section\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  stdio::println(value.slot)\n",
        "  stdio::println(value.lane)\n",
        "  stdio::println(value.row)\n",
        "  stdio::println(value.section)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg19", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\nslot\nlane\nrow\nsection\n"
    );
}

#[test]
fn bytecode_backend_runs_twenty_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn combine(left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String, floor: String) -> {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String, floor: String} effects [concurrency]\n",
        "  { left: left, count: count, marker: marker, suffix: suffix, tail: tail, label: label, trace: trace, shard: shard, region: region, zone: zone, site: site, rack: rack, aisle: aisle, shelf: shelf, bin: bin, slot: slot, lane: lane, row: row, section: section, floor: floor }\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with20(combine, \"hello\", 42, \"done\", \"extra\", \"tail\", \"label\", \"trace\", \"shard\", \"region\", \"zone\", \"site\", \"rack\", \"aisle\", \"shelf\", \"bin\", \"slot\", \"lane\", \"row\", \"section\", \"floor\")\n",
        "  let value: {left: String, count: Int, marker: String, suffix: String, tail: String, label: String, trace: String, shard: String, region: String, zone: String, site: String, rack: String, aisle: String, shelf: String, bin: String, slot: String, lane: String, row: String, section: String, floor: String} = task::join(task)?\n",
        "  stdio::println(value.left)\n",
        "  stdio::println(int_to_string(value.count))\n",
        "  stdio::println(value.marker)\n",
        "  stdio::println(value.suffix)\n",
        "  stdio::println(value.tail)\n",
        "  stdio::println(value.label)\n",
        "  stdio::println(value.trace)\n",
        "  stdio::println(value.shard)\n",
        "  stdio::println(value.region)\n",
        "  stdio::println(value.zone)\n",
        "  stdio::println(value.site)\n",
        "  stdio::println(value.rack)\n",
        "  stdio::println(value.aisle)\n",
        "  stdio::println(value.shelf)\n",
        "  stdio::println(value.bin)\n",
        "  stdio::println(value.slot)\n",
        "  stdio::println(value.lane)\n",
        "  stdio::println(value.row)\n",
        "  stdio::println(value.section)\n",
        "  stdio::println(value.floor)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg20", &program, &[])
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
        "hello\n42\ndone\nextra\ntail\nlabel\ntrace\nshard\nregion\nzone\nsite\nrack\naisle\nshelf\nbin\nslot\nlane\nrow\nsection\nfloor\n"
    );
}

#[test]
fn bytecode_backend_runs_twenty_one_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn last(first: String, second: String, third: String, fourth: String, fifth: String, sixth: String, seventh: String, eighth: String, ninth: String, tenth: String, eleventh: String, twelfth: String, thirteenth: String, fourteenth: String, fifteenth: String, sixteenth: String, seventeenth: String, eighteenth: String, nineteenth: String, twentieth: String, twenty_first: String) -> String effects [concurrency]\n",
        "  twenty_first\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with21(last, \"one\", \"two\", \"three\", \"four\", \"five\", \"six\", \"seven\", \"eight\", \"nine\", \"ten\", \"eleven\", \"twelve\", \"thirteen\", \"fourteen\", \"fifteen\", \"sixteen\", \"seventeen\", \"eighteen\", \"nineteen\", \"twenty\", \"twenty-one\")\n",
        "  let value: String = task::join(task)?\n",
        "  stdio::println(value)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg21", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "twenty-one\n");
}

#[test]
fn bytecode_backend_runs_twenty_two_argument_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn last(first: String, second: String, third: String, fourth: String, fifth: String, sixth: String, seventh: String, eighth: String, ninth: String, tenth: String, eleventh: String, twelfth: String, thirteenth: String, fourteenth: String, fifteenth: String, sixteenth: String, seventeenth: String, eighteenth: String, nineteenth: String, twentieth: String, twenty_first: String, twenty_second: String) -> String effects [concurrency]\n",
        "  twenty_second\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
        "  let task = task::spawn_with22(last, \"one\", \"two\", \"three\", \"four\", \"five\", \"six\", \"seven\", \"eight\", \"nine\", \"ten\", \"eleven\", \"twelve\", \"thirteen\", \"fourteen\", \"fifteen\", \"sixteen\", \"seventeen\", \"eighteen\", \"nineteen\", \"twenty\", \"twenty-one\", \"twenty-two\")\n",
        "  let value: String = task::join(task)?\n",
        "  stdio::println(value)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task-arg22", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "twenty-two\n");
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
        ("byte", "byteValue"),
        ("byte_to_int", "byteToInt"),
        ("flag8_is_set", "flag8IsSet"),
        ("flag8_set", "flag8Set"),
        ("flag8_bits", "flag8RawBits"),
        ("flag8_from_bits", "flag8FromBits"),
        ("flag16be_is_set", "flag16beIsSet"),
        ("flag16be_set", "flag16beSet"),
        ("flag16be_bits", "flag16beRawBits"),
        ("flag16be_from_bits", "flag16beFromBits"),
        ("flag16le_is_set", "flag16leIsSet"),
        ("flag16le_set", "flag16leSet"),
        ("flag16le_bits", "flag16leRawBits"),
        ("flag16le_from_bits", "flag16leFromBits"),
        ("flag32be_is_set", "flag32beIsSet"),
        ("flag32be_set", "flag32beSet"),
        ("flag32be_bits", "flag32beRawBits"),
        ("flag32be_from_bits", "flag32beFromBits"),
        ("flag32le_is_set", "flag32leIsSet"),
        ("flag32le_set", "flag32leSet"),
        ("flag32le_bits", "flag32leRawBits"),
        ("flag32le_from_bits", "flag32leFromBits"),
        ("flag64be_is_set", "flag64beIsSet"),
        ("flag64be_set", "flag64beSet"),
        ("flag64be_bits", "flag64beRawBits"),
        ("flag64be_from_bits", "flag64beFromBits"),
        ("flag64le_is_set", "flag64leIsSet"),
        ("flag64le_set", "flag64leSet"),
        ("flag64le_bits", "flag64leRawBits"),
        ("flag64le_from_bits", "flag64leFromBits"),
        ("byte_chunk", "byteChunk"),
        ("byte_chunk_count", "byteChunkCount"),
        ("byte_append", "byteAppend"),
        ("byte_chunk_from_hex", "byteChunkFromHex"),
        ("byte_take", "byteTake"),
        ("byte_drop", "byteDrop"),
        ("byte_view", "byteView"),
        ("byte_view_to_chunk", "byteViewToChunk"),
        ("byte_view_count", "byteViewCount"),
        ("byte_view_take", "byteViewTake"),
        ("byte_view_drop", "byteViewDrop"),
        ("byte_view_slice", "byteViewSlice"),
        ("byte_chunks_empty", "byteChunksEmpty"),
        ("byte_chunks_one", "byteChunksOne"),
        ("byte_chunks_append", "byteChunksAppend"),
        ("byte_read_u8_be", "byteReadU8Be"),
        ("byte_expect_fixed_u8_be", "byteExpectFixedU8Be"),
        ("byte_decode_http2_frame", "byteDecodeHttp2Frame"),
        (
            "byte_decode_schema_width_sample",
            "byteDecodeSchemaWidthSample",
        ),
        (
            "byte_decode_schema_validation_sample",
            "byteDecodeSchemaValidationSample",
        ),
        (
            "http2_protocol_partial_preface",
            "http2ProtocolPartialPreface",
        ),
        (
            "http2_protocol_invalid_preface",
            "http2ProtocolInvalidPreface",
        ),
        (
            "http2_peer_limit_frame_size_exceeded",
            "http2PeerLimitFrameSizeExceeded",
        ),
        (
            "http2_peer_limit_header_list_size_exceeded",
            "http2PeerLimitHeaderListSizeExceeded",
        ),
        (
            "http2_peer_limit_flow_control_window_exceeded",
            "http2PeerLimitFlowControlWindowExceeded",
        ),
        (
            "http2_peer_limit_concurrent_streams_exceeded",
            "http2PeerLimitConcurrentStreamsExceeded",
        ),
        (
            "http2_peer_limit_settings_value_out_of_range",
            "http2PeerLimitSettingsValueOutOfRange",
        ),
        (
            "hpack_fixture_unsupported_header_block",
            "hpackFixtureUnsupportedHeaderBlock",
        ),
        (
            "http2_protocol_invalid_frame_kind",
            "http2ProtocolInvalidFrameKind",
        ),
        (
            "http2_protocol_invalid_stream_id",
            "http2ProtocolInvalidStreamId",
        ),
        (
            "http2_protocol_invalid_payload_length",
            "http2ProtocolInvalidPayloadLength",
        ),
        (
            "http2_protocol_invalid_data_padding",
            "http2ProtocolInvalidDataPadding",
        ),
        (
            "http2_protocol_unexpected_settings_ack",
            "http2ProtocolUnexpectedSettingsAck",
        ),
        (
            "http2_protocol_invalid_priority_dependency",
            "http2ProtocolInvalidPriorityDependency",
        ),
        (
            "http2_protocol_stream_after_goaway",
            "http2ProtocolStreamAfterGoaway",
        ),
        ("byte_read_u16_be", "byteReadU16Be"),
        ("byte_read_u24_be", "byteReadU24Be"),
        ("byte_read_u31_be", "byteReadU31Be"),
        ("byte_read_u32_be", "byteReadU32Be"),
        ("byte_read_u40_be", "byteReadU40Be"),
        ("byte_read_u48_be", "byteReadU48Be"),
        ("byte_read_u64_be", "byteReadU64Be"),
        ("byte_read_u16_le", "byteReadU16Le"),
        ("byte_read_u24_le", "byteReadU24Le"),
        ("byte_read_u31_le", "byteReadU31Le"),
        ("byte_read_u32_le", "byteReadU32Le"),
        ("byte_read_u40_le", "byteReadU40Le"),
        ("byte_read_u48_le", "byteReadU48Le"),
        ("byte_read_u64_le", "byteReadU64Le"),
        ("byte_write_u8_be", "byteWriteU8Be"),
        ("byte_write_u16_be", "byteWriteU16Be"),
        ("byte_write_u24_be", "byteWriteU24Be"),
        ("byte_write_u31_be", "byteWriteU31Be"),
        ("byte_write_u32_be", "byteWriteU32Be"),
        ("byte_write_u40_be", "byteWriteU40Be"),
        ("byte_write_u48_be", "byteWriteU48Be"),
        ("byte_write_u64_be", "byteWriteU64Be"),
        ("byte_write_u16_le", "byteWriteU16Le"),
        ("byte_write_u24_le", "byteWriteU24Le"),
        ("byte_write_u31_le", "byteWriteU31Le"),
        ("byte_write_u32_le", "byteWriteU32Le"),
        ("byte_write_u40_le", "byteWriteU40Le"),
        ("byte_write_u48_le", "byteWriteU48Le"),
        ("byte_write_u64_le", "byteWriteU64Le"),
        ("byte_count", "byteCount"),
        ("byte_count_to_int", "byteCountToInt"),
        ("byte_offset", "byteOffset"),
        ("byte_offset_to_int", "byteOffsetToInt"),
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
        ("list_nil", "listNil"),
        ("list_cons", "listCons"),
        ("list_is_empty", "listIsEmpty"),
        ("list_fold", "listFold"),
        ("list_reverse", "listReverse"),
        ("list_map", "listMap"),
        ("list_filter", "listFilter"),
        ("list_try_map", "listTryMap"),
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
        ("channel::select_many_priority", "channelSelectManyPriority"),
        ("channel::select_many_timeout", "channelSelectManyTimeout"),
        (
            "channel::select_many_timeout_result",
            "channelSelectManyTimeoutResult",
        ),
        (
            "channel::select_many_timeout_cancellable",
            "channelSelectManyTimeoutCancellable",
        ),
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
        ("task::spawn_with", "taskSpawnWith"),
        ("task::spawn_with2", "taskSpawnWith2"),
        ("task::spawn_with3", "taskSpawnWith3"),
        ("task::spawn_with4", "taskSpawnWith4"),
        ("task::spawn_with5", "taskSpawnWith5"),
        ("task::spawn_with6", "taskSpawnWith6"),
        ("task::spawn_with7", "taskSpawnWith7"),
        ("task::spawn_with8", "taskSpawnWith8"),
        ("task::spawn_with9", "taskSpawnWith9"),
        ("task::spawn_with10", "taskSpawnWith10"),
        ("task::spawn_with11", "taskSpawnWith11"),
        ("task::spawn_with12", "taskSpawnWith12"),
        ("task::spawn_with13", "taskSpawnWith13"),
        ("task::spawn_with14", "taskSpawnWith14"),
        ("task::spawn_with15", "taskSpawnWith15"),
        ("task::spawn_with16", "taskSpawnWith16"),
        ("task::spawn_with17", "taskSpawnWith17"),
        ("task::spawn_with18", "taskSpawnWith18"),
        ("task::spawn_with19", "taskSpawnWith19"),
        ("task::spawn_with20", "taskSpawnWith20"),
        ("task::spawn_with21", "taskSpawnWith21"),
        ("task::spawn_with22", "taskSpawnWith22"),
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
        ("net::receive_chunk", "netReceiveChunk"),
        ("net::send_chunk", "netSendChunk"),
        ("net::listen", "netListen"),
        ("net::accept", "netAccept"),
        ("net::accept_or_end", "netAcceptOrEnd"),
        ("net::accept_until", "netAcceptUntil"),
        ("net::read_chunk", "netReadChunk"),
        ("net::read_chunk_until", "netReadChunkUntil"),
        ("net::read_chunk_or_end", "netReadChunkOrEnd"),
        ("net::write_chunk", "netWriteChunk"),
        ("net::close_stream", "netCloseStream"),
        ("process::args", "processArgs"),
        ("process::env", "processEnv"),
        ("process::cwd", "processCwd"),
        ("process::exit", "processExit"),
        ("time::timeout_ms", "timeTimeoutMs"),
        ("time::deadline_after_ms", "timeDeadlineAfterMs"),
        ("time::wait_until", "timeWaitUntil"),
        ("time::cancel_token", "timeCancelToken"),
        ("time::cancel", "timeCancel"),
        ("time::is_cancelled", "timeIsCancelled"),
        ("time::wait_until_cancellable", "timeWaitUntilCancellable"),
        (
            "time::wait_until_cancellable_outcome",
            "timeWaitUntilCancellableOutcome",
        ),
    ] {
        assert_eq!(standard_library_method(surface), method);
    }

    let panic = std::panic::catch_unwind(|| standard_library_method("fs::unknown"));
    assert!(panic.is_err());
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
    let module = lower_surface_ast_with_module_identity(
        &parsed.tree,
        "main".to_string(),
        source.span(TextRange::at(0)),
    );
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

fn run_jvm_program_when_java_is_available(
    name: &str,
    program: &JvmProgram,
    args: &[&str],
) -> Option<std::process::Output> {
    run_jvm_program_with_env_when_java_is_available(name, program, &[], args)
}

fn run_jvm_program_with_env_when_java_is_available(
    name: &str,
    program: &JvmProgram,
    env: &[(&str, &str)],
    args: &[&str],
) -> Option<std::process::Output> {
    if Command::new("java").arg("-version").output().is_err() {
        return None;
    }

    let root = temp_dir(name);
    write_jvm_program(&root, program);

    let mut command = Command::new("java");
    command
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .current_dir(&root);
    for (key, value) in env {
        command.env(key, value);
    }
    for arg in args {
        command.arg(arg);
    }
    let output = command.output().expect("java should run");
    let _ = fs::remove_dir_all(&root);
    Some(output)
}

fn write_jvm_program(root: &std::path::Path, program: &JvmProgram) {
    for class in &program.classes {
        fs::write(root.join(&class.path), &class.contents).expect("classfile should be written");
    }
}
