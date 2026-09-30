public final class ClockExceptionHarness {
    public static void main(String[] args) {
        for (int attempt = 0; attempt < 2; attempt += 1) {
            try {
                VelnProgram.fn_main();
                throw new AssertionError("Veln clock handler should throw");
            } catch (VelnRuntime.RuntimeFailure expected) {
                if (!expected.getMessage().equals("injected clock failure")) {
                    throw new AssertionError("unexpected clock failure", expected);
                }
            }
            try {
                VelnRuntime.perform("std::host_effects::Clock", "request", new Object[] {"now", 0L});
                throw new AssertionError("clock handler escaped its exceptional scope");
            } catch (IllegalStateException expected) {
                if (!expected.getMessage().contains("unhandled effect")) {
                    throw new AssertionError("handler remained active", expected);
                }
            }
            if (!(VelnRuntime.timeMonotonicMs() instanceof Long)) {
                throw new AssertionError("host clock did not resume");
            }
        }
        System.out.println("exceptional clock scopes restored");
    }
}
