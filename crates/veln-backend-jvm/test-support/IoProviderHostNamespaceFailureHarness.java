import java.lang.reflect.Constructor;
import java.lang.reflect.Method;
import java.nio.file.*;
import java.nio.file.attribute.*;
import java.util.Map;
import java.util.Set;

// Exercise actual native namespace failures through the production adapter.
// Native bytes and bindings are observed separately from the reported outcomes.
public final class IoProviderHostNamespaceFailureHarness {
    private static Object construct(String name, Class<?>[] signature, Object... values) throws Exception {
        Constructor<?> constructor = Class.forName("VelnRuntime$" + name).getDeclaredConstructor(signature);
        constructor.setAccessible(true);
        return constructor.newInstance(values);
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static Object portable(String text) {
        return VelnRuntime.adt("Path::PortablePath", new Object[] {text});
    }

    private static Object invoke(Object resource, String operation, String path) throws Exception {
        Method method = VelnRuntime.class.getDeclaredMethod("ioHostNamespace", resource.getClass(), String.class, Object.class);
        method.setAccessible(true);
        return method.invoke(null, resource, operation, Map.of("path", portable(path), "follow", VelnRuntime.adt("FollowLinks::Follow", new Object[] {})));
    }

    private static void error(Object value, String operation, String path, String kind, String commit) {
        Object error;
        if (operation.equals("create_directory")) {
            check(VelnRuntime.isAdt(value, "MutationOutcome::MutationFailed"), "native mutation unexpectedly succeeded");
            check(VelnRuntime.isAdt(VelnRuntime.adtPayload(value, 0), "Commit::" + commit), "native mutation fabricated commit facts");
            error = VelnRuntime.adtPayload(value, 1);
        } else {
            VelnRuntime.Result result = (VelnRuntime.Result) value;
            check(!result.isOk(), "native query unexpectedly succeeded");
            error = result.value();
        }
        check(operation.equals(VelnRuntime.recordField(error, "operation")), "native error lost its operation");
        check(VelnRuntime.isAdt(VelnRuntime.recordField(error, "kind"), "ErrorKind::" + kind), "native " + operation + " expected " + kind + " but received " + VelnRuntime.recordField(error, "kind"));
        Object encoded = VelnRuntime.optionValue(VelnRuntime.recordField(error, "path"));
        check(path.equals(VelnRuntime.adtPayload(encoded, 0)), "native error changed its portable path");
        check(VelnRuntime.isNone(VelnRuntime.recordField(error, "other_path")), "single-path error invented another path");
    }

    private static String nativeCategory(Path path, boolean create) throws Exception {
        try {
            if (create) Files.createDirectory(path);
            else Files.readAttributes(path, BasicFileAttributes.class);
            throw new AssertionError("native traversal unexpectedly succeeded");
        } catch (FileSystemLoopException failure) { return "LinkLoop"; }
        catch (AccessDeniedException failure) { return "PermissionDenied"; }
        catch (NotDirectoryException failure) { return "NotDirectory"; }
        catch (NoSuchFileException failure) { return "NotFound"; }
        catch (FileAlreadyExistsException failure) { return "AlreadyExists"; }
        catch (FileSystemException failure) { return "Other"; }
    }

    public static void main(String[] args) throws Exception {
        Path root = Files.createTempDirectory("veln-native-namespace-");
        Path denied = Files.createDirectory(root.resolve("denied"));
        Path data = root.resolve("data");
        Files.writeString(data, "original");
        Files.writeString(denied.resolve("secret"), "retained");
        Files.createSymbolicLink(root.resolve("loop"), Path.of("loop"));
        Set<PosixFilePermission> permissions = Files.getPosixFilePermissions(denied);
        try {
            Class<?> ownerType = Class.forName("VelnRuntime$IoProviderOwner");
            Object owner = construct("IoProviderOwner", new Class<?>[0]);
            Object filesystem = construct("IoHostFilesystem", new Class<?>[] {Path.class}, root);
            Object resource = construct("IoProviderResource", new Class<?>[] {ownerType, long.class, String.class, Map.class, Object.class}, owner, 1L, "MutableFilesystem", Map.of(), filesystem);

            Files.setPosixFilePermissions(denied, Set.of());
            check(nativeCategory(denied.resolve("secret"), false).equals("PermissionDenied"), "test process can bypass native permissions");
            error(invoke(resource, "metadata", "denied/secret"), "metadata", "denied/secret", "PermissionDenied", null);
            error(invoke(resource, "entries", "denied"), "entries", "denied", "PermissionDenied", null);
            error(invoke(resource, "create_directory", "denied/new"), "create_directory", "denied/new", "PermissionDenied", "No");
            Files.setPosixFilePermissions(denied, permissions);
            check(Files.readString(denied.resolve("secret")).equals("retained") && !Files.exists(denied.resolve("new")), "denied mutation changed native state");

            String loopKind = nativeCategory(root.resolve("loop"), false);
            error(invoke(resource, "metadata", "loop"), "metadata", "loop", loopKind, null);
            check(Files.readSymbolicLink(root.resolve("loop")).equals(Path.of("loop")), "failed traversal changed a link binding");

            String traversalKind = nativeCategory(data.resolve("child"), false);
            error(invoke(resource, "metadata", "data/child"), "metadata", "data/child", traversalKind, null);
            String mutationKind = nativeCategory(data.resolve("child"), true);
            error(invoke(resource, "create_directory", "data/child"), "create_directory", "data/child", mutationKind, mutationKind.equals("Other") ? "Unknown" : "No");
            check(Files.readString(data).equals("original"), "failed native traversal changed file bytes");

            String longName = "x".repeat(300);
            check(nativeCategory(root.resolve(longName), false).equals("Other"), "host supplied a classified long-name failure");
            error(invoke(resource, "metadata", longName), "metadata", longName, "Other", null);
            check(nativeCategory(root.resolve(longName), true).equals("Other"), "host supplied a classified long-name mutation failure");
            error(invoke(resource, "create_directory", longName), "create_directory", longName, "Other", "Unknown");
            try (var names = Files.list(root)) {
                check(names.map(path -> path.getFileName().toString()).sorted().toList().equals(java.util.List.of("data", "denied", "loop")), "unclassified traversal changed native namespace bindings");
            }

            // FOLLOW_LINKS walking supplies a typed loop exception even on hosts
            // whose single-path stat reports only a generic filesystem error.
            Path cycle = Files.createDirectory(root.resolve("cycle"));
            Files.createSymbolicLink(cycle.resolve("back"), Path.of("."));
            try {
                Files.walkFileTree(cycle, Set.of(FileVisitOption.FOLLOW_LINKS), 10, new SimpleFileVisitor<Path>() {});
                throw new AssertionError("native walking did not detect its cycle");
            } catch (FileSystemLoopException failure) {
                Method translate = VelnRuntime.class.getDeclaredMethod("ioHostError", String.class, Throwable.class, Object.class, Object.class, Object.class);
                translate.setAccessible(true);
                Object translated = translate.invoke(null, "metadata", failure, portable("cycle/back"), null, null);
                check(VelnRuntime.isAdt(VelnRuntime.recordField(translated, "kind"), "ErrorKind::LinkLoop"), "typed native loop lost its category");
            }
            check(Files.readSymbolicLink(cycle.resolve("back")).equals(Path.of(".")), "cycle traversal changed its binding");
            System.out.println("native permission loop and traversal failures verified");
        } finally {
            Files.setPosixFilePermissions(denied, permissions);
            Files.deleteIfExists(root.resolve("cycle/back"));
            Files.deleteIfExists(root.resolve("cycle"));
            Files.deleteIfExists(root.resolve("loop"));
            Files.deleteIfExists(denied.resolve("secret"));
            Files.deleteIfExists(denied.resolve("new"));
            Files.deleteIfExists(denied);
            Files.deleteIfExists(data);
            Files.deleteIfExists(root);
        }
    }
}
