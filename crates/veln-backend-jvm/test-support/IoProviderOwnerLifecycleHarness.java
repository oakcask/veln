public final class IoProviderOwnerLifecycleHarness {
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field INVOKED_HANDLER;

    static {
        try {
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            HANDLERS.setAccessible(true);
            INVOKED_HANDLER.setAccessible(true);
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    @SuppressWarnings("unchecked")
    private static java.util.List<Object> handlers() throws Exception {
        return (java.util.List<Object>) ((ThreadLocal<?>) HANDLERS.get(null)).get();
    }

    @SuppressWarnings("unchecked")
    private static Object owner() throws Exception {
        java.util.List<Object> frames = handlers();
        Object scope = frames.get(frames.size() - 1);
        ThreadLocal<Object> invoked = (ThreadLocal<Object>) INVOKED_HANDLER.get(null);
        invoked.set(scope);
        try {
            return VelnRuntime.ioProviderScopeOwner();
        } finally {
            invoked.remove();
        }
    }

    private static void push(String effect, String operation, VelnRuntime.Fn provider) {
        VelnRuntime.pushHandler(
            effect,
            new Object[] { operation },
            new Object[] { provider },
            new Object[0]
        );
    }

    private static void pushScope() {
        VelnRuntime.pushHandler(
            "std::io_provider::Scope",
            new Object[0],
            new Object[0],
            new Object[0]
        );
    }

    private static VelnRuntime.Result result(Object value) {
        return (VelnRuntime.Result) value;
    }

    private static boolean isInvalidResource(Object value) {
        VelnRuntime.Result result = result(value);
        return !result.isOk() && result.toString().contains("InvalidResource");
    }

    private static boolean hasInvalidResource(Object value) {
        return value.toString().contains("InvalidResource");
    }

    private static boolean slotEquals(Object value, long expected) {
        VelnRuntime.Result result = result(value);
        return result.isOk() && result.value().equals(Long.valueOf(expected));
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static Object family() {
        return VelnRuntime.adt("io_provider::ViewFamily::ReadFile", new Object[0]);
    }

    private static Object closed() {
        return VelnRuntime.adt("io::CloseOutcome::Closed", new Object[0]);
    }

    private static Object closeFailed(String message) {
        Object error = VelnRuntime.adt(
            "io::Error::Error",
            new Object[] {
                "close",
                VelnRuntime.adt("io::ErrorKind::Other", new Object[0]),
                VelnRuntime.none(),
                VelnRuntime.none(),
                VelnRuntime.none(),
                VelnRuntime.none(),
                message
            }
        );
        return VelnRuntime.adt(
            "io::CloseOutcome::CloseFailed",
            new Object[] { VelnRuntime.adt("io::Commit::Unknown", new Object[0]), error }
        );
    }

    private static int resourceCount(Object owner) throws Exception {
        java.lang.reflect.Field resources = owner.getClass().getDeclaredField("resources");
        resources.setAccessible(true);
        return ((java.util.Map<?, ?>) resources.get(owner)).size();
    }

    private static java.util.List<?> releaseFailures(Object scope) throws Exception {
        java.lang.reflect.Field failures = scope.getClass().getDeclaredField("ioProviderReleaseFailures");
        failures.setAccessible(true);
        return (java.util.List<?>) failures.get(scope);
    }

    private static Object makeForeignOwner() throws Exception {
        pushScope();
        Object foreign = owner();
        VelnRuntime.popHandler();
        return foreign;
    }

    private static void pushReadFileHandlers(
        VelnRuntime.Fn read,
        VelnRuntime.Fn close
    ) {
        push("std::io::Read", "read", read);
        push("std::io::Seek", "seek", (Object... supplied) -> VelnRuntime.ok(Long.valueOf(0L)));
        push("std::io::ReadAt", "read_at", (Object... supplied) ->
            VelnRuntime.adt("io::ReadOutcome::End", new Object[0]));
        push("std::io::Close", "close", close);
    }

    private static void popReadFileHandlers() {
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
    }

    private static void registeredRelease(Object foreignOwner, String exitKind) throws Exception {
        java.util.concurrent.atomic.AtomicInteger readCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger closeCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger unrelatedState = new java.util.concurrent.atomic.AtomicInteger(7);
        java.util.concurrent.atomic.AtomicReference<Object> ownerRef = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> resourceRef = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> readerRef = new java.util.concurrent.atomic.AtomicReference<Object>();

        pushScope();
        Object scope = handlers().get(handlers().size() - 1);
        Object releaseOwner = owner();
        ownerRef.set(releaseOwner);
        pushReadFileHandlers(
            (Object... supplied) -> {
                readCalls.incrementAndGet();
                return VelnRuntime.adt("io::ReadOutcome::End", new Object[0]);
            },
            (Object... supplied) -> {
                closeCalls.incrementAndGet();
                Object closer = supplied[0];
                require(
                    hasInvalidResource(VelnRuntime.ioProviderDispatch2(
                        "std::io::Read", "read", readerRef.get(), Long.valueOf(1L)
                    )),
                    exitKind + " cleanup allowed ordinary resource use"
                );
                require(readCalls.get() == 0, exitKind + " cleanup invoked the read provider");
                require(
                    slotEquals(VelnRuntime.ioProviderSlot(ownerRef.get(), resourceRef.get()), 51L),
                    exitKind + " cleanup could not inspect its matching resource slot"
                );
                require(
                    slotEquals(VelnRuntime.ioProviderCloserSlot(ownerRef.get(), closer), 51L),
                    exitKind + " cleanup could not inspect its matching capability slot"
                );
                require(
                    isInvalidResource(VelnRuntime.ioProviderCloserSlot(foreignOwner, closer)),
                    exitKind + " cleanup granted a foreign owner access"
                );
                require(
                    hasInvalidResource(VelnRuntime.ioProviderDispatch1(
                        "std::io::Close", "close", closer
                    )),
                    exitKind + " cleanup allowed recursive close dispatch"
                );
                require(closeCalls.get() == 1, exitKind + " cleanup re-entered the close handler");
                require(unrelatedState.get() == 7, exitKind + " cleanup changed unrelated state");
                return closed();
            }
        );
        VelnRuntime.Result registered = result(
            VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(51L), family())
        );
        require(registered.isOk(), exitKind + " resource registration failed");
        resourceRef.set(registered.value());
        VelnRuntime.Result projected = result(VelnRuntime.ioProviderReadFile(registered.value()));
        require(projected.isOk(), exitKind + " resource projection failed");
        readerRef.set(VelnRuntime.adtPayload(projected.value(), 0));
        popReadFileHandlers();

        VelnRuntime.popHandler();

        require(closeCalls.get() == 1, exitKind + " exit did not release exactly once");
        require(
            releaseFailures(scope).isEmpty(),
            exitKind + " successful release retained a failure: " + releaseFailures(scope)
        );
        require(
            isInvalidResource(VelnRuntime.ioProviderSlot(releaseOwner, registered.value())),
            exitKind + " exit left ordinary slot access available"
        );
    }

    private static void latePublication(
        Object foreignOwner,
        boolean failRelease,
        boolean duplicateSlot,
        boolean throwRelease,
        boolean occupiedNamespaceSlot
    ) throws Exception {
        java.util.concurrent.CountDownLatch openStarted = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch finishOpen = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicInteger closeCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicReference<Object> ownerRef = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> publication = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> staleResource = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> staleCloser = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> childFailure = new java.util.concurrent.atomic.AtomicReference<Throwable>();
        VelnRuntime.RuntimeFailure thrownRelease =
            new VelnRuntime.RuntimeFailure("planned late release runtime failure");
        int expectedCloseCalls = duplicateSlot ? 2 : 1;

        pushScope();
        Object scope = handlers().get(handlers().size() - 1);
        Object releaseOwner = owner();
        ownerRef.set(releaseOwner);
        pushReadFileHandlers(
            (Object... supplied) -> VelnRuntime.adt("io::ReadOutcome::End", new Object[0]),
            (Object... supplied) -> {
                int closeCall = closeCalls.incrementAndGet();
                Object closer = supplied[0];
                require(
                    slotEquals(VelnRuntime.ioProviderCloserSlot(ownerRef.get(), closer), 61L),
                    "late release could not inspect its matching slot"
                );
                require(
                    isInvalidResource(VelnRuntime.ioProviderCloserSlot(foreignOwner, closer)),
                    "late release granted a foreign owner access"
                );
                if (duplicateSlot && closeCall == expectedCloseCalls) {
                    require(
                        isInvalidResource(VelnRuntime.ioProviderSlot(ownerRef.get(), staleResource.get())),
                        "late release granted access to a stale same-slot resource"
                    );
                    require(
                        isInvalidResource(VelnRuntime.ioProviderCloserSlot(ownerRef.get(), staleCloser.get())),
                        "late release granted access to a stale same-slot capability"
                    );
                }
                if (closeCall != expectedCloseCalls) return closed();
                if (throwRelease) throw thrownRelease;
                return failRelease ? closeFailed("planned late release failure") : closed();
            }
        );
        push("std::fs::Read", "open_read", (Object... supplied) -> {
            openStarted.countDown();
            try {
                finishOpen.await();
            } catch (InterruptedException interrupted) {
                Thread.currentThread().interrupt();
                throw new VelnRuntime.RuntimeFailure("gated open interrupted");
            }
            return VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(61L), family());
        });
        if (duplicateSlot) {
            VelnRuntime.Result occupied = result(
                VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(61L), family())
            );
            require(occupied.isOk(), "duplicate-slot setup registration failed");
            staleResource.set(occupied.value());
            VelnRuntime.Result projected = result(VelnRuntime.ioProviderReadFile(occupied.value()));
            require(projected.isOk(), "duplicate-slot setup projection failed");
            staleCloser.set(VelnRuntime.adtPayload(projected.value(), 3));
        } else if (occupiedNamespaceSlot) {
            require(
                result(VelnRuntime.ioProviderFilesystem(releaseOwner, Long.valueOf(61L))).isOk(),
                "occupied namespace-slot setup failed"
            );
        }
        VelnRuntime.Result filesystem = result(
            VelnRuntime.ioProviderFilesystem(releaseOwner, Long.valueOf(60L))
        );
        require(filesystem.isOk(), "late-publication filesystem registration failed");
        java.util.List<Object> inherited = new java.util.ArrayList<Object>(handlers());
        Thread child = new Thread(() -> {
            java.util.List<Object> childHandlers = null;
            try {
                childHandlers = handlers();
                childHandlers.addAll(inherited);
                publication.set(VelnRuntime.ioProviderDispatch2(
                    "std::fs::Read", "open_read", filesystem.value(), "gated"
                ));
            } catch (Throwable failure) {
                childFailure.set(failure);
            } finally {
                if (childHandlers != null) childHandlers.clear();
            }
        });
        child.start();
        openStarted.await();

        VelnRuntime.popHandler();
        popReadFileHandlers();
        VelnRuntime.popHandler();
        finishOpen.countDown();
        child.join();

        require(childFailure.get() == null, "late publication child failed: " + childFailure.get());
        require(isInvalidResource(publication.get()), "late host resource was published");
        require(
            closeCalls.get() == expectedCloseCalls,
            "late host resource was not released exactly once"
        );
        require(
            resourceCount(releaseOwner) == (occupiedNamespaceSlot ? 2 : 1),
            "late resource changed the owner registry"
        );
        require(
            releaseFailures(scope).size() == (failRelease || throwRelease ? 1 : 0),
            "late release failure retention did not match the scripted outcome"
        );
        if (throwRelease) {
            require(
                releaseFailures(scope).get(0) == thrownRelease,
                "late release did not retain the runtime failure identity"
            );
        }
    }

    private static void closedDispatchPaths() throws Exception {
        java.util.concurrent.atomic.AtomicInteger readCalls = new java.util.concurrent.atomic.AtomicInteger();
        push("std::io::Read", "read", (Object... supplied) -> {
            readCalls.incrementAndGet();
            return VelnRuntime.adt("io::ReadOutcome::End", new Object[0]);
        });
        pushScope();
        Object releaseOwner = owner();
        push("std::io::Seek", "seek", (Object... supplied) -> VelnRuntime.ok(Long.valueOf(0L)));
        push("std::io::ReadAt", "read_at", (Object... supplied) ->
            VelnRuntime.adt("io::ReadOutcome::End", new Object[0]));
        push("std::io::Close", "close", (Object... supplied) -> closed());
        VelnRuntime.Result registered = result(
            VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(71L), family())
        );
        require(registered.isOk(), "closed-dispatch resource registration failed");
        VelnRuntime.Result projected = result(VelnRuntime.ioProviderReadFile(registered.value()));
        require(projected.isOk(), "closed-dispatch resource projection failed");
        Object reader = VelnRuntime.adtPayload(projected.value(), 0);
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        require(
            hasInvalidResource(VelnRuntime.ioProviderDispatch2(
                "std::io::Read", "read", reader, Long.valueOf(1L)
            )),
            "closed owner facade dispatch reached its provider"
        );
        require(
            hasInvalidResource(VelnRuntime.perform(
                "std::io::Read", "read", new Object[] { reader, Long.valueOf(1L) }
            )),
            "closed owner lexical dispatch reached its provider"
        );
        require(readCalls.get() == 0, "closed owner dispatch invoked the selected read provider");
        VelnRuntime.popHandler();
    }

    private static void lateReleaseDoesNotHoldRegistryMonitor() throws Exception {
        java.util.concurrent.CountDownLatch openStarted = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch finishOpen = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicReference<Object> publication = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> childFailure = new java.util.concurrent.atomic.AtomicReference<Throwable>();

        pushScope();
        Object releaseOwner = owner();
        pushReadFileHandlers(
            (Object... supplied) -> VelnRuntime.adt("io::ReadOutcome::End", new Object[0]),
            (Object... supplied) -> {
                java.util.concurrent.atomic.AtomicReference<Object> rejected =
                    new java.util.concurrent.atomic.AtomicReference<Object>();
                Thread child = new Thread(() -> rejected.set(VelnRuntime.ioProviderResource(
                    releaseOwner, Long.valueOf(82L), family()
                )));
                child.start();
                try {
                    child.join(2000L);
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    throw new VelnRuntime.RuntimeFailure("late release join interrupted");
                }
                require(!child.isAlive(), "late release held the owner registry monitor during callback");
                require(isInvalidResource(rejected.get()), "late release child registration was not rejected");
                return closed();
            }
        );
        push("std::fs::Read", "open_read", (Object... supplied) -> {
            openStarted.countDown();
            try {
                finishOpen.await();
            } catch (InterruptedException interrupted) {
                Thread.currentThread().interrupt();
                throw new VelnRuntime.RuntimeFailure("gated open interrupted");
            }
            return VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(81L), family());
        });
        VelnRuntime.Result filesystem = result(
            VelnRuntime.ioProviderFilesystem(releaseOwner, Long.valueOf(80L))
        );
        require(filesystem.isOk(), "monitor-boundary filesystem registration failed");
        java.util.List<Object> inherited = new java.util.ArrayList<Object>(handlers());
        Thread publicationThread = new Thread(() -> {
            java.util.List<Object> childHandlers = null;
            try {
                childHandlers = handlers();
                childHandlers.addAll(inherited);
                publication.set(VelnRuntime.ioProviderDispatch2(
                    "std::fs::Read", "open_read", filesystem.value(), "gated"
                ));
            } catch (Throwable failure) {
                childFailure.set(failure);
            } finally {
                if (childHandlers != null) childHandlers.clear();
            }
        });
        publicationThread.start();
        openStarted.await();
        VelnRuntime.popHandler();
        popReadFileHandlers();
        VelnRuntime.popHandler();
        finishOpen.countDown();
        publicationThread.join();
        require(childFailure.get() == null, "monitor-boundary publication failed: " + childFailure.get());
        require(isInvalidResource(publication.get()), "monitor-boundary late publication succeeded");
    }

    public static void main(String[] args) throws Exception {
        Object foreignOwner = makeForeignOwner();
        registeredRelease(foreignOwner, "normal");
        latePublication(foreignOwner, false, false, false, false);
        latePublication(foreignOwner, true, false, false, false);
        latePublication(foreignOwner, false, false, true, false);
        latePublication(foreignOwner, false, true, false, false);
        latePublication(foreignOwner, false, false, false, true);
        closedDispatchPaths();
        lateReleaseDoesNotHoldRegistryMonitor();
    }
}
