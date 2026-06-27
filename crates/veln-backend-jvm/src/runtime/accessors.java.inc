    public static boolean isErr(Object value) {
        return value instanceof Result && !((Result) value).isOk();
    }

    public static boolean isEncodeStepInvalid(Object value) {
        return isAdt(value, "EncodeStep::Invalid");
    }

    public static Object encodeStepInvalidAsErr(Object value) {
        return Result.err(adtPayload(value, 0));
    }

    public static boolean isDecodeStepInvalid(Object value) {
        return isAdt(value, "DecodeStep::Invalid");
    }

    public static Object decodeStepInvalidAsErr(Object value) {
        Object error = adtPayload(value, 0);
        return Result.err(error);
    }

    public static boolean isDecodeStepNeedMore(Object value) {
        return isAdt(value, "DecodeStep::NeedMore");
    }

    public static Object decodeStepNeedMoreAsErr(Object value) {
        return Result.err(value);
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
        if (value instanceof FrozenByteView) {
            return name.equals("ByteView") || name.endsWith("::ByteView");
        }
        return value instanceof Adt
            && (((Adt) value).name.equals(name) || ((Adt) value).name.endsWith("::" + name));
    }

    public static Object adtPayload(Object value, int index) {
        if (value instanceof FrozenByteView) {
            FrozenByteView view = (FrozenByteView) value;
            if (index == 0) {
                return view.chunk;
            }
            if (index == 1) {
                return byteOffsetObject(view.logicalOffset);
            }
            if (index == 2) {
                return byteCountObject(view.count);
            }
            throw new IndexOutOfBoundsException("ByteView payload index " + index);
        }
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
