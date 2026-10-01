public final class CleanupDiagnosticGrowthHarness {
    private static long messageReads = 0;

    private static final class CountingFailure extends RuntimeException {
        private CountingFailure(String message) {
            super(message);
        }

        @Override
        public String getMessage() {
            messageReads += 1;
            return super.getMessage();
        }
    }

    private static Throwable failureAtDepth(int depth) {
        Throwable failure = new CountingFailure("cleanup 0");
        for (int index = 1; index < depth; index += 1) {
            Throwable primary = new CountingFailure("cleanup " + Integer.toString(index));
            VelnRuntime.attachCleanupFailure(primary, failure);
            failure = primary;
        }
        return failure;
    }

    private static long retainedFailureCount(Throwable failure) {
        long failures = 0;
        java.util.ArrayDeque<Throwable> pending = new java.util.ArrayDeque<Throwable>();
        java.util.Set<Throwable> seen = java.util.Collections.newSetFromMap(
            new java.util.IdentityHashMap<Throwable, Boolean>()
        );
        pending.push(failure);
        while (!pending.isEmpty()) {
            Throwable next = pending.pop();
            if (!seen.add(next)) {
                continue;
            }
            failures += 1;
            for (Throwable related : next.getSuppressed()) {
                pending.push(related);
            }
        }
        return failures;
    }

    private static long retainedStackFrames(Throwable failure) {
        long frames = 0;
        java.util.ArrayDeque<Throwable> pending = new java.util.ArrayDeque<Throwable>();
        java.util.Set<Throwable> seen = java.util.Collections.newSetFromMap(
            new java.util.IdentityHashMap<Throwable, Boolean>()
        );
        pending.push(failure);
        while (!pending.isEmpty()) {
            Throwable next = pending.pop();
            if (!seen.add(next)) {
                continue;
            }
            frames += next.getStackTrace().length;
            for (Throwable related : next.getSuppressed()) {
                pending.push(related);
            }
        }
        return frames;
    }

    public static void main(String[] args) {
        messageReads = 0;
        Throwable small = failureAtDepth(64);
        long smallAggregationReads = messageReads;
        messageReads = 0;
        Throwable large = failureAtDepth(192);
        long largeAggregationReads = messageReads;
        long smallFrames = retainedStackFrames(small);
        long largeFrames = retainedStackFrames(large);

        if (retainedFailureCount(small) != 64 || retainedFailureCount(large) != 192) {
            throw new AssertionError("retained cleanup failure count did not grow linearly");
        }
        if (smallAggregationReads > 128 || largeAggregationReads > 384) {
            throw new AssertionError(
                "nested cleanup aggregation traversed prior failures non-linearly: "
                    + smallAggregationReads
                    + " -> "
                    + largeAggregationReads
            );
        }
        if (largeFrames > smallFrames * 2) {
            throw new AssertionError(
                "retained stack frames grew non-linearly: "
                    + smallFrames
                    + " -> "
                    + largeFrames
            );
        }
        System.out.println("cleanup diagnostics aggregate nested failures linearly");
    }
}
