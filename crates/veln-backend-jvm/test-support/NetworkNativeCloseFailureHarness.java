import java.io.FileDescriptor;
import java.io.FileInputStream;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.net.InetSocketAddress;
import java.nio.ByteBuffer;
import java.nio.channels.ServerSocketChannel;
import java.nio.channels.SocketChannel;

// Real Unix socket descriptor failure; no fake network or handler is installed.
// The saved descriptor owns the socket until finally, independently proving
// that logical channel closure is insufficient evidence for native release.
public final class NetworkNativeCloseFailureHarness {
    private static Object field(Object value, String name) throws Exception {
        Field field = value.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(value);
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    private static void failed(Object stream) throws Exception {
        try {
            VelnRuntime.netCloseStream(stream);
            throw new AssertionError("unconfirmed socket release became successful close");
        } catch (RuntimeException failure) {
            Object commit;
            try {
                commit = field(failure, "ownershipCommitted");
            } catch (NoSuchFieldException unexpected) {
                throw new AssertionError("unexpected native close failure: " + failure.getClass().getName() + ": " + failure.getMessage(), failure);
            }
            check(commit == null, "logical socket closure fabricated a known ownership commit");
        }
    }

    public static void main(String[] args) throws Exception {
        try (ServerSocketChannel listener = ServerSocketChannel.open()) {
            listener.bind(new InetSocketAddress("127.0.0.1", 0));
            try (SocketChannel client = SocketChannel.open(listener.getLocalAddress());
                 SocketChannel peer = listener.accept()) {
                ByteBuffer sent = ByteBuffer.wrap(new byte[] {97, 98});
                while (sent.hasRemaining()) peer.write(sent);
                FileDescriptor descriptor = (FileDescriptor) field(client, "fd");
                Field number = FileDescriptor.class.getDeclaredField("fd");
                number.setAccessible(true);
                int original = number.getInt(descriptor);
                check(original > 2, "fixture must own a distinct native socket descriptor");
                FileDescriptor saved = new FileDescriptor();
                number.setInt(saved, original);
                Class<?> streamType = Class.forName("VelnRuntime$NetStream");
                Class<?> handlerType = Class.forName("VelnRuntime$HandlerFrame");
                Constructor<?> constructor = streamType.getDeclaredConstructor(String.class, String.class, String.class, SocketChannel.class, handlerType);
                constructor.setAccessible(true);
                Object stream = constructor.newInstance("native-close", "local", "peer", client, null);
                number.setInt(descriptor, -2);
                try (FileInputStream retained = new FileInputStream(saved)) {
                    failed(stream);
                    check(!client.isOpen(), "real channel should already be logically closed");
                    check(retained.read() == 97, "first close failure must leave the independent descriptor readable");
                    failed(stream);
                    check(retained.read() == 98, "retry must not fabricate physical release of the live descriptor");
                    check(!((Boolean) field(stream, "nativeReleased")), "failed close cannot establish native release");
                } finally {
                    number.setInt(descriptor, -1);
                }
            }
        }
        System.out.println("native socket close uncertainty verified");
    }
}
