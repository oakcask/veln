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
    private static volatile Fn hostOpenWriteOpenedHookForTest;
    private static volatile Fn hostOpenWriteRejectedHookForTest;

    static void setHostOpenWritePublicationHooksForTest(Fn opened, Fn rejected) {
        hostOpenWriteOpenedHookForTest = opened;
        hostOpenWriteRejectedHookForTest = rejected;
    }

    static void clearHostOpenWritePublicationHooksForTest() {
        hostOpenWriteOpenedHookForTest = null;
        hostOpenWriteRejectedHookForTest = null;
    }

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
        long nextHostSlot = Long.MAX_VALUE;

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
        final java.nio.file.Path hostRoot;

        IoMutableFilesystem(
            IoOwner owner,
            long slot,
            HandlerFrame read,
            HandlerFrame mutate
        ) {
            this(owner, slot, read, mutate, null);
        }

        IoMutableFilesystem(
            IoOwner owner,
            long slot,
            HandlerFrame read,
            HandlerFrame mutate,
            java.nio.file.Path hostRoot
        ) {
            this.owner = owner;
            this.slot = slot;
            this.read = read;
            this.mutate = mutate;
            this.hostRoot = hostRoot;
        }
    }

    private static final class HostWriteResource {
        final java.nio.channels.FileChannel channel;
        boolean closed;

        HostWriteResource(java.nio.channels.FileChannel channel) {
            this.channel = channel;
        }

        void releaseRejected() throws java.io.IOException {
            Fn rejectedHook = hostOpenWriteRejectedHookForTest;
            try {
                if (rejectedHook != null) rejectedHook.call(this, channel);
            } finally {
                channel.close();
            }
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
        final HostWriteResource hostWrite;

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
            this(
                owner, slot, family, read, write, seek, readAt, writeAt, close, sync, mutate, null
            );
        }

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
            HandlerFrame mutate,
            HostWriteResource hostWrite
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
            this.hostWrite = hostWrite;
        }
    }

    private static final class IoCapability {
        final IoResource resource;
        final HandlerFrame handler;

        IoCapability(IoResource resource, HandlerFrame handler) {
            this.resource = resource;
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

    private static HandlerFrame hostHandler(String effect, String operation, Fn provider) {
        java.util.Map<String, Fn> providers = new java.util.HashMap<String, Fn>();
        if (operation != null) providers.put(operation, provider);
        return new HandlerFrame(effect, providers, new Object[] {});
    }

    private static long nextHostSlot(IoOwner owner) {
        while (owner.resources.containsKey(Long.valueOf(owner.nextHostSlot))) {
            owner.nextHostSlot -= 1L;
        }
        return owner.nextHostSlot--;
    }

    private static String portablePathText(Object pathValue) {
        if (!isAdt(pathValue, "Path::PortablePath")) {
            throw new IllegalArgumentException("invalid portable path");
        }
        return asString(((Adt) pathValue).payloads[0]);
    }

    private static java.nio.file.Path resolveHostPath(
        IoMutableFilesystem filesystem,
        Object pathValue
    ) {
        String spelling = portablePathText(pathValue);
        java.nio.file.Path resolved = spelling.equals(".")
            ? filesystem.hostRoot
            : filesystem.hostRoot.resolve(spelling).normalize();
        if (!resolved.startsWith(filesystem.hostRoot)) {
            throw new IllegalArgumentException("portable path left the selected filesystem root");
        }
        return resolved;
    }

    private static Result hostFilesystemFailure(
        String operation,
        Throwable failure,
        java.nio.file.Path path
    ) {
        String kind;
        if (failure instanceof java.nio.file.NoSuchFileException) kind = "NotFound";
        else if (failure instanceof java.nio.file.FileAlreadyExistsException) kind = "AlreadyExists";
        else if (failure instanceof java.nio.file.AccessDeniedException
            || failure instanceof SecurityException) kind = "PermissionDenied";
        else if (failure instanceof java.nio.file.FileSystemLoopException) kind = "LinkLoop";
        else if (failure instanceof IllegalArgumentException) kind = "InvalidInput";
        else if (path != null && java.nio.file.Files.isDirectory(path)) kind = "IsDirectory";
        else kind = "Other";
        String message = failure.getMessage() == null
            ? failure.getClass().getSimpleName()
            : failure.getMessage();
        return err(ioProviderError(operation, kind, message));
    }

    private static HostWriteResource hostWriteResource(Object capabilityValue) {
        if (!(capabilityValue instanceof IoCapability)) {
            throw new RuntimeFailure("invalid host file capability");
        }
        IoResource resource = ((IoCapability) capabilityValue).resource;
        if (resource.hostWrite == null) {
            throw new RuntimeFailure("capability is not backed by a host file");
        }
        return resource.hostWrite;
    }

    private static Object hostWrite(Object[] args) {
        HostWriteResource resource = hostWriteResource(args[0]);
        if (resource.closed || !resource.channel.isOpen()) {
            Object error = ioProviderError("write", "Closed", "host file is closed");
            return adt(
                "io::WriteOutcome::WriteFailed",
                new Object[] {
                    adt("io::Progress::Known", new Object[] { byteCountObject(0L) }),
                    error
                }
            );
        }
        java.util.List<Object> values = byteChunkBytes(args[1]);
        byte[] bytes = new byte[values.size()];
        for (int index = 0; index < values.size(); index += 1) {
            bytes[index] = (byte) byteValueLong(values.get(index));
        }
        try {
            int written = resource.channel.write(java.nio.ByteBuffer.wrap(bytes));
            return adt(
                "io::WriteOutcome::Written",
                new Object[] { byteCountObject((long) written) }
            );
        } catch (java.io.IOException failure) {
            Object error = ioProviderError("write", "Other", failure.toString());
            return adt(
                "io::WriteOutcome::WriteFailed",
                new Object[] {
                    adt("io::Progress::Unknown", new Object[] { byteCountObject(0L) }),
                    error
                }
            );
        }
    }

    private static Object hostSeek(Object[] args) {
        HostWriteResource resource = hostWriteResource(args[0]);
        if (resource.closed || !resource.channel.isOpen()) {
            return ioProviderInvalidResource("seek", "host file is closed");
        }
        Object origin = args[1];
        try {
            long target;
            if (isAdt(origin, "SeekOrigin::Start")) {
                target = byteOffsetLong(((Adt) origin).payloads[0]);
            } else if (isAdt(origin, "SeekOrigin::Current")) {
                target = Math.addExact(
                    resource.channel.position(),
                    asLong(((Adt) origin).payloads[0])
                );
            } else if (isAdt(origin, "SeekOrigin::End")) {
                target = Math.addExact(
                    resource.channel.size(),
                    asLong(((Adt) origin).payloads[0])
                );
            } else {
                return ioProviderInvalidInput("seek", "invalid seek origin");
            }
            if (target < 0L) {
                return ioProviderInvalidInput("seek", "seek target must be nonnegative");
            }
            resource.channel.position(target);
            return ok(byteOffsetObject(target));
        } catch (ArithmeticException | IllegalArgumentException failure) {
            return ioProviderInvalidInput("seek", "seek target is outside the supported range");
        } catch (java.nio.channels.ClosedChannelException failure) {
            return err(ioProviderError("seek", "Closed", "host file is closed"));
        } catch (java.io.IOException failure) {
            return err(ioProviderError("seek", "Other", failure.toString()));
        }
    }

    private static Object hostClose(Object[] args) {
        HostWriteResource resource = hostWriteResource(args[0]);
        if (resource.closed) return adt("io::CloseOutcome::Closed", new Object[] {});
        try {
            resource.channel.close();
            resource.closed = true;
            return adt("io::CloseOutcome::Closed", new Object[] {});
        } catch (java.io.IOException failure) {
            Object error = ioProviderError("close", "Other", failure.toString());
            return adt(
                "io::CloseOutcome::CloseFailed",
                new Object[] { adt("io::Commit::Unknown", new Object[] {}), error }
            );
        }
    }

    private static Object hostOpenWrite(Object[] args) {
        if (!(args[0] instanceof IoMutableFilesystem)) {
            return ioProviderInvalidInput("open_write", "invalid mutable filesystem");
        }
        IoMutableFilesystem filesystem = (IoMutableFilesystem) args[0];
        if (filesystem.hostRoot == null) {
            return ioProviderInvalidInput("open_write", "filesystem is not a host filesystem");
        }
        Object mode = args[2];
        if (!isAdt(mode, "WriteMode::WriteMode")) {
            return ioProviderInvalidInput("open_write", "invalid write mode");
        }
        Adt writeMode = (Adt) mode;
        if (!isAdt(writeMode.payloads[0], "CreateMode::Existing")
            || asBool(writeMode.payloads[1])) {
            return ioProviderUnsupported(
                "open_write",
                "the bounded JVM host prototype supports only Existing without truncate"
            );
        }
        java.nio.file.Path path;
        try {
            path = resolveHostPath(filesystem, args[1]);
        } catch (RuntimeException failure) {
            return hostFilesystemFailure("open_write", failure, null);
        }
        try {
            java.nio.channels.FileChannel channel = java.nio.channels.FileChannel.open(
                path,
                java.nio.file.StandardOpenOption.WRITE
            );
            HostWriteResource host = new HostWriteResource(channel);
            Fn openedHook = hostOpenWriteOpenedHookForTest;
            try {
                if (openedHook != null) openedHook.call(host, channel);
            } catch (RuntimeException | Error failure) {
                try {
                    channel.close();
                } catch (java.io.IOException closeFailure) {
                    failure.addSuppressed(closeFailure);
                }
                throw failure;
            }
            HandlerFrame write = hostHandler(IO_WRITE_EFFECT, "write", VelnRuntime::hostWrite);
            HandlerFrame seek = hostHandler(IO_SEEK_EFFECT, "seek", VelnRuntime::hostSeek);
            HandlerFrame close = hostHandler(IO_CLOSE_EFFECT, "close", VelnRuntime::hostClose);
            HandlerFrame writeAt = hostHandler(IO_WRITE_AT_EFFECT, null, null);
            HandlerFrame sync = hostHandler(IO_SYNC_EFFECT, null, null);
            synchronized (filesystem.owner.resources) {
                if (!filesystem.owner.scope.ioProviderScopeClosed) {
                    long slot = nextHostSlot(filesystem.owner);
                    IoResource resource = new IoResource(
                        filesystem.owner,
                        slot,
                        "WriteFile",
                        null,
                        write,
                        seek,
                        null,
                        writeAt,
                        close,
                        sync,
                        filesystem.mutate,
                        host
                    );
                    filesystem.owner.resources.put(Long.valueOf(slot), resource);
                    return ioProviderWriteFile(resource);
                }
            }
            try {
                host.releaseRejected();
            } catch (java.io.IOException ignored) {
                // The channel marks itself closed before provider-specific close work.
            }
            return ioProviderInvalidResource("open_write", "provider owner scope is closed");
        } catch (java.io.IOException | RuntimeException failure) {
            return hostFilesystemFailure("open_write", failure, path);
        }
    }

    public static Object fsSystemOpen(Object rootValue) {
        HandlerFrame scope = findHandler(IO_PROVIDER_SCOPE_EFFECT);
        if (scope == null || scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("system", "filesystem owner scope is unavailable");
        }
        java.nio.file.Path root;
        try {
            root = asPath(rootValue).toAbsolutePath().normalize();
            if (!java.nio.file.Files.isDirectory(root)) {
                return err(ioProviderError("system", "NotDirectory", "filesystem root is not a directory"));
            }
        } catch (RuntimeException failure) {
            return hostFilesystemFailure("system", failure, null);
        }
        IoOwner owner;
        synchronized (scope) {
            if (scope.ioProviderScopeClosed) {
                return ioProviderInvalidResource("system", "filesystem owner scope is unavailable");
            }
            if (scope.ioOwner == null) scope.ioOwner = new IoOwner(scope);
            owner = scope.ioOwner;
        }
        HandlerFrame read = hostHandler(FS_READ_EFFECT, null, null);
        HandlerFrame mutate = hostHandler(FS_MUTATE_EFFECT, "open_write", VelnRuntime::hostOpenWrite);
        synchronized (owner.resources) {
            if (scope.ioProviderScopeClosed) {
                return ioProviderInvalidResource("system", "filesystem owner scope is unavailable");
            }
            long slot = nextHostSlot(owner);
            IoMutableFilesystem filesystem = new IoMutableFilesystem(owner, slot, read, mutate, root);
            owner.resources.put(Long.valueOf(slot), filesystem);
            return ok(filesystem);
        }
    }

    private static boolean ioProviderOwnerIsLexicallyCurrent(IoOwner owner) {
        return findHandler(IO_PROVIDER_SCOPE_EFFECT) == owner.scope;
    }

    private static boolean ioProviderOwnerIsAccessible(IoOwner owner) {
        return ioProviderOwnerIsLexicallyCurrent(owner)
            || IO_PROVIDER_INVOCATION_OWNER.get() == owner;
    }

    private static boolean ioProviderOrdinaryAccess(IoOwner owner) {
        return ioProviderOwnerIsAccessible(owner) && !owner.scope.ioProviderScopeClosed;
    }

    private static boolean ioProviderReleaseAccess(IoResource resource) {
        IoResource releasing = IO_PROVIDER_RELEASE_RESOURCE.get();
        return releasing == resource;
    }

    private static void retainIoProviderReleaseFailure(IoOwner owner, Object failure) {
        synchronized (owner.scope.ioProviderReleaseFailures) {
            owner.scope.ioProviderReleaseFailures.add(failure);
        }
    }

    private static Object releaseIoProviderResource(IoResource resource) {
        IoResource previousResource = IO_PROVIDER_RELEASE_RESOURCE.get();
        IoOwner previousAdmission = IO_PROVIDER_ADMISSION_OWNER.get();
        IO_PROVIDER_RELEASE_RESOURCE.set(resource);
        IO_PROVIDER_ADMISSION_OWNER.remove();
        Object outcome;
        try {
            outcome = invokeHandler(
                resource.close,
                "close",
                new Object[] { new IoCapability(resource, resource.close) }
            );
        } catch (TaskCancellationFailure cancelled) {
            throw cancelled;
        } catch (RuntimeFailure failure) {
            retainIoProviderReleaseFailure(resource.owner, failure);
            return failure;
        } finally {
            if (previousResource == null) IO_PROVIDER_RELEASE_RESOURCE.remove();
            else IO_PROVIDER_RELEASE_RESOURCE.set(previousResource);
            if (previousAdmission == null) IO_PROVIDER_ADMISSION_OWNER.remove();
            else IO_PROVIDER_ADMISSION_OWNER.set(previousAdmission);
        }
        if (!isAdt(outcome, "CloseOutcome::Closed")) {
            retainIoProviderReleaseFailure(resource.owner, outcome);
        }
        return outcome;
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
        java.util.List<IoResource> resources = new java.util.ArrayList<IoResource>();
        synchronized (owner.resources) {
            scope.ioProviderScopeClosed = true;
            for (Object registered : owner.resources.values()) {
                if (registered instanceof IoResource) resources.add((IoResource) registered);
            }
        }
        TaskCancellationFailure cancellation = null;
        for (IoResource resource : resources) {
            try {
                releaseIoProviderResource(resource);
            } catch (TaskCancellationFailure cancelled) {
                if (cancellation == null) cancellation = cancelled;
            } finally {
                synchronized (owner.resources) {
                    owner.resources.remove(Long.valueOf(resource.slot), resource);
                }
            }
        }
        if (cancellation != null) throw cancellation;
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
                if (IO_PROVIDER_ADMISSION_OWNER.get() != owner) {
                    return ioProviderInvalidResource("resource", "provider owner scope is closed");
                }
            } else if (owner.resources.containsKey(Long.valueOf(slot))) {
                return ioProviderInvalidInput("resource", "provider slot is already registered");
            }
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

        boolean releaseAfterAdmission = false;
        synchronized (owner.resources) {
            if (owner.scope.ioProviderScopeClosed) {
                if (IO_PROVIDER_ADMISSION_OWNER.get() == owner) {
                    releaseAfterAdmission = true;
                }
            } else if (owner.resources.containsKey(Long.valueOf(slot))) {
                return ioProviderInvalidInput("resource", "provider slot is already registered");
            } else {
                owner.resources.put(Long.valueOf(slot), resource);
                return ok(resource);
            }
        }
        if (releaseAfterAdmission) releaseIoProviderResource(resource);
        return ioProviderInvalidResource("resource", "provider owner scope is closed");
    }

    public static Object ioProviderReadFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("read_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (resource.owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("read_file", "provider owner scope is closed");
        }
        if (!resource.family.equals("ReadFile")) {
            return ioProviderUnsupported("read_file", "resource does not provide a ReadFile view");
        }
        return ok(adt(
            "fs::ReadFile::ReadFile",
            new Object[] {
                new IoCapability(resource, resource.read),
                new IoCapability(resource, resource.seek),
                new IoCapability(resource, resource.readAt),
                new IoCapability(resource, resource.close)
            }
        ));
    }

    public static Object ioProviderAppendFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("append_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (resource.owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("append_file", "provider owner scope is closed");
        }
        if (!resource.family.equals("AppendFile")) {
            return ioProviderUnsupported("append_file", "resource does not provide an AppendFile view");
        }
        return ok(adt(
            "fs::AppendFile::AppendFile",
            new Object[] {
                new IoCapability(resource, resource.write),
                new IoCapability(resource, resource.close),
                new IoCapability(resource, resource.sync)
            }
        ));
    }

    public static Object ioProviderWriteFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("write_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (resource.owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("write_file", "provider owner scope is closed");
        }
        if (!resource.family.equals("WriteFile")) {
            return ioProviderUnsupported("write_file", "resource does not provide a WriteFile view");
        }
        return ok(adt(
            "fs::WriteFile::WriteFile",
            new Object[] {
                new IoCapability(resource, resource.write),
                new IoCapability(resource, resource.seek),
                new IoCapability(resource, resource.writeAt),
                new IoCapability(resource, resource.close),
                new IoCapability(resource, resource.sync),
                new IoCapability(resource, resource.mutate)
            }
        ));
    }

    public static Object ioProviderUpdateFile(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("update_file", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (resource.owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("update_file", "provider owner scope is closed");
        }
        if (!resource.family.equals("UpdateFile")) {
            return ioProviderUnsupported("update_file", "resource does not provide an UpdateFile view");
        }
        return ok(adt(
            "fs::UpdateFile::UpdateFile",
            new Object[] {
                new IoCapability(resource, resource.read),
                new IoCapability(resource, resource.write),
                new IoCapability(resource, resource.seek),
                new IoCapability(resource, resource.readAt),
                new IoCapability(resource, resource.writeAt),
                new IoCapability(resource, resource.close),
                new IoCapability(resource, resource.sync),
                new IoCapability(resource, resource.mutate)
            }
        ));
    }

    public static Object ioProviderDuplex(Object resourceValue) {
        if (!(resourceValue instanceof IoResource)) {
            return ioProviderInvalidInput("duplex", "invalid provider resource");
        }
        IoResource resource = (IoResource) resourceValue;
        if (resource.owner.scope.ioProviderScopeClosed) {
            return ioProviderInvalidResource("duplex", "provider owner scope is closed");
        }
        if (!resource.family.equals("Duplex")) {
            return ioProviderUnsupported("duplex", "resource does not provide a Duplex view");
        }
        return ok(record(
            "reader", new IoCapability(resource, resource.read),
            "writer", new IoCapability(resource, resource.write),
            "closer", new IoCapability(resource, resource.close)
        ));
    }

    public static Object ioProviderSlot(Object owner, Object resource) {
        if (!(owner instanceof IoOwner) || !(resource instanceof IoResource)) {
            return ioProviderInvalidInput("slot", "invalid provider identity");
        }
        IoResource value = (IoResource) resource;
        return value.owner == owner
            && (ioProviderOrdinaryAccess(value.owner)
                || ioProviderReleaseAccess(value))
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
        return value.resource.owner == owner
            && (ioProviderOrdinaryAccess(value.resource.owner)
                || ioProviderReleaseAccess(value.resource))
            ? ok(Long.valueOf(value.resource.slot))
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

    private static IoOwner ioProviderOperandOwner(Object operand) {
        if (operand instanceof IoCapability) return ((IoCapability) operand).resource.owner;
        if (operand instanceof IoFilesystem) return ((IoFilesystem) operand).owner;
        if (operand instanceof IoMutableFilesystem) return ((IoMutableFilesystem) operand).owner;
        return null;
    }

    private static boolean ioProviderOperandIsAvailable(String effect, Object operand) {
        HandlerFrame handler = ioProviderCapturedHandler(effect, operand);
        IoOwner owner = ioProviderOperandOwner(operand);
        return handler != null && owner != null && !owner.scope.ioProviderScopeClosed;
    }

    private static boolean ioProviderCanPublishResource(String effect, String operation) {
        return (effect.equals(FS_READ_EFFECT) && operation.equals("open_read"))
            || (effect.equals(FS_MUTATE_EFFECT)
                && (operation.equals("open_write")
                    || operation.equals("open_read_write")
                    || operation.equals("open_append")));
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
        IoOwner owner = ioProviderOperandOwner(operand);
        if (owner.scope.ioProviderScopeClosed) {
            return ioProviderForeignFailure(effect, operation);
        }
        if (trailing.length == 1) {
            if (effect.equals(IO_READ_EFFECT)
                && operation.equals("read")
                && byteCountLong(trailing[0]) == 0L) {
                return adt(
                    "io::ReadOutcome::Data",
                    new Object[] { byteChunkFromByteArray(new byte[] {}, 0) }
                );
            }
            if (effect.equals(IO_WRITE_EFFECT)
                && operation.equals("write")
                && byteChunkBytes(trailing[0]).isEmpty()) {
                return adt(
                    "io::WriteOutcome::Written",
                    new Object[] { byteCountObject(0L) }
                );
            }
        }
        Object[] args = new Object[trailing.length + 1];
        args[0] = operand;
        System.arraycopy(trailing, 0, args, 1, trailing.length);
        IoOwner previous = IO_PROVIDER_INVOCATION_OWNER.get();
        IoOwner previousAdmission = IO_PROVIDER_ADMISSION_OWNER.get();
        boolean admitsPublication = ioProviderCanPublishResource(effect, operation);
        IO_PROVIDER_INVOCATION_OWNER.set(owner);
        if (admitsPublication) IO_PROVIDER_ADMISSION_OWNER.set(owner);
        else IO_PROVIDER_ADMISSION_OWNER.remove();
        try {
            return invokeHandler(handler, operation, args);
        } finally {
            if (previous == null) IO_PROVIDER_INVOCATION_OWNER.remove();
            else IO_PROVIDER_INVOCATION_OWNER.set(previous);
            if (previousAdmission == null) IO_PROVIDER_ADMISSION_OWNER.remove();
            else IO_PROVIDER_ADMISSION_OWNER.set(previousAdmission);
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
        IoOwner owner = ioProviderOperandOwner(args[0]);
        HandlerFrame lexicalScope = findHandler(IO_PROVIDER_SCOPE_EFFECT);
        return captured != null && (owner == null || owner.scope.ioProviderScopeClosed
            || captured != selected || owner.scope != lexicalScope)
            ? ioProviderForeignFailure(effect, operation)
            : null;
    }

    private static IoOwner ioProviderLexicalAdmissionOwner(
        String effect,
        String operation,
        Object[] args,
        HandlerFrame selected
    ) {
        if (args.length == 0 || !ioProviderCanPublishResource(effect, operation)) return null;
        HandlerFrame captured = ioProviderCapturedHandler(effect, args[0]);
        if (captured == null || captured != selected) return null;
        if (args[0] instanceof IoFilesystem) return ((IoFilesystem) args[0]).owner;
        if (args[0] instanceof IoMutableFilesystem) return ((IoMutableFilesystem) args[0]).owner;
        return null;
    }

    public static Object ioReadExact(Object reader, Object countValue) {
        long requested = byteCountLong(countValue);
        java.util.ArrayList<Object> accumulated = new java.util.ArrayList<Object>();
        if (requested < 0L || requested > Integer.MAX_VALUE) {
            Object error = ioProviderError(
                "read_exact",
                "InvalidInput",
                "exact read count must be between zero and 2147483647"
            );
            return adt(
                "io::ExactOutcome::ExactFailed",
                new Object[] {
                    byteChunkObject(accumulated),
                    adt("io::Progress::Known", new Object[] { byteCountObject(0L) }),
                    error
                }
            );
        }
        if (requested == 0L) {
            if (!ioProviderOperandIsAvailable(IO_READ_EFFECT, reader)) {
                Object error = ioProviderError(
                    "read_exact",
                    "InvalidResource",
                    "reader is not available"
                );
                return adt(
                    "io::ExactOutcome::ExactFailed",
                    new Object[] {
                        byteChunkObject(accumulated),
                        adt("io::Progress::Known", new Object[] { byteCountObject(0L) }),
                        error
                    }
                );
            }
            return adt(
                "io::ExactOutcome::Complete",
                new Object[] { byteChunkObject(accumulated) }
            );
        }

        long delivered = 0L;
        while (delivered < requested) {
            long remaining = requested - delivered;
            Object read = ioNormalizeCompleteRead(
                ioProviderDispatch2(IO_READ_EFFECT, "read", reader, byteCountObject(remaining)),
                remaining,
                delivered,
                "read_exact",
                "provider read returned more bytes than requested"
            );
            if (isAdt(read, "ReadOutcome::End")) {
                return adt(
                    "io::ExactOutcome::Short",
                    new Object[] { byteChunkObject(accumulated) }
                );
            }
            if (isAdt(read, "ReadOutcome::ReadFailed")) {
                Object failureBytes = adtPayload(read, 0);
                accumulated.addAll(byteChunkBytes(failureBytes));
                return adt(
                    "io::ExactOutcome::ExactFailed",
                    new Object[] {
                        byteChunkObject(accumulated),
                        adtPayload(read, 1),
                        adtPayload(read, 2)
                    }
                );
            }
            Object bytes = adtPayload(read, 0);
            long byteCount = byteChunkBytes(bytes).size();
            accumulated.addAll(byteChunkBytes(bytes));
            delivered += byteCount;
        }
        return adt(
            "io::ExactOutcome::Complete",
            new Object[] { byteChunkObject(accumulated) }
        );
    }

    private static Object ioNormalizeCompleteRead(
        Object read,
        long remaining,
        long consumed,
        String operation,
        String overflowMessage
    ) {
        if (isAdt(read, "ReadOutcome::End")) return read;
        if (isAdt(read, "ReadOutcome::ReadFailed")) {
            Object bytes = adtPayload(read, 0);
            Object progress = adtPayload(read, 1);
            long byteCount = byteChunkBytes(bytes).size();
            long knownPrefix = byteCountLong(adtPayload(progress, 0));
            if (knownPrefix < 0L
                || byteCount > knownPrefix
                || knownPrefix > remaining
                || consumed > Long.MAX_VALUE - knownPrefix) {
                return ioCompleteReadFailure(
                    consumed,
                    operation,
                    "InvalidResource",
                    "provider read returned invalid progress",
                    true
                );
            }
            Object accumulated = adt(
                isAdt(progress, "Progress::Unknown")
                    ? "io::Progress::Unknown"
                    : "io::Progress::Known",
                new Object[] { byteCountObject(consumed + knownPrefix) }
            );
            return adt(
                "io::ReadOutcome::ReadFailed",
                new Object[] { bytes, accumulated, adtPayload(read, 2) }
            );
        }
        if (!isAdt(read, "ReadOutcome::Data")) {
            throw new RuntimeFailure("provider read handler returned an invalid outcome");
        }
        Object bytes = adtPayload(read, 0);
        long byteCount = byteChunkBytes(bytes).size();
        if (byteCount > remaining) {
            return ioCompleteReadFailure(
                consumed,
                operation,
                "InvalidResource",
                "provider read returned more bytes than requested",
                true
            );
        }
        if (consumed > Long.MAX_VALUE - byteCount) {
            return ioCompleteReadFailure(
                consumed,
                operation,
                "InvalidResource",
                overflowMessage,
                true
            );
        }
        if (byteCount == 0L) {
            return ioCompleteReadFailure(
                consumed,
                operation,
                "NoProgress",
                "provider read returned an empty data chunk",
                false
            );
        }
        return read;
    }

    private static Object ioCompleteReadFailure(
        long consumed,
        String operation,
        String kind,
        String message,
        boolean progressUnknown
    ) {
        Object progress = adt(
            progressUnknown ? "io::Progress::Unknown" : "io::Progress::Known",
            new Object[] { byteCountObject(consumed) }
        );
        return adt(
            "io::ReadOutcome::ReadFailed",
            new Object[] {
                byteChunkFromByteArray(new byte[] {}, 0),
                progress,
                ioProviderError(operation, kind, message)
            }
        );
    }

    public static Object ioWriteAll(Object writer, Object bytes) {
        long requested = byteChunkBytes(bytes).size();
        if (requested == 0L) {
            if (!ioProviderOperandIsAvailable(IO_WRITE_EFFECT, writer)) {
                Object error = ioProviderError("write_all", "InvalidResource", "writer is not available");
                Object progress = adt(
                    "io::Progress::Known",
                    new Object[] { byteCountObject(0L) }
                );
                return adt(
                    "io::WriteOutcome::WriteFailed",
                    new Object[] { progress, error }
                );
            }
            return adt(
                "io::WriteOutcome::Written",
                new Object[] { byteCountObject(0L) }
            );
        }

        return ioWriteCompletely(writer, bytes, 0L, "write_all");
    }

    private static Object ioWriteCompletely(
        Object writer,
        Object bytes,
        long committed,
        String operation
    ) {
        int remaining = byteChunkBytes(bytes).size();
        Object suffix = bytes;
        while (remaining > 0) {
            Object write = ioProviderDispatch2(IO_WRITE_EFFECT, "write", writer, suffix);
            if (isAdt(write, "WriteOutcome::WriteFailed")) {
                Object failureProgress = adtPayload(write, 0);
                long knownPrefix = byteCountLong(adtPayload(failureProgress, 0));
                if (knownPrefix < 0L
                    || knownPrefix > remaining
                    || committed > Long.MAX_VALUE - knownPrefix) {
                    return ioCompleteWriteInvalidResource(
                        operation,
                        committed,
                        "provider write returned invalid progress"
                    );
                }
                Object progress = adt(
                    isAdt(failureProgress, "Progress::Unknown")
                        ? "io::Progress::Unknown"
                        : "io::Progress::Known",
                    new Object[] { byteCountObject(committed + knownPrefix) }
                );
                return adt(
                    "io::WriteOutcome::WriteFailed",
                    new Object[] { progress, adtPayload(write, 1) }
                );
            }
            if (!isAdt(write, "WriteOutcome::Written")) {
                throw new RuntimeFailure("provider write handler returned an invalid outcome");
            }
            long written = byteCountLong(adtPayload(write, 0));
            if (written < 0L
                || written > remaining
                || committed > Long.MAX_VALUE - written) {
                return ioCompleteWriteInvalidResource(
                    operation,
                    committed,
                    "provider write returned invalid progress"
                );
            }
            if (written == 0L) {
                Object error = ioProviderError(
                    operation,
                    "NoProgress",
                    "provider write returned zero progress"
                );
                Object progress = adt(
                    "io::Progress::Known",
                    new Object[] { byteCountObject(committed) }
                );
                return adt(
                    "io::WriteOutcome::WriteFailed",
                    new Object[] { progress, error }
                );
            }
            committed += written;
            remaining -= (int) written;
            if (remaining > 0) {
                suffix = byteChunkDerivedObject(suffix, (int) written, remaining);
            }
        }
        return adt(
            "io::WriteOutcome::Written",
            new Object[] { byteCountObject(committed) }
        );
    }

    private static Object ioCompleteWriteInvalidResource(
        String operation,
        long committed,
        String message
    ) {
        Object error = ioProviderError(operation, "InvalidResource", message);
        Object progress = adt(
            "io::Progress::Unknown",
            new Object[] { byteCountObject(committed) }
        );
        return adt(
            "io::WriteOutcome::WriteFailed",
            new Object[] { progress, error }
        );
    }

    public static Object ioCopy(Object reader, Object writer, Object chunkSize) {
        long requested = byteCountLong(chunkSize);
        if (requested <= 0L) {
            Object zero = adt("io::Progress::Known", new Object[] { byteCountObject(0L) });
            Object error = ioProviderError("copy", "InvalidInput", "copy chunk size must be positive");
            return adt("io::CopyOutcome::CopyFailed", new Object[] { zero, zero, error });
        }

        long consumed = 0L;
        long committed = 0L;
        while (true) {
            Object read = ioNormalizeCompleteRead(
                ioProviderDispatch2(IO_READ_EFFECT, "read", reader, chunkSize),
                requested,
                consumed,
                "copy",
                "provider read progress overflowed"
            );
            if (isAdt(read, "ReadOutcome::End")) {
                return adt("io::CopyOutcome::Copied", new Object[] { byteCountObject(committed) });
            }
            if (isAdt(read, "ReadOutcome::ReadFailed")) {
                return adt(
                    "io::CopyOutcome::CopyFailed",
                    new Object[] {
                        adtPayload(read, 1),
                        adt("io::Progress::Known", new Object[] { byteCountObject(committed) }),
                        adtPayload(read, 2)
                    }
                );
            }
            Object bytes = adtPayload(read, 0);
            long count = byteChunkBytes(bytes).size();
            consumed += count;

            Object write = ioWriteCompletely(writer, bytes, committed, "copy");
            if (isAdt(write, "WriteOutcome::WriteFailed")) {
                return adt(
                    "io::CopyOutcome::CopyFailed",
                    new Object[] {
                        adt("io::Progress::Known", new Object[] { byteCountObject(consumed) }),
                        adtPayload(write, 0),
                        adtPayload(write, 1)
                    }
                );
            }
            committed = byteCountLong(adtPayload(write, 0));
        }
    }
