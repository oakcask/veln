    private static final String IO_PROVIDER_SCOPE_EFFECT = "std::io_provider::Scope";
    private static final String IO_READ_EFFECT = "std::io::Read";
    private static final String IO_SEEK_EFFECT = "std::io::Seek";
    private static final String IO_READ_AT_EFFECT = "std::io::ReadAt";
    private static final String IO_CLOSE_EFFECT = "std::io::Close";

    private static final class IoOwner {
        final HandlerFrame scope;
        final java.util.Map<Long, IoResource> resources =
            new java.util.LinkedHashMap<Long, IoResource>();

        IoOwner(HandlerFrame scope) {
            this.scope = scope;
        }
    }

    private static final class IoResource {
        final IoOwner owner;
        final long slot;
        final String family;
        final HandlerFrame read;
        final HandlerFrame seek;
        final HandlerFrame readAt;
        final HandlerFrame close;

        IoResource(
            IoOwner owner,
            long slot,
            String family,
            HandlerFrame read,
            HandlerFrame seek,
            HandlerFrame readAt,
            HandlerFrame close
        ) {
            this.owner = owner;
            this.slot = slot;
            this.family = family;
            this.read = read;
            this.seek = seek;
            this.readAt = readAt;
            this.close = close;
        }
    }

    private static final class IoCapability {
        final IoOwner owner;
        final long slot;
        final HandlerFrame handler;

        IoCapability(IoOwner owner, long slot, HandlerFrame handler) {
            this.owner = owner;
            this.slot = slot;
            this.handler = handler;
        }
    }

    private static Object ioProviderError(String operation, String kind, String message) {
        return adt(
            "io::Error::Error",
            new Object[] {
                operation,
                adt("io::ErrorKind::" + kind, new Object[] {}),
                none(),
                none(),
                none(),
                none(),
                message
            }
        );
    }

    private static Result ioProviderUnsupported(String operation, String message) {
        return err(ioProviderError(operation, "Unsupported", message));
    }

    private static Result ioProviderInvalidInput(String operation, String message) {
        return err(ioProviderError(operation, "InvalidInput", message));
    }

    public static Object ioProviderScopeOwner() {
        HandlerFrame scope = currentHandlerFrame();
        if (!scope.effect.equals(IO_PROVIDER_SCOPE_EFFECT)) {
            throw new IllegalStateException("io provider owner intrinsic used outside its scope handler");
        }
        synchronized (scope) {
            if (scope.ioOwner == null) scope.ioOwner = new IoOwner(scope);
            return scope.ioOwner;
        }
    }

    public static Object ioProviderResource(Object ownerValue, Object slotValue, Object familyValue) {
        if (!(ownerValue instanceof IoOwner)) {
            return ioProviderInvalidInput("resource", "invalid provider owner");
        }
        long slot = asLong(slotValue);
        if (slot < 0L) {
            return ioProviderInvalidInput("resource", "provider slots must be nonnegative");
        }
        IoOwner owner = (IoOwner) ownerValue;
        if (findHandler(IO_PROVIDER_SCOPE_EFFECT) != owner.scope) {
            return ioProviderInvalidInput("resource", "provider owner is not active in this scope");
        }
        if (!isAdt(familyValue, "ViewFamily::ReadFile")) {
            return ioProviderUnsupported("resource", "view family is outside the implemented prototype slice");
        }

        HandlerFrame read = findHandler(IO_READ_EFFECT);
        HandlerFrame seek = findHandler(IO_SEEK_EFFECT);
        HandlerFrame readAt = findHandler(IO_READ_AT_EFFECT);
        HandlerFrame close = findHandler(IO_CLOSE_EFFECT);
        if (read == null || seek == null || readAt == null || close == null) {
            return ioProviderUnsupported(
                "resource",
                "ReadFile registration requires io::Read, io::Seek, io::ReadAt, and io::Close handlers"
            );
        }

        IoResource resource = new IoResource(
            owner, slot, "ReadFile", read, seek, readAt, close
        );
        synchronized (owner.resources) {
            if (owner.resources.containsKey(Long.valueOf(slot))) {
                return ioProviderInvalidInput("resource", "provider slot is already registered");
            }
            owner.resources.put(Long.valueOf(slot), resource);
        }
        return ok(resource);
    }

    public static Object ioProviderReadFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("read_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (!resource.family.equals("ReadFile")) {
            return ioProviderUnsupported("read_file", "resource does not provide a ReadFile view");
        }
        return ok(adt(
            "fs::ReadFile::ReadFile",
            new Object[] {
                new IoCapability(resource.owner, resource.slot, resource.read),
                new IoCapability(resource.owner, resource.slot, resource.seek),
                new IoCapability(resource.owner, resource.slot, resource.readAt),
                new IoCapability(resource.owner, resource.slot, resource.close)
            }
        ));
    }

    public static Object ioProviderDuplex(Object resource) {
        return ioProviderUnsupported("duplex", "resource does not provide a Duplex view");
    }

    public static Object ioProviderSlot(Object owner, Object resource) {
        if (!(owner instanceof IoOwner) || !(resource instanceof IoResource)) {
            return ioProviderInvalidInput("slot", "invalid provider identity");
        }
        IoResource value = (IoResource) resource;
        return value.owner == owner
            ? ok(Long.valueOf(value.slot))
            : ioProviderInvalidInput("slot", "resource belongs to another provider owner");
    }

    private static Object ioProviderCapabilitySlot(String operation, Object owner, Object capability) {
        if (!(owner instanceof IoOwner) || !(capability instanceof IoCapability)) {
            return ioProviderInvalidInput(operation, "invalid provider capability identity");
        }
        IoCapability value = (IoCapability) capability;
        return value.owner == owner
            ? ok(Long.valueOf(value.slot))
            : ioProviderInvalidInput(operation, "capability belongs to another provider owner");
    }

    public static Object ioProviderReaderSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("reader_slot", owner, capability);
    }
    public static Object ioProviderSeekerSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("seeker_slot", owner, capability);
    }
    public static Object ioProviderReaderAtSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("reader_at_slot", owner, capability);
    }
    public static Object ioProviderCloserSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("closer_slot", owner, capability);
    }
