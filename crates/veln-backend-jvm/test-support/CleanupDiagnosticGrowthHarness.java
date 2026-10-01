public final class CleanupDiagnosticGrowthHarness {
    private static Throwable failureAtDepth(int depth) {
        if (depth == 0) {
            return new RuntimeException("primary");
        }
        Throwable primary = failureAtDepth(depth - 1);
        VelnRuntime.attachCleanupFailure(
            primary,
            new RuntimeException("cleanup " + Integer.toString(depth))
        );
        return primary;
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
        Throwable small = failureAtDepth(64);
        Throwable large = failureAtDepth(192);
        long smallFrames = retainedStackFrames(small);
        long largeFrames = retainedStackFrames(large);

        if (small.getSuppressed().length != 64 || large.getSuppressed().length != 192) {
            throw new AssertionError("cleanup diagnostic count did not grow linearly");
        }
        for (int index = 0; index < large.getSuppressed().length; index += 1) {
            Throwable related = large.getSuppressed()[index];
            String expected = "cleanup " + Integer.toString(index + 1);
            if (!expected.equals(related.getMessage())) {
                throw new AssertionError("cleanup diagnostic order changed at " + index);
            }
            if (related.getStackTrace().length != 0) {
                throw new AssertionError("cleanup diagnostic retained a stack trace");
            }
        }
        if (largeFrames > smallFrames * 4) {
            throw new AssertionError(
                "retained stack frames grew non-linearly: "
                    + smallFrames
                    + " -> "
                    + largeFrames
            );
        }
        System.out.println("cleanup diagnostics retained linear stack data");
    }
}
