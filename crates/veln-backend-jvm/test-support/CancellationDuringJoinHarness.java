public final class CancellationDuringJoinHarness {
    private static Object receive(Object receiver, String boundary) throws Exception {
        java.util.concurrent.atomic.AtomicReference<Object> result =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread probe = Thread.ofPlatform().daemon(true).start(() -> {
            try {
                result.set(VelnRuntime.channelRecv(receiver));
            } catch (Throwable error) {
                failure.set(error);
            }
        });
        probe.join(5000L);
        if (probe.isAlive()) {
            throw new AssertionError(boundary + " was not reached");
        }
        if (failure.get() != null) {
            throw new AssertionError(boundary + " failed", failure.get());
        }
        return result.get();
    }

    public static void main(String[] args) throws Exception {
        VelnRuntime.beginTaskLifetime();
        Object context = VelnProgram.fn_start_cancellation_during_join();
        Object target = VelnRuntime.recordField(context, "target");
        Object waiter = VelnRuntime.recordField(context, "waiter");
        Object waiting = VelnRuntime.recordField(context, "waiting");
        Object waiterCleanupStarted = VelnRuntime.recordField(context, "waiter_cleanup_started");
        Object waiterCleanupRelease = VelnRuntime.recordField(context, "waiter_cleanup_release");
        Object waiterCleanupFinished = VelnRuntime.recordField(context, "waiter_cleanup_finished");
        Object targetRelease = VelnRuntime.recordField(context, "target_release");
        Object targetFinished = VelnRuntime.recordField(context, "target_finished");

        receive(waiting, "Veln task-join wait boundary");
        VelnRuntime.taskCancel(waiter);
        receive(waiterCleanupStarted, "cancelled waiter cleanup start");

        java.util.concurrent.atomic.AtomicReference<Object> waiterResult =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> waiterFailure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread waiterJoin = Thread.ofPlatform().daemon(true).start(() -> {
            try {
                waiterResult.set(VelnProgram.fn_classify_cancelled_waiter(waiter));
            } catch (Throwable error) {
                waiterFailure.set(error);
            }
        });
        waiterJoin.join(200L);
        if (!waiterJoin.isAlive()) {
            throw new AssertionError("waiter join completed before its cleanup was released");
        }

        VelnRuntime.channelSend(waiterCleanupRelease, VelnRuntime.UNIT);
        receive(waiterCleanupFinished, "cancelled waiter cleanup completion");
        waiterJoin.join(5000L);
        if (waiterJoin.isAlive()) {
            throw new AssertionError("waiter join remained blocked after cleanup completed");
        }
        if (waiterFailure.get() != null) {
            throw new AssertionError("waiter join failed", waiterFailure.get());
        }
        if (!"cancelled".equals(waiterResult.get())) {
            throw new AssertionError("unexpected waiter classification: " + waiterResult.get());
        }

        VelnRuntime.channelSend(targetRelease, VelnRuntime.UNIT);
        receive(targetFinished, "uncancelled target completion");
        Object targetResult = VelnProgram.fn_join_uncancelled_target(target);
        if (!"target-completed".equals(targetResult)) {
            throw new AssertionError("waiter cancellation affected its target: " + targetResult);
        }

        VelnRuntime.finishTaskLifetime(true);
        System.out.println("join waiter cancelled after cleanup; target released and drained");
    }
}
