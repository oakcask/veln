public final class NetworkSystemLifecycleHarness {
    private static final Class<?> HANDLER_FRAME;
    private static final Class<?> NET_LISTENER;
    private static final Class<?> NET_STREAM;
    private static final java.lang.reflect.Constructor<?> HANDLER_CONSTRUCTOR;
    private static final java.lang.reflect.Constructor<?> LISTENER_CONSTRUCTOR;
    private static final java.lang.reflect.Constructor<?> STREAM_CONSTRUCTOR;
    private static final java.lang.reflect.Method REGISTER_STREAM;
    private static final java.lang.reflect.Method CLEANUP;
    private static final java.lang.reflect.Method FINISH_CONNECT;
    private static final java.lang.reflect.Method PARSE_SOCKET_ADDRESS;
    private static final java.lang.reflect.Method FORMAT_SOCKET_ADDRESS;
    private static final java.lang.reflect.Method ACCEPT_HOST;
    private static final java.lang.reflect.Method READ_HOST;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field NETWORK_RESOURCES;
    private static final java.lang.reflect.Field SYSTEM_OWNER;
    private static final java.lang.reflect.Field READINESS_REGISTRATIONS;
    private static final java.lang.reflect.Field READ_BUFFER_ALLOCATIONS;

    static {
        try {
            HANDLER_FRAME = Class.forName("VelnRuntime$HandlerFrame");
            NET_LISTENER = Class.forName("VelnRuntime$NetListener");
            NET_STREAM = Class.forName("VelnRuntime$NetStream");
            HANDLER_CONSTRUCTOR = HANDLER_FRAME.getDeclaredConstructor(
                String.class,
                java.util.Map.class,
                Object[].class
            );
            LISTENER_CONSTRUCTOR = NET_LISTENER.getDeclaredConstructor(
                String.class,
                String.class,
                java.nio.channels.ServerSocketChannel.class,
                HANDLER_FRAME
            );
            STREAM_CONSTRUCTOR = NET_STREAM.getDeclaredConstructor(
                String.class,
                String.class,
                String.class,
                java.nio.channels.SocketChannel.class,
                HANDLER_FRAME
            );
            REGISTER_STREAM = VelnRuntime.class.getDeclaredMethod(
                "netSystemRegisterStream",
                HANDLER_FRAME,
                NET_STREAM
            );
            CLEANUP = VelnRuntime.class.getDeclaredMethod("cleanupNetworkSystem", HANDLER_FRAME);
            FINISH_CONNECT = VelnRuntime.class.getDeclaredMethod(
                "netSystemFinishConnect",
                java.nio.channels.SocketChannel.class,
                Object.class,
                Object.class
            );
            PARSE_SOCKET_ADDRESS = VelnRuntime.class.getDeclaredMethod(
                "parseSocketAddress",
                String.class
            );
            FORMAT_SOCKET_ADDRESS = VelnRuntime.class.getDeclaredMethod(
                "formatSocketAddress",
                java.net.SocketAddress.class
            );
            ACCEPT_HOST = VelnRuntime.class.getDeclaredMethod(
                "acceptHost",
                NET_LISTENER,
                Object.class,
                Object.class,
                String.class
            );
            READ_HOST = VelnRuntime.class.getDeclaredMethod(
                "readHost",
                NET_STREAM,
                Object.class,
                Object.class,
                String.class
            );
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            NETWORK_RESOURCES = HANDLER_FRAME.getDeclaredField("networkResources");
            SYSTEM_OWNER = NET_STREAM.getDeclaredField("systemOwner");
            READINESS_REGISTRATIONS = VelnRuntime.class.getDeclaredField(
                "SOCKET_READINESS_REGISTRATIONS"
            );
            READ_BUFFER_ALLOCATIONS = VelnRuntime.class.getDeclaredField(
                "NET_READ_BUFFER_ALLOCATIONS"
            );
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLER_CONSTRUCTOR,
                LISTENER_CONSTRUCTOR,
                STREAM_CONSTRUCTOR,
                REGISTER_STREAM,
                CLEANUP,
                FINISH_CONNECT,
                PARSE_SOCKET_ADDRESS,
                FORMAT_SOCKET_ADDRESS,
                ACCEPT_HOST,
                READ_HOST,
                INVOKED_HANDLER,
                NETWORK_RESOURCES,
                SYSTEM_OWNER,
                READINESS_REGISTRATIONS,
                READ_BUFFER_ALLOCATIONS
            }) {
                member.setAccessible(true);
            }
        } catch (ReflectiveOperationException error) {
            throw new ExceptionInInitializerError(error);
        }
    }

    private static Object newOwner() throws Exception {
        return HANDLER_CONSTRUCTOR.newInstance(
            "std::net::IO",
            new java.util.HashMap<String, VelnRuntime.Fn>(),
            new Object[0]
        );
    }

    private static Object newStream(java.nio.channels.SocketChannel socket, int id) throws Exception {
        String name = "stream-" + Integer.toString(id);
        return STREAM_CONSTRUCTOR.newInstance(name, "127.0.0.1:1", "127.0.0.1:2", socket, null);
    }

    private static void register(Object owner, Object stream) throws Exception {
        if (!((Boolean) REGISTER_STREAM.invoke(null, owner, stream)).booleanValue()) {
            throw new AssertionError("resource registration unexpectedly failed");
        }
    }

    @SuppressWarnings("unchecked")
    private static java.util.Set<Object> resources(Object owner) throws Exception {
        return (java.util.Set<Object>) NETWORK_RESOURCES.get(owner);
    }

    @SuppressWarnings("unchecked")
    private static void selectOwner(Object owner) throws Exception {
        ((ThreadLocal<Object>) INVOKED_HANDLER.get(null)).set(owner);
    }

    @SuppressWarnings("unchecked")
    private static void clearOwner() throws Exception {
        ((ThreadLocal<Object>) INVOKED_HANDLER.get(null)).remove();
    }

    private static void verifySuccessfulConnectIsCommitted(Object deadline, Object token) throws Exception {
        java.nio.channels.ServerSocketChannel listener = java.nio.channels.ServerSocketChannel.open();
        java.nio.channels.SocketChannel client = java.nio.channels.SocketChannel.open();
        java.nio.channels.SocketChannel peer = null;
        try {
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            client.configureBlocking(false);
            client.connect((java.net.InetSocketAddress) listener.getLocalAddress());
            peer = listener.accept();
            while (!client.finishConnect()) Thread.yield();
            FINISH_CONNECT.invoke(null, client, deadline, token);
        } finally {
            if (peer != null) peer.close();
            client.close();
            listener.close();
        }
    }

    private static void verifyConnectCommitOrdering() throws Exception {
        verifySuccessfulConnectIsCommitted(Long.valueOf(0L), null);
        Object token = VelnRuntime.timeCancelToken();
        VelnRuntime.timeCancel(token);
        verifySuccessfulConnectIsCommitted(null, token);
    }

    private static void verifyIpv6EndpointTextIsBracketedAndParseable() throws Exception {
        java.net.InetSocketAddress endpoint = new java.net.InetSocketAddress(
            java.net.InetAddress.getByName("::1"),
            443
        );
        String formatted = (String) FORMAT_SOCKET_ADDRESS.invoke(null, endpoint);
        if (!formatted.startsWith("[") || !formatted.endsWith("]:443")) {
            throw new AssertionError("IPv6 endpoint was not bracketed: " + formatted);
        }
        java.net.InetSocketAddress parsed =
            (java.net.InetSocketAddress) PARSE_SOCKET_ADDRESS.invoke(null, formatted);
        if (parsed.isUnresolved()
            || parsed.getPort() != 443
            || !(parsed.getAddress() instanceof java.net.Inet6Address)) {
            throw new AssertionError("bracketed IPv6 endpoint did not round trip: " + parsed);
        }
    }

    private static void verifyIdentityLedgerExplicitCloseAndDetach() throws Exception {
        Object owner = newOwner();
        selectOwner(owner);
        try {
            for (int index = 0; index < 4096; index += 1) {
                Object stream = newStream(java.nio.channels.SocketChannel.open(), index);
                register(owner, stream);
                VelnRuntime.Result closed = (VelnRuntime.Result) VelnRuntime.netSystemCloseStream(stream);
                if (!closed.isOk()) throw new AssertionError("explicit close failed: " + closed);
            }
            if (!resources(owner).isEmpty()) {
                throw new AssertionError("explicit closes left ownership ledger entries");
            }

            Object escaped = newStream(java.nio.channels.SocketChannel.open(), 5000);
            register(owner, escaped);
            for (int index = 0; index < 128; index += 1) {
                register(owner, newStream(java.nio.channels.SocketChannel.open(), 6000 + index));
            }
            CLEANUP.invoke(null, owner);
            if (!resources(owner).isEmpty()) {
                throw new AssertionError("scope cleanup did not detach the ownership ledger");
            }
            if (SYSTEM_OWNER.get(escaped) != owner) {
                throw new AssertionError("escaped resource lost the owner needed for rejection");
            }
        } finally {
            clearOwner();
        }
    }

    private static void verifyInheritedChildRegistrationIsCleaned() throws Exception {
        final Object owner = newOwner();
        final java.util.concurrent.atomic.AtomicReference<java.nio.channels.SocketChannel> socket =
            new java.util.concurrent.atomic.AtomicReference<java.nio.channels.SocketChannel>();
        Thread child = new Thread(new Runnable() {
            public void run() {
                try {
                    java.nio.channels.SocketChannel created = java.nio.channels.SocketChannel.open();
                    socket.set(created);
                    register(owner, newStream(created, 7000));
                } catch (Exception error) {
                    throw new RuntimeException(error);
                }
            }
        });
        child.start();
        child.join();
        CLEANUP.invoke(null, owner);
        if (socket.get() == null || socket.get().isOpen()) {
            throw new AssertionError("parent cleanup missed a child-created resource");
        }
    }

    private static long counter(java.lang.reflect.Field field) throws Exception {
        return ((java.util.concurrent.atomic.AtomicLong) field.get(null)).get();
    }

    private static void verifyIdleAcceptReusesReadinessState() throws Exception {
        java.nio.channels.ServerSocketChannel server = java.nio.channels.ServerSocketChannel.open();
        try {
            server.configureBlocking(false);
            server.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            String local = server.getLocalAddress().toString();
            Object listener = LISTENER_CONSTRUCTOR.newInstance(local, local, server, null);
            long registrationsBefore = counter(READINESS_REGISTRATIONS);
            ACCEPT_HOST.invoke(
                null,
                listener,
                Long.valueOf(System.nanoTime() + 55000000L),
                null,
                "accept"
            );
            long registrations = counter(READINESS_REGISTRATIONS) - registrationsBefore;
            if (registrations != 1L) {
                throw new AssertionError(
                    "idle accept recreated readiness state: registrations=" + registrations
                );
            }
        } finally {
            server.close();
        }
    }

    private static void verifyIdleReadReusesOperationState() throws Exception {
        java.nio.channels.ServerSocketChannel listener = java.nio.channels.ServerSocketChannel.open();
        java.nio.channels.SocketChannel client = java.nio.channels.SocketChannel.open();
        java.nio.channels.SocketChannel peer = null;
        try {
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            client.connect((java.net.InetSocketAddress) listener.getLocalAddress());
            peer = listener.accept();
            client.configureBlocking(false);
            Object stream = newStream(client, 8000);
            long registrationsBefore = counter(READINESS_REGISTRATIONS);
            long buffersBefore = counter(READ_BUFFER_ALLOCATIONS);
            READ_HOST.invoke(
                null,
                stream,
                Long.valueOf(System.nanoTime() + 55000000L),
                null,
                "read"
            );
            long registrations = counter(READINESS_REGISTRATIONS) - registrationsBefore;
            long buffers = counter(READ_BUFFER_ALLOCATIONS) - buffersBefore;
            if (registrations != 1L || buffers != 1L) {
                throw new AssertionError(
                    "idle read recreated operation state: registrations="
                        + registrations
                        + ", buffers="
                        + buffers
                );
            }
        } finally {
            if (peer != null) peer.close();
            client.close();
            listener.close();
        }
    }

    public static void main(String[] args) throws Exception {
        verifyConnectCommitOrdering();
        verifyIpv6EndpointTextIsBracketedAndParseable();
        verifyIdentityLedgerExplicitCloseAndDetach();
        verifyInheritedChildRegistrationIsCleaned();
        verifyIdleAcceptReusesReadinessState();
        verifyIdleReadReusesOperationState();
        System.out.println("network system lifecycle invariants held");
    }
}
