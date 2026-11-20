public final class TaskStartRollbackHarness {
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

    public static void main(String[] args) {
        VelnRuntime.beginTaskLifetime();
        try {
            Object handle = spawnWithStarter(
                ignored -> null,
                ignored -> {
                    throw new RuntimeException("injected start failure");
                }
            );
            throw new AssertionError("failed task start published a handle: " + handle);
        } catch (VelnRuntime.RuntimeFailure failure) {
            if (!"task worker could not start".equals(failure.getMessage())) {
                throw new AssertionError("unexpected failure: " + failure.getMessage());
            }
            if (failure.getCause() == null
                    || !"injected start failure".equals(failure.getCause().getMessage())) {
                throw new AssertionError("start cause was not preserved", failure);
            }
        }
        VelnRuntime.finishTaskLifetime(false);
        System.out.println("rolled back");
    }
}
