public final class ConnectCancellationHarness {
    public static void main(String[] args) throws Exception {
        java.util.concurrent.CountDownLatch waiting = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch cancelled = new java.util.concurrent.CountDownLatch(1);
        Object task = VelnRuntime.taskSpawn(new VelnRuntime.Fn() {
            public Object call(Object... ignored) {
                waiting.countDown();
                try {
                    cancelled.await();
                } catch (InterruptedException expected) {
                    Thread.interrupted();
                }
                return VelnRuntime.netConnect("127.0.0.1:1");
            }
        });
        if (!waiting.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
            throw new AssertionError("connect task did not reach its cancellation gate");
        }
        VelnRuntime.taskCancel(task);
        cancelled.countDown();
        Object joined = VelnRuntime.taskJoin(task);
        VelnRuntime.Result result = (VelnRuntime.Result) joined;
        if (result.isOk() || !((Boolean) VelnRuntime.taskJoinErrorIsCancelled(result.value()))) {
            throw new AssertionError("connect cancellation was not preserved: " + joined);
        }
        System.out.println("connect cancellation remained primary");
    }
}
