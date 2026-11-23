import java.lang.reflect.Field;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicReference;

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

    private static void requireCancelled(Object joined, String boundary) {
        VelnRuntime.Result result = (VelnRuntime.Result) joined;
        require(
            !result.isOk()
                && ((Boolean) VelnRuntime.taskJoinErrorIsCancelled(result.value())).booleanValue(),
            boundary + " did not produce a cancelled join: " + joined
        );
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
        requireCancelled(VelnRuntime.taskJoin(waiter), "pre-admission cancellation");
        require(!invoked.get(), "cancelled waiter entered the callback");
        release(release);
        holder.join();
        require(read(cell) == 10L, "pre-admission cancellation changed state or stranded admission");
    }

    private static void cancellationAtCheckpoint() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(20));
        CountDownLatch checkpointReady = new CountDownLatch(1);
        CountDownLatch checkpointRelease = new CountDownLatch(1);
        AtomicBoolean callbackReturned = new AtomicBoolean(false);
        Object worker = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored ->
            VelnRuntime.ioProviderStateUpdate(
                cell,
                (VelnRuntime.Fn) probe -> {
                    checkpointReady.countDown();
                    awaitIgnoringInterrupt(checkpointRelease, "compiled checkpoint release");
                    return Long.valueOf(1);
                },
                (VelnRuntime.Fn) args -> {
                    Object transition = VelnProgram.fn_atomic_state_tail(
                        Long.valueOf(30000),
                        args[1]
                    );
                    callbackReturned.set(true);
                    return transition;
                }
            )
        );
        await(checkpointReady, "compiled checkpoint boundary");
        VelnRuntime.taskCancel(worker);
        release(checkpointRelease);
        requireCancelled(VelnRuntime.taskJoin(worker), "generated checkpoint cancellation");
        require(!callbackReturned.get(), "compiled tail callback returned after cancellation");
        require(read(cell) == 20L, "checkpoint cancellation published or stranded the state cell");
        VelnRuntime.ioProviderStateUpdate(
            cell,
            Long.valueOf(1),
            (VelnRuntime.Fn) args -> transition(
                Long.valueOf(((Long) args[0]).longValue() + 1L),
                VelnRuntime.UNIT
            )
        );
        require(read(cell) == 21L, "checkpoint cancellation did not release the state cell");
    }

    private static void cancellationAfterCallbackReturnPublishes() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(30));
        CountDownLatch callbackReturned = new CountDownLatch(1);
        CountDownLatch publishedAndReleased = new CountDownLatch(1);
        CountDownLatch cancellationRelease = new CountDownLatch(1);
        AtomicBoolean resultReachedCaller = new AtomicBoolean(false);
        Object worker = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            Object result = VelnRuntime.ioProviderStateUpdateObserved(
                cell,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> {
                    callbackReturned.countDown();
                    return transition(Long.valueOf(31), "unobservable-result");
                },
                () -> {
                    publishedAndReleased.countDown();
                    awaitIgnoringInterrupt(cancellationRelease, "late cancellation release");
                }
            );
            resultReachedCaller.set(true);
            return result;
        });
        await(callbackReturned, "callback return boundary");
        await(publishedAndReleased, "publication and admission release boundary");
        require(read(cell) == 31L, "published replacement was not visible after callback return");
        VelnRuntime.taskCancel(worker);
        release(cancellationRelease);
        Object joined = VelnRuntime.taskJoin(worker);
        requireCancelled(joined, "post-publication cancellation");
        require(!resultReachedCaller.get(), "published callback result reached the task caller");
        require(read(cell) == 31L, "late cancellation rolled back published state");
    }

    private static void failuresAndReentrancyPreserveAndRelease() {
        Object failed = VelnRuntime.ioProviderState(Long.valueOf(40));
        VelnRuntime.RuntimeFailure planned =
            new VelnRuntime.RuntimeFailure("planned callback failure");
        try {
            VelnRuntime.ioProviderStateUpdate(
                failed,
                VelnRuntime.UNIT,
                (VelnRuntime.Fn) args -> { throw planned; }
            );
            throw new AssertionError("callback failure did not propagate");
        } catch (VelnRuntime.RuntimeFailure expected) {
            require(expected == planned, "callback failure identity changed");
        }
        require(read(failed) == 40L, "callback failure published or stranded the cell");
        VelnRuntime.ioProviderStateUpdate(
            failed,
            Long.valueOf(1),
            (VelnRuntime.Fn) args -> transition(
                Long.valueOf(((Long) args[0]).longValue() + 1L),
                VelnRuntime.UNIT
            )
        );
        require(read(failed) == 41L, "callback failure did not release the cell for reuse");

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

    private static Object deepList(long first, int count) {
        Object list = VelnRuntime.listNil();
        for (int index = count - 1; index >= 0; index -= 1) {
            list = VelnRuntime.listCons(Long.valueOf(first + index), list);
        }
        return list;
    }

    private static void requireDeepList(Object value, long first, int count, String boundary) {
        Object current = value;
        for (int index = 0; index < count; index += 1) {
            require(VelnRuntime.isCons(current), boundary + " ended at index " + index);
            require(
                ((Long) VelnRuntime.listHead(current)).longValue() == first + index,
                boundary + " changed value at index " + index
            );
            current = VelnRuntime.listTail(current);
        }
        require(VelnRuntime.isNil(current), boundary + " retained an unexpected tail");
    }

    private static void deeplyNestedValuesAreStackBoundedAndReusable() {
        int count = 100000;
        Object initial = deepList(0L, count);
        Object replacement = deepList(1L, count);
        Object context = deepList(2L, count);
        Object result = deepList(3L, count);
        Object cell = VelnRuntime.ioProviderState(initial);
        Object publishedResult = VelnRuntime.ioProviderStateUpdate(
            cell,
            context,
            (VelnRuntime.Fn) args -> {
                requireDeepList(args[0], 0L, count, "deep initial state");
                requireDeepList(args[1], 2L, count, "deep update context");
                return transition(replacement, result);
            }
        );
        requireDeepList(publishedResult, 3L, count, "deep update result");
        Object observed = VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> transition(args[0], args[0])
        );
        requireDeepList(observed, 1L, count, "deep published replacement");
        Object reused = VelnRuntime.ioProviderStateUpdate(
            cell,
            VelnRuntime.UNIT,
            (VelnRuntime.Fn) args -> transition(Long.valueOf(77), "reused")
        );
        require("reused".equals(reused), "deep state cell did not remain usable");
        require(read(cell) == 77L, "later update did not publish after deep values");
    }

    private static void unrelatedInterruptDoesNotStrandAdmission() throws Exception {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(80));
        CountDownLatch admitted = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        AtomicBoolean waiterInterruptRestored = new AtomicBoolean(false);
        AtomicReference<Throwable> failure = new AtomicReference<Throwable>();
        Thread holder = new Thread(() -> {
            try {
                VelnRuntime.ioProviderStateUpdate(
                    cell,
                    VelnRuntime.UNIT,
                    (VelnRuntime.Fn) args -> {
                        admitted.countDown();
                        await(release, "interrupted waiter holder release");
                        return transition(Long.valueOf(81), VelnRuntime.UNIT);
                    }
                );
            } catch (Throwable caught) {
                failure.compareAndSet(null, caught);
            }
        });
        Thread waiter = new Thread(() -> {
            try {
                VelnRuntime.ioProviderStateUpdate(
                    cell,
                    VelnRuntime.UNIT,
                    (VelnRuntime.Fn) args -> transition(
                        Long.valueOf(((Long) args[0]).longValue() + 1L),
                        VelnRuntime.UNIT
                    )
                );
                waiterInterruptRestored.set(Thread.currentThread().isInterrupted());
            } catch (Throwable caught) {
                failure.compareAndSet(null, caught);
            }
        });
        holder.start();
        await(admitted, "interrupt holder admission");
        waiter.start();
        waitForBlocked(waiter, "interrupt admission waiter");
        waiter.interrupt();
        release(release);
        holder.join(TimeUnit.SECONDS.toMillis(10));
        waiter.join(TimeUnit.SECONDS.toMillis(10));
        require(!holder.isAlive(), "interrupted waiter stranded the admitted publisher");
        require(!waiter.isAlive(), "interrupted waiter did not finish after publication");
        require(failure.get() == null, "interrupt admission case failed: " + failure.get());
        require(waiterInterruptRestored.get(), "unrelated waiter interrupt was not restored");
        require(read(cell) == 82L, "interrupted waiter did not serialize after publication");
    }

    public static void main(String[] args) throws Exception {
        serializationAndIdentity();
        cancellationBeforeAdmission();
        cancellationAtCheckpoint();
        cancellationAfterCallbackReturnPublishes();
        failuresAndReentrancyPreserveAndRelease();
        deeplyNestedValuesAreStackBoundedAndReusable();
        unrelatedInterruptDoesNotStrandAdmission();
    }
}
