public final class IoProviderStateHarness {
    private static final VelnRuntime.Fn COMPILED_TRANSITION = args ->
        VelnProgram.fn_state_transition(args[0], args[1]);
    private static final VelnRuntime.Fn READ = args ->
        VelnRuntime.record("state", args[0], "result", args[0]);

    private static long read(Object cell) {
        return ((Number) VelnRuntime.ioProviderUpdateState(cell, VelnRuntime.UNIT, READ)).longValue();
    }

    private static long readFromOtherThread(Object cell) {
        java.util.concurrent.atomic.AtomicLong observed = new java.util.concurrent.atomic.AtomicLong();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread reader = new Thread(() -> {
            try {
                observed.set(read(cell));
            } catch (Throwable error) {
                failure.set(error);
            }
        }, "provider-state-unlock-reader");
        reader.start();
        try {
            reader.join();
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
            throw new AssertionError("provider state unlock reader was interrupted", error);
        }
        if (failure.get() != null) {
            throw new AssertionError("provider state remained unusable after failure", failure.get());
        }
        return observed.get();
    }

    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void awaitUninterruptibly(java.util.concurrent.CountDownLatch latch) {
        boolean interrupted = false;
        while (true) {
            try {
                latch.await();
                break;
            } catch (InterruptedException expected) {
                interrupted = true;
            }
        }
        if (interrupted) Thread.currentThread().interrupt();
    }

    private static void independentIdentityAndTaskFreeze() {
        Object first = VelnRuntime.ioProviderState(Long.valueOf(1L));
        Object second = VelnRuntime.ioProviderState(Long.valueOf(1L));
        VelnRuntime.ioProviderUpdateState(first, Long.valueOf(2L), COMPILED_TRANSITION);
        require(read(first) == 3L && read(second) == 1L, "state cells shared independent identity");

        Object child = VelnRuntime.taskSpawnWith((VelnRuntime.Fn) args ->
            VelnRuntime.ioProviderUpdateState(args[0], Long.valueOf(4L), COMPILED_TRANSITION), first);
        VelnRuntime.Result joined = (VelnRuntime.Result) VelnRuntime.taskJoin(child);
        require(joined.isOk(), "state child failed after task-boundary freezing");
        require(read(first) == 7L, "task freezing copied provider state identity");
    }

    private static void exclusiveUpdatesDoNotLoseWrites() throws Exception {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(0L));
        int threadCount = 12;
        int updatesPerThread = 80;
        java.util.concurrent.atomic.AtomicInteger active = new java.util.concurrent.atomic.AtomicInteger();
        java.util.concurrent.atomic.AtomicReference<Throwable> failure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        VelnRuntime.Fn increment = args -> {
            if (active.incrementAndGet() != 1) {
                throw new AssertionError("provider state callbacks overlapped");
            }
            try {
                long value = ((Number) args[0]).longValue();
                Thread.yield();
                return VelnRuntime.record(
                    "state", Long.valueOf(value + 1L),
                    "result", Long.valueOf(value)
                );
            } finally {
                active.decrementAndGet();
            }
        };
        Thread[] workers = new Thread[threadCount];
        for (int index = 0; index < workers.length; index += 1) {
            workers[index] = new Thread(() -> {
                try {
                    for (int update = 0; update < updatesPerThread; update += 1) {
                        VelnRuntime.ioProviderUpdateState(cell, VelnRuntime.UNIT, increment);
                    }
                } catch (Throwable error) {
                    failure.compareAndSet(null, error);
                }
            }, "provider-state-worker-" + index);
            workers[index].start();
        }
        for (Thread worker : workers) worker.join();
        if (failure.get() != null) throw new AssertionError("concurrent update failed", failure.get());
        require(read(cell) == (long) threadCount * updatesPerThread, "provider state lost a concurrent write");
    }

    private static void failureAndReentrancyRollbackAndUnlock() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(9L));
        try {
            VelnRuntime.ioProviderUpdateState(cell, VelnRuntime.UNIT, (VelnRuntime.Fn) args -> {
                throw new VelnRuntime.RuntimeFailure("transition failed");
            });
            throw new AssertionError("runtime callback failure was accepted");
        } catch (VelnRuntime.RuntimeFailure expected) {
            require("transition failed".equals(expected.getMessage()), "callback failure identity changed");
        }
        require(
            readFromOtherThread(cell) == 9L,
            "runtime callback failure changed state or stranded the lock"
        );

        try {
            VelnRuntime.ioProviderUpdateState(cell, VelnRuntime.UNIT, (VelnRuntime.Fn) args -> {
                VelnRuntime.ioProviderUpdateState(cell, VelnRuntime.UNIT, READ);
                return VelnRuntime.record("state", Long.valueOf(99L), "result", VelnRuntime.UNIT);
            });
            throw new AssertionError("reentrant update was accepted");
        } catch (VelnRuntime.RuntimeFailure expected) {
            require(
                "reentrant io provider state update".equals(expected.getMessage()),
                "reentrant update reported the wrong failure"
            );
        }
        require(
            readFromOtherThread(cell) == 9L,
            "reentrant update changed state or stranded the lock"
        );
    }

    private static void cancellationBeforeAdmissionDoesNotEnterCallback() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(20L));
        java.util.concurrent.CountDownLatch boundary = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicInteger calls = new java.util.concurrent.atomic.AtomicInteger();
        VelnRuntime.ioProviderStateInstallBoundaryGate(cell, new VelnRuntime.IoProviderStateBoundaryGate() {
            public void beforeAdmission() {
                boundary.countDown();
                awaitUninterruptibly(release);
            }
            public void afterAdmission() {}
            public void afterCommit() {}
        });
        Object task = VelnRuntime.taskSpawn((VelnRuntime.Fn) args -> VelnRuntime.ioProviderUpdateState(
            cell,
            Long.valueOf(1L),
            (VelnRuntime.Fn) transitionArgs -> {
                calls.incrementAndGet();
                return VelnRuntime.record("state", Long.valueOf(21L), "result", VelnRuntime.UNIT);
            }
        ));
        awaitUninterruptibly(boundary);
        VelnRuntime.taskCancel(task);
        release.countDown();
        VelnRuntime.Result joined = (VelnRuntime.Result) VelnRuntime.taskJoin(task);
        require(
            !joined.isOk()
                && ((Boolean) VelnRuntime.taskJoinErrorIsCancelled(joined.value())),
            "pre-admission task did not report cancellation"
        );
        require(calls.get() == 0, "pre-admission cancellation entered the callback");
        VelnRuntime.ioProviderStateInstallBoundaryGate(cell, null);
        require(read(cell) == 20L, "pre-admission cancellation changed state or stranded the lock");
    }

    private static void cancellationAfterAdmissionCommitsBeforeDelivery() {
        Object cell = VelnRuntime.ioProviderState(Long.valueOf(30L));
        java.util.concurrent.CountDownLatch admitted = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch committed = new java.util.concurrent.CountDownLatch(1);
        VelnRuntime.ioProviderStateInstallBoundaryGate(cell, new VelnRuntime.IoProviderStateBoundaryGate() {
            public void beforeAdmission() {}
            public void afterAdmission() {
                admitted.countDown();
                awaitUninterruptibly(release);
            }
            public void afterCommit() {
                committed.countDown();
            }
        });
        java.util.concurrent.atomic.AtomicBoolean returned =
            new java.util.concurrent.atomic.AtomicBoolean(false);
        Object task = VelnRuntime.taskSpawn((VelnRuntime.Fn) args -> {
            Object result = VelnRuntime.ioProviderUpdateState(
                cell,
                Long.valueOf(5L),
                COMPILED_TRANSITION
            );
            returned.set(true);
            return result;
        });
        awaitUninterruptibly(admitted);
        VelnRuntime.taskCancel(task);
        release.countDown();
        awaitUninterruptibly(committed);
        VelnRuntime.Result joined = (VelnRuntime.Result) VelnRuntime.taskJoin(task);
        require(
            !joined.isOk()
                && ((Boolean) VelnRuntime.taskJoinErrorIsCancelled(joined.value())),
            "post-admission task did not report cancellation after commit"
        );
        require(!returned.get(), "post-admission cancellation returned from provider state update");
        VelnRuntime.ioProviderStateInstallBoundaryGate(cell, null);
        require(read(cell) == 35L, "post-admission cancellation prevented or split the commit");
    }

    public static void main(String[] args) throws Exception {
        independentIdentityAndTaskFreeze();
        exclusiveUpdatesDoNotLoseWrites();
        failureAndReentrancyRollbackAndUnlock();
        cancellationBeforeAdmissionDoesNotEnterCallback();
        cancellationAfterAdmissionCommitsBeforeDelivery();
        System.out.println("atomic provider state invariants held");
    }
}
