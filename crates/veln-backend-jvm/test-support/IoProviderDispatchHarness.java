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

    private static boolean slotEquals(Object value, long expected) {
        VelnRuntime.Result result = result(value);
        return result.isOk() && result.value().equals(Long.valueOf(expected));
    }

    private static boolean isInvalidResource(Object value) {
        VelnRuntime.Result result = result(value);
        return !result.isOk() && result.toString().contains("InvalidResource");
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

        java.util.concurrent.atomic.AtomicReference<Object> ownerA = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> ownerB = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> filesystemA = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> filesystemB = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> readerA = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> readerB = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicInteger nestedMode = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger sharedNamespaceCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger sharedByteCalls = new java.util.concurrent.atomic.AtomicInteger();

        push("std::fs::Read", "open_read", (Object... supplied) -> {
            Object operand = supplied[0];
            sharedNamespaceCalls.incrementAndGet();
            if (operand == filesystemB.get()) {
                require(
                    slotEquals(VelnRuntime.ioProviderFilesystemSlot(ownerB.get(), operand), 21L),
                    "shared namespace handler did not receive owner B invocation context"
                );
                require(
                    isInvalidResource(VelnRuntime.ioProviderFilesystemSlot(ownerA.get(), operand)),
                    "shared namespace handler accepted owner A for owner B's token"
                );
                if (nestedMode.get() == 1) return VelnRuntime.err("propagated");
                if (nestedMode.get() == 2) throw new VelnRuntime.RuntimeFailure("planned provider failure");
                return "shared-b";
            }
            require(operand == filesystemA.get(), "shared namespace handler received an unknown token");
            require(
                slotEquals(VelnRuntime.ioProviderFilesystemSlot(ownerA.get(), operand), 11L),
                "shared namespace handler did not receive owner A invocation context"
            );
            Object nested;
            try {
                nested = VelnRuntime.ioProviderDispatch2(
                    "std::fs::Read",
                    "open_read",
                    filesystemB.get(),
                    "nested"
                );
            } catch (VelnRuntime.RuntimeFailure failure) {
                require(
                    nestedMode.get() == 2 && failure.getMessage().contains("planned provider failure"),
                    "unexpected nested provider runtime failure"
                );
                nested = "runtime-failure";
            }
            require(
                slotEquals(VelnRuntime.ioProviderFilesystemSlot(ownerA.get(), operand), 11L),
                "nested namespace dispatch did not restore owner A invocation context"
            );
            return nested;
        });
        push("std::io::Read", "read", (Object... supplied) -> {
            Object operand = supplied[0];
            sharedByteCalls.incrementAndGet();
            if (operand == readerA.get()) {
                require(
                    slotEquals(VelnRuntime.ioProviderReaderSlot(ownerA.get(), operand), 12L),
                    "shared byte handler did not receive owner A invocation context"
                );
                return "reader-a";
            }
            require(operand == readerB.get(), "shared byte handler received an unknown capability");
            require(
                slotEquals(VelnRuntime.ioProviderReaderSlot(ownerB.get(), operand), 22L),
                "shared byte handler did not receive owner B invocation context"
            );
            return "reader-b";
        });
        push("std::io::Seek", "seek", (Object... supplied) -> "seek");
        push("std::io::ReadAt", "read_at", (Object... supplied) -> "read-at");
        push("std::io::Close", "close", (Object... supplied) -> "close");

        pushEmpty("std::io_provider::Scope");
        ownerA.set(owner());
        VelnRuntime.Result sharedFilesystemA = result(
            VelnRuntime.ioProviderFilesystem(ownerA.get(), Long.valueOf(11L))
        );
        require(sharedFilesystemA.isOk(), "owner A could not register through the shared namespace handler");
        filesystemA.set(sharedFilesystemA.value());
        VelnRuntime.Result sharedResourceA = result(
            VelnRuntime.ioProviderResource(ownerA.get(), Long.valueOf(12L), family)
        );
        require(sharedResourceA.isOk(), "owner A could not register through the shared byte handlers");
        readerA.set(VelnRuntime.adtPayload(result(VelnRuntime.ioProviderReadFile(sharedResourceA.value())).value(), 0));

        pushEmpty("std::io_provider::Scope");
        ownerB.set(owner());
        VelnRuntime.Result sharedFilesystemB = result(
            VelnRuntime.ioProviderFilesystem(ownerB.get(), Long.valueOf(21L))
        );
        require(sharedFilesystemB.isOk(), "owner B could not reuse the shared namespace handler");
        filesystemB.set(sharedFilesystemB.value());
        VelnRuntime.Result sharedResourceB = result(
            VelnRuntime.ioProviderResource(ownerB.get(), Long.valueOf(22L), family)
        );
        require(sharedResourceB.isOk(), "owner B could not reuse the shared byte handlers");
        readerB.set(VelnRuntime.adtPayload(result(VelnRuntime.ioProviderReadFile(sharedResourceB.value())).value(), 0));

        nestedMode.set(0);
        require(
            VelnRuntime.ioProviderDispatch2("std::fs::Read", "open_read", filesystemA.get(), "path").equals("shared-b"),
            "normal nested dispatch did not preserve the shared-frame owners"
        );
        nestedMode.set(1);
        Object propagated = VelnRuntime.ioProviderDispatch2("std::fs::Read", "open_read", filesystemA.get(), "path");
        require(!result(propagated).isOk(), "nested propagated failure was not returned");
        nestedMode.set(2);
        require(
            VelnRuntime.ioProviderDispatch2("std::fs::Read", "open_read", filesystemA.get(), "path").equals("runtime-failure"),
            "nested runtime failure did not restore the outer invocation owner"
        );
        require(
            isInvalidResource(VelnRuntime.ioProviderFilesystemSlot(ownerA.get(), filesystemA.get())),
            "captured namespace dispatch leaked owner A into ordinary lexical execution"
        );
        require(
            VelnRuntime.ioProviderDispatch2("std::io::Read", "read", readerA.get(), Long.valueOf(1L)).equals("reader-a")
                && VelnRuntime.ioProviderDispatch2("std::io::Read", "read", readerB.get(), Long.valueOf(1L)).equals("reader-b"),
            "shared byte handler did not dispatch both owners"
        );

        int namespaceBeforeForeign = sharedNamespaceCalls.get();
        int byteBeforeForeign = sharedByteCalls.get();
        require(
            VelnRuntime.perform("std::fs::Read", "open_read", new Object[] { filesystemA.get(), "path" })
                .toString().contains("InvalidResource"),
            "shared-frame foreign namespace token was not rejected"
        );
        require(
            VelnRuntime.perform("std::io::Read", "read", new Object[] { readerA.get(), Long.valueOf(1L) })
                .toString().contains("InvalidResource"),
            "shared-frame foreign byte capability was not rejected"
        );
        require(
            sharedNamespaceCalls.get() == namespaceBeforeForeign && sharedByteCalls.get() == byteBeforeForeign,
            "shared-frame foreign operands reached a provider"
        );

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
    }
}
