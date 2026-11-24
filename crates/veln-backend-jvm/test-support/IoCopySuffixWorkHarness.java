public final class IoCopySuffixWorkHarness {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;

    static {
        try {
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            HANDLERS.setAccessible(true);
            INVOKED_HANDLER.setAccessible(true);
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    @SuppressWarnings("unchecked")
    private static Object owner() throws Exception {
        java.util.List<Object> frames =
            (java.util.List<Object>) ((ThreadLocal<?>) HANDLERS.get(null)).get();
        Object scope = frames.get(frames.size() - 1);
        ThreadLocal<Object> invoked = (ThreadLocal<Object>) INVOKED_HANDLER.get(null);
        invoked.set(scope);
        try {
            return VelnRuntime.ioProviderScopeOwner();
        } finally {
            invoked.remove();
        }
    }

    private static void push(String effect, String operation, VelnRuntime.Fn provider) {
        VelnRuntime.pushHandler(
            effect,
            new Object[] { operation },
            new Object[] { provider },
            new Object[0]
        );
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static Object count(long value) {
        return VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(value)));
    }

    private static Object chunk(String value) {
        return VelnRuntime.unwrapOk(VelnRuntime.byteChunkFromVisibleAsciiString(value));
    }

    private static String repeated(char value, int size) {
        StringBuilder result = new StringBuilder(size);
        for (int index = 0; index < size; index += 1) result.append(value);
        return result.toString();
    }

    @SuppressWarnings("unchecked")
    public static void main(String[] args) throws Exception {
        String source = repeated('a', 64);
        Object sourceChunk = chunk(source);
        java.util.concurrent.atomic.AtomicInteger readCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger writeCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicLong materialized = new java.util.concurrent.atomic.AtomicLong();
        java.util.concurrent.atomic.AtomicReference<Object> previous = new java.util.concurrent.atomic.AtomicReference<Object>();

        VelnRuntime.pushHandler("std::io_provider::Scope", new Object[0], new Object[0], new Object[0]);
        Object owner = owner();
        push("std::io::Read", "read", (Object... supplied) -> {
            if (readCalls.getAndIncrement() == 0) {
                return VelnRuntime.adt("io::ReadOutcome::Data", new Object[] { sourceChunk });
            }
            return VelnRuntime.adt("io::ReadOutcome::End", new Object[0]);
        });
        push("std::io::Write", "write", (Object... supplied) -> {
            Object bytes = supplied[1];
            int call = writeCalls.getAndIncrement();
            String actual = (String) VelnRuntime.unwrapOk(
                VelnRuntime.byteChunkToVisibleAsciiString(bytes)
            );
            require(actual.equals(source.substring(call)), "copy supplied the wrong suffix");
            Object prior = previous.getAndSet(bytes);
            if (prior != null && !VelnRuntime.byteChunksShareStorageForTest(prior, bytes)) {
                materialized.addAndGet(VelnRuntime.byteChunkStorageCapacityForTest(bytes));
            }
            return VelnRuntime.adt("io::WriteOutcome::Written", new Object[] { count(1L) });
        });
        push("std::io::Close", "close", (Object... supplied) ->
            VelnRuntime.adt("io::CloseOutcome::Closed", new Object[0])
        );

        VelnRuntime.Result registered = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(77L),
            VelnRuntime.adt("io_provider::ViewFamily::Duplex", new Object[0])
        );
        require(registered.isOk(), "copy resource registration failed");
        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderDuplex(registered.value());
        require(projected.isOk(), "copy resource projection failed");
        java.util.Map<String, Object> view = (java.util.Map<String, Object>) projected.value();

        Object outcome = VelnRuntime.ioCopy(view.get("reader"), view.get("writer"), count(64L));
        require(VelnRuntime.isAdt(outcome, "CopyOutcome::Copied"), "copy did not complete");
        require(readCalls.get() == 2 && writeCalls.get() == 64, "copy changed its bounded call sequence");
        require(materialized.get() <= 64L, "copy repeatedly materialized suffixes from the original chunk");

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        System.out.println("copy suffix materialization remained linear");
    }
}
