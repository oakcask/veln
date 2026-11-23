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
        if (value instanceof DiagnosticResultValue) {
            DiagnosticResultValue diagnostic = (DiagnosticResultValue) value;
            return new DiagnosticResultValue(
                diagnostic.value,
                diagnostic.fixtureHexDiagnostic,
                diagnostic.byteDiagnostic,
                diagnostic.valueDiagnostic,
                diagnostic.protocolDiagnostic,
                diagnostic.transportFailure
            );
        }
        if (value instanceof java.util.List) {
            return freezeList((java.util.List<Object>) value);
        }
        if (value instanceof java.util.Map) {
            return freezeMap((java.util.Map<Object, Object>) value);
        }
        return value;
    }

    private static final class BoundaryFreezeFrame {
        final Object value;
        final boolean expanded;

        BoundaryFreezeFrame(Object value, boolean expanded) {
            this.value = value;
            this.expanded = expanded;
        }
    }

    private static boolean isBoundaryComposite(Object value) {
        return value instanceof DiagnosticResultValue
            || value instanceof java.util.List
            || value instanceof java.util.Map
            || value instanceof Result
            || value instanceof Option && ((Option) value).some
            || value instanceof ListValue && ((ListValue) value).cons
            || value instanceof Adt;
    }

    private static void pushBoundaryChildren(
        java.util.ArrayDeque<BoundaryFreezeFrame> pending,
        Object value
    ) {
        if (value instanceof DiagnosticResultValue) {
            pending.push(new BoundaryFreezeFrame(((DiagnosticResultValue) value).value, false));
        } else if (value instanceof java.util.List) {
            java.util.List<?> values = (java.util.List<?>) value;
            for (int index = values.size() - 1; index >= 0; index -= 1) {
                pending.push(new BoundaryFreezeFrame(values.get(index), false));
            }
        } else if (value instanceof java.util.Map) {
            java.util.ArrayList<java.util.Map.Entry<?, ?>> entries =
                new java.util.ArrayList<java.util.Map.Entry<?, ?>>(
                    ((java.util.Map<?, ?>) value).entrySet()
                );
            for (int index = entries.size() - 1; index >= 0; index -= 1) {
                java.util.Map.Entry<?, ?> entry = entries.get(index);
                pending.push(new BoundaryFreezeFrame(entry.getValue(), false));
                pending.push(new BoundaryFreezeFrame(entry.getKey(), false));
            }
        } else if (value instanceof Result) {
            pending.push(new BoundaryFreezeFrame(((Result) value).value, false));
        } else if (value instanceof Option) {
            pending.push(new BoundaryFreezeFrame(((Option) value).value, false));
        } else if (value instanceof ListValue) {
            ListValue list = (ListValue) value;
            pending.push(new BoundaryFreezeFrame(list.tail, false));
            pending.push(new BoundaryFreezeFrame(list.head, false));
        } else if (value instanceof Adt) {
            Object[] payloads = ((Adt) value).payloads;
            for (int index = payloads.length - 1; index >= 0; index -= 1) {
                pending.push(new BoundaryFreezeFrame(payloads[index], false));
            }
        }
    }

    private static Object rebuildBoundaryComposite(
        Object value,
        java.util.IdentityHashMap<Object, Object> frozen
    ) {
        if (value instanceof DiagnosticResultValue) {
            DiagnosticResultValue diagnostic = (DiagnosticResultValue) value;
            return new DiagnosticResultValue(
                frozen.get(diagnostic.value),
                diagnostic.fixtureHexDiagnostic,
                diagnostic.byteDiagnostic,
                diagnostic.valueDiagnostic,
                diagnostic.protocolDiagnostic,
                diagnostic.transportFailure,
                true
            );
        }
        if (value instanceof java.util.List) {
            java.util.List<?> values = (java.util.List<?>) value;
            java.util.ArrayList<Object> result = new java.util.ArrayList<Object>(values.size());
            for (Object item : values) result.add(frozen.get(item));
            return java.util.Collections.unmodifiableList(result);
        }
        if (value instanceof java.util.Map) {
            java.util.LinkedHashMap<Object, Object> result =
                new java.util.LinkedHashMap<Object, Object>();
            for (java.util.Map.Entry<?, ?> entry : ((java.util.Map<?, ?>) value).entrySet()) {
                result.put(frozen.get(entry.getKey()), frozen.get(entry.getValue()));
            }
            return java.util.Collections.unmodifiableMap(result);
        }
        if (value instanceof Result) {
            Result result = (Result) value;
            return new Result(result.ok, frozen.get(result.value));
        }
        if (value instanceof Option) {
            Option option = (Option) value;
            return new Option(true, frozen.get(option.value));
        }
        if (value instanceof ListValue) {
            ListValue list = (ListValue) value;
            return new ListValue(true, frozen.get(list.head), frozen.get(list.tail));
        }
        Adt adt = (Adt) value;
        Object[] payloads = new Object[adt.payloads.length];
        for (int index = 0; index < payloads.length; index += 1) {
            payloads[index] = frozen.get(adt.payloads[index]);
        }
        return new Adt(adt.name, payloads);
    }

    private static Object freezeBoundaryValue(Object value) {
        java.util.ArrayDeque<BoundaryFreezeFrame> pending =
            new java.util.ArrayDeque<BoundaryFreezeFrame>();
        java.util.IdentityHashMap<Object, Object> frozen =
            new java.util.IdentityHashMap<Object, Object>();
        pending.push(new BoundaryFreezeFrame(value, false));
        while (!pending.isEmpty()) {
            BoundaryFreezeFrame frame = pending.pop();
            if (frozen.containsKey(frame.value)) continue;
            if (!frame.expanded) {
                if (isByteViewValue(frame.value)) {
                    frozen.put(frame.value, freezeByteView(frame.value));
                } else if (isBoundaryComposite(frame.value)) {
                    pending.push(new BoundaryFreezeFrame(frame.value, true));
                    pushBoundaryChildren(pending, frame.value);
                } else {
                    frozen.put(frame.value, frame.value);
                }
            } else {
                frozen.put(frame.value, rebuildBoundaryComposite(frame.value, frozen));
            }
        }
        return frozen.get(value);
    }
    public static Object call(Object fn, Object... args) {
        if (fn instanceof Fn) {
            return ((Fn) fn).call(args);
        }
        throw new IllegalStateException("value is not callable");
    }
    public static Object callAt(Object fn, Object[] args, Object callsite) {
        if (fn instanceof CallsiteFn) {
            return ((CallsiteFn) fn).call(args, callsite);
        }
        if (fn instanceof Fn) {
            return ((Fn) fn).call(args);
        }
        throw new IllegalStateException("value is not callable");
    }
    public static Object bindCallsite(Object fn, Object callsite) {
        if (fn instanceof BoundCallsiteFn || !(fn instanceof CallsiteFn)) {
            return fn;
        }
        return new BoundCallsiteFn(fn, callsite);
    }
    public static String format(Object value) {
        value = diagnosticResultValue(value);
        if (value == UNIT) {
            return "()";
        }
        return String.valueOf(value);
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
        value = diagnosticResultValue(value);
        if (value instanceof JoinErrorValue) {
            return ((JoinErrorValue) value).message;
        }
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

    private static Adt asAdt(Object value, String name) {
        value = diagnosticResultValue(value);
        Adt adt = (Adt) value;
        if (!adt.name.equals(name)
            && !adt.name.endsWith("::" + name)
            && !name.endsWith("::" + adt.name)) {
            throw new IllegalStateException("expected " + name + ", found " + adt.name);
        }
        return adt;
    }

    private static Object diagnosticResultValue(Object value) {
        return value instanceof DiagnosticResultValue
            ? ((DiagnosticResultValue) value).value
            : value;
    }
