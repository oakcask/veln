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

    public static class RuntimeFailure extends RuntimeException {
        protected RuntimeFailure(String message) {
            super(legacyTransportFailureMessage(message, null));
        }

        protected RuntimeFailure(String message, Throwable cause) {
            super(legacyTransportFailureMessage(message, cause), cause);
        }
    }

    private static final class TransportFailure extends RuntimeFailure {
        private TransportFailure(
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
            super(
                transportFailureMessage(
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
                    platformCause
                ),
                cause
            );
            recordTransportFailure(
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
                platformCause
            );
        }
    }

    private static final class DiagnosticResultValue {
        final Object value;
        final FixtureHexDiagnostic fixtureHexDiagnostic;
        final ByteDiagnostic byteDiagnostic;
        final ValueDiagnostic valueDiagnostic;
        final ProtocolDiagnostic protocolDiagnostic;

        DiagnosticResultValue(
            Object value,
            FixtureHexDiagnostic fixtureHexDiagnostic,
            ByteDiagnostic byteDiagnostic,
            ValueDiagnostic valueDiagnostic,
            ProtocolDiagnostic protocolDiagnostic
        ) {
            this.value = freezeValue(value);
            this.fixtureHexDiagnostic = fixtureHexDiagnostic;
            this.byteDiagnostic = byteDiagnostic;
            this.valueDiagnostic = valueDiagnostic;
            this.protocolDiagnostic = protocolDiagnostic;
        }

        @Override
        public boolean equals(Object other) {
            return java.util.Objects.equals(value, diagnosticResultValue(other));
        }

        @Override
        public int hashCode() {
            return java.util.Objects.hashCode(value);
        }

        @Override
        public String toString() {
            return format(value);
        }
    }

    public interface Fn {
        Object call(Object... args);
    }

    private static final ThreadLocal<java.util.List<HandlerFrame>> HANDLERS =
        new ThreadLocal<java.util.List<HandlerFrame>>() {
            @Override
            protected java.util.List<HandlerFrame> initialValue() {
                return new java.util.ArrayList<HandlerFrame>();
            }
        };

    private static final class HandlerFrame {
        private final String effect;
        private final java.util.Map<String, Fn> providers;
        private final Object[] context;

        private HandlerFrame(String effect, java.util.Map<String, Fn> providers, Object[] context) {
            this.effect = effect;
            this.providers = providers;
            this.context = context;
        }
    }

    public static Object pushHandler(String effect, Object[] operations, Object[] providers, Object[] context) {
        java.util.Map<String, Fn> table = new java.util.HashMap<String, Fn>();
        for (int index = 0; index < operations.length; index += 1) {
            table.put((String) operations[index], (Fn) providers[index]);
        }
        HANDLERS.get().add(new HandlerFrame(effect, table, context));
        return UNIT;
    }

    public static Object popHandler() {
        java.util.List<HandlerFrame> stack = HANDLERS.get();
        stack.remove(stack.size() - 1);
        return UNIT;
    }

    public static Object perform(String effect, String operation, Object[] args) {
        java.util.List<HandlerFrame> stack = HANDLERS.get();
        for (int index = stack.size() - 1; index >= 0; index -= 1) {
            HandlerFrame frame = stack.get(index);
            if (!frame.effect.equals(effect)) {
                continue;
            }
            Fn provider = frame.providers.get(operation);
            if (provider == null) {
                break;
            }
            Object[] providerArgs = new Object[frame.context.length + args.length];
            System.arraycopy(frame.context, 0, providerArgs, 0, frame.context.length);
            System.arraycopy(args, 0, providerArgs, frame.context.length, args.length);
            return provider.call(providerArgs);
        }
        throw new IllegalStateException("unhandled effect operation " + effect + "::" + operation);
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

    private static Object http2DiagnosticDetail(String name, Object[] payloads) {
        return adt(
            "RuntimeDiagnosticDetail::RuntimeHttp2Diagnostic",
            new Object[] { adt(name, payloads) }
        );
    }

    private static Object http2HpackDiagnosticDetail(String name, Object[] payloads) {
        return adt(
            "RuntimeDiagnosticDetail::RuntimeHttp2HpackDiagnostic",
            new Object[] { adt(name, payloads) }
        );
    }
