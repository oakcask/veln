public final class CancellableWaitInterruptionHarness {
    private static boolean isWaiting(Thread thread) {
        for (StackTraceElement frame : thread.getStackTrace()) {
            if (frame.getClassName().equals("VelnRuntime")
                && frame.getMethodName().equals("timeWaitUntilCancellable")) {
                return true;
            }
        }
        return false;
    }

    public static void main(String[] args) throws Exception {
        Object token = VelnRuntime.timeCancelToken();
        Object deadline = VelnRuntime.timeDeadlineAfterMs(Long.valueOf(60000L));
        java.util.concurrent.atomic.AtomicReference<Object> outcome =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        java.util.concurrent.atomic.AtomicBoolean interruptRetained =
            new java.util.concurrent.atomic.AtomicBoolean(false);
        Thread worker = new Thread(() -> {
            try {
                outcome.set(VelnRuntime.timeWaitUntilCancellable(deadline, token));
                interruptRetained.set(Thread.currentThread().isInterrupted());
            } catch (Throwable error) {
                failure.set(error);
            }
        }, "veln-cancellable-wait-interruption");
        worker.start();

        long limit = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        while (!isWaiting(worker) && worker.isAlive() && System.nanoTime() < limit) {
            Thread.yield();
        }
        if (!isWaiting(worker)) {
            throw new AssertionError("worker did not enter the cancellable wait boundary");
        }
        worker.interrupt();
        worker.join(5000L);
        if (worker.isAlive()) {
            throw new AssertionError("interrupted cancellable wait did not finish");
        }
        if (failure.get() != null) {
            throw new AssertionError("interrupted cancellable wait failed", failure.get());
        }
        if (!"Interrupted".equals(String.valueOf(outcome.get()))) {
            throw new AssertionError("unexpected interruption outcome: " + outcome.get());
        }
        if (Boolean.TRUE.equals(VelnRuntime.timeIsCancelled(token))) {
            throw new AssertionError("host interruption cancelled the token");
        }
        if (!interruptRetained.get()) {
            throw new AssertionError("host interruption state was not retained");
        }
        System.out.println("cancellable wait preserved host interruption state");
    }
}
