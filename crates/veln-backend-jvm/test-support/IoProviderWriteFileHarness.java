public final class IoProviderWriteFileHarness extends IoProviderHarnessSupport {
    public static void main(String[] args) throws Exception {
        push("std::io_provider::Scope");
        Object owner = owner();
        Object family = VelnRuntime.adt("ViewFamily::WriteFile", new Object[0]);

        push("std::io::Write");
        Object write = handlers().get(1);
        push("std::io::Seek");
        Object seek = handlers().get(2);
        push("std::io::WriteAt");
        Object writeAt = handlers().get(3);
        pushClose();
        Object close = handlers().get(4);
        push("std::io::Sync");
        Object sync = handlers().get(5);
        push("std::fs::Mutate");
        Object mutate = handlers().get(6);

        VelnRuntime.Result accepted = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(23L),
            family
        );
        require(accepted.isOk(), "WriteFile registration failed before handler identity check");
        Object resource = accepted.value();

        VelnRuntime.ioProviderResource(owner, Long.valueOf(23L), family);
        requireRegisteredIdentity(owner, 23L, resource, "same-family duplicate WriteFile");

        push("std::io::Read");
        push("std::io::ReadAt");
        Object readFamily = VelnRuntime.adt("ViewFamily::ReadFile", new Object[0]);
        VelnRuntime.ioProviderResource(owner, Long.valueOf(23L), readFamily);
        requireRegisteredIdentity(owner, 23L, resource, "incoming ReadFile duplicate");

        VelnRuntime.Result acceptedRead = (VelnRuntime.Result) VelnRuntime.ioProviderResource(
            owner,
            Long.valueOf(29L),
            readFamily
        );
        require(acceptedRead.isOk(), "ReadFile registration failed before cross-family check");
        Object readResource = acceptedRead.value();
        VelnRuntime.ioProviderResource(owner, Long.valueOf(29L), family);
        requireRegisteredIdentity(owner, 29L, readResource, "incoming WriteFile duplicate");

        for (int index = 0; index < 8; index++) VelnRuntime.popHandler();

        VelnRuntime.Result projected = (VelnRuntime.Result) VelnRuntime.ioProviderWriteFile(resource);
        require(projected.isOk(), "WriteFile projection failed before handler identity check");
        Object[] payloads = payloads(projected.value());
        requireCapability(payloads[0], write, "writer");
        requireCapability(payloads[1], seek, "seeker");
        requireCapability(payloads[2], writeAt, "positional writer");
        requireCapability(payloads[3], close, "closer");
        requireCapability(payloads[4], sync, "syncer");
        requireCapability(payloads[5], mutate, "truncater");

        VelnRuntime.popHandler();
    }
}
