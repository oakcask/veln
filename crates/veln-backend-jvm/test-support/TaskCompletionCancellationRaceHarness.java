import java.util.concurrent.CountDownLatch;
import java.util.concurrent.FutureTask;
import java.util.concurrent.TimeUnit;
import java.util.function.Function;

public final class TaskCompletionCancellationRaceHarness {
    private static Object spawnWithCompletionHook(
        VelnRuntime.Fn action,
        Function<FutureTask<Object>, Thread> starter,
        Runnable completionHook
    ) {
        try {
            java.lang.reflect.Method spawn = VelnRuntime.class.getDeclaredMethod(
                "taskSpawnWithWorkerStarterAndCompletionHook",
                VelnRuntime.Fn.class,
                Function.class,
                Runnable.class
            );
            spawn.setAccessible(true);
            return spawn.invoke(null, action, starter, completionHook);
        } catch (ReflectiveOperationException failure) {
            throw new AssertionError("completion race injection should remain available", failure);
        }
    }

    private static void await(CountDownLatch latch, String message) {
        try {
            if (!latch.await(10L, TimeUnit.SECONDS)) {
                throw new AssertionError(message);
            }
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            throw new AssertionError("race harness was interrupted", interrupted);
        }
    }

    public static void main(String[] args) {
        CountDownLatch completionClaimed = new CountDownLatch(1);
        CountDownLatch releaseCompletion = new CountDownLatch(1);
        VelnRuntime.beginTaskLifetime();
        Object task = spawnWithCompletionHook(
            ignored -> Long.valueOf(42L),
            future -> Thread.ofPlatform().daemon(false).start(future),
            () -> {
                completionClaimed.countDown();
                await(releaseCompletion, "cancellation did not run after completion claimed");
            }
        );
        await(completionClaimed, "task did not reach the forced completion race");
        VelnRuntime.taskCancel(task);
        releaseCompletion.countDown();
        VelnRuntime.finishTaskLifetime(true);

        for (int attempt = 0; attempt < 3; attempt += 1) {
            Object joined = VelnRuntime.taskJoin(task);
            if (!"Ok(42)".equals(joined.toString())) {
                throw new AssertionError("completed task changed outcome after cancellation: " + joined);
            }
        }
        System.out.println("completion won cancellation race");
    }
}
