import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Map;

// Real-host translation contract: actual interruptible FileChannels establish
// progress uncertainty and whole-open closure. No Java fake filesystem or
// alternate production handler is installed. Reflection reaches the private
// host adapter without adding a source-visible test hook.
public final class IoProviderHostTransferHarness {
    private static final Class<?> OWNER;
    private static final Class<?> RESOURCE;
    private static final Class<?> FILE;
    private static final Method REQUEST;

    static {
        try {
            OWNER = Class.forName("VelnRuntime$IoProviderOwner");
            RESOURCE = Class.forName("VelnRuntime$IoProviderResource");
            FILE = Class.forName("VelnRuntime$IoHostFile");
            REQUEST = VelnRuntime.class.getDeclaredMethod("ioHostFileRequest", RESOURCE, String.class, Object.class);
            REQUEST.setAccessible(true);
        } catch (ReflectiveOperationException failure) { throw new ExceptionInInitializerError(failure); }
    }

    private static Object construct(Class<?> type, Class<?>[] signature, Object... arguments) throws Exception {
        Constructor<?> constructor = type.getDeclaredConstructor(signature);
        constructor.setAccessible(true);
        return constructor.newInstance(arguments);
    }

    private static Object field(Object instance, String name) throws Exception {
        Field field = instance.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(instance);
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void interrupted(String operation) throws Exception {
        Path path = Files.createTempFile("veln-host-transfer-", ".bin");
        byte[] original = new byte[] {97, 98, 99, 100};
        Files.write(path, original);
        try (FileChannel channel = FileChannel.open(path, StandardOpenOption.READ, StandardOpenOption.WRITE)) {
            Object portable = VelnRuntime.adt("Path::PortablePath", new Object[] {"data"});
            Object file = construct(FILE, new Class<?>[] {FileChannel.class, Object.class, boolean.class}, channel, portable, false);
            Field cursor = FILE.getDeclaredField("cursor");
            cursor.setAccessible(true);
            cursor.setLong(file, 1L);
            Object owner = construct(OWNER, new Class<?>[0]);
            Object resource = construct(RESOURCE, new Class<?>[] {OWNER, long.class, String.class, Map.class, Object.class}, owner, 1L, "UpdateFile", Map.of(), file);
            Object one = VelnRuntime.adt("ByteCount", new Object[] {1L});
            Object two = VelnRuntime.adt("ByteOffset", new Object[] {2L});
            VelnRuntime.Result encoded = (VelnRuntime.Result) VelnRuntime.byteChunkFromVisibleAsciiString("XY");
            Map<String, Object> request = operation.equals("write")
                ? Map.of("bytes", encoded.value())
                : operation.equals("read_at") ? Map.of("offset", two, "maximum", one) : Map.of("maximum", one);
            Object outcome;
            Thread.currentThread().interrupt();
            try { outcome = REQUEST.invoke(null, resource, operation, request); }
            finally { Thread.interrupted(); }
            boolean write = operation.equals("write");
            check(VelnRuntime.isAdt(outcome, write ? "WriteOutcome::WriteFailed" : "ReadOutcome::ReadFailed"), "interrupt lost its typed transfer outcome");
            Object progress = VelnRuntime.adtPayload(outcome, write ? 0 : 1);
            check(VelnRuntime.isAdt(progress, "Progress::Unknown"), "host exception fabricated exact progress");
            check(VelnRuntime.adtPayload(VelnRuntime.adtPayload(progress, 0), 0).equals(0L), "unproved host progress became a known prefix");
            Object error = VelnRuntime.adtPayload(outcome, write ? 1 : 2);
            check(VelnRuntime.isAdt(VelnRuntime.recordField(error, "kind"), "ErrorKind::Cancelled"), "typed interruption category lost");
            check(!channel.isOpen() && Boolean.TRUE.equals(field(resource, "closed")), "interruption did not invalidate the whole open");
            check(cursor.getLong(file) == 1L, "failed positional or sequential operation fabricated a cursor");
            check(field(file, "cursorKnown").equals(operation.equals("read_at")), "positional failure changed cursor knowledge or sequential failure kept it known");
            check(java.util.Arrays.equals(Files.readAllBytes(path), original), "interrupted host call changed the independently observed input");
        } finally { Files.deleteIfExists(path); }
    }

    public static void main(String[] args) throws Exception {
        interrupted("read");
        interrupted("read_at");
        interrupted("write");
        System.out.println("host interruption progress and closure verified");
    }
}
