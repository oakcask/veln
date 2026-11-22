public final class IoProviderAppendFileHarness {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field OWNER_RESOURCES;
    private static final java.lang.reflect.Field RESOURCE_OWNER;
    private static final java.lang.reflect.Field RESOURCE_SLOT;
    private static final java.lang.reflect.Field RESOURCE_WRITE;
    private static final java.lang.reflect.Field RESOURCE_CLOSE;
    private static final java.lang.reflect.Field RESOURCE_SYNC;
    private static final java.lang.reflect.Field ADT_PAYLOADS;
    private static final java.lang.reflect.Field CAPABILITY_OWNER;
    private static final java.lang.reflect.Field CAPABILITY_SLOT;
    private static final java.lang.reflect.Field CAPABILITY_HANDLER;

    static {
        try {
            Class<?> owner = Class.forName("VelnRuntime$IoOwner");
            Class<?> resource = Class.forName("VelnRuntime$IoResource");
            Class<?> capability = Class.forName("VelnRuntime$IoCapability");
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            OWNER_RESOURCES = owner.getDeclaredField("resources");
            RESOURCE_OWNER = resource.getDeclaredField("owner");
            RESOURCE_SLOT = resource.getDeclaredField("slot");
            RESOURCE_WRITE = resource.getDeclaredField("write");
            RESOURCE_CLOSE = resource.getDeclaredField("close");
            RESOURCE_SYNC = resource.getDeclaredField("sync");
            ADT_PAYLOADS = VelnRuntime.Adt.class.getDeclaredField("payloads");
            CAPABILITY_OWNER = capability.getDeclaredField("owner");
            CAPABILITY_SLOT = capability.getDeclaredField("slot");
            CAPABILITY_HANDLER = capability.getDeclaredField("handler");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLERS,
                INVOKED_HANDLER,
                OWNER_RESOURCES,
                RESOURCE_OWNER,
                RESOURCE_SLOT,
                RESOURCE_WRITE,
                RESOURCE_CLOSE,
                RESOURCE_SYNC,
                ADT_PAYLOADS,
                CAPABILITY_OWNER,
                CAPABILITY_SLOT,
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

    private static void requireCapability(
        Object capability,
        Object owner,
        long slot,
        Object handler,
        String label
    ) throws Exception {
        require(CAPABILITY_OWNER.get(capability) == owner, label + " owner was not retained");
        require(CAPABILITY_SLOT.getLong(capability) == slot, label + " slot was not retained");
        require(CAPABILITY_HANDLER.get(capability) == handler, label + " handler was not retained");
    }

    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object owner = owner();
        java.util.Map<?, ?> resources = (java.util.Map<?, ?>) OWNER_RESOURCES.get(owner);
        Object family = VelnRuntime.adt("ViewFamily::AppendFile", new Object[0]);

        push("std::io::Write");
        Object write = handlers().get(1);
        push("std::io::Close");
        Object close = handlers().get(2);

        VelnRuntime.Result rejected = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(17L),
            family
        );
        require(!rejected.isOk(), "registration without Sync was accepted");
        require(resources.isEmpty(), "rejected registration published a resource");

        push("std::io::Sync");
        Object sync = handlers().get(3);
        VelnRuntime.Result accepted = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(17L),
            family
        );
        require(accepted.isOk(), "complete AppendFile registration failed");
        Object resource = accepted.value();
        require(resources.size() == 1, "complete registration did not publish exactly once");
        require(RESOURCE_OWNER.get(resource) == owner, "resource owner was not retained");
        require(RESOURCE_SLOT.getLong(resource) == 17L, "resource slot was not retained");
        require(RESOURCE_WRITE.get(resource) == write, "Write handler identity changed");
        require(RESOURCE_CLOSE.get(resource) == close, "Close handler identity changed");
        require(RESOURCE_SYNC.get(resource) == sync, "Sync handler identity changed");

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        require(
            !((VelnRuntime.Result) VelnRuntime.ioProviderReadFile(resource)).isOk(),
            "AppendFile resource accepted a ReadFile projection"
        );
        require(
            !((VelnRuntime.Result) VelnRuntime.ioProviderDuplex(resource)).isOk(),
            "AppendFile resource accepted a Duplex projection"
        );

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderAppendFile(resource);
        require(projected.isOk(), "AppendFile projection failed after incompatible projections");
        Object[] payloads = (Object[]) ADT_PAYLOADS.get(projected.value());
        require(payloads.length == 3, "AppendFile projection returned the wrong capability count");
        requireCapability(payloads[0], owner, 17L, write, "writer");
        requireCapability(payloads[1], owner, 17L, close, "closer");
        requireCapability(payloads[2], owner, 17L, sync, "syncer");

        VelnRuntime.popHandler();
    }
}
