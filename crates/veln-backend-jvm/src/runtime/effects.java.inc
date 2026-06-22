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

        private NetListener(String address) {
            this.address = address;
        }

        @Override
        public String toString() {
            return "NetListener(" + address + ")";
        }
    }

    private static final class NetStream {
        private final String id;

        private NetStream(String id) {
            this.id = id;
        }

        @Override
        public String toString() {
            return "NetStream(" + id + ")";
        }
    }

    private static final java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.atomic.AtomicInteger>
        netReadIndexes = new java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.atomic.AtomicInteger>();
    private static final java.util.concurrent.atomic.AtomicInteger waitOutcomeIndex =
        new java.util.concurrent.atomic.AtomicInteger(0);

    private static final class CancelToken {
        private volatile boolean cancelled;

        @Override
        public String toString() {
            return cancelled ? "CancelToken(cancelled)" : "CancelToken(active)";
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

    public static Object netAccept(Object listener) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
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
        return new NetStream(streamId);
    }

    public static Object netAcceptOrEnd(Object listener) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
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
        return some(new NetStream(streamId));
    }

    public static Object netAcceptUntil(Object listener, Object deadline) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
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
        return some(new NetStream(streamId));
    }

    public static Object netAcceptUntilCancellable(Object listener, Object deadline, Object token) {
        if (forced("VELN_NET_ACCEPT_FAILED")) {
            throw new RuntimeFailure("transport accept failed: VELN_NET_ACCEPT_FAILED");
        }
        NetListener value = asNetListener(listener);
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
        NetStream stream = new NetStream(streamId);
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

    public static Object netReadChunk(Object stream) {
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
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

    public static Object netReadChunkOrEnd(Object stream) {
        if (forced("VELN_NET_READ_FAILED")) {
            throw new RuntimeFailure("transport read failed: VELN_NET_READ_FAILED");
        }
        NetStream value = asNetStream(stream);
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

    public static Object netCloseStream(Object stream) {
        if (forced("VELN_NET_CLOSE_FAILED")) {
            throw new RuntimeFailure("transport close failed: VELN_NET_CLOSE_FAILED");
        }
        NetStream value = asNetStream(stream);
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

    public static Object timeWaitUntil(Object deadline) {
        String forcedExpiry = System.getenv("VELN_TIME_DEADLINE_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            throw new RuntimeFailure("transport deadline expired: VELN_TIME_DEADLINE_EXPIRED");
        }
        long target = asLong(deadline);
        long remaining = target - System.nanoTime();
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

    public static Object timeCancelToken() {
        return new CancelToken();
    }

    public static Object timeCancel(Object token) {
        asCancelToken(token).cancelled = true;
        return UNIT;
    }

    public static Object timeIsCancelled(Object token) {
        return Boolean.valueOf(asCancelToken(token).cancelled);
    }

    public static Object timeWaitUntilCancellable(Object deadline, Object token) {
        CancelToken cancelToken = asCancelToken(token);
        failIfWaitCancelled(cancelToken);
        String forcedExpiry = System.getenv("VELN_TIME_DEADLINE_EXPIRED");
        if ("1".equals(forcedExpiry)) {
            throw new RuntimeFailure("transport deadline expired: VELN_TIME_DEADLINE_EXPIRED");
        }
        long target = asLong(deadline);
        long remaining = target - System.nanoTime();
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
            remaining = target - System.nanoTime();
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
        long target = asLong(deadline);
        long remaining = target - System.nanoTime();
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
            remaining = target - System.nanoTime();
        }
        if (waitCancelled(cancelToken)) {
            return waitOutcome("WaitCancelled");
        }
        return waitOutcome("WaitCompleted");
    }

    private static void writeNetEvent(String line) throws java.io.IOException {
        String path = System.getenv("VELN_NET_EVENTS");
        if (path == null || path.isEmpty()) {
            return;
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

    private static NetListener asNetListener(Object value) {
        return (NetListener) value;
    }

    private static NetStream asNetStream(Object value) {
        return (NetStream) value;
    }

    private static CancelToken asCancelToken(Object value) {
        return (CancelToken) value;
    }
