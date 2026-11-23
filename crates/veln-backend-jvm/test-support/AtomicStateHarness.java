import java.lang.reflect.Field;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public final class AtomicStateHarness {
    private static Map<String, Object> transition(Object state, Object result) {
        return VelnRuntime.record("state", state, "result", result);
    }

    private static long read(Object cell) {
        return ((Long) VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> transition(args[0], args[0])
        )).longValue();
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void await(CountDownLatch latch, String boundary) {
        try {
            require(latch.await(10, TimeUnit.SECONDS), "timed out at " + boundary);
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            throw new AssertionError("interrupted at " + boundary, interrupted);
        }
    }

    private static void release(CountDownLatch latch) {
        latch.countDown();
    }

    private static void awaitIgnoringInterrupt(CountDownLatch latch, String boundary) {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
        while (latch.getCount() != 0L) {
            try {
                long remaining = deadline - System.nanoTime();
                require(remaining > 0L, "timed out at " + boundary);
                latch.await(remaining, TimeUnit.NANOSECONDS);
            } catch (InterruptedException interrupted) {
                // The update callback deliberately returns after cancellation is
                // requested so publication can establish the post-return rule.
            }
        }
    }

    private static void waitForBlocked(Thread thread, String boundary) {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
        while (thread.getState() != Thread.State.WAITING) {
            require(System.nanoTime() < deadline, "timed out observing " + boundary);
            Thread.yield();
        }
    }

    private static void serializationAndIdentity() throws Exception {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(0));
        Object other = VelnRuntime.ioProviderState(Long.valueOf(0));
        CountDownLatch entered = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        AtomicBoolean secondEntered = new AtomicBoolean(false);

        Thread first = new Thread(() -> VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> {
                entered.countDown();
                await(release, "serialized first callback release");
                return transition(Long.valueOf(((Long) args[0]).longValue() + 1), args[0]);
            }
        ));
        first.start();
        await(entered, "serialized first callback admission");

        Thread second = new Thread(() -> VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> {
                secondEntered.set(true);
                return transition(Long.valueOf(((Long) args[0]).longValue() + 1), args[0]);
            }
        ));
        second.start();
        waitForBlocked(second, "serialized second admission wait");
        require(!secondEntered.get(), "second callback entered before the admitted update released");
        release(release);
        first.join();
        second.join();
        require(read(cell) == 2L, "serialized updates lost a completed replacement");
        require(read(other) == 0L, "independent state cells shared storage");

        Object child = VelnRuntime.taskSpawnWith(
            (VelnRuntime.Fn) args -> VelnRuntime.ioProviderStateUpdate(
                args[0],
                Long.valueOf(3),
                (VelnRuntime.Fn) update -> transition(
                    Long.valueOf(((Long) update[0]).longValue() + ((Long) update[1]).longValue()),
                    update[0]
                )
            ),
            cell
        );
        VelnRuntime.taskJoin(child);
        require(read(cell) == 5L, "task freezing copied a state cell instead of preserving identity");
    }

    private static void cancellationBeforeAdmission() throws Exception {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(10));
        CountDownLatch entered = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        Thread holder = new Thread(() -> VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> {
                entered.countDown();
                await(release, "pre-admission holder release");
                return transition(args[0], VelnRuntime.UNIT);
            }
        ));
        holder.start();
        await(entered, "pre-admission holder entry");
        AtomicBoolean invoked = new AtomicBoolean(false);
        Object waiter = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored ->
            VelnRuntime.ioProviderStateUpdate(
                cell,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> {
                    invoked.set(true);
                    return transition(Long.valueOf(99), VelnRuntime.UNIT);
                }
            )
        );
        Field futureField = waiter.getClass().getDeclaredField("thread");
        futureField.setAccessible(true);
        Thread waiterThread = (Thread) futureField.get(waiter);
        waitForBlocked(waiterThread, "cancelled pre-admission waiter");
        VelnRuntime.taskCancel(waiter);
        VelnRuntime.taskJoin(waiter);
        require(!invoked.get(), "cancelled waiter entered the callback");
        release(release);
        holder.join();
        require(read(cell) == 10L, "pre-admission cancellation changed state or stranded admission");
    }

    private static void cancellationAtCheckpoint() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(20));
        CountDownLatch admitted = new CountDownLatch(1);
        Object worker = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored ->
            VelnRuntime.ioProviderStateUpdate(
                cell,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> {
                    admitted.countDown();
                    return VelnProgram.fn_atomic_state_tail(args[0], args[1]);
                }
            )
        );
        await(admitted, "checkpoint callback admission");
        VelnRuntime.taskCancel(worker);
        VelnRuntime.taskJoin(worker);
        require(read(cell) == 20L, "checkpoint cancellation published or stranded the state cell");
    }

    private static void cancellationAfterCallbackReturnPublishes() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(30));
        CountDownLatch returning = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        Object worker = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored ->
            VelnRuntime.ioProviderStateUpdate(
                cell,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> {
                    returning.countDown();
                    awaitIgnoringInterrupt(release, "post-callback cancellation release");
                    return transition(Long.valueOf(31), "unobservable-result");
                }
            )
        );
        await(returning, "post-callback return boundary");
        VelnRuntime.taskCancel(worker);
        release(release);
        Object joined = VelnRuntime.taskJoin(worker);
        require(
            joined.toString().contains("cancelled"),
            "cancelled update returned its callback result: " + joined
        );
        require(read(cell) == 31L, "completed callback replacement was not published before cancellation");
    }

    private static void failuresAndReentrancyPreserveAndRelease() {
        Object failed = VelnRuntime.ioProviderState(Long.valueOf(40));
        try {
            VelnRuntime.ioProviderStateUpdate(
                failed,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> { throw new VelnRuntime.RuntimeFailure("planned callback failure"); }
            );
            throw new AssertionError("callback failure did not propagate");
        } catch (VelnRuntime.RuntimeFailure expected) {
            require(expected.getMessage().contains("planned"), "callback failure identity changed");
        }
        require(read(failed) == 40L, "callback failure published or stranded the cell");

        Object reentrant = VelnRuntime.ioProviderState(Long.valueOf(50));
        try {
            VelnRuntime.ioProviderStateUpdate(
                reentrant,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> VelnRuntime.ioProviderStateUpdate(
                    reentrant,
                    VelnRuntime.UNIT,
                    (VelnRuntime.Fn) nested -> transition(Long.valueOf(51), VelnRuntime.UNIT)
                )
            );
            throw new AssertionError("same-cell reentrancy did not fail");
        } catch (VelnRuntime.RuntimeFailure expected) {
            require(expected.getMessage().contains("reentrant"), "reentrancy failure changed");
        }
        require(read(reentrant) == 50L, "reentrancy failure published or stranded the cell");
    }

    public static void main(String[] args) throws Exception {
        serializationAndIdentity();
        cancellationBeforeAdmission();
        cancellationAtCheckpoint();
        cancellationAfterCallbackReturnPublishes();
        failuresAndReentrancyPreserveAndRelease();
    }
}
