public final class IoProviderAppendFileHarness {
    public static void main(String[] args) throws Exception {
        IoProviderHarnessSupport.push("std::io_provider::Scope");
        Object owner = IoProviderHarnessSupport.owner();
        Object family = VelnRuntime.adt("ViewFamily::AppendFile", new Object[0]);

        IoProviderHarnessSupport.push("std::io::Write");
        Object write = IoProviderHarnessSupport.handlers().get(1);
        IoProviderHarnessSupport.pushClose();
        Object close = IoProviderHarnessSupport.handlers().get(2);
        IoProviderHarnessSupport.push("std::io::Sync");
        Object sync = IoProviderHarnessSupport.handlers().get(3);
        VelnRuntime.Result accepted = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(17L),
            family
        );
        IoProviderHarnessSupport.require(
            accepted.isOk(),
            "AppendFile registration failed before handler identity check"
        );
        Object resource = accepted.value();

        VelnRuntime.popHandler();
        VelnRuntime.popHandler();
        VelnRuntime.popHandler();

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderAppendFile(resource);
        IoProviderHarnessSupport.require(
            projected.isOk(),
            "AppendFile projection failed before handler identity check"
        );
        Object[] payloads = IoProviderHarnessSupport.payloads(projected.value());
        IoProviderHarnessSupport.require(
            payloads.length == 3,
            "AppendFile projection returned the wrong capability count"
        );
        IoProviderHarnessSupport.requireCapability(payloads[0], write, "writer");
        IoProviderHarnessSupport.requireCapability(payloads[1], close, "closer");
        IoProviderHarnessSupport.requireCapability(payloads[2], sync, "syncer");

        VelnRuntime.popHandler();
    }
}
