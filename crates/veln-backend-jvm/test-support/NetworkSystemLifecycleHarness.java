public final class NetworkSystemLifecycleHarness {
    private static final Class<?> HANDLER_FRAME;
    private static final Class<?> NET_LISTENER;
    private static final Class<?> NET_STREAM;
    private static final Class<?> RESOLVED_ENDPOINT;
    private static final java.lang.reflect.Constructor<?> HANDLER_CONSTRUCTOR;
    private static final java.lang.reflect.Constructor<?> LISTENER_CONSTRUCTOR;
    private static final java.lang.reflect.Constructor<?> STREAM_CONSTRUCTOR;
    private static final java.lang.reflect.Constructor<?> ENDPOINT_CONSTRUCTOR;
    private static final java.lang.reflect.Method PUBLISH_RESOURCE;
    private static final java.lang.reflect.Method CLEANUP;
    private static final java.lang.reflect.Method FINISH_CONNECT;
    private static final java.lang.reflect.Method PARSE_SOCKET_ADDRESS;
    private static final java.lang.reflect.Method FORMAT_SOCKET_ADDRESS;
    private static final java.lang.reflect.Method ACCEPT_HOST;
    private static final java.lang.reflect.Method READ_HOST;
    private static final java.lang.reflect.Method AWAIT_RESOLUTION;
    private static final java.lang.reflect.Method CONNECT_RESOLVED;
    private static final java.lang.reflect.Field INVOKED_HANDLER;
    private static final java.lang.reflect.Field HANDLERS;
    private static final java.lang.reflect.Field NETWORK_RESOURCES;
    private static final java.lang.reflect.Field SYSTEM_OWNER;
    private static final java.lang.reflect.Field READING;
    private static final java.lang.reflect.Field WRITING;
    private static final java.lang.reflect.Field PEER_ENDED;
    private static final java.lang.reflect.Field WRITE_FAILED;
    private static final java.lang.reflect.Field READINESS_REGISTRATIONS;
    private static final java.lang.reflect.Field READ_BUFFER_ALLOCATIONS;
    private static final java.lang.reflect.Field RESOLVERS;
    private static final java.lang.reflect.Field CONNECT_ATTEMPTS;

    static {
        try {
            HANDLER_FRAME = Class.forName("VelnRuntime$HandlerFrame");
            NET_LISTENER = Class.forName("VelnRuntime$NetListener");
            NET_STREAM = Class.forName("VelnRuntime$NetStream");
            RESOLVED_ENDPOINT = Class.forName("VelnRuntime$NetSystemResolvedEndpoint");
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
            ENDPOINT_CONSTRUCTOR = RESOLVED_ENDPOINT.getDeclaredConstructor(
                Object.class,
                String.class
            );
            PUBLISH_RESOURCE = VelnRuntime.class.getDeclaredMethod(
                "netSystemPublishResource",
                HANDLER_FRAME,
                Object.class
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
            AWAIT_RESOLUTION = VelnRuntime.class.getDeclaredMethod(
                "netSystemAwaitResolution",
                java.util.concurrent.Future.class,
                Object.class,
                Object.class,
                String.class
            );
            CONNECT_RESOLVED = VelnRuntime.class.getDeclaredMethod(
                "netSystemConnectResolved",
                HANDLER_FRAME,
                Object.class,
                Object.class,
                Object.class,
                java.util.List.class
            );
            INVOKED_HANDLER = VelnRuntime.class.getDeclaredField("INVOKED_HANDLER");
            HANDLERS = VelnRuntime.class.getDeclaredField("HANDLERS");
            NETWORK_RESOURCES = HANDLER_FRAME.getDeclaredField("networkResources");
            SYSTEM_OWNER = NET_STREAM.getDeclaredField("systemOwner");
            READING = NET_STREAM.getDeclaredField("reading");
            WRITING = NET_STREAM.getDeclaredField("writing");
            PEER_ENDED = NET_STREAM.getDeclaredField("peerEnded");
            WRITE_FAILED = NET_STREAM.getDeclaredField("writeFailed");
            READINESS_REGISTRATIONS = VelnRuntime.class.getDeclaredField(
                "SOCKET_READINESS_REGISTRATIONS"
            );
            READ_BUFFER_ALLOCATIONS = VelnRuntime.class.getDeclaredField(
                "NET_READ_BUFFER_ALLOCATIONS"
            );
            RESOLVERS = VelnRuntime.class.getDeclaredField("NET_SYSTEM_RESOLVERS");
            CONNECT_ATTEMPTS = VelnRuntime.class.getDeclaredField("NET_SYSTEM_CONNECT_ATTEMPTS");
            for (java.lang.reflect.AccessibleObject member : new java.lang.reflect.AccessibleObject[] {
                HANDLER_CONSTRUCTOR,
                LISTENER_CONSTRUCTOR,
                STREAM_CONSTRUCTOR,
                ENDPOINT_CONSTRUCTOR,
                PUBLISH_RESOURCE,
                CLEANUP,
                FINISH_CONNECT,
                PARSE_SOCKET_ADDRESS,
                FORMAT_SOCKET_ADDRESS,
                ACCEPT_HOST,
                READ_HOST,
                AWAIT_RESOLUTION,
                CONNECT_RESOLVED,
                INVOKED_HANDLER,
                HANDLERS,
                NETWORK_RESOURCES,
                SYSTEM_OWNER,
                READING,
                WRITING,
                PEER_ENDED,
                WRITE_FAILED,
                READINESS_REGISTRATIONS,
                READ_BUFFER_ALLOCATIONS,
                RESOLVERS,
                CONNECT_ATTEMPTS
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
        if (!((Boolean) PUBLISH_RESOURCE.invoke(null, owner, stream)).booleanValue()) {
            throw new AssertionError("resource registration unexpectedly failed");
        }
    }

    private static void registerListener(Object owner, Object listener) throws Exception {
        if (!((Boolean) PUBLISH_RESOURCE.invoke(null, owner, listener)).booleanValue()) {
            throw new AssertionError("listener registration unexpectedly failed");
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

    private static void requireResultContains(Object result, String expected, String context) {
        if (!result.toString().contains(expected)) {
            throw new AssertionError(context + " produced " + result);
        }
    }

    private static void verifyOwnedResourcesDoNotCrossHandlerDispatch() throws Exception {
        Object owner = newOwner();
        Object stream = newStream(null, 91);
        SYSTEM_OWNER.set(stream, owner);
        java.util.concurrent.atomic.AtomicBoolean invoked =
            new java.util.concurrent.atomic.AtomicBoolean();
        VelnRuntime.Fn provider = new VelnRuntime.Fn() {
            public Object call(Object... args) {
                invoked.set(true);
                return VelnRuntime.ok(VelnRuntime.UNIT);
            }
        };
        VelnRuntime.pushHandler(
            "std::net::IO",
            new Object[] { "peer_address", "write" },
            new Object[] { provider, provider },
            new Object[0]
        );
        try {
            requireResultContains(
                VelnRuntime.perform(
                    "std::net::IO", "peer_address", new Object[] { stream }
                ),
                "InvalidResource",
                "cross-handler stream query"
            );
            requireResultContains(
                VelnRuntime.perform(
                    "std::net::IO",
                    "write",
                    new Object[] { stream, null, null, null }
                ),
                "WriteFailed(ByteCount(0), NetError(write, None, None, InvalidResource",
                "cross-handler stream write"
            );
            if (invoked.get()) {
                throw new AssertionError("cross-handler resource reached the selected provider");
            }
        } finally {
            VelnRuntime.popHandler();
            SYSTEM_OWNER.set(stream, null);
        }
    }

    private static void verifyTerminalStatesDoNotBypassContention() throws Exception {
        Object owner = newOwner();
        Object stream = newStream(null, 92);
        SYSTEM_OWNER.set(stream, owner);
        selectOwner(owner);
        try {
            java.util.concurrent.atomic.AtomicBoolean reading =
                (java.util.concurrent.atomic.AtomicBoolean) READING.get(stream);
            java.util.concurrent.atomic.AtomicBoolean writing =
                (java.util.concurrent.atomic.AtomicBoolean) WRITING.get(stream);
            PEER_ENDED.setBoolean(stream, true);
            reading.set(true);
            requireResultContains(
                VelnRuntime.netSystemRead(stream, null, null),
                "Busy",
                "peer-ended read while another read still owns the direction"
            );
            reading.set(false);
            requireResultContains(
                VelnRuntime.netSystemRead(stream, null, null),
                "ReadEnd",
                "peer-ended read after direction release"
            );

            WRITE_FAILED.setBoolean(stream, true);
            writing.set(true);
            requireResultContains(
                VelnRuntime.netSystemWrite(stream, null, null, null),
                "Busy",
                "failed write while another write still owns the direction"
            );
            writing.set(false);
            requireResultContains(
                VelnRuntime.netSystemWrite(stream, null, null, null),
                "Closed",
                "failed write after direction release"
            );
        } finally {
            SYSTEM_OWNER.set(stream, null);
            clearOwner();
        }
    }

    private static String unavailableLocalIpv4Address() throws Exception {
        for (String candidate : new String[] {
            "192.0.2.1", "198.51.100.1", "203.0.113.1", "240.0.0.1"
        }) {
            java.net.InetAddress address = java.net.InetAddress.getByName(candidate);
            if (java.net.NetworkInterface.getByInetAddress(address) != null) continue;
            java.nio.channels.ServerSocketChannel probe =
                java.nio.channels.ServerSocketChannel.open();
            try {
                probe.bind(new java.net.InetSocketAddress(address, 0));
            } catch (java.net.BindException expected) {
                return candidate;
            } finally {
                probe.close();
            }
        }
        throw new AssertionError("could not establish an unavailable local IPv4 address");
    }

    private static void requireListenFailureKind(Object result, String kind, String context) {
        if (((VelnRuntime.Result) result).isOk() || !result.toString().contains(kind)) {
            throw new AssertionError(context + " produced the wrong failure: " + result);
        }
    }

    private static void verifyDirectBindFailureClassification() throws Exception {
        Object owner = newOwner();
        java.nio.channels.ServerSocketChannel occupied =
            java.nio.channels.ServerSocketChannel.open();
        selectOwner(owner);
        try {
            java.net.InetAddress loopback = java.net.InetAddress.getByName("127.0.0.1");
            occupied.bind(new java.net.InetSocketAddress(loopback, 0));
            int port = ((java.net.InetSocketAddress) occupied.getLocalAddress()).getPort();
            Object network = VelnRuntime.adt("Network::Tcp4", new Object[0]);
            Object duplicate = VelnRuntime.adt(
                "Address::Address",
                new Object[] { network, "127.0.0.1", Long.valueOf(port) }
            );
            requireListenFailureKind(
                VelnRuntime.netSystemListen(duplicate),
                "AddressInUse",
                "duplicate loopback bind"
            );

            String unavailable = unavailableLocalIpv4Address();
            Object nonlocal = VelnRuntime.adt(
                "Address::Address",
                new Object[] { network, unavailable, Long.valueOf(0L) }
            );
            Object result = VelnRuntime.netSystemListen(nonlocal);
            requireListenFailureKind(result, "Other", "unavailable local-address bind");
            if (result.toString().contains("AddressInUse")) {
                throw new AssertionError("unavailable local address was reported as a collision");
            }
        } finally {
            occupied.close();
            CLEANUP.invoke(null, owner);
            clearOwner();
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

    private static Object closeReply(String status, long ownershipCommitted) {
        return VelnRuntime.record(
            "status", status,
            "value", "",
            "local", "127.0.0.1:1",
            "peer", "127.0.0.1:2",
            "resolved_endpoints", VelnRuntime.listNil(),
            "bytes_committed", Long.valueOf(0L),
            "category", "io_failure",
            "phase", "during_operation",
            "input_committed", Long.valueOf(0L),
            "output_committed", Long.valueOf(0L),
            "ownership_committed", Long.valueOf(ownershipCommitted),
            "cause", "injected close failure"
        );
    }

    private static Object successfulNetworkReply(String operation) {
        Object endpoints = VelnRuntime.listCons(
            VelnRuntime.record("network", "tcp4", "address", "127.0.0.1"),
            VelnRuntime.listNil()
        );
        String value = operation.equals("listen") ? "listener-created"
            : operation.equals("accept") || operation.equals("connect") ? "stream-created"
            : "";
        return VelnRuntime.record(
            "status", "ok",
            "value", value,
            "local", "127.0.0.1:41000",
            "peer", "127.0.0.1:443",
            "resolved_endpoints", endpoints,
            "bytes_committed", Long.valueOf(0L),
            "category", "",
            "phase", "",
            "input_committed", Long.valueOf(0L),
            "output_committed", Long.valueOf(0L),
            "ownership_committed", Long.valueOf(0L),
            "cause", ""
        );
    }

    @SuppressWarnings("unchecked")
    private static Object topHandler() throws Exception {
        java.util.List<Object> handlers =
            ((ThreadLocal<java.util.List<Object>>) HANDLERS.get(null)).get();
        return handlers.get(handlers.size() - 1);
    }

    private static void verifyOrdinaryPopRetriesCleanupAndFailsTerminally(
        long firstCommit
    ) throws Exception {
        for (boolean persistent : new boolean[] { false, true }) {
            java.util.concurrent.atomic.AtomicInteger targetCloses =
                new java.util.concurrent.atomic.AtomicInteger();
            java.util.concurrent.atomic.AtomicInteger independentCloses =
                new java.util.concurrent.atomic.AtomicInteger();
            VelnRuntime.Fn provider = new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    String operation = (String) args[0];
                    String subject = (String) args[1];
                    if (!operation.equals("close_stream")) {
                        throw new AssertionError("unexpected cleanup operation " + operation);
                    }
                    if (subject.equals("target")) {
                        int attempt = targetCloses.getAndIncrement();
                        return persistent || attempt == 0
                            ? closeReply("error", firstCommit)
                            : closeReply("ok", 1L);
                    }
                    independentCloses.incrementAndGet();
                    return closeReply("ok", 1L);
                }
            };
            VelnRuntime.pushHandler(
                "std::host_effects::Network",
                new Object[] { "request" },
                new Object[] { provider },
                new Object[0]
            );
            Object adapter = topHandler();
            VelnRuntime.pushHandler(
                "std::net::IO",
                new Object[0],
                new Object[0],
                new Object[0]
            );
            Object owner = topHandler();
            Object target = STREAM_CONSTRUCTOR.newInstance(
                "target", "127.0.0.1:1", "127.0.0.1:2", null, adapter
            );
            Object independent = STREAM_CONSTRUCTOR.newInstance(
                "independent", "127.0.0.1:1", "127.0.0.1:2", null, adapter
            );
            register(owner, target);
            register(owner, independent);
            Throwable failure = null;
            try {
                VelnRuntime.popHandler();
            } catch (Throwable observed) {
                failure = observed;
            } finally {
                VelnRuntime.popHandler();
            }
            if (persistent) {
                if (!(failure instanceof VelnRuntime.RuntimeFailure)
                    || targetCloses.get() != 3
                    || independentCloses.get() != 1
                    || resources(owner).size() != 1) {
                    throw new AssertionError(
                        "persistent cleanup did not fail after bounded complete passes"
                    );
                }
            } else if (failure != null
                || targetCloses.get() != 2
                || independentCloses.get() != 1
                || !resources(owner).isEmpty()) {
                throw new AssertionError("transient cleanup did not settle through popHandler");
            }
        }
    }

    private static void verifyCleanupUncertaintyIsMonotonic() throws Exception {
        java.util.concurrent.atomic.AtomicInteger closes =
            new java.util.concurrent.atomic.AtomicInteger();
        VelnRuntime.Fn provider = new VelnRuntime.Fn() {
            public Object call(Object... args) {
                String operation = (String) args[0];
                if (!operation.equals("close_stream")) {
                    throw new AssertionError("unexpected cleanup operation " + operation);
                }
                int attempt = closes.getAndIncrement();
                if (attempt == 0) return closeReply("error", -1L);
                if (attempt == 1) return closeReply("error", 0L);
                return closeReply("ok", 1L);
            }
        };
        VelnRuntime.pushHandler(
            "std::host_effects::Network",
            new Object[] { "request" },
            new Object[] { provider },
            new Object[0]
        );
        Object adapter = topHandler();
        VelnRuntime.pushHandler(
            "std::net::IO",
            new Object[0],
            new Object[0],
            new Object[0]
        );
        Object owner = topHandler();
        Object target = STREAM_CONSTRUCTOR.newInstance(
            "target", "127.0.0.1:1", "127.0.0.1:2", null, adapter
        );
        register(owner, target);
        Throwable failure = null;
        try {
            VelnRuntime.popHandler();
        } catch (Throwable observed) {
            failure = observed;
        } finally {
            VelnRuntime.popHandler();
        }
        if (failure != null || closes.get() != 3 || !resources(owner).isEmpty()) {
            throw new AssertionError(
                "cleanup did not retain uncertainty through an uncommitted retry"
            );
        }
    }

    private static void verifyLateProducerPathRetriesDuringProductionCleanup(
        String producer,
        long firstCommit,
        boolean persistent
    ) throws Exception {
        java.util.concurrent.CountDownLatch created = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
        java.util.concurrent.atomic.AtomicInteger closes =
            new java.util.concurrent.atomic.AtomicInteger();
        VelnRuntime.Fn provider = new VelnRuntime.Fn() {
            public Object call(Object... args) {
                String operation = (String) args[0];
                if (operation.equals(producer)) {
                    created.countDown();
                    while (true) {
                        try {
                            release.await();
                            break;
                        } catch (InterruptedException ignored) { }
                    }
                    return successfulNetworkReply(operation);
                }
                if (operation.startsWith("resolve_")) {
                    return successfulNetworkReply(operation);
                }
                if (operation.equals("close_listener") && !producer.equals("listen")) {
                    return closeReply("ok", 1L);
                }
                if (operation.equals("close_listener") || operation.equals("close_stream")) {
                    int attempt = closes.getAndIncrement();
                    return !persistent && attempt != 0
                        ? closeReply("ok", 1L)
                        : closeReply("error", firstCommit);
                }
                throw new AssertionError("unexpected " + producer + " adapter operation " + operation);
            }
        };
        VelnRuntime.pushHandler(
            "std::host_effects::Network",
            new Object[] { "request" },
            new Object[] { provider },
            new Object[0]
        );
        Object adapter = topHandler();
        VelnRuntime.pushHandler(
            "std::net::IO",
            new Object[0],
            new Object[0],
            new Object[0]
        );
        Object owner = topHandler();
        Object network = VelnRuntime.adt("Network::Tcp4", new Object[0]);
        Object address = VelnRuntime.adt(
            "Address::Address",
            new Object[] { network, "producer.test", Long.valueOf(443L) }
        );
        if (producer.equals("accept")) {
            Object listener = LISTENER_CONSTRUCTOR.newInstance(
                "listener-created",
                "127.0.0.1:41000",
                null,
                adapter
            );
            registerListener(owner, listener);
            address = listener;
        }
        Object argument = address;
        java.util.concurrent.atomic.AtomicReference<Object> result =
            new java.util.concurrent.atomic.AtomicReference<Object>();
        java.util.concurrent.atomic.AtomicReference<Throwable> childFailure =
            new java.util.concurrent.atomic.AtomicReference<Throwable>();
        Thread child = new Thread(() -> {
            try {
                selectOwner(owner);
                if (producer.equals("listen")) {
                    result.set(VelnRuntime.netSystemListen(argument));
                } else if (producer.equals("connect")) {
                    result.set(VelnRuntime.netSystemConnect(
                        argument,
                        VelnRuntime.none(),
                        VelnRuntime.none()
                    ));
                } else {
                    result.set(VelnRuntime.netSystemAccept(
                        argument,
                        VelnRuntime.none(),
                        VelnRuntime.none()
                    ));
                }
            } catch (Throwable failure) {
                childFailure.set(failure);
            } finally {
                try { clearOwner(); } catch (Exception failure) { childFailure.set(failure); }
            }
        });
        boolean ownerPopped = false;
        try {
            child.start();
            if (!created.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
                throw new AssertionError(producer + " did not reach the publication barrier");
            }
            java.util.concurrent.atomic.AtomicReference<Throwable> popFailure =
                new java.util.concurrent.atomic.AtomicReference<Throwable>();
            Thread popper = new Thread(() -> {
                try {
                    VelnRuntime.popHandler();
                } catch (Throwable failure) {
                    popFailure.set(failure);
                }
            });
            popper.start();
            popper.join(50L);
            if (!popper.isAlive()) {
                throw new AssertionError(producer + " scope exit did not wait for its producer");
            }
            release.countDown();
            child.join(java.util.concurrent.TimeUnit.SECONDS.toMillis(5L));
            popper.join(java.util.concurrent.TimeUnit.SECONDS.toMillis(5L));
            if (child.isAlive()) {
                child.interrupt();
                throw new AssertionError(producer + " publication barrier did not complete");
            }
            if (popper.isAlive()) {
                popper.interrupt();
                throw new AssertionError(producer + " scope cleanup did not complete");
            }
            java.util.List<Object> parentHandlers =
                ((ThreadLocal<java.util.List<Object>>) HANDLERS.get(null)).get();
            if (parentHandlers.remove(parentHandlers.size() - 1) != owner) {
                throw new AssertionError("production pop removed the wrong network owner");
            }
            ownerPopped = true;
            if (persistent) {
                if (!(childFailure.get() instanceof VelnRuntime.RuntimeFailure)
                    || !(popFailure.get() instanceof VelnRuntime.RuntimeFailure)
                    || closes.get() < 3
                    || closes.get() > 6
                    || resources(owner).size() != 1) {
                    throw new AssertionError(
                        producer + " persistent late cleanup did not fail terminally: child="
                            + childFailure.get() + ", pop=" + popFailure.get()
                            + ", closes=" + closes.get()
                            + ", retained=" + resources(owner).size()
                    );
                }
            } else {
                if (childFailure.get() != null) {
                    throw new AssertionError(producer + " publication failed", childFailure.get());
                }
                if (popFailure.get() != null) {
                    throw new AssertionError(producer + " scope cleanup failed", popFailure.get());
                }
                if (closes.get() != 2 || !resources(owner).isEmpty()) {
                    throw new AssertionError(producer + " cleanup retry did not commit and detach");
                }
                if (((VelnRuntime.Result) result.get()).isOk()
                    || !result.get().toString().contains("InvalidResource")) {
                    throw new AssertionError(producer + " did not reject late publication");
                }
            }
        } finally {
            release.countDown();
            if (!ownerPopped) VelnRuntime.popHandler();
            VelnRuntime.popHandler();
        }
    }

    private static void verifyAtomicResourcePublication() throws Exception {
        verifyCleanupUncertaintyIsMonotonic();
        for (long commit : new long[] { 0L, -1L }) {
            verifyOrdinaryPopRetriesCleanupAndFailsTerminally(commit);
            for (boolean persistent : new boolean[] { false, true }) {
                verifyLateProducerPathRetriesDuringProductionCleanup(
                    "listen", commit, persistent
                );
                verifyLateProducerPathRetriesDuringProductionCleanup(
                    "connect", commit, persistent
                );
                verifyLateProducerPathRetriesDuringProductionCleanup(
                    "accept", commit, persistent
                );
            }
        }
    }

    private static void requireClosedFailure(Object result, String operation) {
        if (((VelnRuntime.Result) result).isOk() || !result.toString().contains("Closed")) {
            throw new AssertionError(operation + " did not return a typed Closed failure: " + result);
        }
    }

    private static void verifyBufferedInputIsDiscardedByShutdownRead() throws Exception {
        Object owner = newOwner();
        java.nio.channels.ServerSocketChannel listener = java.nio.channels.ServerSocketChannel.open();
        java.nio.channels.SocketChannel client = java.nio.channels.SocketChannel.open();
        java.nio.channels.SocketChannel peer = null;
        java.nio.channels.Selector readable = null;
        selectOwner(owner);
        try {
            listener.configureBlocking(false);
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            client.configureBlocking(false);
            client.connect((java.net.InetSocketAddress) listener.getLocalAddress());
            peer = awaitConnectedPeer(listener, client);
            Object stream = newStream(client, 7200);
            register(owner, stream);

            java.nio.ByteBuffer input = java.nio.ByteBuffer.wrap(new byte[] { 1, 2, 3 });
            long writeDeadline = System.nanoTime()
                + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
            while (input.hasRemaining()) {
                peer.write(input);
                if (input.hasRemaining() && System.nanoTime() >= writeDeadline) {
                    throw new java.net.SocketTimeoutException("buffering loopback input timed out");
                }
                if (input.hasRemaining()) Thread.yield();
            }

            readable = java.nio.channels.Selector.open();
            client.register(readable, java.nio.channels.SelectionKey.OP_READ);
            if (readable.select(java.util.concurrent.TimeUnit.SECONDS.toMillis(5L)) == 0) {
                throw new java.net.SocketTimeoutException("buffered loopback input was not readable");
            }

            Object shutdown = VelnRuntime.netSystemShutdownRead(stream);
            if (!((VelnRuntime.Result) shutdown).isOk()) {
                throw new AssertionError("real shutdown_read failed: " + shutdown);
            }
            requireClosedFailure(
                VelnRuntime.netSystemRead(stream, VelnRuntime.none(), VelnRuntime.none()),
                "fresh read after shutdown_read"
            );
            if (client.read(java.nio.ByteBuffer.allocate(8)) != -1) {
                throw new AssertionError("shutdown_read did not discard buffered loopback input");
            }
        } finally {
            if (readable != null) readable.close();
            CLEANUP.invoke(null, owner);
            clearOwner();
            if (peer != null) peer.close();
            client.close();
            listener.close();
        }
    }

    private static void verifySuccessfulCloseClosesHostResources() throws Exception {
        Object owner = newOwner();
        java.nio.channels.ServerSocketChannel listener = java.nio.channels.ServerSocketChannel.open();
        java.nio.channels.SocketChannel streamSocket = java.nio.channels.SocketChannel.open();
        selectOwner(owner);
        try {
            listener.configureBlocking(false);
            listener.bind(new java.net.InetSocketAddress(
                java.net.InetAddress.getByName("127.0.0.1"),
                0
            ));
            String local = listener.getLocalAddress().toString();
            Object listenerValue = LISTENER_CONSTRUCTOR.newInstance(local, local, listener, null);
            registerListener(owner, listenerValue);
            Object listenerClose = VelnRuntime.netSystemCloseListener(listenerValue);
            if (!((VelnRuntime.Result) listenerClose).isOk() || listener.isOpen()) {
                throw new AssertionError("successful listener close left the host channel open");
            }

            Object stream = newStream(streamSocket, 7300);
            register(owner, stream);
            Object streamClose = VelnRuntime.netSystemCloseStream(stream);
            if (!((VelnRuntime.Result) streamClose).isOk() || streamSocket.isOpen()) {
                throw new AssertionError("successful stream close left the host channel open");
            }
        } finally {
            CLEANUP.invoke(null, owner);
            clearOwner();
            streamSocket.close();
            listener.close();
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
                Object[] overloaded = new Object[] {
                    VelnRuntime.netSystemResolve(address),
                    VelnRuntime.netSystemListen(address),
                    VelnRuntime.netSystemConnect(
                        address,
                        VelnRuntime.none(),
                        VelnRuntime.none()
                    )
                };
                String[] operations = new String[] { "resolve", "listen", "connect" };
                for (int index = 0; index < overloaded.length; index += 1) {
                    VelnRuntime.Result result = (VelnRuntime.Result) overloaded[index];
                    if (result.isOk() || !result.toString().contains("Busy")) {
                        throw new AssertionError(
                            operations[index] + " resolver overload did not return typed Busy"
                        );
                    }
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

    @SuppressWarnings("unchecked")
    private static void verifyResolverWaitObservesTaskCancellation() throws Exception {
        java.util.concurrent.ThreadPoolExecutor resolvers =
            (java.util.concurrent.ThreadPoolExecutor) RESOLVERS.get(null);
        for (String operation : new String[] { "resolve", "listen", "connect" }) {
            java.util.concurrent.CountDownLatch started = new java.util.concurrent.CountDownLatch(1);
            java.util.concurrent.CountDownLatch release = new java.util.concurrent.CountDownLatch(1);
            java.util.concurrent.Future<java.util.List<Object>> resolution = resolvers.submit(() -> {
                started.countDown();
                while (true) {
                    try {
                        release.await();
                        break;
                    } catch (InterruptedException ignored) {
                        // Model platform DNS work that ignores interruption.
                    }
                }
                return java.util.Collections.emptyList();
            });
            Object task = VelnRuntime.taskSpawn(new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    try {
                        return AWAIT_RESOLUTION.invoke(
                            null,
                            resolution,
                            null,
                            null,
                            operation
                        );
                    } catch (java.lang.reflect.InvocationTargetException failure) {
                        Throwable cause = failure.getCause();
                        if (cause instanceof RuntimeException) throw (RuntimeException) cause;
                        throw new RuntimeException(cause);
                    } catch (ReflectiveOperationException failure) {
                        throw new RuntimeException(failure);
                    }
                }
            });
            try {
                if (!started.await(5L, java.util.concurrent.TimeUnit.SECONDS)) {
                    throw new AssertionError(operation + " interruption-ignoring resolver did not start");
                }
                VelnRuntime.taskCancel(task);
                Object joined = VelnRuntime.taskJoin(task);
                if (!joined.toString().contains("cancelled")) {
                    throw new AssertionError(operation + " resolver wait ignored task cancellation: " + joined);
                }
                if (resolution.isDone() && !resolution.isCancelled()) {
                    throw new AssertionError(operation + " resolver unexpectedly completed before release");
                }
            } finally {
                release.countDown();
            }
        }
        long waitUntil = System.nanoTime() + java.util.concurrent.TimeUnit.SECONDS.toNanos(5L);
        while (resolvers.getActiveCount() != 0 && System.nanoTime() < waitUntil) Thread.yield();
        if (resolvers.getActiveCount() != 0 || !resolvers.getQueue().isEmpty()) {
            throw new AssertionError("cancelled resolver work exceeded the fixed active capacity");
        }
    }

    private static java.util.List<Object> twoResolvedEndpoints(Object network) throws Exception {
        java.util.List<Object> endpoints = new java.util.ArrayList<Object>();
        endpoints.add(ENDPOINT_CONSTRUCTOR.newInstance(network, "127.0.0.1"));
        endpoints.add(ENDPOINT_CONSTRUCTOR.newInstance(network, "127.0.0.2"));
        return endpoints;
    }

    private static Object connectTimeoutReply() {
        return VelnRuntime.record(
            "status", "error",
            "value", "",
            "local", "",
            "peer", "",
            "resolved_endpoints", VelnRuntime.listNil(),
            "bytes_committed", Long.valueOf(0L),
            "category", "timed_out",
            "phase", "during_operation",
            "input_committed", Long.valueOf(0L),
            "output_committed", Long.valueOf(0L),
            "ownership_committed", Long.valueOf(0L),
            "cause", "injected timeout"
        );
    }

    private static void requireTimedOut(Object result, String boundary) {
        if (((VelnRuntime.Result) result).isOk() || !result.toString().contains("TimedOut")) {
            throw new AssertionError(boundary + " did not return typed TimedOut: " + result);
        }
    }

    private static void verifyConnectTimeoutStopsEndpointFallback() throws Exception {
        Object network = VelnRuntime.adt("Network::Tcp4", new Object[0]);
        Object address = VelnRuntime.adt(
            "Address::Address",
            new Object[] { network, "fallback.test", Long.valueOf(443L) }
        );
        java.util.List<Object> endpoints = twoResolvedEndpoints(network);

        Object directOwner = newOwner();
        selectOwner(directOwner);
        try {
            long before = counter(CONNECT_ATTEMPTS);
            Object result = CONNECT_RESOLVED.invoke(
                null,
                directOwner,
                address,
                Long.valueOf(0L),
                null,
                endpoints
            );
            requireTimedOut(result, "direct host connect");
            if (counter(CONNECT_ATTEMPTS) - before != 1L) {
                throw new AssertionError("direct timeout attempted a second resolved endpoint");
            }
        } finally {
            CLEANUP.invoke(null, directOwner);
            clearOwner();
        }

        Object adapterOwner = newOwner();
        java.util.concurrent.atomic.AtomicInteger adapterAttempts =
            new java.util.concurrent.atomic.AtomicInteger();
        VelnRuntime.Fn provider = new VelnRuntime.Fn() {
            public Object call(Object... args) {
                adapterAttempts.incrementAndGet();
                return connectTimeoutReply();
            }
        };
        VelnRuntime.pushHandler(
            "std::host_effects::Network",
            new Object[] { "request" },
            new Object[] { provider },
            new Object[0]
        );
        selectOwner(adapterOwner);
        try {
            long before = counter(CONNECT_ATTEMPTS);
            Object result = CONNECT_RESOLVED.invoke(
                null,
                adapterOwner,
                address,
                null,
                null,
                endpoints
            );
            requireTimedOut(result, "typed adapter connect");
            if (counter(CONNECT_ATTEMPTS) - before != 1L || adapterAttempts.get() != 1) {
                throw new AssertionError("typed adapter timeout attempted a second resolved endpoint");
            }
        } finally {
            CLEANUP.invoke(null, adapterOwner);
            clearOwner();
            VelnRuntime.popHandler();
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
        verifyOwnedResourcesDoNotCrossHandlerDispatch();
        verifyTerminalStatesDoNotBypassContention();
        verifyDirectBindFailureClassification();
        verifyIdentityLedgerExplicitCloseAndDetach();
        verifyInheritedChildRegistrationIsCleaned();
        verifyAtomicResourcePublication();
        verifyBufferedInputIsDiscardedByShutdownRead();
        verifySuccessfulCloseClosesHostResources();
        verifyResolverCapacityAndHandlerDetachment();
        verifyResolverWaitObservesTaskCancellation();
        verifyConnectTimeoutStopsEndpointFallback();
        verifyUnexpectedFailuresRemainAbrupt();
        verifyIdleAcceptReusesReadinessState();
        verifyIdleReadReusesOperationState();
        System.out.println("network system lifecycle invariants held");
    }
}
