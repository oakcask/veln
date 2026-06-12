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

    private static Object byteViewObject(Object chunk, long offset, long count) {
        return adt(
            "ByteView",
            new Object[] { chunk, byteOffsetObject(offset), byteCountObject(count) }
        );
    }

    private static long byteCountLong(Object value) {
        return asLong(asAdt(value, "ByteCount").payloads[0]);
    }

    private static long byteOffsetLong(Object value) {
        return asLong(asAdt(value, "ByteOffset").payloads[0]);
    }

    private static java.util.List<Object> byteChunkBytes(Object chunk) {
        return asList(asAdt(chunk, "ByteChunk").payloads[0]);
    }

    private static Object byteViewChunk(Object view) {
        return asAdt(view, "ByteView").payloads[0];
    }

    private static long byteViewOffset(Object view) {
        return byteOffsetLong(asAdt(view, "ByteView").payloads[1]);
    }

    private static long byteViewCount(Object view) {
        return byteCountLong(asAdt(view, "ByteView").payloads[2]);
    }

    private static long byteValueLong(Object value) {
        return asLong(asAdt(value, "Byte").payloads[0]);
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

    public static Object byteChunkFromHex(Object text) {
        String input = asString(text);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        int highNibble = -1;
        int highNibbleIndex = -1;

        for (int index = 0; index < input.length(); index += 1) {
            char value = input.charAt(index);
            if (value > 127) {
                return fixtureHexInvalidCharacter(
                    input,
                    index,
                    bytes.size(),
                    highNibble >= 0 ? "low" : "high"
                );
            }
            if (isAsciiWhitespace(value)) {
                if (highNibble >= 0) {
                    return fixtureHexInvalidCharacter(input, index, bytes.size(), "low");
                }
                continue;
            }

            int nibble = hexNibble(value);
            if (nibble < 0) {
                return fixtureHexInvalidCharacter(
                    input,
                    index,
                    bytes.size(),
                    highNibble >= 0 ? "low" : "high"
                );
            }
            if (highNibble < 0) {
                highNibble = nibble;
                highNibbleIndex = index;
            } else {
                bytes.add(byteObject((highNibble << 4) | nibble));
                highNibble = -1;
                highNibbleIndex = -1;
            }
        }

        if (highNibble >= 0) {
            return fixtureHexOddLength(input, highNibbleIndex, bytes.size());
        }
        return ok(byteChunkObject(bytes));
    }

    private static Object fixtureHexInvalidCharacter(
        String input,
        int textIndex,
        int byteOffset,
        String nibble
    ) {
        String message = "fixture.hex.invalid_character: expected ASCII hex digit at byte offset "
            + byteOffset
            + " "
            + nibble
            + " nibble";
        registerFixtureHexDiagnostic(
            message,
            "fixture.hex.invalid_character",
            input,
            textIndex,
            textIndex + 1,
            byteOffset,
            nibble
        );
        return err(message);
    }

    private static Object fixtureHexOddLength(String input, int textIndex, int byteOffset) {
        String message = "fixture.hex.odd_length: dangling hex nibble at byte offset "
            + byteOffset
            + " high nibble";
        registerFixtureHexDiagnostic(
            message,
            "fixture.hex.odd_length",
            input,
            textIndex,
            textIndex + 1,
            byteOffset,
            "high"
        );
        return err(message);
    }

    private static boolean isAsciiWhitespace(char value) {
        return value == ' '
            || value == '\t'
            || value == '\n'
            || value == '\r'
            || value == '\f'
            || value == 11;
    }

    private static int hexNibble(char value) {
        if (value >= '0' && value <= '9') {
            return value - '0';
        }
        if (value >= 'a' && value <= 'f') {
            return value - 'a' + 10;
        }
        if (value >= 'A' && value <= 'F') {
            return value - 'A' + 10;
        }
        return -1;
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

    public static Object byteView(Object chunk, Object offset, Object count) {
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        long requestedOffset = byteOffsetLong(offset);
        long requestedCount = byteCountLong(count);
        if (requestedOffset < 0) {
            return err("byte offset must be non-negative");
        }
        if (requestedCount < 0) {
            return err("byte count must be non-negative");
        }
        if (requestedOffset > bytes.size()) {
            return err("byte view offset exceeds chunk length");
        }
        if (requestedCount > bytes.size() - requestedOffset) {
            return err("byte view range exceeds chunk length");
        }
        return ok(byteViewObject(chunk, requestedOffset, requestedCount));
    }

    public static Object byteViewToChunk(Object view) {
        java.util.List<Object> bytes = byteChunkBytes(byteViewChunk(view));
        long offset = byteViewOffset(view);
        long count = byteViewCount(view);
        if (offset < 0 || count < 0 || offset > bytes.size() || count > bytes.size() - offset) {
            return byteChunkObject(java.util.Collections.emptyList());
        }
        return byteChunkObject(bytes.subList((int) offset, (int) (offset + count)));
    }

    public static Object byteReadU8Be(Object view) {
        return byteReadUnsignedBe(view, 1, 0xffL, "byte_read_u8_be");
    }

    public static Object byteExpectFixedU8Be(
        Object view,
        Object expected,
        Object schemaName,
        Object fieldName
    ) {
        long expectedValue = asLong(expected);
        if (expectedValue < 0 || expectedValue > 0xffL) {
            return err("byte_expect_fixed_u8_be expected value must be between 0 and 255");
        }
        Object read = byteReadUnsignedBe(view, 1, 0xffL, "byte_read_u8_be");
        if (isErr(read)) {
            return read;
        }
        long actualValue = asLong(unwrapOk(read));
        if (actualValue == expectedValue) {
            return read;
        }

        long offset = byteViewOffset(view);
        String schema = asString(schemaName);
        String field = asString(fieldName);
        String message = "fixed field mismatch at byte offset " + offset;
        registerSchemaFixedFieldMismatchDiagnostic(
            message,
            offset,
            schema,
            field,
            expectedValue,
            actualValue,
            byteViewContextHex(view, 8)
        );
        return err(message);
    }

    public static Object byteDecodeHttp2FrameHeader(Object view) {
        Object boundsError = byteViewBoundsError(
            byteChunkBytes(byteViewChunk(view)),
            byteViewOffset(view),
            byteViewCount(view)
        );
        if (boundsError != null) {
            return boundsError;
        }

        Object lengthRead = schemaReadUnsignedBe(
            view,
            0,
            3,
            0xffffffL,
            "Http2FrameHeader",
            "length"
        );
        if (isErr(lengthRead)) {
            return lengthRead;
        }
        Object kindRead = schemaReadUnsignedBe(
            view,
            3,
            1,
            0xffL,
            "Http2FrameHeader",
            "kind"
        );
        if (isErr(kindRead)) {
            return kindRead;
        }
        Object flagsRead = schemaReadUnsignedBe(
            view,
            4,
            1,
            0xffL,
            "Http2FrameHeader",
            "flags"
        );
        if (isErr(flagsRead)) {
            return flagsRead;
        }
        Object streamRead = schemaReadUnsignedBe(
            view,
            5,
            4,
            0xffffffffL,
            "Http2FrameHeader",
            "stream_id"
        );
        if (isErr(streamRead)) {
            return streamRead;
        }

        long rawStream = asLong(unwrapOk(streamRead));
        long actualReserved = (rawStream >>> 31) & 0x01L;
        if (actualReserved != 0L) {
            long offset = byteViewOffset(view) + 5L;
            String message = "reserved bits mismatch at byte offset " + offset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                offset,
                "Http2FrameHeader",
                "stream_reserved",
                1L,
                0L,
                actualReserved,
                byteViewContextHex(view, 8)
            );
            return err(message);
        }

        return ok(record(
            "length",
            unwrapOk(lengthRead),
            "kind",
            unwrapOk(kindRead),
            "flags",
            unwrapOk(flagsRead),
            "stream_id",
            Long.valueOf(rawStream & 0x7fffffffL)
        ));
    }

    public static Object byteDecodeHttp2Frame(Object view) {
        Object headerRead = byteDecodeHttp2FrameHeader(view);
        if (isErr(headerRead)) {
            return headerRead;
        }

        java.util.Map<Object, Object> header = asMap(unwrapOk(headerRead));
        long length = asLong(header.get("length"));
        long baseOffset = byteViewOffset(view);
        long count = byteViewCount(view);
        long headerCount = 9L;
        long availablePayload = Math.max(0L, count - headerCount);
        if (length > availablePayload) {
            long byteOffset = baseOffset + headerCount + availablePayload;
            String message = "payload length out of bounds at byte offset " + byteOffset;
            registerSchemaLengthOutOfBoundsDiagnostic(
                message,
                byteOffset,
                "Http2FrameHeader",
                "payload",
                length,
                availablePayload,
                byteViewContextHex(view, 8)
            );
            return err(message);
        }

        Object payload = byteViewObject(byteViewChunk(view), baseOffset + headerCount, length);
        return ok(record(
            "length",
            header.get("length"),
            "kind",
            header.get("kind"),
            "flags",
            header.get("flags"),
            "stream_id",
            header.get("stream_id"),
            "payload",
            payload
        ));
    }

    public static Object byteDecodeSchemaWidthSample(Object view) {
        Object shortRead = schemaReadUnsignedBe(
            view,
            0,
            2,
            0xffffL,
            "SchemaWidthSample",
            "short_value"
        );
        if (isErr(shortRead)) {
            return shortRead;
        }
        Object wideRead = schemaReadUnsignedBe(
            view,
            2,
            4,
            0xffffffffL,
            "SchemaWidthSample",
            "wide_value"
        );
        if (isErr(wideRead)) {
            return wideRead;
        }

        return ok(record(
            "short_value",
            unwrapOk(shortRead),
            "wide_value",
            unwrapOk(wideRead)
        ));
    }

    public static Object byteDecodeSchemaValidationSample(Object view) {
        Object lengthRead = schemaReadUnsignedBe(
            view,
            0,
            3,
            0xffffffL,
            "SchemaValidationSample",
            "length"
        );
        if (isErr(lengthRead)) {
            return lengthRead;
        }
        Object paddingLengthRead = schemaReadUnsignedBe(
            view,
            3,
            1,
            0xffL,
            "SchemaValidationSample",
            "padding_length"
        );
        if (isErr(paddingLengthRead)) {
            return paddingLengthRead;
        }

        long length = asLong(unwrapOk(lengthRead));
        long paddingLength = asLong(unwrapOk(paddingLengthRead));
        if (paddingLength > length) {
            long byteOffset = byteViewOffset(view) + 3L;
            String predicate = "padding_length <= length";
            String message = "schema validation failed at byte offset " + byteOffset;
            registerSchemaValidationFailedDiagnostic(
                message,
                byteOffset,
                "SchemaValidationSample",
                "padding_length",
                predicate,
                paddingLength,
                length,
                paddingLength,
                byteViewContextHex(view, 8)
            );
            return err(message);
        }

        return ok(record(
            "length",
            unwrapOk(lengthRead),
            "padding_length",
            unwrapOk(paddingLengthRead)
        ));
    }

    public static Object http2ProtocolClosedWithPending(
        Object offset,
        Object pendingCount,
        Object activeContinuation
    ) {
        long byteOffset = asLong(offset);
        long pending = asLong(pendingCount);
        String continuation = asString(activeContinuation);
        String message = "HTTP/2 input ended with " + pending
            + " pending byte(s) at byte offset " + byteOffset;
        registerHttp2ProtocolClosedWithPendingDiagnostic(
            message,
            byteOffset,
            pending,
            continuation
        );
        return err(message);
    }

    public static Object http2ProtocolContinuationExpected(
        Object offset,
        Object actualKind,
        Object actualStream,
        Object expectedStream,
        Object startedKind,
        Object startedOffset,
        Object activeContinuation
    ) {
        long byteOffset = asLong(offset);
        long frameKind = asLong(actualKind);
        long streamId = asLong(actualStream);
        long expectedStreamId = asLong(expectedStream);
        long startedFrameKind = asLong(startedKind);
        long startedByteOffset = asLong(startedOffset);
        String continuation = asString(activeContinuation);
        String message = "HTTP/2 expected CONTINUATION frame at byte offset " + byteOffset;
        registerHttp2ProtocolContinuationExpectedDiagnostic(
            message,
            byteOffset,
            frameKind,
            streamId,
            expectedStreamId,
            startedFrameKind,
            startedByteOffset,
            continuation
        );
        return err(message);
    }

    public static Object byteReadU16Be(Object view) {
        return byteReadUnsignedBe(view, 2, 0xffffL, "byte_read_u16_be");
    }

    public static Object byteReadU24Be(Object view) {
        return byteReadUnsignedBe(view, 3, 0xffffffL, "byte_read_u24_be");
    }

    public static Object byteReadU31Be(Object view) {
        return byteReadUnsignedBe(view, 4, 0x7fffffffL, "byte_read_u31_be");
    }

    public static Object byteReadU32Be(Object view) {
        return byteReadUnsignedBe(view, 4, 0xffffffffL, "byte_read_u32_be");
    }

    private static Object byteReadUnsignedBe(Object view, int width, long maxValue, String name) {
        java.util.List<Object> bytes = byteChunkBytes(byteViewChunk(view));
        long offset = byteViewOffset(view);
        long count = byteViewCount(view);
        Object boundsError = byteViewBoundsError(bytes, offset, count);
        if (boundsError != null) {
            return boundsError;
        }
        if (count < width) {
            String message = "byte read requires " + width + " bytes but view has " + count;
            registerCodecIncompleteInputDiagnostic(message, offset + count, width, count);
            return err(message);
        }
        long value = 0;
        for (int index = 0; index < width; index += 1) {
            value = (value << 8) | byteValueLong(bytes.get((int) offset + index));
        }
        if (value > maxValue) {
            return err(name + " value exceeds maximum " + maxValue);
        }
        return ok(Long.valueOf(value));
    }

    private static Object schemaReadUnsignedBe(
        Object view,
        int relativeOffset,
        int width,
        long maxValue,
        String schemaName,
        String fieldName
    ) {
        java.util.List<Object> bytes = byteChunkBytes(byteViewChunk(view));
        long baseOffset = byteViewOffset(view);
        long count = byteViewCount(view);
        Object boundsError = byteViewBoundsError(bytes, baseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        if (available < width) {
            long byteOffset = baseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                fieldName,
                width,
                available,
                byteViewContextHex(view, 8)
            );
            return err(message);
        }

        long value = 0;
        for (int index = 0; index < width; index += 1) {
            value = (value << 8) | byteValueLong(bytes.get((int) baseOffset + relativeOffset + index));
        }
        if (value > maxValue) {
            return err("schema field `" + fieldName + "` value exceeds maximum " + maxValue);
        }
        return ok(Long.valueOf(value));
    }

    private static String byteViewContextHex(Object view, int radius) {
        java.util.List<Object> bytes = byteChunkBytes(byteViewChunk(view));
        long offset = byteViewOffset(view);
        long count = byteViewCount(view);
        Object boundsError = byteViewBoundsError(bytes, offset, count);
        if (boundsError != null) {
            return "";
        }
        long start = Math.max(0, offset - radius);
        long end = Math.min(bytes.size(), offset + Math.max(1, count) + radius);
        StringBuilder out = new StringBuilder((int) ((end - start) * 2));
        char[] digits = "0123456789abcdef".toCharArray();
        for (long index = start; index < end; index += 1) {
            int value = (int) byteValueLong(bytes.get((int) index));
            out.append(digits[value >>> 4]);
            out.append(digits[value & 0x0f]);
        }
        return out.toString();
    }

    private static Object byteViewBoundsError(
        java.util.List<Object> bytes,
        long offset,
        long count
    ) {
        if (offset < 0) {
            return err("byte offset must be non-negative");
        }
        if (count < 0) {
            return err("byte count must be non-negative");
        }
        if (offset > bytes.size()) {
            return err("byte view offset exceeds chunk length");
        }
        if (count > bytes.size() - offset) {
            return err("byte view range exceeds chunk length");
        }
        return null;
    }

    public static Object byteWriteU8Be(Object value) {
        return byteWriteUnsignedBe(value, 1, 0xffL, "byte_write_u8_be");
    }

    public static Object byteWriteU16Be(Object value) {
        return byteWriteUnsignedBe(value, 2, 0xffffL, "byte_write_u16_be");
    }

    public static Object byteWriteU24Be(Object value) {
        return byteWriteUnsignedBe(value, 3, 0xffffffL, "byte_write_u24_be");
    }

    public static Object byteWriteU31Be(Object value) {
        return byteWriteUnsignedBe(value, 4, 0x7fffffffL, "byte_write_u31_be");
    }

    public static Object byteWriteU32Be(Object value) {
        return byteWriteUnsignedBe(value, 4, 0xffffffffL, "byte_write_u32_be");
    }

    private static Object byteWriteUnsignedBe(Object value, int width, long maxValue, String name) {
        long unsigned = asLong(value);
        if (unsigned < 0 || unsigned > maxValue) {
            return err(name + " value must be between 0 and " + maxValue);
        }
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>(width);
        for (int index = width - 1; index >= 0; index -= 1) {
            bytes.add(byteObject((unsigned >> (index * 8)) & 0xffL));
        }
        return ok(byteChunkObject(bytes));
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
