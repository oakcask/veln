    private static final ThreadLocal<Boolean> IO_PROVIDER_UPDATING =
        ThreadLocal.withInitial(() -> Boolean.FALSE);

    private static final class IoProviderState {
        private Object value;

        private IoProviderState(Object initial) {
            value = freezeBoundaryValue(initial);
        }
    }

    public static Object ioProviderState(Object initial) {
        throwTaskCancellationIfRequested();
        return new IoProviderState(initial);
    }

    public static Object ioProviderUpdate(Object storage, Object context, Object action) {
        if (!(storage instanceof IoProviderState)) {
            throw new RuntimeFailure("invalid provider state");
        }
        // Reject nested cell updates before taking another monitor. This includes
        // different cells, avoiding lock-order cycles between pure transitions.
        if (IO_PROVIDER_UPDATING.get().booleanValue()) {
            throw new RuntimeFailure("reentrant provider state update");
        }
        throwTaskCancellationIfRequested();
        IoProviderState state = (IoProviderState) storage;
        Object result;
        synchronized (state) {
            throwTaskCancellationIfRequested();
            IO_PROVIDER_UPDATING.set(Boolean.TRUE);
            try {
                Object transition = call(action, state.value, context);
                if (!(transition instanceof java.util.Map)) {
                    throw new RuntimeFailure("provider state transition must return state and result");
                }
                java.util.Map<?, ?> fields = (java.util.Map<?, ?>) transition;
                if (!fields.containsKey("state") || !fields.containsKey("result")) {
                    throw new RuntimeFailure("provider state transition must return state and result");
                }
                // Freeze both outputs before publishing either: a failed
                // transition, including failed freezing, preserves the old value.
                Object replacement = freezeBoundaryValue(fields.get("state"));
                result = freezeBoundaryValue(fields.get("result"));
                state.value = replacement;
            } finally {
                IO_PROVIDER_UPDATING.remove();
            }
        }
        throwTaskCancellationIfRequested();
        return result;
    }

    private static final class IoProviderOwner {
        private boolean closing;
        private int producers;
        private int activeCloses;
        private long nextHostSlot;
        private final java.util.List<IoProviderResource> resources = new java.util.ArrayList<>();
        private final java.util.List<IoProviderResource> unpublished = new java.util.ArrayList<>();
        private final java.util.List<Throwable> lateFailures = new java.util.ArrayList<>();
        private final java.util.Set<Long> slots = new java.util.HashSet<>();
    }

    private static final class IoProviderResource {
        private final IoProviderOwner owner;
        private final long slot;
        private final String family;
        private final java.util.Map<String, HandlerFrame> handlers;
        private final Object host;
        private volatile NetStream network;
        private boolean closed;
        private boolean unavailable;
        private boolean busy;
        private boolean closing;

        private IoProviderResource(IoProviderOwner owner, long slot, String family, java.util.Map<String, HandlerFrame> handlers, Object host) {
            this.owner = owner;
            this.slot = slot;
            this.family = family;
            this.handlers = handlers;
            this.host = host;
        }
    }

    private static final class IoProviderView {
        private final IoProviderResource resource;
        private final String capability;

        private IoProviderView(IoProviderResource resource, String capability) {
            this.resource = resource;
            this.capability = capability;
        }
    }

    private static final ThreadLocal<IoProviderResource> IO_PROVIDER_RELEASING = new ThreadLocal<>();
    private static final ThreadLocal<IoProviderOwner> IO_PROVIDER_OPERATION_OWNER = new ThreadLocal<>();

    private static Object ioError(String operation, String kind, String message) {
        return record("operation", operation, "kind", adt("ErrorKind::" + kind, new Object[] {}),
            "path", none(), "other_path", none(), "network", none(), "commit", none(), "message", message);
    }

    private static Object ioProviderFailure(String operation, String kind) {
        return err(ioError(operation, kind, kind));
    }

    private static String ioNativeTextError(String text) {
        for (int index = 0; index < text.length(); index++) {
            char unit = text.charAt(index);
            if (unit == 0) return "InvalidInput";
            if (Character.isHighSurrogate(unit)) {
                if (++index == text.length() || !Character.isLowSurrogate(text.charAt(index))) {
                    return "UnrepresentablePath";
                }
            } else if (Character.isLowSurrogate(unit)) {
                return "UnrepresentablePath";
            }
        }
        return null;
    }

    public static Object ioProviderNativePath(Object value) {
        String text = asString(value);
        String invalidKind = ioNativeTextError(text);
        if (invalidKind != null) return ioProviderFailure("native_path", invalidKind);
        try {
            return ok(pathValue(java.nio.file.Paths.get(text)));
        } catch (java.nio.file.InvalidPathException invalid) {
            return err(ioError("native_path", "InvalidInput", invalid.getMessage()));
        }
    }

    public static Object ioProviderDisplayNative(Object value) {
        return asPath(value).toString();
    }

    public static Object ioProviderOwner() {
        HandlerFrame frame = currentHandlerFrame();
        if (frame.ioOwner == null) throw new RuntimeFailure("provider owner requested outside its scope handler");
        return frame.ioOwner;
    }

    private static String ioFamily(Object value) {
        for (String family : new String[] {"ReadFile", "WriteFile", "UpdateFile", "AppendFile", "Duplex"}) {
            if (isAdt(value, "ViewFamily::" + family)) return family;
        }
        return null;
    }

    private static String[] ioFamilyCapabilities(String family) {
        switch (family) {
            case "ReadFile": return new String[] {"reader", "seeker", "reader_at", "closer"};
            case "WriteFile": return new String[] {"writer", "seeker", "writer_at", "closer", "syncer", "truncater"};
            case "UpdateFile": return new String[] {"reader", "writer", "seeker", "reader_at", "writer_at", "closer", "syncer", "truncater"};
            case "AppendFile": return new String[] {"writer", "closer", "syncer"};
            case "Duplex": return new String[] {"reader", "writer", "closer"};
            case "Filesystem": return new String[] {"filesystem"};
            case "MutableFilesystem": return new String[] {"filesystem", "mutable_filesystem"};
            default: throw new RuntimeFailure("invalid provider view family");
        }
    }

    private static String ioCapabilityEffect(String capability) {
        switch (capability) {
            case "reader": return "std::io::Read";
            case "writer": return "std::io::Write";
            case "seeker": return "std::io::Seek";
            case "reader_at": return "std::io::ReadAt";
            case "writer_at": return "std::io::WriteAt";
            case "closer": return "std::io::Close";
            case "syncer": return "std::io::Sync";
            case "filesystem": return "std::fs::Read";
            case "truncater":
            case "mutable_filesystem": return "std::fs::Mutate";
            default: throw new RuntimeFailure("invalid provider capability");
        }
    }

    private static String[] ioCapabilityOperations(String capability) {
        switch (capability) {
            case "reader": return new String[] {"read"};
            case "writer": return new String[] {"write"};
            case "seeker": return new String[] {"seek"};
            case "reader_at": return new String[] {"read_at"};
            case "writer_at": return new String[] {"write_at"};
            case "closer": return new String[] {"close"};
            case "syncer": return new String[] {"sync"};
            case "truncater": return new String[] {"truncate"};
            case "filesystem": return new String[] {"open_read", "metadata", "entries"};
            case "mutable_filesystem": return new String[] {"open_write", "open_read_write", "open_append", "create_directory", "remove_file", "remove_directory", "rename", "truncate"};
            default: throw new RuntimeFailure("invalid provider capability");
        }
    }

    private static Object ioProviderRegister(Object ownerValue, Object slotValue, String family) {
        return ioProviderRegister(ownerValue, slotValue, family, null);
    }

    private static Object ioProviderRegister(Object ownerValue, Object slotValue, String family, Object host) {
        if (!(ownerValue instanceof IoProviderOwner)) return ioProviderFailure("register", "InvalidResource");
        IoProviderOwner owner = (IoProviderOwner) ownerValue;
        long slot = asLong(slotValue);
        if (slot < 0L) return ioProviderFailure("register", "InvalidInput");
        HandlerFrame scope = findHandler("std::io_provider::Scope");
        IoProviderOwner operationOwner = IO_PROVIDER_OPERATION_OWNER.get();
        IoProviderOwner permittedOwner = operationOwner != null ? operationOwner : scope == null ? null : scope.ioOwner;
        if (permittedOwner != owner) return ioProviderFailure("register", "InvalidResource");
        java.util.Map<String, HandlerFrame> captured = new java.util.HashMap<>();
        for (String capability : ioFamilyCapabilities(family)) {
            HandlerFrame frame = findHandler(ioCapabilityEffect(capability));
            if (frame == null) return ioProviderFailure("register", "Unsupported");
            for (String operation : ioCapabilityOperations(capability)) {
                if (!frame.providers.containsKey(operation)) return ioProviderFailure("register", "Unsupported");
            }
            captured.put(capability, frame);
        }
        IoProviderResource resource = new IoProviderResource(owner, slot, family, captured, host);
        synchronized (owner) {
            if (!owner.closing) {
                if (!owner.slots.add(slot)) return ioProviderFailure("register", "InvalidResource");
                owner.resources.add(resource);
                return ok(resource);
            }
        }
        // A producer racing scope exit must release rather than publish its
        // newly opened resource. The close callback can still access its slot.
        if (captured.containsKey("closer")) {
            Object outcome;
            try {
                outcome = ioProviderCloseResource(resource);
            } catch (RuntimeException | Error failure) {
                ioProviderRetainLateFailure(resource, failure);
                throw failure;
            }
            if (isAdt(outcome, "CloseOutcome::CloseFailed")) {
                RuntimeFailure failure = ioProviderReleaseFailure("late resource release failed", outcome);
                ioProviderRetainLateFailure(resource, failure);
                throw failure;
            }
        }
        return ioProviderFailure("register", "Closed");
    }

    private static void ioProviderRetainLateFailure(IoProviderResource resource, Throwable failure) {
        synchronized (resource.owner) {
            if (!resource.closed) resource.owner.unpublished.add(resource);
            resource.owner.lateFailures.add(failure);
        }
    }

    private static RuntimeFailure ioProviderReleaseFailure(String message, Object outcome) {
        RuntimeFailure failure = new RuntimeFailure(message);
        attachCleanupFailure(failure, new RuntimeFailure("release outcome: " + format(outcome)));
        return failure;
    }

    public static Object ioProviderResource(Object owner, Object slot, Object family) {
        String selected = ioFamily(family);
        return selected == null ? ioProviderFailure("register", "InvalidInput") : ioProviderRegister(owner, slot, selected);
    }

    private static Object ioProviderNamespace(Object owner, Object slot, String family, String capability) {
        Result registered = (Result) ioProviderRegister(owner, slot, family);
        return registered.isOk() ? ok(new IoProviderView((IoProviderResource) registered.value(), capability)) : registered;
    }

    public static Object ioProviderFilesystem(Object owner, Object slot) {
        return ioProviderNamespace(owner, slot, "Filesystem", "filesystem");
    }

    public static Object ioProviderMutableFilesystem(Object owner, Object slot) {
        return ioProviderNamespace(owner, slot, "MutableFilesystem", "mutable_filesystem");
    }

    private static Object ioProviderProject(Object value, String family) {
        if (!(value instanceof IoProviderResource)) return ioProviderFailure("project", "InvalidResource");
        IoProviderResource resource = (IoProviderResource) value;
        if (!resource.family.equals(family)) return ioProviderFailure("project", "Unsupported");
        java.util.Map<String, Object> views = new java.util.LinkedHashMap<>();
        for (String capability : ioFamilyCapabilities(family)) views.put(capability, new IoProviderView(resource, capability));
        return ok(views);
    }

    public static Object ioProviderReadFile(Object resource) { return ioProviderProject(resource, "ReadFile"); }
    public static Object ioProviderWriteFile(Object resource) { return ioProviderProject(resource, "WriteFile"); }
    public static Object ioProviderUpdateFile(Object resource) { return ioProviderProject(resource, "UpdateFile"); }
    public static Object ioProviderAppendFile(Object resource) { return ioProviderProject(resource, "AppendFile"); }
    public static Object ioProviderDuplex(Object resource) { return ioProviderProject(resource, "Duplex"); }

    public static Object ioProviderSlot(Object ownerValue, Object value) {
        IoProviderResource resource = value instanceof IoProviderView ? ((IoProviderView) value).resource
            : value instanceof IoProviderResource ? (IoProviderResource) value : null;
        if (resource == null || ownerValue != resource.owner) return ioProviderFailure("slot", "InvalidResource");
        synchronized (resource.owner) {
            if (IO_PROVIDER_RELEASING.get() == resource) return ok(Long.valueOf(resource.slot));
            if (resource.owner.closing || resource.closed || resource.unavailable) return ioProviderFailure("slot", "Closed");
            return ok(Long.valueOf(resource.slot));
        }
    }

    private static Object ioProviderCloseResource(IoProviderResource resource) {
        return ioProviderCloseResource(resource, true);
    }

    private static Object ioProviderCloseResource(IoProviderResource resource, boolean allowClosingOwner) {
        boolean previouslyUnavailable;
        synchronized (resource.owner) {
            if (resource.closed) return adt("CloseOutcome::Closed", new Object[] {});
            if (!allowClosingOwner && resource.owner.closing) return ioDispatchFailure("close", "Closed");
            if (resource.closing) {
                return adt("CloseOutcome::CloseFailed", new Object[] {adt("Commit::No", new Object[] {}), ioError("close", "Busy", "close is already active")});
            }
            previouslyUnavailable = resource.unavailable;
            resource.closing = true;
            resource.unavailable = true;
            resource.owner.activeCloses++;
        }
        IoProviderResource previous = IO_PROVIDER_RELEASING.get();
        IoProviderOwner previousOwner = IO_PROVIDER_OPERATION_OWNER.get();
        IO_PROVIDER_RELEASING.set(resource);
        IO_PROVIDER_OPERATION_OWNER.set(resource.owner);
        try {
            Object outcome = invokeHandler(resource.handlers.get("closer"), "close", new Object[] {new IoProviderView(resource, "closer")});
            synchronized (resource.owner) {
                if (isAdt(outcome, "CloseOutcome::Closed")
                    || isAdt(outcome, "CloseOutcome::CloseFailed") && isAdt(adtPayload(outcome, 0), "Commit::Yes")) {
                    resource.closed = true;
                } else if (isAdt(outcome, "CloseOutcome::CloseFailed") && isAdt(adtPayload(outcome, 0), "Commit::No")) {
                    resource.unavailable = previouslyUnavailable;
                }
            }
            return outcome;
        } finally {
            if (previous == null) IO_PROVIDER_RELEASING.remove();
            else IO_PROVIDER_RELEASING.set(previous);
            if (previousOwner == null) IO_PROVIDER_OPERATION_OWNER.remove();
            else IO_PROVIDER_OPERATION_OWNER.set(previousOwner);
            synchronized (resource.owner) {
                resource.closing = false;
                resource.owner.activeCloses--;
                resource.owner.notifyAll();
            }
        }
    }

    private static void cleanupIoProvider(IoProviderOwner owner) {
        java.util.List<IoProviderResource> resources;
        int activeProducers;
        int activeCloses;
        boolean interrupted = false;
        synchronized (owner) {
            owner.closing = true;
            long deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(1L);
            while (owner.producers != 0 || owner.activeCloses != 0) {
                long remaining = deadline - System.nanoTime();
                if (remaining <= 0L) break;
                try {
                    java.util.concurrent.TimeUnit.NANOSECONDS.timedWait(owner, remaining);
                } catch (InterruptedException cancellation) {
                    interrupted = true;
                }
            }
            resources = new java.util.ArrayList<>(owner.resources);
        }
        RuntimeFailure failure = null;
        for (int index = resources.size() - 1; index >= 0; index--) {
            IoProviderResource resource = resources.get(index);
            if (!resource.handlers.containsKey("closer")) continue;
            try {
                Object outcome = ioProviderCloseResource(resource);
                if (isAdt(outcome, "CloseOutcome::CloseFailed")) {
                    throw ioProviderReleaseFailure("resource release failed", outcome);
                }
            } catch (RuntimeException closeFailure) {
                if (failure == null) failure = new RuntimeFailure("provider cleanup could not confirm resource closure");
                attachCleanupFailure(failure, closeFailure);
            }
        }
        synchronized (owner) {
            activeProducers = owner.producers;
            activeCloses = owner.activeCloses;
            for (Throwable lateFailure : owner.lateFailures) {
                if (failure == null) failure = new RuntimeFailure("provider cleanup could not confirm resource closure");
                attachCleanupFailure(failure, lateFailure);
            }
        }
        if (activeProducers != 0) {
            if (failure == null) failure = new RuntimeFailure("provider cleanup could not confirm resource closure");
            attachCleanupFailure(failure, new RuntimeFailure(activeProducers + " resource producer(s) still active"));
        }
        if (activeCloses != 0) {
            if (failure == null) failure = new RuntimeFailure("provider cleanup could not confirm resource closure");
            attachCleanupFailure(failure, new RuntimeFailure(activeCloses + " close operation(s) still active"));
        }
        if (interrupted) Thread.currentThread().interrupt();
        try {
            throwTaskCancellationIfRequested();
        } catch (RuntimeException cancellation) {
            if (failure != null) attachCleanupFailure(cancellation, failure);
            throw cancellation;
        }
        if (failure != null) throw failure;
    }

    public static Object ioProviderReadOnly(Object value) {
        if (!(value instanceof IoProviderView) || !((IoProviderView) value).capability.equals("mutable_filesystem")) {
            throw new RuntimeFailure("invalid mutable filesystem");
        }
        return new IoProviderView(((IoProviderView) value).resource, "filesystem");
    }

    private static Object ioDispatchFailure(String operation, String kind) {
        Object error = ioError(operation, kind, kind);
        Object zero = adt("Progress::Known", new Object[] {byteCountObject(0L)});
        switch (operation) {
            case "read": case "read_at":
                return adt("ReadOutcome::ReadFailed", new Object[] {byteChunkObject(java.util.Collections.emptyList()), zero, error});
            case "write": case "write_at":
                return adt("WriteOutcome::WriteFailed", new Object[] {zero, error});
            case "close":
                return adt("CloseOutcome::CloseFailed", new Object[] {adt("Commit::No", new Object[] {}), error});
            case "truncate": case "create_directory": case "remove_file": case "remove_directory": case "rename":
                return adt("MutationOutcome::MutationFailed", new Object[] {adt("Commit::No", new Object[] {}), error});
            default: return err(error);
        }
    }

    private static Object ioProviderDispatch(Object value, String capability, String operation, Object[] arguments) {
        if (!(value instanceof IoProviderView)) return ioDispatchFailure(operation, "InvalidResource");
        IoProviderView view = (IoProviderView) value;
        if (!view.capability.equals(capability)) return ioDispatchFailure(operation, "InvalidResource");
        IoProviderResource resource = view.resource;
        boolean namespace = capability.equals("filesystem") || capability.equals("mutable_filesystem");
        boolean producer = namespace && operation.startsWith("open_");
        if (operation.equals("close")) {
            Object result = ioProviderCloseResource(resource, false);
            throwTaskCancellationIfRequested();
            return result;
        }
        synchronized (resource.owner) {
            if (resource.owner.closing || resource.closed || resource.unavailable) return ioDispatchFailure(operation, "Closed");
            String networkFailure = ioProviderNetworkUnavailable(resource.network, operation);
            if (networkFailure != null) return ioDispatchFailure(operation, networkFailure);
            Object immediate = ioProviderImmediate(operation, arguments);
            if (immediate != null) return immediate;
            if (!namespace && resource.busy) return ioDispatchFailure(operation, "Busy");
            if (!namespace) resource.busy = true;
            if (producer) resource.owner.producers++;
        }
        try {
            Object[] operands = new Object[arguments.length + 1];
            operands[0] = value;
            System.arraycopy(arguments, 0, operands, 1, arguments.length);
            IoProviderOwner previousOwner = IO_PROVIDER_OPERATION_OWNER.get();
            Object result;
            IO_PROVIDER_OPERATION_OWNER.set(resource.owner);
            try {
                result = invokeHandler(resource.handlers.get(capability), operation, operands);
            } finally {
                if (previousOwner == null) IO_PROVIDER_OPERATION_OWNER.remove();
                else IO_PROVIDER_OPERATION_OWNER.set(previousOwner);
            }
            if ((operation.equals("read") || operation.equals("read_at")) && isAdt(result, "ReadOutcome::Data")) {
                long count = byteChunkBytes(adtPayload(result, 0)).size();
                long requested = byteCountLong(arguments[arguments.length - 1]);
                if (count > requested) return ioInvalidReplyFailure(operation);
                if (count == 0L && requested > 0L) return ioDispatchFailure(operation, "NoProgress");
            }
            if ((operation.equals("write") || operation.equals("write_at")) && isAdt(result, "WriteOutcome::Written")) {
                long count = byteCountLong(adtPayload(result, 0));
                if (count < 0L || count > byteChunkBytes(arguments[arguments.length - 1]).size()) return ioInvalidReplyFailure(operation);
                if (count == 0L && !byteChunkBytes(arguments[arguments.length - 1]).isEmpty()) return ioDispatchFailure(operation, "NoProgress");
            }
            if ((operation.equals("read") || operation.equals("read_at")) && isAdt(result, "ReadOutcome::ReadFailed")) {
                long delivered = byteChunkBytes(adtPayload(result, 0)).size();
                long requested = byteCountLong(arguments[arguments.length - 1]);
                long prefix = byteCountLong(adtPayload(adtPayload(result, 1), 0));
                if (delivered > requested || prefix < delivered || prefix > requested) return ioInvalidReplyFailure(operation);
            }
            if ((operation.equals("write") || operation.equals("write_at")) && isAdt(result, "WriteOutcome::WriteFailed")) {
                long prefix = byteCountLong(adtPayload(adtPayload(result, 0), 0));
                if (prefix < 0L || prefix > byteChunkBytes(arguments[arguments.length - 1]).size()) return ioInvalidReplyFailure(operation);
            }
            return result;
        } finally {
            synchronized (resource.owner) {
                if (!namespace) resource.busy = false;
                if (producer) {
                    resource.owner.producers--;
                    resource.owner.notifyAll();
                }
            }
        }
    }

    public static Object ioProviderDispatchRead(Object reader, Object maximum) {
        return ioProviderDispatch(reader, "reader", "read", new Object[] {maximum});
    }

    public static Object ioProviderDispatchWrite(Object writer, Object bytes) {
        return ioProviderDispatch(writer, "writer", "write", new Object[] {bytes});
    }

    public static Object ioProviderDispatchSeek(Object seeker, Object origin) {
        return ioProviderDispatch(seeker, "seeker", "seek", new Object[] {origin});
    }

    public static Object ioProviderDispatchReadAt(Object reader, Object offset, Object maximum) {
        return ioProviderDispatch(reader, "reader_at", "read_at", new Object[] {offset, maximum});
    }

    public static Object ioProviderDispatchWriteAt(Object writer, Object offset, Object bytes) {
        return ioProviderDispatch(writer, "writer_at", "write_at", new Object[] {offset, bytes});
    }

    public static Object ioProviderDispatchClose(Object closer) {
        return ioProviderDispatch(closer, "closer", "close", new Object[] {});
    }

    public static Object ioProviderDispatchSync(Object syncer) {
        return ioProviderDispatch(syncer, "syncer", "sync", new Object[] {});
    }

    public static Object ioProviderDispatchOpenRead(Object filesystem, Object path) {
        return ioProviderDispatch(filesystem, "filesystem", "open_read", new Object[] {path});
    }

    public static Object ioProviderDispatchMetadata(Object filesystem, Object path, Object follow) {
        return ioProviderDispatch(filesystem, "filesystem", "metadata", new Object[] {path, follow});
    }

    public static Object ioProviderDispatchEntries(Object filesystem, Object path) {
        return ioProviderDispatch(filesystem, "filesystem", "entries", new Object[] {path});
    }

    public static Object ioProviderDispatchOpenWrite(Object filesystem, Object path, Object mode) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "open_write", new Object[] {path, mode});
    }

    public static Object ioProviderDispatchOpenReadWrite(Object filesystem, Object path, Object mode) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "open_read_write", new Object[] {path, mode});
    }

    public static Object ioProviderDispatchOpenAppend(Object filesystem, Object path, Object create) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "open_append", new Object[] {path, create});
    }

    public static Object ioProviderDispatchCreateDirectory(Object filesystem, Object path) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "create_directory", new Object[] {path});
    }

    public static Object ioProviderDispatchRemoveFile(Object filesystem, Object path) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "remove_file", new Object[] {path});
    }

    public static Object ioProviderDispatchRemoveDirectory(Object filesystem, Object path) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "remove_directory", new Object[] {path});
    }

    public static Object ioProviderDispatchRename(Object filesystem, Object from, Object to) {
        return ioProviderDispatch(filesystem, "mutable_filesystem", "rename", new Object[] {from, to});
    }

    public static Object ioProviderDispatchTruncate(Object file, Object size) {
        return ioProviderDispatch(file, "truncater", "truncate", new Object[] {size});
    }

    public static Object ioProviderJoinChunks(Object chunks) {
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<>();
        ListValue cursor = asListValue(chunks);
        while (cursor.cons) {
            bytes.addAll(byteChunkBytes(cursor.head));
            cursor = asListValue(cursor.tail);
        }
        return byteChunkObject(bytes);
    }

    private static Object ioInvalidReplyFailure(String operation) {
        Object error = ioError(operation, "InvalidResource", "provider reply exceeds the submitted transfer bounds");
        Object unknown = adt("Progress::Unknown", new Object[] {byteCountObject(0L)});
        if (operation.equals("read") || operation.equals("read_at")) {
            return adt("ReadOutcome::ReadFailed", new Object[] {byteChunkObject(java.util.Collections.emptyList()), unknown, error});
        }
        return adt("WriteOutcome::WriteFailed", new Object[] {unknown, error});
    }

    private static Object ioProviderImmediate(String operation, Object[] args) {
        if (operation.equals("read_at") || operation.equals("write_at")) {
            if (asLong(byteOffsetToInt(args[0])) < 0L) return ioDispatchFailure(operation, "InvalidInput");
        }
        if (operation.equals("truncate") && asLong(byteOffsetToInt(args[0])) < 0L) {
            return ioDispatchFailure(operation, "InvalidInput");
        }
        if (operation.equals("seek") && isAdt(args[0], "SeekOrigin::Start")
            && asLong(byteOffsetToInt(adtPayload(args[0], 0))) < 0L) {
            return ioDispatchFailure(operation, "InvalidInput");
        }
        if (operation.equals("read") || operation.equals("read_at")) {
            long count = byteCountLong(args[args.length - 1]);
            if (count < 0L) return ioDispatchFailure(operation, "InvalidInput");
            if (count == 0L) return adt("ReadOutcome::Data", new Object[] {byteChunkObject(java.util.Collections.emptyList())});
        }
        if ((operation.equals("write") || operation.equals("write_at")) && byteChunkBytes(args[args.length - 1]).isEmpty()) {
            return adt("WriteOutcome::Written", new Object[] {byteCountObject(0L)});
        }
        if (operation.equals("open_write") || operation.equals("open_read_write")) {
            Object mode = args[1];
            if (isAdt(recordField(mode, "create"), "CreateMode::CreateNew") && Boolean.TRUE.equals(recordField(mode, "truncate"))) {
                return ioDispatchFailure(operation, "InvalidInput");
            }
        }
        return null;
    }

    public static Object ioProviderCompareText(Object leftValue, Object rightValue) {
        String left = asString(leftValue);
        String right = asString(rightValue);
        int a = 0;
        int b = 0;
        while (a < left.length() && b < right.length()) {
            int first = left.codePointAt(a);
            int second = right.codePointAt(b);
            if (first != second) return Long.valueOf(first < second ? -1L : 1L);
            a += Character.charCount(first);
            b += Character.charCount(second);
        }
        return Long.valueOf(a < left.length() ? 1L : b < right.length() ? -1L : 0L);
    }

    private static String ioProviderNetworkUnavailable(NetStream stream, String operation) {
        if (stream == null) return null;
        HandlerFrame owner = stream.systemOwner;
        if (owner == null || owner.networkSystemClosed) return "InvalidResource";
        // Read volatile facts without taking the stream monitor while holding
        // the byte owner monitor. A network close can invoke source handlers.
        if (stream.systemClosed || stream.closeUncertain) return "Closed";
        if (operation.equals("read") && (stream.readShutdown || stream.readShutdownUncertain || stream.readFailed)) return "Closed";
        if (operation.equals("write") && (stream.writeShutdown || stream.writeShutdownUncertain || stream.writeFailed)) return "Closed";
        return null;
    }
    private static HandlerFrame ioProviderNetworkOwner(Object value) {
        if (!(value instanceof NetStream)) return null;
        NetStream stream = (NetStream) value;
        HandlerFrame owner = stream.systemOwner;
        return owner == null || owner.networkSystemClosed ? null : owner;
    }
    public static Object ioProviderNetworkResource(Object byteOwner, Object value) {
        HandlerFrame owner = ioProviderNetworkOwner(value);
        if (owner == null) return ioProviderFailure("network", "InvalidResource");
        if (!owner.providers.containsKey("read_bounded") || !owner.providers.containsKey("write") || !owner.providers.containsKey("close_stream"))
            return ioProviderFailure("network", "Unsupported");
        if (netSystemStreamUnavailable((NetStream) value)) return ioProviderFailure("network", "Closed");
        if (!(byteOwner instanceof IoProviderOwner)) return ioProviderFailure("network", "InvalidResource");
        try {
            Object result = ioProviderRegister(byteOwner, Long.valueOf(ioHostSlot((IoProviderOwner) byteOwner)), "Duplex");
            if (result instanceof Result && ((Result) result).isOk())
                ((IoProviderResource) ((Result) result).value()).network = (NetStream) value;
            return result;
        } catch (IoHostFailure failure) {
            return ioProviderFailure("network", failure.kind);
        }
    }
    public static Object ioProviderNetworkRead(Object value, Object maximum) {
        HandlerFrame owner = ioProviderNetworkOwner(value);
        if (owner == null) return record("outcome", err(netSystemInvalidResource("read_bounded")), "attempted", Boolean.FALSE);
        return record("outcome", invokeHandler(owner, "read_bounded", new Object[] {value, maximum, none(), none()}), "attempted", Boolean.TRUE);
    }
    public static Object ioProviderNetworkWrite(Object value, Object bytes) {
        HandlerFrame owner = ioProviderNetworkOwner(value);
        if (owner == null) return netSystemWriteFailure(0L, netSystemInvalidResource("write"));
        return invokeHandler(owner, "write", new Object[] {value, bytes, none(), none()});
    }
    public static Object ioProviderNetworkClose(Object value) {
        NetStream stream = asNetStream(value);
        HandlerFrame owner = ioProviderNetworkOwner(value);
        if (owner == null) {
            if (stream.systemClosed && !stream.closeUncertain)
                return record("outcome", ok(UNIT), "released", ioHostCommit("Yes"));
            return record("outcome", err(netSystemInvalidResource("close_stream")), "released", ioHostCommit("No"));
        }
        Object outcome = invokeHandler(owner, "close_stream", new Object[] {value});
        Boolean released = null;
        if (outcome instanceof Result && ((Result) outcome).isOk()) released = Boolean.TRUE;
        else if (outcome instanceof Result) {
            Object failure = ((Result) outcome).value();
            if (failure instanceof DiagnosticResultValue) {
                TransportFailure transport = ((DiagnosticResultValue) failure).transportFailure;
                if (transport != null) released = transport.ownershipCommitted;
            }
        }
        return record("outcome", outcome, "released", ioHostCommit(released == null ? "Unknown" : released.booleanValue() ? "Yes" : "No"));
    }
