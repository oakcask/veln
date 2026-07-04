    private static java.util.List<Object> processArgs =
        java.util.Collections.unmodifiableList(new java.util.ArrayList<Object>());

    public static void setProcessArgs(String[] args) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>();
        for (String arg : args) {
            values.add(arg);
        }
        processArgs = freezeList(values);
    }

    private static final class NetListener {
        private final String address;
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
            this(address, false, null, null, null, 0);
        }

        private NetListener(String address, java.net.ServerSocket server, LoopbackClient loopbackClient) {
            this(
                address,
                true,
                server,
                loopbackClient,
                null,
                loopbackClient == null ? 1 : loopbackClient.streamCount()
            );
        }

        private NetListener(String address, java.util.List<InMemoryLoopback> inMemoryLoopbacks) {
            this(address, true, null, null, inMemoryLoopbacks, inMemoryLoopbacks.size());
        }

        private NetListener(
            String address,
            boolean productionSocket,
            java.net.ServerSocket server,
            LoopbackClient loopbackClient,
            java.util.List<InMemoryLoopback> inMemoryLoopbacks,
            int productionStreamCount
        ) {
            this.address = address;
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
        private final java.util.concurrent.atomic.AtomicBoolean writeShutdown =
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
                throw new RuntimeFailure("transport close failed: could not capture loopback writes", error);
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
            for (int index = 0; index < outboundStreams.size(); index += 1) {
                java.util.List<byte[]> outboundChunks = outboundStreams.get(index);
                try (java.net.Socket socket = new java.net.Socket()) {
                    socket.connect(new java.net.InetSocketAddress(host, port), productionTimeoutMillis());
                    java.io.OutputStream output = socket.getOutputStream();
                    for (byte[] outbound : outboundChunks) {
                        output.write(outbound);
                        output.flush();
                    }
                    socket.shutdownOutput();
                    capture.record(index, readAllBytes(socket.getInputStream()));
                } catch (java.io.IOException error) {
                    throw new RuntimeException(error);
                }
            }
        }

        private void await() {
            if (thread != null) {
                try {
                    thread.join(productionTimeoutMillis());
                } catch (InterruptedException error) {
                    Thread.currentThread().interrupt();
                    throw new RuntimeFailure("transport close failed: loopback client interrupted", error);
                }
                if (thread.isAlive()) {
                    throw new RuntimeFailure("transport close failed: loopback client timed out");
                }
            }
            if (failure != null) {
                throw new RuntimeFailure("transport close failed: loopback client failed", failure);
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
        if (forced("VELN_NET_LISTEN_FAILED")) {
            throw new RuntimeFailure("transport listen failed: VELN_NET_LISTEN_FAILED");
        }
        String value = asString(address);
        if (productionSocketRuntime()) {
            return netListenProduction(value);
        }
        try {
            writeNetEvent("listen " + value);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport listen failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return new NetListener(value);
    }

    public static Object netConnect(Object address) {
        if (forced("VELN_NET_CONNECT_FAILED")) {
            throw new RuntimeFailure("transport connect failed: VELN_NET_CONNECT_FAILED");
        }
        String value = asString(address);
        if (productionSocketRuntime()) {
            return netConnectProduction(value);
        }
        String streamId = connectedNetStreamId();
        try {
            writeNetEvent("connect " + value + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport connect failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return new NetStream(streamId, streamId, value);
    }

    private static Object netConnectProduction(String address) {
        ProductionListenerBinding binding = productionListenerBindings.get(address);
        if (binding != null && !binding.listener.isClosed()) {
            return netConnectProductionListener(address, binding);
        }
        java.util.List<java.util.List<byte[]>> inputs = loopbackClientReadStreams();
        if (inputs.isEmpty()) {
            inputs = java.util.Collections.singletonList(
                java.util.Collections.<byte[]>emptyList()
            );
        }
        ProductionWriteCapture capture = new ProductionWriteCapture(1);
        InMemoryLoopback loopback = new InMemoryLoopback(inputs.get(0), 0, capture);
        String streamId = productionStreamId(0, 1);
        try {
            writeNetEvent("connect-production " + address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport connect failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return new NetStream(streamId, loopbackPeerAddress(0), address, loopback, true);
    }

    private static Object netConnectProductionListener(
        String address,
        ProductionListenerBinding binding
    ) {
        if (binding.host == null) {
            return netConnectProductionInMemoryListener(address, binding.listener);
        }
        java.net.Socket socket = null;
        try {
            socket = new java.net.Socket();
            socket.connect(
                new java.net.InetSocketAddress(binding.host, binding.port),
                productionTimeoutMillis()
            );
            socket.setSoTimeout(productionTimeoutMillis());
            int index = binding.listener.recordSourceConnection();
            String streamId = sourceProductionStreamId(index);
            writeNetEvent("connect-production " + address + " " + streamId);
            String localAddress = formatSocketAddress(socket.getLocalSocketAddress());
            String peerAddress = formatSocketAddress(socket.getRemoteSocketAddress());
            return new NetStream(
                streamId,
                localAddress,
                peerAddress,
                socket,
                null,
                false,
                socket.getInputStream(),
                socket.getOutputStream()
            );
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(socket);
            throw new RuntimeFailure("transport connect failed: production socket failed", error);
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
            writeNetEvent("connect-production " + address + " " + clientStreamId);
            return new NetStream(
                clientStreamId,
                loopbackPeerAddress(index),
                address,
                null,
                null,
                false,
                loopback.clientInput,
                loopback.clientOutput
            );
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure("transport connect failed: production socket failed", error);
        }
    }

    private static Object netListenProduction(String address) {
        java.util.List<java.util.List<byte[]>> inputs = loopbackClientReadStreams();
        if (productionReadInputsRequireChunkBoundaries(inputs)) {
            return netListenProductionInMemory(address, inputs);
        }
        java.net.InetSocketAddress socketAddress;
        try {
            socketAddress = parseSocketAddress(address);
        } catch (RuntimeException error) {
            throw new RuntimeFailure("transport listen failed: invalid socket address", error);
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
            writeNetEvent("listen-production " + address);
            NetListener listener = new NetListener(address, server, client);
            productionListenerBindings.put(
                address,
                new ProductionListenerBinding(
                    listener,
                    loopbackConnectHost(server),
                    server.getLocalPort()
                )
            );
            return listener;
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(server);
            return netListenProductionInMemory(address);
        }
    }

    private static Object netListenProductionInMemory(String address) {
        return netListenProductionInMemory(address, loopbackClientReadStreams());
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
            throw new RuntimeFailure(
                "transport listen failed: could not record VELN_NET_EVENTS",
                error
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
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
        failIfListenerClosed(value);
        if (value.isProductionSocket()) {
            return acceptProduction(value, false);
        }
        String streamId = acceptedNetStreamIdOrNull();
        if (streamId == null) {
            throw new RuntimeFailure("transport accept failed: listener ended");
        }
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport accept failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return new NetStream(streamId, value.address, streamId);
    }

    public static Object netAcceptOrEnd(Object listener) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
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
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return none();
        }
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport accept failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return some(new NetStream(streamId, value.address, streamId));
    }

    public static Object netAcceptUntil(Object listener, Object deadline) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
        failIfListenerClosed(value);
        if (acceptDeadlineExpired(deadline)) {
            try {
                writeNetEvent("accept-deadline " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
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
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return none();
        }
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport accept failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return some(new NetStream(streamId, value.address, streamId));
    }

    public static Object netAcceptUntilCancellable(Object listener, Object deadline, Object token) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
        failIfListenerClosed(value);
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("accept-cancelled " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return acceptOutcome("AcceptCancelled");
        }
        if (acceptDeadlineExpired(deadline)) {
            try {
                writeNetEvent("accept-deadline " + value.address);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
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
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return acceptOutcome("AcceptEnd");
        }
        NetStream stream = new NetStream(streamId, value.address, streamId);
        try {
            writeNetEvent("accept " + value.address + " " + streamId);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport accept failed: could not record VELN_NET_EVENTS",
                error
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
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            if (optional) {
                return null;
            }
            throw new RuntimeFailure("transport accept failed: production listener ended");
        }
        String streamId = productionStreamId(index, listener.productionStreamCount);
        boolean last = index + 1 >= listener.productionStreamCount;
        NetStream sourceAcceptedStream = listener.pollSourceAcceptedStream();
        if (sourceAcceptedStream != null) {
            try {
                writeNetEvent("accept-production " + listener.address + " " + sourceAcceptedStream.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            if (last) {
                removeProductionListenerBinding(listener);
            }
            return sourceAcceptedStream;
        }
        if (listener.inMemoryLoopbacks != null) {
            try {
                writeNetEvent("accept-production " + listener.address + " " + streamId);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport accept failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return new NetStream(
                streamId,
                listener.address,
                loopbackPeerAddress(index),
                listener.inMemoryLoopbacks.get(index),
                last
            );
        }
        try {
            if (listener.loopbackClient != null && !listener.hasSourceConnection()) {
                listener.loopbackClient.start();
            }
            java.net.Socket socket = listener.server.accept();
            socket.setSoTimeout(productionTimeoutMillis());
            if (last) {
                closeQuietly(listener.server);
                removeProductionListenerBinding(listener);
            }
            writeNetEvent("accept-production " + listener.address + " " + streamId);
            String localAddress = formatSocketAddress(socket.getLocalSocketAddress());
            String peerAddress = formatSocketAddress(socket.getRemoteSocketAddress());
            return new NetStream(
                streamId,
                localAddress,
                peerAddress,
                socket,
                listener.loopbackClient,
                last,
                socket.getInputStream(),
                socket.getOutputStream()
            );
        } catch (java.net.SocketTimeoutException error) {
            if (optional) {
                closeQuietly(listener.server);
                throw new RuntimeFailure("transport accept failed: production socket timed out", error);
            }
            closeQuietly(listener.server);
            throw new RuntimeFailure("transport accept failed: production socket timed out", error);
        } catch (java.io.IOException | RuntimeException error) {
            closeQuietly(listener.server);
            throw new RuntimeFailure("transport accept failed: production socket failed", error);
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
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
        if (value.isProductionSocket()) {
            return readProductionChunk(value);
        }
        String sourceName = "VELN_NET_READ_CHUNK_HEX";
        String input = nextNetReadChunkHex(value.id);
        if (System.getenv("VELN_NET_READ_CHUNKS_HEX") != null) {
            sourceName = "VELN_NET_READ_CHUNKS_HEX";
        }
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport read failed: malformed " + sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport read failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return unwrapOk(chunk);
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

    public static Object netReadChunkOrEnd(Object stream) {
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null ? none() : some(chunk);
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return none();
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport read failed: malformed " + sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport read failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return some(unwrapOk(chunk));
    }

    public static Object netReadChunkUntil(Object stream, Object deadline) {
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
        if (readDeadlineExpired(deadline)) {
            try {
                writeNetEvent("read-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return none();
        }
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null ? none() : some(chunk);
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return none();
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport read failed: malformed " + sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport read failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return some(unwrapOk(chunk));
    }

    public static Object netReadChunkUntilCancellable(Object stream, Object deadline, Object token) {
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("read-cancelled " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return streamReadOutcome("ReadCancelled");
        }
        if (readDeadlineExpired(deadline)) {
            try {
                writeNetEvent("read-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return streamReadOutcome("ReadDeadlineExpired");
        }
        if (value.isProductionSocket()) {
            Object chunk = readProductionChunkOrNull(value);
            return chunk == null
                ? streamReadOutcome("ReadEnd")
                : streamReadOutcome("ReadChunk", chunk);
        }
        String input = nextNetReadChunkHexOrNull(value.id);
        if (input == null) {
            try {
                writeNetEvent("read-end " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport read failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return streamReadOutcome("ReadEnd");
        }
        String sourceName = System.getenv("VELN_NET_READ_CHUNKS_HEX") != null
            ? "VELN_NET_READ_CHUNKS_HEX"
            : "VELN_NET_READ_CHUNK_HEX";
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport read failed: malformed " + sourceName);
        }
        try {
            writeNetEvent("read " + value.id + " " + input);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport read failed: could not record VELN_NET_EVENTS",
                error
            );
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
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw new RuntimeFailure("transport write failed: VELN_NET_WRITE_FAILED");
        }
        NetStream value = asNetStream(stream);
        failIfStreamWriteShutdown(value);
        if (value.isProductionSocket()) {
            writeProductionChunk(value, bytes);
            return UNIT;
        }
        try {
            writeNetEvent("write " + value.id + " " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport write failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return UNIT;
    }

    public static Object netWriteChunkUntil(Object stream, Object bytes, Object deadline) {
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw new RuntimeFailure("transport write failed: VELN_NET_WRITE_FAILED");
        }
        NetStream value = asNetStream(stream);
        failIfStreamWriteShutdown(value);
        if (writeDeadlineExpired(deadline)) {
            try {
                writeNetEvent("write-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport write failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return streamWriteOutcome("WriteDeadlineExpired");
        }
        netWriteChunk(stream, bytes);
        return streamWriteOutcome("WriteCompleted");
    }

    public static Object netWriteChunkUntilCancellable(Object stream, Object bytes, Object deadline, Object token) {
        if (forced("VELN_NET_WRITE_FAILED")) {
            throw new RuntimeFailure("transport write failed: VELN_NET_WRITE_FAILED");
        }
        NetStream value = asNetStream(stream);
        failIfStreamWriteShutdown(value);
        CancelToken cancelToken = asCancelToken(token);
        if (waitCancelled(cancelToken)) {
            try {
                writeNetEvent("write-cancelled " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport write failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
            return streamWriteOutcome("WriteCancelled");
        }
        if (writeDeadlineExpired(deadline)) {
            try {
                writeNetEvent("write-deadline " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport write failed: could not record VELN_NET_EVENTS",
                    error
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
        ListValue current = asListValue(chunks);
        while (current.cons) {
            netWriteChunk(stream, current.head);
            current = asListValue(current.tail);
        }
        return UNIT;
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
        if (value.isProductionSocket()) {
            shutdownProductionWrite(value);
            return UNIT;
        }
        if (value.shutdownWriteOnce()) {
            try {
                writeNetEvent("shutdown-write " + value.id);
            } catch (java.io.IOException | RuntimeException error) {
                throw new RuntimeFailure(
                    "transport shutdown_write failed: could not record VELN_NET_EVENTS",
                    error
                );
            }
        }
        return UNIT;
    }

    public static Object netCloseStream(Object stream) {
        if (forced("VELN_NET_CLOSE_FAILED")) {
            throw new RuntimeFailure("transport close failed: VELN_NET_CLOSE_FAILED");
        }
        NetStream value = asNetStream(stream);
        if (value.isProductionSocket()) {
            closeProductionStream(value);
            return UNIT;
        }
        try {
            writeNetEvent("close " + value.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport close failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return UNIT;
    }

    public static Object netCloseListener(Object listener) {
        if (forced("VELN_NET_CLOSE_FAILED")) {
            throw new RuntimeFailure("transport listener close failed: VELN_NET_CLOSE_FAILED");
        }
        NetListener value = asNetListener(listener);
        boolean production = value.isProductionSocket();
        if (value.closeOnce()) {
            if (value.server != null) {
                try {
                    value.server.close();
                } catch (java.io.IOException | RuntimeException error) {
                    throw new RuntimeFailure(
                        "transport listener close failed: production socket failed",
                        error
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
            throw new RuntimeFailure(
                "transport listener close failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return UNIT;
    }

    private static void failIfListenerClosed(NetListener listener) {
        if (listener.isClosed()) {
            throw new RuntimeFailure("transport accept failed: listener closed");
        }
    }

    private static void failIfStreamWriteShutdown(NetStream stream) {
        if (stream.isWriteShutdown()) {
            throw new RuntimeFailure("transport write failed: stream write side is shut down");
        }
    }

    private static Object readProductionChunk(NetStream stream) {
        Object chunk = readProductionChunkOrNull(stream);
        if (chunk == null) {
            throw new RuntimeFailure("transport read failed: production stream ended");
        }
        return chunk;
    }

    private static Object readProductionChunkOrNull(NetStream stream) {
        byte[] buffer = new byte[4096];
        try {
            int count = stream.input.read(buffer);
            if (count < 0) {
                writeNetEvent("read-end-production " + stream.id);
                return null;
            }
            Object chunk = byteChunkFromByteArray(buffer, count);
            writeNetEvent("read-production " + stream.id + " " + hexBytes(buffer, count));
            return chunk;
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure("transport read failed: production socket failed", error);
        }
    }

    private static void writeProductionChunk(NetStream stream, Object bytes) {
        failIfStreamWriteShutdown(stream);
        try {
            byte[] raw = byteChunkToByteArray(bytes);
            stream.output.write(raw);
            stream.output.flush();
            writeNetEvent("write-production " + stream.id + " " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure("transport write failed: production socket failed", error);
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
            writeNetEvent("shutdown-write-production " + stream.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure("transport shutdown_write failed: production socket failed", error);
        }
        if (stream.inMemoryLoopback != null) {
            stream.inMemoryLoopback.capture();
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
            writeNetEvent("close-production " + stream.id);
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure("transport close failed: production socket failed", error);
        }
        if (stream.inMemoryLoopback != null) {
            stream.inMemoryLoopback.capture();
        }
        if (stream.loopbackClient != null && stream.lastProductionStream) {
            stream.loopbackClient.await();
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
        return "production-loopback".equals(System.getenv("VELN_NET_RUNTIME"));
    }

    private static boolean loopbackClientEnabled() {
        return productionSocketRuntime();
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
                throw new RuntimeFailure("transport listen failed: malformed " + sourceName);
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
