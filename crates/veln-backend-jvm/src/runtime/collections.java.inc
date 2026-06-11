    private static Object byteObject(long value) {
        return adt("Byte", new Object[] { Long.valueOf(value) });
    }

    private static Object byteChunkObject(java.util.List<Object> bytes) {
        return adt("ByteChunk", new Object[] { freezeList(bytes) });
    }

    private static Object byteCountObject(long value) {
        return adt("ByteCount", new Object[] { Long.valueOf(value) });
    }

    private static Object byteOffsetObject(long value) {
        return adt("ByteOffset", new Object[] { Long.valueOf(value) });
    }

    private static long byteCountLong(Object value) {
        return asLong(asAdt(value, "ByteCount").payloads[0]);
    }

    private static java.util.List<Object> byteChunkBytes(Object chunk) {
        return asList(asAdt(chunk, "ByteChunk").payloads[0]);
    }

    public static Object byteValue(Object value) {
        long byteValue = asLong(value);
        if (byteValue < 0 || byteValue > 255) {
            return err("byte value must be between 0 and 255");
        }
        return ok(byteObject(byteValue));
    }

    public static Object byteToInt(Object value) {
        return Long.valueOf(asLong(asAdt(value, "Byte").payloads[0]));
    }

    public static Object byteChunk(Object bytes) {
        return byteChunkObject(asList(bytes));
    }

    public static Object byteChunkCount(Object chunk) {
        return byteCountObject(byteChunkBytes(chunk).size());
    }

    public static Object byteAppend(Object left, Object right) {
        java.util.ArrayList<Object> copy = new java.util.ArrayList<Object>(byteChunkBytes(left));
        copy.addAll(byteChunkBytes(right));
        return byteChunkObject(copy);
    }

    public static Object byteTake(Object chunk, Object count) {
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        long requested = byteCountLong(count);
        if (requested < 0) {
            return err("byte count must be non-negative");
        }
        if (requested > bytes.size()) {
            return err("byte count exceeds chunk length");
        }
        return ok(byteChunkObject(bytes.subList(0, (int) requested)));
    }

    public static Object byteDrop(Object chunk, Object count) {
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        long requested = byteCountLong(count);
        if (requested < 0) {
            return err("byte count must be non-negative");
        }
        if (requested > bytes.size()) {
            return err("byte count exceeds chunk length");
        }
        return ok(byteChunkObject(bytes.subList((int) requested, bytes.size())));
    }

    public static Object byteCount(Object value) {
        long count = asLong(value);
        if (count < 0) {
            return err("byte count must be non-negative");
        }
        return ok(byteCountObject(count));
    }

    public static Object byteCountToInt(Object value) {
        return Long.valueOf(byteCountLong(value));
    }

    public static Object byteOffset(Object value) {
        long offset = asLong(value);
        if (offset < 0) {
            return err("byte offset must be non-negative");
        }
        return ok(byteOffsetObject(offset));
    }

    public static Object byteOffsetToInt(Object value) {
        return Long.valueOf(asLong(asAdt(value, "ByteOffset").payloads[0]));
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
