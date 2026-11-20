public final class TaskStartRollbackOverlapHarness {
    private static final java.util.concurrent.CountDownLatch CHILD_START_ENTERED =
        new java.util.concurrent.CountDownLatch(1);
    private static final java.util.concurrent.atomic.AtomicBoolean CANCELLATION_OBSERVED =
        new java.util.concurrent.atomic.AtomicBoolean(false);

    private static Object spawnWithStarter(
        VelnRuntime.Fn action,
        java.util.function.Function<java.util.concurrent.FutureTask<Object>, Thread> starter
    ) {
        try {
            java.lang.reflect.Method spawn = VelnRuntime.class.getDeclaredMethod(
                "taskSpawnWithWorkerStarter",
                VelnRuntime.Fn.class,
                java.util.function.Function.class
            );
            spawn.setAccessible(true);
            return spawn.invoke(null, action, starter);
        } catch (java.lang.reflect.InvocationTargetException failed) {
            Throwable cause = failed.getCause();
            if (cause instanceof RuntimeException runtimeFailure) {
                throw runtimeFailure;
            }
            if (cause instanceof Error error) {
                throw error;
            }
            throw new AssertionError("task start failed with a checked exception", cause);
        } catch (ReflectiveOperationException reflectionFailure) {
            throw new AssertionError("task start injection should remain available", reflectionFailure);
        }
    }

    public static void main(String[] args) throws Exception {
        VelnRuntime.beginTaskLifetime();
        Object parent = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
            try {
                Object unpublished = spawnWithStarter(
                    childIgnored -> null,
                    childFuture -> {
                        CHILD_START_ENTERED.countDown();
                        try {
                            new java.util.concurrent.CountDownLatch(1).await();
                            throw new AssertionError("child start wait returned without cancellation");
                        } catch (InterruptedException cancelled) {
                            CANCELLATION_OBSERVED.set(true);
                            Thread.currentThread().interrupt();
                            throw new RuntimeException("overlapped start failure");
                        }
                    }
                );
                throw new AssertionError("failed child start published " + unpublished);
            } catch (VelnRuntime.RuntimeFailure failure) {
                if (!"task worker could not start".equals(failure.getMessage())) {
                    throw new AssertionError("unexpected failure: " + failure.getMessage());
                }
                if (failure.getCause() == null
                        || !"overlapped start failure".equals(
                            failure.getCause().getMessage())) {
                    throw new AssertionError("start cause was not preserved", failure);
                }
            }
            return null;
        });
        if (!CHILD_START_ENTERED.await(10, java.util.concurrent.TimeUnit.SECONDS)) {
            throw new AssertionError("child start did not pause");
        }
        VelnRuntime.finishTaskLifetime(false);
        if (!CANCELLATION_OBSERVED.get()) {
            throw new AssertionError("child start did not overlap the cancellation snapshot");
        }
        System.out.println("overlap drained " + VelnRuntime.taskJoin(parent));
    }
}
