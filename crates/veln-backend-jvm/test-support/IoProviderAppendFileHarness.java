public final class IoProviderAppendFileHarness {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field ADT_PAYLOADS;
    private static final java.lang.reflect.Field CAPABILITY_HANDLER;

    static {
        try {
            Class<?> capability = Class.forName("VelnRuntime$IoCapability");
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            ADT_PAYLOADS = VelnRuntime.Adt.class.getDeclaredField("payloads");
            CAPABILITY_HANDLER = capability.getDeclaredField("handler");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLERS,
                INVOKED_HANDLER,
                ADT_PAYLOADS,
                CAPABILITY_HANDLER
            }) {
                member.setAccessible(true);
            }
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    private static void push(String effect) {
        VelnRuntime.pushHandler(effect, new Object[0], new Object[0], new Object[0]);
    }

    private static void pushClose() {
        VelnRuntime.pushHandler(
            "std::io::Close",
            new Object[] { "close" },
            new Object[] { (VelnRuntime.Fn) (Object... supplied) ->
                VelnRuntime.adt("io::CloseOutcome::Closed", new Object[0]) },
            new Object[0]
        );
    }

    @SuppressWarnings("unchecked")
    private static java.util.List<Object> handlers() throws Exception {
        return (java.util.List<Object>) ((ThreadLocal<?>) HANDLERS.get(null)).get();
    }

    private static Object owner() throws Exception {
        Object scope = handlers().get(0);
        ThreadLocal<Object> invoked = (ThreadLocal<Object>) INVOKED_HANDLER.get(null);
        invoked.set(scope);
        try {
            return VelnRuntime.ioProviderScopeOwner();
        } finally {
            invoked.remove();
        }
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void requireCapability(
        Object capability,
        Object handler,
        String label
    ) throws Exception {
        require(CAPABILITY_HANDLER.get(capability) == handler, label + " handler was not retained");
    }

    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object owner = owner();
        Object family = VelnRuntime.adt("ViewFamily::AppendFile", new Object[0]);

        push("std::io::Write");
        Object write = handlers().get(1);
        pushClose();
        Object close = handlers().get(2);
        push("std::io::Sync");
        Object sync = handlers().get(3);
        VelnRuntime.Result accepted = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(17L),
            family
        );
        require(accepted.isOk(), "AppendFile registration failed before handler identity check");
        Object resource = accepted.value();

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderAppendFile(resource);
        require(projected.isOk(), "AppendFile projection failed before handler identity check");
        Object[] payloads = (Object[]) ADT_PAYLOADS.get(projected.value());
        require(payloads.length == 3, "AppendFile projection returned the wrong capability count");
        requireCapability(payloads[0], write, "writer");
        requireCapability(payloads[1], close, "closer");
        requireCapability(payloads[2], sync, "syncer");

        VelnRuntime.popHandler();
    }
}
