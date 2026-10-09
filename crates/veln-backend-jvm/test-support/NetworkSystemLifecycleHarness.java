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
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field NETWORK_RESOURCES;
    private static final java.lang.reflect.Field SYSTEM_OWNER;
    private static final java.lang.reflect.Field READINESS_REGISTRATIONS;
    private static final java.lang.reflect.Field READ_BUFFER_ALLOCATIONS;
    private static final java.lang.reflect.Field RESOLVERS;

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
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            NETWORK_RESOURCES = HANDLER_FRAME.getDeclaredField("networkResources");
            SYSTEM_OWNER = NET_STREAM.getDeclaredField("systemOwner");
            READINESS_REGISTRATIONS = VelnRuntime.class.getDeclaredField(
                "SOCKET_READINESS_REGISTRATIONS"
            );
            READ_BUFFER_ALLOCATIONS = VelnRuntime.class.getDeclaredField(
                "NET_READ_BUFFER_ALLOCATIONS"
            );
            RESOLVERS = VelnRuntime.class.getDeclaredField("NET_SYSTEM_RESOLVERS");
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
                HANDLERS,
                NETWORK_RESOURCES,
                SYSTEM_OWNER,
                READINESS_REGISTRATIONS,
                READ_BUFFER_ALLOCATIONS,
                RESOLVERS
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

    private static java.nio.channels.SocketChannel awaitConnectedPeer(
        java.nio.channels.ServerSocketChannel listener,
        java.nio.channels.SocketChannel client
    ) throws Exception {
        long deadline = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        java.nio.channels.SocketChannel peer = null;
        boolean connected = false;
        while (peer == null || !connected) {
            if (peer == null) peer = listener.accept();
            if (!connected) connected = client.finishConnect();
            if ((peer == null || !connected) && System.nanoTime() >= deadline) {
                if (peer != null) peer.close();
                throw new java.net.SocketTimeoutException("loopback connection timed out");
            }
            Thread.yield();
        }
        return peer;
    }

    private static void verifySuccessfulConnectIsCommitted(Object deadline, Object token) throws Exception {
        java.nio.channels.ServerSocketChannel listener = java.nio.channels.ServerSocketChannel.open();
        java.nio.channels.SocketChannel client = java.nio.channels.SocketChannel.open();
        java.nio.channels.SocketChannel peer = null;
        try {
            listener.configureBlocking(false);
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            client.configureBlocking(false);
            client.connect((java.net.InetSocketAddress) listener.getLocalAddress());
            peer = awaitConnectedPeer(listener, client);
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
        child.join(java.util.concurrent.TimeUnit.SECONDS.toMillis(5L));
        if (child.isAlive()) {
            child.interrupt();
            throw new AssertionError("child resource registration did not complete");
        }
        CLEANUP.invoke(null, owner);
        if (socket.get() == null || socket.get().isOpen()) {
            throw new AssertionError("parent cleanup missed a child-created resource");
        }
    }

    @SuppressWarnings("unchecked")
    private static void verifyResolverCapacityAndHandlerDetachment() throws Exception {
        java.util.concurrent.ThreadPoolExecutor resolvers =
            (java.util.concurrent.ThreadPoolExecutor) RESOLVERS.get(null);
        java.util.concurrent.CountDownLatch started = new java.util.concurrent.CountDownLatch(4);
        java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
        java.util.List<java.util.concurrent.Future<Boolean>> work = new java.util.ArrayList<>();
        java.util.concurrent.atomic.AtomicBoolean retainedHandler =
            new java.util.concurrent.atomic.AtomicBoolean();
        VelnRuntime.pushHandler(
            "resolver-parent",
            new Object[0],
            new Object[0],
            new Object[] { new Object() }
        );
        try {
            for (int index = 0; index < 4; index += 1) {
                work.add(resolvers.submit(() -> {
                    java.util.List<Object> inherited =
                        ((ThreadLocal<java.util.List<Object>>) HANDLERS.get(null)).get();
                    if (!inherited.isEmpty()) retainedHandler.set(true);
                    started.countDown();
                    while (true) {
                        try {
                            release.await();
                            break;
                        } catch (InterruptedException ignored) {
                            // Model a host resolver that ignores interruption.
                        }
                    }
                    return Boolean.valueOf(inherited.isEmpty());
                }));
            }
            if (!started.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
                throw new AssertionError("bounded resolver workers did not start");
            }
            for (java.util.concurrent.Future<Boolean> item : work) item.cancel(true);
            for (int index = 0; index < 32; index += 1) {
                try {
                    resolvers.submit(() -> Boolean.TRUE);
                    throw new AssertionError("resolver accepted work beyond its declared bound");
                } catch (java.util.concurrent.RejectedExecutionException expected) { }
            }
            Object owner = newOwner();
            selectOwner(owner);
            try {
                Object network = VelnRuntime.adt("Network::Tcp4", new Object[0]);
                Object address = VelnRuntime.adt(
                    "Address::Address",
                    new Object[] { network, "127.0.0.1", Long.valueOf(9L) }
                );
                VelnRuntime.Result overloaded = (VelnRuntime.Result) VelnRuntime.netSystemConnect(
                    address,
                    VelnRuntime.none(),
                    VelnRuntime.none()
                );
                if (overloaded.isOk() || !overloaded.toString().contains("Busy")) {
                    throw new AssertionError("resolver overload did not return typed Busy");
                }
            } finally {
                clearOwner();
                CLEANUP.invoke(null, owner);
            }
            if (resolvers.getPoolSize() > 4 || resolvers.getActiveCount() > 4
                || !resolvers.getQueue().isEmpty()) {
                throw new AssertionError("resolver worker or queue bound changed");
            }
        } finally {
            VelnRuntime.popHandler();
            release.countDown();
        }
        long waitUntil = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        while (resolvers.getActiveCount() != 0 && System.nanoTime() < waitUntil) {
            Thread.yield();
        }
        if (retainedHandler.get()) {
            throw new AssertionError("resolver worker retained a lexical handler frame");
        }
        if (resolvers.getActiveCount() != 0 || !resolvers.getQueue().isEmpty()) {
            throw new AssertionError("resolver retained queued work after release");
        }
    }

    private static final class SentinelFailure extends VelnRuntime.RuntimeFailure {
        SentinelFailure(String message) { super(message); }
    }

    @SuppressWarnings("unchecked")
    private static void verifyUnexpectedFailuresRemainAbrupt() throws Exception {
        for (Throwable sentinel : new Throwable[] {
            new SentinelFailure("sentinel runtime failure"),
            new AssertionError("sentinel JVM error")
        }) {
            Object owner = newOwner();
            java.nio.channels.SocketChannel socket = java.nio.channels.SocketChannel.open();
            register(owner, newStream(socket, sentinel instanceof Error ? 9101 : 9100));
            VelnRuntime.Fn provider = new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    if (sentinel instanceof Error) throw (Error) sentinel;
                    throw (RuntimeException) sentinel;
                }
            };
            VelnRuntime.pushHandler(
                "std::host_effects::Network",
                new Object[] { "request" },
                new Object[] { provider },
                new Object[0]
            );
            selectOwner(owner);
            Throwable observed = null;
            try {
                Object network = VelnRuntime.adt("Network::Tcp4", new Object[0]);
                Object address = VelnRuntime.adt(
                    "Address::Address",
                    new Object[] { network, "failure.test", Long.valueOf(443L) }
                );
                VelnRuntime.netSystemResolve(address);
            } catch (Throwable failure) {
                observed = failure;
            } finally {
                CLEANUP.invoke(null, owner);
                clearOwner();
                VelnRuntime.popHandler();
            }
            if (observed != sentinel) {
                throw new AssertionError("unexpected host failure was translated or replaced");
            }
            if (socket.isOpen() || !resources(owner).isEmpty()) {
                throw new AssertionError("abrupt network unwind did not clean every owned resource");
            }
            java.util.List<Object> handlers =
                ((ThreadLocal<java.util.List<Object>>) HANDLERS.get(null)).get();
            if (!handlers.isEmpty()) {
                throw new AssertionError("abrupt network unwind did not restore handler frames");
            }
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
            listener.configureBlocking(false);
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            client.configureBlocking(false);
            client.connect((java.net.InetSocketAddress) listener.getLocalAddress());
            peer = awaitConnectedPeer(listener, client);
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
        verifyResolverCapacityAndHandlerDetachment();
        verifyUnexpectedFailuresRemainAbrupt();
        verifyIdleAcceptReusesReadinessState();
        verifyIdleReadReusesOperationState();
        System.out.println("network system lifecycle invariants held");
    }
}
