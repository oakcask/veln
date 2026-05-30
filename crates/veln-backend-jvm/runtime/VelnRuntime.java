public final class VelnRuntime {
    public static final Unit UNIT = new Unit();

    private VelnRuntime() {}

    public static final class Unit {
        private Unit() {}

        @Override
        public String toString() {
            return "()";
        }
    }

    public static final class PathValue {
        private final java.nio.file.Path value;

        private PathValue(java.nio.file.Path value) {
            this.value = value;
        }

        private java.nio.file.Path asNioPath() {
            return value;
        }

        @Override
        public boolean equals(Object other) {
            if (!(other instanceof PathValue)) {
                return false;
            }
            return value.equals(((PathValue) other).value);
        }

        @Override
        public int hashCode() {
            return value.hashCode();
        }

        @Override
        public String toString() {
            return value.toString();
        }
    }

    public static final class Result {
        private final boolean ok;
        private final Object value;

        private Result(boolean ok, Object value) {
            this.ok = ok;
            this.value = value;
        }

        public static Result ok(Object value) {
            return new Result(true, freezeValue(value));
        }

        public static Result err(Object value) {
            return new Result(false, freezeValue(value));
        }

        public boolean isOk() {
            return ok;
        }

        public Object value() {
            return value;
        }

        @Override
        public String toString() {
            return ok ? "Ok(" + format(value) + ")" : "Err(" + format(value) + ")";
        }
    }

    public static final class Option {
        private final boolean some;
        private final Object value;

        private Option(boolean some, Object value) {
            this.some = some;
            this.value = value;
        }

        public static Option some(Object value) {
            return new Option(true, freezeValue(value));
        }

        public static Option none() {
            return new Option(false, null);
        }

        @Override
        public String toString() {
            return some ? "Some(" + format(value) + ")" : "None";
        }
    }

    public static final class ListValue {
        private final boolean cons;
        private final Object head;
        private final Object tail;

        private ListValue(boolean cons, Object head, Object tail) {
            this.cons = cons;
            this.head = head;
            this.tail = tail;
        }

        public static ListValue nil() {
            return new ListValue(false, null, null);
        }

        public static ListValue cons(Object head, Object tail) {
            return new ListValue(true, freezeValue(head), freezeValue(tail));
        }

        @Override
        public String toString() {
            return cons ? "Cons(" + format(head) + ", " + format(tail) + ")" : "Nil";
        }
    }

    public static final class Adt {
        private final String name;
        private final Object[] payloads;

        private Adt(String name, Object[] payloads) {
            this.name = name;
            this.payloads = new Object[payloads.length];
            for (int index = 0; index < payloads.length; index += 1) {
                this.payloads[index] = freezeValue(payloads[index]);
            }
        }

        @Override
        public String toString() {
            String leaf = name;
            int separator = leaf.lastIndexOf("::");
            if (separator >= 0) {
                leaf = leaf.substring(separator + 2);
            }
            if (payloads.length == 0) {
                return leaf;
            }
            StringBuilder out = new StringBuilder();
            out.append(leaf).append("(");
            for (int index = 0; index < payloads.length; index += 1) {
                if (index > 0) {
                    out.append(", ");
                }
                out.append(format(payloads[index]));
            }
            out.append(")");
            return out.toString();
        }
    }

    public static final class ContractFailure extends RuntimeException {
        public final String clause;
        public final String predicate;
        public final String function;
        public final String blame;
        public final String nodeId;
        public final String sourceFile;
        public final int startLine;
        public final int startColumn;
        public final int endLine;
        public final int endColumn;

        private ContractFailure(
            String clause,
            String predicate,
            String function,
            String blame,
            String nodeId,
            String sourceFile,
            int startLine,
            int startColumn,
            int endLine,
            int endColumn
        ) {
            super("contract failure: "
                + clause
                + " `"
                + predicate
                + "` in `"
                + function
                + "` blame "
                + blame
                + " at "
                + sourceFile
                + " "
                + nodeId);
            this.clause = clause;
            this.predicate = predicate;
            this.function = function;
            this.blame = blame;
            this.nodeId = nodeId;
            this.sourceFile = sourceFile;
            this.startLine = startLine;
            this.startColumn = startColumn;
            this.endLine = endLine;
            this.endColumn = endColumn;
        }
    }

    public interface Fn {
        Object call(Object... args);
    }

    private static final Object SELECT_PENDING = new Object();
    private static final Object SELECT_CLOSED = new Object();
    private static final java.util.concurrent.atomic.AtomicInteger SELECT_CURSOR =
        new java.util.concurrent.atomic.AtomicInteger(0);

    public static final class Channel {
        private final java.util.ArrayDeque<Object> queue;
        private final long capacity;
        private long waitingReceivers;
        private boolean hasRendezvousValue;
        private Object rendezvousValue;
        private boolean closed;

        private Channel(long capacity) {
            if (capacity < 0L || capacity > Integer.MAX_VALUE) {
                throw new IllegalArgumentException("channel capacity is out of range");
            }
            this.queue = new java.util.ArrayDeque<Object>();
            this.capacity = capacity;
            this.waitingReceivers = 0L;
            this.hasRendezvousValue = false;
            this.rendezvousValue = null;
            this.closed = false;
        }
    }

    public static final class Sender {
        private final Channel channel;

        private Sender(Channel channel) {
            this.channel = channel;
        }
    }

    public static final class Receiver {
        private final Channel channel;

        private Receiver(Channel channel) {
            this.channel = channel;
        }
    }

    public static final class Task {
        private final java.util.concurrent.FutureTask<Object> future;
        private final Thread thread;

        private Task(Fn fn) {
            this.future = new java.util.concurrent.FutureTask<Object>(
                () -> freezeValue(call(fn))
            );
            this.thread = new Thread(this.future);
            this.thread.start();
        }
    }

    public static Result ok(Object value) {
        return Result.ok(value);
    }

    public static Result err(Object value) {
        return Result.err(value);
    }

    public static Option some(Object value) {
        return Option.some(value);
    }

    public static Option none() {
        return Option.none();
    }

    public static Object listNil() {
        return ListValue.nil();
    }

    public static Object listCons(Object head, Object tail) {
        return ListValue.cons(head, tail);
    }

    public static Object adt(String name, Object[] payloads) {
        return new Adt(name, payloads);
    }

    public static Object channelBounded(Object capacity) {
        Channel channel = new Channel(asLong(capacity));
        return record("tx", new Sender(channel), "rx", new Receiver(channel));
    }

    public static Object channelClone(Object sender) {
        Sender tx = (Sender) sender;
        return new Sender(tx.channel);
    }

    public static Object channelSend(Object sender, Object value) {
        Sender tx = (Sender) sender;
        synchronized (tx.channel) {
            if (tx.channel.closed) {
                return err("closed");
            }
            if (tx.channel.capacity == 0L) {
                Object frozen = freezeValue(value);
                while (!tx.channel.closed
                    && (tx.channel.waitingReceivers == 0L || tx.channel.hasRendezvousValue)) {
                    try {
                        tx.channel.wait();
                    } catch (InterruptedException interrupted) {
                        Thread.currentThread().interrupt();
                        return err("interrupted");
                    }
                }
                if (tx.channel.closed) {
                    return err("closed");
                }
                tx.channel.rendezvousValue = frozen;
                tx.channel.hasRendezvousValue = true;
                tx.channel.notifyAll();
                boolean interruptedAfterTransfer = false;
                while (tx.channel.hasRendezvousValue) {
                    try {
                        tx.channel.wait();
                    } catch (InterruptedException interrupted) {
                        interruptedAfterTransfer = true;
                    }
                }
                if (interruptedAfterTransfer) {
                    Thread.currentThread().interrupt();
                }
                return ok(UNIT);
            }
            while (!tx.channel.closed && tx.channel.queue.size() >= tx.channel.capacity) {
                try {
                    tx.channel.wait();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return err("interrupted");
                }
            }
            if (tx.channel.closed) {
                return err("closed");
            }
            tx.channel.queue.addLast(freezeValue(value));
            tx.channel.notifyAll();
            return ok(UNIT);
        }
    }

    public static Object channelRecv(Object receiver) {
        Receiver rx = (Receiver) receiver;
        synchronized (rx.channel) {
            if (rx.channel.capacity == 0L) {
                rx.channel.waitingReceivers += 1L;
                rx.channel.notifyAll();
                boolean interruptedAfterTransfer = false;
                try {
                    while (!rx.channel.hasRendezvousValue && !rx.channel.closed) {
                        try {
                            rx.channel.wait();
                        } catch (InterruptedException interrupted) {
                            if (!rx.channel.hasRendezvousValue) {
                                Thread.currentThread().interrupt();
                                return none();
                            }
                            interruptedAfterTransfer = true;
                        }
                    }
                    if (!rx.channel.hasRendezvousValue) {
                        return none();
                    }
                    Object value = rx.channel.rendezvousValue;
                    rx.channel.rendezvousValue = null;
                    rx.channel.hasRendezvousValue = false;
                    rx.channel.notifyAll();
                    if (interruptedAfterTransfer) {
                        Thread.currentThread().interrupt();
                    }
                    return some(value);
                } finally {
                    rx.channel.waitingReceivers -= 1L;
                    rx.channel.notifyAll();
                }
            }
            while (rx.channel.queue.isEmpty() && !rx.channel.closed) {
                try {
                    rx.channel.wait();
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return none();
                }
            }
            Object value = rx.channel.queue.pollFirst();
            if (value == null) {
                return none();
            }
            rx.channel.notifyAll();
            return some(value);
        }
    }

    public static Object channelSelect(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, true, false);
    }

    public static Object channelSelectPriority(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, false, false);
    }

    public static Object channelSelectTimeout(
        Object leftReceiver,
        Object rightReceiver,
        Object timeoutMillis
    ) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, asLong(timeoutMillis), true, false);
    }

    public static Object channelSelectResult(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, true, true);
    }

    public static Object channelSelectPriorityResult(Object leftReceiver, Object rightReceiver) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, -1L, false, true);
    }

    public static Object channelSelectTimeoutResult(
        Object leftReceiver,
        Object rightReceiver,
        Object timeoutMillis
    ) {
        return channelSelectWithTimeout(leftReceiver, rightReceiver, asLong(timeoutMillis), true, true);
    }

    private static Object channelSelectWithTimeout(
        Object leftReceiver,
        Object rightReceiver,
        long timeoutMillis,
        boolean rotateTies,
        boolean reportInterrupt
    ) {
        Receiver left = (Receiver) leftReceiver;
        Receiver right = (Receiver) rightReceiver;
        Receiver[] receivers = new Receiver[] { left, right };
        boolean[] registered = new boolean[] { false, false };
        long startNanos = timeoutMillis >= 0L ? System.nanoTime() : 0L;
        long timeoutNanos = timeoutMillis >= 0L
            ? (timeoutMillis > Long.MAX_VALUE / 1000000L
                ? Long.MAX_VALUE
                : timeoutMillis * 1000000L)
            : -1L;
        try {
            for (int i = 0; i < receivers.length; i += 1) {
                Channel channel = receivers[i].channel;
                if (channel.capacity == 0L) {
                    synchronized (channel) {
                        channel.waitingReceivers += 1L;
                        registered[i] = true;
                        channel.notifyAll();
                    }
                }
            }
            int start = rotateTies
                ? Math.floorMod(SELECT_CURSOR.getAndIncrement(), receivers.length)
                : 0;
            while (true) {
                boolean allClosed = true;
                for (int offset = 0; offset < receivers.length; offset += 1) {
                    int i = (start + offset) % receivers.length;
                    Object selected = channelSelectPoll(receivers[i], i);
                    if (selected == SELECT_PENDING) {
                        allClosed = false;
                    } else if (selected != SELECT_CLOSED) {
                        return channelSelectReturn(selected, reportInterrupt);
                    }
                }
                if (allClosed) {
                    return channelSelectReturn(none(), reportInterrupt);
                }
                if (timeoutNanos >= 0L && System.nanoTime() - startNanos >= timeoutNanos) {
                    return channelSelectReturn(none(), reportInterrupt);
                }
                try {
                    Thread.sleep(1L);
                } catch (InterruptedException interrupted) {
                    Thread.currentThread().interrupt();
                    return reportInterrupt ? err("interrupted") : none();
                }
            }
        } finally {
            for (int i = 0; i < receivers.length; i += 1) {
                if (registered[i]) {
                    Channel channel = receivers[i].channel;
                    synchronized (channel) {
                        channel.waitingReceivers -= 1L;
                        channel.notifyAll();
                    }
                }
            }
        }
    }

    private static Object channelSelectReturn(Object selected, boolean reportInterrupt) {
        return reportInterrupt ? ok(selected) : selected;
    }

    private static Object channelSelectPoll(Receiver rx, int index) {
        synchronized (rx.channel) {
            Object value;
            if (rx.channel.capacity == 0L) {
                if (rx.channel.hasRendezvousValue) {
                    value = rx.channel.rendezvousValue;
                    rx.channel.rendezvousValue = null;
                    rx.channel.hasRendezvousValue = false;
                    rx.channel.notifyAll();
                    return some(record("index", Long.valueOf(index), "value", value));
                }
                return rx.channel.closed ? SELECT_CLOSED : SELECT_PENDING;
            }
            value = rx.channel.queue.pollFirst();
            if (value != null) {
                rx.channel.notifyAll();
                return some(record("index", Long.valueOf(index), "value", value));
            }
            return rx.channel.closed ? SELECT_CLOSED : SELECT_PENDING;
        }
    }

    public static Object channelClose(Object sender) {
        Sender tx = (Sender) sender;
        synchronized (tx.channel) {
            tx.channel.closed = true;
            tx.channel.notifyAll();
        }
        return UNIT;
    }

    public static Object taskSpawn(Object fn) {
        return new Task((Fn) fn);
    }

    public static Object taskJoin(Object task) {
        Task handle = (Task) task;
        try {
            return ok(handle.future.get());
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            return err("interrupted");
        } catch (java.util.concurrent.ExecutionException failed) {
            return err("failed");
        } catch (java.util.concurrent.CancellationException cancelled) {
            return err("cancelled");
        }
    }

    public static Object taskCancel(Object task) {
        Task handle = (Task) task;
        handle.future.cancel(true);
        handle.thread.interrupt();
        return UNIT;
    }

    private static java.util.List<Object> processArgs =
        java.util.Collections.unmodifiableList(new java.util.ArrayList<Object>());

    public static void setProcessArgs(String[] args) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>();
        for (String arg : args) {
            values.add(arg);
        }
        processArgs = freezeList(values);
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

    public static boolean isErr(Object value) {
        return value instanceof Result && !((Result) value).isOk();
    }

    public static void checkContract(
        Object value,
        String clause,
        String predicate,
        String function,
        String blame,
        String nodeId,
        String sourceFile,
        int startLine,
        int startColumn,
        int endLine,
        int endColumn
    ) {
        if (!asBool(value)) {
            throw new ContractFailure(
                clause,
                predicate,
                function,
                blame,
                nodeId,
                sourceFile,
                startLine,
                startColumn,
                endLine,
                endColumn
            );
        }
    }

    public static boolean isOk(Object value) {
        return value instanceof Result && ((Result) value).isOk();
    }

    public static Object resultValue(Object value) {
        return asResult(value).value();
    }

    public static boolean isSome(Object value) {
        return value instanceof Option && ((Option) value).some;
    }

    public static boolean isNone(Object value) {
        return value instanceof Option && !((Option) value).some;
    }

    public static Object optionValue(Object value) {
        return asOption(value).value;
    }

    public static boolean isNil(Object value) {
        return value instanceof ListValue && !((ListValue) value).cons;
    }

    public static boolean isCons(Object value) {
        return value instanceof ListValue && ((ListValue) value).cons;
    }

    public static Object listHead(Object value) {
        return asListValue(value).head;
    }

    public static Object listTail(Object value) {
        return asListValue(value).tail;
    }

    public static boolean isAdt(Object value, String name) {
        return value instanceof Adt
            && (((Adt) value).name.equals(name) || ((Adt) value).name.endsWith("::" + name));
    }

    public static Object adtPayload(Object value, int index) {
        return ((Adt) value).payloads[index];
    }

    public static Object unwrapOk(Object value) {
        if (value instanceof Result) {
            Result result = (Result) value;
            if (result.isOk()) {
                return result.value();
            }
        }
        throw new IllegalStateException("expected Ok result");
    }

    public static java.util.Map<String, Object> record(Object... entries) {
        java.util.LinkedHashMap<String, Object> map = new java.util.LinkedHashMap<String, Object>();
        for (int index = 0; index + 1 < entries.length; index += 2) {
            map.put((String) entries[index], entries[index + 1]);
        }
        return freezeMap(map);
    }

    public static Object recordField(Object record, String field) {
        return asMap(record).get(field);
    }

    public static boolean recordHasField(Object record, String field) {
        return asMap(record).containsKey(field);
    }

    public static java.util.List<Object> list(Object... values) {
        return freezeList(new java.util.ArrayList<Object>(java.util.Arrays.asList(values)));
    }

    public static java.util.Map<Object, Object> dict(Object... entries) {
        java.util.LinkedHashMap<Object, Object> map = new java.util.LinkedHashMap<Object, Object>();
        for (int i = 0; i + 1 < entries.length; i += 2) {
            map.put(entries[i], entries[i + 1]);
        }
        return freezeMap(map);
    }

    public static Object vecLen(Object items) {
        return Long.valueOf(asList(items).size());
    }

    public static Object stringSplitOnce(Object text, Object separator) {
        String input = asString(text);
        String needle = asString(separator);
        int index = input.indexOf(needle);
        if (index < 0) {
            return none();
        }
        java.util.LinkedHashMap<String, Object> parts = new java.util.LinkedHashMap<String, Object>();
        parts.put("left", input.substring(0, index));
        parts.put("right", input.substring(index + needle.length()));
        return some(freezeMap(parts));
    }

    public static Object stringParseInt(Object text) {
        String input = asString(text);
        try {
            return ok(Long.valueOf(input));
        } catch (NumberFormatException error) {
            return err(input);
        }
    }

    public static Object intToString(Object value) {
        return String.valueOf(asLong(value));
    }

    public static Object vecIsEmpty(Object items) {
        return Boolean.valueOf(asList(items).isEmpty());
    }

    public static Object vecPush(Object items, Object value) {
        java.util.ArrayList<Object> copy = new java.util.ArrayList<Object>(asList(items));
        copy.add(value);
        return freezeList(copy);
    }

    public static Object vecConcat(Object left, Object right) {
        java.util.ArrayList<Object> copy = new java.util.ArrayList<Object>(asList(left));
        copy.addAll(asList(right));
        return freezeList(copy);
    }

    public static Object vecMap(Object items, Object fn) {
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {
            mapped.add(call(fn, item));
        }
        return freezeList(mapped);
    }

    public static Object vecFilter(Object items, Object fn) {
        java.util.ArrayList<Object> filtered = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {
            if (asBool(call(fn, item))) {
                filtered.add(item);
            }
        }
        return freezeList(filtered);
    }

    public static Object vecFold(Object items, Object initial, Object fn) {
        Object accumulator = initial;
        for (Object item : asList(items)) {
            accumulator = call(fn, accumulator, item);
        }
        return accumulator;
    }

    public static Object vecTryMap(Object items, Object fn) {
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {
            Object result = call(fn, item);
            if (isErr(result)) {
                return result;
            }
            mapped.add(unwrapOk(result));
        }
        return ok(freezeList(mapped));
    }

    public static Object vecTryMapWith(Object context, Object items, Object fn) {
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>();
        for (Object item : asList(items)) {
            Object result = call(fn, context, item);
            if (isErr(result)) {
                return result;
            }
            mapped.add(unwrapOk(result));
        }
        return ok(freezeList(mapped));
    }

    public static Object listIsEmpty(Object items) {
        return Boolean.valueOf(!asListValue(items).cons);
    }

    public static Object listFold(Object items, Object initial, Object fn) {
        Object accumulator = initial;
        ListValue current = asListValue(items);
        while (current.cons) {
            accumulator = call(fn, accumulator, current.head);
            current = asListValue(current.tail);
        }
        return accumulator;
    }

    public static Object listReverse(Object items) {
        Object reversed = listNil();
        ListValue current = asListValue(items);
        while (current.cons) {
            reversed = listCons(current.head, reversed);
            current = asListValue(current.tail);
        }
        return reversed;
    }

    public static Object listMap(Object items, Object fn) {
        Object reversed = listNil();
        ListValue current = asListValue(items);
        while (current.cons) {
            reversed = listCons(call(fn, current.head), reversed);
            current = asListValue(current.tail);
        }
        return listReverse(reversed);
    }

    public static Object listFilter(Object items, Object fn) {
        Object reversed = listNil();
        ListValue current = asListValue(items);
        while (current.cons) {
            if (asBool(call(fn, current.head))) {
                reversed = listCons(current.head, reversed);
            }
            current = asListValue(current.tail);
        }
        return listReverse(reversed);
    }

    public static Object listTryMap(Object items, Object fn) {
        Object reversed = listNil();
        ListValue current = asListValue(items);
        while (current.cons) {
            Object result = call(fn, current.head);
            if (isErr(result)) {
                return result;
            }
            reversed = listCons(unwrapOk(result), reversed);
            current = asListValue(current.tail);
        }
        return ok(listReverse(reversed));
    }

    public static Object dictGet(Object dict, Object key) {
        java.util.Map<Object, Object> map = asMap(dict);
        if (map.containsKey(key)) {
            return some(map.get(key));
        }
        return none();
    }

    public static Object dictContains(Object dict, Object key) {
        return Boolean.valueOf(asMap(dict).containsKey(key));
    }

    public static Object dictInsert(Object dict, Object key, Object value) {
        java.util.LinkedHashMap<Object, Object> copy =
            new java.util.LinkedHashMap<Object, Object>(asMap(dict));
        copy.put(key, value);
        return freezeMap(copy);
    }

    public static Object dictRemove(Object dict, Object key) {
        java.util.LinkedHashMap<Object, Object> copy =
            new java.util.LinkedHashMap<Object, Object>(asMap(dict));
        copy.remove(key);
        return freezeMap(copy);
    }

    private static java.util.List<Object> freezeList(java.util.List<Object> values) {
        java.util.ArrayList<Object> frozen = new java.util.ArrayList<Object>(values.size());
        for (Object value : values) {
            frozen.add(freezeValue(value));
        }
        return java.util.Collections.unmodifiableList(frozen);
    }

    private static <K, V> java.util.Map<K, V> freezeMap(java.util.Map<K, V> values) {
        java.util.LinkedHashMap<K, V> frozen = new java.util.LinkedHashMap<K, V>();
        for (java.util.Map.Entry<K, V> entry : values.entrySet()) {
            @SuppressWarnings("unchecked")
            K key = (K) freezeValue(entry.getKey());
            @SuppressWarnings("unchecked")
            V value = (V) freezeValue(entry.getValue());
            frozen.put(key, value);
        }
        return java.util.Collections.unmodifiableMap(frozen);
    }

    @SuppressWarnings("unchecked")
    private static Object freezeValue(Object value) {
        if (value instanceof java.util.List) {
            return freezeList((java.util.List<Object>) value);
        }
        if (value instanceof java.util.Map) {
            return freezeMap((java.util.Map<Object, Object>) value);
        }
        return value;
    }

    public static Object optionMap(Object option, Object fn) {
        Option value = asOption(option);
        if (!value.some) {
            return none();
        }
        return some(call(fn, value.value));
    }

    public static Object optionAndThen(Object option, Object fn) {
        Option value = asOption(option);
        if (!value.some) {
            return none();
        }
        return call(fn, value.value);
    }

    public static Object optionUnwrapOr(Object option, Object fallback) {
        Option value = asOption(option);
        return value.some ? value.value : fallback;
    }

    public static Object resultMap(Object result, Object fn) {
        Result value = asResult(result);
        if (!value.isOk()) {
            return value;
        }
        return ok(call(fn, value.value()));
    }

    public static Object resultMapErr(Object result, Object fn) {
        Result value = asResult(result);
        if (value.isOk()) {
            return value;
        }
        return err(call(fn, value.value()));
    }

    public static Object resultAndThen(Object result, Object fn) {
        Result value = asResult(result);
        if (!value.isOk()) {
            return value;
        }
        return call(fn, value.value());
    }

    public static Object floatNegate(Object value) {
        return Double.valueOf(-asDouble(value));
    }

    public static Object floatAdd(Object left, Object right) {
        return Double.valueOf(asDouble(left) + asDouble(right));
    }

    public static Object floatSubtract(Object left, Object right) {
        return Double.valueOf(asDouble(left) - asDouble(right));
    }

    public static Object floatMultiply(Object left, Object right) {
        return Double.valueOf(asDouble(left) * asDouble(right));
    }

    public static Object floatDivide(Object left, Object right) {
        return Double.valueOf(asDouble(left) / asDouble(right));
    }

    public static Object floatLess(Object left, Object right) {
        return Boolean.valueOf(asDouble(left) < asDouble(right));
    }

    public static Object floatLessEqual(Object left, Object right) {
        return Boolean.valueOf(asDouble(left) <= asDouble(right));
    }

    public static Object floatGreater(Object left, Object right) {
        return Boolean.valueOf(asDouble(left) > asDouble(right));
    }

    public static Object floatGreaterEqual(Object left, Object right) {
        return Boolean.valueOf(asDouble(left) >= asDouble(right));
    }

    private static final Object stdioLock = new Object();
    private static int stdioSequence = 0;

    public static Object stdioPrint(Object value) {
        return stdioPrint(value, null, null);
    }

    public static Object stdioPrint(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.out.print(text);
            recordStdioEvent("stdout", "print", "none", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioPrintln(Object value) {
        return stdioPrintln(value, null, null);
    }

    public static Object stdioPrintln(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.out.print(text);
            System.out.print(System.lineSeparator());
            recordStdioEvent("stdout", "println", "newline", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioEprint(Object value) {
        return stdioEprint(value, null, null);
    }

    public static Object stdioEprint(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.err.print(text);
            recordStdioEvent("stderr", "eprint", "none", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioEprintln(Object value) {
        return stdioEprintln(value, null, null);
    }

    public static Object stdioEprintln(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.err.print(text);
            System.err.print(System.lineSeparator());
            recordStdioEvent("stderr", "eprintln", "newline", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object call(Object fn, Object... args) {
        if (fn instanceof Fn) {
            return ((Fn) fn).call(args);
        }
        throw new IllegalStateException("value is not callable");
    }

    public static Object not(Object value) {
        return Boolean.valueOf(!asBool(value));
    }

    public static Object negate(Object value) {
        if (isFloating(value)) {
            return floatNegate(value);
        }
        return Long.valueOf(-asLong(value));
    }

    public static Object add(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatAdd(left, right);
        }
        return Long.valueOf(asLong(left) + asLong(right));
    }

    public static Object subtract(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatSubtract(left, right);
        }
        return Long.valueOf(asLong(left) - asLong(right));
    }

    public static Object multiply(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatMultiply(left, right);
        }
        return Long.valueOf(asLong(left) * asLong(right));
    }

    public static Object divide(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatDivide(left, right);
        }
        return Long.valueOf(asLong(left) / asLong(right));
    }

    public static Object equal(Object left, Object right) {
        return Boolean.valueOf(java.util.Objects.equals(left, right));
    }

    public static Object notEqual(Object left, Object right) {
        return Boolean.valueOf(!java.util.Objects.equals(left, right));
    }

    public static Object less(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatLess(left, right);
        }
        return Boolean.valueOf(asLong(left) < asLong(right));
    }

    public static Object lessEqual(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatLessEqual(left, right);
        }
        return Boolean.valueOf(asLong(left) <= asLong(right));
    }

    public static Object greater(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatGreater(left, right);
        }
        return Boolean.valueOf(asLong(left) > asLong(right));
    }

    public static Object greaterEqual(Object left, Object right) {
        if (isFloating(left) || isFloating(right)) {
            return floatGreaterEqual(left, right);
        }
        return Boolean.valueOf(asLong(left) >= asLong(right));
    }

    public static Object and(Object left, Object right) {
        return Boolean.valueOf(asBool(left) && asBool(right));
    }

    public static Object or(Object left, Object right) {
        return Boolean.valueOf(asBool(left) || asBool(right));
    }

    public static Object pipe(Object left, Object right) {
        return right;
    }

    public static String format(Object value) {
        if (value == UNIT) {
            return "()";
        }
        return String.valueOf(value);
    }

    private static void recordStdioEvent(
        String stream,
        String operation,
        String terminator,
        String text,
        String nodeId,
        String sourceFile
    ) {
        String path = System.getenv("VELN_STDIO_EVENTS");
        if (path == null || path.isEmpty()) {
            return;
        }
        stdioSequence += 1;
        String line = Integer.toString(stdioSequence)
            + "\t" + stream
            + "\t" + operation
            + "\t" + terminator
            + "\t" + (nodeId == null ? "" : nodeId)
            + "\t" + (sourceFile == null ? "" : sourceFile)
            + "\t" + hex(text)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException error) {
            throw new RuntimeException("failed to record stdio event", error);
        }
    }

    public static void recordContractFailure(ContractFailure error) {
        String path = System.getenv("VELN_CONTRACT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String line = "contract"
            + "\t" + error.clause
            + "\t" + hex(error.predicate)
            + "\t" + hex(error.function)
            + "\t" + error.blame
            + "\t" + hex(error.nodeId)
            + "\t" + hex(error.sourceFile)
            + "\t" + Integer.toString(error.startLine)
            + "\t" + Integer.toString(error.startColumn)
            + "\t" + Integer.toString(error.endLine)
            + "\t" + Integer.toString(error.endColumn)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record contract error", ioError);
        }
    }

    public static void recordResultFailure(Object result) {
        String path = System.getenv("VELN_RESULT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String value = format(asResult(result).value());
        String line = "result"
            + "\t" + hex(value)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record result error", ioError);
        }
    }

    private static String hex(String text) {
        byte[] bytes = text.getBytes(java.nio.charset.StandardCharsets.UTF_8);
        char[] digits = "0123456789abcdef".toCharArray();
        char[] encoded = new char[bytes.length * 2];
        for (int index = 0; index < bytes.length; index += 1) {
            int value = bytes[index] & 0xff;
            encoded[index * 2] = digits[value >>> 4];
            encoded[index * 2 + 1] = digits[value & 0x0f];
        }
        return new String(encoded);
    }

    private static boolean asBool(Object value) {
        return ((Boolean) value).booleanValue();
    }

    private static long asLong(Object value) {
        return ((Number) value).longValue();
    }

    private static double asDouble(Object value) {
        return ((Number) value).doubleValue();
    }

    private static PathValue pathValue(java.nio.file.Path value) {
        return new PathValue(value);
    }

    private static java.nio.file.Path asPath(Object value) {
        return ((PathValue) value).asNioPath();
    }

    private static String asString(Object value) {
        return (String) value;
    }

    private static boolean isFloating(Object value) {
        return value instanceof Double || value instanceof Float;
    }

    @SuppressWarnings("unchecked")
    private static java.util.List<Object> asList(Object value) {
        return (java.util.List<Object>) value;
    }

    @SuppressWarnings("unchecked")
    private static java.util.Map<Object, Object> asMap(Object value) {
        return (java.util.Map<Object, Object>) value;
    }

    private static Option asOption(Object value) {
        return (Option) value;
    }

    private static ListValue asListValue(Object value) {
        return (ListValue) value;
    }

    private static Result asResult(Object value) {
        return (Result) value;
    }
}
