import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

// Backend contract: atomic callback publication and cancellation checkpoints.
// Filesystem decisions and fake filesystem state do not belong in this harness.
public final class IoProviderStateHarness {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void await(CountDownLatch latch) {
        long end = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
        boolean interrupted = false;
        try {
            while (true) {
                try {
                    check(latch.await(Math.max(0, end - System.nanoTime()), TimeUnit.NANOSECONDS), "gate timed out");
                    return;
                } catch (InterruptedException cancellation) {
                    interrupted = true;
                }
            }
        } finally {
            if (interrupted) Thread.currentThread().interrupt();
        }
    }

    private static Object read(Object cell) {
        return VelnRuntime.ioProviderUpdate(cell, 0L,
            (VelnRuntime.Fn) args -> Map.of("state", args[0], "result", args[0]));
    }

    private static void expectFailure(Runnable action, String message) {
        try {
            action.run();
        } catch (RuntimeException failure) {
            check(message.equals(failure.getMessage()), "unexpected failure: " + failure);
            return;
        }
        throw new AssertionError("missing failure: " + message);
    }

    private static void failuresPreserveStateAndUnlock() {
        Object cell = VelnRuntime.ioProviderState(7L);
        expectFailure(() -> VelnRuntime.ioProviderUpdate(cell, 0L,
            (VelnRuntime.Fn) args -> { throw new IllegalStateException("callback failed"); }), "callback failed");
        check(read(cell).equals(7L), "failed callback published state");
        expectFailure(() -> VelnRuntime.ioProviderUpdate(cell, 0L,
            (VelnRuntime.Fn) args -> read(cell)), "reentrant provider state update");
        Object other = VelnRuntime.ioProviderState(20L);
        expectFailure(() -> VelnRuntime.ioProviderUpdate(cell, 0L,
            (VelnRuntime.Fn) args -> read(other)), "reentrant provider state update");
        expectFailure(() -> VelnRuntime.ioProviderUpdate(cell, 0L,
            (VelnRuntime.Fn) args -> Map.of("result", 1L)), "provider state transition must return state and result");
        Object child = VelnRuntime.taskSpawn((VelnRuntime.Fn) args -> read(cell));
        VelnRuntime.Result outcome = (VelnRuntime.Result) VelnRuntime.taskJoin(child);
        check(outcome.isOk() && outcome.value().equals(7L), "callback failure retained the monitor");
    }

    private static void cancellationAfterCommit() {
        Object cell = VelnRuntime.ioProviderState(3L);
        CountDownLatch entered = new CountDownLatch(1);
        CountDownLatch cancelled = new CountDownLatch(1);
        AtomicBoolean checkpointReturned = new AtomicBoolean();
        Object task = VelnRuntime.taskSpawn((VelnRuntime.Fn) args ->
            VelnRuntime.ioProviderUpdate(cell, 0L, (VelnRuntime.Fn) transition -> {
                entered.countDown();
                await(cancelled);
                VelnRuntime.taskCancellationCheckpoint();
                checkpointReturned.set(true);
                return Map.of("state", 9L, "result", 42L);
            }));
        await(entered);
        VelnRuntime.taskCancel(task);
        cancelled.countDown();
        VelnRuntime.Result outcome = (VelnRuntime.Result) VelnRuntime.taskJoin(task);
        check(!outcome.isOk() && (Boolean) VelnRuntime.taskJoinErrorIsCancelled(outcome.value()), "cancellation not delivered");
        check(checkpointReturned.get(), "cancellation entered the transition");
        Object observer = VelnRuntime.taskSpawn((VelnRuntime.Fn) args -> read(cell));
        VelnRuntime.Result observed = (VelnRuntime.Result) VelnRuntime.taskJoin(observer);
        check(observed.isOk() && observed.value().equals(9L), "cancelled commit lost state or stranded the monitor");
    }

    private static void cancellationBeforeEntry() {
        Object cell = VelnRuntime.ioProviderState(5L);
        CountDownLatch started = new CountDownLatch(1);
        CountDownLatch cancelled = new CountDownLatch(1);
        AtomicBoolean invoked = new AtomicBoolean();
        Object task = VelnRuntime.taskSpawn((VelnRuntime.Fn) args -> {
            started.countDown();
            await(cancelled);
            return VelnRuntime.ioProviderUpdate(cell, 0L, (VelnRuntime.Fn) transition -> {
                invoked.set(true);
                return Map.of("state", 99L, "result", 0L);
            });
        });
        await(started);
        VelnRuntime.taskCancel(task);
        cancelled.countDown();
        VelnRuntime.Result outcome = (VelnRuntime.Result) VelnRuntime.taskJoin(task);
        check(!outcome.isOk() && (Boolean) VelnRuntime.taskJoinErrorIsCancelled(outcome.value()), "pre-entry cancellation not delivered");
        check(!invoked.get() && read(cell).equals(5L), "cancelled caller entered the transition");
    }

    public static void main(String[] args) {
        failuresPreserveStateAndUnlock();
        cancellationBeforeEntry();
        cancellationAfterCommit();
        System.out.println("provider state atomicity verified");
    }
}
