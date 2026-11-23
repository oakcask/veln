public final class IoProviderDuplexHarness {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field OWNER_RESOURCES;
    private static final java.lang.reflect.Field CAPABILITY_HANDLER;

    static {
        try {
            Class<?> capability = Class.forName("VelnRuntime$IoCapability");
            Class<?> owner = Class.forName("VelnRuntime$IoOwner");
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            OWNER_RESOURCES = owner.getDeclaredField("resources");
            CAPABILITY_HANDLER = capability.getDeclaredField("handler");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLERS,
                INVOKED_HANDLER,
                OWNER_RESOURCES,
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

    private static VelnRuntime.Result register(Object owner, long slot, String family) {
        return (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(slot),
            VelnRuntime.adt("ViewFamily::" + family, new Object[0])
        );
    }

    private static void requireInvalidDuplicate(
        Object owner,
        long slot,
        String family,
        Object original
    ) throws Exception {
        VelnRuntime.Result duplicate = register(owner, slot, family);
        require(!duplicate.isOk(), family + " duplicate registration succeeded");
        require(duplicate.toString().contains("InvalidInput"), family + " duplicate was not InvalidInput");
        require(resources(owner).get(Long.valueOf(slot)) == original, family + " duplicate replaced the resource");
    }

    private static void requireMissing(Object owner, long slot, String label) throws Exception {
        VelnRuntime.Result missing = register(owner, slot, "Duplex");
        require(!missing.isOk(), "Duplex registration without " + label + " was accepted");
        require(missing.toString().contains("Unsupported"), "missing " + label + " did not report Unsupported");
        require(!resources(owner).containsKey(Long.valueOf(slot)), "missing " + label + " published a resource");
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

        VelnRuntime.Result inactive = register(inactiveOwner, 5L, "Duplex");
        require(inactive.toString().contains("InvalidInput"), "inactive owner resolved Duplex handlers");
        require(resources(inactiveOwner).isEmpty(), "inactive owner published a Duplex resource");

        VelnRuntime.Result negative = register(owner, -1L, "Duplex");
        require(negative.toString().contains("InvalidInput"), "negative Duplex slot resolved handlers");
        require(resources(owner).isEmpty(), "negative Duplex slot published a resource");

        push("std::io::Write");
        push("std::io::Close");
        requireMissing(owner, 21L, "io::Read");
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        push("std::io::Read");
        push("std::io::Close");
        requireMissing(owner, 22L, "io::Write");
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        push("std::io::Read");
        push("std::io::Write");
        requireMissing(owner, 23L, "io::Close");
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        push("std::io::Read");
        Object read = handlers().get(1);
        push("std::io::Write");
        Object write = handlers().get(2);
        push("std::io::Close");
        Object close = handlers().get(3);

        VelnRuntime.Result accepted = register(owner, 31L, "Duplex");
        require(accepted.isOk(), "complete Duplex registration failed");
        Object resource = accepted.value();
        for (String family : new String[] {"ReadFile", "WriteFile", "UpdateFile", "AppendFile", "Duplex"}) {
            requireInvalidDuplicate(owner, 31L, family, resource);
        }

        for (long slot : new long[] {21L, 22L, 23L}) {
            require(register(owner, slot, "Duplex").isOk(), "failed missing-handler slot stayed consumed");
        }

        push("std::io::Seek");
        push("std::io::ReadAt");
        push("std::io::WriteAt");
        push("std::io::Sync");
        push("std::fs::Mutate");
        long existingSlot = 41L;
        for (String family : new String[] {"ReadFile", "WriteFile", "UpdateFile", "AppendFile"}) {
            VelnRuntime.Result existing = register(owner, existingSlot, family);
            require(existing.isOk(), family + " registration failed before reverse duplicate check");
            requireInvalidDuplicate(owner, existingSlot, "Duplex", existing.value());
            existingSlot += 1L;
        }

        for (int index = 0; index < 8; index++) VelnRuntime.popHandler();
        requireInvalidDuplicate(owner, 31L, "Duplex", resource);

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderDuplex(resource);
        require(projected.isOk(), "Duplex projection failed after handler scopes exited");
        @SuppressWarnings("unchecked")
        java.util.Map<String, Object> view = (java.util.Map<String, Object>) projected.value();
        require(view.size() == 3, "Duplex projection returned the wrong capability count");
        requireCapability(view.get("reader"), read, "reader");
        requireCapability(view.get("writer"), write, "writer");
        requireCapability(view.get("closer"), close, "closer");

        VelnRuntime.popHandler();
    }
}
