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
        recordTransportFailure(
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
        if (cause.startsWith("VELN_NET_")) {
            return "forced_failure";
        }
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

    private static TransportFailure readEventFailure(
        NetStream stream,
        String phase,
        boolean inputCommitted,
        Throwable cause
    ) {
        return streamTransportFailure(
            "read", "event_record_failed", stream, phase,
            Boolean.valueOf(inputCommitted), Boolean.FALSE, false,
            "could not record VELN_NET_EVENTS", cause
        );
    }

    private static TransportFailure invalidReadFixture(NetStream stream, String sourceName) {
        return streamTransportFailure(
            "read", "invalid_input", stream, "input_consumed",
            Boolean.TRUE, Boolean.FALSE, false, "malformed " + sourceName, null
        );
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

    private static final class NetListener {
        private final String address;
        private final String localAddress;
        private final boolean productionSocket;
        private final java.net.ServerSocket server;
        private final LoopbackClient loopbackClient;
        private java.util.List<InMemoryLoopback> inMemoryLoopbacks;
        private final int productionStreamCount;
        private final java.util.concurrent.atomic.AtomicInteger sourceConnectionCount =
            new java.util.concurrent.atomic.AtomicInteger(0);
        private final java.util.concurrent.ConcurrentLinkedQueue<NetStream> sourceAcceptedStreams =
            new java.util.concurrent.ConcurrentLinkedQueue<NetStream>();
        private final java.util.concurrent.atomic.AtomicBoolean closed =
            new java.util.concurrent.atomic.AtomicBoolean(false);
        private final java.util.concurrent.atomic.AtomicInteger productionAcceptIndex =
            new java.util.concurrent.atomic.AtomicInteger(0);

        private NetListener(String address) {
            this(address, address, false, null, null, null, 0);
        }

        private NetListener(String address, java.net.ServerSocket server, LoopbackClient loopbackClient) {
            this(
                address,
                formatSocketAddress(server.getLocalSocketAddress()),
                true,
                server,
                loopbackClient,
                null,
                loopbackClient == null ? 1 : loopbackClient.streamCount()
            );
        }

        private NetListener(String address, java.util.List<InMemoryLoopback> inMemoryLoopbacks) {
            this(address, address, true, null, null, inMemoryLoopbacks, inMemoryLoopbacks.size());
        }

        private NetListener(
            String address,
            String localAddress,
            boolean productionSocket,
            java.net.ServerSocket server,
            LoopbackClient loopbackClient,
            java.util.List<InMemoryLoopback> inMemoryLoopbacks,
            int productionStreamCount
        ) {
            this.address = address;
            this.localAddress = localAddress;
            this.productionSocket = productionSocket;
            this.server = server;
            this.loopbackClient = loopbackClient;
            this.inMemoryLoopbacks = inMemoryLoopbacks;
            this.productionStreamCount = productionStreamCount;
        }

        private boolean isProductionSocket() {
            return productionSocket;
        }

        private int nextProductionAcceptIndex() {
            return productionAcceptIndex.getAndIncrement();
        }

        private int recordSourceConnection() {
            return sourceConnectionCount.getAndIncrement();
        }

        private boolean hasSourceConnection() {
            return sourceConnectionCount.get() > 0;
        }

        private void addSourceAcceptedStream(NetStream stream) {
            sourceAcceptedStreams.add(stream);
        }

        private NetStream pollSourceAcceptedStream() {
            return sourceAcceptedStreams.poll();
        }

        private boolean isClosed() {
            return closed.get();
        }

        private boolean closeOnce() {
            return closed.compareAndSet(false, true);
        }

        private void releaseLoopbackListenerState() {
            inMemoryLoopbacks = null;
        }

        @Override
        public String toString() {
            return "NetListener(" + address + ")";
        }
    }

    private static final class NetStream {
        private final String id;
        private final String localAddress;
        private final String peerAddress;
        private final java.net.Socket socket;
        private final LoopbackClient loopbackClient;
        private final InMemoryLoopback inMemoryLoopback;
        private final boolean lastProductionStream;
        private final java.io.InputStream input;
        private final java.io.OutputStream output;
        private final java.util.concurrent.atomic.AtomicBoolean readShutdown =
            new java.util.concurrent.atomic.AtomicBoolean(false);
        private final java.util.concurrent.atomic.AtomicBoolean writeShutdown =
            new java.util.concurrent.atomic.AtomicBoolean(false);
        private final java.util.concurrent.atomic.AtomicBoolean closed =
            new java.util.concurrent.atomic.AtomicBoolean(false);

        private NetStream(String id) {
            this(id, id, id, null, null, null, false, null, null);
        }

        private NetStream(String id, String localAddress, String peerAddress) {
            this(id, localAddress, peerAddress, null, null, null, false, null, null);
        }

        private NetStream(
            String id,
            String localAddress,
            String peerAddress,
            java.net.Socket socket,
            LoopbackClient loopbackClient,
            boolean lastProductionStream,
            java.io.InputStream input,
            java.io.OutputStream output
        ) {
            this(
                id,
                localAddress,
                peerAddress,
                socket,
                loopbackClient,
                null,
                lastProductionStream,
                input,
                output
            );
        }

        private NetStream(
            String id,
            String localAddress,
            String peerAddress,
            InMemoryLoopback inMemoryLoopback,
            boolean lastProductionStream
        ) {
            this(
                id,
                localAddress,
                peerAddress,
                null,
                null,
                inMemoryLoopback,
                lastProductionStream,
                inMemoryLoopback.serverInput(),
                inMemoryLoopback.serverOutput()
            );
        }

        private NetStream(
            String id,
            String localAddress,
            String peerAddress,
            java.net.Socket socket,
            LoopbackClient loopbackClient,
            InMemoryLoopback inMemoryLoopback,
            boolean lastProductionStream,
            java.io.InputStream input,
            java.io.OutputStream output
        ) {
            this.id = id;
            this.localAddress = localAddress;
            this.peerAddress = peerAddress;
            this.socket = socket;
            this.loopbackClient = loopbackClient;
            this.inMemoryLoopback = inMemoryLoopback;
            this.lastProductionStream = lastProductionStream;
            this.input = input;
            this.output = output;
        }

        private boolean isProductionSocket() {
            return input != null && output != null;
        }

        private boolean isWriteShutdown() {
            return writeShutdown.get();
        }

        private boolean isReadShutdown() {
            return readShutdown.get();
        }

        private boolean isClosed() {
            return closed.get() || (socket != null && socket.isClosed());
        }

        private boolean closeOnce() {
            return closed.compareAndSet(false, true);
        }

        private boolean shutdownReadOnce() {
            return readShutdown.compareAndSet(false, true);
        }

        private boolean shutdownWriteOnce() {
            return writeShutdown.compareAndSet(false, true);
        }

        @Override
        public String toString() {
            return "NetStream(" + id + ")";
        }
    }

    private static final class InMemoryLoopback {
        private final java.util.List<byte[]> clientInputChunks;
        private final int index;
        private final ProductionWriteCapture capture;
        private final java.io.ByteArrayOutputStream clientWrites =
            new java.io.ByteArrayOutputStream();

        private InMemoryLoopback(
            java.util.List<byte[]> clientInputChunks,
            int index,
            ProductionWriteCapture capture
        ) {
            this.clientInputChunks = clientInputChunks;
            this.index = index;
            this.capture = capture;
        }

        private java.io.InputStream serverInput() {
            return new ChunkedLoopbackInputStream(clientInputChunks);
        }

        private java.io.OutputStream serverOutput() {
            return clientWrites;
        }

        private void capture() {
            capture.record(index, clientWrites.toByteArray());
        }
    }

    private static final class ChunkedLoopbackInputStream extends java.io.InputStream {
        private final java.util.List<byte[]> chunks;
        private int chunkIndex;
        private int byteIndex;

        private ChunkedLoopbackInputStream(java.util.List<byte[]> chunks) {
            this.chunks = chunks;
        }

        @Override
        public int read() {
            while (chunkIndex < chunks.size()) {
                byte[] chunk = chunks.get(chunkIndex);
                if (byteIndex < chunk.length) {
                    int value = chunk[byteIndex] & 0xff;
                    byteIndex += 1;
                    return value;
                }
                chunkIndex += 1;
                byteIndex = 0;
            }
            return -1;
        }

        @Override
        public int read(byte[] buffer, int offset, int length) {
            if (buffer == null) {
                throw new NullPointerException("buffer");
            }
            if (offset < 0 || length < 0 || length > buffer.length - offset) {
                throw new IndexOutOfBoundsException();
            }
            if (length == 0) {
                return 0;
            }
            while (chunkIndex < chunks.size()) {
                byte[] chunk = chunks.get(chunkIndex);
                if (byteIndex >= chunk.length) {
                    chunkIndex += 1;
                    byteIndex = 0;
                    continue;
                }
                int count = Math.min(length, chunk.length - byteIndex);
                System.arraycopy(chunk, byteIndex, buffer, offset, count);
                byteIndex += count;
                return count;
            }
            return -1;
        }
    }

    private static final class ProductionWriteCapture {
        private final byte[][] writes;

        private ProductionWriteCapture(int count) {
            this.writes = new byte[count][];
        }

        private synchronized void record(int index, byte[] bytes) {
            writes[index] = bytes;
            for (byte[] write : writes) {
                if (write == null) {
                    return;
                }
            }
            String capture = System.getenv("VELN_NET_PRODUCTION_CLIENT_WRITES");
            if (capture == null || capture.isEmpty()) {
                return;
            }
            try {
                java.nio.file.Files.write(
                    java.nio.file.Paths.get(capture),
                    joinedHexBytes(writes).getBytes(java.nio.charset.StandardCharsets.UTF_8)
                );
            } catch (java.io.IOException error) {
                throw new RuntimeException("could not capture loopback writes", error);
            }
        }
    }

    private static final class ProductionListenerBinding {
        private final NetListener listener;
        private final String host;
        private final int port;

        private ProductionListenerBinding(NetListener listener, String host, int port) {
            this.listener = listener;
            this.host = host;
            this.port = port;
        }
    }

    private static final class SourceLoopback {
        private final java.io.PipedInputStream serverInput;
        private final java.io.PipedInputStream clientInput;
        private final java.io.PipedOutputStream serverOutput;
        private final java.io.PipedOutputStream clientOutput;

        private SourceLoopback() throws java.io.IOException {
            serverInput = new java.io.PipedInputStream();
            clientInput = new java.io.PipedInputStream();
            serverOutput = new java.io.PipedOutputStream(clientInput);
            clientOutput = new java.io.PipedOutputStream(serverInput);
        }
    }

    private static final class LoopbackClient implements Runnable {
        private final String host;
        private final int port;
        private final java.util.List<java.util.List<byte[]>> outboundStreams;
        private final ProductionWriteCapture capture;
        private volatile RuntimeException failure;
        private Thread thread;

        private LoopbackClient(
            String host,
            int port,
            java.util.List<java.util.List<byte[]>> outboundStreams
        ) {
            this.host = host;
            this.port = port;
            this.outboundStreams = outboundStreams;
            this.capture = new ProductionWriteCapture(outboundStreams.size());
        }

        private int streamCount() {
            return outboundStreams.size();
        }

        private synchronized void start() {
            if (thread != null) {
                return;
            }
            thread = new Thread(this, "veln-net-loopback-client");
            thread.setDaemon(true);
            thread.start();
        }

        @Override
        public void run() {
            try {
                runClient();
            } catch (RuntimeException error) {
                failure = error;
            }
        }

        private void runClient() {
            java.util.ArrayList<java.net.Socket> sockets = new java.util.ArrayList<java.net.Socket>();
            try {
                for (java.util.List<byte[]> outboundChunks : outboundStreams) {
                    java.net.Socket socket = new java.net.Socket();
                    sockets.add(socket);
                    socket.connect(new java.net.InetSocketAddress(host, port), productionTimeoutMillis());
                    java.io.OutputStream output = socket.getOutputStream();
                    for (byte[] outbound : outboundChunks) {
                        output.write(outbound);
                        output.flush();
                    }
                    socket.shutdownOutput();
                }
                for (int index = 0; index < sockets.size(); index += 1) {
                    capture.record(index, readAllBytes(sockets.get(index).getInputStream()));
                }
            } catch (java.io.IOException error) {
                throw new RuntimeException(error);
            } finally {
                for (java.net.Socket socket : sockets) {
                    closeQuietly(socket);
                }
            }
        }

        private void await() {
            if (thread != null) {
                try {
                    thread.join(productionTimeoutMillis());
                } catch (InterruptedException error) {
                    Thread.currentThread().interrupt();
                    throw new RuntimeException("loopback client interrupted", error);
                }
                if (thread.isAlive()) {
                    throw new RuntimeException("loopback client timed out");
                }
            }
            if (failure != null) {
                throw new RuntimeException("loopback client failed", failure);
            }
        }
    }

    private static final java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.atomic.AtomicInteger>
        netReadIndexes = new java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.atomic.AtomicInteger>();
    private static final java.util.concurrent.ConcurrentHashMap<String, ProductionListenerBinding>
        productionListenerBindings = new java.util.concurrent.ConcurrentHashMap<String, ProductionListenerBinding>();
    private static final java.util.concurrent.atomic.AtomicInteger waitOutcomeIndex =
        new java.util.concurrent.atomic.AtomicInteger(0);

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

    public static Object netReceiveChunk() {
        String input = System.getenv("VELN_NET_CHUNK_HEX");
        if (input == null) {
            input = "";
        }
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport receive failed: malformed VELN_NET_CHUNK_HEX");
        }
        return unwrapOk(chunk);
    }

    public static Object netSendChunk(Object bytes) {
        try {
            writeNetEvent("send " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport send failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return UNIT;
    }

    public static Object netListen(Object address) {
        String value = asString(address);
        if (forced("VELN_NET_LISTEN_FAILED")) {
            throw transportFailure(
                "listen", "forced_failure", value, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                "VELN_NET_LISTEN_FAILED", null
            );
        }
        if (productionSocketRuntime()) {
            return netListenProduction(value);
        }
        try {
            writeNetEvent("listen " + value);
        } catch (java.io.IOException | RuntimeException error) {
            throw transportFailure(
                "listen", "event_record_failed", value, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return new NetListener(value);
    }

    public static Object netConnect(Object address) {
        String value = asString(address);
        if (forced("VELN_NET_CONNECT_FAILED")) {
            throw transportFailure(
                "connect", "forced_failure", null, value, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                "VELN_NET_CONNECT_FAILED", null
            );
        }
        if (productionSocketRuntime()) {
            return netConnectProduction(value);
        }
        String streamId = connectedNetStreamId();
        NetStream stream = new NetStream(streamId, streamId, value);
        try {
            writeNetEvent("connect " + value + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "connect", "event_record_failed", stream, "stream_connected",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return stream;
    }

    private static Object netConnectProduction(String address) {
        if (externalSocketRuntime()) {
            return netConnectExternal(address);
        }
        ProductionListenerBinding binding = productionListenerBindings.get(address);
        if (binding != null && !binding.listener.isClosed()) {
            return netConnectProductionListener(address, binding);
        }
        java.util.List<java.util.List<byte[]>> inputs;
        try {
            inputs = loopbackClientReadStreams();
        } catch (IllegalArgumentException error) {
            throw transportFailure(
                "connect", "invalid_input", null, address, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
        if (inputs.isEmpty()) {
            inputs = java.util.Collections.singletonList(
                java.util.Collections.<byte[]>emptyList()
            );
        }
        ProductionWriteCapture capture = new ProductionWriteCapture(1);
        InMemoryLoopback loopback = new InMemoryLoopback(inputs.get(0), 0, capture);
        String streamId = productionStreamId(0, 1);
        NetStream stream = new NetStream(streamId, loopbackPeerAddress(0), address, loopback, true);
        try {
            writeNetEvent("connect-production " + address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "connect", "event_record_failed", stream, "stream_connected",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return stream;
    }

    private static Object netConnectExternal(String address) {
        java.net.InetSocketAddress socketAddress;
        try {
            socketAddress = parseSocketAddress(address);
        } catch (RuntimeException error) {
            throw transportFailure(
                "connect", "invalid_endpoint", null, address, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
        java.net.Socket socket = null;
        String streamId = null;
        String localAddress = null;
        String peerAddress = address;
        boolean connected = false;
        try {
            socket = new java.net.Socket();
            socket.connect(socketAddress, productionTimeoutMillis());
            connected = true;
            socket.setSoTimeout(productionTimeoutMillis());
            streamId = sourceProductionStreamId(0);
            localAddress = formatSocketAddress(socket.getLocalSocketAddress());
            peerAddress = formatSocketAddress(socket.getRemoteSocketAddress());
            NetStream stream = new NetStream(
                streamId,
                localAddress,
                peerAddress,
                socket,
                null,
                false,
                socket.getInputStream(),
                socket.getOutputStream()
            );
            try {
                writeNetEvent("connect-production " + address + " " + streamId);
            } catch (java.io.IOException | RuntimeException error) {
                closeQuietly(socket);
                throw streamTransportFailure(
                    "connect", "event_record_failed", stream, "stream_connected",
                    Boolean.FALSE, Boolean.FALSE, true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return stream;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(socket);
            throw transportFailure(
                "connect", transportIoCategory(error), localAddress, peerAddress,
                null, streamId, connected ? "socket_connected" : "during_operation",
                Boolean.FALSE, Boolean.FALSE, Boolean.valueOf(connected),
                platformCause("production socket failed", error), error
            );
        }
    }

    private static Object netConnectProductionListener(
        String address,
        ProductionListenerBinding binding
    ) {
        if (binding.host == null) {
            return netConnectProductionInMemoryListener(address, binding.listener);
        }
        java.net.Socket socket = null;
        String streamId = null;
        String localAddress = null;
        String peerAddress = address;
        boolean connected = false;
        try {
            socket = new java.net.Socket();
            socket.connect(
                new java.net.InetSocketAddress(binding.host, binding.port),
                productionTimeoutMillis()
            );
            connected = true;
            socket.setSoTimeout(productionTimeoutMillis());
            int index = binding.listener.recordSourceConnection();
            streamId = sourceProductionStreamId(index);
            localAddress = formatSocketAddress(socket.getLocalSocketAddress());
            peerAddress = formatSocketAddress(socket.getRemoteSocketAddress());
            NetStream stream = new NetStream(
                streamId,
                localAddress,
                peerAddress,
                socket,
                null,
                false,
                socket.getInputStream(),
                socket.getOutputStream()
            );
            try {
                writeNetEvent("connect-production " + address + " " + streamId);
            } catch (java.io.IOException | RuntimeException error) {
                closeQuietly(socket);
                throw streamTransportFailure(
                    "connect", "event_record_failed", stream, "stream_connected",
                    Boolean.FALSE, Boolean.FALSE, true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return stream;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(socket);
            throw transportFailure(
                "connect", transportIoCategory(error), localAddress, peerAddress,
                null, streamId, connected ? "socket_connected" : "during_operation",
                Boolean.FALSE, Boolean.FALSE, Boolean.valueOf(connected),
                platformCause("production socket failed", error), error
            );
        }
    }

    private static Object netConnectProductionInMemoryListener(String address, NetListener listener) {
        try {
            int index = listener.recordSourceConnection();
            SourceLoopback loopback = new SourceLoopback();
            String clientStreamId = sourceProductionStreamId(index);
            String serverStreamId = productionStreamId(index, listener.productionStreamCount);
            listener.addSourceAcceptedStream(new NetStream(
                serverStreamId,
                address,
                loopbackPeerAddress(index),
                null,
                null,
                false,
                loopback.serverInput,
                loopback.serverOutput
            ));
            NetStream stream = new NetStream(
                clientStreamId,
                loopbackPeerAddress(index),
                address,
                null,
                null,
                false,
                loopback.clientInput,
                loopback.clientOutput
            );
            try {
                writeNetEvent("connect-production " + address + " " + clientStreamId);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "connect", "event_record_failed", stream, "stream_connected",
                    Boolean.FALSE, Boolean.FALSE, true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return stream;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.io.IOException | RuntimeException error) {
            throw transportFailure(
                "connect", transportIoCategory(error), null, address, null, null,
                "during_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                platformCause("production socket failed", error), error
            );
        }
    }

    private static Object netListenProduction(String address) {
        if (externalSocketRuntime()) {
            return netListenExternal(address);
        }
        java.util.List<java.util.List<byte[]>> inputs;
        try {
            inputs = loopbackClientReadStreams();
        } catch (IllegalArgumentException error) {
            throw transportFailure(
                "listen", "invalid_input", address, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
        if (productionReadInputsRequireChunkBoundaries(inputs)) {
            return netListenProductionInMemory(address, inputs);
        }
        java.net.InetSocketAddress socketAddress;
        try {
            socketAddress = parseSocketAddress(address);
        } catch (RuntimeException error) {
            throw transportFailure(
                "listen", "invalid_endpoint", address, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
        java.net.ServerSocket server = null;
        try {
            server = new java.net.ServerSocket();
            server.setReuseAddress(true);
            server.bind(socketAddress);
            server.setSoTimeout(productionTimeoutMillis());
            LoopbackClient client = null;
            if (loopbackClientEnabled()) {
                client = new LoopbackClient(
                    loopbackConnectHost(server),
                    server.getLocalPort(),
                    inputs
                );
            }
            NetListener listener = new NetListener(address, server, client);
            try {
                writeNetEvent("listen-production " + address);
            } catch (java.io.IOException | RuntimeException error) {
                closeQuietly(server);
                throw listenerTransportFailure(
                    "listen", "event_record_failed", listener, "listener_bound", true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            productionListenerBindings.put(
                address,
                new ProductionListenerBinding(
                    listener,
                    loopbackConnectHost(server),
                    server.getLocalPort()
                )
            );
            return listener;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(server);
            return netListenProductionInMemory(address);
        }
    }

    private static Object netListenExternal(String address) {
        java.net.InetSocketAddress socketAddress;
        try {
            socketAddress = parseSocketAddress(address);
        } catch (RuntimeException error) {
            throw transportFailure(
                "listen", "invalid_endpoint", address, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
        java.net.ServerSocket server = null;
        try {
            server = new java.net.ServerSocket();
            server.setReuseAddress(true);
            server.bind(socketAddress);
            server.setSoTimeout(productionTimeoutMillis());
            NetListener listener = new NetListener(address, server, null);
            try {
                writeNetEvent("listen-production " + address);
            } catch (java.io.IOException | RuntimeException error) {
                closeQuietly(server);
                throw listenerTransportFailure(
                    "listen", "event_record_failed", listener, "listener_bound", true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return listener;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(server);
            throw transportFailure(
                "listen", transportIoCategory(error), address, null, null, null,
                "during_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                platformCause("production socket failed", error), error
            );
        }
    }

    private static Object netListenProductionInMemory(String address) {
        try {
            return netListenProductionInMemory(address, loopbackClientReadStreams());
        } catch (IllegalArgumentException error) {
            throw transportFailure(
                "listen", "invalid_input", address, null, null, null,
                "before_operation", Boolean.FALSE, Boolean.FALSE, Boolean.FALSE,
                error.getMessage(), error
            );
        }
    }

    private static Object netListenProductionInMemory(
        String address,
        java.util.List<java.util.List<byte[]>> inputs
    ) {
        ProductionWriteCapture capture = new ProductionWriteCapture(inputs.size());
        java.util.ArrayList<InMemoryLoopback> loopbacks = new java.util.ArrayList<InMemoryLoopback>();
        for (int index = 0; index < inputs.size(); index += 1) {
            loopbacks.add(new InMemoryLoopback(inputs.get(index), index, capture));
        }
        try {
            writeNetEvent("listen-production " + address);
        } catch (java.io.IOException | RuntimeException error) {
            throw transportFailure(
                "listen", "event_record_failed", address, null, address, null,
                "listener_bound", Boolean.FALSE, Boolean.FALSE, Boolean.TRUE,
                "could not record VELN_NET_EVENTS", error
            );
        }
        NetListener listener = new NetListener(address, loopbacks);
        productionListenerBindings.put(
            address,
            new ProductionListenerBinding(listener, null, 0)
        );
        return listener;
    }

    public static Object netAccept(Object listener) {
        NetListener value = asNetListener(listener);
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw listenerTransportFailure(
                "accept", "forced_failure", value, "before_operation", false,
                "VELN_NET_ACCEPT_FAILED", null
            );
        }
        failIfListenerClosed(value);
        if (value.isProductionSocket()) {
            return acceptProduction(value, false);
        }
        String streamId = acceptedNetStreamIdOrNull();
        if (streamId == null) {
            throw acceptTransportFailure(
                "end_of_stream", value, null, "clean_end", false, "listener ended", null
            );
        }
        NetStream stream = new NetStream(streamId, value.address, streamId);
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw acceptTransportFailure(
                "event_record_failed", value, stream, "stream_accepted", true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return stream;
    }

    public static Object netAcceptOrEnd(Object listener) {
        NetListener value = asNetListener(listener);
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw listenerTransportFailure(
                "accept", "forced_failure", value, "before_operation", false,
                "VELN_NET_ACCEPT_FAILED", null
            );
        }
        failIfListenerClosed(value);
        if (value.isProductionSocket()) {
            NetStream stream = acceptProduction(value, true);
            return stream == null ? none() : some(stream);
        }
        String streamId = acceptedNetStreamIdOrNull();
        if (streamId == null) {
            try {
                writeNetEvent("accept-end " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "clean_end", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return none();
        }
        NetStream stream = new NetStream(streamId, value.address, streamId);
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw acceptTransportFailure(
                "event_record_failed", value, stream, "stream_accepted", true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return some(stream);
    }

    public static Object netAcceptUntil(Object listener, Object deadline) {
        NetListener value = asNetListener(listener);
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw listenerTransportFailure(
                "accept", "forced_failure", value, "before_operation", false,
                "VELN_NET_ACCEPT_FAILED", null
            );
        }
        failIfListenerClosed(value);
        if (acceptDeadlineExpired(deadline)) {
            try {
                writeNetEvent("accept-deadline " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "deadline_expired", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return none();
        }
        if (value.isProductionSocket()) {
            NetStream stream = acceptProduction(value, true);
            return stream == null ? none() : some(stream);
        }
        String streamId = acceptedNetStreamIdOrNull();
        if (streamId == null) {
            try {
                writeNetEvent("accept-end " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "clean_end", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return none();
        }
        NetStream stream = new NetStream(streamId, value.address, streamId);
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw acceptTransportFailure(
                "event_record_failed", value, stream, "stream_accepted", true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return some(stream);
    }

    public static Object netAcceptUntilCancellable(Object listener, Object deadline, Object token) {
        NetListener value = asNetListener(listener);
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw listenerTransportFailure(
                "accept", "forced_failure", value, "before_operation", false,
                "VELN_NET_ACCEPT_FAILED", null
            );
        }
        failIfListenerClosed(value);
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("accept-cancelled " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "cancelled", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return acceptOutcome("AcceptCancelled");
        }
        if (acceptDeadlineExpired(deadline)) {
            try {
                writeNetEvent("accept-deadline " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "deadline_expired", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return acceptOutcome("AcceptDeadlineExpired");
        }
        if (value.isProductionSocket()) {
            NetStream stream = acceptProduction(value, true);
            return stream == null
                ? acceptOutcome("AcceptEnd")
                : acceptOutcome("AcceptStream", stream);
        }
        String streamId = acceptedNetStreamIdOrNull();
        if (streamId == null) {
            try {
                writeNetEvent("accept-end " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", value, null, "clean_end", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return acceptOutcome("AcceptEnd");
        }
        NetStream stream = new NetStream(streamId, value.address, streamId);
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw acceptTransportFailure(
                "event_record_failed", value, stream, "stream_accepted", true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return acceptOutcome("AcceptStream", stream);
    }

    private static NetStream acceptProduction(NetListener listener, boolean optional) {
        failIfListenerClosed(listener);
        int index = listener.nextProductionAcceptIndex();
        if (index >= listener.productionStreamCount) {
            closeQuietly(listener.server);
            removeProductionListenerBinding(listener);
            try {
                writeNetEvent("accept-end-production " + listener.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", listener, null, "clean_end", false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            if (optional) {
                return null;
            }
            throw acceptTransportFailure(
                "end_of_stream", listener, null, "clean_end", false,
                "production listener ended", null
            );
        }
        String streamId = productionStreamId(index, listener.productionStreamCount);
        boolean last = index + 1 >= listener.productionStreamCount;
        NetStream sourceAcceptedStream = listener.pollSourceAcceptedStream();
        if (sourceAcceptedStream != null) {
            try {
                writeNetEvent("accept-production " + listener.address + " " + sourceAcceptedStream.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", listener, sourceAcceptedStream,
                    "stream_accepted", true, "could not record VELN_NET_EVENTS", error
                );
            }
            if (last) {
                removeProductionListenerBinding(listener);
            }
            return sourceAcceptedStream;
        }
        if (listener.inMemoryLoopbacks != null) {
            NetStream stream = new NetStream(
                streamId,
                listener.address,
                loopbackPeerAddress(index),
                listener.inMemoryLoopbacks.get(index),
                last
            );
            try {
                writeNetEvent("accept-production " + listener.address + " " + streamId);
            } catch (java.io.IOException | RuntimeException error) {
                throw acceptTransportFailure(
                    "event_record_failed", listener, stream, "stream_accepted", true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return stream;
        }
        java.net.Socket socket = null;
        NetStream stream = null;
        try {
            if (listener.loopbackClient != null && !listener.hasSourceConnection()) {
                listener.loopbackClient.start();
            }
            socket = listener.server.accept();
            socket.setSoTimeout(productionTimeoutMillis());
            if (last) {
                closeQuietly(listener.server);
                removeProductionListenerBinding(listener);
            }
            String localAddress = formatSocketAddress(socket.getLocalSocketAddress());
            String peerAddress = formatSocketAddress(socket.getRemoteSocketAddress());
            stream = new NetStream(
                streamId,
                localAddress,
                peerAddress,
                socket,
                listener.loopbackClient,
                last,
                socket.getInputStream(),
                socket.getOutputStream()
            );
            try {
                writeNetEvent("accept-production " + listener.address + " " + streamId);
            } catch (java.io.IOException | RuntimeException error) {
                closeQuietly(socket);
                throw acceptTransportFailure(
                    "event_record_failed", listener, stream, "stream_accepted", true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return stream;
        } catch (TransportFailure failure) {
            throw failure;
        } catch (java.net.SocketTimeoutException error) {
            closeQuietly(listener.server);
            throw acceptTransportFailure(
                "timed_out", listener, stream,
                stream == null ? "during_operation" : "stream_accepted",
                stream != null, platformCause("production socket timed out", error), error
            );
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(listener.server);
            closeQuietly(socket);
            throw acceptTransportFailure(
                transportIoCategory(error), listener, stream,
                stream == null ? "during_operation" : "stream_accepted",
                stream != null, platformCause("production socket failed", error), error
            );
        }
    }

    private static boolean acceptDeadlineExpired(Object deadline) {
        long target = asLong(deadline);
        return forced("VELN_NET_ACCEPT_DEADLINE_EXPIRED")
            || forced("VELN_TIME_DEADLINE_EXPIRED")
            || target <= System.nanoTime();
    }

    private static String acceptedNetStreamIdOrNull() {
        if (forced("VELN_NET_ACCEPT_END")) {
            return null;
        }
        String streamId = System.getenv("VELN_NET_ACCEPT_STREAM");
        if (streamId == null || streamId.isEmpty()) {
            streamId = "stream-1";
        }
        return streamId;
    }

    private static String connectedNetStreamId() {
        String streamId = System.getenv("VELN_NET_CONNECT_STREAM");
        if (streamId == null || streamId.isEmpty()) {
            streamId = "client-stream";
        }
        return streamId;
    }

    public static Object netReadChunk(Object stream) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_READ_FAILED")) {
            throw streamTransportFailure(
                "read", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_READ_FAILED", null
            );
        }
        failIfStreamClosed(value, "read");
        if (value.isProductionSocket()) {
            return readProductionChunk(value);
        }
        if (value.isReadShutdown()) {
            throw streamTransportFailure(
                "read", "closed", value, "read_shutdown",
                Boolean.FALSE, Boolean.FALSE, true, "stream read side is shut down", null
            );
        }
        String sourceName = "VELN_NET_READ_CHUNK_HEX";
        String input = nextNetReadChunkHex(value.id);
        if (System.getenv("VELN_NET_READ_CHUNKS_HEX") != null) {
            sourceName = "VELN_NET_READ_CHUNKS_HEX";
        }
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw invalidReadFixture(value, sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw readEventFailure(value, "input_consumed", true, error);
        }
        return unwrapOk(chunk);
    }

    public static Object netListenerLocalAddr(Object listener) {
        if (forced("VELN_NET_ADDR_FAILED")) {
            throw new RuntimeFailure("transport address failed: VELN_NET_ADDR_FAILED");
        }
        NetListener value = asNetListener(listener);
        return checkedListenerAddress(value.localAddress);
    }

    public static Object netStreamLocalAddr(Object stream) {
        if (forced("VELN_NET_ADDR_FAILED")) {
            throw new RuntimeFailure("transport address failed: VELN_NET_ADDR_FAILED");
        }
        NetStream value = asNetStream(stream);
        return checkedStreamAddress(value.localAddress, "local");
    }

    public static Object netStreamPeerAddr(Object stream) {
        if (forced("VELN_NET_ADDR_FAILED")) {
            throw new RuntimeFailure("transport address failed: VELN_NET_ADDR_FAILED");
        }
        NetStream value = asNetStream(stream);
        return checkedStreamAddress(value.peerAddress, "peer");
    }

    public static Object netStreamCanRead(Object stream) {
        NetStream value = asNetStream(stream);
        return Boolean.valueOf(!value.isClosed() && !value.isReadShutdown());
    }

    public static Object netStreamCanWrite(Object stream) {
        NetStream value = asNetStream(stream);
        return Boolean.valueOf(!value.isClosed() && !value.isWriteShutdown());
    }

    public static Object netStreamIsClosed(Object stream) {
        NetStream value = asNetStream(stream);
        return Boolean.valueOf(value.isClosed());
    }

    public static Object netReadChunkOrEnd(Object stream) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_READ_FAILED")) {
            throw streamTransportFailure(
                "read", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_READ_FAILED", null
            );
        }
        failIfStreamClosed(value, "read");
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null ? none() : some(chunk);
        }
        if (value.isReadShutdown()) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return none();
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return none();
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw invalidReadFixture(value, sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw readEventFailure(value, "input_consumed", true, error);
        }
        return some(unwrapOk(chunk));
    }

    public static Object netReadChunkUntil(Object stream, Object deadline) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_READ_FAILED")) {
            throw streamTransportFailure(
                "read", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_READ_FAILED", null
            );
        }
        failIfStreamClosed(value, "read");
        if (readDeadlineExpired(deadline)) {
            try {
                writeNetEvent("read-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "deadline_expired", false, error);
            }
            return none();
        }
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null ? none() : some(chunk);
        }
        if (value.isReadShutdown()) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return none();
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return none();
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw invalidReadFixture(value, sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw readEventFailure(value, "input_consumed", true, error);
        }
        return some(unwrapOk(chunk));
    }

    public static Object netReadChunkUntilCancellable(Object stream, Object deadline, Object token) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_READ_FAILED")) {
            throw streamTransportFailure(
                "read", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_READ_FAILED", null
            );
        }
        CancelToken cancelToken = asCancelToken(token);
        failIfStreamClosed(value, "read");
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("read-cancelled " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "cancelled", false, error);
            }
            return streamReadOutcome("ReadCancelled");
        }
        if (readDeadlineExpired(deadline)) {
            try {
                writeNetEvent("read-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "deadline_expired", false, error);
            }
            return streamReadOutcome("ReadDeadlineExpired");
        }
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null
                ? streamReadOutcome("ReadEnd")
                : streamReadOutcome("ReadChunk", chunk);
        }
        if (value.isReadShutdown()) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return streamReadOutcome("ReadEnd");
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(value, "clean_end", true, error);
            }
            return streamReadOutcome("ReadEnd");
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw invalidReadFixture(value, sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw readEventFailure(value, "input_consumed", true, error);
        }
        return streamReadOutcome("ReadChunk", unwrapOk(chunk));
    }

    private static boolean readDeadlineExpired(Object deadline) {
        long target = asLong(deadline);
        return forced("VELN_NET_READ_DEADLINE_EXPIRED")
            || forced("VELN_TIME_DEADLINE_EXPIRED")
            || target <= System.nanoTime();
    }

    private static String nextNetReadChunkHex(String streamId) {
        String input = nextNetReadChunkHexOrNull(streamId);
        if (input == null) {
            return "";
        }
        return input;
    }

    private static String nextNetReadChunkHexOrNull(String streamId) {
        String inputs = System.getenv("VELN_NET_READ_CHUNKS_HEX");
        if (inputs != null) {
            String[] parts = inputs.split(",", -1);
            int index = netReadIndexes
                .computeIfAbsent(streamId, key -> new java.util.concurrent.atomic.AtomicInteger())
                .getAndIncrement();
            if (index < parts.length) {
                return parts[index].trim();
            }
            return null;
        }
        String input = System.getenv("VELN_NET_READ_CHUNK_HEX");
        if (input == null) {
            return null;
        }
        return input;
    }

    public static Object netWriteChunk(Object stream, Object bytes) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw streamTransportFailure(
                "write", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_WRITE_FAILED", null
            );
        }
        failIfStreamWriteShutdown(value);
        if (value.isProductionSocket()) {
            writeProductionChunk(value, bytes);
            return UNIT;
        }
        try {
            writeNetEvent("write " + value.id + " " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "write", "event_record_failed", value, "during_operation",
                Boolean.FALSE, Boolean.FALSE, false,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return UNIT;
    }

    public static Object netWriteChunkUntil(Object stream, Object bytes, Object deadline) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw streamTransportFailure(
                "write", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_WRITE_FAILED", null
            );
        }
        failIfStreamWriteShutdown(value);
        if (writeDeadlineExpired(deadline)) {
            try {
                writeNetEvent("write-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "write", "event_record_failed", value, "deadline_expired",
                    Boolean.FALSE, Boolean.FALSE, false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return streamWriteOutcome("WriteDeadlineExpired");
        }
        netWriteChunk(stream, bytes);
        return streamWriteOutcome("WriteCompleted");
    }

    public static Object netWriteChunkUntilCancellable(Object stream, Object bytes, Object deadline, Object token) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw streamTransportFailure(
                "write", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_WRITE_FAILED", null
            );
        }
        failIfStreamWriteShutdown(value);
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("write-cancelled " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "write", "event_record_failed", value, "cancelled",
                    Boolean.FALSE, Boolean.FALSE, false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return streamWriteOutcome("WriteCancelled");
        }
        if (writeDeadlineExpired(deadline)) {
            try {
                writeNetEvent("write-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "write", "event_record_failed", value, "deadline_expired",
                    Boolean.FALSE, Boolean.FALSE, false,
                    "could not record VELN_NET_EVENTS", error
                );
            }
            return streamWriteOutcome("WriteDeadlineExpired");
        }
        netWriteChunk(stream, bytes);
        return streamWriteOutcome("WriteCompleted");
    }

    private static boolean writeDeadlineExpired(Object deadline) {
        long target = asLong(deadline);
        return forced("VELN_NET_WRITE_DEADLINE_EXPIRED")
            || forced("VELN_TIME_DEADLINE_EXPIRED")
            || target <= System.nanoTime();
    }

    public static Object netWriteChunks(Object stream, Object chunks) {
        NetStream value = asNetStream(stream);
        ListValue current = asListValue(chunks);
        long written = 0L;
        Long failAfter = writeChunksFailAfter();
        while (current.cons) {
            if (failAfter != null && written >= failAfter.longValue()) {
                throw streamTransportFailure(
                    "write", "forced_failure", value, "before_operation",
                    Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_WRITE_FAIL_AFTER", null
                );
            }
            netWriteChunk(stream, current.head);
            written += 1L;
            current = asListValue(current.tail);
        }
        return UNIT;
    }

    private static Long writeChunksFailAfter() {
        String value = System.getenv("VELN_NET_WRITE_FAIL_AFTER");
        if (value == null) {
            return null;
        }
        try {
            long count = Long.parseLong(value.trim());
            return Long.valueOf(count < 0L ? 0L : count);
        } catch (NumberFormatException error) {
            return Long.valueOf(0L);
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

    public static Object netShutdownWrite(Object stream) {
        NetStream value = asNetStream(stream);
        failIfStreamClosed(value, "shutdown_write");
        if (value.isProductionSocket()) {
            shutdownProductionWrite(value);
            return UNIT;
        }
        if (value.shutdownWriteOnce()) {
            try {
                writeNetEvent("shutdown-write " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "shutdown", "event_record_failed", value, "write_shutdown",
                    Boolean.FALSE, Boolean.FALSE, true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
        }
        return UNIT;
    }

    public static Object netShutdownRead(Object stream) {
        NetStream value = asNetStream(stream);
        failIfStreamClosed(value, "shutdown_read");
        if (value.isProductionSocket()) {
            shutdownProductionRead(value);
            return UNIT;
        }
        if (value.shutdownReadOnce()) {
            try {
                writeNetEvent("shutdown-read " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw streamTransportFailure(
                    "shutdown", "event_record_failed", value, "read_shutdown",
                    Boolean.FALSE, Boolean.FALSE, true,
                    "could not record VELN_NET_EVENTS", error
                );
            }
        }
        return UNIT;
    }

    public static Object netCloseStream(Object stream) {
        NetStream value = asNetStream(stream);
        if (forced("VELN_NET_CLOSE_FAILED")) {
            throw streamTransportFailure(
                "stream_close", "forced_failure", value, "before_operation",
                Boolean.FALSE, Boolean.FALSE, false, "VELN_NET_CLOSE_FAILED", null
            );
        }
        value.closeOnce();
        if (value.isProductionSocket()) {
            closeProductionStream(value);
            return UNIT;
        }
        try {
            writeNetEvent("close " + value.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "stream_close", "event_record_failed", value, "closed_handle",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return UNIT;
    }

    public static Object netCloseListener(Object listener) {
        NetListener value = asNetListener(listener);
        if (forced("VELN_NET_CLOSE_FAILED")) {
            throw listenerTransportFailure(
                "listener_close", "forced_failure", value, "before_operation", false,
                "VELN_NET_CLOSE_FAILED", null
            );
        }
        boolean production = value.isProductionSocket();
        if (value.closeOnce()) {
            if (value.server != null) {
                try {
                    value.server.close();
                } catch (java.io.IOException | RuntimeException error) {
                    throw listenerTransportFailure(
                        "listener_close", transportIoCategory(error), value,
                        "closed_handle", true,
                        platformCause("production socket failed", error), error
                    );
                }
            }
            value.releaseLoopbackListenerState();
            removeProductionListenerBinding(value);
        }
        try {
            writeNetEvent(
                (production ? "close-listener-production " : "close-listener ")
                    + value.address
            );
        } catch (java.io.IOException | RuntimeException error) {
            throw listenerTransportFailure(
                "listener_close", "event_record_failed", value, "closed_handle", true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        return UNIT;
    }

    private static void failIfListenerClosed(NetListener listener) {
        if (listener.isClosed()) {
            throw listenerTransportFailure(
                "accept", "closed", listener, "closed_handle", false, "listener closed", null
            );
        }
    }

    private static void failIfStreamClosed(NetStream stream, String operation) {
        if (stream.isClosed()) {
            String publicOperation = operation.startsWith("shutdown") ? "shutdown" : operation;
            throw streamTransportFailure(
                publicOperation, "closed", stream, "closed_handle",
                Boolean.FALSE, Boolean.FALSE, false, "stream closed", null
            );
        }
    }

    private static void failIfStreamWriteShutdown(NetStream stream) {
        failIfStreamClosed(stream, "write");
        if (stream.isWriteShutdown()) {
            throw streamTransportFailure(
                "write", "closed", stream, "write_shutdown",
                Boolean.FALSE, Boolean.FALSE, true, "stream write side is shut down", null
            );
        }
    }

    private static Object readProductionChunk(NetStream stream) {
        Object chunk = readProductionChunkOrNull(stream);
        if (chunk == null) {
            throw streamTransportFailure(
                "read", "end_of_stream", stream, "clean_end",
                Boolean.TRUE, Boolean.FALSE, false, "production stream ended", null
            );
        }
        return chunk;
    }

    private static Object readProductionChunkOrNull(NetStream stream) {
        failIfStreamClosed(stream, "read");
        if (stream.isReadShutdown()) {
            try {
                writeNetEvent("read-end-production " + stream.id);
                return null;
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(stream, "clean_end", true, error);
            }
        }
        byte[] buffer = new byte[4096];
        int count;
        try {
            count = stream.input.read(buffer);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "read", transportIoCategory(error), stream, "during_operation",
                null, Boolean.FALSE, false,
                platformCause("production socket failed", error), error
            );
        }
        if (count < 0) {
            try {
                writeNetEvent("read-end-production " + stream.id);
                return null;
            } catch (java.io.IOException | RuntimeException error) {
                throw readEventFailure(stream, "clean_end", true, error);
            }
        }
        Object chunk = byteChunkFromByteArray(buffer, count);
        try {
            writeNetEvent("read-production " + stream.id + " " + hexBytes(buffer, count));
        } catch (java.io.IOException | RuntimeException error) {
            throw readEventFailure(stream, "input_consumed", true, error);
        }
        return chunk;
    }

    private static void writeProductionChunk(NetStream stream, Object bytes) {
        failIfStreamWriteShutdown(stream);
        byte[] raw = byteChunkToByteArray(bytes);
        try {
            stream.output.write(raw);
            stream.output.flush();
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "write", transportIoCategory(error), stream, "during_operation",
                Boolean.FALSE, null, false,
                platformCause("production socket failed", error), error
            );
        }
        try {
            writeNetEvent("write-production " + stream.id + " " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "write", "event_record_failed", stream, "output_committed",
                Boolean.FALSE, Boolean.TRUE, false,
                "could not record VELN_NET_EVENTS", error
            );
        }
    }

    private static void shutdownProductionRead(NetStream stream) {
        if (!stream.shutdownReadOnce()) {
            return;
        }
        try {
            if (stream.socket != null) {
                stream.socket.shutdownInput();
            }
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "shutdown", transportIoCategory(error), stream, "read_shutdown",
                Boolean.FALSE, Boolean.FALSE, true,
                platformCause("production socket failed", error), error
            );
        }
        try {
            writeNetEvent("shutdown-read-production " + stream.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "shutdown", "event_record_failed", stream, "read_shutdown",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
    }

    private static void shutdownProductionWrite(NetStream stream) {
        if (!stream.shutdownWriteOnce()) {
            return;
        }
        try {
            stream.output.flush();
            if (stream.socket != null) {
                stream.socket.shutdownOutput();
            } else {
                stream.output.close();
            }
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "shutdown", transportIoCategory(error), stream, "write_shutdown",
                Boolean.FALSE, Boolean.FALSE, true,
                platformCause("production socket failed", error), error
            );
        }
        try {
            writeNetEvent("shutdown-write-production " + stream.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "shutdown", "event_record_failed", stream, "write_shutdown",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        if (stream.inMemoryLoopback != null) {
            try {
                stream.inMemoryLoopback.capture();
            } catch (RuntimeException error) {
                throw streamTransportFailure(
                    "shutdown", "io_failure", stream, "write_shutdown",
                    Boolean.FALSE, Boolean.FALSE, true,
                    platformCause("production capture failed", error), error
                );
            }
        }
    }

    private static void closeProductionStream(NetStream stream) {
        try {
            if (stream.socket != null) {
                stream.socket.close();
            } else {
                stream.output.close();
                stream.input.close();
            }
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "stream_close", transportIoCategory(error), stream, "closed_handle",
                Boolean.FALSE, Boolean.FALSE, true,
                platformCause("production socket failed", error), error
            );
        }
        try {
            writeNetEvent("close-production " + stream.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw streamTransportFailure(
                "stream_close", "event_record_failed", stream, "closed_handle",
                Boolean.FALSE, Boolean.FALSE, true,
                "could not record VELN_NET_EVENTS", error
            );
        }
        if (stream.inMemoryLoopback != null) {
            try {
                stream.inMemoryLoopback.capture();
            } catch (RuntimeException error) {
                throw streamTransportFailure(
                    "stream_close", "io_failure", stream, "closed_handle",
                    Boolean.FALSE, Boolean.FALSE, true,
                    platformCause("production capture failed", error), error
                );
            }
        }
        if (stream.loopbackClient != null && stream.lastProductionStream) {
            try {
                stream.loopbackClient.await();
            } catch (RuntimeException error) {
                throw streamTransportFailure(
                    "stream_close", "io_failure", stream, "closed_handle",
                    Boolean.FALSE, Boolean.FALSE, true,
                    platformCause("loopback client failed", error), error
                );
            }
        }
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

    public static Object timeMonotonicMs() {
        return Long.valueOf(System.nanoTime() / 1000000L);
    }

    public static Object timeTimeoutMs(Object milliseconds) {
        long value = asLong(milliseconds);
        String forcedExpiry = System.getenv("VELN_TIME_TIMEOUT_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            throw new RuntimeFailure("transport timeout expired: VELN_TIME_TIMEOUT_EXPIRED");
        }
        if (value <= 0L) {
            return UNIT;
        }
        try {
            Thread.sleep(value);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        }
        return UNIT;
    }

    public static Object timeDeadlineAfterMs(Object milliseconds) {
        long value = asLong(milliseconds);
        if (value <= 0L) {
            return Long.valueOf(System.nanoTime());
        }
        long delta;
        try {
            delta = Math.multiplyExact(value, 1000000L);
        } catch (ArithmeticException error) {
            delta = Long.MAX_VALUE;
        }
        long now = System.nanoTime();
        long deadline = now + delta;
        if (deadline < now) {
            deadline = Long.MAX_VALUE;
        }
        return Long.valueOf(deadline);
    }

    public static Object timeDeadlineAtMs(Object targetMilliseconds) {
        long target = asLong(targetMilliseconds);
        try {
            return Long.valueOf(Math.multiplyExact(target, 1000000L));
        } catch (ArithmeticException error) {
            return Long.valueOf(target >= 0L ? Long.MAX_VALUE : Long.MIN_VALUE);
        }
    }

    public static Object timeWaitUntil(Object deadline) {
        String forcedExpiry = System.getenv("VELN_TIME_DEADLINE_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            throw new RuntimeFailure("transport deadline expired: VELN_TIME_DEADLINE_EXPIRED");
        }
        long remaining = deadlineRemainingNanos(deadline);
        if (remaining <= 0L) {
            return UNIT;
        }
        try {
            long millis = remaining / 1000000L;
            int nanos = (int) (remaining % 1000000L);
            Thread.sleep(millis, nanos);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        }
        return UNIT;
    }

    private static long deadlineRemainingNanos(Object deadline) {
        long target = asLong(deadline);
        long now = System.nanoTime();
        if (target <= now) {
            return 0L;
        }
        try {
            return Math.subtractExact(target, now);
        } catch (ArithmeticException error) {
            return Long.MAX_VALUE;
        }
    }

    public static Object timeCancelToken() {
        return new CancelToken(false);
    }

    public static Object timeCancelOwner() {
        return new CancelOwner();
    }

    public static Object timeCancelTokenFrom(Object owner) {
        return asCancelOwner(owner).token;
    }

    public static Object timeCancelOwned(Object owner) {
        asCancelOwner(owner).token.cancelled = true;
        return UNIT;
    }

    public static Object timeCancel(Object token) {
        CancelToken cancelToken = asCancelToken(token);
        if (cancelToken.observerOnly) {
            throw new RuntimeFailure("cancellation token is observer-only");
        }
        cancelToken.cancelled = true;
        return UNIT;
    }

    public static Object timeIsCancelled(Object token) {
        return Boolean.valueOf(asCancelToken(token).cancelled);
    }

    public static Object timeIsCancelledOwner(Object owner) {
        return Boolean.valueOf(asCancelOwner(owner).token.cancelled);
    }

    public static Object timeWaitUntilCancellable(Object deadline, Object token) {
        CancelToken cancelToken = asCancelToken(token);
        failIfWaitCancelled(cancelToken);
        String forcedExpiry = System.getenv("VELN_TIME_DEADLINE_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            throw new RuntimeFailure("transport deadline expired: VELN_TIME_DEADLINE_EXPIRED");
        }
        long remaining = deadlineRemainingNanos(deadline);
        while (remaining > 0L) {
            failIfWaitCancelled(cancelToken);
            try {
                long slice = Math.min(remaining, 10000000L);
                long millis = slice / 1000000L;
                int nanos = (int) (slice % 1000000L);
                Thread.sleep(millis, nanos);
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                cancelToken.cancelled = true;
                throw new RuntimeFailure("transport wait cancelled: thread interrupted");
            }
            remaining = deadlineRemainingNanos(deadline);
        }
        failIfWaitCancelled(cancelToken);
        return UNIT;
    }

    public static Object timeWaitUntilCancellableOutcome(Object deadline, Object token) {
        Object forcedOutcome = forcedWaitOutcomeOrNull();
        if (forcedOutcome != null) {
            return forcedOutcome;
        }
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            return waitOutcome("WaitCancelled");
        }
        String forcedExpiry = System.getenv("VELN_TIME_DEADLINE_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            return waitOutcome("WaitDeadlineExpired");
        }
        long remaining = deadlineRemainingNanos(deadline);
        while (remaining > 0L) {
            if (waitCancelled(cancelToken)) {
                return waitOutcome("WaitCancelled");
            }
            try {
                long slice = Math.min(remaining, 10000000L);
                long millis = slice / 1000000L;
                int nanos = (int) (slice % 1000000L);
                Thread.sleep(millis, nanos);
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                cancelToken.cancelled = true;
                return waitOutcome("WaitCancelled");
            }
            remaining = deadlineRemainingNanos(deadline);
        }
        if (waitCancelled(cancelToken)) {
            return waitOutcome("WaitCancelled");
        }
        return waitOutcome("WaitCompleted");
    }

    private static boolean productionSocketRuntime() {
        String runtime = System.getenv("VELN_NET_RUNTIME");
        return "production-loopback".equals(runtime) || "external".equals(runtime);
    }

    private static boolean externalSocketRuntime() {
        return "external".equals(System.getenv("VELN_NET_RUNTIME"));
    }

    private static boolean loopbackClientEnabled() {
        return "production-loopback".equals(System.getenv("VELN_NET_RUNTIME"));
    }

    private static java.util.List<java.util.List<byte[]>> loopbackClientReadStreams() {
        String inputs = System.getenv("VELN_NET_PRODUCTION_READS_HEX");
        String sourceName = "VELN_NET_PRODUCTION_READS_HEX";
        String[] streamParts;
        if (inputs == null) {
            String input = System.getenv("VELN_NET_PRODUCTION_READ_HEX");
            sourceName = "VELN_NET_PRODUCTION_READ_HEX";
            streamParts = new String[] { input == null ? "" : input };
        } else {
            streamParts = inputs.split(",", -1);
            if (streamParts.length == 0) {
                streamParts = new String[] { "" };
            }
        }
        java.util.ArrayList<java.util.List<byte[]>> streams =
            new java.util.ArrayList<java.util.List<byte[]>>();
        for (String streamPart : streamParts) {
            streams.add(loopbackClientReadStreamChunks(streamPart, sourceName));
        }
        return streams;
    }

    private static java.util.List<byte[]> loopbackClientReadStreamChunks(
        String streamPart,
        String sourceName
    ) {
        java.util.ArrayList<byte[]> chunks = new java.util.ArrayList<byte[]>();
        String trimmedStream = streamPart.trim();
        if (trimmedStream.isEmpty()) {
            return chunks;
        }
        String[] chunkParts = trimmedStream.split(";", -1);
        for (String chunkPart : chunkParts) {
            Object chunk = byteChunkFromHex(chunkPart.trim());
            if (isErr(chunk)) {
                throw new IllegalArgumentException("malformed " + sourceName);
            }
            chunks.add(byteChunkToByteArray(unwrapOk(chunk)));
        }
        return chunks;
    }

    private static boolean productionReadInputsRequireChunkBoundaries(
        java.util.List<java.util.List<byte[]>> inputs
    ) {
        for (java.util.List<byte[]> stream : inputs) {
            if (stream.size() > 1) {
                return true;
            }
        }
        return false;
    }

    private static int productionTimeoutMillis() {
        String value = System.getenv("VELN_NET_PRODUCTION_TIMEOUT_MS");
        if (value == null || value.isEmpty()) {
            return 5000;
        }
        try {
            int parsed = Integer.parseInt(value);
            return parsed <= 0 ? 5000 : parsed;
        } catch (NumberFormatException error) {
            return 5000;
        }
    }

    private static java.net.InetSocketAddress parseSocketAddress(String address) {
        int separator = address.lastIndexOf(':');
        if (separator <= 0 || separator == address.length() - 1) {
            throw new IllegalArgumentException("missing host or port");
        }
        String host = address.substring(0, separator);
        int port;
        try {
            port = Integer.parseInt(address.substring(separator + 1));
        } catch (NumberFormatException error) {
            throw new IllegalArgumentException("invalid port", error);
        }
        if (port < 0 || port > 65535) {
            throw new IllegalArgumentException("port out of range");
        }
        return new java.net.InetSocketAddress(host, port);
    }

    private static String loopbackConnectHost(java.net.ServerSocket server) {
        java.net.InetAddress address = server.getInetAddress();
        if (address == null || address.isAnyLocalAddress()) {
            return "127.0.0.1";
        }
        return address.getHostAddress();
    }

    private static String productionStreamId(int index, int count) {
        if (count <= 1) {
            return "production-stream";
        }
        return "production-stream-" + (index + 1);
    }

    private static String sourceProductionStreamId(int index) {
        if (index <= 0) {
            return "production-client-stream";
        }
        return "production-client-stream-" + (index + 1);
    }

    private static void removeProductionListenerBinding(NetListener listener) {
        ProductionListenerBinding binding = productionListenerBindings.get(listener.address);
        if (binding != null && binding.listener == listener) {
            productionListenerBindings.remove(listener.address, binding);
        }
    }

    private static String loopbackPeerAddress(int index) {
        return "127.0.0.1:" + (index + 1);
    }

    private static String checkedStreamAddress(String address, String side) {
        if (address == null || address.isEmpty()) {
            throw new RuntimeFailure("transport address failed: missing " + side + " stream address");
        }
        return address;
    }

    private static String checkedListenerAddress(String address) {
        if (address == null || address.isEmpty()) {
            throw new RuntimeFailure("transport address failed: missing listener address");
        }
        return address;
    }

    private static String formatSocketAddress(java.net.SocketAddress value) {
        if (value instanceof java.net.InetSocketAddress) {
            java.net.InetSocketAddress address = (java.net.InetSocketAddress) value;
            java.net.InetAddress inetAddress = address.getAddress();
            String host = inetAddress == null ? address.getHostString() : inetAddress.getHostAddress();
            return host + ":" + address.getPort();
        }
        return value == null ? null : value.toString();
    }

    private static byte[] byteChunkToByteArray(Object chunk) {
        java.util.List<Object> values = byteChunkBytes(chunk);
        byte[] bytes = new byte[values.size()];
        for (int index = 0; index < values.size(); index += 1) {
            bytes[index] = (byte) byteValueLong(values.get(index));
        }
        return bytes;
    }

    private static Object byteChunkFromByteArray(byte[] bytes, int count) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>();
        for (int index = 0; index < count; index += 1) {
            values.add(byteObject(bytes[index] & 0xff));
        }
        return byteChunkObject(values);
    }

    private static byte[] readAllBytes(java.io.InputStream input) throws java.io.IOException {
        java.io.ByteArrayOutputStream out = new java.io.ByteArrayOutputStream();
        byte[] buffer = new byte[4096];
        while (true) {
            int count = input.read(buffer);
            if (count < 0) {
                return out.toByteArray();
            }
            out.write(buffer, 0, count);
        }
    }

    private static String hexBytes(byte[] bytes) {
        return hexBytes(bytes, bytes.length);
    }

    private static String hexBytes(byte[] bytes, int count) {
        char[] digits = "0123456789abcdef".toCharArray();
        char[] encoded = new char[count * 2];
        for (int index = 0; index < count; index += 1) {
            int value = bytes[index] & 0xff;
            encoded[index * 2] = digits[value >>> 4];
            encoded[index * 2 + 1] = digits[value & 0x0f];
        }
        return new String(encoded);
    }

    private static String joinedHexBytes(byte[][] byteGroups) {
        StringBuilder builder = new StringBuilder();
        for (int index = 0; index < byteGroups.length; index += 1) {
            if (index > 0) {
                builder.append('\n');
            }
            builder.append(hexBytes(byteGroups[index]));
        }
        return builder.toString();
    }

    private static void closeQuietly(java.net.ServerSocket server) {
        if (server == null) {
            return;
        }
        try {
            server.close();
        } catch (java.io.IOException error) {
        }
    }

    private static void closeQuietly(java.net.Socket socket) {
        if (socket == null) {
            return;
        }
        try {
            socket.close();
        } catch (java.io.IOException error) {
        }
    }

    private static void writeNetEvent(String line) throws java.io.IOException {
        String path = System.getenv("VELN_NET_EVENTS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String failedPrefix = System.getenv("VELN_NET_EVENT_RECORD_FAILED_PREFIX");
        if (failedPrefix != null && !failedPrefix.isEmpty() && line.startsWith(failedPrefix)) {
            throw new java.io.IOException("VELN_NET_EVENT_RECORD_FAILED_PREFIX");
        }
        java.nio.file.Files.write(
            java.nio.file.Paths.get(path),
            java.util.Collections.singletonList(line),
            java.nio.charset.StandardCharsets.UTF_8,
            java.nio.file.StandardOpenOption.CREATE,
            java.nio.file.StandardOpenOption.APPEND
        );
    }

    private static boolean forced(String name) {
        return "1".equals(System.getenv(name));
    }

    private static void failIfWaitCancelled(CancelToken token) {
        if (forced("VELN_TIME_WAIT_CANCELLED")) {
            token.cancelled = true;
            throw new RuntimeFailure("transport wait cancelled: VELN_TIME_WAIT_CANCELLED");
        }
        if (token.cancelled) {
            throw new RuntimeFailure("transport wait cancelled: cancellation token");
        }
    }

    private static boolean waitCancelled(CancelToken token) {
        if (forced("VELN_TIME_WAIT_CANCELLED")) {
            token.cancelled = true;
        }
        return token.cancelled;
    }

    private static Object forcedWaitOutcomeOrNull() {
        String plan = System.getenv("VELN_TIME_CANCELLABLE_OUTCOMES");
        if (plan == null || plan.isEmpty()) {
            return null;
        }
        String[] outcomes = plan.split(",");
        int index = waitOutcomeIndex.getAndIncrement();
        if (index >= outcomes.length) {
            return null;
        }
        String outcome = outcomes[index].trim();
        if ("completed".equals(outcome) || "WaitCompleted".equals(outcome)) {
            return waitOutcome("WaitCompleted");
        }
        if (
            "deadline-expired".equals(outcome)
                || "deadline".equals(outcome)
                || "WaitDeadlineExpired".equals(outcome)
        ) {
            return waitOutcome("WaitDeadlineExpired");
        }
        if ("cancelled".equals(outcome) || "WaitCancelled".equals(outcome)) {
            return waitOutcome("WaitCancelled");
        }
        throw new RuntimeFailure(
            "transport wait outcome fixture invalid: VELN_TIME_CANCELLABLE_OUTCOMES"
        );
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

    private static NetListener asNetListener(Object value) {
        return (NetListener) value;
    }

    private static NetStream asNetStream(Object value) {
        return (NetStream) value;
    }

    private static CancelToken asCancelToken(Object value) {
        return (CancelToken) value;
    }

    private static CancelOwner asCancelOwner(Object value) {
        return (CancelOwner) value;
    }
