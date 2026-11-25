public final class HostFilesystemPublicationHarness extends IoProviderHarnessSupport {
    private static final java.lang.reflect.Field SCOPE_OWNER;
    private static final java.lang.reflect.Field SCOPE_RELEASE_FAILURES;
    private static final java.lang.reflect.Field RESOURCE_CLOSE;
    private static final java.lang.reflect.Field RESOURCE_HOST_WRITE;
    private static final java.lang.reflect.Field FRAME_PROVIDERS;
    private static final java.lang.reflect.Field HOST_CHANNEL;
    private static final java.lang.reflect.Field HOST_CLOSED;

    static {
        try {
            Class<?> scope = Class.forName("VelnRuntime$HandlerFrame");
            Class<?> resource = Class.forName("VelnRuntime$IoResource");
            Class<?> hostWrite = Class.forName("VelnRuntime$HostWriteResource");
            SCOPE_OWNER = scope.getDeclaredField("ioOwner");
            SCOPE_RELEASE_FAILURES = scope.getDeclaredField("ioProviderReleaseFailures");
            RESOURCE_CLOSE = resource.getDeclaredField("close");
            RESOURCE_HOST_WRITE = resource.getDeclaredField("hostWrite");
            FRAME_PROVIDERS = scope.getDeclaredField("providers");
            HOST_CHANNEL = hostWrite.getDeclaredField("channel");
            HOST_CLOSED = hostWrite.getDeclaredField("closed");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                SCOPE_OWNER,
                SCOPE_RELEASE_FAILURES,
                RESOURCE_CLOSE,
                RESOURCE_HOST_WRITE,
                FRAME_PROVIDERS,
                HOST_CHANNEL,
                HOST_CLOSED
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

    private static Object currentScope() throws ReflectiveOperationException {
        java.util.List<Object> frames = handlers();
        return frames.get(frames.size() - 1);
    }

    private static Object closed() {
        return VelnRuntime.adt("io::CloseOutcome::Closed", new Object[0]);
    }

    private static boolean isInvalidResource(Object value) {
        VelnRuntime.Result observed = result(value);
        return !observed.isOk() && observed.toString().contains("InvalidResource");
    }

    private static void pushOperation(
        String effect,
        String operation,
        VelnRuntime.Fn provider
    ) {
        VelnRuntime.pushHandler(
            effect,
            new Object[] { operation },
            new Object[] { provider },
            new Object[0]
        );
    }

    private static void awaitBlocked(Thread thread, String boundary) {
        long deadline = System.nanoTime() + 5_000_000_000L;
        while (thread.getState() != Thread.State.BLOCKED) {
            require(thread.isAlive(), boundary + " publisher exited before the barrier");
            require(System.nanoTime() < deadline, boundary + " publisher did not reach the barrier");
            Thread.onSpinWait();
        }
    }

    private static void join(Thread thread, String boundary) throws InterruptedException {
        thread.join(5000L);
        require(!thread.isAlive(), boundary + " publisher did not leave the barrier");
    }

    private static Thread systemPublisher(
        java.nio.file.Path root,
        java.util.concurrent.atomic.AtomicReference<Object> publication,
        java.util.concurrent.atomic.AtomicReference<Throwable> failure
    ) {
        return new Thread(() -> {
            try {
                publication.set(VelnRuntime.fsSystemOpen(VelnRuntime.fsPath(root.toString())));
            } catch (Throwable thrown) {
                failure.set(thrown);
            }
        });
    }

    private static void rejectsSystemPublicationAfterClosure(
        java.nio.file.Path root,
        boolean createOwner
    ) throws Exception {
        push("std::io_provider::Scope");
        Object scope = currentScope();
        Object existingOwner = createOwner ? owner() : null;
        int resourcesBefore = createOwner ? resources(existingOwner).size() : 0;
        java.util.concurrent.atomic.AtomicReference<Object> publication =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread child;

        synchronized (scope) {
            child = systemPublisher(root, publication, failure);
            child.start();
            awaitBlocked(child, createOwner ? "existing-owner closure" : "ownerless closure");
            VelnRuntime.popHandler();
        }
        join(child, createOwner ? "existing-owner closure" : "ownerless closure");

        require(failure.get() == null, "system publisher failed: " + failure.get());
        require(isInvalidResource(publication.get()), "closed scope accepted a system filesystem");
        require(
            SCOPE_OWNER.get(scope) == existingOwner,
            createOwner
                ? "closed scope replaced its existing owner"
                : "closed ownerless scope created an owner"
        );
        if (createOwner) {
            require(
                resources(existingOwner).size() == resourcesBefore,
                "closed existing owner changed its registry"
            );
        }
    }

    private static void preservesPublicNamespaceAtHostCandidate(
        java.nio.file.Path root
    ) throws Exception {
        push("std::io_provider::Scope");
        Object providerOwner = owner();
        push("std::fs::Read");
        push("std::fs::Mutate");
        java.util.Map<Long, Object> registry = resources(providerOwner);
        java.util.concurrent.atomic.AtomicReference<Object> publication =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        VelnRuntime.Result publicFilesystem;
        Thread child;

        synchronized (registry) {
            child = systemPublisher(root, publication, failure);
            child.start();
            awaitBlocked(child, "host namespace publication");
            publicFilesystem = result(VelnRuntime.ioProviderMutableFilesystem(
                providerOwner,
                Long.valueOf(Long.MAX_VALUE)
            ));
            require(publicFilesystem.isOk(), "public namespace registration failed");
        }
        join(child, "host namespace publication");

        require(failure.get() == null, "host namespace publisher failed: " + failure.get());
        VelnRuntime.Result hostFilesystem = result(publication.get());
        require(hostFilesystem.isOk(), "host namespace publication failed: " + hostFilesystem);
        require(
            registry.get(Long.valueOf(Long.MAX_VALUE)) == publicFilesystem.value(),
            "host namespace replaced the public candidate identity"
        );
        require(
            registry.get(Long.valueOf(Long.MAX_VALUE - 1L)) == hostFilesystem.value(),
            "host namespace did not select the next free slot"
        );
        require(registry.size() == 2, "namespace contention produced an inconsistent registry");

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
    }

    private static void preservesPublicResourceAtHostCandidate(
        java.nio.file.Path root
    ) throws Exception {
        java.nio.file.Path file = root.resolve("publication.bin");
        java.nio.file.Files.write(file, new byte[] { 1, 2, 3 });
        java.util.concurrent.atomic.AtomicInteger publicCloseCalls =
            new java.util.concurrent.atomic.AtomicInteger();

        push("std::io_provider::Scope");
        Object scope = currentScope();
        Object providerOwner = owner();
        pushOperation("std::fs::Mutate", "unused", (Object... supplied) -> null);
        pushOperation("std::io::Write", "write", (Object... supplied) ->
            VelnRuntime.adt(
                "io::WriteOutcome::Written",
                new Object[] { VelnRuntime.adt("ByteCount", new Object[] { Long.valueOf(0L) }) }
            )
        );
        pushOperation("std::io::Seek", "seek", (Object... supplied) -> VelnRuntime.ok(Long.valueOf(0L)));
        pushOperation("std::io::WriteAt", "write_at", (Object... supplied) -> null);
        pushOperation("std::io::Close", "close", (Object... supplied) -> {
            publicCloseCalls.incrementAndGet();
            return closed();
        });
        pushOperation("std::io::Sync", "sync", (Object... supplied) -> null);

        VelnRuntime.Result system = result(
            VelnRuntime.fsSystemOpen(VelnRuntime.fsPath(root.toString()))
        );
        require(system.isOk(), "host namespace setup failed: " + system);
        java.util.Map<Long, Object> registry = resources(providerOwner);
        require(
            registry.get(Long.valueOf(Long.MAX_VALUE)) == system.value(),
            "host namespace did not occupy its independently expected slot"
        );

        Object path = VelnRuntime.adt(
            "Path::PortablePath",
            new Object[] { "publication.bin" }
        );
        Object mode = VelnRuntime.adt(
            "WriteMode::WriteMode",
            new Object[] {
                VelnRuntime.adt("CreateMode::Existing", new Object[0]),
                Boolean.FALSE
            }
        );
        java.util.concurrent.atomic.AtomicReference<Object> publication =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        VelnRuntime.Result publicResource;
        Thread child;

        synchronized (registry) {
            child = new Thread(() -> {
                try {
                    publication.set(VelnRuntime.ioProviderDispatch3(
                        "std::fs::Mutate",
                        "open_write",
                        system.value(),
                        path,
                        mode
                    ));
                } catch (Throwable thrown) {
                    failure.set(thrown);
                }
            });
            child.start();
            awaitBlocked(child, "host write publication");
            publicResource = register(providerOwner, Long.MAX_VALUE - 1L, "WriteFile");
            require(publicResource.isOk(), "public write resource registration failed");
        }
        join(child, "host write publication");

        require(failure.get() == null, "host write publisher failed: " + failure.get());
        VelnRuntime.Result hostWriteFile = result(publication.get());
        require(hostWriteFile.isOk(), "host write publication failed: " + hostWriteFile);
        Object hostResource = registry.get(Long.valueOf(Long.MAX_VALUE - 2L));
        require(
            registry.get(Long.valueOf(Long.MAX_VALUE - 1L)) == publicResource.value(),
            "host write publication replaced the public candidate identity"
        );
        require(hostResource != null && hostResource != publicResource.value(),
            "host write publication did not select the next free slot");
        require(registry.size() == 3, "write contention produced an inconsistent registry");

        Object host = RESOURCE_HOST_WRITE.get(hostResource);
        java.nio.channels.FileChannel channel =
            (java.nio.channels.FileChannel) HOST_CHANNEL.get(host);
        require(channel.isOpen(), "published host channel was not open");
        java.util.concurrent.atomic.AtomicInteger hostCloseCalls =
            new java.util.concurrent.atomic.AtomicInteger();
        @SuppressWarnings("unchecked")
        java.util.Map<String, VelnRuntime.Fn> hostCloseProviders =
            (java.util.Map<String, VelnRuntime.Fn>) FRAME_PROVIDERS.get(
                RESOURCE_CLOSE.get(hostResource)
            );
        VelnRuntime.Fn hostClose = hostCloseProviders.get("close");
        hostCloseProviders.put("close", (Object... supplied) -> {
            hostCloseCalls.incrementAndGet();
            return hostClose.call(supplied);
        });

        for (int index = 0; index < 6; index += 1) VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        require(publicCloseCalls.get() == 1, "public resource close handler did not run exactly once");
        require(hostCloseCalls.get() == 1, "host resource close handler did not run exactly once");
        require(Boolean.TRUE.equals(HOST_CLOSED.get(host)), "host resource close handler did not mark closed");
        require(!channel.isOpen(), "host resource close handler left its channel open");
        require(registry.size() == 1, "scope exit left a byte resource registered");
        require(
            ((java.util.List<?>) SCOPE_RELEASE_FAILURES.get(scope)).isEmpty(),
            "scope exit retained an unexpected resource-release failure"
        );
        require(
            registry.get(Long.valueOf(Long.MAX_VALUE)) == system.value(),
            "scope exit changed the host namespace identity"
        );
    }

    public static void main(String[] args) throws Exception {
        java.nio.file.Path root = java.nio.file.Files.createTempDirectory("veln-host-publication");
        try {
            rejectsSystemPublicationAfterClosure(root, false);
            rejectsSystemPublicationAfterClosure(root, true);
            preservesPublicNamespaceAtHostCandidate(root);
            preservesPublicResourceAtHostCandidate(root);
        } finally {
            java.nio.file.Files.deleteIfExists(root.resolve("publication.bin"));
            java.nio.file.Files.deleteIfExists(root);
        }
    }
}
