import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

// Real-host publication boundary: an actual FileChannel opened before owner
// teardown is released rather than published. The bridges below invoke only
// production host operations; there is no scripted filesystem or close reply.
// Scoped Veln support separately owns deterministic provider failure plans.
public final class IoProviderHostPublicationHarness {
    private static final Class<?> OWNER;
    private static final Class<?> FILE;
    private static final Method REGISTER;
    private static final Method FIND_HANDLER;

    static {
        try {
            OWNER = Class.forName("VelnRuntime$IoProviderOwner");
            FILE = Class.forName("VelnRuntime$IoHostFile");
            REGISTER = VelnRuntime.class.getDeclaredMethod("ioProviderRegister", Object.class, Object.class, String.class, Object.class);
            REGISTER.setAccessible(true);
            FIND_HANDLER = VelnRuntime.class.getDeclaredMethod("findHandler", String.class);
            FIND_HANDLER.setAccessible(true);
        } catch (ReflectiveOperationException failure) { throw new ExceptionInInitializerError(failure); }
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static Object field(Object value, String name) throws Exception {
        Field field = value.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(value);
    }

    private static void await(CountDownLatch gate) {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5);
        boolean interrupted = false;
        try {
            while (true) {
                try {
                    check(gate.await(Math.max(0L, deadline - System.nanoTime()), TimeUnit.NANOSECONDS), "publication gate timed out");
                    return;
                } catch (InterruptedException cancellation) { interrupted = true; }
            }
        } finally {
            if (interrupted) Thread.currentThread().interrupt();
        }
    }

    private static Object register(Object owner, FileChannel channel) {
        try {
            Constructor<?> constructor = FILE.getDeclaredConstructor(FileChannel.class, Object.class, boolean.class);
            constructor.setAccessible(true);
            Object portable = VelnRuntime.adt("Path::PortablePath", new Object[] {"data"});
            Object host = constructor.newInstance(channel, portable, false);
            return REGISTER.invoke(null, owner, 1L, "ReadFile", host);
        } catch (InvocationTargetException failure) {
            if (failure.getCause() instanceof RuntimeException) throw (RuntimeException) failure.getCause();
            throw new AssertionError(failure.getCause());
        } catch (ReflectiveOperationException failure) { throw new AssertionError(failure); }
    }

    private static void hostHandler(String effect, String operation, String hostOperation, VelnRuntime.Fn request) {
        VelnRuntime.pushHandler(effect, new Object[] {operation}, new Object[] {(VelnRuntime.Fn) arguments -> {
            // Generated Veln handler callbacks begin with this checkpoint.
            VelnRuntime.taskCancellationCheckpoint();
            return VelnRuntime.ioProviderHostRequest(hostOperation, arguments[0], request.call(arguments));
        }}, new Object[0]);
    }

    private static void lateOpen(boolean cancel) throws Exception {
        Path path = Files.createTempFile("veln-host-publication-", ".bin");
        byte[] original = new byte[] {97, 98, 99, 100};
        Files.write(path, original);
        VelnRuntime.pushHandler("std::io_provider::Scope", new Object[0], new Object[0], new Object[0]);
        Object owner = field(FIND_HANDLER.invoke(null, "std::io_provider::Scope"), "ioOwner");
        hostHandler("std::io::Read", "read", "read", arguments -> Map.of("maximum", arguments[1]));
        hostHandler("std::io::Seek", "seek", "seek", arguments -> Map.of("origin", arguments[1]));
        hostHandler("std::io::ReadAt", "read_at", "read_at", arguments -> Map.of("offset", arguments[1], "maximum", arguments[2]));
        hostHandler("std::io::Close", "close", "close", arguments -> VelnRuntime.UNIT);
        AtomicReference<FileChannel> opened = new AtomicReference<>();
        CountDownLatch entered = new CountDownLatch(1);
        CountDownLatch complete = new CountDownLatch(1);
        try {
            Object task = VelnRuntime.taskSpawn((VelnRuntime.Fn) ignored -> {
                try {
                    FileChannel channel = FileChannel.open(path, StandardOpenOption.READ);
                    opened.set(channel);
                    entered.countDown();
                    await(complete);
                    VelnRuntime.Result result = (VelnRuntime.Result) register(owner, channel);
                    check(!result.isOk(), "late host open was published");
                    check(VelnRuntime.isAdt(VelnRuntime.recordField(result.value(), "kind"), "ErrorKind::Closed"), "late host open lost its closed-owner error");
                    return VelnRuntime.UNIT;
                } catch (java.io.IOException failure) { throw new AssertionError(failure); }
            });
            await(entered);
            Field closing = OWNER.getDeclaredField("closing");
            closing.setAccessible(true);
            synchronized (owner) { closing.setBoolean(owner, true); }
            if (cancel) VelnRuntime.taskCancel(task);
            complete.countDown();
            VelnRuntime.Result result = (VelnRuntime.Result) VelnRuntime.taskJoin(task);
            check(cancel ? !result.isOk() && (Boolean) VelnRuntime.taskJoinErrorIsCancelled(result.value()) : result.isOk(), "late producer classification changed");
            check(!opened.get().isOpen(), "late or cancelled producer stranded its actual FileChannel");
            check(((java.util.List<?>) field(owner, "resources")).isEmpty(), "late host resource was retained as a published handle");
            check(((java.util.List<?>) field(owner, "unpublished")).isEmpty(), "successful host release retained an unknown obligation");
            check(java.util.Arrays.equals(Files.readAllBytes(path), original), "late read-only open modified native bytes");
        } finally {
            complete.countDown();
            if (opened.get() != null) opened.get().close();
            for (int index = 0; index < 5; index++) VelnRuntime.popHandler();
            Files.deleteIfExists(path);
        }
    }

    public static void main(String[] args) throws Exception {
        lateOpen(false);
        lateOpen(true);
        System.out.println("late host publication and cancelled release verified");
    }
}
