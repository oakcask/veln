    private static final String IO_PROVIDER_SCOPE_EFFECT = "std::io_provider::Scope";
    private static final String IO_READ_EFFECT = "std::io::Read";
    private static final String IO_WRITE_EFFECT = "std::io::Write";
    private static final String IO_SEEK_EFFECT = "std::io::Seek";
    private static final String IO_READ_AT_EFFECT = "std::io::ReadAt";
    private static final String IO_WRITE_AT_EFFECT = "std::io::WriteAt";
    private static final String IO_CLOSE_EFFECT = "std::io::Close";
    private static final String IO_SYNC_EFFECT = "std::io::Sync";
    private static final String FS_READ_EFFECT = "std::fs::Read";
    private static final String FS_MUTATE_EFFECT = "std::fs::Mutate";

    private static final class IoState {
        Object value;
        Thread admittedBy;
        IoStateWaiter admissionHead;
        IoStateWaiter admissionTail;
        long admissionSignalCount;

        IoState(Object value) {
            this.value = freezeBoundaryValue(value);
        }
    }

    private static final class IoStateWaiter {
        final Thread thread;
        IoStateWaiter previous;
        IoStateWaiter next;
        boolean queued;
        boolean admitted;

        IoStateWaiter(Thread thread) {
            this.thread = thread;
        }
    }

    private static void enqueueIoStateWaiter(IoState state, IoStateWaiter waiter) {
        waiter.previous = state.admissionTail;
        waiter.queued = true;
        if (state.admissionTail == null) {
            state.admissionHead = waiter;
        } else {
            state.admissionTail.next = waiter;
        }
        state.admissionTail = waiter;
    }

    private static void unlinkIoStateWaiter(IoState state, IoStateWaiter waiter) {
        if (!waiter.queued) return;
        if (waiter.previous == null) {
            state.admissionHead = waiter.next;
        } else {
            waiter.previous.next = waiter.next;
        }
        if (waiter.next == null) {
            state.admissionTail = waiter.previous;
        } else {
            waiter.next.previous = waiter.previous;
        }
        waiter.previous = null;
        waiter.next = null;
        waiter.queued = false;
    }

    private static void releaseIoStateAdmission(IoState state, Thread current) {
        synchronized (state) {
            if (state.admittedBy != current) return;
            IoStateWaiter successor = state.admissionHead;
            if (successor == null) {
                state.admittedBy = null;
                return;
            }
            unlinkIoStateWaiter(state, successor);
            state.admittedBy = successor.thread;
            synchronized (successor) {
                successor.admitted = true;
                state.admissionSignalCount += 1L;
                successor.notify();
            }
        }
    }

    private static void cancelIoStateAdmission(
        IoState state,
        IoStateWaiter waiter,
        Thread current
    ) {
        synchronized (state) {
            if (state.admittedBy == current) {
                releaseIoStateAdmission(state, current);
            } else {
                unlinkIoStateWaiter(state, waiter);
            }
        }
    }

    public static Object ioProviderState(Object initial) {
        throwTaskCancellationIfRequested();
        return new IoState(initial);
    }

    public static Object ioProviderStateUpdate(Object stateValue, Object context, Object action) {
        return ioProviderStateUpdateObserved(stateValue, context, action, null);
    }

    static Object ioProviderStateUpdateObserved(
        Object stateValue,
        Object context,
        Object action,
        Runnable afterPublication
    ) {
        if (!(stateValue instanceof IoState)) {
            throw new RuntimeFailure("invalid io provider state cell");
        }
        IoState state = (IoState) stateValue;
        Thread current = Thread.currentThread();
        Object oldValue;
        boolean admissionWaitInterrupted = false;
        IoStateWaiter waiter = null;
        synchronized (state) {
            if (state.admittedBy == current) {
                throw new RuntimeFailure("reentrant io provider state update");
            }
            throwTaskCancellationIfRequested();
            if (state.admittedBy == null && state.admissionHead == null) {
                state.admittedBy = current;
            } else {
                waiter = new IoStateWaiter(current);
                enqueueIoStateWaiter(state, waiter);
            }
        }
        if (waiter != null) {
            try {
                synchronized (waiter) {
                    while (!waiter.admitted) {
                        throwTaskCancellationIfRequested();
                        try {
                            waiter.wait();
                        } catch (InterruptedException interrupted) {
                            throwTaskCancellationIfRequested();
                            admissionWaitInterrupted = true;
                        }
                    }
                }
                throwTaskCancellationIfRequested();
            } catch (RuntimeException | Error failure) {
                cancelIoStateAdmission(state, waiter, current);
                throw failure;
            }
        }
        synchronized (state) {
            oldValue = state.value;
        }
        if (admissionWaitInterrupted) current.interrupt();

        boolean published = false;
        try {
            Object transition = call(action, oldValue, freezeBoundaryValue(context));
            if (!(transition instanceof java.util.Map)) {
                throw new RuntimeFailure("io provider state update callback returned an invalid transition");
            }
            java.util.Map<?, ?> fields = (java.util.Map<?, ?>) transition;
            if (!fields.containsKey("state") || !fields.containsKey("result")) {
                throw new RuntimeFailure("io provider state update callback returned an invalid transition");
            }
            Object replacement = freezeBoundaryValue(fields.get("state"));
            Object result = freezeBoundaryValue(fields.get("result"));
            synchronized (state) {
                state.value = replacement;
                published = true;
            }
            releaseIoStateAdmission(state, current);
            if (afterPublication != null) afterPublication.run();
            throwTaskCancellationIfRequested();
            return result;
        } finally {
            if (!published) {
                releaseIoStateAdmission(state, current);
            }
        }
    }

    private static final class IoOwner {
        final HandlerFrame scope;
        final java.util.Map<Long, Object> resources =
            new java.util.LinkedHashMap<Long, Object>();

        IoOwner(HandlerFrame scope) {
            this.scope = scope;
        }
    }

    private static final class IoFilesystem {
        final IoOwner owner;
        final long slot;
        final HandlerFrame read;

        IoFilesystem(IoOwner owner, long slot, HandlerFrame read) {
            this.owner = owner;
            this.slot = slot;
            this.read = read;
        }
    }

    private static final class IoMutableFilesystem {
        final IoOwner owner;
        final long slot;
        final HandlerFrame read;
        final HandlerFrame mutate;

        IoMutableFilesystem(
            IoOwner owner,
            long slot,
            HandlerFrame read,
            HandlerFrame mutate
        ) {
            this.owner = owner;
            this.slot = slot;
            this.read = read;
            this.mutate = mutate;
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

    private static Result ioProviderInvalidResource(String operation, String message) {
        return err(ioProviderError(operation, "InvalidResource", message));
    }

    private static boolean ioProviderOwnerIsLexicallyCurrent(IoOwner owner) {
        return findHandler(IO_PROVIDER_SCOPE_EFFECT) == owner.scope;
    }

    private static boolean ioProviderOwnerIsAccessible(IoOwner owner) {
        return ioProviderOwnerIsLexicallyCurrent(owner)
            || IO_PROVIDER_INVOCATION_OWNER.get() == owner;
    }

    private static void closeIoProviderScope(HandlerFrame scope) {
        if (!scope.effect.equals(IO_PROVIDER_SCOPE_EFFECT)) return;
        IoOwner owner;
        synchronized (scope) {
            owner = scope.ioOwner;
            if (owner == null) {
                scope.ioProviderScopeClosed = true;
                return;
            }
        }
        synchronized (owner.resources) {
            scope.ioProviderScopeClosed = true;
        }
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

    private static Object ioProviderNamespace(
        String operation,
        Object ownerValue,
        Object slotValue,
        boolean mutable
    ) {
        if (!(ownerValue instanceof IoOwner)) {
            return ioProviderInvalidInput(operation, "invalid provider owner");
        }
        long slot = asLong(slotValue);
        if (slot < 0L) {
            return ioProviderInvalidInput(operation, "provider slots must be nonnegative");
        }
        IoOwner owner = (IoOwner) ownerValue;
        if (!ioProviderOwnerIsLexicallyCurrent(owner)) {
            return ioProviderInvalidInput(operation, "provider owner is not active in this scope");
        }
        synchronized (owner.resources) {
            if (owner.scope.ioProviderScopeClosed) {
                return ioProviderInvalidInput(operation, "provider owner scope is closed");
            }
            if (owner.resources.containsKey(Long.valueOf(slot))) {
                return ioProviderInvalidInput(operation, "provider slot is already registered");
            }
            HandlerFrame read = findHandler(FS_READ_EFFECT);
            HandlerFrame mutate = mutable ? findHandler(FS_MUTATE_EFFECT) : null;
            if (read == null || (mutable && mutate == null)) {
                return ioProviderUnsupported(
                    operation,
                    mutable
                        ? "mutable filesystem registration requires fs::Read and fs::Mutate handlers"
                        : "filesystem registration requires an fs::Read handler"
                );
            }
            Object filesystem = mutable
                ? new IoMutableFilesystem(owner, slot, read, mutate)
                : new IoFilesystem(owner, slot, read);
            owner.resources.put(Long.valueOf(slot), filesystem);
            return ok(filesystem);
        }
    }

    public static Object ioProviderFilesystem(Object owner, Object slot) {
        return ioProviderNamespace("filesystem", owner, slot, false);
    }

    public static Object ioProviderMutableFilesystem(Object owner, Object slot) {
        return ioProviderNamespace("mutable_filesystem", owner, slot, true);
    }

    public static Object fsReadOnly(Object filesystem) {
        if (!(filesystem instanceof IoMutableFilesystem)) {
            throw new RuntimeFailure("invalid mutable filesystem token");
        }
        IoMutableFilesystem value = (IoMutableFilesystem) filesystem;
        return new IoFilesystem(value.owner, value.slot, value.read);
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
        if (!ioProviderOwnerIsAccessible(owner)) {
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
            if (owner.scope.ioProviderScopeClosed) {
                return ioProviderInvalidInput("resource", "provider owner scope is closed");
            }
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
        return value.owner == owner && ioProviderOwnerIsAccessible(value.owner)
            ? ok(Long.valueOf(value.slot))
            : ioProviderInvalidResource("slot", "resource belongs to another provider owner");
    }

    private static Object ioProviderNamespaceSlot(
        String operation,
        Object ownerValue,
        Object filesystemValue,
        boolean mutable
    ) {
        if (!(ownerValue instanceof IoOwner)) {
            return ioProviderInvalidInput(operation, "invalid provider owner");
        }
        IoOwner owner = (IoOwner) ownerValue;
        if (!ioProviderOwnerIsAccessible(owner)) {
            return ioProviderInvalidResource(operation, "provider owner is not active in this scope");
        }
        if (owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource(operation, "provider owner scope is closed");
        }
        if (mutable) {
            if (!(filesystemValue instanceof IoMutableFilesystem)) {
                return ioProviderInvalidInput(operation, "invalid mutable filesystem identity");
            }
            IoMutableFilesystem filesystem = (IoMutableFilesystem) filesystemValue;
            return filesystem.owner == owner
                ? ok(Long.valueOf(filesystem.slot))
                : ioProviderInvalidResource(operation, "filesystem belongs to another provider owner");
        }
        if (!(filesystemValue instanceof IoFilesystem)) {
            return ioProviderInvalidInput(operation, "invalid filesystem identity");
        }
        IoFilesystem filesystem = (IoFilesystem) filesystemValue;
        return filesystem.owner == owner
            ? ok(Long.valueOf(filesystem.slot))
            : ioProviderInvalidResource(operation, "filesystem belongs to another provider owner");
    }

    public static Object ioProviderFilesystemSlot(Object owner, Object filesystem) {
        return ioProviderNamespaceSlot("filesystem_slot", owner, filesystem, false);
    }

    public static Object ioProviderMutableFilesystemSlot(Object owner, Object filesystem) {
        return ioProviderNamespaceSlot("mutable_filesystem_slot", owner, filesystem, true);
    }

    private static Object ioProviderCapabilitySlot(String operation, Object owner, Object capability) {
        if (!(owner instanceof IoOwner) || !(capability instanceof IoCapability)) {
            return ioProviderInvalidInput(operation, "invalid provider capability identity");
        }
        IoCapability value = (IoCapability) capability;
        return value.owner == owner && ioProviderOwnerIsAccessible(value.owner)
            ? ok(Long.valueOf(value.slot))
            : ioProviderInvalidResource(operation, "capability belongs to another provider owner");
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

    private static HandlerFrame ioProviderCapturedHandler(String effect, Object operand) {
        if (operand instanceof IoFilesystem && effect.equals(FS_READ_EFFECT)) {
            return ((IoFilesystem) operand).read;
        }
        if (operand instanceof IoMutableFilesystem) {
            IoMutableFilesystem filesystem = (IoMutableFilesystem) operand;
            if (effect.equals(FS_READ_EFFECT)) return filesystem.read;
            if (effect.equals(FS_MUTATE_EFFECT)) return filesystem.mutate;
        }
        if (operand instanceof IoCapability) {
            IoCapability capability = (IoCapability) operand;
            return capability.handler != null && capability.handler.effect.equals(effect)
                ? capability.handler
                : null;
        }
        return null;
    }

    private static Object ioProviderDispatch(
        Object effectValue,
        Object operationValue,
        Object operand,
        Object[] trailing
    ) {
        String effect = asString(effectValue);
        String operation = asString(operationValue);
        HandlerFrame handler = ioProviderCapturedHandler(effect, operand);
        if (handler == null) {
            throw new RuntimeFailure("invalid provider-bound operand for " + effect + "::" + operation);
        }
        IoOwner owner = operand instanceof IoCapability
            ? ((IoCapability) operand).owner
            : operand instanceof IoFilesystem
                ? ((IoFilesystem) operand).owner
                : ((IoMutableFilesystem) operand).owner;
        Object[] args = new Object[trailing.length + 1];
        args[0] = operand;
        System.arraycopy(trailing, 0, args, 1, trailing.length);
        IoOwner previous = IO_PROVIDER_INVOCATION_OWNER.get();
        IO_PROVIDER_INVOCATION_OWNER.set(owner);
        try {
            return invokeHandler(handler, operation, args);
        } finally {
            if (previous == null) IO_PROVIDER_INVOCATION_OWNER.remove();
            else IO_PROVIDER_INVOCATION_OWNER.set(previous);
        }
    }

    public static Object ioProviderDispatch1(Object effect, Object operation, Object operand) {
        return ioProviderDispatch(effect, operation, operand, new Object[] {});
    }

    public static Object ioProviderDispatch2(
        Object effect,
        Object operation,
        Object operand,
        Object argument
    ) {
        return ioProviderDispatch(effect, operation, operand, new Object[] { argument });
    }

    public static Object ioProviderDispatch3(
        Object effect,
        Object operation,
        Object operand,
        Object first,
        Object second
    ) {
        return ioProviderDispatch(effect, operation, operand, new Object[] { first, second });
    }

    private static Object ioProviderForeignFailure(String effect, String operation) {
        Object error = ioProviderError(operation, "InvalidResource", "operand belongs to another provider");
        Object zero = adt("io::Progress::Known", new Object[] { byteCountObject(0L) });
        Object empty = byteChunkFromByteArray(new byte[] {}, 0);
        if (effect.equals(FS_READ_EFFECT)) return err(error);
        if (effect.equals(FS_MUTATE_EFFECT)) {
            if (operation.startsWith("open_")) return err(error);
            return adt("fs::MutationOutcome::MutationFailed", new Object[] { adt("io::Commit::No", new Object[] {}), error });
        }
        if (effect.equals(IO_READ_EFFECT) || effect.equals(IO_READ_AT_EFFECT)) {
            return adt("io::ReadOutcome::ReadFailed", new Object[] { empty, zero, error });
        }
        if (effect.equals(IO_WRITE_EFFECT) || effect.equals(IO_WRITE_AT_EFFECT)) {
            return adt("io::WriteOutcome::WriteFailed", new Object[] { zero, error });
        }
        if (effect.equals(IO_CLOSE_EFFECT)) {
            return adt("io::CloseOutcome::CloseFailed", new Object[] { adt("io::Commit::No", new Object[] {}), error });
        }
        return err(error);
    }

    private static Object ioProviderLexicalDispatchFailure(
        String effect,
        String operation,
        Object[] args,
        HandlerFrame selected
    ) {
        if (args.length == 0) return null;
        HandlerFrame captured = ioProviderCapturedHandler(effect, args[0]);
        IoOwner owner = args[0] instanceof IoCapability
            ? ((IoCapability) args[0]).owner
            : args[0] instanceof IoFilesystem
                ? ((IoFilesystem) args[0]).owner
                : args[0] instanceof IoMutableFilesystem
                    ? ((IoMutableFilesystem) args[0]).owner
                    : null;
        HandlerFrame lexicalScope = findHandler(IO_PROVIDER_SCOPE_EFFECT);
        return captured != null && (captured != selected || owner == null || owner.scope != lexicalScope)
            ? ioProviderForeignFailure(effect, operation)
            : null;
    }

    public static Object ioCopy(Object reader, Object writer, Object chunkSize) {
        long total = 0L;
        while (true) {
            Object read = ioProviderDispatch2(IO_READ_EFFECT, "read", reader, chunkSize);
            if (isAdt(read, "ReadOutcome::End")) {
                return adt("io::CopyOutcome::Copied", new Object[] { byteCountObject(total) });
            }
            if (isAdt(read, "ReadOutcome::ReadFailed")) {
                return adt(
                    "io::CopyOutcome::CopyFailed",
                    new Object[] { adtPayload(read, 1), adt("io::Progress::Known", new Object[] { byteCountObject(total) }), adtPayload(read, 2) }
                );
            }
            if (!isAdt(read, "ReadOutcome::Data")) {
                throw new RuntimeFailure("provider read handler returned an invalid outcome");
            }
            Object bytes = adtPayload(read, 0);
            long count = byteChunkBytes(bytes).size();
            if (count == 0L) {
                Object error = ioProviderError("copy", "NoProgress", "provider read returned an empty data chunk");
                Object known = adt("io::Progress::Known", new Object[] { byteCountObject(total) });
                return adt("io::CopyOutcome::CopyFailed", new Object[] { known, known, error });
            }
            Object write = ioProviderDispatch2(IO_WRITE_EFFECT, "write", writer, bytes);
            if (isAdt(write, "WriteOutcome::WriteFailed")) {
                return adt(
                    "io::CopyOutcome::CopyFailed",
                    new Object[] { adt("io::Progress::Known", new Object[] { byteCountObject(total + count) }), adtPayload(write, 0), adtPayload(write, 1) }
                );
            }
            if (!isAdt(write, "WriteOutcome::Written")) {
                throw new RuntimeFailure("provider write handler returned an invalid outcome");
            }
            long written = byteCountLong(adtPayload(write, 0));
            total += written;
            if (written != count) {
                Object error = ioProviderError("copy", "NoProgress", "provider write did not consume the complete chunk");
                return adt(
                    "io::CopyOutcome::CopyFailed",
                    new Object[] {
                        adt("io::Progress::Known", new Object[] { byteCountObject(total - written + count) }),
                        adt("io::Progress::Known", new Object[] { byteCountObject(total) }),
                        error
                    }
                );
            }
        }
    }
