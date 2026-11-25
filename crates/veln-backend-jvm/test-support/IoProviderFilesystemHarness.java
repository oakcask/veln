public final class IoProviderFilesystemHarness extends IoProviderHarnessSupport {
    private static final java.lang.reflect.Field READONLY_OWNER;
    private static final java.lang.reflect.Field READONLY_SLOT;
    private static final java.lang.reflect.Field READONLY_READ;
    private static final java.lang.reflect.Field MUTABLE_OWNER;
    private static final java.lang.reflect.Field MUTABLE_SLOT;
    private static final java.lang.reflect.Field MUTABLE_READ;
    private static final java.lang.reflect.Field MUTABLE_MUTATE;

    static {
        try {
            Class<?> readonly = Class.forName("VelnRuntime$IoFilesystem");
            Class<?> mutable = Class.forName("VelnRuntime$IoMutableFilesystem");
            READONLY_OWNER = readonly.getDeclaredField("owner");
            READONLY_SLOT = readonly.getDeclaredField("slot");
            READONLY_READ = readonly.getDeclaredField("read");
            MUTABLE_OWNER = mutable.getDeclaredField("owner");
            MUTABLE_SLOT = mutable.getDeclaredField("slot");
            MUTABLE_READ = mutable.getDeclaredField("read");
            MUTABLE_MUTATE = mutable.getDeclaredField("mutate");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                READONLY_OWNER,
                READONLY_SLOT,
                READONLY_READ,
                MUTABLE_OWNER,
                MUTABLE_SLOT,
                MUTABLE_READ,
                MUTABLE_MUTATE
            }) {
                member.setAccessible(true);
            }
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    private static VelnRuntime.Result result(Object value) {
        return (VelnRuntime.Result) value;
    }

    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object owner = owner();

        push("std::fs::Read");
        Object readonlyRead = handlers().get(1);
        VelnRuntime.Result readonlyResult = result(
            VelnRuntime.ioProviderFilesystem(owner, Long.valueOf(11L))
        );
        require(readonlyResult.isOk(), "read-only filesystem factory failed");
        Object readonly = readonlyResult.value();
        require(READONLY_OWNER.get(readonly) == owner, "read-only filesystem lost its owner");
        require(((Long) READONLY_SLOT.get(readonly)).longValue() == 11L, "read-only filesystem lost its slot");
        require(READONLY_READ.get(readonly) == readonlyRead, "read-only filesystem lost its read handler");
        VelnRuntime.popHandler();

        push("std::fs::Read");
        Object mutableRead = handlers().get(1);
        push("std::fs::Mutate");
        Object mutableMutate = handlers().get(2);
        VelnRuntime.Result mutableResult = result(
            VelnRuntime.ioProviderMutableFilesystem(owner, Long.valueOf(12L))
        );
        require(mutableResult.isOk(), "mutable filesystem factory failed");
        Object mutable = mutableResult.value();
        require(MUTABLE_OWNER.get(mutable) == owner, "mutable filesystem lost its owner");
        require(((Long) MUTABLE_SLOT.get(mutable)).longValue() == 12L, "mutable filesystem lost its slot");
        require(MUTABLE_READ.get(mutable) == mutableRead, "mutable filesystem lost its read handler");
        require(MUTABLE_MUTATE.get(mutable) == mutableMutate, "mutable filesystem lost its mutate handler");
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        push("std::io_provider::Scope");
        Object otherOwner = owner();
        require(otherOwner != owner, "nested provider scope reused the outer owner");
        VelnRuntime.Result wrongOwner = result(VelnRuntime.ioProviderFilesystemSlot(otherOwner, readonly));
        require(wrongOwner.toString().contains("InvalidResource"), "wrong owner accessed a filesystem slot");
        VelnRuntime.popHandler();

        require(
            result(VelnRuntime.ioProviderFilesystemSlot(owner, readonly)).isOk(),
            "active owner could not access its filesystem slot"
        );

        push("std::fs::Read");
        final java.util.concurrent.atomic.AtomicReference<Object> closedSlotResult =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        final java.util.concurrent.atomic.AtomicReference<Object> closedFactoryResult =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        Thread inherited = new Thread(() -> {
            closedSlotResult.set(VelnRuntime.ioProviderFilesystemSlot(owner, readonly));
            closedFactoryResult.set(VelnRuntime.ioProviderFilesystem(owner, Long.valueOf(13L)));
        });
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        Object projected = VelnRuntime.fsReadOnly(mutable);
        require(READONLY_OWNER.get(projected) == owner, "closed-owner read-only projection changed the owner");
        require(((Long) READONLY_SLOT.get(projected)).longValue() == 12L, "closed-owner read-only projection changed the slot");
        require(READONLY_READ.get(projected) == mutableRead, "closed-owner read-only projection changed the read handler");

        inherited.start();
        inherited.join();
        require(
            result(closedSlotResult.get()).toString().contains("InvalidResource"),
            "inherited handler frame kept a closed owner usable"
        );
        String factoryFailure = result(closedFactoryResult.get()).toString();
        require(
            factoryFailure.contains("filesystem")
                && factoryFailure.contains("InvalidInput")
                && factoryFailure.contains("provider owner scope is closed"),
            "inherited handler frame published a namespace token through a closed owner: " + factoryFailure
        );
    }
}
