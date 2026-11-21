// JVM host-boundary contract: malformed UTF-16 must not become a different
// native filename through replacement encoding. Ordinary source API cases
// remain in the filesystem companion tests.
public final class IoProviderNativePathHarness {
    private static void reject(String value) {
        VelnRuntime.Result result = (VelnRuntime.Result) VelnRuntime.ioProviderNativePath(value);
        if (result.isOk()
            || !VelnRuntime.isAdt(VelnRuntime.recordField(result.value(), "kind"), "ErrorKind::UnrepresentablePath")) {
            throw new AssertionError("malformed UTF-16 crossed the native path boundary");
        }
    }

    public static void main(String[] args) {
        reject("bad\uD800");
        reject("bad\uDC00");
        reject("bad\uD800x");
        String scalar = "native-\uD83D\uDE00";
        VelnRuntime.Result result = (VelnRuntime.Result) VelnRuntime.ioProviderNativePath(scalar);
        if (!result.isOk() || !scalar.equals(VelnRuntime.ioProviderDisplayNative(result.value()))) {
            throw new AssertionError("valid supplementary scalar changed at the native boundary");
        }
        System.out.println("native path encoding boundary verified");
    }
}
