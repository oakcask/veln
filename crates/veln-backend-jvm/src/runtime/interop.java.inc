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
    public static Object call(Object fn, Object... args) {
        if (fn instanceof Fn) {
            return ((Fn) fn).call(args);
        }
        throw new IllegalStateException("value is not callable");
    }
    public static String format(Object value) {
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
        Adt adt = (Adt) value;
        if (!adt.name.equals(name)) {
            throw new IllegalStateException("expected " + name + ", found " + adt.name);
        }
        return adt;
    }
