    private static final class IoHostFilesystem {
        private final java.nio.file.Path root;
        private IoHostFilesystem(java.nio.file.Path root) { this.root = root; }
    }

    private static final class IoHostFile {
        private final java.nio.channels.FileChannel channel;
        private final Object path;
        private final boolean append;
        private long cursor;
        private boolean cursorKnown = true;
        private volatile boolean released;
        private IoHostFile(java.nio.channels.FileChannel channel, Object path, boolean append) {
            this.channel = channel;
            this.path = path;
            this.append = append;
        }
    }

    private static final class IoHostFailure extends RuntimeException {
        private final String kind;
        private IoHostFailure(String kind, String message) { super(message); this.kind = kind; }
    }

    private static long ioHostSlot(IoProviderOwner owner) {
        synchronized (owner) {
            while (owner.slots.contains(owner.nextHostSlot)) {
                if (owner.nextHostSlot == Long.MAX_VALUE) throw new IoHostFailure("InvalidResource", "resource slots exhausted");
                owner.nextHostSlot++;
            }
            if (owner.nextHostSlot == Long.MAX_VALUE) throw new IoHostFailure("InvalidResource", "resource slots exhausted");
            return owner.nextHostSlot++;
        }
    }

    private static String ioHostKind(Throwable failure) {
        if (failure instanceof IoHostFailure) return ((IoHostFailure) failure).kind;
        if (failure instanceof java.nio.file.NoSuchFileException) return "NotFound";
        if (failure instanceof java.nio.file.FileAlreadyExistsException) return "AlreadyExists";
        if (failure instanceof java.nio.file.AccessDeniedException || failure instanceof SecurityException) return "PermissionDenied";
        if (failure instanceof java.nio.file.NotDirectoryException) return "NotDirectory";
        if (failure instanceof java.nio.file.DirectoryNotEmptyException) return "DirectoryNotEmpty";
        if (failure instanceof java.nio.file.FileSystemLoopException) return "LinkLoop";
        if (failure instanceof java.nio.channels.ClosedByInterruptException) return "Cancelled";
        if (failure instanceof java.nio.channels.ClosedChannelException) return "Closed";
        if (failure instanceof UnsupportedOperationException) return "Unsupported";
        if (failure instanceof IllegalArgumentException || failure instanceof ArithmeticException) return "InvalidInput";
        return "Other";
    }

    private static Object ioHostError(String operation, Throwable failure, Object path, Object other, Object commit) {
        return record("operation", operation, "kind", adt("ErrorKind::" + ioHostKind(failure), new Object[] {}),
            "path", path == null ? none() : some(path), "other_path", other == null ? none() : some(other),
            "network", none(), "commit", commit == null ? none() : some(commit),
            "message", String.valueOf(failure.getMessage()));
    }

    private static Object ioHostCommit(String value) { return adt("Commit::" + value, new Object[] {}); }

    private static String ioHostFailureCommit(Throwable failure, boolean attempted) {
        if (!attempted) return "No";
        switch (ioHostKind(failure)) {
            case "NotFound": case "AlreadyExists": case "PermissionDenied": case "NotDirectory":
            case "IsDirectory": case "DirectoryNotEmpty": case "LinkLoop": case "Unsupported": case "InvalidInput":
                return "No";
            default: return "Unknown";
        }
    }

    private static Object ioHostPortable(String text) {
        return adt("Path::PortablePath", new Object[] {text});
    }

    private static java.nio.file.Path ioHostPath(IoHostFilesystem filesystem, Object portable) {
        String text = asString(adtPayload(portable, 0));
        java.nio.file.Path selected = filesystem.root;
        if (text.equals(".")) return selected;
        for (String component : text.split("/", -1)) {
            if (component.isEmpty() || component.equals(".") || component.equals("..") || component.indexOf(0) >= 0) {
                throw new IoHostFailure("InvalidInput", "invalid portable path component");
            }
            if (ioNativeTextError(component) != null) throw new IoHostFailure("UnrepresentablePath", "portable component is not a Unicode scalar name");
            java.nio.file.Path nativeComponent;
            try { nativeComponent = selected.getFileSystem().getPath(component); }
            catch (java.nio.file.InvalidPathException invalid) { throw new IoHostFailure("UnrepresentablePath", "portable component is not representable by the host"); }
            if (nativeComponent.getRoot() != null || nativeComponent.getNameCount() != 1 || !nativeComponent.toString().equals(component)) {
                throw new IoHostFailure("UnrepresentablePath", "portable component is not one exact native name");
            }
            selected = selected.resolve(nativeComponent);
        }
        return selected;
    }

    private static String ioHostEntryName(java.nio.file.Path nativeName) {
        String text = nativeName.toString();
        try {
            if (text.isEmpty() || text.equals(".") || text.equals("..") || text.indexOf('/') >= 0 || ioNativeTextError(text) != null
                || !nativeName.getFileSystem().getPath(text).equals(nativeName)) {
                throw new IoHostFailure("UnrepresentablePath", "native directory name has no exact portable representation");
            }
        } catch (java.nio.file.InvalidPathException invalid) {
            throw new IoHostFailure("UnrepresentablePath", "native directory name has no exact portable representation");
        }
        return text;
    }

    private static String ioHostEntryKind(java.nio.file.attribute.BasicFileAttributes attrs) {
        return attrs.isRegularFile() ? "RegularFile" : attrs.isDirectory() ? "Directory" : attrs.isSymbolicLink() ? "SymbolicLink" : "Other";
    }

    private static IoProviderResource ioHostResource(Object value, String capability) {
        if (!(value instanceof IoProviderView)) throw new IoHostFailure("InvalidResource", "expected a host capability");
        IoProviderView view = (IoProviderView) value;
        if (!view.capability.equals(capability) || view.resource.handlers.get(capability) != currentHandlerFrame()) {
            throw new IoHostFailure("InvalidResource", "capability belongs to another operation provider");
        }
        return view.resource;
    }

    public static Object ioProviderHostRequest(Object operationValue, Object receiver, Object request) {
        String operation = asString(operationValue);
        if (operation.equals("system")) return ioHostSystem(receiver, request);
        String capability;
        switch (operation) {
            case "open_read": case "metadata": case "entries": capability = "filesystem"; break;
            case "open_write": case "open_read_write": case "open_append": case "create_directory":
            case "remove_file": case "remove_directory": case "rename": capability = "mutable_filesystem"; break;
            case "read": capability = "reader"; break;
            case "read_at": capability = "reader_at"; break;
            case "write": capability = "writer"; break;
            case "write_at": capability = "writer_at"; break;
            case "seek": capability = "seeker"; break;
            case "truncate": capability = "truncater"; break;
            case "sync": capability = "syncer"; break;
            case "close": capability = "closer"; break;
            default: return ioDispatchFailure(operation, "Unsupported");
        }
        IoProviderResource resource;
        try { resource = ioHostResource(receiver, capability); }
        catch (IoHostFailure failure) { return ioDispatchFailure(operation, failure.kind); }
        if (resource.host instanceof IoHostFilesystem) {
            return ioHostNamespace(resource, operation, request);
        }
        if (resource.host instanceof IoHostFile) return ioHostFileRequest(resource, operation, request);
        return ioDispatchFailure(operation, "InvalidResource");
    }

    private static Object ioHostSystem(Object ownerValue, Object rootValue) {
        if (!(ownerValue instanceof IoProviderOwner)) return ioProviderFailure("system", "InvalidResource");
        IoProviderOwner owner = (IoProviderOwner) ownerValue;
        HandlerFrame scope = findHandler("std::io_provider::Scope");
        if (scope == null || scope.ioOwner != owner) return ioProviderFailure("system", "InvalidResource");
        synchronized (owner) {
            if (owner.closing) return ioProviderFailure("system", "Closed");
        }
        IoProviderOwner previous = IO_PROVIDER_OPERATION_OWNER.get();
        IO_PROVIDER_OPERATION_OWNER.set(owner);
        try {
            throwTaskCancellationIfRequested();
            java.nio.file.Path root = asPath(rootValue).toAbsolutePath().normalize();
            java.nio.file.attribute.BasicFileAttributes attrs = java.nio.file.Files.readAttributes(root, java.nio.file.attribute.BasicFileAttributes.class);
            if (!attrs.isDirectory()) throw new IoHostFailure("NotDirectory", "filesystem root is not a directory");
            Result registered = (Result) ioProviderRegister(owner, Long.valueOf(ioHostSlot(owner)), "MutableFilesystem", new IoHostFilesystem(root));
            return registered.isOk() ? ok(new IoProviderView((IoProviderResource) registered.value(), "mutable_filesystem")) : registered;
        } catch (java.io.IOException | SecurityException | IoHostFailure failure) {
            return err(ioHostError("system", failure, null, null, null));
        } finally {
            if (previous == null) IO_PROVIDER_OPERATION_OWNER.remove();
            else IO_PROVIDER_OPERATION_OWNER.set(previous);
        }
    }

    private static Object ioHostNamespace(IoProviderResource resource, String operation, Object request) {
        IoHostFilesystem filesystem = (IoHostFilesystem) resource.host;
        Object portable = recordField(request, "path");
        Object target = operation.equals("rename") ? recordField(request, "target") : null;
        boolean mutation = operation.equals("create_directory") || operation.equals("remove_file") || operation.equals("remove_directory") || operation.equals("rename");
        boolean attempted = false;
        boolean applied = false;
        try {
            throwTaskCancellationIfRequested();
            java.nio.file.Path path = ioHostPath(filesystem, portable);
            if (operation.startsWith("open_")) return ioHostOpen(resource.owner, operation, path, portable, request);
            if (operation.equals("metadata")) {
                boolean follow = isAdt(recordField(request, "follow"), "FollowLinks::Follow");
                java.nio.file.LinkOption[] options = follow ? new java.nio.file.LinkOption[0] : new java.nio.file.LinkOption[] {java.nio.file.LinkOption.NOFOLLOW_LINKS};
                java.nio.file.attribute.BasicFileAttributes attrs = java.nio.file.Files.readAttributes(path, java.nio.file.attribute.BasicFileAttributes.class, options);
                return ok(record("kind", adt("Kind::" + ioHostEntryKind(attrs), new Object[] {}),
                    "length", attrs.isRegularFile() ? some(byteCountObject(attrs.size())) : none()));
            }
            if (operation.equals("entries")) {
                java.util.TreeMap<String, Object> entries = new java.util.TreeMap<>((a, b) -> (int) asLong(ioProviderCompareText(a, b)));
                try (java.nio.file.DirectoryStream<java.nio.file.Path> listing = java.nio.file.Files.newDirectoryStream(path)) {
                    for (java.nio.file.Path entry : listing) {
                        String name = ioHostEntryName(entry.getFileName());
                        java.nio.file.attribute.BasicFileAttributes attrs = java.nio.file.Files.readAttributes(entry, java.nio.file.attribute.BasicFileAttributes.class, java.nio.file.LinkOption.NOFOLLOW_LINKS);
                        entries.putIfAbsent(name, record("name", ioHostPortable(name), "kind", adt("Kind::" + ioHostEntryKind(attrs), new Object[] {})));
                    }
                }
                Object list = listNil();
                for (Object entry : entries.descendingMap().values()) list = listCons(entry, list);
                return ok(list);
            }
            if (operation.equals("rename")) {
                java.nio.file.Path destination = ioHostPath(filesystem, target);
                java.nio.file.Files.readAttributes(path, java.nio.file.attribute.BasicFileAttributes.class, java.nio.file.LinkOption.NOFOLLOW_LINKS);
                try {
                    java.nio.file.Files.readAttributes(destination, java.nio.file.attribute.BasicFileAttributes.class, java.nio.file.LinkOption.NOFOLLOW_LINKS);
                    throw new java.nio.file.FileAlreadyExistsException(destination.toString());
                } catch (java.nio.file.NoSuchFileException absent) {
                    // The destination absence is only validation. It cannot
                    // authorize a later non-atomic move.
                }
                // ATOMIC_MOVE does not promise no replacement. A preflight
                // existence check cannot turn it into that atomic operation.
                throw new IoHostFailure("Unsupported", "host API has no atomic no-replace rename primitive");
            }
            if (operation.equals("create_directory")) {
                attempted = true;
                java.nio.file.Files.createDirectory(path);
            } else if (operation.equals("remove_file") || operation.equals("remove_directory")) {
                java.nio.file.attribute.BasicFileAttributes attrs = java.nio.file.Files.readAttributes(path, java.nio.file.attribute.BasicFileAttributes.class, java.nio.file.LinkOption.NOFOLLOW_LINKS);
                boolean directory = operation.equals("remove_directory");
                if (directory && !attrs.isDirectory()) throw new IoHostFailure("NotDirectory", "entry is not a directory");
                if (!directory && attrs.isDirectory()) throw new IoHostFailure("IsDirectory", "entry is a directory");
                java.nio.file.Path parent = path.getParent();
                if (parent == null || path.getFileName() == null) throw new IoHostFailure("InvalidInput", "cannot remove a filesystem root");
                try (java.nio.file.DirectoryStream<java.nio.file.Path> opened = java.nio.file.Files.newDirectoryStream(parent)) {
                    if (!(opened instanceof java.nio.file.SecureDirectoryStream)) throw new IoHostFailure("Unsupported", "host API cannot delete the selected entry kind atomically");
                    @SuppressWarnings("unchecked")
                    java.nio.file.SecureDirectoryStream<java.nio.file.Path> selected = (java.nio.file.SecureDirectoryStream<java.nio.file.Path>) opened;
                    attempted = true;
                    if (directory) selected.deleteDirectory(path.getFileName());
                    else selected.deleteFile(path.getFileName());
                    applied = true;
                }
            } else return ioDispatchFailure(operation, "Unsupported");
            return adt("MutationOutcome::Applied", new Object[] {});
        } catch (java.nio.file.DirectoryIteratorException failure) {
            return err(ioHostError(operation, failure.getCause(), portable, target, null));
        } catch (java.io.IOException | SecurityException | IllegalArgumentException | UnsupportedOperationException | IoHostFailure failure) {
            Object commit = mutation ? ioHostCommit(applied ? "Yes" : ioHostFailureCommit(failure, attempted)) : null;
            Object error = ioHostError(operation, failure, portable, target, commit);
            return mutation ? adt("MutationOutcome::MutationFailed", new Object[] {commit, error}) : err(error);
        }
    }

    private static Object ioHostOpen(IoProviderOwner owner, String operation, java.nio.file.Path path, Object portable, Object request) {
        boolean readOnly = operation.equals("open_read");
        boolean append = operation.equals("open_append");
        Object mode = !readOnly && !append ? recordField(request, "mode") : null;
        Object create = readOnly ? adt("CreateMode::Existing", new Object[] {}) : append ? recordField(request, "create") : recordField(mode, "create");
        boolean truncate = mode != null && Boolean.TRUE.equals(recordField(mode, "truncate"));
        boolean mayMutate = truncate || !isAdt(create, "CreateMode::Existing");
        boolean attempted = false;
        java.nio.channels.FileChannel channel = null;
        try {
            if (truncate && isAdt(create, "CreateMode::CreateNew")) throw new IoHostFailure("InvalidInput", "exclusive create cannot also request truncation");
            if (!isAdt(create, "CreateMode::CreateNew")) {
                try {
                    java.nio.file.attribute.BasicFileAttributes attrs = java.nio.file.Files.readAttributes(path, java.nio.file.attribute.BasicFileAttributes.class);
                    if (attrs.isDirectory()) throw new IoHostFailure("IsDirectory", "entry is a directory");
                    if (!attrs.isRegularFile()) throw new IoHostFailure("Unsupported", "entry cannot supply a seekable regular-file view");
                } catch (java.nio.file.NoSuchFileException missing) {
                    if (!mayMutate || isAdt(create, "CreateMode::Existing")) throw missing;
                }
            }
            java.util.Set<java.nio.file.OpenOption> options = new java.util.HashSet<>();
            if (readOnly || operation.equals("open_read_write")) options.add(java.nio.file.StandardOpenOption.READ);
            if (!readOnly) options.add(java.nio.file.StandardOpenOption.WRITE);
            if (append) options.add(java.nio.file.StandardOpenOption.APPEND);
            if (truncate) options.add(java.nio.file.StandardOpenOption.TRUNCATE_EXISTING);
            if (isAdt(create, "CreateMode::CreateNew")) options.add(java.nio.file.StandardOpenOption.CREATE_NEW);
            else if (isAdt(create, "CreateMode::CreateIfMissing")) options.add(java.nio.file.StandardOpenOption.CREATE);
            attempted = true;
            channel = java.nio.channels.FileChannel.open(path, options);
            String family = readOnly ? "ReadFile" : append ? "AppendFile" : operation.equals("open_read_write") ? "UpdateFile" : "WriteFile";
            Result registered = (Result) ioProviderRegister(owner, Long.valueOf(ioHostSlot(owner)), family, new IoHostFile(channel, portable, append));
            if (registered.isOk()) return ioProviderProject(registered.value(), family);
            Object error = registered.value();
            if (channel.isOpen()) channel.close();
            Object commit = ioHostCommit(truncate || isAdt(create, "CreateMode::CreateNew") ? "Yes" : mayMutate ? "Unknown" : "No");
            java.util.Map<String, Object> context = new java.util.LinkedHashMap<>();
            for (String field : new String[] {"operation", "kind", "path", "other_path", "network", "commit", "message"}) {
                context.put(field, recordField(error, field));
            }
            context.put("operation", operation);
            context.put("path", some(portable));
            context.put("commit", some(commit));
            return err(context);
        } catch (java.io.IOException | SecurityException | IllegalArgumentException | UnsupportedOperationException | IoHostFailure failure) {
            if (channel != null && channel.isOpen()) {
                try { channel.close(); } catch (java.io.IOException closeFailure) { failure.addSuppressed(closeFailure); }
            }
            Object commit = mayMutate ? ioHostCommit(ioHostFailureCommit(failure, attempted)) : ioHostCommit("No");
            return err(ioHostError(operation, failure, portable, null, commit));
        }
    }

    private static Object ioHostFileRequest(IoProviderResource resource, String operation, Object request) {
        IoHostFile file = (IoHostFile) resource.host;
        boolean positional = operation.equals("read_at") || operation.equals("write_at");
        java.nio.ByteBuffer buffer = null;
        boolean attempted = false;
        try {
            if (operation.equals("close") && file.released) return adt("CloseOutcome::Closed", new Object[] {});
            if (!file.channel.isOpen()) throw new IoHostFailure("Closed", "host file is closed");
            if (operation.equals("close")) {
                attempted = true;
                file.channel.close();
                file.released = true;
                return adt("CloseOutcome::Closed", new Object[] {});
            }
            if (operation.equals("sync")) {
                file.channel.force(true);
                return ok(UNIT);
            }
            if (operation.equals("seek")) {
                Object origin = recordField(request, "origin");
                boolean absolute = isAdt(origin, "SeekOrigin::Start");
                if (!absolute && !file.cursorKnown) throw new IoHostFailure("InvalidResource", "sequential cursor is unknown");
                long displacement = absolute ? byteOffsetLong(adtPayload(origin, 0)) : asLong(adtPayload(origin, 0));
                long base = absolute ? 0L : isAdt(origin, "SeekOrigin::Current") ? file.cursor : file.channel.size();
                long next = Math.addExact(base, displacement);
                if (next < 0L) throw new IoHostFailure("InvalidInput", "seek target is negative");
                file.cursor = next;
                file.cursorKnown = true;
                return ok(byteOffsetObject(next));
            }
            if (operation.equals("truncate")) {
                long size = byteOffsetLong(recordField(request, "size"));
                if (size < 0L) throw new IoHostFailure("InvalidInput", "truncate size is negative");
                if (size < file.channel.size()) {
                    attempted = true;
                    file.channel.truncate(size);
                    if (file.cursorKnown) file.cursor = Math.min(file.cursor, size);
                }
                return adt("MutationOutcome::Applied", new Object[] {});
            }
            if (!positional && !file.append && !file.cursorKnown) throw new IoHostFailure("InvalidResource", "sequential cursor is unknown");
            long position = positional ? byteOffsetLong(recordField(request, "offset")) : file.cursor;
            if (position < 0L) throw new IoHostFailure("InvalidInput", "file offset is negative");
            boolean read = operation.equals("read") || operation.equals("read_at");
            if (read) {
                long maximum = byteCountLong(recordField(request, "maximum"));
                if (maximum < 0L) throw new IoHostFailure("InvalidInput", "read count is negative");
                int size = (int) Math.min(maximum, 65536L);
                buffer = java.nio.ByteBuffer.allocate(size);
                attempted = true;
                int count = file.channel.read(buffer, position);
                if (count < 0) return adt("ReadOutcome::End", new Object[] {});
                if (!positional) file.cursor = Math.addExact(file.cursor, count);
                return adt("ReadOutcome::Data", new Object[] {ioHostBuffer(buffer)});
            }
            java.util.List<Object> bytes = byteChunkBytes(recordField(request, "bytes"));
            if (!file.append && bytes.size() > Long.MAX_VALUE - position) throw new IoHostFailure("InvalidInput", "write would overflow its byte offset");
            buffer = java.nio.ByteBuffer.allocate(Math.min(bytes.size(), 65536));
            for (int index = 0; index < buffer.capacity(); index++) buffer.put((byte) asLong(adtPayload(bytes.get(index), 0)));
            buffer.flip();
            attempted = true;
            int written = file.append ? file.channel.write(buffer) : file.channel.write(buffer, position);
            if (!positional && !file.append) file.cursor += written;
            return adt("WriteOutcome::Written", new Object[] {byteCountObject(written)});
        } catch (java.io.IOException | SecurityException | IllegalArgumentException | UnsupportedOperationException | ArithmeticException | IoHostFailure failure) {
            if (!file.channel.isOpen()) synchronized (resource.owner) {
                if (file.released) resource.closed = true;
                else resource.unavailable = true;
            }
            Object error = ioHostError(operation, failure, file.path, null, null);
            Object progress = adt("Progress::" + (attempted ? "Unknown" : "Known"), new Object[] {byteCountObject(buffer == null ? 0L : buffer.position())});
            if (operation.equals("read") || operation.equals("read_at")) {
                if (attempted && !positional) file.cursorKnown = false;
                return adt("ReadOutcome::ReadFailed", new Object[] {buffer == null ? byteChunkObject(java.util.Collections.emptyList()) : ioHostBuffer(buffer), progress, error});
            }
            if (operation.equals("write") || operation.equals("write_at")) {
                if (attempted && !positional && !file.append) file.cursorKnown = false;
                return adt("WriteOutcome::WriteFailed", new Object[] {progress, error});
            }
            if (operation.equals("close")) {
                Object released = ioHostCommit(file.released ? "Yes" : attempted || !file.channel.isOpen() ? "Unknown" : "No");
                return adt("CloseOutcome::CloseFailed", new Object[] {released, error});
            }
            if (operation.equals("truncate")) {
                if (attempted) file.cursorKnown = false;
                return adt("MutationOutcome::MutationFailed", new Object[] {ioHostCommit(ioHostFailureCommit(failure, attempted)), error});
            }
            return err(error);
        }
    }

    private static Object ioHostBuffer(java.nio.ByteBuffer buffer) {
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<>(buffer.position());
        for (int index = 0; index < buffer.position(); index++) bytes.add(byteObject(buffer.get(index) & 255L));
        return byteChunkObject(bytes);
    }
