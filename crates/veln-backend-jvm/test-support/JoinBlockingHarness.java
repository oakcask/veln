public final class JoinBlockingHarness {
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
        Object cleanupGate = VelnRuntime.recordField(context, "cleanup_gate");
        Object cleanupFinished = VelnRuntime.recordField(context, "cleanup_finished");
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
        System.out.println("join waited for cleanup completion");
    }
}
