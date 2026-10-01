public final class CancellationIdentityHarness {
    public static void main(String[] args) {
        Object task = VelnRuntime.taskSpawn(new VelnRuntime.Fn() {
            public Object call(Object... ignored) {
                throw new VelnRuntime.RuntimeFailure("task cancelled");
            }
        });

        try {
            VelnRuntime.taskJoin(task);
            throw new AssertionError("same-message runtime failure was classified as cancellation");
        } catch (VelnRuntime.RuntimeFailure expected) {
            if (!"task cancelled".equals(expected.getMessage())) {
                throw new AssertionError("runtime failure message changed", expected);
            }
        }
        System.out.println("runtime failure identity remained primary");
    }
}
