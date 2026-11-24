public final class IoProviderUpdateFileHarness {
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
        @SuppressWarnings("unchecked")
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

    @SuppressWarnings("unchecked")
    private static java.util.Map<Long, Object> resources(Object owner) throws Exception {
        return (java.util.Map<Long, Object>) OWNER_RESOURCES.get(owner);
    }

    private static Object register(Object owner, long slot, String family) {
        return VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(slot),
            VelnRuntime.adt("ViewFamily::" + family, new Object[0])
        );
    }

    private static void requireRegisteredIdentity(
        Object owner,
        long slot,
        Object expected,
        String label
    ) throws Exception {
        require(
            resources(owner).get(Long.valueOf(slot)) == expected,
            label + " registration changed the live registry entry"
        );
    }

    private static void requireCapability(Object capability, Object handler, String label)
        throws Exception {
        require(CAPABILITY_HANDLER.get(capability) == handler, label + " handler was not retained");
    }

    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object inactiveOwner = owner();
        VelnRuntime.popHandler();

        push("std::io_provider::Scope");
        Object owner = owner();

        VelnRuntime.Result inactive = (VelnRuntime.Result) register(inactiveOwner, 5L, "UpdateFile");
        require(inactive.toString().contains("InvalidInput"), "inactive owner resolved operation handlers");
        require(resources(inactiveOwner).isEmpty(), "inactive owner published an UpdateFile resource");

        VelnRuntime.Result negative = (VelnRuntime.Result) register(owner, -1L, "UpdateFile");
        require(!negative.isOk(), "negative UpdateFile slot was accepted without handlers");
        require(negative.toString().contains("InvalidInput"), "negative slot did not validate before handlers");
        require(resources(owner).isEmpty(), "negative UpdateFile slot published a resource");

        push("std::io::Read");
        Object read = handlers().get(1);
        push("std::io::Write");
        Object write = handlers().get(2);
        push("std::io::Seek");
        Object seek = handlers().get(3);
        push("std::io::ReadAt");
        Object readAt = handlers().get(4);
        push("std::io::WriteAt");
        Object writeAt = handlers().get(5);
        pushClose();
        Object close = handlers().get(6);
        push("std::io::Sync");
        Object sync = handlers().get(7);

        VelnRuntime.Result missing = (VelnRuntime.Result) register(owner, 31L, "UpdateFile");
        require(!missing.isOk(), "UpdateFile registration without fs::Mutate was accepted");
        require(missing.toString().contains("Unsupported"), "missing handler did not report Unsupported");
        require(!resources(owner).containsKey(Long.valueOf(31L)), "incomplete UpdateFile was published");

        push("std::fs::Mutate");
        Object mutate = handlers().get(8);
        VelnRuntime.Result accepted = (VelnRuntime.Result) register(owner, 31L, "UpdateFile");
        require(accepted.isOk(), "complete UpdateFile registration failed");
        Object resource = accepted.value();

        register(owner, 31L, "UpdateFile");
        requireRegisteredIdentity(owner, 31L, resource, "same-family duplicate UpdateFile");
        for (String family : new String[] {"ReadFile", "WriteFile", "AppendFile"}) {
            register(owner, 31L, family);
            requireRegisteredIdentity(owner, 31L, resource, "incoming " + family + " duplicate");
        }

        long existingSlot = 41L;
        for (String family : new String[] {"ReadFile", "WriteFile", "AppendFile"}) {
            VelnRuntime.Result existing = (VelnRuntime.Result) register(owner, existingSlot, family);
            require(existing.isOk(), family + " registration failed before reverse duplicate check");
            Object existingResource = existing.value();
            register(owner, existingSlot, "UpdateFile");
            requireRegisteredIdentity(owner, existingSlot, existingResource, "incoming UpdateFile duplicate");
            existingSlot += 1L;
        }

        for (int index = 0; index < 8; index++) VelnRuntime.popHandler();
        VelnRuntime.Result duplicate = (VelnRuntime.Result) register(owner, 31L, "UpdateFile");
        require(duplicate.toString().contains("InvalidInput"), "duplicate slot resolved missing handlers");
        requireRegisteredIdentity(owner, 31L, resource, "handler-free duplicate UpdateFile");

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderUpdateFile(resource);
        require(projected.isOk(), "UpdateFile projection failed after handler scopes exited");
        Object[] payloads = (Object[]) ADT_PAYLOADS.get(projected.value());
        require(payloads.length == 8, "UpdateFile projection returned the wrong capability count");
        requireCapability(payloads[0], read, "reader");
        requireCapability(payloads[1], write, "writer");
        requireCapability(payloads[2], seek, "seeker");
        requireCapability(payloads[3], readAt, "positional reader");
        requireCapability(payloads[4], writeAt, "positional writer");
        requireCapability(payloads[5], close, "closer");
        requireCapability(payloads[6], sync, "syncer");
        requireCapability(payloads[7], mutate, "truncater");

        VelnRuntime.popHandler();
    }
}
