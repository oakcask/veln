    private static java.util.List<Object> processArgs =
        java.util.Collections.unmodifiableList(new java.util.ArrayList<Object>());

    private static TransportFailure transportFailure(
        String operation,
        String category,
        String localEndpoint,
        String peerEndpoint,
        String listenerId,
        String streamId,
        String phase,
        Boolean inputCommitted,
        Boolean outputCommitted,
        Boolean ownershipCommitted,
        String platformCause,
        Throwable cause
    ) {
        return new TransportFailure(
            operation,
            category,
            localEndpoint,
            peerEndpoint,
            listenerId,
            streamId,
            phase,
            inputCommitted,
            outputCommitted,
            ownershipCommitted,
            platformCause,
            cause
        );
    }

    private static TransportFailure listenerTransportFailure(
        String operation,
        String category,
        NetListener listener,
        String phase,
        boolean ownershipCommitted,
        String platformCause,
        Throwable cause
    ) {
        return transportFailure(
            operation,
            category,
            listener.localAddress,
            null,
            listener.address,
            null,
            phase,
            Boolean.FALSE,
            Boolean.FALSE,
            Boolean.valueOf(ownershipCommitted),
            platformCause,
            cause
        );
    }

    private static TransportFailure streamTransportFailure(
        String operation,
        String category,
        NetStream stream,
        String phase,
        Boolean inputCommitted,
        Boolean outputCommitted,
        boolean ownershipCommitted,
        String platformCause,
        Throwable cause
    ) {
        return transportFailure(
            operation,
            category,
            stream.localAddress,
            stream.peerAddress,
            null,
            stream.id,
            phase,
            inputCommitted,
            outputCommitted,
            Boolean.valueOf(ownershipCommitted),
            platformCause,
            cause
        );
    }

    private static TransportFailure acceptTransportFailure(
        String category,
        NetListener listener,
        NetStream stream,
        String phase,
        boolean ownershipCommitted,
        String platformCause,
        Throwable cause
    ) {
        return transportFailure(
            "accept",
            category,
            stream == null ? listener.localAddress : stream.localAddress,
            stream == null ? null : stream.peerAddress,
            listener.address,
            stream == null ? null : stream.id,
            phase,
            Boolean.FALSE,
            Boolean.FALSE,
            Boolean.valueOf(ownershipCommitted),
            platformCause,
            cause
        );
    }

    private static String transportFailureMessage(
        String operation,
        String category,
        String localEndpoint,
        String peerEndpoint,
        String listenerId,
        String streamId,
        String phase,
        Boolean inputCommitted,
        Boolean outputCommitted,
        Boolean ownershipCommitted,
        String platformCause
    ) {
        StringBuilder message = new StringBuilder(
            "transport " + operation.replace('_', ' ') + " failed: " + category
        );
        if (localEndpoint != null) {
            message.append("\nnote: Local endpoint: ").append(localEndpoint).append(".");
        }
        if (peerEndpoint != null) {
            message.append("\nnote: Peer endpoint: ").append(peerEndpoint).append(".");
        }
        if (listenerId != null) {
            message.append("\nnote: Listener identity: ").append(listenerId).append(".");
        }
        if (streamId != null) {
            message.append("\nnote: Stream identity: ").append(streamId).append(".");
        }
        message.append("\nnote: Lifecycle phase: ").append(phase).append(".");
        message.append("\nnote: Commit state: input=")
            .append(commitFact(inputCommitted))
            .append(", output=")
            .append(commitFact(outputCommitted))
            .append(", ownership=")
            .append(commitFact(ownershipCommitted))
            .append(".");
        if (platformCause != null) {
            message.append("\nnote: Platform cause: ").append(platformCause).append(".");
        }
        return message.toString();
    }

    private static String legacyTransportFailureMessage(String message, Throwable cause) {
        if (!message.startsWith("transport ") || message.indexOf('\n') >= 0) {
            return message;
        }
        int separator = message.indexOf(" failed: ");
        if (separator < 0) {
            return message;
        }
        String rawOperation = message.substring("transport ".length(), separator);
        String operation;
        if (rawOperation.equals("shutdown_write") || rawOperation.equals("shutdown_read")) {
            operation = "shutdown";
        } else if (rawOperation.equals("close")) {
            operation = "stream_close";
        } else if (rawOperation.equals("listener close")) {
            operation = "listener_close";
        } else if (
            rawOperation.equals("listen")
                || rawOperation.equals("connect")
                || rawOperation.equals("accept")
                || rawOperation.equals("read")
                || rawOperation.equals("write")
        ) {
            operation = rawOperation;
        } else {
            return message;
        }
        String platformCause = message.substring(separator + " failed: ".length());
        String category = legacyTransportCategory(platformCause);
        String causeText = cause != null && cause.getMessage() != null
            ? platformCause + ": " + cause.getMessage()
            : platformCause;
        return transportFailureMessage(
            operation,
            category,
            null,
            null,
            null,
            null,
            "during_operation",
            null,
            null,
            null,
            causeText
        );
    }

    private static String legacyTransportCategory(String cause) {
        if (cause.startsWith("could not record")) {
            return "event_record_failed";
        }
        if (cause.contains("timed out")) {
            return "timed_out";
        }
        if (cause.contains("closed") || cause.contains("shut down")) {
            return "closed";
        }
        if (cause.contains("ended")) {
            return "end_of_stream";
        }
        if (cause.startsWith("invalid socket address")) {
            return "invalid_endpoint";
        }
        if (cause.startsWith("malformed")) {
            return "invalid_input";
        }
        if (cause.contains("interrupted")) {
            return "interrupted";
        }
        return "io_failure";
    }

    private static String commitFact(Boolean value) {
        return value == null ? "unknown" : value.toString();
    }

    private static String transportIoCategory(Throwable cause) {
        if (cause instanceof java.net.UnknownHostException) return "name_not_found";
        if (cause instanceof java.net.BindException) return "address_in_use";
        if (cause instanceof java.net.ConnectException) return "connection_refused";
        if (cause instanceof java.net.SocketTimeoutException) return "timed_out";
        if (cause instanceof java.nio.channels.ClosedChannelException) return "closed";
        if (cause instanceof java.nio.file.AccessDeniedException || cause instanceof SecurityException) return "permission_denied";
        if (cause instanceof java.net.SocketException && cause.getMessage() != null) {
            String message = cause.getMessage().toLowerCase();
            if (message.contains("closed") || message.contains("shut down")) return "closed";
            if (message.contains("reset")) return "connection_reset";
        }
        return "io_failure";
    }

    private static String platformCause(String context, Throwable cause) {
        if (cause == null || cause.getMessage() == null || cause.getMessage().isEmpty()) {
            return context;
        }
        return context + ": " + cause.getMessage();
    }

    private static String traceText(String value) {
        return value == null ? "-" : hex(value);
    }

    private static void recordTransportFailure(
        String operation,
        String category,
        String localEndpoint,
        String peerEndpoint,
        String listenerId,
        String streamId,
        String phase,
        Boolean inputCommitted,
        Boolean outputCommitted,
        Boolean ownershipCommitted,
        String platformCause
    ) {
        String path = System.getenv("VELN_TRANSPORT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String line = "transport"
            + "\t" + traceText(operation)
            + "\t" + traceText(category)
            + "\t" + traceText(localEndpoint)
            + "\t" + traceText(peerEndpoint)
            + "\t" + traceText(listenerId)
            + "\t" + traceText(streamId)
            + "\t" + traceText(phase)
            + "\t" + commitFact(inputCommitted)
            + "\t" + commitFact(outputCommitted)
            + "\t" + commitFact(ownershipCommitted)
            + "\t" + traceText(platformCause)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ignored) {
            // The original transport failure remains authoritative.
        }
    }

    public static void setProcessArgs(String[] args) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>();
        for (String arg : args) {
            values.add(arg);
        }
        processArgs = freezeList(values);
    }

    private static final class CancelToken {
        private final boolean observerOnly;
        private volatile boolean cancelled;

        private CancelToken(boolean observerOnly) {
            this.observerOnly = observerOnly;
        }

        @Override
        public String toString() {
            return cancelled ? "CancelToken(cancelled)" : "CancelToken(active)";
        }
    }

    private static final class CancelOwner {
        private final CancelToken token = new CancelToken(true);

        @Override
        public String toString() {
            return token.cancelled ? "CancelOwner(cancelled)" : "CancelOwner(active)";
        }
    }

    public static Object fsReadToString(Object path) {
        try {
            byte[] bytes = java.nio.file.Files.readAllBytes(asPath(path));
            return ok(new String(bytes, java.nio.charset.StandardCharsets.UTF_8));
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsWriteString(Object path, Object text) {
        try {
            java.nio.file.Files.write(
                asPath(path),
                asString(text).getBytes(java.nio.charset.StandardCharsets.UTF_8)
            );
            return ok(UNIT);
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsExists(Object path) {
        try {
            return ok(Boolean.valueOf(java.nio.file.Files.exists(asPath(path))));
        } catch (RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsReadDir(Object path) {
        try (java.util.stream.Stream<java.nio.file.Path> stream =
            java.nio.file.Files.list(asPath(path))) {
            java.util.ArrayList<Object> paths = new java.util.ArrayList<Object>();
            stream.forEach(entry -> paths.add(pathValue(entry)));
            return ok(freezeList(paths));
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object streamAdapterDrainActions(Object stream, Object handler) {
        Object route = channelBounded(Long.valueOf(1));
        Object actions = listNil();
        while (true) {
            Object input = streamAdapterInputFromRead(netReadChunkOrEnd(stream));
            Object routed = streamAdapterRouteInput(route, input);
            Object handled = call(handler, routed);
            actions = streamAdapterAppend(actions, handled);
            if (isAdt(routed, "StreamInput::End")) {
                break;
            }
        }
        netWriteChunks(stream, streamAdapterSendChunks(actions));
        return actions;
    }

    public static Object streamAdapterAcceptLoop(Object listener, Object handler) {
        while (true) {
            Option accepted = asOption(netAcceptOrEnd(listener));
            if (!accepted.some) {
                netCloseListener(listener);
                return UNIT;
            }
            streamAdapterDrainActions(accepted.value, handler);
            netCloseStream(accepted.value);
        }
    }

    public static Object streamAdapterDrainActionsUntilCancellable(
        Object stream,
        Object handler,
        Object deadline,
        Object token
    ) {
        Object route = channelBounded(Long.valueOf(1));
        Object actions = listNil();
        while (true) {
            Object input = streamAdapterInputFromRead(netReadChunkOrEnd(stream));
            Object routed = streamAdapterRouteInput(route, input);
            Object handled = call(handler, routed);
            actions = streamAdapterAppend(actions, handled);
            if (isAdt(routed, "StreamInput::End")) {
                break;
            }
        }
        return netWriteChunksUntilCancellable(
            stream,
            streamAdapterSendChunks(actions),
            deadline,
            token
        );
    }

    private static Object streamAdapterInputFromRead(Object readResult) {
        Option read = asOption(readResult);
        if (read.some) {
            return adt("StreamInput::Chunk", new Object[] { read.value });
        }
        return adt("StreamInput::End", new Object[] {});
    }

    private static Object streamAdapterRouteInput(Object route, Object input) {
        channelSend(recordField(route, "tx"), input);
        Option received = asOption(channelRecv(recordField(route, "rx")));
        if (received.some) {
            return received.value;
        }
        return adt("StreamInput::End", new Object[] {});
    }

    private static Object streamAdapterAppend(Object left, Object right) {
        Object reversed = listNil();
        ListValue current = asListValue(left);
        while (current.cons) {
            reversed = listCons(current.head, reversed);
            current = asListValue(current.tail);
        }
        Object appended = right;
        current = asListValue(reversed);
        while (current.cons) {
            appended = listCons(current.head, appended);
            current = asListValue(current.tail);
        }
        return appended;
    }

    private static Object streamAdapterSendChunks(Object actions) {
        Object reversed = listNil();
        ListValue current = asListValue(actions);
        while (current.cons) {
            if (isAdt(current.head, "StreamAdapterAction::SendBytes")) {
                Adt send = (Adt) diagnosticResultValue(current.head);
                reversed = listCons(send.payloads[0], reversed);
            }
            current = asListValue(current.tail);
        }
        return listReverse(reversed);
    }

    public static Object netWriteChunksUntil(Object stream, Object chunks, Object deadline) {
        ListValue current = asListValue(chunks);
        while (current.cons) {
            Object outcome = netWriteChunkUntil(stream, current.head, deadline);
            if (!isAdt(outcome, "StreamWriteOutcome::WriteCompleted")) {
                return outcome;
            }
            current = asListValue(current.tail);
        }
        return streamWriteOutcome("WriteCompleted");
    }

    public static Object netWriteChunksUntilCancellable(
        Object stream,
        Object chunks,
        Object deadline,
        Object token
    ) {
        ListValue current = asListValue(chunks);
        while (current.cons) {
            Object outcome = netWriteChunkUntilCancellable(
                stream,
                current.head,
                deadline,
                token
            );
            if (!isAdt(outcome, "StreamWriteOutcome::WriteCompleted")) {
                return outcome;
            }
            current = asListValue(current.tail);
        }
        return streamWriteOutcome("WriteCompleted");
    }

    public static Object processArgs() {
        return processArgs;
    }

    public static Object processEnv(Object name) {
        String value = System.getenv(asString(name));
        return value == null ? none() : some(value);
    }

    public static Object processCwd() {
        try {
            return ok(pathValue(java.nio.file.Paths.get("").toAbsolutePath().normalize()));
        } catch (RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object processExit(Object status) {
        long code = asLong(status);
        if (code < 0L) {
            code = 0L;
        }
        if (code > 255L) {
            code = 255L;
        }
        System.exit((int) code);
        return UNIT;
    }

    private static Object waitOutcome(String name) {
        return adt(name, new Object[] {});
    }

    private static Object acceptOutcome(String name) {
        return acceptOutcome(name, new Object[] {});
    }

    private static Object acceptOutcome(String name, Object payload) {
        return acceptOutcome(name, new Object[] { payload });
    }

    private static Object acceptOutcome(String name, Object[] payloads) {
        return adt("AcceptOutcome::" + name, payloads);
    }

    private static Object streamReadOutcome(String name) {
        return streamReadOutcome(name, new Object[] {});
    }

    private static Object streamReadOutcome(String name, Object payload) {
        return streamReadOutcome(name, new Object[] { payload });
    }

    private static Object streamReadOutcome(String name, Object[] payloads) {
        return adt("StreamReadOutcome::" + name, payloads);
    }

    private static Object streamWriteOutcome(String name) {
        return streamWriteOutcome(name, new Object[] {});
    }

    private static Object streamWriteOutcome(String name, Object[] payloads) {
        return adt("StreamWriteOutcome::" + name, payloads);
    }

    private static CancelToken asCancelToken(Object value) {
        return (CancelToken) value;
    }

    private static CancelOwner asCancelOwner(Object value) {
        return (CancelOwner) value;
    }

    private static final String NETWORK_EFFECT = "std::host_effects::Network";
    private static final String CLOCK_EFFECT = "std::host_effects::Clock";
    private static final String TASK_JOIN_EFFECT = "std::host_effects::TaskJoin";

    private static final class NetListener {
        final String address;
        final String localAddress;
        final java.nio.channels.ServerSocketChannel server;
        final HandlerFrame adapter;
        HandlerFrame systemOwner;
        volatile boolean systemClosed;
        volatile boolean closeUncertain;
        final Object lifecycleLock = new Object();
        final java.util.concurrent.atomic.AtomicBoolean accepting = new java.util.concurrent.atomic.AtomicBoolean();
        NetListener(String address, String localAddress, java.nio.channels.ServerSocketChannel server, HandlerFrame adapter) {
            this.address = address;
            this.localAddress = localAddress;
            this.server = server;
            this.adapter = adapter;
        }
        public String toString() { return "NetListener(" + address + ")"; }
    }

    private static final class NetStream {
        final String id;
        final String localAddress;
        final String peerAddress;
        final java.nio.channels.SocketChannel socket;
        final HandlerFrame adapter;
        HandlerFrame systemOwner;
        volatile boolean readShutdown;
        volatile boolean writeShutdown;
        volatile boolean systemClosed;
        volatile boolean peerEnded;
        volatile boolean readFailed;
        volatile boolean writeFailed;
        volatile boolean readShutdownUncertain;
        volatile boolean writeShutdownUncertain;
        volatile boolean closeUncertain;
        final Object lifecycleLock = new Object();
        final java.util.concurrent.atomic.AtomicBoolean reading = new java.util.concurrent.atomic.AtomicBoolean();
        final java.util.concurrent.atomic.AtomicBoolean writing = new java.util.concurrent.atomic.AtomicBoolean();
        NetStream(String id, String local, String peer, java.nio.channels.SocketChannel socket, HandlerFrame adapter) {
            this.id = id;
            this.localAddress = local;
            this.peerAddress = peer;
            this.socket = socket;
            this.adapter = adapter;
        }
        public String toString() { return "NetStream(" + id + ")"; }
    }

    private static final class NetSystemResolvedEndpoint {
        final Object network;
        final String address;
        NetSystemResolvedEndpoint(Object network, String address) {
            this.network = network;
            this.address = address;
        }
    }
    private static final class NetworkProtocolFailure extends RuntimeException {
        NetworkProtocolFailure(String message) { super(message); }
    }
    private static final int NET_SYSTEM_RESOLVER_WORKERS = 4;
    private static final java.util.concurrent.ThreadPoolExecutor NET_SYSTEM_RESOLVERS =
        new java.util.concurrent.ThreadPoolExecutor(
            NET_SYSTEM_RESOLVER_WORKERS,
            NET_SYSTEM_RESOLVER_WORKERS,
            0L,
            java.util.concurrent.TimeUnit.MILLISECONDS,
            new java.util.concurrent.SynchronousQueue<Runnable>(),
            new java.util.concurrent.ThreadFactory() {
                private final java.util.concurrent.atomic.AtomicLong next =
                    new java.util.concurrent.atomic.AtomicLong();
                public Thread newThread(Runnable task) {
                    Runnable detached = () -> {
                        HANDLERS.remove();
                        INVOKED_HANDLER.remove();
                        try { task.run(); }
                        finally {
                            HANDLERS.remove();
                            INVOKED_HANDLER.remove();
                        }
                    };
                    Thread worker = new Thread(
                        detached,
                        "veln-network-resolver-" + next.incrementAndGet()
                    );
                    worker.setDaemon(true);
                    return worker;
                }
            },
            new java.util.concurrent.ThreadPoolExecutor.AbortPolicy()
        );

    private static NetListener asNetListener(Object value) { return (NetListener) value; }
    private static NetStream asNetStream(Object value) { return (NetStream) value; }
    private static Object emptyNetworkBytes() { return byteChunkObject(java.util.Collections.emptyList()); }
    private static String replyText(Object reply, String key) { return asString(recordField(reply, key)); }
    private static Boolean replyCommit(Object reply, String key) {
        long value = asLong(recordField(reply, key));
        return value < 0L ? null : Boolean.valueOf(value != 0L);
    }
    private static String optionalEndpoint(String value) { return value.isEmpty() ? null : value; }
    private static Object rawNetworkRequest(HandlerFrame adapter, String operation, String subject, Object bytes, Object deadline, Object token) {
        long remaining = deadline == null ? -1L : deadlineRemainingNanos(deadline) / 1000000L;
        return invokeHandler(adapter, "request", new Object[] {
            operation, subject, bytes, Long.valueOf(remaining),
            Boolean.valueOf(token != null && asCancelToken(token).cancelled)
        });
    }
    private static RuntimeFailure networkReplyFailure(Object reply, String operation, String subject) {
        if (operation.equals("receive") || operation.equals("send"))
            return new RuntimeFailure("transport " + operation + " failed: " + replyText(reply, "cause"));
        if (operation.endsWith("_addr"))
            return new RuntimeFailure("transport address failed: " + replyText(reply, "cause"));
        String kind = operation;
        if (kind.startsWith("read")) kind = "read";
        else if (kind.startsWith("write")) kind = "write";
        else if (kind.startsWith("accept")) kind = "accept";
        else if (kind.startsWith("resolve")) kind = "resolve";
        else if (kind.startsWith("shutdown")) kind = "shutdown";
        else if (kind.equals("close_stream")) kind = "stream_close";
        else if (kind.equals("close_listener")) kind = "listener_close";
        return transportFailure(kind, replyText(reply, "category"),
            optionalEndpoint(replyText(reply, "local")), kind.equals("listener_close") ? null : optionalEndpoint(replyText(reply, "peer")),
            kind.equals("listen") ? optionalEndpoint(replyText(reply, "value")) : kind.equals("accept") || kind.equals("listener_close") ? subject : null,
            kind.equals("listen") || kind.equals("listener_close") ? null : kind.equals("connect") || kind.equals("accept") ? optionalEndpoint(replyText(reply, "value")) : optionalEndpoint(subject),
            replyText(reply, "phase"), replyCommit(reply, "input_committed"),
            replyCommit(reply, "output_committed"), replyCommit(reply, "ownership_committed"),
            optionalEndpoint(replyText(reply, "cause")), null);
    }
    private static Object networkRequest(HandlerFrame adapter, String operation, String subject, Object bytes, Object deadline, Object token) {
        Object reply = rawNetworkRequest(adapter, operation, subject, bytes, deadline, token);
        if (replyText(reply, "status").equals("error")) {
            throw networkReplyFailure(reply, operation, subject);
        }
        return reply;
    }
    private static Object listenerRequest(NetListener listener, String operation, Object deadline, Object token) {
        return networkRequest(listener.adapter, operation, listener.address, emptyNetworkBytes(), deadline, token);
    }
    private static Object streamRequest(NetStream stream, String operation, Object bytes, Object deadline, Object token) {
        return networkRequest(stream.adapter, operation, stream.id, bytes, deadline, token);
    }
    private static NetStream adapterStream(HandlerFrame adapter, Object reply) {
        return new NetStream(replyText(reply, "value"), replyText(reply, "local"), replyText(reply, "peer"), null, adapter);
    }
    private static java.net.InetSocketAddress parseSocketAddress(String address) {
        int separator = address.lastIndexOf(':');
        if (separator <= 0 || separator == address.length() - 1) throw new IllegalArgumentException("missing host or port");
        int port = Integer.parseInt(address.substring(separator + 1));
        if (port < 0 || port > 65535) throw new IllegalArgumentException("port out of range");
        String host = address.substring(0, separator);
        if (host.startsWith("[") && host.endsWith("]")) host = host.substring(1, host.length() - 1);
        else if (host.indexOf(':') >= 0) throw new IllegalArgumentException("IPv6 host must be bracketed");
        if (host.isEmpty()) throw new IllegalArgumentException("missing host or port");
        return new java.net.InetSocketAddress(host, port);
    }
    private static String formatSocketAddress(java.net.SocketAddress value) {
        java.net.InetSocketAddress address = (java.net.InetSocketAddress) value;
        return netSystemJoinHostPort(address.getAddress().getHostAddress(), address.getPort());
    }
    private static final java.util.concurrent.atomic.AtomicLong SOCKET_READINESS_REGISTRATIONS =
        new java.util.concurrent.atomic.AtomicLong();
    private static final java.util.concurrent.atomic.AtomicLong NET_READ_BUFFER_ALLOCATIONS =
        new java.util.concurrent.atomic.AtomicLong();
    private static final class SocketReadiness implements AutoCloseable {
        private final java.nio.channels.Selector selector;

        private SocketReadiness(java.nio.channels.SelectableChannel channel, int operation) throws java.io.IOException {
            selector = java.nio.channels.Selector.open();
            try {
                channel.register(selector, operation);
                SOCKET_READINESS_REGISTRATIONS.incrementAndGet();
            } catch (Throwable failure) {
                try { selector.close(); } catch (java.io.IOException ignored) { }
                throw failure;
            }
        }

        private void await(long timeoutMillis) throws java.io.IOException {
            selector.selectedKeys().clear();
            if (timeoutMillis > 0L) selector.select(timeoutMillis);
            else selector.select();
            throwTaskCancellationIfRequested();
        }

        @Override
        public void close() throws java.io.IOException {
            selector.close();
        }
    }
    private static void awaitSocketReady(java.nio.channels.SelectableChannel channel, int operation, long timeoutMillis) throws java.io.IOException {
        try (SocketReadiness readiness = new SocketReadiness(channel, operation)) {
            readiness.await(timeoutMillis);
        }
    }

    private static final class StdinPump {
        private static final java.util.concurrent.BlockingQueue<byte[]> chunks =
            new java.util.concurrent.ArrayBlockingQueue<byte[]>(1);
        private static volatile boolean started;
        private static volatile boolean ended;
        private static volatile java.io.IOException failure;

        private static synchronized void start() {
            if (started) return;
            started = true;
            Thread reader = new Thread(() -> {
                try {
                    while (true) {
                        byte[] bytes = new byte[4096];
                        int count = System.in.read(bytes);
                        if (count < 0) { ended = true; return; }
                        chunks.put(java.util.Arrays.copyOf(bytes, count));
                    }
                } catch (java.io.IOException error) {
                    failure = error;
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                }
            }, "veln-stdin-pump");
            reader.setDaemon(true);
            reader.start();
        }

        private static byte[] receive() throws java.io.IOException {
            start();
            while (true) {
                throwTaskCancellationIfRequested();
                try {
                    byte[] bytes = chunks.poll(10L, java.util.concurrent.TimeUnit.MILLISECONDS);
                    if (bytes != null) return bytes;
                } catch (InterruptedException interrupted) {
                    throwTaskCancellationIfRequested();
                    Thread.currentThread().interrupt();
                    throw new java.io.IOException("standard input wait interrupted", interrupted);
                }
                if (failure != null) throw failure;
                if (ended && chunks.isEmpty()) return new byte[0];
            }
        }
    }
    private static byte[] byteChunkToByteArray(Object chunk) {
        java.util.List<Object> values = byteChunkBytes(chunk);
        byte[] bytes = new byte[values.size()];
        for (int index = 0; index < values.size(); index++) bytes[index] = (byte) byteValueLong(values.get(index));
        return bytes;
    }
    private static Object byteChunkFromByteArray(byte[] bytes, int count) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<>();
        for (int index = 0; index < count; index++) values.add(byteObject(bytes[index] & 255));
        return byteChunkObject(values);
    }
    private static TransportFailure hostStreamFailure(String operation, NetStream stream, Exception error) {
        return streamTransportFailure(operation, transportIoCategory(error), stream, "during_operation",
            operation.equals("read") ? null : Boolean.FALSE,
            operation.equals("write") ? null : Boolean.FALSE, false,
            platformCause("production socket failed", error), error);
    }
    public static Object netReceiveChunk() {
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) return recordField(networkRequest(adapter, "receive", "", emptyNetworkBytes(), null, null), "bytes");
        try {
            byte[] bytes = StdinPump.receive();
            return byteChunkFromByteArray(bytes, bytes.length);
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw new RuntimeFailure("transport receive failed: " + error.getMessage(), error);
        }
    }
    public static Object netSendChunk(Object bytes) {
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) networkRequest(adapter, "send", "", bytes, null, null);
        else { try { System.out.write(byteChunkToByteArray(bytes)); System.out.flush(); } catch (java.io.IOException error) { throw new RuntimeFailure("transport send failed: " + error.getMessage(), error); } }
        return UNIT;
    }
    public static Object netListen(Object address) {
        String text = asString(address);
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) {
            Object reply = networkRequest(adapter, "listen", text, emptyNetworkBytes(), null, null);
            return new NetListener(replyText(reply, "value"), replyText(reply, "local"), null, adapter);
        }
        java.nio.channels.ServerSocketChannel server = null;
        try {
            java.net.InetSocketAddress endpoint = parseSocketAddress(text);
            server = java.nio.channels.ServerSocketChannel.open();
            server.configureBlocking(false);
            server.bind(endpoint);
            return new NetListener(text, formatSocketAddress(server.getLocalAddress()), server, null);
        } catch (java.io.IOException | RuntimeException error) {
            if (server != null) try { server.close(); } catch (java.io.IOException ignored) { }
            throw transportFailure("listen", error instanceof IllegalArgumentException ? "invalid_endpoint" : transportIoCategory(error), text, null, null, null, error instanceof IllegalArgumentException ? "before_operation" : "during_operation", false, false, false, platformCause("production socket failed", error), error);
        }
    }
    public static Object netConnect(Object address) {
        String text = asString(address); HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) return adapterStream(adapter, networkRequest(adapter, "connect", text, emptyNetworkBytes(), null, null));
        java.nio.channels.SocketChannel socket = null;
        try {
            throwTaskCancellationIfRequested();
            socket = java.nio.channels.SocketChannel.open();
            socket.configureBlocking(false);
            socket.connect(parseSocketAddress(text));
            long connectDeadline = System.nanoTime() + 5000000000L;
            while (!socket.finishConnect()) {
                long remainingMillis = Math.max(1L, (connectDeadline - System.nanoTime()) / 1000000L);
                if (System.nanoTime() >= connectDeadline) throw new java.net.SocketTimeoutException("connect timed out");
                awaitSocketReady(socket, java.nio.channels.SelectionKey.OP_CONNECT, remainingMillis);
            }
            return new NetStream(formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getRemoteAddress()), socket, null);
        } catch (java.io.IOException | RuntimeException error) {
            if (socket != null) try { socket.close(); } catch (java.io.IOException ignored) { }
            if (error instanceof RuntimeFailure) throw (RuntimeFailure) error;
            throwTaskCancellationIfRequested();
            throw transportFailure("connect", error instanceof IllegalArgumentException ? "invalid_endpoint" : transportIoCategory(error), null, text, null, null, "during_operation", false, false, false, platformCause("production socket failed", error), error);
        }
    }
    private static Object acceptHost(NetListener listener, Object deadline, Object token, String operation) {
        if (listener.adapter != null) {
            Object reply = listenerRequest(listener, operation, deadline, token);
            String status = replyText(reply, "status");
            if (status.equals("end")) return acceptOutcome("AcceptEnd");
            if (status.equals("deadline")) return acceptOutcome("AcceptDeadlineExpired");
            if (status.equals("cancelled")) return acceptOutcome("AcceptCancelled");
            return acceptOutcome("AcceptStream", adapterStream(listener.adapter, reply));
        }
        SocketReadiness readiness = null;
        try {
            throwTaskCancellationIfRequested();
            if (!listener.server.isOpen()) throw listenerTransportFailure(
                "accept", "closed", listener, "closed_handle", false, "listener closed", null);
            while (true) {
                if (token != null && waitCancelled(asCancelToken(token))) return acceptOutcome("AcceptCancelled");
                long remaining = deadline == null ? Long.MAX_VALUE : deadlineRemainingNanos(deadline);
                if (remaining == 0L) return acceptOutcome("AcceptDeadlineExpired");
                java.nio.channels.SocketChannel socket = listener.server.accept();
                if (socket == null) {
                    if (readiness == null) readiness = new SocketReadiness(
                        listener.server, java.nio.channels.SelectionKey.OP_ACCEPT);
                    readiness.await(listener.systemOwner != null || deadline != null || token != null ? 10L : 0L);
                    continue;
                }
                socket.configureBlocking(false);
                NetStream stream = new NetStream(formatSocketAddress(socket.getRemoteAddress()), formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getRemoteAddress()), socket, null);
                return acceptOutcome("AcceptStream", stream);
            }
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw listenerTransportFailure("accept", transportIoCategory(error), listener, "during_operation", false, platformCause("production socket failed", error), error);
        } finally {
            if (readiness != null) try { readiness.close(); } catch (java.io.IOException ignored) { }
        }
    }
    private static Object acceptedOption(Object outcome) { return isAdt(outcome, "AcceptOutcome::AcceptStream") ? some(((Adt)outcome).payloads[0]) : none(); }
    public static Object netAccept(Object listener) {
        Object outcome = acceptHost(asNetListener(listener), null, null, "accept");
        if (isAdt(outcome, "AcceptOutcome::AcceptStream")) return ((Adt)outcome).payloads[0];
        throw acceptTransportFailure("end_of_stream", asNetListener(listener), null, "clean_end", false, "listener ended", null);
    }
    public static Object netAcceptOrEnd(Object listener) { return acceptedOption(acceptHost(asNetListener(listener), null, null, "accept_or_end")); }
    public static Object netAcceptUntil(Object listener, Object deadline) { return acceptedOption(acceptHost(asNetListener(listener), deadline, null, "accept_until")); }
    public static Object netAcceptUntilCancellable(Object listener, Object deadline, Object token) { return acceptHost(asNetListener(listener), deadline, token, "accept_until_cancellable"); }
    public static Object netListenerLocalAddr(Object value) {
        NetListener listener = asNetListener(value);
        return listener.adapter == null ? listener.localAddress : replyText(listenerRequest(listener, "listener_local_addr", null, null), "value");
    }
    private static Object streamProperty(Object value, String operation) {
        NetStream stream = asNetStream(value);
        if (stream.adapter != null) return replyText(streamRequest(stream, operation, emptyNetworkBytes(), null, null), "value");
        switch (operation) {
            case "stream_local_addr": return stream.localAddress;
            case "stream_peer_addr": return stream.peerAddress;
            case "stream_can_read": return Boolean.toString(stream.socket.isOpen() && !stream.readShutdown);
            case "stream_can_write": return Boolean.toString(stream.socket.isOpen() && !stream.writeShutdown);
            default: return Boolean.toString(!stream.socket.isOpen());
        }
    }
    public static Object netStreamLocalAddr(Object stream) { return streamProperty(stream, "stream_local_addr"); }
    public static Object netStreamPeerAddr(Object stream) { return streamProperty(stream, "stream_peer_addr"); }
    public static Object netStreamCanRead(Object stream) { return Boolean.valueOf(asString(streamProperty(stream, "stream_can_read"))); }
    public static Object netStreamCanWrite(Object stream) { return Boolean.valueOf(asString(streamProperty(stream, "stream_can_write"))); }
    public static Object netStreamIsClosed(Object stream) { return Boolean.valueOf(asString(streamProperty(stream, "stream_is_closed"))); }
    private static Object readHost(NetStream stream, Object deadline, Object token, String operation) {
        if (stream.adapter != null) {
            Object reply = streamRequest(stream, operation, emptyNetworkBytes(), deadline, token);
            String status = replyText(reply, "status");
            if (status.equals("end")) return streamReadOutcome("ReadEnd");
            if (status.equals("deadline")) return streamReadOutcome("ReadDeadlineExpired");
            if (status.equals("cancelled")) return streamReadOutcome("ReadCancelled");
            if (!status.equals("ok")) throw new NetworkProtocolFailure("network handler returned an invalid read status");
            Object bytes = recordField(reply, "bytes");
            if (byteChunkToByteArray(bytes).length == 0) throw new NetworkProtocolFailure("network handler returned an empty successful read");
            return streamReadOutcome("ReadChunk", bytes);
        }
        SocketReadiness readiness = null;
        try {
            throwTaskCancellationIfRequested();
            NET_READ_BUFFER_ALLOCATIONS.incrementAndGet();
            java.nio.ByteBuffer bytes = java.nio.ByteBuffer.allocate(4096);
            while (true) {
                failIfHostStreamClosed(stream, "read");
                if (stream.readShutdown) return streamReadOutcome("ReadEnd");
                if (token != null && waitCancelled(asCancelToken(token))) return streamReadOutcome("ReadCancelled");
                long remaining = deadline == null ? Long.MAX_VALUE : deadlineRemainingNanos(deadline);
                if (remaining == 0L) return streamReadOutcome("ReadDeadlineExpired");
                bytes.clear();
                int count = stream.socket.read(bytes);
                if (count < 0) return streamReadOutcome("ReadEnd");
                if (count == 0) {
                    if (readiness == null) readiness = new SocketReadiness(
                        stream.socket, java.nio.channels.SelectionKey.OP_READ);
                    readiness.await(stream.systemOwner != null || deadline != null || token != null ? 10L : 0L);
                    continue;
                }
                return streamReadOutcome("ReadChunk", byteChunkFromByteArray(bytes.array(), count));
            }
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw hostStreamFailure("read", stream, error);
        } finally {
            if (readiness != null) try { readiness.close(); } catch (java.io.IOException ignored) { }
        }
    }
    private static Object readOption(Object outcome) { return isAdt(outcome, "StreamReadOutcome::ReadChunk") ? some(((Adt)outcome).payloads[0]) : none(); }
    public static Object netReadChunk(Object stream) {
        Object outcome = readHost(asNetStream(stream), null, null, "read_chunk");
        if (isAdt(outcome, "StreamReadOutcome::ReadChunk")) return ((Adt)outcome).payloads[0];
        throw streamTransportFailure("read", "end_of_stream", asNetStream(stream), "clean_end", true, false, false, "stream ended", null);
    }
    public static Object netReadChunkOrEnd(Object stream) { return readOption(readHost(asNetStream(stream), null, null, "read_chunk_or_end")); }
    public static Object netReadChunkUntil(Object stream, Object deadline) { return readOption(readHost(asNetStream(stream), deadline, null, "read_chunk_until")); }
    public static Object netReadChunkUntilCancellable(Object stream, Object deadline, Object token) { return readHost(asNetStream(stream), deadline, token, "read_chunk_until_cancellable"); }
    private static Object writeHost(NetStream stream, Object bytes, Object deadline, Object token, String operation) {
        if (stream.adapter != null) {
            String status = replyText(streamRequest(stream, operation, bytes, deadline, token), "status");
            return streamWriteOutcome(status.equals("deadline") ? "WriteDeadlineExpired" : status.equals("cancelled") ? "WriteCancelled" : "WriteCompleted");
        }
        SocketReadiness readiness = null;
        try {
            throwTaskCancellationIfRequested();
            failIfHostStreamClosed(stream, "write");
            if (stream.writeShutdown) throw streamTransportFailure("write", "closed", stream,
                "write_shutdown", false, false, true, "stream write side is shut down", null);
            if (token != null && waitCancelled(asCancelToken(token))) return streamWriteOutcome("WriteCancelled");
            if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return streamWriteOutcome("WriteDeadlineExpired");
            java.nio.ByteBuffer output = java.nio.ByteBuffer.wrap(byteChunkToByteArray(bytes));
            while (output.hasRemaining()) {
                int written = stream.socket.write(output);
                if (!output.hasRemaining()) break;
                if (token != null && waitCancelled(asCancelToken(token))) return streamWriteOutcome("WriteCancelled");
                if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return streamWriteOutcome("WriteDeadlineExpired");
                if (written == 0) {
                    if (readiness == null) readiness = new SocketReadiness(
                        stream.socket, java.nio.channels.SelectionKey.OP_WRITE);
                    readiness.await(deadline != null || token != null ? 10L : 0L);
                    if (token != null && waitCancelled(asCancelToken(token))) return streamWriteOutcome("WriteCancelled");
                    if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return streamWriteOutcome("WriteDeadlineExpired");
                }
            }
            return streamWriteOutcome("WriteCompleted");
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw hostStreamFailure("write", stream, error);
        } finally {
            if (readiness != null) try { readiness.close(); } catch (java.io.IOException ignored) { }
        }
    }
    public static Object netWriteChunk(Object stream, Object bytes) { writeHost(asNetStream(stream), bytes, null, null, "write_chunk"); return UNIT; }
    public static Object netWriteChunkUntil(Object stream, Object bytes, Object deadline) { return writeHost(asNetStream(stream), bytes, deadline, null, "write_chunk_until"); }
    public static Object netWriteChunkUntilCancellable(Object stream, Object bytes, Object deadline, Object token) { return writeHost(asNetStream(stream), bytes, deadline, token, "write_chunk_until_cancellable"); }
    public static Object netWriteChunks(Object stream, Object chunks) {
        ListValue current = asListValue(chunks);
        while (current.cons) { netWriteChunk(stream, current.head); current = asListValue(current.tail); }
        return UNIT;
    }
    private static void failIfHostStreamClosed(NetStream stream, String operation) {
        if (!stream.socket.isOpen()) throw streamTransportFailure(operation, "closed", stream,
            "closed_handle", false, false, false, "stream closed", null);
    }
    private static Object streamLifecycle(Object value, String operation) {
        NetStream stream = asNetStream(value);
        if (stream.adapter != null) { streamRequest(stream, operation, emptyNetworkBytes(), null, null); return UNIT; }
        try {
            if (operation.equals("close_stream")) stream.socket.close();
            else if (operation.equals("shutdown_read")) {
                failIfHostStreamClosed(stream, "shutdown");
                if (!stream.socket.socket().isInputShutdown()) stream.socket.socket().shutdownInput();
            } else {
                failIfHostStreamClosed(stream, "shutdown");
                if (!stream.socket.socket().isOutputShutdown()) stream.socket.socket().shutdownOutput();
            }
            return UNIT;
        } catch (java.io.IOException error) {
            Boolean inputCommitted = operation.equals("shutdown_read")
                ? Boolean.valueOf(stream.socket.socket().isInputShutdown()) : Boolean.FALSE;
            Boolean outputCommitted = operation.equals("shutdown_write")
                ? Boolean.valueOf(stream.socket.socket().isOutputShutdown()) : Boolean.FALSE;
            Boolean ownershipCommitted = operation.equals("close_stream")
                ? Boolean.valueOf(!stream.socket.isOpen()) : Boolean.FALSE;
            throw streamTransportFailure(operation.equals("close_stream") ? "stream_close" : "shutdown",
                transportIoCategory(error), stream, operation.equals("close_stream") ? "closed_handle" : operation.equals("shutdown_read") ? "read_shutdown" : "write_shutdown",
                inputCommitted, outputCommitted, ownershipCommitted,
                platformCause("production socket failed", error), error);
        }
    }
    public static Object netShutdownWrite(Object stream) { return streamLifecycle(stream, "shutdown_write"); }
    public static Object netShutdownRead(Object stream) { return streamLifecycle(stream, "shutdown_read"); }
    public static Object netCloseStream(Object stream) { return streamLifecycle(stream, "close_stream"); }
    public static Object netCloseListener(Object value) {
        NetListener listener = asNetListener(value);
        if (listener.adapter != null) { listenerRequest(listener, "close_listener", null, null); return UNIT; }
        try { listener.server.close(); return UNIT; }
        catch (java.io.IOException error) { throw listenerTransportFailure(
            "listener_close", transportIoCategory(error), listener, "during_operation",
            !listener.server.isOpen(), error.getMessage(), error); }
    }

    private static Object netSystemNetwork(String name) { return adt("Network::" + name, new Object[] {}); }
    private static Object netSystemEndpoint(Object network, String address) {
        return adt("Endpoint::Endpoint", new Object[] { network, address });
    }
    private static Object netSystemError(String operation, Object network, String address, String kind, String message) {
        return adt("NetError::NetError", new Object[] {
            operation,
            network == null ? none() : some(network),
            address == null ? none() : some(address),
            adt("NetErrorKind::" + kind, new Object[] {}),
            message == null || message.isEmpty() ? kind : message
        });
    }
    private static Object netSystemFailure(String operation, Object network, String address, Throwable failure) {
        if (failure instanceof TaskCancellationFailure) throw (TaskCancellationFailure) failure;
        if (failure instanceof TransportFailure) {
            String category = ((TransportFailure) failure).category;
            String kind = category.equals("timed_out") ? "TimedOut"
                : category.equals("cancelled") ? "Cancelled"
                : category.equals("closed") ? "Closed"
                : category.equals("connection_refused") ? "ConnectionRefused"
                : category.equals("connection_reset") ? "ConnectionReset"
                : category.equals("address_in_use") ? "AddressInUse"
                : category.equals("permission_denied") ? "PermissionDenied"
                : category.equals("name_not_found") ? "NameNotFound"
                : category.equals("invalid_endpoint") || category.equals("invalid_input") ? "InvalidAddress"
                : null;
            return netSystemError(operation, network, address, kind == null ? "Other" : kind, failure.getMessage());
        }
        if (failure instanceof RuntimeFailure) throw (RuntimeFailure) failure;
        if (failure instanceof Error) throw (Error) failure;
        Throwable cause = failure;
        String kind = "Other";
        if (cause instanceof java.net.UnknownHostException) kind = "NameNotFound";
        else if (cause instanceof java.net.BindException) kind = "AddressInUse";
        else if (cause instanceof java.net.ConnectException) kind = "ConnectionRefused";
        else if (cause instanceof java.net.SocketTimeoutException) kind = "TimedOut";
        else if (cause instanceof java.nio.channels.ClosedChannelException || cause instanceof java.net.SocketException && cause.getMessage() != null && cause.getMessage().toLowerCase().contains("closed")) kind = "Closed";
        else if (cause instanceof java.net.SocketException && cause.getMessage() != null && cause.getMessage().toLowerCase().contains("reset")) kind = "ConnectionReset";
        else if (cause instanceof java.nio.file.AccessDeniedException || cause instanceof SecurityException) kind = "PermissionDenied";
        else if (cause instanceof java.util.concurrent.RejectedExecutionException) kind = "Busy";
        else if (cause instanceof NetworkProtocolFailure) kind = "Other";
        else if (!(cause instanceof java.io.IOException)
            && !(cause instanceof java.util.concurrent.CancellationException)
        ) {
            if (cause instanceof RuntimeException) throw (RuntimeException) cause;
            throw new RuntimeFailure("unexpected network host failure", cause);
        }
        return netSystemError(operation, network, address, kind, cause.getMessage());
    }
    private static void netSystemRejectUnexpectedFailure(Throwable failure) {
        netSystemFailure("host", null, null, failure);
    }
    private static Boolean netSystemFailureCommit(Throwable failure, boolean input, boolean output) {
        if (!(failure instanceof TransportFailure)) return null;
        TransportFailure transport = (TransportFailure) failure;
        return input ? transport.inputCommitted
            : output ? transport.outputCommitted
            : transport.ownershipCommitted;
    }
    private static Object netSystemInvalidResource(String operation) {
        return netSystemError(operation, null, null, "InvalidResource", "resource belongs to another or expired system handler");
    }
    private static boolean netSystemFailureKeepsResourceOpen(Object error) {
        Adt netError = (Adt) error;
        Object kind = netError.payloads[3];
        return isAdt(kind, "NetErrorKind::TimedOut") || isAdt(kind, "NetErrorKind::Cancelled");
    }
    private static HandlerFrame netSystemOwner() { return currentHandlerFrame(); }
    private static boolean netSystemOwns(NetListener listener, HandlerFrame owner) { return !owner.networkSystemClosed && listener.systemOwner == owner; }
    private static boolean netSystemOwns(NetStream stream, HandlerFrame owner) { return !owner.networkSystemClosed && stream.systemOwner == owner; }
    private static boolean netSystemListenerUnavailable(NetListener listener) {
        synchronized (listener.lifecycleLock) {
            return listener.systemClosed || listener.closeUncertain;
        }
    }
    private static boolean netSystemStreamUnavailable(NetStream stream) {
        synchronized (stream.lifecycleLock) {
            return stream.systemClosed || stream.closeUncertain;
        }
    }
    private static boolean netSystemReadUnavailable(NetStream stream) {
        synchronized (stream.lifecycleLock) {
            return stream.systemClosed || stream.closeUncertain
                || stream.readShutdown || stream.readShutdownUncertain;
        }
    }
    private static boolean netSystemWriteUnavailable(NetStream stream) {
        synchronized (stream.lifecycleLock) {
            return stream.systemClosed || stream.closeUncertain
                || stream.writeShutdown || stream.writeShutdownUncertain;
        }
    }
    private static boolean netSystemRegisterListener(HandlerFrame owner, NetListener listener) {
        synchronized (owner.networkResources) {
            if (owner.networkSystemClosed) return false;
            listener.systemOwner = owner;
            owner.networkResources.add(listener);
            return true;
        }
    }
    private static boolean netSystemRegisterStream(HandlerFrame owner, NetStream stream) {
        synchronized (owner.networkResources) {
            if (owner.networkSystemClosed) return false;
            stream.systemOwner = owner;
            owner.networkResources.add(stream);
            return true;
        }
    }
    private static void netSystemReleaseResource(HandlerFrame owner, Object resource) {
        synchronized (owner.networkResources) {
            owner.networkResources.remove(resource);
        }
    }
    private static Object netSystemOptionValue(Object option) {
        Option value = asOption(option);
        return value.some ? value.value : null;
    }
    private static String netSystemJoinHostPort(String host, long port) {
        return (host.indexOf(':') >= 0 ? "[" + host + "]" : host) + ":" + port;
    }
    private static Object netSystemAddressNetwork(Object address) { return ((Adt) address).payloads[0]; }
    private static String netSystemAddressHost(Object address) { return asString(((Adt) address).payloads[1]); }
    private static long netSystemAddressPort(Object address) { return asLong(((Adt) address).payloads[2]); }
    private static String netSystemNetworkName(Object network) {
        if (isAdt(network, "Network::Tcp4")) return "tcp4";
        if (isAdt(network, "Network::Tcp6")) return "tcp6";
        return "tcp";
    }
    private static Object netSystemNetworkFromName(String name) {
        if (name.equals("tcp4")) return netSystemNetwork("Tcp4");
        if (name.equals("tcp6")) return netSystemNetwork("Tcp6");
        throw new IllegalArgumentException("resolver returned an unsupported network");
    }
    private static java.util.List<NetSystemResolvedEndpoint> netSystemResolvedWithAdapter(
        Object network,
        String host,
        Object deadline,
        Object token,
        HandlerFrame adapter
    ) {
        String operation = "resolve_" + netSystemNetworkName(network);
        Object reply = networkRequest(adapter, operation, host, emptyNetworkBytes(), deadline, token);
        java.util.LinkedHashMap<String, NetSystemResolvedEndpoint> unique = new java.util.LinkedHashMap<>();
        ListValue current = asListValue(recordField(reply, "resolved_endpoints"));
        while (current.cons) {
            String resolvedNetwork = asString(recordField(current.head, "network"));
            String resolvedAddress = asString(recordField(current.head, "address"));
            Object endpointNetwork = netSystemNetworkFromName(resolvedNetwork);
            boolean matches = isAdt(network, "Network::Tcp")
                || isAdt(network, "Network::Tcp4") && isAdt(endpointNetwork, "Network::Tcp4")
                || isAdt(network, "Network::Tcp6") && isAdt(endpointNetwork, "Network::Tcp6");
            if (matches) unique.put(resolvedNetwork + "\u0000" + resolvedAddress,
                new NetSystemResolvedEndpoint(endpointNetwork, resolvedAddress));
            current = asListValue(current.tail);
        }
        return new java.util.ArrayList<>(unique.values());
    }
    private static java.util.List<NetSystemResolvedEndpoint> netSystemResolvedOnHost(
        Object network,
        String host
    ) throws java.net.UnknownHostException {
        java.net.InetAddress[] found = java.net.InetAddress.getAllByName(host);
        java.util.LinkedHashMap<String, NetSystemResolvedEndpoint> unique = new java.util.LinkedHashMap<>();
        for (java.net.InetAddress endpoint : found) {
            boolean matches = isAdt(network, "Network::Tcp")
                || isAdt(network, "Network::Tcp4") && endpoint instanceof java.net.Inet4Address
                || isAdt(network, "Network::Tcp6") && endpoint instanceof java.net.Inet6Address;
            if (matches) {
                Object endpointNetwork = endpoint instanceof java.net.Inet4Address ? netSystemNetwork("Tcp4") : netSystemNetwork("Tcp6");
                unique.put(netSystemNetworkName(endpointNetwork) + "\u0000" + endpoint.getHostAddress(),
                    new NetSystemResolvedEndpoint(endpointNetwork, endpoint.getHostAddress()));
            }
        }
        return new java.util.ArrayList<>(unique.values());
    }
    private static java.util.List<NetSystemResolvedEndpoint> netSystemResolved(Object address, Object deadline, Object token) throws java.net.UnknownHostException {
        Object network = netSystemAddressNetwork(address);
        String host = netSystemAddressHost(address);
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) {
            return netSystemResolvedWithAdapter(network, host, deadline, token, adapter);
        }
        return netSystemResolvedOnHost(network, host);
    }
    private static java.util.List<NetSystemResolvedEndpoint> netSystemResolved(Object address) throws java.net.UnknownHostException {
        return netSystemResolved(address, null, null);
    }
    private static long netSystemConnectControlWaitMillis(Object deadline, Object token) throws java.net.SocketTimeoutException {
        throwTaskCancellationIfRequested();
        if (token != null && waitCancelled(asCancelToken(token))) {
            throw new java.util.concurrent.CancellationException("connect cancelled");
        }
        if (deadline == null) return 10L;
        long remaining = deadlineRemainingNanos(deadline);
        if (remaining == 0L) throw new java.net.SocketTimeoutException("connect timed out");
        long roundedMillis = (remaining - 1L) / 1000000L + 1L;
        return Math.min(10L, roundedMillis);
    }
    private static java.util.List<NetSystemResolvedEndpoint> netSystemResolvedForConnect(
        Object address,
        Object deadline,
        Object token
    ) throws Throwable {
        netSystemConnectControlWaitMillis(deadline, token);
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) {
            java.util.List<NetSystemResolvedEndpoint> resolved = netSystemResolved(address, deadline, token);
            netSystemConnectControlWaitMillis(deadline, token);
            return resolved;
        }
        Object network = netSystemAddressNetwork(address);
        String host = netSystemAddressHost(address);
        java.util.concurrent.Future<java.util.List<NetSystemResolvedEndpoint>> resolution =
            NET_SYSTEM_RESOLVERS.submit(() -> netSystemResolvedOnHost(network, host));
        try {
            while (true) {
                long waitMillis = netSystemConnectControlWaitMillis(deadline, token);
                try {
                    java.util.List<NetSystemResolvedEndpoint> resolved = resolution.get(
                        waitMillis,
                        java.util.concurrent.TimeUnit.MILLISECONDS
                    );
                    netSystemConnectControlWaitMillis(deadline, token);
                    return resolved;
                } catch (java.util.concurrent.TimeoutException pending) {
                    // Recheck cancellation, deadline, and task cancellation before waiting again.
                } catch (InterruptedException interrupted) {
                    throwTaskCancellationIfRequested();
                    Thread.currentThread().interrupt();
                    throw new RuntimeFailure("network resolution interrupted", interrupted);
                } catch (java.util.concurrent.ExecutionException failed) {
                    throw failed.getCause();
                }
            }
        } finally {
            resolution.cancel(true);
        }
    }
    private static Object netSystemEndpoints(Object address) throws java.net.UnknownHostException {
        long port = netSystemAddressPort(address);
        Object endpoints = listNil();
        java.util.List<NetSystemResolvedEndpoint> resolved = netSystemResolved(address);
        for (int index = resolved.size() - 1; index >= 0; index--) {
            NetSystemResolvedEndpoint endpoint = resolved.get(index);
            endpoints = listCons(netSystemEndpoint(endpoint.network, netSystemJoinHostPort(endpoint.address, port)), endpoints);
        }
        return endpoints;
    }
    private static Object netSystemValidateAddress(String operation, Object address) {
        long port = netSystemAddressPort(address);
        if (port < 0L || port > 65535L) return netSystemError(operation, netSystemAddressNetwork(address), netSystemAddressHost(address), "InvalidAddress", "port must be between 0 and 65535");
        return null;
    }
    public static Object netSystemResolve(Object address) {
        HandlerFrame owner = netSystemOwner();
        if (owner.networkSystemClosed) return err(netSystemInvalidResource("resolve"));
        Object invalid = netSystemValidateAddress("resolve", address);
        if (invalid != null) return err(invalid);
        try {
            Object endpoints = netSystemEndpoints(address);
            if (!asListValue(endpoints).cons) return err(netSystemError("resolve", netSystemAddressNetwork(address), netSystemAddressHost(address), "NameNotFound", "no endpoint matched the requested network"));
            return ok(endpoints);
        } catch (Throwable failure) {
            return err(netSystemFailure("resolve", netSystemAddressNetwork(address), netSystemAddressHost(address), failure));
        }
    }
    public static Object netSystemListen(Object address) {
        HandlerFrame owner = netSystemOwner();
        if (owner.networkSystemClosed) return err(netSystemInvalidResource("listen"));
        Object invalid = netSystemValidateAddress("listen", address);
        if (invalid != null) return err(invalid);
        Object network = netSystemAddressNetwork(address);
        String subject = netSystemJoinHostPort(netSystemAddressHost(address), netSystemAddressPort(address));
        try {
            if (netSystemAddressHost(address).isEmpty()) {
                subject = netSystemJoinHostPort(isAdt(network, "Network::Tcp6") ? "::" : "0.0.0.0", netSystemAddressPort(address));
            } else {
                java.util.List<NetSystemResolvedEndpoint> resolved = netSystemResolved(address);
                if (resolved.isEmpty()) return err(netSystemError("listen", network, subject, "NameNotFound", "no endpoint matched the requested network"));
                subject = netSystemJoinHostPort(resolved.get(0).address, netSystemAddressPort(address));
            }
            NetListener listener = asNetListener(netListen(subject));
            if (!netSystemRegisterListener(owner, listener)) {
                try { netCloseListener(listener); } catch (Throwable ignored) { }
                return err(netSystemInvalidResource("listen"));
            }
            return ok(listener);
        } catch (Throwable failure) {
            return err(netSystemFailure("listen", network, subject, failure));
        }
    }
    private static void netSystemFinishConnect(java.nio.channels.SocketChannel socket, Object deadline, Object token) throws java.io.IOException {
        SocketReadiness readiness = null;
        try {
            while (!socket.finishConnect()) {
                if (token != null && waitCancelled(asCancelToken(token))) throw new java.util.concurrent.CancellationException("connect cancelled");
                long remaining = deadline == null ? Long.MAX_VALUE : deadlineRemainingNanos(deadline);
                if (remaining == 0L) throw new java.net.SocketTimeoutException("connect timed out");
                if (readiness == null) readiness = new SocketReadiness(
                    socket, java.nio.channels.SelectionKey.OP_CONNECT);
                readiness.await(deadline != null || token != null ? 10L : 0L);
            }
        } finally {
            if (readiness != null) try { readiness.close(); } catch (java.io.IOException ignored) { }
        }
    }
    private static NetStream netSystemConnectEndpoint(String subject, Object deadline, Object token) throws java.io.IOException {
        HandlerFrame adapter = findHandler(NETWORK_EFFECT);
        if (adapter != null) return adapterStream(adapter, networkRequest(adapter, "connect", subject, emptyNetworkBytes(), deadline, token));
        java.nio.channels.SocketChannel socket = java.nio.channels.SocketChannel.open();
        try {
            socket.configureBlocking(false);
            if (token != null && waitCancelled(asCancelToken(token))) throw new java.util.concurrent.CancellationException("connect cancelled");
            if (deadline != null && deadlineRemainingNanos(deadline) == 0L) throw new java.net.SocketTimeoutException("connect timed out");
            socket.connect(parseSocketAddress(subject));
            netSystemFinishConnect(socket, deadline, token);
            return new NetStream(formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getRemoteAddress()), socket, null);
        } catch (Throwable failure) {
            try { socket.close(); } catch (java.io.IOException ignored) { }
            if (failure instanceof java.io.IOException) throw (java.io.IOException) failure;
            throw failure;
        }
    }
    public static Object netSystemConnect(Object address, Object deadlineOption, Object tokenOption) {
        HandlerFrame owner = netSystemOwner();
        if (owner.networkSystemClosed) return err(netSystemInvalidResource("connect"));
        Object invalid = netSystemValidateAddress("connect", address);
        if (invalid != null) return err(invalid);
        Object deadline = netSystemOptionValue(deadlineOption);
        Object token = netSystemOptionValue(tokenOption);
        Object network = netSystemAddressNetwork(address);
        Throwable last = null;
        try {
            for (NetSystemResolvedEndpoint endpoint : netSystemResolvedForConnect(address, deadline, token)) {
                String subject = netSystemJoinHostPort(endpoint.address, netSystemAddressPort(address));
                try {
                    NetStream stream = netSystemConnectEndpoint(subject, deadline, token);
                    if (!netSystemRegisterStream(owner, stream)) {
                        try { netCloseStream(stream); } catch (Throwable ignored) { }
                        return err(netSystemInvalidResource("connect"));
                    }
                    return ok(stream);
                } catch (java.util.concurrent.CancellationException cancelled) {
                    return err(netSystemError("connect", network, subject, "Cancelled", cancelled.getMessage()));
                } catch (TaskCancellationFailure cancelled) {
                    throw cancelled;
                } catch (Throwable failure) {
                    netSystemRejectUnexpectedFailure(failure);
                    if (failure instanceof TransportFailure) {
                        String category = ((TransportFailure) failure).category;
                        if (category.equals("cancelled") || category.equals("timed_out")) {
                            return err(netSystemFailure("connect", network, subject, failure));
                        }
                    }
                    last = failure;
                }
            }
            if (last == null) return err(netSystemError("connect", network, netSystemAddressHost(address), "NameNotFound", "no endpoint matched the requested network"));
            return err(netSystemFailure("connect", network, netSystemAddressHost(address), last));
        } catch (java.util.concurrent.CancellationException cancelled) {
            return err(netSystemError("connect", network, netSystemAddressHost(address), "Cancelled", cancelled.getMessage()));
        } catch (Throwable failure) {
            return err(netSystemFailure("connect", network, netSystemAddressHost(address), failure));
        }
    }
    private static Object netSystemEndpointFromText(String text) {
        try {
            java.net.InetSocketAddress address = parseSocketAddress(text);
            Object network = address.getAddress() instanceof java.net.Inet6Address ? netSystemNetwork("Tcp6") : netSystemNetwork("Tcp4");
            return netSystemEndpoint(network, formatSocketAddress(address));
        } catch (RuntimeException failure) {
            if (failure instanceof RuntimeFailure) throw (RuntimeFailure) failure;
            return netSystemEndpoint(netSystemNetwork("Tcp"), text);
        }
    }
    public static Object netSystemAccept(Object value, Object deadlineOption, Object tokenOption) {
        HandlerFrame owner = netSystemOwner();
        NetListener listener = asNetListener(value);
        if (!netSystemOwns(listener, owner)) return err(netSystemInvalidResource("accept"));
        if (netSystemListenerUnavailable(listener)) return err(netSystemError("accept", null, listener.localAddress, "Closed", "listener is closed"));
        if (!listener.accepting.compareAndSet(false, true)) return err(netSystemError("accept", null, listener.localAddress, "Busy", "another accept is active"));
        try {
            Object outcome = acceptHost(listener, netSystemOptionValue(deadlineOption), netSystemOptionValue(tokenOption), "accept");
            if (netSystemListenerUnavailable(listener)) {
                if (isAdt(outcome, "AcceptOutcome::AcceptStream")) {
                    try { netCloseStream(asNetStream(((Adt) outcome).payloads[0])); } catch (Throwable ignored) { }
                }
                return err(netSystemError("accept", null, listener.localAddress, "Closed", "listener is closed"));
            }
            if (isAdt(outcome, "AcceptOutcome::AcceptDeadlineExpired")) return err(netSystemError("accept", null, listener.localAddress, "TimedOut", "accept deadline elapsed"));
            if (isAdt(outcome, "AcceptOutcome::AcceptCancelled")) return err(netSystemError("accept", null, listener.localAddress, "Cancelled", "accept was cancelled"));
            if (isAdt(outcome, "AcceptOutcome::AcceptEnd")) return err(netSystemError("accept", null, listener.localAddress, "Closed", "listener reached clean end"));
            NetStream stream = asNetStream(((Adt) outcome).payloads[0]);
            if (!netSystemRegisterStream(owner, stream)) {
                try { netCloseStream(stream); } catch (Throwable ignored) { }
                return err(netSystemInvalidResource("accept"));
            }
            return ok(stream);
        } catch (Throwable failure) {
            netSystemRejectUnexpectedFailure(failure);
            return err(netSystemListenerUnavailable(listener) ? netSystemError("accept", null, listener.localAddress, "Closed", "listener is closed") : netSystemFailure("accept", null, listener.localAddress, failure));
        } finally { listener.accepting.set(false); }
    }
    public static Object netSystemListenerAddress(Object value) {
        HandlerFrame owner = netSystemOwner(); NetListener listener = asNetListener(value);
        if (!netSystemOwns(listener, owner)) return err(netSystemInvalidResource("listener_address"));
        if (netSystemListenerUnavailable(listener)) return err(netSystemError("listener_address", null, listener.localAddress, "Closed", "listener is closed"));
        try { return ok(netSystemEndpointFromText(asString(netListenerLocalAddr(listener)))); }
        catch (Throwable failure) { return err(netSystemFailure("listener_address", null, listener.localAddress, failure)); }
    }
    public static Object netSystemCloseListener(Object value) {
        HandlerFrame owner = netSystemOwner(); NetListener listener = asNetListener(value);
        if (!netSystemOwns(listener, owner)) return err(netSystemInvalidResource("close_listener"));
        synchronized (listener.lifecycleLock) {
            if (listener.systemClosed) return ok(UNIT);
            try {
                netCloseListener(listener);
                listener.systemClosed = true;
                listener.closeUncertain = false;
                netSystemReleaseResource(owner, listener);
                return ok(UNIT);
            } catch (Throwable failure) {
                Object error = netSystemFailure("close_listener", null, listener.localAddress, failure);
                Boolean committed = netSystemFailureCommit(failure, false, false);
                if (Boolean.TRUE.equals(committed)) {
                    listener.systemClosed = true;
                    listener.closeUncertain = false;
                    netSystemReleaseResource(owner, listener);
                } else {
                    listener.closeUncertain = committed == null;
                }
                return err(error);
            }
        }
    }
    public static Object netSystemRead(Object value, Object deadlineOption, Object tokenOption) {
        HandlerFrame owner = netSystemOwner(); NetStream stream = asNetStream(value);
        if (!netSystemOwns(stream, owner)) return err(netSystemInvalidResource("read"));
        if (netSystemReadUnavailable(stream) || stream.readFailed) return err(netSystemError("read", null, stream.peerAddress, "Closed", "stream read side is closed"));
        if (stream.peerEnded) return ok(adt("ReadOutcome::ReadEnd", new Object[] {}));
        if (!stream.reading.compareAndSet(false, true)) return err(netSystemError("read", null, stream.peerAddress, "Busy", "another read is active"));
        try {
            Object outcome = readHost(stream, netSystemOptionValue(deadlineOption), netSystemOptionValue(tokenOption), "read");
            if (netSystemReadUnavailable(stream)) return err(netSystemError("read", null, stream.peerAddress, "Closed", "stream read side is closed"));
            if (isAdt(outcome, "StreamReadOutcome::ReadDeadlineExpired")) return err(netSystemError("read", null, stream.peerAddress, "TimedOut", "read deadline elapsed"));
            if (isAdt(outcome, "StreamReadOutcome::ReadCancelled")) return err(netSystemError("read", null, stream.peerAddress, "Cancelled", "read was cancelled"));
            if (isAdt(outcome, "StreamReadOutcome::ReadEnd")) { stream.peerEnded = true; return ok(adt("ReadOutcome::ReadEnd", new Object[] {})); }
            return ok(adt("ReadOutcome::ReadChunk", new Object[] { ((Adt) outcome).payloads[0] }));
        } catch (Throwable failure) {
            netSystemRejectUnexpectedFailure(failure);
            if (netSystemReadUnavailable(stream)) return err(netSystemError("read", null, stream.peerAddress, "Closed", "stream read side is closed"));
            Object error = netSystemFailure("read", null, stream.peerAddress, failure);
            if (!netSystemFailureKeepsResourceOpen(error)) stream.readFailed = true;
            return err(error);
        } finally { stream.reading.set(false); }
    }
    private static Object netSystemWriteFailure(long committed, Object error) {
        return adt("WriteOutcome::WriteFailed", new Object[] { byteCountObject(committed), error });
    }
    public static Object netSystemWrite(Object value, Object bytes, Object deadlineOption, Object tokenOption) {
        HandlerFrame owner = netSystemOwner(); NetStream stream = asNetStream(value);
        if (!netSystemOwns(stream, owner)) return netSystemWriteFailure(0L, netSystemInvalidResource("write"));
        if (netSystemWriteUnavailable(stream) || stream.writeFailed) return netSystemWriteFailure(0L, netSystemError("write", null, stream.peerAddress, "Closed", "stream write side is closed"));
        if (!stream.writing.compareAndSet(false, true)) return netSystemWriteFailure(0L, netSystemError("write", null, stream.peerAddress, "Busy", "another write is active"));
        long committed = 0L;
        SocketReadiness readiness = null;
        try {
            Object deadline = netSystemOptionValue(deadlineOption); Object token = netSystemOptionValue(tokenOption);
            byte[] requestedBytes = byteChunkToByteArray(bytes);
            if (requestedBytes.length == 0) return adt("WriteOutcome::Written", new Object[] { byteCountObject(0L) });
            if (token != null && waitCancelled(asCancelToken(token))) return netSystemWriteFailure(0L, netSystemError("write", null, stream.peerAddress, "Cancelled", "write was cancelled"));
            if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return netSystemWriteFailure(0L, netSystemError("write", null, stream.peerAddress, "TimedOut", "write deadline elapsed"));
            if (stream.adapter != null) {
                Object reply = rawNetworkRequest(stream.adapter, "write", stream.id, bytes, deadline, token);
                String status = replyText(reply, "status");
                long reported = asLong(recordField(reply, "bytes_committed"));
                long requested = requestedBytes.length;
                if (reported < 0L || reported > requested) throw new NetworkProtocolFailure("network handler returned an invalid committed byte count");
                committed = reported;
                if (netSystemWriteUnavailable(stream)) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "Closed", "stream write side is closed"));
                if (status.equals("deadline")) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "TimedOut", "write deadline elapsed"));
                if (status.equals("cancelled")) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "Cancelled", "write was cancelled"));
                if (status.equals("error")) {
                    Object error = netSystemFailure("write", null, stream.peerAddress, networkReplyFailure(reply, "write", stream.id));
                    if (!netSystemFailureKeepsResourceOpen(error)) stream.writeFailed = true;
                    return netSystemWriteFailure(committed, error);
                }
                return adt("WriteOutcome::Written", new Object[] { byteCountObject(committed) });
            }
            failIfHostStreamClosed(stream, "write");
            java.nio.ByteBuffer output = java.nio.ByteBuffer.wrap(requestedBytes);
            while (output.hasRemaining()) {
                if (netSystemWriteUnavailable(stream)) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "Closed", "stream write side is closed"));
                if (token != null && waitCancelled(asCancelToken(token))) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "Cancelled", "write was cancelled"));
                if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "TimedOut", "write deadline elapsed"));
                int written = stream.socket.write(output);
                committed += written;
                if (written == 0) {
                    if (readiness == null) readiness = new SocketReadiness(
                        stream.socket, java.nio.channels.SelectionKey.OP_WRITE);
                    readiness.await(10L);
                }
            }
            return adt("WriteOutcome::Written", new Object[] { byteCountObject(committed) });
        } catch (Throwable failure) {
            netSystemRejectUnexpectedFailure(failure);
            if (netSystemWriteUnavailable(stream)) return netSystemWriteFailure(committed, netSystemError("write", null, stream.peerAddress, "Closed", "stream write side is closed"));
            Object error = netSystemFailure("write", null, stream.peerAddress, failure);
            if (!netSystemFailureKeepsResourceOpen(error)) stream.writeFailed = true;
            return netSystemWriteFailure(committed, error);
        } finally {
            if (readiness != null) try { readiness.close(); } catch (java.io.IOException ignored) { }
            stream.writing.set(false);
        }
    }
    private static Object netSystemStreamAddress(Object value, boolean peer) {
        HandlerFrame owner = netSystemOwner(); NetStream stream = asNetStream(value);
        String operation = peer ? "peer_address" : "local_address";
        if (!netSystemOwns(stream, owner)) return err(netSystemInvalidResource(operation));
        if (netSystemStreamUnavailable(stream)) return err(netSystemError(operation, null, null, "Closed", "stream is closed"));
        try { return ok(netSystemEndpointFromText(asString(peer ? netStreamPeerAddr(stream) : netStreamLocalAddr(stream)))); }
        catch (Throwable failure) { return err(netSystemFailure(operation, null, null, failure)); }
    }
    public static Object netSystemLocalAddress(Object stream) { return netSystemStreamAddress(stream, false); }
    public static Object netSystemPeerAddress(Object stream) { return netSystemStreamAddress(stream, true); }
    private static Object netSystemShutdown(Object value, boolean read) {
        HandlerFrame owner = netSystemOwner(); NetStream stream = asNetStream(value);
        String operation = read ? "shutdown_read" : "shutdown_write";
        if (!netSystemOwns(stream, owner)) return err(netSystemInvalidResource(operation));
        synchronized (stream.lifecycleLock) {
            if (stream.systemClosed || read && stream.readShutdown || !read && stream.writeShutdown) return ok(UNIT);
            try {
                if (read) netShutdownRead(stream); else netShutdownWrite(stream);
                if (read) {
                    stream.readShutdown = true;
                    stream.readShutdownUncertain = false;
                } else {
                    stream.writeShutdown = true;
                    stream.writeShutdownUncertain = false;
                }
                return ok(UNIT);
            } catch (Throwable failure) {
                Object error = netSystemFailure(operation, null, stream.peerAddress, failure);
                Boolean committed = netSystemFailureCommit(failure, read, !read);
                if (read) {
                    stream.readShutdown = Boolean.TRUE.equals(committed);
                    stream.readShutdownUncertain = committed == null;
                } else {
                    stream.writeShutdown = Boolean.TRUE.equals(committed);
                    stream.writeShutdownUncertain = committed == null;
                }
                return err(error);
            }
        }
    }
    public static Object netSystemShutdownRead(Object stream) { return netSystemShutdown(stream, true); }
    public static Object netSystemShutdownWrite(Object stream) { return netSystemShutdown(stream, false); }
    public static Object netSystemCloseStream(Object value) {
        HandlerFrame owner = netSystemOwner(); NetStream stream = asNetStream(value);
        if (!netSystemOwns(stream, owner)) return err(netSystemInvalidResource("close_stream"));
        synchronized (stream.lifecycleLock) {
            if (stream.systemClosed) return ok(UNIT);
            try {
                netCloseStream(stream);
                stream.systemClosed = true; stream.closeUncertain = false;
                stream.readShutdown = true; stream.writeShutdown = true;
                netSystemReleaseResource(owner, stream);
                return ok(UNIT);
            } catch (Throwable failure) {
                Object error = netSystemFailure("close_stream", null, stream.peerAddress, failure);
                Boolean committed = netSystemFailureCommit(failure, false, false);
                if (Boolean.TRUE.equals(committed)) {
                    stream.systemClosed = true; stream.closeUncertain = false;
                    stream.readShutdown = true; stream.writeShutdown = true;
                    netSystemReleaseResource(owner, stream);
                } else {
                    stream.closeUncertain = committed == null;
                }
                return err(error);
            }
        }
    }
    private static void cleanupNetworkSystem(HandlerFrame owner) {
        java.util.List<Object> resources;
        synchronized (owner.networkResources) {
            owner.networkSystemClosed = true;
            resources = new java.util.ArrayList<Object>(owner.networkResources);
            owner.networkResources.clear();
        }
        for (Object resource : resources) {
            if (resource instanceof NetStream) {
                NetStream stream = (NetStream) resource;
                synchronized (stream.lifecycleLock) {
                    stream.systemClosed = true; stream.closeUncertain = false;
                    stream.readShutdown = true; stream.writeShutdown = true;
                    try { if (stream.adapter != null) streamRequest(stream, "close_stream", emptyNetworkBytes(), null, null); else stream.socket.close(); }
                    catch (Throwable ignored) { }
                }
            } else {
                NetListener listener = (NetListener) resource;
                synchronized (listener.lifecycleLock) {
                    listener.systemClosed = true; listener.closeUncertain = false;
                    try { if (listener.adapter != null) listenerRequest(listener, "close_listener", null, null); else listener.server.close(); }
                    catch (Throwable ignored) { }
                }
            }
        }
    }

    private static long clockRequest(String operation, long milliseconds) {
        Object reply = perform(CLOCK_EFFECT, "request", new Object[] { operation, Long.valueOf(milliseconds) });
        if (isErr(reply)) throw new RuntimeFailure(asString(asResult(reply).value));
        return asLong(unwrapOk(reply));
    }
    private static long monotonicNanos() {
        return findHandler(CLOCK_EFFECT) == null ? System.nanoTime() : clockRequest("now", 0L) * 1000000L;
    }
    public static Object timeMonotonicMs() { return Long.valueOf(monotonicNanos() / 1000000L); }
    public static Object timeWallTime() {
        if (findHandler(CLOCK_EFFECT) != null) {
            Object reply = perform(CLOCK_EFFECT, "wall_time", new Object[0]);
            if (isErr(reply)) throw new RuntimeFailure(asString(asResult(reply).value));
            Object value = unwrapOk(reply);
            long seconds = asLong(recordField(value, "unix_seconds"));
            long nanosecond = asLong(recordField(value, "nanosecond"));
            long carry = Math.floorDiv(nanosecond, 1000000000L);
            try { seconds = Math.addExact(seconds, carry); }
            catch (ArithmeticException error) { throw new RuntimeFailure("wall-clock seconds exceed Int range"); }
            return record(
                "unix_seconds", Long.valueOf(seconds),
                "nanosecond", Long.valueOf(Math.floorMod(nanosecond, 1000000000L))
            );
        }
        java.time.Instant instant = java.time.Instant.now();
        return record(
            "unix_seconds", Long.valueOf(instant.getEpochSecond()),
            "nanosecond", Long.valueOf(instant.getNano())
        );
    }
    public static Object timeTimeoutMs(Object milliseconds) {
        long value = asLong(milliseconds);
        if (findHandler(CLOCK_EFFECT) != null) { clockRequest("sleep", value); return UNIT; }
        if (value > 0L) try { Thread.sleep(value); }
        catch (InterruptedException error) {
            throwTaskCancellationIfRequested();
            Thread.currentThread().interrupt();
        }
        return UNIT;
    }
    public static Object timeDeadlineAfterMs(Object milliseconds) {
        long now = monotonicNanos(); long value = Math.max(0L, asLong(milliseconds));
        try { return Long.valueOf(Math.addExact(now, Math.multiplyExact(value, 1000000L))); }
        catch (ArithmeticException error) { return Long.valueOf(Long.MAX_VALUE); }
    }
    public static Object timeDeadlineAtMs(Object milliseconds) {
        long value = asLong(milliseconds);
        try { return Long.valueOf(Math.multiplyExact(value, 1000000L)); }
        catch (ArithmeticException error) { return Long.valueOf(value < 0L ? Long.MIN_VALUE : Long.MAX_VALUE); }
    }
    private static long deadlineRemainingNanos(Object deadline) {
        long target = asLong(deadline); long now = monotonicNanos();
        if (target <= now) return 0L;
        try { return Math.subtractExact(target, now); } catch (ArithmeticException error) { return Long.MAX_VALUE; }
    }
    public static Object timeWaitUntil(Object deadline) {
        long remaining = deadlineRemainingNanos(deadline);
        if (findHandler(CLOCK_EFFECT) != null) { clockRequest("wait", remaining / 1000000L); return UNIT; }
        if (remaining > 0L) try { Thread.sleep(remaining / 1000000L, (int)(remaining % 1000000L)); }
        catch (InterruptedException error) {
            throwTaskCancellationIfRequested();
            Thread.currentThread().interrupt();
        }
        return UNIT;
    }
    public static Object timeCancelToken() { return new CancelToken(false); }
    public static Object timeCancelOwner() { return new CancelOwner(); }
    public static Object timeCancelTokenFrom(Object owner) { return asCancelOwner(owner).token; }
    public static Object timeCancelOwned(Object owner) { asCancelOwner(owner).token.cancelled = true; return UNIT; }
    public static Object timeCancel(Object value) {
        CancelToken token = asCancelToken(value);
        if (token.observerOnly) throw new RuntimeFailure("cancellation token is observer-only");
        token.cancelled = true; return UNIT;
    }
    public static Object timeIsCancelled(Object token) { return Boolean.valueOf(asCancelToken(token).cancelled); }
    public static Object timeIsCancelledOwner(Object owner) { return Boolean.valueOf(asCancelOwner(owner).token.cancelled); }
    private static boolean waitCancelled(CancelToken token) {
        if (findHandler(CLOCK_EFFECT) != null && clockRequest("cancelled", 0L) != 0L) token.cancelled = true;
        return token.cancelled;
    }
    public static Object timeWaitUntilCancellable(Object deadline, Object token) {
        Object outcome = timeWaitUntilCancellableOutcome(deadline, token);
        if (isAdt(outcome, "WaitCancelled")) throw new RuntimeFailure("transport wait cancelled: cancellation token");
        if (isAdt(outcome, "WaitDeadlineExpired")) throw new RuntimeFailure("transport deadline expired: injected clock");
        return UNIT;
    }
    public static Object timeWaitUntilCancellableOutcome(Object deadline, Object value) {
        CancelToken token = asCancelToken(value);
        if (waitCancelled(token)) return waitOutcome("WaitCancelled");
        if (findHandler(CLOCK_EFFECT) != null) {
            long result = clockRequest("wait_cancellable", deadlineRemainingNanos(deadline) / 1000000L);
            return waitOutcome(result == 1L ? "WaitDeadlineExpired" : result == 2L ? "WaitCancelled" : "WaitCompleted");
        }
        long remaining;
        while ((remaining = deadlineRemainingNanos(deadline)) > 0L) {
            if (waitCancelled(token)) return waitOutcome("WaitCancelled");
            try { long slice = Math.min(remaining, 10000000L); Thread.sleep(slice / 1000000L, (int)(slice % 1000000L)); }
            catch (InterruptedException error) {
                throwTaskCancellationIfRequested();
                Thread.currentThread().interrupt();
                token.cancelled = true;
                return waitOutcome("WaitCancelled");
            }
        }
        return waitOutcome(token.cancelled ? "WaitCancelled" : "WaitCompleted");
    }

    public static Object fsPath(Object text) { return pathValue(java.nio.file.Paths.get(asString(text))); }

    public static Object stringConcat(Object left, Object right) { return asString(left) + asString(right); }
