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

    private static boolean isClosedOwnerResourceError(Object value) {
        VelnRuntime.Result result = result(value);
        if (result.isOk()) return false;
        Object error = result.value();
        return VelnRuntime.isAdt(error, "io::Error::Error")
            && VelnRuntime.adtPayload(error, 0).equals("resource")
            && VelnRuntime.isAdt(
                VelnRuntime.adtPayload(error, 1),
                "io::ErrorKind::InvalidResource"
            )
            && VelnRuntime.adtPayload(error, 6).equals("provider owner scope is closed");
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
        boolean occupiedNamespaceSlot,
        boolean lexicalOpen
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
        Object failedOutcome = closeFailed("planned late release failure");
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
                return failRelease ? failedOutcome : closed();
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
                publication.set(lexicalOpen
                    ? VelnRuntime.perform(
                        "std::fs::Read", "open_read", new Object[] { filesystem.value(), "gated" }
                    )
                    : VelnRuntime.ioProviderDispatch2(
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
        } else if (failRelease) {
            require(
                releaseFailures(scope).get(0) == failedOutcome,
                "late release did not retain the failed close outcome"
            );
        }
    }

    private static void crossOwnerLatePublicationDuringRelease() throws Exception {
        java.util.concurrent.CountDownLatch openStarted = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch finishOpen = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicInteger ownerACloseCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger ownerBCloseCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger unrelatedState = new java.util.concurrent.atomic.AtomicInteger(19);
        java.util.concurrent.atomic.AtomicBoolean ownerBBoundaryObserved = new java.util.concurrent.atomic.AtomicBoolean();
        java.util.concurrent.atomic.AtomicReference<Object> ownerBCloser = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> publication = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> childFailure = new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Object ownerBFailure = closeFailed("planned cross-owner late release failure");

        pushScope();
        Object ownerAScope = handlers().get(handlers().size() - 1);
        Object ownerA = owner();
        java.util.concurrent.atomic.AtomicReference<Thread> publicationThread =
            new java.util.concurrent.atomic.AtomicReference<Thread>();
        java.util.concurrent.atomic.AtomicReference<Object> ownerBRef =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Object> ownerBScopeRef =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        pushReadFileHandlers(
            (Object... supplied) -> VelnRuntime.adt("io::ReadOutcome::End", new Object[0]),
            (Object... supplied) -> {
                ownerACloseCalls.incrementAndGet();
                finishOpen.countDown();
                try {
                    publicationThread.get().join();
                    require(childFailure.get() == null, "cross-owner publication failed: " + childFailure.get());
                    require(isInvalidResource(publication.get()), "cross-owner late resource was published");
                    require(ownerBCloseCalls.get() == 1, "cross-owner late resource was not released exactly once");
                    require(ownerBBoundaryObserved.get(), "owner B release boundary was not externally observed");
                    require(
                        isInvalidResource(VelnRuntime.ioProviderCloserSlot(ownerA, ownerBCloser.get())),
                        "owner A inspected owner B's releasing capability"
                    );
                    require(resourceCount(ownerBRef.get()) == 1, "owner B late release changed its registry");
                    require(
                        releaseFailures(ownerBScopeRef.get()).size() == 1
                            && releaseFailures(ownerBScopeRef.get()).get(0) == ownerBFailure,
                        "owner B did not retain its exact release failure"
                    );
                    require(unrelatedState.get() == 19, "cross-owner release changed unrelated state");
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    throw new VelnRuntime.RuntimeFailure("cross-owner publication join interrupted");
                } catch (Exception failure) {
                    throw new VelnRuntime.RuntimeFailure("cross-owner release assertion failed: " + failure);
                }
                return closed();
            }
        );
        VelnRuntime.Result ownerAResource = result(
            VelnRuntime.ioProviderResource(ownerA, Long.valueOf(91L), family())
        );
        require(ownerAResource.isOk(), "owner A resource registration failed");
        popReadFileHandlers();

        pushScope();
        Object ownerBScope = handlers().get(handlers().size() - 1);
        Object ownerB = owner();
        ownerBRef.set(ownerB);
        ownerBScopeRef.set(ownerBScope);
        pushReadFileHandlers(
            (Object... supplied) -> VelnRuntime.adt("io::ReadOutcome::End", new Object[0]),
            (Object... supplied) -> {
                ownerBCloseCalls.incrementAndGet();
                Object closer = supplied[0];
                ownerBCloser.set(closer);
                ownerBBoundaryObserved.set(
                    slotEquals(VelnRuntime.ioProviderCloserSlot(ownerB, closer), 93L)
                        && isInvalidResource(VelnRuntime.ioProviderCloserSlot(ownerA, closer))
                        && unrelatedState.get() == 19
                );
                return ownerBFailure;
            }
        );
        push("std::fs::Read", "open_read", (Object... supplied) -> {
            openStarted.countDown();
            try {
                finishOpen.await();
            } catch (InterruptedException interrupted) {
                Thread.currentThread().interrupt();
                throw new VelnRuntime.RuntimeFailure("cross-owner gated open interrupted");
            }
            return VelnRuntime.ioProviderResource(ownerB, Long.valueOf(93L), family());
        });
        VelnRuntime.Result filesystem = result(VelnRuntime.ioProviderFilesystem(ownerB, Long.valueOf(92L)));
        require(filesystem.isOk(), "owner B filesystem registration failed");
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
        publicationThread.set(child);
        child.start();
        openStarted.await();

        VelnRuntime.popHandler();
        popReadFileHandlers();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        require(ownerACloseCalls.get() == 1, "owner A did not release exactly once");
        require(resourceCount(ownerA) == 0, "owner A registry was not cleared");
        require(releaseFailures(ownerAScope).isEmpty(), "owner A retained an unexpected release failure");
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

    private static void capturedByteOperationCannotPublishAfterClosure(
        Object foreignOwner,
        boolean nestedOpen,
        boolean lexicalByte
    ) throws Exception {
        java.util.concurrent.CountDownLatch readStarted = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch finishRead = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicInteger readCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger registeredCloseCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger rejectedCloseCalls = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicInteger unrelatedState = new java.util.concurrent.atomic.AtomicInteger(29);
        java.util.concurrent.atomic.AtomicReference<Object> publication = new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> childFailure = new java.util.concurrent.atomic.AtomicReference<Throwable>();
        int foreignResources = resourceCount(foreignOwner);

        pushScope();
        Object scope = handlers().get(handlers().size() - 1);
        Object releaseOwner = owner();
        pushReadFileHandlers(
            (Object... supplied) -> {
                readCalls.incrementAndGet();
                readStarted.countDown();
                try {
                    finishRead.await();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    throw new VelnRuntime.RuntimeFailure("gated byte read interrupted");
                }
                push("std::io::Close", "close", (Object... closer) -> {
                    rejectedCloseCalls.incrementAndGet();
                    return closed();
                });
                try {
                    return VelnRuntime.ioProviderResource(
                        releaseOwner,
                        Long.valueOf(73L),
                        family()
                    );
                } finally {
                    VelnRuntime.popHandler();
                }
            },
            (Object... supplied) -> {
                registeredCloseCalls.incrementAndGet();
                return closed();
            }
        );
        java.util.concurrent.atomic.AtomicInteger openCalls = new java.util.concurrent.atomic.AtomicInteger();
        if (nestedOpen) {
            push("std::fs::Read", "open_read", (Object... supplied) -> {
                openCalls.incrementAndGet();
                return lexicalByte
                    ? VelnRuntime.perform(
                        "std::io::Read", "read", new Object[] { supplied[1], Long.valueOf(1L) }
                    )
                    : VelnRuntime.ioProviderDispatch2(
                        "std::io::Read", "read", supplied[1], Long.valueOf(1L)
                    );
            });
        }
        VelnRuntime.Result registered = result(
            VelnRuntime.ioProviderResource(releaseOwner, Long.valueOf(72L), family())
        );
        require(registered.isOk(), "gated byte-read resource registration failed");
        VelnRuntime.Result projected = result(VelnRuntime.ioProviderReadFile(registered.value()));
        require(projected.isOk(), "gated byte-read resource projection failed");
        Object reader = VelnRuntime.adtPayload(projected.value(), 0);
        VelnRuntime.Result filesystem = nestedOpen
            ? result(VelnRuntime.ioProviderFilesystem(releaseOwner, Long.valueOf(74L)))
            : null;
        if (nestedOpen) require(filesystem.isOk(), "nested-open filesystem registration failed");
        java.util.List<Object> inherited = new java.util.ArrayList<Object>(handlers());
        Thread child = new Thread(() -> {
            java.util.List<Object> childHandlers = null;
            try {
                childHandlers = handlers();
                childHandlers.addAll(inherited);
                publication.set(nestedOpen
                    ? VelnRuntime.ioProviderDispatch2(
                        "std::fs::Read", "open_read", filesystem.value(), reader
                    )
                    : VelnRuntime.ioProviderDispatch2(
                        "std::io::Read", "read", reader, Long.valueOf(1L)
                    ));
            } catch (Throwable failure) {
                childFailure.set(failure);
            } finally {
                if (childHandlers != null) childHandlers.clear();
            }
        });
        child.start();
        readStarted.await();

        if (nestedOpen) VelnRuntime.popHandler();
        popReadFileHandlers();
        VelnRuntime.popHandler();
        finishRead.countDown();
        child.join();

        require(childFailure.get() == null, "gated byte read failed: " + childFailure.get());
        require(
            isClosedOwnerResourceError(publication.get()),
            "gated byte read did not return the exact closed-owner resource error"
        );
        require(readCalls.get() == 1, "gated byte read did not invoke its captured provider once");
        require(openCalls.get() == (nestedOpen ? 1 : 0), "gated byte read used an unexpected open path");
        require(registeredCloseCalls.get() == 1, "owner closure did not release the registered resource once");
        require(rejectedCloseCalls.get() == 0, "rejected byte-read publication invoked its close handler");
        require(
            resourceCount(releaseOwner) == (nestedOpen ? 1 : 0),
            "rejected byte-read publication changed the owner registry"
        );
        require(releaseFailures(scope).isEmpty(), "rejected byte-read publication retained a release failure");
        require(unrelatedState.get() == 29, "rejected byte-read publication changed unrelated provider state");
        require(
            resourceCount(foreignOwner) == foreignResources,
            "rejected byte-read publication changed a foreign owner registry"
        );
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
        latePublication(foreignOwner, false, false, false, false, false);
        latePublication(foreignOwner, true, false, false, false, false);
        latePublication(foreignOwner, false, false, true, false, false);
        latePublication(foreignOwner, false, true, false, false, false);
        latePublication(foreignOwner, false, false, false, true, false);
        latePublication(foreignOwner, false, false, false, false, true);
        latePublication(foreignOwner, true, false, false, false, true);
        latePublication(foreignOwner, false, false, true, false, true);
        crossOwnerLatePublicationDuringRelease();
        closedDispatchPaths();
        capturedByteOperationCannotPublishAfterClosure(foreignOwner, false, false);
        capturedByteOperationCannotPublishAfterClosure(foreignOwner, true, false);
        capturedByteOperationCannotPublishAfterClosure(foreignOwner, true, true);
        lateReleaseDoesNotHoldRegistryMonitor();
    }
}
