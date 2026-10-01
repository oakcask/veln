public final class JoinBlockingHarness {
    private static final int CANCELLER_COUNT = 32;

    private static boolean isWaitingInTaskJoin(Thread thread) {
        if (thread.getState() != Thread.State.WAITING) return false;
        for (StackTraceElement frame : thread.getStackTrace()) {
            if (frame.getClassName().equals("VelnRuntime")
                && frame.getMethodName().equals("taskJoin")) {
                return true;
            }
        }
        return false;
    }

    public static void main(String[] args) throws Exception {
        Object context = VelnProgram.fn_start_cleanup_worker();
        Object worker = VelnRuntime.recordField(context, "worker");
        Object cleanupStarted = VelnRuntime.recordField(context, "cleanup_started");
        Object cleanupGate = VelnRuntime.recordField(context, "cleanup_gate");
        Object cleanupFinished = VelnRuntime.recordField(context, "cleanup_finished");
        java.util.concurrent.CountDownLatch cancellersReady =
            new java.util.concurrent.CountDownLatch(CANCELLER_COUNT);
        java.util.concurrent.CountDownLatch startCancellation =
            new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch cancellersDone =
            new java.util.concurrent.CountDownLatch(CANCELLER_COUNT);
        java.util.concurrent.atomic.AtomicReference<Throwable> cancellationFailure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        for (int index = 0; index < CANCELLER_COUNT; index += 1) {
            Thread canceller = new Thread(() -> {
                cancellersReady.countDown();
                try {
                    startCancellation.await();
                    VelnRuntime.taskCancel(worker);
                } catch (Throwable error) {
                    cancellationFailure.compareAndSet(null, error);
                } finally {
                    cancellersDone.countDown();
                }
            }, "veln-canceller-" + index);
            canceller.start();
        }
        if (!cancellersReady.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
            throw new AssertionError("concurrent cancellers did not become ready");
        }
        startCancellation.countDown();
        Object started = VelnRuntime.channelRecv(cleanupStarted);
        if (!started.toString().contains("started")) {
            throw new AssertionError("cleanup did not start after cancellation: " + started);
        }
        if (!cancellersDone.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
            throw new AssertionError("concurrent cancellers did not complete");
        }
        if (cancellationFailure.get() != null) {
            throw new AssertionError("concurrent cancellation failed", cancellationFailure.get());
        }
        java.util.concurrent.atomic.AtomicReference<Object> result =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread joiner = new Thread(() -> {
            try {
                result.set(VelnProgram.fn_join_cleanup_worker(worker));
            } catch (Throwable error) {
                failure.set(error);
            }
        }, "veln-join-probe");
        joiner.start();

        long deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        while (!isWaitingInTaskJoin(joiner) && System.nanoTime() < deadline) {
            Thread.yield();
        }
        if (!isWaitingInTaskJoin(joiner)) {
            throw new AssertionError("joiner did not reach the blocking taskJoin boundary");
        }
        if (result.get() != null || failure.get() != null) {
            throw new AssertionError("join completed while cleanup remained gated");
        }

        VelnRuntime.channelSend(cleanupGate, "release");
        Object finished = VelnRuntime.channelRecv(cleanupFinished);
        if (!finished.toString().contains("finished")) {
            throw new AssertionError("cleanup completion was not observed: " + finished);
        }
        joiner.join(5000L);
        if (joiner.isAlive()) {
            throw new AssertionError("join remained blocked after cleanup completed");
        }
        if (failure.get() != null) {
            throw new AssertionError("join failed unexpectedly", failure.get());
        }
        if (!"cancelled".equals(result.get())) {
            throw new AssertionError("unexpected join result: " + result.get());
        }
        System.out.println("concurrent cancellation preserved cleanup completion ordering");
    }
}
