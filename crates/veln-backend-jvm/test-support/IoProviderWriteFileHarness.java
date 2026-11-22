public final class IoProviderWriteFileHarness {
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

    private static void requireCapability(Object capability, Object handler, String label)
        throws Exception {
        require(CAPABILITY_HANDLER.get(capability) == handler, label + " handler was not retained");
    }

    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object owner = owner();
        Object family = VelnRuntime.adt("ViewFamily::WriteFile", new Object[0]);

        push("std::io::Write");
        Object write = handlers().get(1);
        push("std::io::Seek");
        Object seek = handlers().get(2);
        push("std::io::WriteAt");
        Object writeAt = handlers().get(3);
        push("std::io::Close");
        Object close = handlers().get(4);
        push("std::io::Sync");
        Object sync = handlers().get(5);
        push("std::fs::Mutate");
        Object mutate = handlers().get(6);

        VelnRuntime.Result accepted = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(23L),
            family
        );
        require(accepted.isOk(), "WriteFile registration failed before handler identity check");
        Object resource = accepted.value();

        for (int index = 0; index < 6; index++) VelnRuntime.popHandler();

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderWriteFile(resource);
        require(projected.isOk(), "WriteFile projection failed before handler identity check");
        Object[] payloads = (Object[]) ADT_PAYLOADS.get(projected.value());
        requireCapability(payloads[0], write, "writer");
        requireCapability(payloads[1], seek, "seeker");
        requireCapability(payloads[2], writeAt, "positional writer");
        requireCapability(payloads[3], close, "closer");
        requireCapability(payloads[4], sync, "syncer");
        requireCapability(payloads[5], mutate, "truncater");

        VelnRuntime.popHandler();
    }
}
