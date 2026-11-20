public final class VirtualTaskProgressHarness {
    private static Object spawn(VelnRuntime.Fn action) {
        return VelnRuntime.taskSpawn(action);
    }

    private static Object field(Object record, String name) {
        return VelnRuntime.recordField(record, name);
    }

    private static long joinedLong(Object task, String context) {
        Object joined = VelnRuntime.taskJoin(task);
        if (!VelnRuntime.isOk(joined)) {
            throw new AssertionError(context + " failed: " + joined);
        }
        return ((Long) VelnRuntime.resultValue(joined)).longValue();
    }

    public static void main(String[] args) {
        VelnRuntime.beginTaskLifetime();
        Object parent = spawn((VelnRuntime.Fn) ignored -> {
            Object rendezvous = VelnRuntime.channelBounded(Long.valueOf(0L));
            Object sender = spawn((VelnRuntime.Fn) childIgnored -> {
                VelnRuntime.timeTimeoutMs(Long.valueOf(5L));
                VelnRuntime.channelSend(field(rendezvous, "tx"), Long.valueOf(11L));
                return Long.valueOf(13L);
            });
            Object received = VelnRuntime.channelRecv(field(rendezvous, "rx"));
            long rendezvousValue = ((Long) VelnRuntime.optionValue(received)).longValue();
            long senderValue = joinedLong(sender, "timer/rendezvous child");

            Object bounded = VelnRuntime.channelBounded(Long.valueOf(1L));
            VelnRuntime.channelSend(field(bounded, "tx"), Long.valueOf(17L));
            Object drainer = spawn((VelnRuntime.Fn) childIgnored -> {
                long first = ((Long) VelnRuntime.optionValue(
                    VelnRuntime.channelRecv(field(bounded, "rx")))).longValue();
                long second = ((Long) VelnRuntime.optionValue(
                    VelnRuntime.channelRecv(field(bounded, "rx")))).longValue();
                return Long.valueOf(first + second);
            });
            VelnRuntime.channelSend(field(bounded, "tx"), Long.valueOf(19L));
            long drained = joinedLong(drainer, "full-buffer drainer");

            Object nested = spawn((VelnRuntime.Fn) childIgnored -> {
                Object grandchild = spawn((VelnRuntime.Fn) grandchildIgnored -> {
                    VelnRuntime.timeTimeoutMs(Long.valueOf(5L));
                    return Long.valueOf(23L);
                });
                return Long.valueOf(joinedLong(grandchild, "nested grandchild") + 29L);
            });
            long nestedValue = joinedLong(nested, "nested join child");
            return Long.valueOf(
                rendezvousValue + senderValue + drained + nestedValue
            );
        });
        VelnRuntime.finishTaskLifetime(true);
        long result = joinedLong(parent, "parent");
        if (result != 112L) {
            throw new AssertionError("candidate progress result changed: " + result);
        }
        System.out.println("production virtual progress complete");
    }
}
