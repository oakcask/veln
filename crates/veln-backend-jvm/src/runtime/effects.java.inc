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
        return cause instanceof java.net.SocketTimeoutException ? "timed_out" : "io_failure";
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

    private static final class NetListener {
        final String address;
        final String localAddress;
        final java.nio.channels.ServerSocketChannel server;
        final HandlerFrame adapter;
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
        boolean readShutdown;
        boolean writeShutdown;
        NetStream(String id, String local, String peer, java.nio.channels.SocketChannel socket, HandlerFrame adapter) {
            this.id = id;
            this.localAddress = local;
            this.peerAddress = peer;
            this.socket = socket;
            this.adapter = adapter;
        }
        public String toString() { return "NetStream(" + id + ")"; }
    }

    private static NetListener asNetListener(Object value) { return (NetListener) value; }
    private static NetStream asNetStream(Object value) { return (NetStream) value; }
    private static Object emptyNetworkBytes() { return byteChunkObject(java.util.Collections.emptyList()); }
    private static String replyText(Object reply, String key) { return asString(recordField(reply, key)); }
    private static Boolean replyCommit(Object reply, String key) {
        long value = asLong(recordField(reply, key));
        return value < 0L ? null : Boolean.valueOf(value != 0L);
    }
    private static String optionalEndpoint(String value) { return value.isEmpty() ? null : value; }
    private static Object networkRequest(HandlerFrame adapter, String operation, String subject, Object bytes, Object deadline, Object token) {
        long remaining = deadline == null ? -1L : deadlineRemainingNanos(deadline) / 1000000L;
        Object reply = invokeHandler(adapter, "request", new Object[] {
            operation, subject, bytes, Long.valueOf(remaining),
            Boolean.valueOf(token != null && asCancelToken(token).cancelled)
        });
        if (replyText(reply, "status").equals("error")) {
            if (operation.equals("receive") || operation.equals("send"))
                throw new RuntimeFailure("transport " + operation + " failed: " + replyText(reply, "cause"));
            if (operation.endsWith("_addr"))
                throw new RuntimeFailure("transport address failed: " + replyText(reply, "cause"));
            String kind = operation;
            if (kind.startsWith("read")) kind = "read";
            else if (kind.startsWith("write")) kind = "write";
            else if (kind.startsWith("accept")) kind = "accept";
            else if (kind.startsWith("shutdown")) kind = "shutdown";
            else if (kind.equals("close_stream")) kind = "stream_close";
            else if (kind.equals("close_listener")) kind = "listener_close";
            throw transportFailure(kind, replyText(reply, "category"),
                optionalEndpoint(replyText(reply, "local")), kind.equals("listener_close") ? null : optionalEndpoint(replyText(reply, "peer")),
                kind.equals("listen") ? optionalEndpoint(replyText(reply, "value")) : kind.equals("accept") || kind.equals("listener_close") ? subject : null,
                kind.equals("listen") || kind.equals("listener_close") ? null : kind.equals("connect") || kind.equals("accept") ? optionalEndpoint(replyText(reply, "value")) : optionalEndpoint(subject),
                replyText(reply, "phase"), replyCommit(reply, "input_committed"),
                replyCommit(reply, "output_committed"), replyCommit(reply, "ownership_committed"),
                optionalEndpoint(replyText(reply, "cause")), null);
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
        return new java.net.InetSocketAddress(address.substring(0, separator), port);
    }
    private static String formatSocketAddress(java.net.SocketAddress value) {
        java.net.InetSocketAddress address = (java.net.InetSocketAddress) value;
        return address.getAddress().getHostAddress() + ":" + address.getPort();
    }
    private static void awaitSocketReady(java.nio.channels.SelectableChannel channel, int operation, long timeoutMillis) throws java.io.IOException {
        try (java.nio.channels.Selector selector = java.nio.channels.Selector.open()) {
            channel.register(selector, operation);
            if (timeoutMillis > 0L) selector.select(timeoutMillis);
            else selector.select();
        }
        throwTaskCancellationIfRequested();
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
                    awaitSocketReady(listener.server, java.nio.channels.SelectionKey.OP_ACCEPT,
                        deadline != null || token != null ? 10L : 0L);
                    continue;
                }
                socket.configureBlocking(false);
                NetStream stream = new NetStream(formatSocketAddress(socket.getRemoteAddress()), formatSocketAddress(socket.getLocalAddress()), formatSocketAddress(socket.getRemoteAddress()), socket, null);
                return acceptOutcome("AcceptStream", stream);
            }
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw listenerTransportFailure("accept", transportIoCategory(error), listener, "during_operation", false, platformCause("production socket failed", error), error);
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
            return streamReadOutcome("ReadChunk", recordField(reply, "bytes"));
        }
        try {
            throwTaskCancellationIfRequested();
            while (true) {
                failIfHostStreamClosed(stream, "read");
                if (stream.readShutdown) return streamReadOutcome("ReadEnd");
                if (token != null && waitCancelled(asCancelToken(token))) return streamReadOutcome("ReadCancelled");
                long remaining = deadline == null ? Long.MAX_VALUE : deadlineRemainingNanos(deadline);
                if (remaining == 0L) return streamReadOutcome("ReadDeadlineExpired");
                java.nio.ByteBuffer bytes = java.nio.ByteBuffer.allocate(4096);
                int count = stream.socket.read(bytes);
                if (count < 0) return streamReadOutcome("ReadEnd");
                if (count == 0) {
                    awaitSocketReady(stream.socket, java.nio.channels.SelectionKey.OP_READ,
                        deadline != null || token != null ? 10L : 0L);
                    continue;
                }
                return streamReadOutcome("ReadChunk", byteChunkFromByteArray(bytes.array(), count));
            }
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw hostStreamFailure("read", stream, error);
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
                    awaitSocketReady(stream.socket, java.nio.channels.SelectionKey.OP_WRITE,
                        deadline != null || token != null ? 10L : 0L);
                    if (token != null && waitCancelled(asCancelToken(token))) return streamWriteOutcome("WriteCancelled");
                    if (deadline != null && deadlineRemainingNanos(deadline) == 0L) return streamWriteOutcome("WriteDeadlineExpired");
                }
            }
            return streamWriteOutcome("WriteCompleted");
        } catch (java.io.IOException error) {
            throwTaskCancellationIfRequested();
            throw hostStreamFailure("write", stream, error);
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
                if (!stream.readShutdown) { stream.readShutdown = true; stream.socket.socket().shutdownInput(); }
            } else {
                failIfHostStreamClosed(stream, "shutdown");
                if (!stream.writeShutdown) { stream.writeShutdown = true; stream.socket.socket().shutdownOutput(); }
            }
            return UNIT;
        } catch (java.io.IOException error) {
            throw streamTransportFailure(operation.equals("close_stream") ? "stream_close" : "shutdown",
                transportIoCategory(error), stream, operation.equals("close_stream") ? "closed_handle" : operation.equals("shutdown_read") ? "read_shutdown" : "write_shutdown",
                false, false, true, platformCause("production socket failed", error), error);
        }
    }
    public static Object netShutdownWrite(Object stream) { return streamLifecycle(stream, "shutdown_write"); }
    public static Object netShutdownRead(Object stream) { return streamLifecycle(stream, "shutdown_read"); }
    public static Object netCloseStream(Object stream) { return streamLifecycle(stream, "close_stream"); }
    public static Object netCloseListener(Object value) {
        NetListener listener = asNetListener(value);
        if (listener.adapter != null) { listenerRequest(listener, "close_listener", null, null); return UNIT; }
        try { listener.server.close(); return UNIT; }
        catch (java.io.IOException error) { throw listenerTransportFailure("listener_close", transportIoCategory(error), listener, "during_operation", false, error.getMessage(), error); }
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
