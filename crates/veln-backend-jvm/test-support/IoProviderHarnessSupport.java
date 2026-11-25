class IoProviderHarnessSupport {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field OWNER_RESOURCES;
    private static final java.lang.reflect.Field ADT_PAYLOADS;
    private static final java.lang.reflect.Field CAPABILITY_HANDLER;

    static {
        try {
            Class<?> capability = Class.forName("VelnRuntime$IoCapability");
            Class<?> owner = Class.forName("VelnRuntime$IoOwner");
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            OWNER_RESOURCES = owner.getDeclaredField("resources");
            ADT_PAYLOADS = VelnRuntime.Adt.class.getDeclaredField("payloads");
            CAPABILITY_HANDLER = capability.getDeclaredField("handler");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLERS,
                INVOKED_HANDLER,
                OWNER_RESOURCES,
                ADT_PAYLOADS,
                CAPABILITY_HANDLER
            }) {
                member.setAccessible(true);
            }
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    IoProviderHarnessSupport() {}

    static void push(String effect) {
        VelnRuntime.pushHandler(effect, new Object[0], new Object[0], new Object[0]);
    }

    static void pushClose() {
        VelnRuntime.pushHandler(
            "std::io::Close",
            new Object[] { "close" },
            new Object[] { (VelnRuntime.Fn) (Object... supplied) ->
                VelnRuntime.adt("io::CloseOutcome::Closed", new Object[0]) },
            new Object[0]
        );
    }

    @SuppressWarnings("unchecked")
    static java.util.List<Object> handlers() throws ReflectiveOperationException {
        return (java.util.List<Object>) ((ThreadLocal<?>) HANDLERS.get(null)).get();
    }

    static Object owner() throws ReflectiveOperationException {
        Object scope = handlers().get(0);
        @SuppressWarnings("unchecked")
        ThreadLocal<Object> invoked = (ThreadLocal<Object>) INVOKED_HANDLER.get(null);
        invoked.set(scope);
        try {
            return VelnRuntime.ioProviderScopeOwner();
        } finally {
            invoked.remove();
        }
    }

    static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    @SuppressWarnings("unchecked")
    static java.util.Map<Long, Object> resources(Object owner) throws ReflectiveOperationException {
        return (java.util.Map<Long, Object>) OWNER_RESOURCES.get(owner);
    }

    static VelnRuntime.Result register(Object owner, long slot, String family) {
        return (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(slot),
            VelnRuntime.adt("ViewFamily::" + family, new Object[0])
        );
    }

    static void requireRegisteredIdentity(
        Object owner,
        long slot,
        Object expected,
        String label
    ) throws ReflectiveOperationException {
        require(
            resources(owner).get(Long.valueOf(slot)) == expected,
            label + " registration changed the live registry entry"
        );
    }

    static Object[] payloads(Object value) throws ReflectiveOperationException {
        return (Object[]) ADT_PAYLOADS.get(value);
    }

    static void requireCapability(Object capability, Object handler, String label)
        throws ReflectiveOperationException {
        require(CAPABILITY_HANDLER.get(capability) == handler, label + " handler was not retained");
    }
}
