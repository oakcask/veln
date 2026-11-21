import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.channels.FileChannel;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.util.Map;

// Unix host contract: a real failed native close can leave its descriptor live
// after AbstractInterruptibleChannel has marked the FileChannel closed. No fake
// filesystem or alternate handler is installed. The saved descriptor owns the
// real file until finally; only this channel's descriptor field is invalidated.
public final class IoProviderHostCloseFailureHarness {
    private static Object construct(Class<?> type, Class<?>[] signature, Object... args) throws Exception {
        Constructor<?> constructor = type.getDeclaredConstructor(signature);
        constructor.setAccessible(true);
        return constructor.newInstance(args);
    }

    private static Object field(Object value, String name) throws Exception {
        Field field = value.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(value);
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void unknown(Object outcome) {
        check(VelnRuntime.isAdt(outcome, "CloseOutcome::CloseFailed"), "native close failure lost its typed outcome");
        check(VelnRuntime.isAdt(VelnRuntime.adtPayload(outcome, 0), "Commit::Unknown"), "logical closure fabricated proven native release");
        Object error = VelnRuntime.adtPayload(outcome, 1);
        check(VelnRuntime.recordField(error, "operation").equals("close"), "close operation context lost");
        check(VelnRuntime.isSome(VelnRuntime.recordField(error, "path")), "close portable path context lost");
    }

    private static void cursorRecovery(String operation) throws Exception {
        Path path = Files.createTempFile("veln-host-cursor-failure-", ".bin");
        Files.write(path, new byte[] {97, 98, 99, 100});
        try (FileChannel channel = FileChannel.open(path, StandardOpenOption.READ, StandardOpenOption.WRITE)) {
            FileDescriptor descriptor = (FileDescriptor) field(channel, "fd");
            Field number = FileDescriptor.class.getDeclaredField("fd");
            number.setAccessible(true);
            int saved = number.getInt(descriptor);
            Class<?> ownerType = Class.forName("VelnRuntime$IoProviderOwner");
            Class<?> resourceType = Class.forName("VelnRuntime$IoProviderResource");
            Class<?> fileType = Class.forName("VelnRuntime$IoHostFile");
            Object file = construct(fileType, new Class<?>[] {FileChannel.class, Object.class, boolean.class}, channel, VelnRuntime.adt("Path::PortablePath", new Object[] {"data"}), false);
            Field cursor = fileType.getDeclaredField("cursor");
            cursor.setAccessible(true);
            cursor.setLong(file, 1L);
            Object resource = construct(resourceType, new Class<?>[] {ownerType, long.class, String.class, Map.class, Object.class}, construct(ownerType, new Class<?>[0]), 1L, "UpdateFile", Map.of(), file);
            Method method = VelnRuntime.class.getDeclaredMethod("ioHostFileRequest", resourceType, String.class, Object.class);
            method.setAccessible(true);
            Object one = VelnRuntime.adt("ByteCount", new Object[] {1L});
            Object offset = VelnRuntime.adt("ByteOffset", new Object[] {2L});
            Object bytes = ((VelnRuntime.Result) VelnRuntime.byteChunkFromVisibleAsciiString("Z")).value();
            boolean read = operation.startsWith("read");
            boolean positional = operation.endsWith("_at");
            Map<String, Object> request = read ? Map.of("maximum", one, "offset", offset) : Map.of("bytes", bytes, "offset", offset);
            try {
                number.setInt(descriptor, -2);
                // These failures happen before any position/namespace change.
                for (String action : new String[] {"sync", "seek", "truncate"}) {
                    Object reply = method.invoke(null, resource, action, Map.of("origin", VelnRuntime.adt("SeekOrigin::End", new Object[] {0L}), "size", offset));
                    if (action.equals("truncate")) {
                        check(VelnRuntime.isAdt(reply, "MutationOutcome::MutationFailed") && VelnRuntime.isAdt(VelnRuntime.adtPayload(reply, 0), "Commit::No"), "size-query failure fabricated a truncate commit");
                    } else check(!((VelnRuntime.Result) reply).isOk(), "native seek/sync error became success");
                    check(cursor.getLong(file) == 1L && Boolean.TRUE.equals(field(file, "cursorKnown")), "failed seek/sync/size-query changed cursor facts");
                }
                Object failure = method.invoke(null, resource, operation, request);
                check(VelnRuntime.isAdt(failure, read ? "ReadOutcome::ReadFailed" : "WriteOutcome::WriteFailed"), "native transfer error lost its typed outcome");
                Object progress = VelnRuntime.adtPayload(failure, read ? 1 : 0);
                check(VelnRuntime.isAdt(progress, "Progress::Unknown") && VelnRuntime.adtPayload(VelnRuntime.adtPayload(progress, 0), 0).equals(0L), "native error fabricated exact progress");
                check(channel.isOpen() && Boolean.FALSE.equals(field(resource, "unavailable")), "ordinary native error invalidated a still-live channel");
                check(cursor.getLong(file) == 1L && field(file, "cursorKnown").equals(positional), "positional failure changed cursor knowledge or sequential failure kept it known");
            } finally {
                number.setInt(descriptor, saved);
                if (!channel.isOpen()) {
                    FileDescriptor stranded = new FileDescriptor();
                    number.setInt(stranded, saved);
                    try (FileInputStream cleanup = new FileInputStream(stranded)) { }
                    number.setInt(descriptor, -1);
                }
            }
            check(java.util.Arrays.equals(Files.readAllBytes(path), new byte[] {97, 98, 99, 100}), "failed native write changed independently observed bytes");
            if (!positional) {
                for (String origin : new String[] {"Current", "End"}) {
                    VelnRuntime.Result seek = (VelnRuntime.Result) method.invoke(null, resource, "seek", Map.of("origin", VelnRuntime.adt("SeekOrigin::" + origin, new Object[] {0L})));
                    check(!seek.isOk() && VelnRuntime.isAdt(VelnRuntime.recordField(seek.value(), "kind"), "ErrorKind::InvalidResource"), "relative seek fabricated a known cursor");
                }
                Object blocked = method.invoke(null, resource, "read", Map.of("maximum", one));
                check(VelnRuntime.isAdt(blocked, "ReadOutcome::ReadFailed")
                    && VelnRuntime.isAdt(VelnRuntime.adtPayload(blocked, 1), "Progress::Known")
                    && VelnRuntime.adtPayload(VelnRuntime.adtPayload(VelnRuntime.adtPayload(blocked, 1), 0), 0).equals(0L)
                    && VelnRuntime.isAdt(VelnRuntime.recordField(VelnRuntime.adtPayload(blocked, 2), "kind"), "ErrorKind::InvalidResource"), "unknown sequential cursor admitted another transfer");
                Object at = method.invoke(null, resource, "read_at", Map.of("maximum", one, "offset", offset));
                check(VelnRuntime.isAdt(at, "ReadOutcome::Data") && VelnRuntime.format(VelnRuntime.byteChunkToVisibleAsciiString(VelnRuntime.adtPayload(at, 0))).equals("Ok(c)"), "positional recovery read selected the wrong bytes");
                check(Boolean.FALSE.equals(field(file, "cursorKnown")) && cursor.getLong(file) == 1L, "positional success restored an unknown sequential cursor");
                Object zero = VelnRuntime.adt("ByteOffset", new Object[] {0L});
                Object written = method.invoke(null, resource, "write_at", Map.of("bytes", bytes, "offset", zero));
                check(VelnRuntime.isAdt(written, "WriteOutcome::Written") && VelnRuntime.adtPayload(VelnRuntime.adtPayload(written, 0), 0).equals(1L), "positional recovery write lost its committed count");
                check(Boolean.FALSE.equals(field(file, "cursorKnown")) && cursor.getLong(file) == 1L, "positional write restored an unknown sequential cursor");
                check(java.util.Arrays.equals(Files.readAllBytes(path), new byte[] {90, 98, 99, 100}), "positional recovery write changed the wrong native bytes");
                VelnRuntime.Result seek = (VelnRuntime.Result) method.invoke(null, resource, "seek", Map.of("origin", VelnRuntime.adt("SeekOrigin::Start", new Object[] {offset})));
                check(seek.isOk() && Boolean.TRUE.equals(field(file, "cursorKnown")), "absolute seek did not restore cursor knowledge");
            }
            Object sequential = method.invoke(null, resource, "read", Map.of("maximum", one));
            String expected = positional ? "Ok(b)" : "Ok(c)";
            check(VelnRuntime.isAdt(sequential, "ReadOutcome::Data") && VelnRuntime.format(VelnRuntime.byteChunkToVisibleAsciiString(VelnRuntime.adtPayload(sequential, 0))).equals(expected), "recovered sequential read selected the wrong bytes");
            check(cursor.getLong(file) == (positional ? 2L : 3L), "recovered sequential cursor advanced incorrectly");
        } finally { Files.deleteIfExists(path); }
    }

    public static void main(String[] args) throws Exception {
        Path path = Files.createTempFile("veln-host-close-failure-", ".bin");
        Files.write(path, new byte[] {97, 98, 99, 100});
        FileChannel channel = FileChannel.open(path, StandardOpenOption.READ);
        FileDescriptor original = (FileDescriptor) field(channel, "fd");
        Field number = FileDescriptor.class.getDeclaredField("fd");
        number.setAccessible(true);
        int descriptor = number.getInt(original);
        check(descriptor > 2, "test must own an ordinary native file descriptor");
        FileDescriptor saved = new FileDescriptor();
        number.setInt(saved, descriptor);
        try (FileInputStream retained = new FileInputStream(saved)) {
            Class<?> ownerType = Class.forName("VelnRuntime$IoProviderOwner");
            Class<?> resourceType = Class.forName("VelnRuntime$IoProviderResource");
            Class<?> fileType = Class.forName("VelnRuntime$IoHostFile");
            Object owner = construct(ownerType, new Class<?>[0]);
            Object portable = VelnRuntime.adt("Path::PortablePath", new Object[] {"data"});
            Object file = construct(fileType, new Class<?>[] {FileChannel.class, Object.class, boolean.class}, channel, portable, false);
            Object resource = construct(resourceType, new Class<?>[] {ownerType, long.class, String.class, Map.class, Object.class}, owner, 1L, "ReadFile", Map.of(), file);
            Method request = VelnRuntime.class.getDeclaredMethod("ioHostFileRequest", resourceType, String.class, Object.class);
            request.setAccessible(true);
            // The actual descriptor remains open through retained. close(-2)
            // fails in the real host, without closing any unrelated descriptor.
            number.setInt(original, -2);
            Object failed = request.invoke(null, resource, "close", VelnRuntime.UNIT);
            check(!channel.isOpen(), "host did not establish logical channel closure");
            check(retained.read() == 97, "the independent descriptor did not remain live after failed close");
            unknown(failed);
            check(Boolean.FALSE.equals(field(resource, "closed")), "unproved native release became a confirmed closed resource");
            check(Boolean.TRUE.equals(field(resource, "unavailable")), "logical closure did not block every transfer alias");
            unknown(request.invoke(null, resource, "close", VelnRuntime.UNIT));
            check(retained.read() == 98, "retry fabricated release of the still-live descriptor");
            Class<?> viewType = Class.forName("VelnRuntime$IoProviderView");
            Object reader = construct(viewType, new Class<?>[] {resourceType, String.class}, resource, "reader");
            Object read = VelnRuntime.ioProviderDispatchRead(reader, VelnRuntime.adt("ByteCount", new Object[] {1L}));
            check(VelnRuntime.isAdt(read, "ReadOutcome::ReadFailed"), "logical closed alias admitted a read");
            check(VelnRuntime.isAdt(VelnRuntime.recordField(VelnRuntime.adtPayload(read, 2), "kind"), "ErrorKind::Closed"), "logical closed alias lost its admission error");
        } finally {
            channel.close();
            Files.deleteIfExists(path);
        }
        for (String operation : new String[] {"read", "write", "read_at", "write_at"}) cursorRecovery(operation);
        System.out.println("native close failure preserves unknown release and blocked aliases");
    }
}
