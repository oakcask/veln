public final class IoProviderDispatchHarness {
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
    private static java.util.List<Object> handlers() throws Exception {
        return (java.util.List<Object>) ((ThreadLocal<?>) HANDLERS.get(null)).get();
    }

    @SuppressWarnings("unchecked")
    private static Object owner() throws Exception {
        java.util.List<Object> frames = handlers();
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

    private static void pushEmpty(String effect) {
        VelnRuntime.pushHandler(effect, new Object[0], new Object[0], new Object[0]);
    }

    private static VelnRuntime.Result result(Object value) {
        return (VelnRuntime.Result) value;
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) throws Exception {
        java.util.concurrent.atomic.AtomicInteger namespaceCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger byteCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger fallbackCalls = new java.util.concurrent.atomic.AtomicInteger();

        pushEmpty("std::io_provider::Scope");
        Object owner = owner();

        push("std::fs::Read", "open_read", (Object... supplied) -> {
            require(
                supplied.length == 2 && supplied[0].getClass().getName().endsWith("$IoFilesystem"),
                "namespace dispatch exposed a resource instead of the restricted filesystem token"
            );
            namespaceCalls.incrementAndGet();
            return "captured-namespace";
        });
        VelnRuntime.Result filesystemResult = result(
            VelnRuntime.ioProviderFilesystem(owner, Long.valueOf(7L))
        );
        require(filesystemResult.isOk(), "filesystem token allocation failed");
        Object filesystem = filesystemResult.value();
        VelnRuntime.popHandler();

        push("std::fs::Read", "open_read", (Object... supplied) -> {
            fallbackCalls.incrementAndGet();
            return "fallback-namespace";
        });
        Object foreignNamespace = VelnRuntime.perform(
            "std::fs::Read",
            "open_read",
            new Object[] { filesystem, "path" }
        );
        require(
            foreignNamespace.toString().contains("InvalidResource"),
            "foreign namespace token was not rejected before lexical dispatch"
        );
        require(namespaceCalls.get() == 0 && fallbackCalls.get() == 0, "foreign namespace dispatch invoked a provider");
        Object namespaceResult = VelnRuntime.ioProviderDispatch2(
            "std::fs::Read",
            "open_read",
            filesystem,
            "path"
        );
        require(namespaceResult.equals("captured-namespace"), "namespace dispatch did not select the captured handler");
        require(namespaceCalls.get() == 1 && fallbackCalls.get() == 0, "namespace dispatch used the active fallback handler");
        VelnRuntime.popHandler();

        push("std::io::Read", "read", (Object... supplied) -> {
            require(
                supplied.length == 2 && supplied[0].getClass().getName().endsWith("$IoCapability"),
                "byte dispatch exposed a resource instead of the restricted capability"
            );
            byteCalls.incrementAndGet();
            return "captured-read";
        });
        push("std::io::Seek", "seek", (Object... supplied) -> "seek");
        push("std::io::ReadAt", "read_at", (Object... supplied) -> "read-at");
        push("std::io::Close", "close", (Object... supplied) -> "close");
        Object family = VelnRuntime.adt(
            "io_provider::ViewFamily::ReadFile",
            new Object[0]
        );
        VelnRuntime.Result resourceResult = result(
            VelnRuntime.ioProviderResource(owner, Long.valueOf(8L), family)
        );
        require(resourceResult.isOk(), "read resource registration failed");
        VelnRuntime.Result projectedResult = result(
            VelnRuntime.ioProviderReadFile(resourceResult.value())
        );
        require(projectedResult.isOk(), "read resource projection failed");
        Object reader = VelnRuntime.adtPayload(projectedResult.value(), 0);
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        push("std::io::Read", "read", (Object... supplied) -> {
            fallbackCalls.incrementAndGet();
            return "fallback-read";
        });
        Object foreignReader = VelnRuntime.perform(
            "std::io::Read",
            "read",
            new Object[] { reader, Long.valueOf(1L) }
        );
        require(
            foreignReader.toString().contains("InvalidResource"),
            "foreign byte capability was not rejected before lexical dispatch"
        );
        require(byteCalls.get() == 0 && fallbackCalls.get() == 0, "foreign byte dispatch invoked a provider");
        Object byteResult = VelnRuntime.ioProviderDispatch2(
            "std::io::Read",
            "read",
            reader,
            Long.valueOf(1L)
        );
        require(byteResult.equals("captured-read"), "byte dispatch did not select the captured handler");
        require(byteCalls.get() == 1 && fallbackCalls.get() == 0, "byte dispatch used the active host fallback sentinel");
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
    }
}
