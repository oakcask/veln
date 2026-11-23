    private static final String IO_PROVIDER_SCOPE_EFFECT = "std::io_provider::Scope";
    private static final String IO_READ_EFFECT = "std::io::Read";
    private static final String IO_WRITE_EFFECT = "std::io::Write";
    private static final String IO_SEEK_EFFECT = "std::io::Seek";
    private static final String IO_READ_AT_EFFECT = "std::io::ReadAt";
    private static final String IO_WRITE_AT_EFFECT = "std::io::WriteAt";
    private static final String IO_CLOSE_EFFECT = "std::io::Close";
    private static final String IO_SYNC_EFFECT = "std::io::Sync";
    private static final String FS_MUTATE_EFFECT = "std::fs::Mutate";

    private static final class IoProviderState {
        Object value;
        Thread updatingThread;
        IoProviderStateBoundaryGate boundaryGate;

        IoProviderState(Object value) {
            this.value = freezeBoundaryValue(value);
        }
    }

    public interface IoProviderStateBoundaryGate {
        void beforeAdmission();
        void afterAdmission();
        void afterCommit();
    }

    public static Object ioProviderStateInstallBoundaryGate(
        Object stateValue,
        IoProviderStateBoundaryGate gate
    ) {
        IoProviderState state = (IoProviderState) stateValue;
        synchronized (state) {
            state.boundaryGate = gate;
        }
        return UNIT;
    }

    public static Object ioProviderState(Object initial) {
        return new IoProviderState(initial);
    }

    public static Object ioProviderUpdateState(Object stateValue, Object context, Object action) {
        if (!(stateValue instanceof IoProviderState)) {
            throw new RuntimeFailure("invalid io provider state cell");
        }
        IoProviderState state = (IoProviderState) stateValue;
        IoProviderStateBoundaryGate gate = state.boundaryGate;
        if (gate != null) gate.beforeAdmission();
        throwTaskCancellationIfRequested();
        Object result;
        synchronized (state) {
            if (state.updatingThread == Thread.currentThread()) {
                throw new RuntimeFailure("reentrant io provider state update");
            }
            throwTaskCancellationIfRequested();
            state.updatingThread = Thread.currentThread();
            deferTaskCancellationDelivery();
            try {
                if (gate != null) gate.afterAdmission();
                Object transition = call(action, state.value, context);
                Object replacement = recordField(transition, "state");
                Object transitionResult = recordField(transition, "result");
                Object frozenReplacement = freezeBoundaryValue(replacement);
                result = freezeBoundaryValue(transitionResult);
                state.value = frozenReplacement;
            } finally {
                resumeTaskCancellationDelivery();
                state.updatingThread = null;
            }
        }
        if (gate != null) gate.afterCommit();
        throwTaskCancellationIfRequested();
        return result;
    }

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
        final HandlerFrame write;
        final HandlerFrame seek;
        final HandlerFrame readAt;
        final HandlerFrame writeAt;
        final HandlerFrame close;
        final HandlerFrame sync;
        final HandlerFrame mutate;

        IoResource(
            IoOwner owner,
            long slot,
            String family,
            HandlerFrame read,
            HandlerFrame write,
            HandlerFrame seek,
            HandlerFrame readAt,
            HandlerFrame writeAt,
            HandlerFrame close,
            HandlerFrame sync,
            HandlerFrame mutate
        ) {
            this.owner = owner;
            this.slot = slot;
            this.family = family;
            this.read = read;
            this.write = write;
            this.seek = seek;
            this.readAt = readAt;
            this.writeAt = writeAt;
            this.close = close;
            this.sync = sync;
            this.mutate = mutate;
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
        boolean readFile = isAdt(familyValue, "ViewFamily::ReadFile");
        boolean writeFile = isAdt(familyValue, "ViewFamily::WriteFile");
        boolean updateFile = isAdt(familyValue, "ViewFamily::UpdateFile");
        boolean appendFile = isAdt(familyValue, "ViewFamily::AppendFile");
        boolean duplex = isAdt(familyValue, "ViewFamily::Duplex");
        if (!readFile && !writeFile && !updateFile && !appendFile && !duplex) {
            return ioProviderUnsupported("resource", "view family is outside the implemented prototype slice");
        }

        synchronized (owner.resources) {
            if (owner.resources.containsKey(Long.valueOf(slot))) {
                return ioProviderInvalidInput("resource", "provider slot is already registered");
            }

            IoResource resource;
            if (readFile) {
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
                resource = new IoResource(
                    owner, slot, "ReadFile", read, null, seek, readAt, null, close, null, null
                );
            } else if (writeFile) {
                HandlerFrame write = findHandler(IO_WRITE_EFFECT);
                HandlerFrame seek = findHandler(IO_SEEK_EFFECT);
                HandlerFrame writeAt = findHandler(IO_WRITE_AT_EFFECT);
                HandlerFrame close = findHandler(IO_CLOSE_EFFECT);
                HandlerFrame sync = findHandler(IO_SYNC_EFFECT);
                HandlerFrame mutate = findHandler(FS_MUTATE_EFFECT);
                if (write == null || seek == null || writeAt == null || close == null || sync == null || mutate == null) {
                    return ioProviderUnsupported(
                        "resource",
                        "WriteFile registration requires io::Write, io::Seek, io::WriteAt, io::Close, io::Sync, and fs::Mutate handlers"
                    );
                }
                resource = new IoResource(
                    owner, slot, "WriteFile", null, write, seek, null, writeAt, close, sync, mutate
                );
            } else if (updateFile) {
                HandlerFrame read = findHandler(IO_READ_EFFECT);
                HandlerFrame write = findHandler(IO_WRITE_EFFECT);
                HandlerFrame seek = findHandler(IO_SEEK_EFFECT);
                HandlerFrame readAt = findHandler(IO_READ_AT_EFFECT);
                HandlerFrame writeAt = findHandler(IO_WRITE_AT_EFFECT);
                HandlerFrame close = findHandler(IO_CLOSE_EFFECT);
                HandlerFrame sync = findHandler(IO_SYNC_EFFECT);
                HandlerFrame mutate = findHandler(FS_MUTATE_EFFECT);
                if (read == null || write == null || seek == null || readAt == null || writeAt == null
                    || close == null || sync == null || mutate == null) {
                    return ioProviderUnsupported(
                        "resource",
                        "UpdateFile registration requires io::Read, io::Write, io::Seek, io::ReadAt, io::WriteAt, io::Close, io::Sync, and fs::Mutate handlers"
                    );
                }
                resource = new IoResource(
                    owner, slot, "UpdateFile", read, write, seek, readAt, writeAt, close, sync, mutate
                );
            } else if (appendFile) {
                HandlerFrame write = findHandler(IO_WRITE_EFFECT);
                HandlerFrame close = findHandler(IO_CLOSE_EFFECT);
                HandlerFrame sync = findHandler(IO_SYNC_EFFECT);
                if (write == null || close == null || sync == null) {
                    return ioProviderUnsupported(
                        "resource",
                        "AppendFile registration requires io::Write, io::Close, and io::Sync handlers"
                    );
                }
                resource = new IoResource(
                    owner, slot, "AppendFile", null, write, null, null, null, close, sync, null
                );
            } else {
                HandlerFrame read = findHandler(IO_READ_EFFECT);
                HandlerFrame write = findHandler(IO_WRITE_EFFECT);
                HandlerFrame close = findHandler(IO_CLOSE_EFFECT);
                if (read == null || write == null || close == null) {
                    return ioProviderUnsupported(
                        "resource",
                        "Duplex registration requires io::Read, io::Write, and io::Close handlers"
                    );
                }
                resource = new IoResource(
                    owner, slot, "Duplex", read, write, null, null, null, close, null, null
                );
            }
            owner.resources.put(Long.valueOf(slot), resource);
            return ok(resource);
        }
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

    public static Object ioProviderAppendFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("append_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (!resource.family.equals("AppendFile")) {
            return ioProviderUnsupported("append_file", "resource does not provide an AppendFile view");
        }
        return ok(adt(
            "fs::AppendFile::AppendFile",
            new Object[] {
                new IoCapability(resource.owner, resource.slot, resource.write),
                new IoCapability(resource.owner, resource.slot, resource.close),
                new IoCapability(resource.owner, resource.slot, resource.sync)
            }
        ));
    }

    public static Object ioProviderWriteFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("write_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (!resource.family.equals("WriteFile")) {
            return ioProviderUnsupported("write_file", "resource does not provide a WriteFile view");
        }
        return ok(adt(
            "fs::WriteFile::WriteFile",
            new Object[] {
                new IoCapability(resource.owner, resource.slot, resource.write),
                new IoCapability(resource.owner, resource.slot, resource.seek),
                new IoCapability(resource.owner, resource.slot, resource.writeAt),
                new IoCapability(resource.owner, resource.slot, resource.close),
                new IoCapability(resource.owner, resource.slot, resource.sync),
                new IoCapability(resource.owner, resource.slot, resource.mutate)
            }
        ));
    }

    public static Object ioProviderUpdateFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("update_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (!resource.family.equals("UpdateFile")) {
            return ioProviderUnsupported("update_file", "resource does not provide an UpdateFile view");
        }
        return ok(adt(
            "fs::UpdateFile::UpdateFile",
            new Object[] {
                new IoCapability(resource.owner, resource.slot, resource.read),
                new IoCapability(resource.owner, resource.slot, resource.write),
                new IoCapability(resource.owner, resource.slot, resource.seek),
                new IoCapability(resource.owner, resource.slot, resource.readAt),
                new IoCapability(resource.owner, resource.slot, resource.writeAt),
                new IoCapability(resource.owner, resource.slot, resource.close),
                new IoCapability(resource.owner, resource.slot, resource.sync),
                new IoCapability(resource.owner, resource.slot, resource.mutate)
            }
        ));
    }

    public static Object ioProviderDuplex(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("duplex", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (!resource.family.equals("Duplex")) {
            return ioProviderUnsupported("duplex", "resource does not provide a Duplex view");
        }
        return ok(record(
            "reader", new IoCapability(resource.owner, resource.slot, resource.read),
            "writer", new IoCapability(resource.owner, resource.slot, resource.write),
            "closer", new IoCapability(resource.owner, resource.slot, resource.close)
        ));
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
    public static Object ioProviderWriterSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("writer_slot", owner, capability);
    }
    public static Object ioProviderSeekerSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("seeker_slot", owner, capability);
    }
    public static Object ioProviderReaderAtSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("reader_at_slot", owner, capability);
    }
    public static Object ioProviderWriterAtSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("writer_at_slot", owner, capability);
    }
    public static Object ioProviderCloserSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("closer_slot", owner, capability);
    }
    public static Object ioProviderSyncerSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("syncer_slot", owner, capability);
    }
    public static Object ioProviderTruncaterSlot(Object owner, Object capability) {
        return ioProviderCapabilitySlot("truncater_slot", owner, capability);
    }
