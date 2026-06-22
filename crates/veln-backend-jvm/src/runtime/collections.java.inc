    private static final class FrozenByteView {
        private final Object chunk;
        private final long storageOffset;
        private final long logicalOffset;
        private final long count;

        private FrozenByteView(Object chunk, long storageOffset, long logicalOffset, long count) {
            this.chunk = chunk;
            this.storageOffset = storageOffset;
            this.logicalOffset = logicalOffset;
            this.count = count;
        }

        @Override
        public String toString() {
            return "ByteView("
                + format(chunk)
                + ", "
                + format(byteOffsetObject(logicalOffset))
                + ", "
                + format(byteCountObject(count))
                + ")";
        }
    }

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

    private static Object frozenByteViewObject(
        Object chunk,
        long storageOffset,
        long logicalOffset,
        long count
    ) {
        return new FrozenByteView(chunk, storageOffset, logicalOffset, count);
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

    private static boolean isByteViewValue(Object value) {
        return value instanceof FrozenByteView || isAdtNamed(value, "ByteView");
    }

    private static Object freezeByteView(Object view) {
        if (view instanceof FrozenByteView) {
            return view;
        }
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageOffset, count);
        if (boundsError != null) {
            return view;
        }
        Object chunk = byteChunkObject(
            bytes.subList((int) storageOffset, (int) (storageOffset + count))
        );
        return frozenByteViewObject(chunk, 0L, byteViewOffset(view), count);
    }

    private static Object byteViewChunk(Object view) {
        if (view instanceof FrozenByteView) {
            return ((FrozenByteView) view).chunk;
        }
        return asAdt(view, "ByteView").payloads[0];
    }

    private static long byteViewStorageOffset(Object view) {
        if (view instanceof FrozenByteView) {
            return ((FrozenByteView) view).storageOffset;
        }
        return byteViewOffset(view);
    }

    private static long byteViewOffset(Object view) {
        if (view instanceof FrozenByteView) {
            return ((FrozenByteView) view).logicalOffset;
        }
        return byteOffsetLong(asAdt(view, "ByteView").payloads[1]);
    }

    private static long byteViewCountLong(Object view) {
        if (view instanceof FrozenByteView) {
            return ((FrozenByteView) view).count;
        }
        return byteCountLong(asAdt(view, "ByteView").payloads[2]);
    }

    private static java.util.List<Object> byteViewBytes(Object view) {
        return byteChunkBytes(byteViewChunk(view));
    }

    private static Object byteViewDerivedObject(
        Object view,
        long relativeOffset,
        long count
    ) {
        if (view instanceof FrozenByteView) {
            return frozenByteViewObject(
                byteViewChunk(view),
                byteViewStorageOffset(view) + relativeOffset,
                byteViewOffset(view) + relativeOffset,
                count
            );
        }
        return byteViewObject(
            byteViewChunk(view),
            byteViewOffset(view) + relativeOffset,
            count
        );
    }

    private static long byteValueLong(Object value) {
        return asLong(asAdt(value, "Byte").payloads[0]);
    }

    private interface SchemaIntExpr {
        long eval(java.util.Map<String, Long> decoded);

        void collectRefs(java.util.Set<String> refs);
    }

    private interface SchemaBoolExpr {
        boolean eval(java.util.Map<String, Long> decoded);

        void collectRefs(java.util.Set<String> refs);
    }

    private static final class SchemaFieldSpec {
        final String name;
        final int width;
        final long maxValue;
        final boolean littleEndian;
        final String flagType;
        final SchemaPredicate predicate;
        final SchemaDispatchSpec dispatch;
        final SchemaRepeatSpec repeat;
        final String lengthField;
        final int reservedBitWidth;
        final long reservedValue;
        final Long fixedValue;

        SchemaFieldSpec(String name, int width, long maxValue, SchemaPredicate predicate) {
            this(name, width, maxValue, false, "", predicate, null, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            SchemaPredicate predicate
        ) {
            this(name, width, maxValue, littleEndian, "", predicate, null, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            SchemaPredicate predicate,
            Long fixedValue
        ) {
            this(name, width, maxValue, littleEndian, "", predicate, null, null, "", 0, 0L, fixedValue);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            boolean flag8,
            SchemaPredicate predicate
        ) {
            this(
                name,
                width,
                maxValue,
                littleEndian,
                flag8 ? "Flag8" : "",
                predicate,
                null,
                null,
                "",
                0,
                0L,
                null
            );
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            String flagType,
            SchemaPredicate predicate
        ) {
            this(name, width, maxValue, littleEndian, flagType, predicate, null, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            boolean flag8,
            SchemaPredicate predicate,
            Long fixedValue
        ) {
            this(
                name,
                width,
                maxValue,
                littleEndian,
                flag8 ? "Flag8" : "",
                predicate,
                null,
                null,
                "",
                0,
                0L,
                fixedValue
            );
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            String flagType,
            SchemaPredicate predicate,
            Long fixedValue
        ) {
            this(name, width, maxValue, littleEndian, flagType, predicate, null, null, "", 0, 0L, fixedValue);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            SchemaPredicate predicate,
            SchemaDispatchSpec dispatch
        ) {
            this(name, width, maxValue, false, "", predicate, dispatch, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(String name, String lengthField) {
            this(name, 0, 0L, false, "", null, null, null, lengthField, 0, 0L, null);
        }

        SchemaFieldSpec(String name, SchemaRepeatSpec repeat) {
            this(name, 0, 0L, false, "", null, null, repeat, "", 0, 0L, null);
        }

        SchemaFieldSpec(String name, int reservedBitWidth, long reservedValue) {
            this(name, 0, 0L, false, "", null, null, null, "", reservedBitWidth, reservedValue, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            String flagType,
            SchemaPredicate predicate,
            SchemaDispatchSpec dispatch,
            SchemaRepeatSpec repeat,
            String lengthField,
            int reservedBitWidth,
            long reservedValue,
            Long fixedValue
        ) {
            this.name = name;
            this.width = width;
            this.maxValue = maxValue;
            this.littleEndian = littleEndian;
            this.flagType = flagType;
            this.predicate = predicate;
            this.dispatch = dispatch;
            this.repeat = repeat;
            this.lengthField = lengthField;
            this.reservedBitWidth = reservedBitWidth;
            this.reservedValue = reservedValue;
            this.fixedValue = fixedValue;
        }

        boolean isReserved() {
            return reservedBitWidth > 0;
        }

        boolean isLengthBoundedView() {
            return dispatch == null && repeat == null && !lengthField.isEmpty();
        }

        boolean isRepeat() {
            return repeat != null;
        }

        boolean isFlagBitset() {
            return dispatch == null
                && repeat == null
                && lengthField.isEmpty()
                && reservedBitWidth == 0
                && !flagType.isEmpty();
        }
    }

    private static final class SchemaRepeatSpec {
        final String countField;
        final int width;
        final long maxValue;
        final boolean littleEndian;
        final String byteViewLengthField;
        final BinarySchemaSpec payloadSchema;

        SchemaRepeatSpec(String countField, int width, long maxValue, boolean littleEndian) {
            this(countField, width, maxValue, littleEndian, "", null);
        }

        SchemaRepeatSpec(
            String countField,
            int width,
            long maxValue,
            boolean littleEndian,
            String byteViewLengthField,
            BinarySchemaSpec payloadSchema
        ) {
            this.countField = countField;
            this.width = width;
            this.maxValue = maxValue;
            this.littleEndian = littleEndian;
            this.byteViewLengthField = byteViewLengthField;
            this.payloadSchema = payloadSchema;
        }

        boolean isByteViewPayload() {
            return !byteViewLengthField.isEmpty();
        }
    }

    private static int reservedByteWidth(int bitWidth) {
        return bitWidth > 0 && bitWidth % 8 == 0 ? bitWidth / 8 : 0;
    }

    private static long reservedMaxValue(int bitWidth) {
        return (1L << bitWidth) - 1L;
    }

    private static boolean supportedReservedBits(int bitWidth, long reservedValue) {
        if (bitWidth <= 0 || bitWidth > 63) {
            return false;
        }
        if (bitWidth < 8
            || (bitWidth > 8 && bitWidth < 16)
            || (bitWidth > 16 && bitWidth < 24)
            || (bitWidth > 24 && bitWidth < 32)
            || (bitWidth > 32 && bitWidth < 40)
            || (bitWidth > 40 && bitWidth < 48)
            || (bitWidth > 48 && bitWidth < 56)
            || (bitWidth > 56 && bitWidth < 64)) {
            return reservedValue >= 0L && reservedValue <= reservedMaxValue(bitWidth);
        }
        if (bitWidth % 8 != 0) {
            return false;
        }
        return bitWidth <= 32 && reservedValue >= 0L && reservedValue <= reservedMaxValue(bitWidth);
    }

    private static int packedReservedStorageByteWidth(int bitWidth) {
        if (bitWidth >= 1 && bitWidth <= 7) {
            return 1;
        }
        if (bitWidth >= 9 && bitWidth <= 15) {
            return 2;
        }
        if (bitWidth >= 17 && bitWidth <= 23) {
            return 3;
        }
        if (bitWidth >= 25 && bitWidth <= 31) {
            return 4;
        }
        return 0;
    }

    private static int packedVisibleBitWidth(SchemaFieldSpec reserved, SchemaFieldSpec visible) {
        if (!reserved.isReserved()
            || packedReservedStorageByteWidth(reserved.reservedBitWidth) == 0
            || visible.isReserved()
            || visible.dispatch != null
            || visible.repeat != null
            || visible.isLengthBoundedView()
            || visible.width != 1
            || visible.littleEndian) {
            return 0;
        }
        int visibleBitWidth =
            packedReservedStorageByteWidth(reserved.reservedBitWidth) * 8
                - reserved.reservedBitWidth;
        return visible.maxValue == reservedMaxValue(visibleBitWidth) ? visibleBitWidth : 0;
    }

    private static int packedStorageByteWidth(SchemaFieldSpec reserved, SchemaFieldSpec visible) {
        return packedVisibleBitWidth(reserved, visible) > 0
            ? packedReservedStorageByteWidth(reserved.reservedBitWidth)
            : 0;
    }

    private static int reservedBytePrefixVisibleBitWidth(
        SchemaFieldSpec reserved,
        SchemaFieldSpec visible
    ) {
        if (reserved == null
            || visible == null
            || !reserved.isReserved()
            || reserved.reservedBitWidth != 2
            || reserved.reservedValue != 0L
            || visible.isReserved()
            || visible.dispatch != null
            || visible.repeat != null
            || visible.isLengthBoundedView()
            || visible.isFlagBitset()
            || visible.width != 1
            || visible.maxValue != 0xffL
            || visible.littleEndian) {
            return 0;
        }
        return 8;
    }

    private static int reservedBytePrefixStorageByteWidth(
        SchemaFieldSpec reserved,
        SchemaFieldSpec visible
    ) {
        return reservedBytePrefixVisibleBitWidth(reserved, visible) > 0 ? 2 : 0;
    }

    private static int suffixPackedVisibleBitWidth(SchemaFieldSpec visible, SchemaFieldSpec reserved) {
        if (reserved == null
            || !reserved.isReserved()
            || visible.isReserved()
            || visible.dispatch != null
            || visible.repeat != null
            || visible.isLengthBoundedView()
            || visible.width != 1
            || visible.littleEndian) {
            return 0;
        }
        int storageByteWidth = packedReservedStorageByteWidth(reserved.reservedBitWidth);
        if (storageByteWidth == 0) {
            return 0;
        }
        int visibleBitWidth = storageByteWidth * 8 - reserved.reservedBitWidth;
        return visibleBitWidth > 0 && visible.maxValue == reservedMaxValue(visibleBitWidth)
            ? visibleBitWidth
            : 0;
    }

    private static int suffixPackedStorageByteWidth(SchemaFieldSpec visible, SchemaFieldSpec reserved) {
        return suffixPackedVisibleBitWidth(visible, reserved) > 0
            ? packedReservedStorageByteWidth(reserved.reservedBitWidth)
            : 0;
    }

    private static int bitPackedVisibleBitWidth(SchemaFieldSpec field) {
        if (field == null
            || field.isReserved()
            || field.dispatch != null
            || field.repeat != null
            || field.isLengthBoundedView()
            || field.isFlagBitset()
            || field.littleEndian
            || field.width < 1
            || field.width > 4) {
            return 0;
        }
        for (int bitWidth = 1; bitWidth <= 32; bitWidth += 1) {
            if (field.maxValue == reservedMaxValue(bitWidth)) {
                return bitWidth;
            }
        }
        return 0;
    }

    private static final class BitPackedFieldGroupSpec {
        final int fieldCount;
        final int storageByteWidth;
        final int totalBitWidth;

        BitPackedFieldGroupSpec(int fieldCount, int storageByteWidth, int totalBitWidth) {
            this.fieldCount = fieldCount;
            this.storageByteWidth = storageByteWidth;
            this.totalBitWidth = totalBitWidth;
        }
    }

    private static int bitPackedGroupFieldBitWidth(SchemaFieldSpec field) {
        if (field == null) {
            return 0;
        }
        if (field.isReserved()) {
            return reservedByteWidth(field.reservedBitWidth) == 0
                && supportedReservedBits(field.reservedBitWidth, field.reservedValue)
                ? field.reservedBitWidth
                : 0;
        }
        int visibleBitWidth = bitPackedVisibleBitWidth(field);
        return visibleBitWidth > 0 && visibleBitWidth % 8 != 0 ? visibleBitWidth : 0;
    }

    private static boolean bitPackedStorageBitWidth(int bitWidth) {
        return bitWidth == 8
            || bitWidth == 16
            || bitWidth == 24
            || bitWidth == 32
            || bitWidth == 40
            || bitWidth == 48
            || bitWidth == 56
            || bitWidth == 64;
    }

    private static BitPackedFieldGroupSpec bitPackedFieldGroupSpec(
        SchemaFieldSpec[] fields,
        int index
    ) {
        int totalBitWidth = 0;
        int fieldCount = 0;
        boolean hasReserved = false;
        boolean hasVisible = false;
        for (int fieldIndex = index; fieldIndex < fields.length; fieldIndex += 1) {
            SchemaFieldSpec field = fields[fieldIndex];
            int bitWidth = bitPackedGroupFieldBitWidth(field);
            if (bitWidth == 0) {
                return null;
            }
            totalBitWidth += bitWidth;
            fieldCount += 1;
            hasReserved = hasReserved || field.isReserved();
            hasVisible = hasVisible || !field.isReserved();
            if (bitPackedStorageBitWidth(totalBitWidth)) {
                return hasReserved && hasVisible
                    ? new BitPackedFieldGroupSpec(fieldCount, totalBitWidth / 8, totalBitWidth)
                    : null;
            }
            if (totalBitWidth > 64) {
                return null;
            }
        }
        return null;
    }

    private static int middleReservedGroupStorageByteWidth(SchemaFieldSpec[] fields, int index) {
        if (index + 2 >= fields.length) {
            return 0;
        }
        SchemaFieldSpec left = fields[index];
        SchemaFieldSpec reserved = fields[index + 1];
        SchemaFieldSpec right = fields[index + 2];
        if (reserved == null
            || !reserved.isReserved()
            || reservedByteWidth(reserved.reservedBitWidth) > 0
            || !supportedReservedBits(reserved.reservedBitWidth, reserved.reservedValue)) {
            return 0;
        }
        int leftBitWidth = bitPackedVisibleBitWidth(left);
        int rightBitWidth = bitPackedVisibleBitWidth(right);
        int totalBitWidth = leftBitWidth + reserved.reservedBitWidth + rightBitWidth;
        return leftBitWidth > 0
            && rightBitWidth > 0
            && leftBitWidth % 8 != 0
            && (leftBitWidth + reserved.reservedBitWidth) % 8 != 0
            && (totalBitWidth == 8
                || totalBitWidth == 16
                || totalBitWidth == 24
                || totalBitWidth == 32)
            ? totalBitWidth / 8
            : 0;
    }

    private static int byteInterleavedReservedGroupStorageByteWidth(SchemaFieldSpec[] fields, int index) {
        if (index + 3 >= fields.length) {
            return 0;
        }
        SchemaFieldSpec first = fields[index];
        SchemaFieldSpec reserved = fields[index + 1];
        SchemaFieldSpec middle = fields[index + 2];
        SchemaFieldSpec last = fields[index + 3];
        if (reserved == null
            || !reserved.isReserved()
            || reserved.reservedBitWidth <= 0
            || reserved.reservedBitWidth > 7
            || !supportedReservedBits(reserved.reservedBitWidth, reserved.reservedValue)) {
            return 0;
        }
        int firstBitWidth = bitPackedVisibleBitWidth(first);
        int middleBitWidth = bitPackedVisibleBitWidth(middle);
        int lastBitWidth = bitPackedVisibleBitWidth(last);
        int totalBitWidth =
            firstBitWidth + reserved.reservedBitWidth + middleBitWidth + lastBitWidth;
        return firstBitWidth > 0
            && firstBitWidth < 8
            && middleBitWidth == 8
            && lastBitWidth > 0
            && lastBitWidth < 8
            && (firstBitWidth + reserved.reservedBitWidth) % 8 != 0
            && totalBitWidth == 16
            ? 2
            : 0;
    }

    private static int prefixReservedGroupStorageByteWidth(SchemaFieldSpec[] fields, int index) {
        if (index + 2 >= fields.length) {
            return 0;
        }
        SchemaFieldSpec reserved = fields[index];
        SchemaFieldSpec first = fields[index + 1];
        SchemaFieldSpec second = fields[index + 2];
        if (reserved == null
            || !reserved.isReserved()
            || !supportedReservedBits(reserved.reservedBitWidth, reserved.reservedValue)) {
            return 0;
        }
        int firstBitWidth = bitPackedVisibleBitWidth(first);
        int secondBitWidth = bitPackedVisibleBitWidth(second);
        int totalBitWidth = reserved.reservedBitWidth + firstBitWidth + secondBitWidth;
        if (firstBitWidth <= 0
            || secondBitWidth <= 0
            || firstBitWidth > 8
            || secondBitWidth > 8) {
            return 0;
        }
        boolean supportedOneByteGroup =
            reservedByteWidth(reserved.reservedBitWidth) == 0
                && reserved.reservedBitWidth % 8 != 0
                && (reserved.reservedBitWidth + firstBitWidth) % 8 != 0
                && totalBitWidth == 8;
        boolean supportedTwoByteGroup = totalBitWidth == 16;
        boolean supportedThreeByteGroup =
            reserved.reservedBitWidth >= 17
                && reserved.reservedBitWidth <= 23
                && totalBitWidth == 24;
        boolean supportedFourByteGroup =
            reserved.reservedBitWidth >= 25
                && reserved.reservedBitWidth <= 31
                && totalBitWidth == 32;
        boolean supportedFiveByteGroup =
            reserved.reservedBitWidth == 33 && totalBitWidth == 40;
        boolean supportedSixByteGroup =
            reserved.reservedBitWidth == 41 && totalBitWidth == 48;
        return (supportedOneByteGroup
                || supportedTwoByteGroup
                || supportedThreeByteGroup
                || supportedFourByteGroup
                || supportedFiveByteGroup
                || supportedSixByteGroup)
            ? totalBitWidth / 8
            : 0;
    }

    private static final class SchemaDispatchSpec {
        final String tagField;
        final String lengthField;
        final boolean preservesUnknown;
        final SchemaDispatchCaseSpec[] cases;

        SchemaDispatchSpec(
            String tagField,
            String lengthField,
            boolean preservesUnknown,
            SchemaDispatchCaseSpec[] cases
        ) {
            this.tagField = tagField;
            this.lengthField = lengthField;
            this.preservesUnknown = preservesUnknown;
            this.cases = cases;
        }
    }

    private static final class SchemaDispatchCaseSpec {
        final long tag;
        final int width;
        final long maxValue;
        final boolean littleEndian;
        final BinarySchemaSpec payloadSchema;
        final String payloadSchemaName;

        SchemaDispatchCaseSpec(long tag, int width) {
            this(tag, width, false);
        }

        SchemaDispatchCaseSpec(long tag, int width, boolean littleEndian) {
            this.tag = tag;
            this.width = width;
            this.littleEndian = littleEndian;
            this.payloadSchema = null;
            this.payloadSchemaName = "";
            long value = 0L;
            for (int index = 0; index < width; index += 1) {
                value = (value << 8) | 0xffL;
            }
            this.maxValue = value;
        }

        SchemaDispatchCaseSpec(long tag, BinarySchemaSpec payloadSchema) {
            this.tag = tag;
            this.width = 0;
            this.maxValue = 0L;
            this.littleEndian = false;
            this.payloadSchema = payloadSchema;
            this.payloadSchemaName = "";
        }

        SchemaDispatchCaseSpec(long tag, String payloadSchemaName) {
            this.tag = tag;
            this.width = 0;
            this.maxValue = 0L;
            this.littleEndian = false;
            this.payloadSchema = null;
            this.payloadSchemaName = payloadSchemaName;
        }

        BinarySchemaSpec payloadSchema(BinarySchemaSpec currentSchema) {
            if (payloadSchema != null) {
                return payloadSchema;
            }
            if (payloadSchemaName.equals(currentSchema.name)) {
                return currentSchema;
            }
            return null;
        }

        long width(BinarySchemaSpec currentSchema) {
            if (payloadSchema != null) {
                return schemaTotalWidth(payloadSchema);
            }
            if (payloadSchemaName.equals(currentSchema.name)) {
                return 0L;
            }
            return width;
        }
    }

    private static final class SchemaMappingFieldSpec {
        final String target;
        final Object expr;

        SchemaMappingFieldSpec(String target, Object expr) {
            this.target = target;
            this.expr = expr;
        }
    }

    private static final class SchemaMappingSpec {
        final String selectorField;
        final String selectorOperator;
        final Long selectorValue;
        final SchemaPredicate selectorPredicate;
        final SchemaMappingFieldSpec[] fields;

        SchemaMappingSpec(SchemaMappingFieldSpec[] fields) {
            this("", "==", null, fields);
        }

        SchemaMappingSpec(String selectorText, SchemaMappingFieldSpec[] fields) {
            this.selectorField = "";
            this.selectorOperator = "==";
            this.selectorValue = null;
            this.selectorPredicate = new SchemaPredicateParser(selectorText).parse();
            this.fields = fields;
        }

        SchemaMappingSpec(
            String selectorField,
            String selectorOperator,
            Long selectorValue,
            SchemaMappingFieldSpec[] fields
        ) {
            this.selectorField = selectorField;
            this.selectorOperator = selectorOperator;
            this.selectorValue = selectorValue;
            this.selectorPredicate = selectorValue == null
                ? null
                : schemaPredicate(
                    selectorField + " " + selectorOperator + " " + selectorValue,
                    schemaCompare(
                        selectorOperator,
                        schemaRef(selectorField),
                        schemaLiteral(selectorValue.longValue())
                    )
                );
            this.fields = fields;
        }

        boolean matches(java.util.Map<?, Object> decoded) {
            if (selectorPredicate == null) {
                return true;
            }
            java.util.LinkedHashMap<String, Long> ints = new java.util.LinkedHashMap<String, Long>();
            for (java.util.Map.Entry<?, Object> entry : decoded.entrySet()) {
                if (entry.getKey() instanceof String && entry.getValue() instanceof Long) {
                    ints.put((String) entry.getKey(), (Long) entry.getValue());
                }
            }
            try {
                return selectorPredicate.eval(ints);
            } catch (IllegalStateException failure) {
                return false;
            }
        }
    }

    private static final class SchemaEncodeMappingValue {
        final String source;
        final Object value;

        SchemaEncodeMappingValue(String source, Object value) {
            this.source = source;
            this.value = value;
        }
    }

    private static final class BinarySchemaSpec {
        final String name;
        final SchemaFieldSpec[] fields;
        final SchemaPredicate validation;
        final SchemaMappingSpec[] mapping;

        BinarySchemaSpec(String name, SchemaFieldSpec[] fields) {
            this(name, fields, null, new SchemaMappingSpec[0]);
        }

        BinarySchemaSpec(
            String name,
            SchemaFieldSpec[] fields,
            SchemaMappingSpec[] mapping
        ) {
            this(name, fields, null, mapping);
        }

        BinarySchemaSpec(
            String name,
            SchemaFieldSpec[] fields,
            SchemaPredicate validation,
            SchemaMappingSpec[] mapping
        ) {
            this.name = name;
            this.fields = fields;
            this.validation = validation;
            this.mapping = mapping;
        }
    }

    private static final class SchemaPredicate {
        final String text;
        final SchemaBoolExpr expr;

        SchemaPredicate(String text, SchemaBoolExpr expr) {
            this.text = text;
            this.expr = expr;
        }

        boolean eval(java.util.Map<String, Long> decoded) {
            return expr.eval(decoded);
        }

        java.util.Set<String> refs() {
            java.util.LinkedHashSet<String> refs = new java.util.LinkedHashSet<String>();
            expr.collectRefs(refs);
            return refs;
        }
    }

    private static final class SchemaPredicateParser {
        final String input;
        int pos;

        SchemaPredicateParser(String input) {
            this.input = input;
            this.pos = 0;
        }

        SchemaPredicate parse() {
            SchemaBoolExpr expr = parseOr();
            skipWhitespace();
            if (!isAtEnd()) {
                throw new IllegalStateException(
                    "unsupported schema predicate syntax near `" + input.substring(pos) + "`"
                );
            }
            return new SchemaPredicate(input, expr);
        }

        private SchemaBoolExpr parseOr() {
            SchemaBoolExpr expr = parseAnd();
            while (eatKeyword("or")) {
                expr = new SchemaBoolOp("or", expr, parseAnd());
            }
            return expr;
        }

        private SchemaBoolExpr parseAnd() {
            SchemaBoolExpr expr = parseBoolUnary();
            while (eatKeyword("and")) {
                expr = new SchemaBoolOp("and", expr, parseBoolUnary());
            }
            return expr;
        }

        private SchemaBoolExpr parseBoolUnary() {
            if (eatKeyword("not")) {
                return new SchemaBoolNot(parseBoolUnary());
            }
            return parseBoolAtom();
        }

        private SchemaBoolExpr parseBoolAtom() {
            if (eatKeyword("true")) {
                return new SchemaBoolLiteral(true);
            }
            if (eatKeyword("false")) {
                return new SchemaBoolLiteral(false);
            }
            if (peekSymbol("(")) {
                int start = pos;
                try {
                    eatSymbol("(");
                    SchemaBoolExpr expr = parseOr();
                    if (!eatSymbol(")")) {
                        throw new IllegalStateException(
                            "unsupported schema predicate syntax: expected `)`"
                        );
                    }
                    return expr;
                } catch (IllegalStateException failure) {
                    pos = start;
                }
            }
            SchemaIntExpr left = parseAddSub();
            String op = parseComparisonOperator();
            if (op == null) {
                throw new IllegalStateException(
                    "unsupported schema predicate syntax near `" + input.substring(pos) + "`"
                );
            }
            return new SchemaCompare(op, left, parseAddSub());
        }

        private SchemaIntExpr parseAddSub() {
            SchemaIntExpr expr = parseMulDiv();
            while (true) {
                if (eatSymbol("+")) {
                    expr = new SchemaArithmetic("+", expr, parseMulDiv());
                } else if (eatSymbol("-")) {
                    expr = new SchemaArithmetic("-", expr, parseMulDiv());
                } else {
                    return expr;
                }
            }
        }

        private SchemaIntExpr parseMulDiv() {
            SchemaIntExpr expr = parseIntAtom();
            while (true) {
                if (eatSymbol("*")) {
                    expr = new SchemaArithmetic("*", expr, parseIntAtom());
                } else if (eatSymbol("/")) {
                    expr = new SchemaArithmetic("/", expr, parseIntAtom());
                } else {
                    return expr;
                }
            }
        }

        private SchemaIntExpr parseIntAtom() {
            skipWhitespace();
            if (eatSymbol("-")) {
                return new SchemaNegate(parseIntAtom());
            }
            if (eatSymbol("(")) {
                SchemaIntExpr expr = parseAddSub();
                if (!eatSymbol(")")) {
                    throw new IllegalStateException("unsupported schema predicate syntax: expected `)`");
                }
                return expr;
            }
            if (!isAtEnd() && Character.isDigit(input.charAt(pos))) {
                int start = pos;
                while (!isAtEnd() && Character.isDigit(input.charAt(pos))) {
                    pos += 1;
                }
                return new SchemaLiteral(Long.parseLong(input.substring(start, pos)));
            }
            String name = parseIdent();
            if (name != null) {
                return new SchemaRef(name);
            }
            throw new IllegalStateException(
                "unsupported schema predicate syntax near `" + input.substring(pos) + "`"
            );
        }

        private String parseComparisonOperator() {
            for (String op : new String[] { "<=", ">=", "==", "!=", "<", ">" }) {
                if (eatSymbol(op)) {
                    return op;
                }
            }
            return null;
        }

        private boolean eatKeyword(String keyword) {
            skipWhitespace();
            int end = pos + keyword.length();
            if (end > input.length() || !input.substring(pos, end).equals(keyword)) {
                return false;
            }
            if (end < input.length()) {
                char next = input.charAt(end);
                if (Character.isLetterOrDigit(next) || next == '_') {
                    return false;
                }
            }
            pos = end;
            return true;
        }

        private boolean eatSymbol(String symbol) {
            skipWhitespace();
            if (!input.startsWith(symbol, pos)) {
                return false;
            }
            pos += symbol.length();
            return true;
        }

        private boolean peekSymbol(String symbol) {
            skipWhitespace();
            return input.startsWith(symbol, pos);
        }

        private String parseIdent() {
            skipWhitespace();
            if (isAtEnd()) {
                return null;
            }
            char first = input.charAt(pos);
            if (!(Character.isLetter(first) || first == '_')) {
                return null;
            }
            int start = pos;
            pos += 1;
            while (!isAtEnd()) {
                char ch = input.charAt(pos);
                if (!(Character.isLetterOrDigit(ch) || ch == '_')) {
                    break;
                }
                pos += 1;
            }
            return input.substring(start, pos);
        }

        private void skipWhitespace() {
            while (!isAtEnd() && Character.isWhitespace(input.charAt(pos))) {
                pos += 1;
            }
        }

        private boolean isAtEnd() {
            return pos >= input.length();
        }
    }

    private static final class SchemaLiteral implements SchemaIntExpr {
        final long value;

        SchemaLiteral(long value) {
            this.value = value;
        }

        public long eval(java.util.Map<String, Long> decoded) {
            return value;
        }

        public void collectRefs(java.util.Set<String> refs) {}
    }

    private static final class SchemaRef implements SchemaIntExpr {
        final String name;

        SchemaRef(String name) {
            this.name = name;
        }

        public long eval(java.util.Map<String, Long> decoded) {
            Long value = decoded.get(name);
            if (value == null) {
                throw new IllegalStateException("schema predicate reference `" + name + "` is not decoded");
            }
            return value.longValue();
        }

        public void collectRefs(java.util.Set<String> refs) {
            refs.add(name);
        }
    }

    private static final class SchemaArithmetic implements SchemaIntExpr {
        final String op;
        final SchemaIntExpr left;
        final SchemaIntExpr right;

        SchemaArithmetic(String op, SchemaIntExpr left, SchemaIntExpr right) {
            this.op = op;
            this.left = left;
            this.right = right;
        }

        public long eval(java.util.Map<String, Long> decoded) {
            long leftValue = left.eval(decoded);
            long rightValue = right.eval(decoded);
            if (op.equals("+")) {
                return leftValue + rightValue;
            }
            if (op.equals("-")) {
                return leftValue - rightValue;
            }
            if (op.equals("*")) {
                return leftValue * rightValue;
            }
            if (op.equals("/")) {
                if (rightValue == 0L) {
                    throw new IllegalStateException("schema predicate division by zero");
                }
                return leftValue / rightValue;
            }
            throw new IllegalStateException("unsupported schema arithmetic operator `" + op + "`");
        }

        public void collectRefs(java.util.Set<String> refs) {
            left.collectRefs(refs);
            right.collectRefs(refs);
        }
    }

    private static final class SchemaNegate implements SchemaIntExpr {
        final SchemaIntExpr expr;

        SchemaNegate(SchemaIntExpr expr) {
            this.expr = expr;
        }

        public long eval(java.util.Map<String, Long> decoded) {
            return -expr.eval(decoded);
        }

        public void collectRefs(java.util.Set<String> refs) {
            expr.collectRefs(refs);
        }
    }

    private static final class SchemaCompare implements SchemaBoolExpr {
        final String op;
        final SchemaIntExpr left;
        final SchemaIntExpr right;

        SchemaCompare(String op, SchemaIntExpr left, SchemaIntExpr right) {
            this.op = op;
            this.left = left;
            this.right = right;
        }

        public boolean eval(java.util.Map<String, Long> decoded) {
            long leftValue = left.eval(decoded);
            long rightValue = right.eval(decoded);
            if (op.equals("<")) {
                return leftValue < rightValue;
            }
            if (op.equals("<=")) {
                return leftValue <= rightValue;
            }
            if (op.equals(">")) {
                return leftValue > rightValue;
            }
            if (op.equals(">=")) {
                return leftValue >= rightValue;
            }
            if (op.equals("==")) {
                return leftValue == rightValue;
            }
            if (op.equals("!=")) {
                return leftValue != rightValue;
            }
            throw new IllegalStateException("unsupported schema comparison operator `" + op + "`");
        }

        public void collectRefs(java.util.Set<String> refs) {
            left.collectRefs(refs);
            right.collectRefs(refs);
        }
    }

    private static final class SchemaBoolLiteral implements SchemaBoolExpr {
        final boolean value;

        SchemaBoolLiteral(boolean value) {
            this.value = value;
        }

        public boolean eval(java.util.Map<String, Long> decoded) {
            return value;
        }

        public void collectRefs(java.util.Set<String> refs) {}
    }

    private static final class SchemaBoolNot implements SchemaBoolExpr {
        final SchemaBoolExpr expr;

        SchemaBoolNot(SchemaBoolExpr expr) {
            this.expr = expr;
        }

        public boolean eval(java.util.Map<String, Long> decoded) {
            return !expr.eval(decoded);
        }

        public void collectRefs(java.util.Set<String> refs) {
            expr.collectRefs(refs);
        }
    }

    private static final class SchemaBoolOp implements SchemaBoolExpr {
        final String op;
        final SchemaBoolExpr left;
        final SchemaBoolExpr right;

        SchemaBoolOp(String op, SchemaBoolExpr left, SchemaBoolExpr right) {
            this.op = op;
            this.left = left;
            this.right = right;
        }

        public boolean eval(java.util.Map<String, Long> decoded) {
            if (op.equals("and")) {
                return left.eval(decoded) && right.eval(decoded);
            }
            if (op.equals("or")) {
                return left.eval(decoded) || right.eval(decoded);
            }
            throw new IllegalStateException("unsupported schema boolean operator `" + op + "`");
        }

        public void collectRefs(java.util.Set<String> refs) {
            left.collectRefs(refs);
            right.collectRefs(refs);
        }
    }

    private static SchemaFieldSpec unsignedSchemaField(String name, int width, SchemaPredicate predicate) {
        return new SchemaFieldSpec(name, width, unsignedMaxValue(width), predicate);
    }

    private static long unsignedMaxValue(int width) {
        long maxValue = 0L;
        for (int index = 0; index < width; index += 1) {
            maxValue = (maxValue << 8) | 0xffL;
        }
        return maxValue;
    }

    private static SchemaPredicate schemaPredicate(String text, SchemaBoolExpr expr) {
        return new SchemaPredicate(text, expr);
    }

    private static Long schemaFixedFieldValue(String fieldName, String predicateText, long maxValue) {
        int operator = predicateText.indexOf("==");
        if (operator < 0 || predicateText.indexOf("==", operator + 2) >= 0) {
            return null;
        }
        String left = predicateText.substring(0, operator).trim();
        String right = predicateText.substring(operator + 2).trim();
        Long value = null;
        if (left.equals(fieldName)) {
            value = schemaFixedFieldLiteral(right);
        } else if (right.equals(fieldName)) {
            value = schemaFixedFieldLiteral(left);
        }
        if (value == null || value.longValue() < 0L || value.longValue() > maxValue) {
            return null;
        }
        return value;
    }

    private static Long schemaFixedFieldLiteral(String text) {
        if (text.isEmpty()) {
            return null;
        }
        for (int index = 0; index < text.length(); index += 1) {
            if (!Character.isDigit(text.charAt(index))) {
                return null;
            }
        }
        try {
            return Long.valueOf(Long.parseLong(text));
        } catch (NumberFormatException failure) {
            return null;
        }
    }

    private static SchemaIntExpr schemaLiteral(long value) {
        return new SchemaLiteral(value);
    }

    private static SchemaIntExpr schemaRef(String name) {
        return new SchemaRef(name);
    }

    private static SchemaIntExpr schemaArithmetic(String op, SchemaIntExpr left, SchemaIntExpr right) {
        return new SchemaArithmetic(op, left, right);
    }

    private static SchemaIntExpr schemaNegate(SchemaIntExpr expr) {
        return new SchemaNegate(expr);
    }

    private static SchemaBoolExpr schemaCompare(String op, SchemaIntExpr left, SchemaIntExpr right) {
        return new SchemaCompare(op, left, right);
    }

    private static SchemaBoolExpr schemaBoolLiteral(boolean value) {
        return new SchemaBoolLiteral(value);
    }

    private static SchemaBoolExpr schemaBoolNot(SchemaBoolExpr expr) {
        return new SchemaBoolNot(expr);
    }

    private static SchemaBoolExpr schemaBoolOp(String op, SchemaBoolExpr left, SchemaBoolExpr right) {
        return new SchemaBoolOp(op, left, right);
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

    public static Object flag8IsSet(Object flags, Object index) {
        Object maskResult = flag8BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag8Bits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag8Set(Object flags, Object index) {
        Object maskResult = flag8BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag8Bits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag8Object(bits + mask));
    }

    public static Object flag8RawBits(Object flags) {
        return Long.valueOf(flag8Bits(flags));
    }

    public static Object flag8FromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 255) {
            return err("flag8 bits must be between 0 and 255");
        }
        return ok(flag8Object(bits));
    }

    private static Object flag8BitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 7) {
            return err("flag bit index must be between 0 and 7");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag16beIsSet(Object flags, Object index) {
        Object maskResult = flag16beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag16beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag16beSet(Object flags, Object index) {
        Object maskResult = flag16beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag16beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag16beObject(bits + mask));
    }

    public static Object flag16beRawBits(Object flags) {
        return Long.valueOf(flag16beBits(flags));
    }

    public static Object flag16beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 65535) {
            return err("flag16be bits must be between 0 and 65535");
        }
        return ok(flag16beObject(bits));
    }

    private static Object flag16beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 15) {
            return err("flag bit index must be between 0 and 15");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag16leIsSet(Object flags, Object index) {
        Object maskResult = flag16leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag16leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag16leSet(Object flags, Object index) {
        Object maskResult = flag16leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag16leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag16leObject(bits + mask));
    }

    public static Object flag16leRawBits(Object flags) {
        return Long.valueOf(flag16leBits(flags));
    }

    public static Object flag16leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 65535) {
            return err("flag16le bits must be between 0 and 65535");
        }
        return ok(flag16leObject(bits));
    }

    private static Object flag16leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 15) {
            return err("flag bit index must be between 0 and 15");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag24beIsSet(Object flags, Object index) {
        Object maskResult = flag24beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag24beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag24beSet(Object flags, Object index) {
        Object maskResult = flag24beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag24beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag24beObject(bits + mask));
    }

    public static Object flag24beRawBits(Object flags) {
        return Long.valueOf(flag24beBits(flags));
    }

    public static Object flag24beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 16777215) {
            return err("flag24be bits must be between 0 and 16777215");
        }
        return ok(flag24beObject(bits));
    }

    private static Object flag24beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 23) {
            return err("flag bit index must be between 0 and 23");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag24leIsSet(Object flags, Object index) {
        Object maskResult = flag24leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag24leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag24leSet(Object flags, Object index) {
        Object maskResult = flag24leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag24leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag24leObject(bits + mask));
    }

    public static Object flag24leRawBits(Object flags) {
        return Long.valueOf(flag24leBits(flags));
    }

    public static Object flag24leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 16777215) {
            return err("flag24le bits must be between 0 and 16777215");
        }
        return ok(flag24leObject(bits));
    }

    private static Object flag24leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 23) {
            return err("flag bit index must be between 0 and 23");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag32beIsSet(Object flags, Object index) {
        Object maskResult = flag32beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag32beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag32beSet(Object flags, Object index) {
        Object maskResult = flag32beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag32beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag32beObject(bits + mask));
    }

    public static Object flag32beRawBits(Object flags) {
        return Long.valueOf(flag32beBits(flags));
    }

    public static Object flag32beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 4294967295L) {
            return err("flag32be bits must be between 0 and 4294967295");
        }
        return ok(flag32beObject(bits));
    }

    private static Object flag32beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 31) {
            return err("flag bit index must be between 0 and 31");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag32leIsSet(Object flags, Object index) {
        Object maskResult = flag32leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag32leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag32leSet(Object flags, Object index) {
        Object maskResult = flag32leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag32leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag32leObject(bits + mask));
    }

    public static Object flag32leRawBits(Object flags) {
        return Long.valueOf(flag32leBits(flags));
    }

    public static Object flag32leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 4294967295L) {
            return err("flag32le bits must be between 0 and 4294967295");
        }
        return ok(flag32leObject(bits));
    }

    private static Object flag32leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 31) {
            return err("flag bit index must be between 0 and 31");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag40beIsSet(Object flags, Object index) {
        Object maskResult = flag40beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag40beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag40beSet(Object flags, Object index) {
        Object maskResult = flag40beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag40beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag40beObject(bits + mask));
    }

    public static Object flag40beRawBits(Object flags) {
        return Long.valueOf(flag40beBits(flags));
    }

    public static Object flag40beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 1099511627775L) {
            return err("flag40be bits must be between 0 and 1099511627775");
        }
        return ok(flag40beObject(bits));
    }

    private static Object flag40beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 39) {
            return err("flag bit index must be between 0 and 39");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag40leIsSet(Object flags, Object index) {
        Object maskResult = flag40leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag40leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag40leSet(Object flags, Object index) {
        Object maskResult = flag40leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag40leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag40leObject(bits + mask));
    }

    public static Object flag40leRawBits(Object flags) {
        return Long.valueOf(flag40leBits(flags));
    }

    public static Object flag40leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 1099511627775L) {
            return err("flag40le bits must be between 0 and 1099511627775");
        }
        return ok(flag40leObject(bits));
    }

    private static Object flag40leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 39) {
            return err("flag bit index must be between 0 and 39");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag48beIsSet(Object flags, Object index) {
        Object maskResult = flag48beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag48beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag48beSet(Object flags, Object index) {
        Object maskResult = flag48beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag48beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag48beObject(bits + mask));
    }

    public static Object flag48beRawBits(Object flags) {
        return Long.valueOf(flag48beBits(flags));
    }

    public static Object flag48beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 281474976710655L) {
            return err("flag48be bits must be between 0 and 281474976710655");
        }
        return ok(flag48beObject(bits));
    }

    private static Object flag48beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 47) {
            return err("flag bit index must be between 0 and 47");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag48leIsSet(Object flags, Object index) {
        Object maskResult = flag48leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag48leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag48leSet(Object flags, Object index) {
        Object maskResult = flag48leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag48leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag48leObject(bits + mask));
    }

    public static Object flag48leRawBits(Object flags) {
        return Long.valueOf(flag48leBits(flags));
    }

    public static Object flag48leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 281474976710655L) {
            return err("flag48le bits must be between 0 and 281474976710655");
        }
        return ok(flag48leObject(bits));
    }

    private static Object flag48leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 47) {
            return err("flag bit index must be between 0 and 47");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag56beIsSet(Object flags, Object index) {
        Object maskResult = flag56beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag56beBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag56beSet(Object flags, Object index) {
        Object maskResult = flag56beBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag56beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag56beObject(bits + mask));
    }

    public static Object flag56beRawBits(Object flags) {
        return Long.valueOf(flag56beBits(flags));
    }

    public static Object flag56beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 72057594037927935L) {
            return err("flag56be bits must be between 0 and 72057594037927935");
        }
        return ok(flag56beObject(bits));
    }

    private static Object flag56beBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 55) {
            return err("flag bit index must be between 0 and 55");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag56leIsSet(Object flags, Object index) {
        Object maskResult = flag56leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag56leBits(flags);
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag56leSet(Object flags, Object index) {
        Object maskResult = flag56leBitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag56leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag56leObject(bits + mask));
    }

    public static Object flag56leRawBits(Object flags) {
        return Long.valueOf(flag56leBits(flags));
    }

    public static Object flag56leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0 || bits > 72057594037927935L) {
            return err("flag56le bits must be between 0 and 72057594037927935");
        }
        return ok(flag56leObject(bits));
    }

    private static Object flag56leBitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 55) {
            return err("flag bit index must be between 0 and 55");
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    public static Object flag64beIsSet(Object flags, Object index) {
        Object maskResult = flag64BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long bitIndex = asLong(index);
        long bits = flag64beBits(flags);
        if (bitIndex == 63) {
            return ok(Boolean.valueOf(bits < 0));
        }
        long mask = asLong(resultValue(maskResult));
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag64beSet(Object flags, Object index) {
        Object maskResult = flag64BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long bitIndex = asLong(index);
        if (bitIndex == 63) {
            return err(flag64BitsError("flag64be"));
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag64beBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag64beObject(bits + mask));
    }

    public static Object flag64beRawBits(Object flags) {
        return Long.valueOf(flag64beBits(flags));
    }

    public static Object flag64beFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0) {
            return err(flag64BitsError("flag64be"));
        }
        return ok(flag64beObject(bits));
    }

    public static Object flag64leIsSet(Object flags, Object index) {
        Object maskResult = flag64BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long bitIndex = asLong(index);
        long bits = flag64leBits(flags);
        if (bitIndex == 63) {
            return ok(Boolean.valueOf(bits < 0));
        }
        long mask = asLong(resultValue(maskResult));
        return ok(Boolean.valueOf(((bits / mask) - (bits / (mask * 2)) * 2) == 1));
    }

    public static Object flag64leSet(Object flags, Object index) {
        Object maskResult = flag64BitMask(index);
        if (!isOk(maskResult)) {
            return maskResult;
        }
        long bitIndex = asLong(index);
        if (bitIndex == 63) {
            return err(flag64BitsError("flag64le"));
        }
        long mask = asLong(resultValue(maskResult));
        long bits = flag64leBits(flags);
        if (((bits / mask) - (bits / (mask * 2)) * 2) == 1) {
            return ok(flags);
        }
        return ok(flag64leObject(bits + mask));
    }

    public static Object flag64leRawBits(Object flags) {
        return Long.valueOf(flag64leBits(flags));
    }

    public static Object flag64leFromBits(Object value) {
        long bits = asLong(value);
        if (bits < 0) {
            return err(flag64BitsError("flag64le"));
        }
        return ok(flag64leObject(bits));
    }

    private static Object flag64BitMask(Object index) {
        long bitIndex = asLong(index);
        if (bitIndex < 0 || bitIndex > 63) {
            return err("flag bit index must be between 0 and 63");
        }
        if (bitIndex == 63) {
            return ok(Long.valueOf(Long.MIN_VALUE));
        }
        return ok(Long.valueOf(1L << bitIndex));
    }

    private static String flag64BitsError(String name) {
        return name + " bits must be between 0 and 18446744073709551615";
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

    public static Object byteChunkToVisibleAsciiString(Object chunk) {
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        StringBuilder out = new StringBuilder();
        for (Object byteValue : bytes) {
            long value = byteValueLong(byteValue);
            if (value < 33L || value > 126L) {
                return err("byte chunk contains non-visible ASCII byte");
            }
            out.append((char) value);
        }
        return ok(out.toString());
    }

    public static Object byteChunkFromVisibleAsciiString(Object text) {
        String input = asString(text);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        for (int index = 0; index < input.length(); index += 1) {
            char value = input.charAt(index);
            if (value < 33 || value > 126) {
                return err("string contains non-visible ASCII character");
            }
            bytes.add(byteObject(value));
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
            String message = "byte view range exceeds chunk length";
            long availableCount = Math.max(0L, bytes.size() - requestedOffset);
            registerCodecByteRangeOutOfBoundsDiagnostic(
                message,
                requestedOffset,
                requestedCount,
                availableCount,
                byteRangePreviewTrace(bytes, requestedOffset, availableCount, 8)
            );
            return err(message);
        }
        return ok(byteViewObject(chunk, requestedOffset, requestedCount));
    }

    public static Object byteViewToChunk(Object view) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long offset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        if (offset < 0 || count < 0 || offset > bytes.size() || count > bytes.size() - offset) {
            return byteChunkObject(java.util.Collections.emptyList());
        }
        return byteChunkObject(bytes.subList((int) offset, (int) (offset + count)));
    }

    public static Object byteViewCount(Object view) {
        return byteCountObject(byteViewCountLong(view));
    }

    public static Object byteViewTake(Object view, Object count) {
        long requested = byteCountLong(count);
        if (requested < 0) {
            return err("byte count must be non-negative");
        }
        long available = byteViewCountLong(view);
        if (requested > available) {
            return err("byte view count exceeds view length");
        }
        return ok(byteViewDerivedObject(view, 0L, requested));
    }

    public static Object byteViewDrop(Object view, Object count) {
        long requested = byteCountLong(count);
        if (requested < 0) {
            return err("byte count must be non-negative");
        }
        long available = byteViewCountLong(view);
        if (requested > available) {
            return err("byte view count exceeds view length");
        }
        return ok(byteViewDerivedObject(view, requested, available - requested));
    }

    public static Object byteViewSlice(Object view, Object offset, Object count) {
        long requestedOffset = byteCountLong(offset);
        long requestedCount = byteCountLong(count);
        if (requestedOffset < 0 || requestedCount < 0) {
            return err("byte count must be non-negative");
        }
        long available = byteViewCountLong(view);
        if (requestedOffset > available || requestedCount > available - requestedOffset) {
            String message = "byte view range exceeds view length";
            long availableCount = Math.max(0L, available - requestedOffset);
            registerCodecByteRangeOutOfBoundsDiagnostic(
                message,
                byteViewOffset(view) + requestedOffset,
                requestedCount,
                availableCount,
                byteRangePreviewTrace(
                    byteViewBytes(view),
                    byteViewStorageOffset(view) + requestedOffset,
                    availableCount,
                    8
                )
            );
            return err(message);
        }
        return ok(byteViewDerivedObject(view, requestedOffset, requestedCount));
    }

    public static Object byteChunksEmpty() {
        return listNil();
    }

    public static Object byteChunksOne(Object chunk) {
        return listCons(chunk, listNil());
    }

    public static Object byteChunksAppend(Object left, Object right) {
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
            byteViewPreviewTrace(view, 8)
        );
        return err(message);
    }

    public static Object byteDecodeHttp2FrameHeader(Object view) {
        Object boundsError = byteViewBoundsError(view);
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
                byteViewPreviewTrace(view, 8)
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
        long count = byteViewCountLong(view);
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
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }

        Object payload = byteViewDerivedObject(view, headerCount, length);
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
        return byteDecodeBinarySchema(
            view,
            new BinarySchemaSpec(
                "SchemaWidthSample",
                new SchemaFieldSpec[] {
                    unsignedSchemaField("short_value", 2, null),
                    unsignedSchemaField("wide_value", 4, null)
                }
            )
        );
    }

    public static Object byteDecodeSchemaValidationSample(Object view) {
        return byteDecodeBinarySchema(
            view,
            new BinarySchemaSpec(
                "SchemaValidationSample",
                new SchemaFieldSpec[] {
                    unsignedSchemaField("length", 3, null),
                    unsignedSchemaField(
                        "padding_length",
                        1,
                        schemaPredicate(
                            "padding_length <= length",
                            schemaCompare("<=", schemaRef("padding_length"), schemaRef("length"))
                        )
                    )
                }
            )
        );
    }

    public static Object byteDecodeDeclaredBinarySchema(
        Object view,
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object schemaValidation,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        Object schema = declaredBinarySchemaSpec(
            schemaName,
            fieldNames,
            widths,
            maxValues,
            littleEndianValues,
            flag8Values,
            repeatCountFields,
            repeatWidths,
            repeatMaxValues,
            repeatLittleEndianValues,
            repeatByteViewLengthFields,
            repeatSchemaSpecs,
            reservedBitWidths,
            reservedValues,
            predicates,
            schemaValidation,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs,
            mappingTargets,
            mappingSources
        );
        if (isErr(schema)) {
            return schema;
        }
        return byteDecodeBinarySchema(view, (BinarySchemaSpec) unwrapOk(schema));
    }

    public static Object byteDecodeStepDeclaredBinarySchema(
        Object view,
        Object baseOffset,
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object schemaValidation,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        Object schema = declaredBinarySchemaSpec(
            schemaName,
            fieldNames,
            widths,
            maxValues,
            littleEndianValues,
            flag8Values,
            repeatCountFields,
            repeatWidths,
            repeatMaxValues,
            repeatLittleEndianValues,
            repeatByteViewLengthFields,
            repeatSchemaSpecs,
            reservedBitWidths,
            reservedValues,
            predicates,
            schemaValidation,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs,
            mappingTargets,
            mappingSources
        );
        if (isErr(schema)) {
            return invalidDecodeStep(
                "schema.decode_metadata",
                baseOffset,
                asString(schemaName)
            );
        }
        return byteDecodeStepBinarySchema(
            view,
            baseOffset,
            (BinarySchemaSpec) unwrapOk(schema)
        );
    }

    public static Object validateCodecDecodeStep(
        Object view,
        Object baseOffset,
        Object step,
        Object codecName
    ) {
        if (!isAdt(step, "DecodeStep::Decoded")) {
            return step;
        }
        long consumed = byteCountLong(adtPayload(step, 1));
        if (consumed >= 0L && consumed <= byteViewCountLong(view)) {
            return step;
        }
        return invalidDecodeStep(
            "codec.consumed_count_invalid",
            baseOffset,
            asString(codecName)
        );
    }

    public static Object validateDeclaredSchemaValue(
        Object value,
        Object schemaName,
        Object fieldNames,
        Object predicates,
        Object schemaValidation
    ) {
        String name = asString(schemaName);
        java.util.List<Object> fieldNameValues = asList(fieldNames);
        java.util.List<Object> predicateValues = asList(predicates);
        if (fieldNameValues.size() != predicateValues.size()) {
            return err("invalid schema validation metadata for schema `" + name + "`");
        }
        String schemaValidationText = asString(schemaValidation);
        SchemaPredicate validation = null;
        if (!schemaValidationText.isEmpty()) {
            try {
                validation = new SchemaPredicateParser(schemaValidationText).parse();
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
        }
        java.util.Map<Object, Object> record = asMap(value);
        java.util.LinkedHashMap<String, Long> supplied = new java.util.LinkedHashMap<String, Long>();
        for (int index = 0; index < fieldNameValues.size(); index += 1) {
            String fieldName = asString(fieldNameValues.get(index));
            Object fieldValue = record.get(fieldName);
            Long intValue = fieldValue instanceof Long ? (Long) fieldValue : null;
            if (intValue != null) {
                supplied.put(fieldName, intValue);
            }
            String predicateText = asString(predicateValues.get(index));
            if (predicateText.isEmpty()) {
                continue;
            }
            if (intValue == null) {
                return err(
                    "schema validation field `" + fieldName + "` in schema `" + name + "` is not an Int"
                );
            }
            SchemaPredicate predicate;
            try {
                predicate = new SchemaPredicateParser(predicateText).parse();
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
            boolean passed;
            try {
                passed = predicate.eval(supplied);
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
            if (!passed) {
                String message = "schema value validation failed for field `" + fieldName + "`";
                registerSchemaValueValidationFailedDiagnostic(
                    message,
                    name,
                    fieldName,
                    predicate.text,
                    intValue,
                    supplied
                );
                return err(message);
            }
        }
        if (validation != null) {
            boolean passed;
            try {
                passed = validation.eval(supplied);
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
            if (!passed) {
                String message = "schema value validation failed for schema `" + name + "`";
                registerSchemaValueValidationFailedDiagnostic(
                    message,
                    name,
                    "",
                    validation.text,
                    null,
                    supplied
                );
                return err(message);
            }
        }
        return ok(value);
    }

    public static Object byteEncodeDeclaredBinarySchema(
        Object value,
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        String name = asString(schemaName);
        java.util.List<Object> fieldNameValues = asList(fieldNames);
        java.util.List<Object> widthValues = asList(widths);
        java.util.List<Object> maxValueValues = asList(maxValues);
        java.util.List<Object> littleEndianValueList = asList(littleEndianValues);
        java.util.List<Object> flag8ValueList = asList(flag8Values);
        java.util.List<Object> repeatCountFieldValues = asList(repeatCountFields);
        java.util.List<Object> repeatWidthValues = asList(repeatWidths);
        java.util.List<Object> repeatMaxValueValues = asList(repeatMaxValues);
        java.util.List<Object> repeatLittleEndianValueList = asList(repeatLittleEndianValues);
        java.util.List<Object> repeatByteViewLengthFieldValues = asList(repeatByteViewLengthFields);
        java.util.List<Object> repeatSchemaSpecValues = asList(repeatSchemaSpecs);
        java.util.List<Object> reservedBitWidthValues = asList(reservedBitWidths);
        java.util.List<Object> reservedValueValues = asList(reservedValues);
        java.util.List<Object> predicateValues = asList(predicates);
        java.util.List<Object> dispatchTagFieldValues = asList(dispatchTagFields);
        java.util.List<Object> dispatchLengthFieldValues = asList(dispatchLengthFields);
        java.util.List<Object> dispatchCaseTagValues = asList(dispatchCaseTags);
        java.util.List<Object> dispatchCaseWidthValues = asList(dispatchCaseWidths);
        java.util.List<Object> dispatchCaseLittleEndianValueList =
            asList(dispatchCaseLittleEndianValues);
        java.util.List<Object> dispatchCaseSchemaSpecValues = asList(dispatchCaseSchemaSpecs);
        java.util.List<Object> mappingTargetValues = asList(mappingTargets);
        java.util.List<Object> mappingSourceValues = asList(mappingSources);
        if (fieldNameValues.size() != widthValues.size()
            || fieldNameValues.size() != maxValueValues.size()
            || fieldNameValues.size() != littleEndianValueList.size()
            || fieldNameValues.size() != flag8ValueList.size()
            || fieldNameValues.size() != repeatCountFieldValues.size()
            || fieldNameValues.size() != repeatWidthValues.size()
            || fieldNameValues.size() != repeatMaxValueValues.size()
            || fieldNameValues.size() != repeatLittleEndianValueList.size()
            || fieldNameValues.size() != repeatByteViewLengthFieldValues.size()
            || fieldNameValues.size() != repeatSchemaSpecValues.size()
            || fieldNameValues.size() != reservedBitWidthValues.size()
            || fieldNameValues.size() != reservedValueValues.size()
            || fieldNameValues.size() != predicateValues.size()
            || fieldNameValues.size() != dispatchTagFieldValues.size()
            || fieldNameValues.size() != dispatchLengthFieldValues.size()
            || fieldNameValues.size() != dispatchCaseTagValues.size()
            || fieldNameValues.size() != dispatchCaseWidthValues.size()
            || fieldNameValues.size() != dispatchCaseLittleEndianValueList.size()
            || fieldNameValues.size() != dispatchCaseSchemaSpecValues.size()) {
            return err(encodeError(
                "codec.metadata",
                name,
                "invalid schema encoder metadata for schema `" + name + "`"
            ));
        }
        if (mappingTargetValues.size() != mappingSourceValues.size()) {
            return err(encodeError(
                "codec.metadata",
                name,
                "invalid schema encoder mapping metadata for schema `" + name + "`"
            ));
        }
        SchemaFieldSpec[] schemaFields = new SchemaFieldSpec[fieldNameValues.size()];
        for (int index = 0; index < fieldNameValues.size(); index += 1) {
            String fieldName = asString(fieldNameValues.get(index));
            int width = (int) asLong(widthValues.get(index));
            long maxValue = asLong(maxValueValues.get(index));
            int reservedBitWidth = (int) asLong(reservedBitWidthValues.get(index));
            long reservedValue = asLong(reservedValueValues.get(index));
            String tagField = asString(dispatchTagFieldValues.get(index));
            String lengthField = asString(dispatchLengthFieldValues.get(index));
            String predicateText = asString(predicateValues.get(index));
            SchemaPredicate predicate = null;
            Long fixedValue = null;
            boolean littleEndian = asBool(littleEndianValueList.get(index));
            String flagType = schemaFlagTypeFromMetadata(flag8ValueList.get(index));
            String repeatCountField = asString(repeatCountFieldValues.get(index));
            String repeatByteViewLengthField =
                asString(repeatByteViewLengthFieldValues.get(index));
            if (!predicateText.isEmpty()) {
                fixedValue = schemaFixedFieldValue(fieldName, predicateText, maxValue);
                if (fixedValue == null) {
                    try {
                        predicate = new SchemaPredicateParser(predicateText).parse();
                    } catch (IllegalStateException failure) {
                        return err(encodeError(
                            "codec.metadata",
                            name + "." + fieldName,
                            failure.getMessage()
                        ));
                    }
                }
            }
            if (reservedBitWidth > 0) {
                schemaFields[index] = new SchemaFieldSpec(
                    fieldName,
                    reservedBitWidth,
                    reservedValue
                );
                continue;
            }
            if (!repeatCountField.isEmpty()) {
                Object repeatPayloadSchema = repeatSchemaSpecValues.get(index);
                BinarySchemaSpec payloadSchema = null;
                if (repeatPayloadSchema instanceof java.util.List<?>) {
                    Object parsedSchema = declaredBinarySchemaSpecFromMetadata(repeatPayloadSchema);
                    if (isErr(parsedSchema)) {
                        return err(encodeError(
                            "codec.metadata",
                            name + "." + fieldName,
                            asString(resultValue(parsedSchema))
                        ));
                    }
                    payloadSchema = (BinarySchemaSpec) unwrapOk(parsedSchema);
                }
                schemaFields[index] = new SchemaFieldSpec(
                    fieldName,
                    new SchemaRepeatSpec(
                        repeatCountField,
                        (int) asLong(repeatWidthValues.get(index)),
                        asLong(repeatMaxValueValues.get(index)),
                        asBool(repeatLittleEndianValueList.get(index)),
                        repeatByteViewLengthField,
                        payloadSchema
                    )
                );
                continue;
            }
            if (!lengthField.isEmpty() && tagField.isEmpty()) {
                schemaFields[index] = new SchemaFieldSpec(fieldName, lengthField);
                continue;
            }
            if (!tagField.isEmpty()) {
                Object dispatch = declaredSchemaDispatch(
                    name,
                    fieldName,
                    tagField,
                    lengthField,
                    dispatchCaseTagValues.get(index),
                    dispatchCaseWidthValues.get(index),
                    dispatchCaseLittleEndianValueList.get(index),
                    dispatchCaseSchemaSpecValues.get(index)
                );
                if (isErr(dispatch)) {
                    return err(encodeError(
                        "codec.metadata",
                        name + "." + fieldName,
                        asString(resultValue(dispatch))
                    ));
                }
                schemaFields[index] = new SchemaFieldSpec(
                    fieldName,
                    0,
                    0L,
                    null,
                    (SchemaDispatchSpec) unwrapOk(dispatch)
                );
                continue;
            }
            schemaFields[index] = new SchemaFieldSpec(
                fieldName,
                width,
                maxValue,
                littleEndian,
                flagType,
                predicate,
                fixedValue
            );
        }
        SchemaMappingSpec[] mapping = schemaMappingSpecs(mappingTargetValues, mappingSourceValues);
        return byteEncodeBinarySchema(value, new BinarySchemaSpec(name, schemaFields, mapping));
    }

    private static Object byteEncodeBinarySchema(Object value, BinarySchemaSpec schema) {
        Object encodeValue = schemaEncodeRecord(value, schema);
        if (isErr(encodeValue)) {
            return encodeValue;
        }
        java.util.Map<Object, Object> fields = asMap(unwrapOk(encodeValue));
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            int prefixStorageByteWidth = prefixReservedGroupStorageByteWidth(schema.fields, index);
            if (prefixStorageByteWidth > 0) {
                SchemaFieldSpec first = schema.fields[index + 1];
                SchemaFieldSpec second = schema.fields[index + 2];
                long firstValue = asLong(schemaFieldEncodeValue(fields.get(first.name), first));
                long secondValue = asLong(schemaFieldEncodeValue(fields.get(second.name), second));
                if (firstValue < 0L || firstValue > first.maxValue) {
                    return err(encodeError(
                        "codec.encode_value_unrepresentable",
                        schema.name + "." + first.name,
                        "value must be between 0 and " + first.maxValue
                    ));
                }
                if (secondValue < 0L || secondValue > second.maxValue) {
                    return err(encodeError(
                        "codec.encode_value_unrepresentable",
                        schema.name + "." + second.name,
                        "value must be between 0 and " + second.maxValue
                    ));
                }
                int firstBitWidth = bitPackedVisibleBitWidth(first);
                int secondBitWidth = bitPackedVisibleBitWidth(second);
                long combined =
                    (field.reservedValue << (firstBitWidth + secondBitWidth))
                        | (firstValue << secondBitWidth)
                        | secondValue;
                for (int byteIndex = prefixStorageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                    bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                }
                index += 2;
                continue;
            }
            int byteInterleavedStorageByteWidth =
                byteInterleavedReservedGroupStorageByteWidth(schema.fields, index);
            if (byteInterleavedStorageByteWidth > 0) {
                long combined = 0L;
                for (int offset = 0; offset < 4; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    int bitWidth = groupField.isReserved()
                        ? groupField.reservedBitWidth
                        : bitPackedVisibleBitWidth(groupField);
                    long fieldValue;
                    if (groupField.isReserved()) {
                        fieldValue = groupField.reservedValue;
                    } else {
                        fieldValue =
                            asLong(schemaFieldEncodeValue(fields.get(groupField.name), groupField));
                        if (fieldValue < 0L || fieldValue > groupField.maxValue) {
                            return err(encodeError(
                                "codec.encode_value_unrepresentable",
                                schema.name + "." + groupField.name,
                                "value must be between 0 and " + groupField.maxValue
                            ));
                        }
                    }
                    combined = (combined << bitWidth) | fieldValue;
                }
                for (int byteIndex = byteInterleavedStorageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                    bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                }
                index += 3;
                continue;
            }
            BitPackedFieldGroupSpec bitPackedGroup =
                bitPackedFieldGroupSpec(schema.fields, index);
            if (bitPackedGroup != null) {
                long combined = 0L;
                for (int offset = 0; offset < bitPackedGroup.fieldCount; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    int bitWidth = bitPackedGroupFieldBitWidth(groupField);
                    long fieldValue;
                    if (groupField.isReserved()) {
                        fieldValue = groupField.reservedValue;
                    } else {
                        fieldValue =
                            asLong(schemaFieldEncodeValue(fields.get(groupField.name), groupField));
                        if (fieldValue < 0L || fieldValue > groupField.maxValue) {
                            return err(encodeError(
                                "codec.encode_value_unrepresentable",
                                schema.name + "." + groupField.name,
                                "value must be between 0 and " + groupField.maxValue
                            ));
                        }
                    }
                    combined = (combined << bitWidth) | fieldValue;
                }
                for (int byteIndex = bitPackedGroup.storageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                    bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                }
                index += bitPackedGroup.fieldCount - 1;
                continue;
            }
            int middleStorageByteWidth = middleReservedGroupStorageByteWidth(schema.fields, index);
            if (middleStorageByteWidth > 0) {
                SchemaFieldSpec reserved = schema.fields[index + 1];
                SchemaFieldSpec right = schema.fields[index + 2];
                long leftValue = asLong(schemaFieldEncodeValue(fields.get(field.name), field));
                long rightValue = asLong(schemaFieldEncodeValue(fields.get(right.name), right));
                if (leftValue < 0L || leftValue > field.maxValue) {
                    return err(encodeError(
                        "codec.encode_value_unrepresentable",
                        schema.name + "." + field.name,
                        "value must be between 0 and " + field.maxValue
                    ));
                }
                if (rightValue < 0L || rightValue > right.maxValue) {
                    return err(encodeError(
                        "codec.encode_value_unrepresentable",
                        schema.name + "." + right.name,
                        "value must be between 0 and " + right.maxValue
                    ));
                }
                int rightBitWidth = bitPackedVisibleBitWidth(right);
                long combined =
                    (leftValue << (reserved.reservedBitWidth + rightBitWidth))
                        | (reserved.reservedValue << rightBitWidth)
                        | rightValue;
                for (int byteIndex = middleStorageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                    bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                }
                index += 2;
                continue;
            }
            SchemaFieldSpec reservedBytePrefixVisible =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int reservedBytePrefixStorageByteWidth =
                reservedBytePrefixStorageByteWidth(field, reservedBytePrefixVisible);
            if (reservedBytePrefixStorageByteWidth > 0) {
                int visibleBitWidth =
                    reservedBytePrefixVisibleBitWidth(field, reservedBytePrefixVisible);
                long visibleValue =
                    asLong(schemaFieldEncodeValue(
                        fields.get(reservedBytePrefixVisible.name),
                        reservedBytePrefixVisible
                    ));
                if (visibleValue < 0L || visibleValue > reservedBytePrefixVisible.maxValue) {
                    return err(encodeError(
                        "codec.encode_value_unrepresentable",
                        schema.name + "." + reservedBytePrefixVisible.name,
                        "value must be between 0 and " + reservedBytePrefixVisible.maxValue
                    ));
                }
                int lowPaddingBitWidth =
                    reservedBytePrefixStorageByteWidth * 8
                        - field.reservedBitWidth
                        - visibleBitWidth;
                long combined =
                    (field.reservedValue << (visibleBitWidth + lowPaddingBitWidth))
                        | (visibleValue << lowPaddingBitWidth);
                for (int byteIndex = reservedBytePrefixStorageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                    bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                }
                index += 1;
                continue;
            }
            if (field.isReserved()) {
                if (!supportedReservedBits(field.reservedBitWidth, field.reservedValue)) {
                    return err(encodeError(
                        "codec.metadata",
                        schema.name + "." + field.name,
                        "unsupported reserved bits encode field"
                    ));
                }
                int reservedByteWidth = reservedByteWidth(field.reservedBitWidth);
                if (reservedByteWidth > 0) {
                    Object encoded = schemaWriteUnsigned(
                        Long.valueOf(field.reservedValue),
                        reservedByteWidth,
                        reservedMaxValue(field.reservedBitWidth),
                        false,
                        schema.name,
                        field.name
                    );
                    if (isErr(encoded)) {
                        return encoded;
                    }
                    bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
                    continue;
                }
                if (index + 1 < schema.fields.length) {
                    SchemaFieldSpec nextField = schema.fields[index + 1];
                    int visibleBitWidth = packedVisibleBitWidth(field, nextField);
                    if (visibleBitWidth > 0) {
                        int storageByteWidth = packedStorageByteWidth(field, nextField);
                        long visibleValue = asLong(fields.get(nextField.name));
                        if (visibleValue < 0L || visibleValue > nextField.maxValue) {
                            return err(encodeError(
                                "codec.encode_value_unrepresentable",
                                schema.name + "." + nextField.name,
                                "value must be between 0 and " + nextField.maxValue
                            ));
                        }
                        long combined =
                            (field.reservedValue << visibleBitWidth) | visibleValue;
                        for (int byteIndex = storageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                            bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                        }
                        index += 1;
                        continue;
                    }
                }
                continue;
            }
            if (index + 1 < schema.fields.length) {
                SchemaFieldSpec nextField = schema.fields[index + 1];
                int suffixVisibleBitWidth = suffixPackedVisibleBitWidth(field, nextField);
                if (suffixVisibleBitWidth > 0) {
                    int suffixStorageByteWidth = suffixPackedStorageByteWidth(field, nextField);
                    long visibleValue = asLong(schemaFieldEncodeValue(fields.get(field.name), field));
                    if (visibleValue < 0L || visibleValue > field.maxValue) {
                        return err(encodeError(
                            "codec.encode_value_unrepresentable",
                            schema.name + "." + field.name,
                            "value must be between 0 and " + field.maxValue
                        ));
                    }
                    long combined =
                        (visibleValue << nextField.reservedBitWidth) | nextField.reservedValue;
                    for (int byteIndex = suffixStorageByteWidth - 1; byteIndex >= 0; byteIndex -= 1) {
                        bytes.add(byteObject((combined >> (byteIndex * 8)) & 0xffL));
                    }
                    index += 1;
                    continue;
                }
            }
            if (field.dispatch != null) {
                Object encoded = schemaWriteDispatchPayload(
                    fields,
                    schema,
                    field.dispatch,
                    field.name
                );
                if (isErr(encoded)) {
                    return encoded;
                }
                bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
                continue;
            }
            if (field.isRepeat()) {
                Object encoded = schemaWriteRepeatedField(fields, field, schema.name);
                if (isErr(encoded)) {
                    return encoded;
                }
                bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
                continue;
            }
            if (field.isLengthBoundedView()) {
                Object encoded = schemaWriteLengthBoundedView(fields, field, schema.name);
                if (isErr(encoded)) {
                    return encoded;
                }
                bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
                continue;
            }
            if (field.width < 1 || field.width > 8) {
                return err(encodeError(
                    "codec.metadata",
                    schema.name + "." + field.name,
                    "unsupported schema field width " + field.width
                ));
            }
            Object encoded = schemaWriteUnsigned(
                schemaFieldEncodeValue(fields.get(field.name), field),
                field.width,
                field.maxValue,
                field.littleEndian,
                schema.name,
                field.name
            );
            if (isErr(encoded)) {
                return encoded;
            }
            long fieldValue = asLong(schemaFieldEncodeValue(fields.get(field.name), field));
            if (field.fixedValue != null && fieldValue != field.fixedValue.longValue()) {
                return err(encodeError(
                    "codec.encode_value_unrepresentable",
                    schema.name + "." + field.name,
                    "fixed field expected value "
                        + field.fixedValue
                        + "; actual value was "
                        + fieldValue
                ));
            }
            bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
        }
        Object validation = validateEncodedSchemaFields(fields, schema);
        if (isErr(validation)) {
            return validation;
        }
        return ok(byteChunkObject(bytes));
    }

    private static Object validateEncodedSchemaFields(
        java.util.Map<Object, Object> fields,
        BinarySchemaSpec schema
    ) {
        java.util.LinkedHashMap<String, Long> supplied = new java.util.LinkedHashMap<String, Long>();
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            Object value = fields.get(field.name);
            Long intValue = schemaEncodedIntValue(value, field);
            if (intValue != null) {
                supplied.put(field.name, intValue);
            }
            if (field.predicate == null) {
                continue;
            }
            if (intValue == null) {
                return err(encodeError(
                    "codec.metadata",
                    schema.name + "." + field.name,
                    "schema validation field `" + field.name + "` is not an Int"
                ));
            }
            boolean passed;
            try {
                passed = field.predicate.eval(supplied);
            } catch (IllegalStateException failure) {
                return err(encodeError(
                    "codec.metadata",
                    schema.name + "." + field.name,
                    failure.getMessage()
                ));
            }
            if (!passed) {
                Object error = encodeError(
                    "schema.validation_failed",
                    schema.name + "." + field.name,
                    "schema encode validation failed for field `" + field.name + "`"
                );
                registerSchemaValueValidationFailedDiagnostic(
                    format(error),
                    schema.name,
                    field.name,
                    field.predicate.text,
                    intValue,
                    supplied
                );
                return err(error);
            }
        }
        return ok(null);
    }

    public static Object byteEncodeStepDeclaredBinarySchema(
        Object value,
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        Object encoded = byteEncodeDeclaredBinarySchema(
            value,
            schemaName,
            fieldNames,
            widths,
            maxValues,
            littleEndianValues,
            flag8Values,
            repeatCountFields,
            repeatWidths,
            repeatMaxValues,
            repeatLittleEndianValues,
            repeatByteViewLengthFields,
            repeatSchemaSpecs,
            reservedBitWidths,
            reservedValues,
            predicates,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs,
            mappingTargets,
            mappingSources
        );
        if (isErr(encoded)) {
            return adt("EncodeStep::Invalid", new Object[] { resultValue(encoded) });
        }
        Object chunks = listCons(unwrapOk(encoded), listNil());
        return adt("EncodeStep::Encoded", new Object[] { chunks });
    }

    public static Object byteEncodeStepDeclaredBinarySchemaBudgeted(
        Object value,
        Object budget,
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        Object encoded = byteEncodeDeclaredBinarySchema(
            value,
            schemaName,
            fieldNames,
            widths,
            maxValues,
            littleEndianValues,
            flag8Values,
            repeatCountFields,
            repeatWidths,
            repeatMaxValues,
            repeatLittleEndianValues,
            repeatByteViewLengthFields,
            repeatSchemaSpecs,
            reservedBitWidths,
            reservedValues,
            predicates,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs,
            mappingTargets,
            mappingSources
        );
        if (isErr(encoded)) {
            return adt("EncodeStep::Invalid", new Object[] { resultValue(encoded) });
        }

        Object chunk = unwrapOk(encoded);
        java.util.List<Object> bytes = byteChunkBytes(chunk);
        long total = bytes.size();
        long offset = Math.max(0L, Math.min(encodeResumeOffset(value), total));
        long limit = Math.max(0L, byteCountLong(budget));
        long remaining = total - offset;
        if (remaining <= limit) {
            Object output = byteChunkObject(bytes.subList((int) offset, (int) total));
            return adt(
                "EncodeStep::Encoded",
                new Object[] { listCons(output, listNil()) }
            );
        }

        long nextOffset = offset + limit;
        Object output = byteChunkObject(bytes.subList((int) offset, (int) nextOffset));
        Object state = encodeResumeState(value, nextOffset);
        return adt(
            "EncodeStep::Partial",
            new Object[] { listCons(output, listNil()), byteCountObject(limit), state }
        );
    }

    private static long encodeResumeOffset(Object value) {
        if (!(value instanceof java.util.Map)) {
            return 0L;
        }
        java.util.Map<Object, Object> fields = asMap(value);
        if (!fields.containsKey("encoded_offset")) {
            return 0L;
        }
        return byteCountLong(fields.get("encoded_offset"));
    }

    private static Object encodeResumeState(Object value, long offset) {
        java.util.LinkedHashMap<Object, Object> fields =
            new java.util.LinkedHashMap<Object, Object>();
        if (value instanceof java.util.Map) {
            fields.putAll(asMap(value));
        }
        fields.put("encoded_offset", byteCountObject(offset));
        return freezeMap(fields);
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

    public static Object http2ProtocolPartialPreface(
        Object offset,
        Object pendingCount,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long pending = asLong(pendingCount);
        String message = "HTTP/2 input ended with partial client connection preface at byte offset "
            + byteOffset;
        registerHttp2ProtocolPartialPrefaceDiagnostic(
            message,
            byteOffset,
            pending,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidPreface(
        Object offset,
        Object expectedByte,
        Object actualByte,
        Object matchedCount,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long expected = asLong(expectedByte);
        long actual = asLong(actualByte);
        long matched = asLong(matchedCount);
        String message = "HTTP/2 invalid client connection preface at byte offset "
            + byteOffset;
        registerHttp2ProtocolInvalidPrefaceDiagnostic(
            message,
            byteOffset,
            expected,
            actual,
            matched,
            byteViewPreviewTrace(preview, 8)
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

    public static Object http2ProtocolInvalidFrameKind(
        Object offset,
        Object actualKind,
        Object streamId,
        Object expectedKind,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long frameKind = asLong(actualKind);
        long stream = asLong(streamId);
        long expectedFrameKind = asLong(expectedKind);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid frame kind at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidFrameKindDiagnostic(
            message,
            byteOffset,
            frameKind,
            stream,
            expectedFrameKind,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidStreamId(
        Object offset,
        Object frameKind,
        Object streamId,
        Object requiredDomain,
        Object endpointRole,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String domain = asString(requiredDomain);
        String role = asString(endpointRole);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid stream id at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidStreamIdDiagnostic(
            message,
            byteOffset,
            kind,
            stream,
            domain,
            role,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidPayloadLength(
        Object offset,
        Object frameKind,
        Object streamId,
        Object observedLength,
        Object expectedLength,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        long observed = asLong(observedLength);
        long expected = asLong(expectedLength);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid payload length at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidPayloadLengthDiagnostic(
            message,
            byteOffset,
            kind,
            stream,
            observed,
            expected,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidWindowUpdateIncrement(
        Object offset,
        Object streamId,
        Object observedIncrement,
        Object acceptedMinIncrement,
        Object acceptedMaxIncrement,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long observed = asLong(observedIncrement);
        long acceptedMin = asLong(acceptedMinIncrement);
        long acceptedMax = asLong(acceptedMaxIncrement);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid WINDOW_UPDATE increment at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(
            message,
            byteOffset,
            stream,
            observed,
            acceptedMin,
            acceptedMax,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidDataPadding(
        Object offset,
        Object streamId,
        Object padLength,
        Object remainingPayloadLength,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long pad = asLong(padLength);
        long remaining = asLong(remainingPayloadLength);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid DATA padding at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidDataPaddingDiagnostic(
            message,
            byteOffset,
            stream,
            pad,
            remaining,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolUnexpectedSettingsAck(
        Object offset,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 unexpected SETTINGS ACK at byte offset " + byteOffset;
        registerHttp2ProtocolUnexpectedSettingsAckDiagnostic(
            message,
            byteOffset,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidRequestHeaderList(
        Object offset,
        Object frameKind,
        Object streamId,
        Object failedHeaderFact,
        Object headerName,
        Object decodedHeaderNames,
        Object activeState,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String fact = asString(failedHeaderFact);
        String header = asString(headerName);
        String names = asString(decodedHeaderNames);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message;
        if ("missing_required_pseudo_header".equals(fact)) {
            message = "HTTP/2 request header list is missing " + header
                + " at byte offset " + byteOffset;
        } else if ("response_only_pseudo_header".equals(fact)) {
            message = "HTTP/2 request header list contains response-only " + header
                + " at byte offset " + byteOffset;
        } else if ("duplicate_pseudo_header".equals(fact)) {
            message = "HTTP/2 request header list contains duplicate " + header
                + " at byte offset " + byteOffset;
        } else if ("pseudo_header_after_regular_header".equals(fact)) {
            message = "HTTP/2 request header list places " + header
                + " after a regular header at byte offset " + byteOffset;
        } else if ("ordinary_header_name_not_lowercase".equals(fact)) {
            message = "HTTP/2 request header list contains uppercase ordinary header "
                + header + " at byte offset " + byteOffset;
        } else if ("ordinary_header_name_invalid_token".equals(fact)) {
            message = "HTTP/2 request header list contains invalid ordinary header name "
                + header + " at byte offset " + byteOffset;
        } else if ("connection_specific_header".equals(fact)) {
            message = "HTTP/2 request header list contains connection-specific header "
                + header + " at byte offset " + byteOffset;
        } else if ("te_header_value_not_trailers".equals(fact)) {
            message = "HTTP/2 request header list contains te value other than trailers"
                + " at byte offset " + byteOffset;
        } else if ("scheme_value_not_http_or_https".equals(fact)) {
            message = "HTTP/2 request header list contains :scheme value other than http or https"
                + " at byte offset " + byteOffset;
        } else if ("content_length_invalid".equals(fact)) {
            message = "HTTP/2 request header list contains invalid content-length"
                + " at byte offset " + byteOffset;
        } else if ("content_length_mismatch".equals(fact)) {
            message = "HTTP/2 request header list contains mismatched content-length values"
                + " at byte offset " + byteOffset;
        } else {
            message = "HTTP/2 invalid request header list at byte offset " + byteOffset;
        }
        registerHttp2ProtocolInvalidRequestHeaderListDiagnostic(
            message,
            byteOffset,
            kind,
            stream,
            fact,
            header,
            names,
            state,
            provenance
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidResponseHeaderList(
        Object offset,
        Object frameKind,
        Object streamId,
        Object failedHeaderFact,
        Object headerName,
        Object decodedHeaderNames,
        Object activeState,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String fact = asString(failedHeaderFact);
        String header = asString(headerName);
        String names = asString(decodedHeaderNames);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message;
        if ("missing_required_pseudo_header".equals(fact)) {
            message = "HTTP/2 response header list is missing " + header
                + " at byte offset " + byteOffset;
        } else if ("request_only_pseudo_header".equals(fact)) {
            message = "HTTP/2 response header list contains request-only " + header
                + " at byte offset " + byteOffset;
        } else if ("duplicate_pseudo_header".equals(fact)) {
            message = "HTTP/2 response header list contains duplicate " + header
                + " at byte offset " + byteOffset;
        } else if ("pseudo_header_after_regular_header".equals(fact)) {
            message = "HTTP/2 response header list places " + header
                + " after a regular header at byte offset " + byteOffset;
        } else if ("ordinary_header_name_not_lowercase".equals(fact)) {
            message = "HTTP/2 response header list contains uppercase ordinary header "
                + header + " at byte offset " + byteOffset;
        } else if ("ordinary_header_name_invalid_token".equals(fact)) {
            message = "HTTP/2 response header list contains invalid ordinary header name "
                + header + " at byte offset " + byteOffset;
        } else if ("te_header_value_not_trailers".equals(fact)) {
            message = "HTTP/2 response header list contains te value other than trailers"
                + " at byte offset " + byteOffset;
        } else if ("content_length_invalid".equals(fact)) {
            message = "HTTP/2 response header list contains invalid content-length"
                + " at byte offset " + byteOffset;
        } else if ("content_length_mismatch".equals(fact)) {
            message = "HTTP/2 response header list contains mismatched content-length values"
                + " at byte offset " + byteOffset;
        } else {
            message = "HTTP/2 invalid response header list at byte offset " + byteOffset;
        }
        registerHttp2ProtocolInvalidResponseHeaderListDiagnostic(
            message,
            byteOffset,
            kind,
            stream,
            fact,
            header,
            names,
            state,
            provenance
        );
        return err(message);
    }

    public static Object http2ProtocolInvalidPriorityDependency(
        Object offset,
        Object streamId,
        Object dependencyStreamId,
        Object activeState,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long dependency = asLong(dependencyStreamId);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 invalid PRIORITY dependency at byte offset " + byteOffset;
        registerHttp2ProtocolInvalidPriorityDependencyDiagnostic(
            message,
            byteOffset,
            stream,
            dependency,
            state,
            provenance,
            byteViewPreviewTrace(preview, 8)
        );
        return err(message);
    }

    public static Object http2ProtocolStreamAfterGoaway(
        Object offset,
        Object streamId,
        Object lastStreamId,
        Object shutdownState,
        Object endpointRole,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long lastStream = asLong(lastStreamId);
        String shutdown = asString(shutdownState);
        String role = asString(endpointRole);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 stream opened after graceful shutdown at byte offset "
            + byteOffset;
        registerHttp2ProtocolStreamAfterGoawayDiagnostic(
            message,
            byteOffset,
            stream,
            lastStream,
            shutdown,
            role,
            provenance
        );
        return err(message);
    }

    public static Object http2PeerLimitFrameSizeExceeded(
        Object offset,
        Object observedLength,
        Object allowedLength,
        Object frameKind,
        Object streamId,
        Object receiveLimitProvenance
    ) {
        long byteOffset = asLong(offset);
        long observed = asLong(observedLength);
        long allowed = asLong(allowedLength);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String provenance = asString(receiveLimitProvenance);
        String message = "HTTP/2 frame payload length exceeds receive maximum at byte offset "
            + byteOffset;
        registerHttp2PeerLimitFrameSizeExceededDiagnostic(
            message,
            byteOffset,
            observed,
            allowed,
            kind,
            stream,
            provenance
        );
        return err(message);
    }

    public static Object http2PeerLimitHeaderListSizeExceeded(
        Object offset,
        Object observedSize,
        Object allowedSize,
        Object frameKind,
        Object streamId,
        Object receiveLimitProvenance,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long observed = asLong(observedSize);
        long allowed = asLong(allowedSize);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String limitProvenance = asString(receiveLimitProvenance);
        String rule = asString(ruleProvenance);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HTTP/2 header list size exceeds receive maximum at byte offset "
            + byteOffset;
        registerHttp2PeerLimitHeaderListSizeExceededDiagnostic(
            message,
            byteOffset,
            observed,
            allowed,
            kind,
            stream,
            limitProvenance,
            rule,
            nearbyContext
        );
        return err(message);
    }

    public static Object http2PeerLimitHeaderTableSizeExceeded(
        Object offset,
        Object observedSize,
        Object allowedSize,
        Object frameKind,
        Object streamId,
        Object receiveLimitProvenance,
        Object ruleProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long observed = asLong(observedSize);
        long allowed = asLong(allowedSize);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String limitProvenance = asString(receiveLimitProvenance);
        String rule = asString(ruleProvenance);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HTTP/2 header table size exceeds receive maximum at byte offset "
            + byteOffset;
        registerHttp2PeerLimitHeaderTableSizeExceededDiagnostic(
            message,
            byteOffset,
            observed,
            allowed,
            kind,
            stream,
            limitProvenance,
            rule,
            nearbyContext
        );
        return err(message);
    }

    public static Object http2PeerLimitFlowControlWindowExceeded(
        Object offset,
        Object observedLength,
        Object allowedWindowCredit,
        Object frameKind,
        Object streamId,
        Object activeState,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long observed = asLong(observedLength);
        long allowed = asLong(allowedWindowCredit);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 flow-control window exceeded at byte offset "
            + byteOffset;
        registerHttp2PeerLimitFlowControlWindowExceededDiagnostic(
            message,
            byteOffset,
            observed,
            allowed,
            kind,
            stream,
            state,
            provenance
        );
        return err(message);
    }

    public static Object http2PeerLimitConcurrentStreamsExceeded(
        Object offset,
        Object streamId,
        Object attemptedCount,
        Object allowedCount,
        Object endpointRole,
        Object activeState,
        Object receiveLimitProvenance,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long attempted = asLong(attemptedCount);
        long allowed = asLong(allowedCount);
        String role = asString(endpointRole);
        String state = asString(activeState);
        String limitProvenance = asString(receiveLimitProvenance);
        String rule = asString(ruleProvenance);
        String message = "HTTP/2 concurrent stream receive limit exceeded at byte offset "
            + byteOffset;
        registerHttp2PeerLimitConcurrentStreamsExceededDiagnostic(
            message,
            byteOffset,
            stream,
            attempted,
            allowed,
            role,
            state,
            limitProvenance,
            rule
        );
        return err(message);
    }

    public static Object http2PeerLimitSettingsValueOutOfRange(
        Object offset,
        Object settingIdentifier,
        Object settingName,
        Object observedValue,
        Object acceptedMinValue,
        Object acceptedMaxValue,
        Object peerLimitProvenance,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long identifier = asLong(settingIdentifier);
        String name = asString(settingName);
        long observed = asLong(observedValue);
        long acceptedMin = asLong(acceptedMinValue);
        long acceptedMax = asLong(acceptedMaxValue);
        String provenance = asString(peerLimitProvenance);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HTTP/2 SETTINGS value outside accepted range at byte offset "
            + byteOffset;
        registerHttp2PeerLimitSettingsValueOutOfRangeDiagnostic(
            message,
            byteOffset,
            identifier,
            name,
            observed,
            acceptedMin,
            acceptedMax,
            provenance,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureUnsupportedHeaderBlock(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture unsupported header block at byte offset "
            + byteOffset;
        registerHpackFixtureUnsupportedHeaderBlockDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureMalformedStringLength(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture malformed string length at byte offset "
            + byteOffset;
        registerHpackFixtureMalformedStringLengthDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureMalformedRawStringValue(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture malformed raw string value at byte offset "
            + byteOffset;
        registerHpackFixtureMalformedRawStringValueDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureMalformedHuffmanPadding(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture malformed Huffman padding at byte offset "
            + byteOffset;
        registerHpackFixtureMalformedHuffmanPaddingDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureHuffmanEosSymbol(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture Huffman EOS symbol at byte offset "
            + byteOffset;
        registerHpackFixtureHuffmanEosSymbolDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureHuffmanNonVisibleValue(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture Huffman non-visible value at byte offset "
            + byteOffset;
        registerHpackFixtureHuffmanNonVisibleValueDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object hpackFixtureTableSizeUpdateNotAtStart(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object observedUpdateSize,
        Object frameKind,
        Object streamId,
        Object activeState,
        Object expectedFixture,
        Object codecModule,
        Object preview
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        long updateSize = asLong(observedUpdateSize);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String state = asString(activeState);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String nearbyContext = byteViewPreviewTrace(preview, 8);
        String message = "HPACK fixture table-size update after header field at byte offset "
            + byteOffset;
        registerHpackFixtureTableSizeUpdateNotAtStartDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            updateSize,
            kind,
            stream,
            state,
            expected,
            module,
            nearbyContext
        );
        return err(message);
    }

    public static Object byteReadU16Be(Object view) {
        return byteReadUnsigned(view, 2, 0xffffL, "byte_read_u16_be", false);
    }

    public static Object byteReadU24Be(Object view) {
        return byteReadUnsigned(view, 3, 0xffffffL, "byte_read_u24_be", false);
    }

    public static Object byteReadU31Be(Object view) {
        return byteReadUnsigned(view, 4, 0x7fffffffL, "byte_read_u31_be", false);
    }

    public static Object byteReadU32Be(Object view) {
        return byteReadUnsigned(view, 4, 0xffffffffL, "byte_read_u32_be", false);
    }

    public static Object byteReadU40Be(Object view) {
        return byteReadUnsigned(view, 5, 0xffffffffffL, "byte_read_u40_be", false);
    }

    public static Object byteReadU48Be(Object view) {
        return byteReadUnsigned(view, 6, 0xffffffffffffL, "byte_read_u48_be", false);
    }

    public static Object byteReadU64Be(Object view) {
        return byteReadUnsigned(view, 8, Long.MAX_VALUE, "byte_read_u64_be", false);
    }

    public static Object byteReadU16Le(Object view) {
        return byteReadUnsigned(view, 2, 0xffffL, "byte_read_u16_le", true);
    }

    public static Object byteReadU24Le(Object view) {
        return byteReadUnsigned(view, 3, 0xffffffL, "byte_read_u24_le", true);
    }

    public static Object byteReadU31Le(Object view) {
        return byteReadUnsigned(view, 4, 0x7fffffffL, "byte_read_u31_le", true);
    }

    public static Object byteReadU32Le(Object view) {
        return byteReadUnsigned(view, 4, 0xffffffffL, "byte_read_u32_le", true);
    }

    public static Object byteReadU40Le(Object view) {
        return byteReadUnsigned(view, 5, 0xffffffffffL, "byte_read_u40_le", true);
    }

    public static Object byteReadU48Le(Object view) {
        return byteReadUnsigned(view, 6, 0xffffffffffffL, "byte_read_u48_le", true);
    }

    public static Object byteReadU64Le(Object view) {
        return byteReadUnsigned(view, 8, Long.MAX_VALUE, "byte_read_u64_le", true);
    }

    private static Object byteReadUnsigned(
        Object view,
        int width,
        long maxValue,
        String name,
        boolean littleEndian
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageOffset = byteViewStorageOffset(view);
        long offset = byteViewOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageOffset, count);
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
            int byteIndex = littleEndian ? width - 1 - index : index;
            value = (value << 8) | byteValueLong(bytes.get((int) storageOffset + byteIndex));
        }
        if (value < 0L || value > maxValue) {
            return err(name + " value exceeds maximum " + maxValue);
        }
        return ok(Long.valueOf(value));
    }

    private static Object byteReadUnsignedBe(Object view, int width, long maxValue, String name) {
        return byteReadUnsigned(view, width, maxValue, name, false);
    }

    private static Object schemaReadUnsignedBe(
        Object view,
        int relativeOffset,
        int width,
        long maxValue,
        String schemaName,
        String fieldName
    ) {
        return schemaReadUnsignedWithBase(
            view,
            byteViewOffset(view),
            relativeOffset,
            width,
            maxValue,
            false,
            schemaName,
            fieldName
        );
    }

    private static Object schemaReadUnsignedWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        int width,
        long maxValue,
        boolean littleEndian,
        String schemaName,
        String fieldName
    ) {
        return schemaReadUnsignedWithBasePath(
            view,
            diagnosticBaseOffset,
            relativeOffset,
            width,
            maxValue,
            littleEndian,
            schemaName,
            fieldName,
            schemaFieldPath(schemaName, fieldName)
        );
    }

    private static Object schemaReadUnsignedWithBasePath(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        int width,
        long maxValue,
        boolean littleEndian,
        String schemaName,
        String fieldName,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        if (available < width) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldPathDiagnostic(
                message,
                byteOffset,
                fieldPath,
                width,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }

        long value = 0;
        for (int index = 0; index < width; index += 1) {
            int byteIndex = littleEndian ? width - 1 - index : index;
            value = (value << 8)
                | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + byteIndex));
        }
        if (value < 0L || value > maxValue) {
            if (width == 1 && maxValue < 0xffL) {
                value = value & maxValue;
            } else {
                long byteOffset = diagnosticBaseOffset + relativeOffset;
                String message = "schema integer out of range at byte offset " + byteOffset;
                registerSchemaIntegerOutOfRangePathDiagnostic(
                    message,
                    byteOffset,
                    fieldPath,
                    width,
                    0L,
                    maxValue,
                    value,
                    byteViewPreviewTrace(view, 8)
                );
                return err(message);
            }
        }
        return ok(Long.valueOf(value));
    }

    private static Object schemaReadReservedBits(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        int bitWidth,
        long expectedValue,
        String schemaName,
        String fieldName
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        if (!supportedReservedBits(bitWidth, expectedValue)) {
            return err(
                "unsupported reserved bits field `"
                    + fieldName
                    + "` in schema `"
                    + schemaName
                    + "`"
            );
        }
        int byteWidth = reservedByteWidth(bitWidth);
        int requiredBytes = byteWidth > 0 ? byteWidth : packedReservedStorageByteWidth(bitWidth);
        long available = Math.max(0L, count - relativeOffset);
        if (available < requiredBytes) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                fieldName,
                requiredBytes,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long actualValue = 0L;
        if (byteWidth > 0) {
            for (int index = 0; index < byteWidth; index += 1) {
                actualValue =
                    (actualValue << 8)
                        | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
            }
        } else {
            long combined = 0L;
            for (int index = 0; index < requiredBytes; index += 1) {
                combined =
                    (combined << 8)
                        | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
            }
            long mask = reservedMaxValue(bitWidth);
            actualValue =
                (combined >> (requiredBytes * 8 - bitWidth)) & mask;
        }
        if (actualValue != expectedValue) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "reserved bits mismatch at byte offset " + byteOffset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                byteOffset,
                schemaName,
                fieldName,
                bitWidth,
                expectedValue,
                actualValue,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        return ok(Long.valueOf(actualValue));
    }

    private static Object schemaReadPackedUnsignedWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        int bitWidth,
        int storageByteWidth,
        String schemaName,
        String fieldName
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        if (available < storageByteWidth) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                fieldName,
                storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        long value = combined & reservedMaxValue(bitWidth);
        return ok(Long.valueOf(value));
    }

    private static Object schemaReadSuffixPackedUnsignedWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        int visibleBitWidth,
        SchemaFieldSpec reserved,
        String schemaName,
        String fieldName,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        int requiredBytes = packedReservedStorageByteWidth(reserved.reservedBitWidth);
        long available = Math.max(0L, count - relativeOffset);
        if (available < requiredBytes) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldPathDiagnostic(
                message,
                byteOffset,
                fieldPath,
                requiredBytes,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < requiredBytes; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        long actualReserved = combined & reservedMaxValue(reserved.reservedBitWidth);
        if (actualReserved != reserved.reservedValue) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "reserved bits mismatch at byte offset " + byteOffset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                byteOffset,
                schemaName,
                reserved.name,
                reserved.reservedBitWidth,
                reserved.reservedValue,
                actualReserved,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long value = (combined >> reserved.reservedBitWidth) & reservedMaxValue(visibleBitWidth);
        return ok(Long.valueOf(value));
    }

    private static Object schemaReadReservedBytePrefixUnsignedWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        SchemaFieldSpec reserved,
        SchemaFieldSpec visible,
        String schemaName,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        int storageByteWidth = reservedBytePrefixStorageByteWidth(reserved, visible);
        long available = Math.max(0L, count - relativeOffset);
        if (available < storageByteWidth) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + visible.name + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldPathDiagnostic(
                message,
                byteOffset,
                fieldPath,
                storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        long actualReserved =
            (combined >> (storageByteWidth * 8 - reserved.reservedBitWidth))
                & reservedMaxValue(reserved.reservedBitWidth);
        if (actualReserved != reserved.reservedValue) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "reserved bits mismatch at byte offset " + byteOffset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                byteOffset,
                schemaName,
                reserved.name,
                reserved.reservedBitWidth,
                reserved.reservedValue,
                actualReserved,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        int visibleBitWidth = reservedBytePrefixVisibleBitWidth(reserved, visible);
        int lowPaddingBitWidth = storageByteWidth * 8 - reserved.reservedBitWidth - visibleBitWidth;
        long value = (combined >> lowPaddingBitWidth) & reservedMaxValue(visibleBitWidth);
        return ok(Long.valueOf(value));
    }

    private static Object schemaReadMiddleReservedGroupWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        SchemaFieldSpec left,
        SchemaFieldSpec reserved,
        SchemaFieldSpec right,
        int storageByteWidth,
        String schemaName
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        if (available < storageByteWidth) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + left.name + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                left.name,
                storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        int leftBitWidth = bitPackedVisibleBitWidth(left);
        int rightBitWidth = bitPackedVisibleBitWidth(right);
        long actualReserved =
            (combined >> rightBitWidth) & reservedMaxValue(reserved.reservedBitWidth);
        if (actualReserved != reserved.reservedValue) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "reserved bits mismatch at byte offset " + byteOffset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                byteOffset,
                schemaName,
                reserved.name,
                reserved.reservedBitWidth,
                reserved.reservedValue,
                actualReserved,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long leftValue =
            (combined >> (reserved.reservedBitWidth + rightBitWidth))
                & reservedMaxValue(leftBitWidth);
        long rightValue = combined & reservedMaxValue(rightBitWidth);
        return ok(new long[] { leftValue, rightValue });
    }

    private static Object schemaReadByteInterleavedReservedGroupWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        SchemaFieldSpec[] fields,
        int fieldIndex,
        int storageByteWidth,
        String schemaName,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPathPrefix
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        SchemaFieldSpec first = fields[fieldIndex];
        if (available < storageByteWidth) {
            long byteOffset = diagnosticBaseOffset + count;
            String message =
                "truncated schema field `" + first.name + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldPathDiagnostic(
                message,
                byteOffset,
                schemaFieldPathWithPrefix(fieldPathPrefix, schemaName, first.name),
                storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        long[] groupValues = new long[4];
        int remainingBitWidth = storageByteWidth * 8;
        for (int offset = 0; offset < 4; offset += 1) {
            SchemaFieldSpec field = fields[fieldIndex + offset];
            int bitWidth = field.isReserved()
                ? field.reservedBitWidth
                : bitPackedVisibleBitWidth(field);
            remainingBitWidth -= bitWidth;
            long value = (combined >> remainingBitWidth) & reservedMaxValue(bitWidth);
            if (field.isReserved()) {
                if (value != field.reservedValue) {
                    long byteOffset = diagnosticBaseOffset + relativeOffset;
                    String message = "reserved bits mismatch at byte offset " + byteOffset;
                    registerSchemaReservedBitsMismatchDiagnostic(
                        message,
                        byteOffset,
                        schemaName,
                        field.name,
                        field.reservedBitWidth,
                        field.reservedValue,
                        value,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
            } else {
                groupValues[offset] = value;
            }
        }
        return ok(groupValues);
    }

    private static Object schemaReadBitPackedFieldGroupWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        SchemaFieldSpec[] fields,
        int fieldIndex,
        BitPackedFieldGroupSpec group,
        String schemaName,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPathPrefix
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        SchemaFieldSpec first = fields[fieldIndex];
        if (available < group.storageByteWidth) {
            SchemaFieldSpec truncatedField = first;
            if (group.fieldCount == 2 && first.isReserved() && first.reservedBitWidth == 1) {
                SchemaFieldSpec second = fields[fieldIndex + 1];
                if (!second.isReserved() && bitPackedGroupFieldBitWidth(second) == 31) {
                    truncatedField = second;
                }
            }
            long byteOffset = diagnosticBaseOffset + count;
            String message =
                "truncated schema field `" + truncatedField.name + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldPathDiagnostic(
                message,
                byteOffset,
                schemaFieldPathWithPrefix(fieldPathPrefix, schemaName, truncatedField.name),
                group.storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < group.storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        long[] groupValues = new long[group.fieldCount];
        int remainingBitWidth = group.totalBitWidth;
        for (int offset = 0; offset < group.fieldCount; offset += 1) {
            SchemaFieldSpec field = fields[fieldIndex + offset];
            int bitWidth = bitPackedGroupFieldBitWidth(field);
            remainingBitWidth -= bitWidth;
            long value = (combined >> remainingBitWidth) & reservedMaxValue(bitWidth);
            if (field.isReserved()) {
                if (value != field.reservedValue) {
                    long byteOffset = diagnosticBaseOffset + relativeOffset;
                    String message = "reserved bits mismatch at byte offset " + byteOffset;
                    registerSchemaReservedBitsMismatchDiagnostic(
                        message,
                        byteOffset,
                        schemaName,
                        field.name,
                        field.reservedBitWidth,
                        field.reservedValue,
                        value,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
            } else {
                groupValues[offset] = value;
            }
        }
        return ok(groupValues);
    }

    private static Object schemaReadPrefixReservedGroupWithBase(
        Object view,
        long diagnosticBaseOffset,
        int relativeOffset,
        SchemaFieldSpec reserved,
        SchemaFieldSpec first,
        SchemaFieldSpec second,
        int storageByteWidth,
        String schemaName
    ) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long storageBaseOffset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, storageBaseOffset, count);
        if (boundsError != null) {
            return boundsError;
        }
        long available = Math.max(0L, count - relativeOffset);
        if (available < storageByteWidth) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + reserved.name + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                reserved.name,
                storageByteWidth,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long combined = 0L;
        for (int index = 0; index < storageByteWidth; index += 1) {
            combined =
                (combined << 8)
                    | byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset + index));
        }
        int firstBitWidth = bitPackedVisibleBitWidth(first);
        int secondBitWidth = bitPackedVisibleBitWidth(second);
        long actualReserved =
            (combined >> (firstBitWidth + secondBitWidth))
                & reservedMaxValue(reserved.reservedBitWidth);
        if (actualReserved != reserved.reservedValue) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "reserved bits mismatch at byte offset " + byteOffset;
            registerSchemaReservedBitsMismatchDiagnostic(
                message,
                byteOffset,
                schemaName,
                reserved.name,
                reserved.reservedBitWidth,
                reserved.reservedValue,
                actualReserved,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long firstValue = (combined >> secondBitWidth) & reservedMaxValue(firstBitWidth);
        long secondValue = combined & reservedMaxValue(secondBitWidth);
        return ok(new long[] { firstValue, secondValue });
    }

    private static Object schemaPutDecodedVisibleField(
        java.util.LinkedHashMap<String, Long> decoded,
        java.util.LinkedHashMap<String, Object> values,
        SchemaFieldSpec field,
        long fieldValue,
        BinarySchemaSpec schema,
        long relativeOffset,
        Object view,
        long diagnosticBaseOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPathPrefix
    ) {
        if (field.fixedValue != null && fieldValue != field.fixedValue.longValue()) {
            long byteOffset = diagnosticBaseOffset + relativeOffset;
            String message = "fixed field mismatch at byte offset " + byteOffset;
            registerSchemaFixedFieldMismatchPathDiagnostic(
                message,
                byteOffset,
                schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                field.fixedValue.longValue(),
                fieldValue,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        decoded.put(field.name, Long.valueOf(fieldValue));
        values.put(
            field.name,
            field.isFlagBitset()
                ? schemaFlagObject(field, fieldValue)
                : Long.valueOf(fieldValue)
        );

        if (field.predicate != null) {
            boolean passed;
            try {
                passed = field.predicate.eval(decoded);
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
            if (!passed) {
                long byteOffset = diagnosticBaseOffset + relativeOffset;
                String message = "schema validation failed at byte offset " + byteOffset;
                registerSchemaValidationFailedDiagnostic(
                    message,
                    byteOffset,
                    schema.name,
                    field.name,
                    field.predicate.text,
                    fieldValue,
                    decoded,
                    byteViewPreviewTrace(view, 8)
                );
                return err(message);
            }
        }
        return ok(null);
    }

    private static Object byteDecodeBinarySchema(Object view, BinarySchemaSpec schema) {
        return byteDecodeBinarySchemaWithBase(view, schema, byteViewOffset(view));
    }

    private static Object byteDecodeBinarySchemaWithBase(
        Object view,
        BinarySchemaSpec schema,
        long diagnosticBaseOffset
    ) {
        return byteDecodeBinarySchemaWithBasePath(view, schema, diagnosticBaseOffset, null);
    }

    private static Object byteDecodeBinarySchemaAtPath(
        Object view,
        BinarySchemaSpec schema,
        long diagnosticBaseOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPathPrefix
    ) {
        Object decoded = byteDecodeBinarySchemaWithBasePath(
            view,
            schema,
            diagnosticBaseOffset,
            fieldPathPrefix
        );
        if (isErr(decoded)) {
            return decoded;
        }
        long consumed = schemaConsumedWidth(view, schema, diagnosticBaseOffset);
        return ok(record(
            "value",
            unwrapOk(decoded),
            "consumed",
            Long.valueOf(consumed)
        ));
    }

    private static Object byteDecodeBinarySchemaWithBasePath(
        Object view,
        BinarySchemaSpec schema,
        long diagnosticBaseOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPathPrefix
    ) {
        Object boundsError = byteViewBoundsError(view);
        if (boundsError != null) {
            return boundsError;
        }

        Object unsupportedPredicate = unsupportedSchemaPredicate(schema);
        if (unsupportedPredicate != null) {
            return unsupportedPredicate;
        }

        java.util.LinkedHashMap<String, Long> decoded = new java.util.LinkedHashMap<String, Long>();
        java.util.LinkedHashMap<String, Object> values = new java.util.LinkedHashMap<String, Object>();
        long relativeOffset = 0L;
        SchemaFieldSpec pendingPackedReserved = null;
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            int prefixStorageByteWidth = prefixReservedGroupStorageByteWidth(schema.fields, index);
            if (prefixStorageByteWidth > 0) {
                SchemaFieldSpec first = schema.fields[index + 1];
                SchemaFieldSpec second = schema.fields[index + 2];
                Object read = schemaReadPrefixReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    first,
                    second,
                    prefixStorageByteWidth,
                    schema.name
                );
                if (isErr(read)) {
                    return read;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                Object firstStore = schemaPutDecodedVisibleField(
                    decoded,
                    values,
                    first,
                    groupValues[0],
                    schema,
                    relativeOffset,
                    view,
                    diagnosticBaseOffset,
                    fieldPathPrefix
                );
                if (isErr(firstStore)) {
                    return firstStore;
                }
                Object secondStore = schemaPutDecodedVisibleField(
                    decoded,
                    values,
                    second,
                    groupValues[1],
                    schema,
                    relativeOffset,
                    view,
                    diagnosticBaseOffset,
                    fieldPathPrefix
                );
                if (isErr(secondStore)) {
                    return secondStore;
                }
                relativeOffset += prefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            int byteInterleavedStorageByteWidth =
                byteInterleavedReservedGroupStorageByteWidth(schema.fields, index);
            if (byteInterleavedStorageByteWidth > 0) {
                Object read = schemaReadByteInterleavedReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    schema.fields,
                    index,
                    byteInterleavedStorageByteWidth,
                    schema.name,
                    fieldPathPrefix
                );
                if (isErr(read)) {
                    return read;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                for (int offset = 0; offset < 4; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    if (groupField.isReserved()) {
                        continue;
                    }
                    Object store = schemaPutDecodedVisibleField(
                        decoded,
                        values,
                        groupField,
                        groupValues[offset],
                        schema,
                        relativeOffset,
                        view,
                        diagnosticBaseOffset,
                        fieldPathPrefix
                    );
                    if (isErr(store)) {
                        return store;
                    }
                }
                relativeOffset += byteInterleavedStorageByteWidth;
                pendingPackedReserved = null;
                index += 3;
                continue;
            }
            BitPackedFieldGroupSpec bitPackedGroup =
                bitPackedFieldGroupSpec(schema.fields, index);
            if (bitPackedGroup != null) {
                Object read = schemaReadBitPackedFieldGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    schema.fields,
                    index,
                    bitPackedGroup,
                    schema.name,
                    fieldPathPrefix
                );
                if (isErr(read)) {
                    return read;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                for (int offset = 0; offset < bitPackedGroup.fieldCount; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    if (groupField.isReserved()) {
                        continue;
                    }
                    Object store = schemaPutDecodedVisibleField(
                        decoded,
                        values,
                        groupField,
                        groupValues[offset],
                        schema,
                        relativeOffset,
                        view,
                        diagnosticBaseOffset,
                        fieldPathPrefix
                    );
                    if (isErr(store)) {
                        return store;
                    }
                }
                relativeOffset += bitPackedGroup.storageByteWidth;
                pendingPackedReserved = null;
                index += bitPackedGroup.fieldCount - 1;
                continue;
            }
            int middleStorageByteWidth = middleReservedGroupStorageByteWidth(schema.fields, index);
            if (middleStorageByteWidth > 0) {
                SchemaFieldSpec reserved = schema.fields[index + 1];
                SchemaFieldSpec right = schema.fields[index + 2];
                Object read = schemaReadMiddleReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    reserved,
                    right,
                    middleStorageByteWidth,
                    schema.name
                );
                if (isErr(read)) {
                    return read;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                Object leftStore = schemaPutDecodedVisibleField(
                    decoded,
                    values,
                    field,
                    groupValues[0],
                    schema,
                    relativeOffset,
                    view,
                    diagnosticBaseOffset,
                    fieldPathPrefix
                );
                if (isErr(leftStore)) {
                    return leftStore;
                }
                Object rightStore = schemaPutDecodedVisibleField(
                    decoded,
                    values,
                    right,
                    groupValues[1],
                    schema,
                    relativeOffset,
                    view,
                    diagnosticBaseOffset,
                    fieldPathPrefix
                );
                if (isErr(rightStore)) {
                    return rightStore;
                }
                relativeOffset += middleStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            SchemaFieldSpec reservedBytePrefixVisible =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int reservedBytePrefixStorageByteWidth =
                reservedBytePrefixStorageByteWidth(field, reservedBytePrefixVisible);
            if (reservedBytePrefixStorageByteWidth > 0) {
                Object read = schemaReadReservedBytePrefixUnsignedWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    reservedBytePrefixVisible,
                    schema.name,
                    schemaFieldPathWithPrefix(
                        fieldPathPrefix,
                        schema.name,
                        reservedBytePrefixVisible.name
                    )
                );
                if (isErr(read)) {
                    return read;
                }
                Object store = schemaPutDecodedVisibleField(
                    decoded,
                    values,
                    reservedBytePrefixVisible,
                    asLong(unwrapOk(read)),
                    schema,
                    relativeOffset,
                    view,
                    diagnosticBaseOffset,
                    fieldPathPrefix
                );
                if (isErr(store)) {
                    return store;
                }
                relativeOffset += reservedBytePrefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 1;
                continue;
            }
            if (field.isReserved()) {
                Object read = schemaReadReservedBits(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field.reservedBitWidth,
                    field.reservedValue,
                    schema.name,
                    field.name
                );
                if (isErr(read)) {
                    return read;
                }
                int reservedByteWidth = reservedByteWidth(field.reservedBitWidth);
                if (reservedByteWidth > 0) {
                    relativeOffset += reservedByteWidth;
                    pendingPackedReserved = null;
                } else {
                    pendingPackedReserved = field;
                }
                continue;
            }
            if (field.isLengthBoundedView()) {
                pendingPackedReserved = null;
                Object payloadLengthValue = schemaLengthExpressionValue(
                    decoded,
                    field.lengthField,
                    "payload",
                    schema.name,
                    Long.valueOf(diagnosticBaseOffset + relativeOffset),
                    schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                    byteViewPreviewTrace(view, 8)
                );
                if (isErr(payloadLengthValue)) {
                    return payloadLengthValue;
                }
                long payloadLength = asLong(unwrapOk(payloadLengthValue));
                long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                if (payloadLength < 0L || payloadLength > availablePayload) {
                    long byteOffset = diagnosticBaseOffset + relativeOffset + availablePayload;
                    String message = "payload length out of bounds at byte offset " + byteOffset;
                    registerSchemaLengthOutOfBoundsPathDiagnostic(
                        message,
                        byteOffset,
                        schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                        payloadLength,
                        availablePayload,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
                Object payloadView = byteViewDerivedObject(view, relativeOffset, payloadLength);
                values.put(field.name, payloadView);
                relativeOffset += payloadLength;
                continue;
            }
            if (field.isRepeat()) {
                pendingPackedReserved = null;
                Object repeatCountValue = schemaRepeatCountExpressionValue(
                    decoded,
                    field.repeat.countField,
                    schema.name,
                    Long.valueOf(diagnosticBaseOffset + relativeOffset),
                    schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                    byteViewPreviewTrace(view, 8)
                );
                if (isErr(repeatCountValue)) {
                    return repeatCountValue;
                }
                long repeatCount = asLong(unwrapOk(repeatCountValue));
                if (repeatCount < 0L) {
                    long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                    long byteOffset = diagnosticBaseOffset + relativeOffset + availablePayload;
                    String message = "repeat count out of bounds at byte offset " + byteOffset;
                    registerSchemaLengthOutOfBoundsPathDiagnostic(
                        message,
                        byteOffset,
                        schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                        repeatCount,
                        availablePayload,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
                java.util.ArrayList<Object> elements = new java.util.ArrayList<Object>();
                long repeatRelativeOffset = relativeOffset;
                for (long elementIndex = 0L; elementIndex < repeatCount; elementIndex += 1L) {
                    if (field.repeat.isByteViewPayload()) {
                        Object payloadLengthValue = schemaLengthExpressionValue(
                            decoded,
                            field.repeat.byteViewLengthField,
                            "repeat payload",
                            schema.name,
                            Long.valueOf(diagnosticBaseOffset + repeatRelativeOffset),
                            schemaRepeatedFieldPath(schema.name, field.name, elementIndex),
                            byteViewPreviewTrace(view, 8)
                        );
                        if (isErr(payloadLengthValue)) {
                            return payloadLengthValue;
                        }
                        long payloadLength = asLong(unwrapOk(payloadLengthValue));
                        long availablePayload =
                            Math.max(0L, byteViewCountLong(view) - repeatRelativeOffset);
                        if (availablePayload < payloadLength) {
                            long byteOffset =
                                diagnosticBaseOffset + repeatRelativeOffset + availablePayload;
                            String message =
                                "truncated schema field `" + field.name + "` at byte offset " + byteOffset;
                            registerSchemaTruncatedFieldPathDiagnostic(
                                message,
                                byteOffset,
                                schemaRepeatedFieldPath(schema.name, field.name, elementIndex),
                                payloadLength,
                                availablePayload,
                                byteViewPreviewTrace(view, 8)
                            );
                            return err(message);
                        }
                        Object payloadView =
                            byteViewDerivedObject(view, repeatRelativeOffset, payloadLength);
                        elements.add(payloadView);
                        repeatRelativeOffset += payloadLength;
                    } else if (field.repeat.payloadSchema != null) {
                        Object elementView = byteViewDerivedObject(
                            view,
                            repeatRelativeOffset,
                            Math.max(0L, byteViewCountLong(view) - repeatRelativeOffset)
                        );
                        Object read = byteDecodeBinarySchemaAtPath(
                            elementView,
                            field.repeat.payloadSchema,
                            diagnosticBaseOffset + repeatRelativeOffset,
                            schemaRepeatedFieldPath(schema.name, field.name, elementIndex)
                        );
                        if (isErr(read)) {
                            return read;
                        }
                        java.util.Map<Object, Object> value = asMap(unwrapOk(read));
                        elements.add(value.get("value"));
                        repeatRelativeOffset += asLong(value.get("consumed"));
                    } else {
                        Object read = schemaReadUnsignedWithBasePath(
                            view,
                            diagnosticBaseOffset,
                            (int) repeatRelativeOffset,
                            field.repeat.width,
                            field.repeat.maxValue,
                            field.repeat.littleEndian,
                            schema.name,
                            field.name,
                            schemaRepeatedFieldPath(schema.name, field.name, elementIndex)
                        );
                        if (isErr(read)) {
                            return read;
                        }
                        elements.add(Long.valueOf(asLong(unwrapOk(read))));
                        repeatRelativeOffset += field.repeat.width;
                    }
                }
                Object repeated = listNil();
                for (int elementIndex = elements.size() - 1; elementIndex >= 0; elementIndex -= 1) {
                    repeated = listCons(elements.get(elementIndex), repeated);
                }
                decoded.put(field.name, Long.valueOf(0L));
                values.put(field.name, repeated);
                relativeOffset = repeatRelativeOffset;
                continue;
            }
            if (field.dispatch != null) {
                pendingPackedReserved = null;
                SchemaDispatchCaseSpec dispatchCase = schemaDispatchCase(field, decoded);
                if (dispatchCase == null) {
                    Long tagValue = decoded.get(field.dispatch.tagField);
                    if (field.dispatch.preservesUnknown && tagValue != null) {
                        Long payloadLength = decoded.get(field.dispatch.lengthField);
                        if (payloadLength == null) {
                            return err(
                                "missing dispatch length field `"
                                    + field.dispatch.lengthField
                                    + "` in schema `"
                                    + schema.name
                                    + "`"
                            );
                        }
                        long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                        if (payloadLength.longValue() > availablePayload) {
                            long byteOffset =
                                diagnosticBaseOffset + relativeOffset + availablePayload;
                            String message = "payload length out of bounds at byte offset "
                                + byteOffset;
                            registerSchemaLengthOutOfBoundsPathDiagnostic(
                                message,
                                byteOffset,
                                schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                                payloadLength.longValue(),
                                availablePayload,
                                byteViewPreviewTrace(view, 8)
                            );
                            return err(message);
                        }
                        Object payload = byteViewDerivedObject(view, relativeOffset, payloadLength.longValue());
                        values.put(
                            field.name,
                            adt(
                                "SchemaDispatchPayload::Unknown",
                                new Object[] { tagValue, payload }
                            )
                        );
                        relativeOffset += payloadLength.longValue();
                        continue;
                    }
                    long byteOffset = diagnosticBaseOffset + relativeOffset;
                    String message = "unknown dispatch tag at byte offset " + byteOffset;
                    registerSchemaDispatchUnknownTagDiagnostic(
                        message,
                        byteOffset,
                        schema.name,
                        field.name,
                        field.dispatch.tagField,
                        tagValue == null ? -1L : tagValue.longValue(),
                        schemaDispatchExpectedTags(field.dispatch),
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
                BinarySchemaSpec dispatchPayloadSchema = dispatchCase.payloadSchema(schema);
                long payloadWidth = dispatchPayloadSchema == null ? dispatchCase.width : 0L;
                boolean lengthBoundedPayload = !field.dispatch.lengthField.isEmpty();
                if (lengthBoundedPayload) {
                    Long payloadLength = decoded.get(field.dispatch.lengthField);
                    if (payloadLength == null) {
                        return err(
                            "missing dispatch length field `"
                                + field.dispatch.lengthField
                                + "` in schema `"
                                + schema.name
                                + "`"
                        );
                    }
                    long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                    if (payloadLength.longValue() > availablePayload) {
                        long byteOffset =
                            diagnosticBaseOffset + relativeOffset + availablePayload;
                        String message = "payload length out of bounds at byte offset "
                            + byteOffset;
                        registerSchemaLengthOutOfBoundsPathDiagnostic(
                            message,
                            byteOffset,
                            schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                            payloadLength.longValue(),
                            availablePayload,
                            byteViewPreviewTrace(view, 8)
                        );
                        return err(message);
                    }
                    payloadWidth = payloadLength.longValue();
                }
                Object dispatchValue;
                if (dispatchPayloadSchema != null) {
                    long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                    long payloadViewCount = lengthBoundedPayload
                        ? Math.min(payloadWidth, availablePayload)
                        : availablePayload;
                    Object payloadView = byteViewDerivedObject(view, relativeOffset, payloadViewCount);
                    Object nestedRead = byteDecodeBinarySchemaAtPath(
                        payloadView,
                        dispatchPayloadSchema,
                        diagnosticBaseOffset + relativeOffset,
                        schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name)
                    );
                    if (isErr(nestedRead)) {
                        return nestedRead;
                    }
                    java.util.Map<Object, Object> nestedValue = asMap(unwrapOk(nestedRead));
                    dispatchValue = nestedValue.get("value");
                    if (!lengthBoundedPayload) {
                        payloadWidth = asLong(nestedValue.get("consumed"));
                    }
                } else {
                    Object readView = view;
                    long readBaseOffset = diagnosticBaseOffset;
                    int readRelativeOffset = (int) relativeOffset;
                    if (lengthBoundedPayload) {
                        readView = byteViewDerivedObject(view, relativeOffset, payloadWidth);
                        readBaseOffset = diagnosticBaseOffset + relativeOffset;
                        readRelativeOffset = 0;
                    }
                    Object dispatchRead = schemaReadUnsignedWithBase(
                        readView,
                        readBaseOffset,
                        readRelativeOffset,
                        dispatchCase.width,
                        dispatchCase.maxValue,
                        dispatchCase.littleEndian,
                        schema.name,
                        field.name
                    );
                    if (isErr(dispatchRead)) {
                        return dispatchRead;
                    }
                    dispatchValue = Long.valueOf(asLong(unwrapOk(dispatchRead)));
                }
                decoded.put(
                    field.name,
                    dispatchValue instanceof Long ? (Long) dispatchValue : Long.valueOf(0L)
                );
                if (field.dispatch.preservesUnknown) {
                    values.put(
                        field.name,
                        adt(
                            "SchemaDispatchPayload::Known",
                            new Object[] { dispatchValue }
                        )
                    );
                } else {
                    values.put(field.name, dispatchValue);
                }
                relativeOffset += payloadWidth;
                continue;
            }

            int packedVisibleBitWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedVisibleBitWidth(pendingPackedReserved, field);
            int packedStorageByteWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedStorageByteWidth(pendingPackedReserved, field);
            SchemaFieldSpec suffixReserved =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int suffixVisibleBitWidth = suffixPackedVisibleBitWidth(field, suffixReserved);
            int suffixStorageByteWidth = suffixPackedStorageByteWidth(field, suffixReserved);
            Object read =
                suffixVisibleBitWidth > 0
                    ? schemaReadSuffixPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        suffixVisibleBitWidth,
                        suffixReserved,
                        schema.name,
                        field.name,
                        schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name)
                    )
                    : packedVisibleBitWidth > 0
                    ? schemaReadPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        packedVisibleBitWidth,
                        packedStorageByteWidth,
                        schema.name,
                        field.name
                    )
                    : schemaReadUnsignedWithBasePath(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        field.width,
                        field.maxValue,
                        field.littleEndian,
                        schema.name,
                        field.name,
                        schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name)
                    );
            if (isErr(read)) {
                return read;
            }
            long fieldValue = asLong(unwrapOk(read));
            if (field.fixedValue != null && fieldValue != field.fixedValue.longValue()) {
                long byteOffset = diagnosticBaseOffset + relativeOffset;
                String message = "fixed field mismatch at byte offset " + byteOffset;
                registerSchemaFixedFieldMismatchPathDiagnostic(
                    message,
                    byteOffset,
                    schemaFieldPathWithPrefix(fieldPathPrefix, schema.name, field.name),
                    field.fixedValue.longValue(),
                    fieldValue,
                    byteViewPreviewTrace(view, 8)
                );
                return err(message);
            }
            decoded.put(field.name, Long.valueOf(fieldValue));
            values.put(
                field.name,
                field.isFlagBitset()
                    ? schemaFlagObject(field, fieldValue)
                    : Long.valueOf(fieldValue)
            );

            if (field.predicate != null) {
                boolean passed;
                try {
                    passed = field.predicate.eval(decoded);
                } catch (IllegalStateException failure) {
                    return err(failure.getMessage());
                }
                if (!passed) {
                    long byteOffset = diagnosticBaseOffset + relativeOffset;
                    String message = "schema validation failed at byte offset " + byteOffset;
                    registerSchemaValidationFailedDiagnostic(
                        message,
                        byteOffset,
                        schema.name,
                        field.name,
                        field.predicate.text,
                        fieldValue,
                        decoded,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
            }

            relativeOffset += suffixVisibleBitWidth > 0
                ? suffixStorageByteWidth
                : packedVisibleBitWidth > 0 ? packedStorageByteWidth : field.width;
            pendingPackedReserved = null;
            if (suffixVisibleBitWidth > 0) {
                index += 1;
            }
        }

        if (schema.validation != null) {
            boolean passed;
            try {
                passed = schema.validation.eval(decoded);
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
            if (!passed) {
                long byteOffset = diagnosticBaseOffset + relativeOffset;
                String message = "schema validation failed at byte offset " + byteOffset;
                registerSchemaStructuralValidationFailedDiagnostic(
                    message,
                    byteOffset,
                    schema.name,
                    schema.validation.text,
                    decoded,
                    byteViewPreviewTrace(view, 8)
                );
                return err(message);
            }
        }

        Object decodedRecord = schemaDecodedRecord(
            schema,
            values,
            diagnosticBaseOffset + relativeOffset,
            byteViewPreviewTrace(view, 8)
        );
        if (isErr(decodedRecord)) {
            return decodedRecord;
        }
        return ok(decodedRecord);
    }

    private static Object byteDecodeStepBinarySchema(
        Object view,
        Object baseOffset,
        BinarySchemaSpec schema
    ) {
        Object boundsError = byteViewBoundsError(view);
        if (boundsError != null) {
            return invalidDecodeStep("schema.byte_view", baseOffset, schema.name);
        }

        long totalWidth = schemaTotalWidth(schema);
        if (byteViewCountLong(view) < totalWidth) {
            return adt(
                "DecodeStep::NeedMore",
                new Object[] {
                    adt("DecodeReadiness::NeedBytes", new Object[] { byteCountObject(totalWidth) })
                }
            );
        }

        long diagnosticBaseOffset = byteOffsetLong(baseOffset);
        Object decoded = byteDecodeBinarySchemaWithBase(view, schema, diagnosticBaseOffset);
        if (isErr(decoded)) {
            return invalidDecodeStepFromSchemaDecodeError(decoded, baseOffset, schema.name);
        }
        long consumedWidth = schemaConsumedWidth(view, schema, diagnosticBaseOffset);
        return adt(
            "DecodeStep::Decoded",
            new Object[] { unwrapOk(decoded), byteCountObject(consumedWidth) }
        );
    }

    private static long schemaConsumedWidth(
        Object view,
        BinarySchemaSpec schema,
        long diagnosticBaseOffset
    ) {
        java.util.LinkedHashMap<String, Long> decoded = new java.util.LinkedHashMap<String, Long>();
        long relativeOffset = 0L;
        SchemaFieldSpec pendingPackedReserved = null;
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            int byteInterleavedStorageByteWidth =
                byteInterleavedReservedGroupStorageByteWidth(schema.fields, index);
            if (byteInterleavedStorageByteWidth > 0) {
                Object read = schemaReadByteInterleavedReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    schema.fields,
                    index,
                    byteInterleavedStorageByteWidth,
                    schema.name,
                    null
                );
                if (isErr(read)) {
                    return relativeOffset;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                for (int offset = 0; offset < 4; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    if (!groupField.isReserved()) {
                        decoded.put(groupField.name, Long.valueOf(groupValues[offset]));
                    }
                }
                relativeOffset += byteInterleavedStorageByteWidth;
                pendingPackedReserved = null;
                index += 3;
                continue;
            }
            BitPackedFieldGroupSpec bitPackedGroup =
                bitPackedFieldGroupSpec(schema.fields, index);
            if (bitPackedGroup != null) {
                Object read = schemaReadBitPackedFieldGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    schema.fields,
                    index,
                    bitPackedGroup,
                    schema.name,
                    null
                );
                if (isErr(read)) {
                    return relativeOffset;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                for (int offset = 0; offset < bitPackedGroup.fieldCount; offset += 1) {
                    SchemaFieldSpec groupField = schema.fields[index + offset];
                    if (!groupField.isReserved()) {
                        decoded.put(groupField.name, Long.valueOf(groupValues[offset]));
                    }
                }
                relativeOffset += bitPackedGroup.storageByteWidth;
                pendingPackedReserved = null;
                index += bitPackedGroup.fieldCount - 1;
                continue;
            }
            int prefixStorageByteWidth = prefixReservedGroupStorageByteWidth(schema.fields, index);
            if (prefixStorageByteWidth > 0) {
                SchemaFieldSpec first = schema.fields[index + 1];
                SchemaFieldSpec second = schema.fields[index + 2];
                Object read = schemaReadPrefixReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    first,
                    second,
                    prefixStorageByteWidth,
                    schema.name
                );
                if (isErr(read)) {
                    return relativeOffset;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                decoded.put(first.name, Long.valueOf(groupValues[0]));
                decoded.put(second.name, Long.valueOf(groupValues[1]));
                relativeOffset += prefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            int middleStorageByteWidth = middleReservedGroupStorageByteWidth(schema.fields, index);
            if (middleStorageByteWidth > 0) {
                SchemaFieldSpec reserved = schema.fields[index + 1];
                SchemaFieldSpec right = schema.fields[index + 2];
                Object read = schemaReadMiddleReservedGroupWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    reserved,
                    right,
                    middleStorageByteWidth,
                    schema.name
                );
                if (isErr(read)) {
                    return relativeOffset;
                }
                long[] groupValues = (long[]) unwrapOk(read);
                decoded.put(field.name, Long.valueOf(groupValues[0]));
                decoded.put(right.name, Long.valueOf(groupValues[1]));
                relativeOffset += middleStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            SchemaFieldSpec reservedBytePrefixVisible =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int reservedBytePrefixStorageByteWidth =
                reservedBytePrefixStorageByteWidth(field, reservedBytePrefixVisible);
            if (reservedBytePrefixStorageByteWidth > 0) {
                Object read = schemaReadReservedBytePrefixUnsignedWithBase(
                    view,
                    diagnosticBaseOffset,
                    (int) relativeOffset,
                    field,
                    reservedBytePrefixVisible,
                    schema.name,
                    null
                );
                if (isErr(read)) {
                    return relativeOffset;
                }
                decoded.put(
                    reservedBytePrefixVisible.name,
                    Long.valueOf(asLong(unwrapOk(read)))
                );
                relativeOffset += reservedBytePrefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 1;
                continue;
            }
            if (field.isReserved()) {
                int reservedByteWidth = reservedByteWidth(field.reservedBitWidth);
                if (reservedByteWidth > 0) {
                    relativeOffset += reservedByteWidth;
                    pendingPackedReserved = null;
                } else {
                    pendingPackedReserved = field;
                }
                continue;
            }
            if (field.isLengthBoundedView()) {
                pendingPackedReserved = null;
                Object payloadLengthValue = schemaLengthExpressionValue(
                    decoded,
                    field.lengthField,
                    "payload",
                    schema.name
                );
                if (isErr(payloadLengthValue)) {
                    return relativeOffset;
                }
                long payloadLength = asLong(unwrapOk(payloadLengthValue));
                if (payloadLength < 0L) {
                    return relativeOffset;
                }
                relativeOffset += payloadLength;
                continue;
            }
            if (field.isRepeat()) {
                pendingPackedReserved = null;
                Object repeatCountValue = schemaRepeatCountExpressionValue(
                    decoded,
                    field.repeat.countField,
                    schema.name
                );
                if (isErr(repeatCountValue)) {
                    return relativeOffset;
                }
                long repeatCount = asLong(unwrapOk(repeatCountValue));
                if (repeatCount < 0L) {
                    return relativeOffset;
                }
                if (field.repeat.isByteViewPayload()) {
                    Object payloadLengthValue = schemaLengthExpressionValue(
                        decoded,
                        field.repeat.byteViewLengthField,
                        "repeat payload",
                        schema.name
                    );
                    if (isErr(payloadLengthValue)) {
                        return relativeOffset;
                    }
                    long payloadLength = asLong(unwrapOk(payloadLengthValue));
                    if (payloadLength < 0L) {
                        return relativeOffset;
                    }
                    relativeOffset += repeatCount * payloadLength;
                } else if (field.repeat.payloadSchema != null) {
                    for (long elementIndex = 0L; elementIndex < repeatCount; elementIndex += 1L) {
                        Object elementView = byteViewDerivedObject(
                            view,
                            relativeOffset,
                            Math.max(0L, byteViewCountLong(view) - relativeOffset)
                        );
                        long consumed = schemaConsumedWidth(
                            elementView,
                            field.repeat.payloadSchema,
                            diagnosticBaseOffset + relativeOffset
                        );
                        if (consumed <= 0L) {
                            return relativeOffset;
                        }
                        relativeOffset += consumed;
                    }
                } else {
                    relativeOffset += repeatCount * field.repeat.width;
                }
                decoded.put(field.name, Long.valueOf(0L));
                continue;
            }
            if (field.dispatch != null) {
                pendingPackedReserved = null;
                SchemaDispatchCaseSpec dispatchCase = schemaDispatchCase(field, decoded);
                if (dispatchCase == null) {
                    if (!field.dispatch.preservesUnknown) {
                        return relativeOffset;
                    }
                    Long payloadLength = decoded.get(field.dispatch.lengthField);
                    if (payloadLength == null) {
                        return relativeOffset;
                    }
                    relativeOffset += payloadLength.longValue();
                    continue;
                }
                BinarySchemaSpec dispatchPayloadSchema = dispatchCase.payloadSchema(schema);
                long payloadWidth = dispatchPayloadSchema == null ? dispatchCase.width : 0L;
                if (!field.dispatch.lengthField.isEmpty()) {
                    Long payloadLength = decoded.get(field.dispatch.lengthField);
                    if (payloadLength == null) {
                        return relativeOffset;
                    }
                    payloadWidth = payloadLength.longValue();
                } else if (dispatchPayloadSchema != null) {
                    Object payloadView = byteViewDerivedObject(
                        view,
                        relativeOffset,
                        Math.max(0L, byteViewCountLong(view) - relativeOffset)
                    );
                    payloadWidth = schemaConsumedWidth(
                        payloadView,
                        dispatchPayloadSchema,
                        diagnosticBaseOffset + relativeOffset
                    );
                    if (payloadWidth <= 0L) {
                        return relativeOffset;
                    }
                }
                decoded.put(field.name, Long.valueOf(0L));
                relativeOffset += payloadWidth;
                continue;
            }
            int packedVisibleBitWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedVisibleBitWidth(pendingPackedReserved, field);
            int packedStorageByteWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedStorageByteWidth(pendingPackedReserved, field);
            SchemaFieldSpec suffixReserved =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int suffixVisibleBitWidth = suffixPackedVisibleBitWidth(field, suffixReserved);
            int suffixStorageByteWidth = suffixPackedStorageByteWidth(field, suffixReserved);
            Object read =
                suffixVisibleBitWidth > 0
                    ? schemaReadSuffixPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        suffixVisibleBitWidth,
                        suffixReserved,
                        schema.name,
                        field.name,
                        schemaFieldPath(schema.name, field.name)
                    )
                    : packedVisibleBitWidth > 0
                    ? schemaReadPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        packedVisibleBitWidth,
                        packedStorageByteWidth,
                        schema.name,
                        field.name
                    )
                    : schemaReadUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        field.width,
                        field.maxValue,
                        field.littleEndian,
                        schema.name,
                        field.name
                    );
            if (isErr(read)) {
                return relativeOffset;
            }
            decoded.put(field.name, Long.valueOf(asLong(unwrapOk(read))));
            relativeOffset += suffixVisibleBitWidth > 0
                ? suffixStorageByteWidth
                : packedVisibleBitWidth > 0 ? packedStorageByteWidth : field.width;
            pendingPackedReserved = null;
            if (suffixVisibleBitWidth > 0) {
                index += 1;
            }
        }
        return relativeOffset;
    }

    private static long schemaTotalWidth(BinarySchemaSpec schema) {
        long total = 0L;
        SchemaFieldSpec pendingPackedReserved = null;
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            int byteInterleavedStorageByteWidth =
                byteInterleavedReservedGroupStorageByteWidth(schema.fields, index);
            if (byteInterleavedStorageByteWidth > 0) {
                total += byteInterleavedStorageByteWidth;
                pendingPackedReserved = null;
                index += 3;
                continue;
            }
            BitPackedFieldGroupSpec bitPackedGroup =
                bitPackedFieldGroupSpec(schema.fields, index);
            if (bitPackedGroup != null) {
                total += bitPackedGroup.storageByteWidth;
                pendingPackedReserved = null;
                index += bitPackedGroup.fieldCount - 1;
                continue;
            }
            int prefixStorageByteWidth = prefixReservedGroupStorageByteWidth(schema.fields, index);
            if (prefixStorageByteWidth > 0) {
                total += prefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            int middleStorageByteWidth = middleReservedGroupStorageByteWidth(schema.fields, index);
            if (middleStorageByteWidth > 0) {
                total += middleStorageByteWidth;
                pendingPackedReserved = null;
                index += 2;
                continue;
            }
            SchemaFieldSpec reservedBytePrefixVisible =
                index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
            int reservedBytePrefixStorageByteWidth =
                reservedBytePrefixStorageByteWidth(field, reservedBytePrefixVisible);
            if (reservedBytePrefixStorageByteWidth > 0) {
                total += reservedBytePrefixStorageByteWidth;
                pendingPackedReserved = null;
                index += 1;
                continue;
            }
            if (field.isReserved()) {
                int reservedByteWidth = reservedByteWidth(field.reservedBitWidth);
                if (reservedByteWidth > 0) {
                    total += reservedByteWidth;
                    pendingPackedReserved = null;
                } else {
                    pendingPackedReserved = field;
                }
                continue;
            } else if (field.isLengthBoundedView()) {
                pendingPackedReserved = null;
                continue;
            } else if (field.isRepeat()) {
                pendingPackedReserved = null;
                continue;
            } else if (field.dispatch == null) {
                SchemaFieldSpec suffixReserved =
                    index + 1 < schema.fields.length ? schema.fields[index + 1] : null;
                int suffixByteWidth =
                    suffixPackedStorageByteWidth(field, suffixReserved);
                int packedByteWidth =
                    pendingPackedReserved == null
                        ? 0
                        : packedStorageByteWidth(pendingPackedReserved, field);
                total += suffixByteWidth > 0
                    ? suffixByteWidth
                    : packedByteWidth > 0 ? packedByteWidth : field.width;
                pendingPackedReserved = null;
                if (suffixByteWidth > 0) {
                    index += 1;
                }
            } else {
                pendingPackedReserved = null;
                total += schemaDispatchMaxWidth(schema, field.dispatch);
            }
        }
        return total;
    }

    private static SchemaDispatchCaseSpec schemaDispatchCase(
        SchemaFieldSpec field,
        java.util.Map<String, Long> decoded
    ) {
        Long tagValue = decoded.get(field.dispatch.tagField);
        if (tagValue == null) {
            return null;
        }
        for (SchemaDispatchCaseSpec dispatchCase : field.dispatch.cases) {
            if (dispatchCase.tag == tagValue.longValue()) {
                return dispatchCase;
            }
        }
        return null;
    }

    private static long schemaDispatchMaxWidth(
        BinarySchemaSpec schema,
        SchemaDispatchSpec dispatch
    ) {
        long width = 0L;
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            width = Math.max(width, dispatchCase.width(schema));
        }
        return width;
    }

    private static String schemaDispatchExpectedTags(SchemaDispatchSpec dispatch) {
        StringBuilder out = new StringBuilder();
        for (int index = 0; index < dispatch.cases.length; index += 1) {
            if (index > 0) {
                out.append(", ");
            }
            out.append(dispatch.cases[index].tag);
        }
        return out.toString();
    }

    private static Object invalidDecodeStep(String id, Object baseOffset, String fieldPath) {
        return adt(
            "DecodeStep::Invalid",
            new Object[] {
                adt(
                    "DecodeError::DecodeError",
                    new Object[] { id, baseOffset, fieldPath }
                )
            }
        );
    }

    private static Object invalidDecodeStepFromSchemaDecodeError(
        Object decoded,
        Object baseOffset,
        String schemaName
    ) {
        ByteDiagnostic diagnostic = byteDiagnosticForMessage(resultValue(decoded));
        if (diagnostic == null) {
            return invalidDecodeStep("schema.decode_failed", baseOffset, schemaName);
        }
        return invalidDecodeStep(
            diagnostic.id,
            byteOffsetObject(diagnostic.byteOffset),
            byteDiagnosticFieldPathDisplay(diagnostic)
        );
    }

    private static Object schemaDecodedRecord(
        BinarySchemaSpec schema,
        java.util.LinkedHashMap<String, Object> decoded,
        long byteOffset,
        String nearbyContext
    ) {
        if (schema.mapping.length > 0) {
            SchemaMappingSpec mapping = selectedSchemaMapping(schema, decoded);
            if (mapping == null) {
                if (schemaHasUnknownExtensionDispatch(schema, decoded)) {
                    return schemaRawDecodedRecord(decoded);
                }
                return err(
                    "schema mapping selection did not match any mapping in schema `"
                        + schema.name
                        + "`"
                );
            }
            java.util.ArrayList<Object> entries =
                new java.util.ArrayList<Object>(mapping.fields.length * 2);
            java.util.LinkedHashMap<String, Object> mappingDecoded =
                schemaMappingDecodedValues(schema, decoded, mapping);
            for (SchemaMappingFieldSpec field : mapping.fields) {
                entries.add(field.target);
                try {
                    entries.add(schemaMappingExprValue(mappingDecoded, field.expr));
                } catch (ArithmeticException failure) {
                    String message = "schema mapping division by zero at byte offset " + byteOffset;
                    registerSchemaMappingDivisionByZeroDiagnostic(
                        message,
                        byteOffset,
                        schema.name,
                        field.target,
                        nearbyContext
                    );
                    return err(message);
                }
            }
            return record(entries.toArray(new Object[entries.size()]));
        }

        return schemaRawDecodedRecord(decoded);
    }

    private static Object schemaRawDecodedRecord(java.util.LinkedHashMap<String, Object> decoded) {
        java.util.ArrayList<Object> entries = new java.util.ArrayList<Object>(decoded.size() * 2);
        for (java.util.Map.Entry<String, Object> entry : decoded.entrySet()) {
            entries.add(entry.getKey());
            entries.add(entry.getValue());
        }
        return record(entries.toArray(new Object[entries.size()]));
    }

    private static java.util.LinkedHashMap<String, Object> schemaMappingDecodedValues(
        BinarySchemaSpec schema,
        java.util.LinkedHashMap<String, Object> decoded,
        SchemaMappingSpec mapping
    ) {
        java.util.LinkedHashMap<String, Object> values =
            new java.util.LinkedHashMap<String, Object>(decoded);
        if (mapping.selectorField == null) {
            return values;
        }
        for (SchemaFieldSpec field : schema.fields) {
            if (field.dispatch == null
                || !field.dispatch.preservesUnknown
                || !field.dispatch.tagField.equals(mapping.selectorField)) {
                continue;
            }
            SchemaDispatchCaseSpec dispatchCase =
                schemaDispatchCaseForTag(field.dispatch, decoded.get(field.dispatch.tagField));
            Object payload = decoded.get(field.name);
            if (dispatchCase != null && isAdtNamed(payload, "SchemaDispatchPayload::Known")) {
                values.put(field.name, asAdt(payload, "SchemaDispatchPayload::Known").payloads[0]);
            }
        }
        return values;
    }

    private static boolean schemaHasUnknownExtensionDispatch(
        BinarySchemaSpec schema,
        java.util.Map<String, Object> decoded
    ) {
        for (SchemaFieldSpec field : schema.fields) {
            if (field.dispatch == null || !field.dispatch.preservesUnknown) {
                continue;
            }
            if (decoded.get(field.dispatch.tagField) instanceof Long
                && schemaDispatchCaseForTag(field.dispatch, decoded.get(field.dispatch.tagField))
                    == null) {
                return true;
            }
        }
        return false;
    }

    private static SchemaMappingSpec selectedSchemaMapping(
        BinarySchemaSpec schema,
        java.util.Map<String, Object> decoded
    ) {
        SchemaMappingSpec selected = null;
        for (SchemaMappingSpec mapping : schema.mapping) {
            if (!mapping.matches(decoded)) {
                continue;
            }
            if (selected != null) {
                return null;
            }
            selected = mapping;
        }
        return selected;
    }

    private static Object schemaEncodeRecord(Object value, BinarySchemaSpec schema) {
        if (schema.mapping.length == 0) {
            return ok(value);
        }
        if (schema.mapping.length == 1 && schema.mapping[0].selectorValue == null) {
            return schemaEncodeRecordWithMapping(value, schema, schema.mapping[0]);
        }
        if (schema.mapping.length > 1) {
            return schemaEncodeSelectedRecord(value, schema);
        }
        return err(encodeError(
            "codec.metadata",
            schema.name,
            "unsupported schema encode mapping selection for schema `" + schema.name + "`"
        ));
    }

    private static Object schemaEncodeSelectedRecord(Object value, BinarySchemaSpec schema) {
        Object selected = null;
        for (SchemaMappingSpec mapping : schema.mapping) {
            if (mapping.selectorValue == null) {
                return err(encodeError(
                    "codec.metadata",
                    schema.name,
                    "unsupported schema encode mapping selection for schema `" + schema.name + "`"
                ));
            }
            Object projected = schemaEncodeRecordWithMapping(value, schema, mapping);
            if (isErr(projected)) {
                if (schemaEncodeProjectionVariantMismatch(projected)) {
                    continue;
                }
                return projected;
            }
            java.util.Map<Object, Object> fields = asMap(unwrapOk(projected));
            if (!mapping.matches(fields)) {
                continue;
            }
            if (selected != null) {
                return err(encodeError(
                    "codec.metadata",
                    schema.name,
                    "ambiguous schema encode mapping selection for schema `" + schema.name + "`"
                ));
            }
            selected = projected;
        }
        if (selected == null) {
            if (schemaEncodeValueHasUnknownExtensionDispatch(value, schema)) {
                return ok(value);
            }
            Object dispatchMismatch = schemaEncodeSelectedClosedDispatchMismatch(value, schema);
            if (dispatchMismatch != null) {
                return dispatchMismatch;
            }
            return err(encodeError(
                "codec.encode_mapping_mismatch",
                schema.name,
                "encoded value does not match a selected mapping selector"
            ));
        }
        return selected;
    }

    private static Object schemaEncodeSelectedClosedDispatchMismatch(
        Object value,
        BinarySchemaSpec schema
    ) {
        if (!(value instanceof java.util.Map)) {
            return null;
        }
        java.util.Map<Object, Object> targetFields = asMap(value);
        for (SchemaFieldSpec field : schema.fields) {
            SchemaDispatchSpec dispatch = field.dispatch;
            if (dispatch == null || dispatch.preservesUnknown) {
                continue;
            }
            if (!schemaSelectedMappingsCoverDispatch(schema.mapping, dispatch)) {
                continue;
            }
            String targetTagField = schemaMappingTargetForSource(schema.mapping, dispatch.tagField);
            if (targetTagField == null || !targetFields.containsKey(targetTagField)) {
                continue;
            }
            Object tag = targetFields.get(targetTagField);
            if (!(tag instanceof Long)) {
                continue;
            }
            long tagValue = ((Long) tag).longValue();
            String fieldPath = schema.name + "." + field.name;
            if (schemaDispatchCaseForTag(dispatch, tag) == null) {
                return err(encodeError(
                    "codec.dispatch_unknown_tag",
                    fieldPath,
                    "tag field `"
                        + dispatch.tagField
                        + "` value "
                        + tagValue
                        + " must be one of "
                        + schemaDispatchExpectedTags(dispatch)
                ));
            }
            return err(encodeError(
                "codec.dispatch_mismatch",
                fieldPath,
                "tag field `" + dispatch.tagField + "` value " + tagValue
                    + " does not match selected payload mapping"
            ));
        }
        return null;
    }

    private static boolean schemaSelectedMappingsCoverDispatch(
        SchemaMappingSpec[] mappings,
        SchemaDispatchSpec dispatch
    ) {
        if (mappings.length == 0 || mappings.length != dispatch.cases.length) {
            return false;
        }
        java.util.LinkedHashSet<Long> tags = new java.util.LinkedHashSet<Long>();
        for (SchemaMappingSpec mapping : mappings) {
            if (!dispatch.tagField.equals(mapping.selectorField)
                || !mapping.selectorOperator.equals("==")
                || mapping.selectorValue == null) {
                return false;
            }
            if (schemaDispatchCaseForTag(dispatch, mapping.selectorValue) == null
                || !tags.add(mapping.selectorValue)) {
                return false;
            }
        }
        return tags.size() == dispatch.cases.length;
    }

    private static String schemaMappingTargetForSource(
        SchemaMappingSpec[] mappings,
        String source
    ) {
        String target = null;
        for (SchemaMappingSpec mapping : mappings) {
            String candidate = null;
            for (SchemaMappingFieldSpec field : mapping.fields) {
                String fieldSource = schemaMappingDirectSource(field.expr);
                if (source.equals(fieldSource)) {
                    candidate = field.target;
                    break;
                }
            }
            if (candidate == null) {
                return null;
            }
            if (target == null) {
                target = candidate;
            } else if (!target.equals(candidate)) {
                return null;
            }
        }
        return target;
    }

    private static boolean schemaEncodeValueHasUnknownExtensionDispatch(
        Object value,
        BinarySchemaSpec schema
    ) {
        if (!(value instanceof java.util.Map)) {
            return false;
        }
        java.util.Map<Object, Object> fields = asMap(value);
        for (SchemaFieldSpec field : schema.fields) {
            if (field.dispatch == null || !field.dispatch.preservesUnknown) {
                continue;
            }
            Object tagValue = fields.get(field.dispatch.tagField);
            if (!(tagValue instanceof Long)
                || schemaDispatchCaseForTag(field.dispatch, tagValue) != null) {
                continue;
            }
            if (isAdtNamed(fields.get(field.name), "SchemaDispatchPayload::Unknown")) {
                return true;
            }
        }
        return false;
    }

    private static boolean schemaEncodeProjectionVariantMismatch(Object projected) {
        Object error = resultValue(projected);
        if (!isAdtNamed(error, "EncodeError::EncodeError")) {
            return false;
        }
        Adt encodeError = asAdt(error, "EncodeError::EncodeError");
        return asString(encodeError.payloads[0]).equals("codec.encode_mapping_mismatch")
            && asString(encodeError.payloads[2]).startsWith("expected constructor `");
    }

    private static Object schemaEncodeRecordWithMapping(
        Object value,
        BinarySchemaSpec schema,
        SchemaMappingSpec mapping
    ) {
        java.util.Map<Object, Object> targetFields = asMap(value);
        java.util.ArrayList<Object> entries =
            new java.util.ArrayList<Object>(mapping.fields.length * 2);
        for (SchemaMappingFieldSpec field : mapping.fields) {
            Object mapped = schemaEncodeMappingValues(
                targetFields.get(field.target),
                field.expr,
                schema.name,
                field.target
            );
            if (isErr(mapped)) {
                Object error = resultValue(mapped);
                if (isAdtNamed(error, "EncodeError::EncodeError")) {
                    return err(error);
                }
                return err(encodeError(
                    "codec.metadata",
                    schema.name,
                    "unsupported schema encode mapping expression for schema `" + schema.name + "`"
                ));
            }
            java.util.List<Object> mappingValues = asList(unwrapOk(mapped));
            for (Object entry : mappingValues) {
                SchemaEncodeMappingValue mappingValue = (SchemaEncodeMappingValue) entry;
                entries.add(mappingValue.source);
                entries.add(mappingValue.value);
            }
        }
        Object projected = record(entries.toArray(new Object[entries.size()]));
        return ok(schemaWrapSelectedExtensionDispatchPayloads(projected, schema, mapping));
    }

    private static Object schemaWrapSelectedExtensionDispatchPayloads(
        Object projected,
        BinarySchemaSpec schema,
        SchemaMappingSpec mapping
    ) {
        if (mapping.selectorField == null
            || !mapping.selectorOperator.equals("==")
            || !(projected instanceof java.util.Map)) {
            return projected;
        }
        java.util.Map<Object, Object> fields = asMap(projected);
        java.util.ArrayList<Object> entries = new java.util.ArrayList<Object>(fields.size() * 2);
        for (java.util.Map.Entry<Object, Object> entry : fields.entrySet()) {
            String fieldName = asString(entry.getKey());
            Object value = entry.getValue();
            SchemaFieldSpec schemaField = schemaFieldByName(schema, fieldName);
            if (schemaField != null
                && schemaField.dispatch != null
                && schemaField.dispatch.preservesUnknown
                && schemaField.dispatch.tagField.equals(mapping.selectorField)
                && schemaDispatchCaseForTag(schemaField.dispatch, fields.get(mapping.selectorField))
                    != null
                && !isAdtNamed(value, "SchemaDispatchPayload::Known")) {
                value = adt("SchemaDispatchPayload::Known", new Object[] { value });
            }
            entries.add(fieldName);
            entries.add(value);
        }
        return record(entries.toArray(new Object[entries.size()]));
    }

    private static SchemaFieldSpec schemaFieldByName(BinarySchemaSpec schema, String name) {
        for (SchemaFieldSpec field : schema.fields) {
            if (field.name.equals(name)) {
                return field;
            }
        }
        return null;
    }

    private static SchemaDispatchCaseSpec schemaDispatchCaseForTag(
        SchemaDispatchSpec dispatch,
        Object tag
    ) {
        if (!(tag instanceof Long)) {
            return null;
        }
        long tagValue = ((Long) tag).longValue();
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            if (dispatchCase.tag == tagValue) {
                return dispatchCase;
            }
        }
        return null;
    }

    private static Object schemaEncodeMappingValues(
        Object targetValue,
        Object expr,
        String schemaName,
        String targetField
    ) {
        if (expr instanceof String) {
            return ok(schemaEncodeMappingValueList(
                new SchemaEncodeMappingValue(asString(expr), targetValue)
            ));
        }
        java.util.List<Object> spec = asList(expr);
        if (spec.size() == 2 && asString(spec.get(0)).equals("field")) {
            return ok(schemaEncodeMappingValueList(
                new SchemaEncodeMappingValue(asString(spec.get(1)), targetValue)
            ));
        }
        if (spec.size() == 3 && asString(spec.get(0)).equals("field_access")) {
            String source = schemaMappingSelectedRecordDirectSource(
                spec.get(1),
                asString(spec.get(2))
            );
            if (source == null) {
                return err("unsupported schema encode mapping field access");
            }
            return ok(schemaEncodeMappingValueList(
                new SchemaEncodeMappingValue(source, targetValue)
            ));
        }
        if (spec.size() == 3 && asString(spec.get(0)).equals("record")) {
            return schemaEncodeMappingRecordValues(
                targetValue,
                asList(spec.get(1)),
                asList(spec.get(2)),
                schemaName,
                targetField,
                "mapped field `" + targetField + "`"
            );
        }
        if (spec.size() == 3 && asString(spec.get(0)).equals("constructor")) {
            String constructorName = asString(spec.get(1));
            java.util.List<Object> args = asList(spec.get(2));
            if (!isAdtNamed(targetValue, constructorName)) {
                return err(encodeError(
                    "codec.encode_mapping_mismatch",
                    schemaName + "." + targetField,
                    schemaEncodeConstructorMismatchReason(targetValue, constructorName)
                ));
            }
            Adt value = asAdt(targetValue, constructorName);
            if (value.payloads.length != args.size()) {
                return err("unsupported constructor mapping");
            }
            java.util.ArrayList<Object> values = new java.util.ArrayList<Object>(args.size());
            for (int index = 0; index < args.size(); index += 1) {
                Object mapped = schemaEncodeMappingArgValues(
                    value.payloads[index],
                    args.get(index),
                    schemaName,
                    targetField,
                    constructorName
                );
                if (isErr(mapped)) {
                    return mapped;
                }
                values.addAll(asList(unwrapOk(mapped)));
            }
            return ok(values);
        }
        if (spec.size() == 4 && asString(spec.get(0)).equals("converter")) {
            Object projected = call(spec.get(2), targetValue);
            Object roundTrip = call(spec.get(1), projected);
            if (!java.util.Objects.equals(roundTrip, targetValue)) {
                return err(encodeError(
                    "codec.encode_mapping_mismatch",
                    schemaName + "." + targetField,
                    "inverse converter projection does not round-trip through mapped converter"
                ));
            }
            return schemaEncodeMappingArgValues(
                projected,
                spec.get(3),
                schemaName,
                targetField,
                "converter projection"
            );
        }
        return err("unsupported schema encode mapping");
    }

    private static String schemaMappingSelectedRecordDirectSource(
        Object expr,
        String selectedField
    ) {
        java.util.List<Object> spec = asList(expr);
        if (spec.size() != 3 || !asString(spec.get(0)).equals("record")) {
            return null;
        }
        java.util.List<Object> names = asList(spec.get(1));
        java.util.List<Object> values = asList(spec.get(2));
        for (int index = 0; index < names.size(); index += 1) {
            if (!asString(names.get(index)).equals(selectedField)) {
                continue;
            }
            return schemaMappingDirectSource(values.get(index));
        }
        return null;
    }

    private static java.util.List<Object> schemaEncodeMappingValueList(
        SchemaEncodeMappingValue value
    ) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>(1);
        values.add(value);
        return values;
    }

    private static Object schemaEncodeMappingArgValues(
        Object payloadValue,
        Object expr,
        String schemaName,
        String targetField,
        String constructorName
    ) {
        String source = schemaMappingDirectSource(expr);
        if (source != null) {
            return ok(schemaEncodeMappingValueList(
                new SchemaEncodeMappingValue(source, payloadValue)
            ));
        }
        java.util.List<Object> spec = asList(expr);
        if (spec.size() == 3 && asString(spec.get(0)).equals("field_access")) {
            String selectedSource = schemaMappingSelectedRecordDirectSource(
                spec.get(1),
                asString(spec.get(2))
            );
            if (selectedSource == null) {
                return err("unsupported constructor mapping");
            }
            return ok(schemaEncodeMappingValueList(
                new SchemaEncodeMappingValue(selectedSource, payloadValue)
            ));
        }
        if (spec.size() == 3 && asString(spec.get(0)).equals("record")) {
            return schemaEncodeMappingRecordValues(
                payloadValue,
                asList(spec.get(1)),
                asList(spec.get(2)),
                schemaName,
                targetField,
                "constructor `" + constructorName + "` payload"
            );
        }
        if (spec.size() == 3 && asString(spec.get(0)).equals("constructor")) {
            String nestedConstructor = asString(spec.get(1));
            java.util.List<Object> args = asList(spec.get(2));
            if (!isAdtNamed(payloadValue, nestedConstructor)) {
                return err(encodeError(
                    "codec.encode_mapping_mismatch",
                    schemaName + "." + targetField,
                    schemaEncodeConstructorMismatchReason(payloadValue, nestedConstructor)
                ));
            }
            Adt value = asAdt(payloadValue, nestedConstructor);
            if (value.payloads.length != args.size()) {
                return err("unsupported constructor mapping");
            }
            java.util.ArrayList<Object> values = new java.util.ArrayList<Object>(args.size());
            for (int index = 0; index < args.size(); index += 1) {
                Object mapped = schemaEncodeMappingArgValues(
                    value.payloads[index],
                    args.get(index),
                    schemaName,
                    targetField,
                    nestedConstructor
                );
                if (isErr(mapped)) {
                    return mapped;
                }
                values.addAll(asList(unwrapOk(mapped)));
            }
            return ok(values);
        }
        return err("unsupported constructor mapping");
    }

    private static Object schemaEncodeMappingRecordValues(
        Object recordValue,
        java.util.List<Object> names,
        java.util.List<Object> values,
        String schemaName,
        String targetField,
        String recordLabel
    ) {
        if (!(recordValue instanceof java.util.Map)) {
            return err(encodeError(
                "codec.encode_mapping_mismatch",
                schemaName + "." + targetField,
                "expected record value for " + recordLabel
            ));
        }
        java.util.Map<Object, Object> recordFields = asMap(recordValue);
        java.util.ArrayList<Object> mapped = new java.util.ArrayList<Object>(values.size());
        for (int index = 0; index < names.size(); index += 1) {
            String fieldName = asString(names.get(index));
            if (!recordFields.containsKey(fieldName)) {
                return err(encodeError(
                    "codec.encode_mapping_mismatch",
                    schemaName + "." + targetField,
                    "expected record field `" + fieldName + "` in " + recordLabel
                ));
            }
            Object fieldMapped = schemaEncodeMappingArgValues(
                recordFields.get(fieldName),
                values.get(index),
                schemaName,
                targetField,
                recordLabel
            );
            if (isErr(fieldMapped)) {
                return fieldMapped;
            }
            mapped.addAll(asList(unwrapOk(fieldMapped)));
        }
        return ok(mapped);
    }

    private static String schemaEncodeConstructorMismatchReason(
        Object value,
        String expectedConstructor
    ) {
        if (value instanceof Adt) {
            return "expected constructor `" + expectedConstructor + "`, found `"
                + ((Adt) value).name + "`";
        }
        return "expected constructor `" + expectedConstructor + "`";
    }

    private static String schemaMappingDirectSource(Object expr) {
        if (expr instanceof String) {
            return asString(expr);
        }
        java.util.List<Object> spec = asList(expr);
        if (spec.size() == 2 && asString(spec.get(0)).equals("field")) {
            return asString(spec.get(1));
        }
        return null;
    }

    private static SchemaMappingSpec[] schemaMappingSpecs(
        java.util.List<Object> mappingTargetValues,
        java.util.List<Object> mappingSourceValues
    ) {
        boolean selectedMappings =
            !mappingTargetValues.isEmpty() && mappingTargetValues.get(0) instanceof java.util.List;
        if (selectedMappings) {
            SchemaMappingSpec[] mapping = new SchemaMappingSpec[mappingTargetValues.size()];
            for (int index = 0; index < mappingTargetValues.size(); index += 1) {
                java.util.List<Object> targetSpec = asList(mappingTargetValues.get(index));
                boolean selectorExpr = asString(targetSpec.get(0)).equals("select_expr");
                java.util.List<Object> targets =
                    asList(targetSpec.get(selectorExpr ? 2 : (targetSpec.size() > 4 ? 4 : 3)));
                java.util.List<Object> sources = asList(mappingSourceValues.get(index));
                SchemaMappingFieldSpec[] mappingFields =
                    new SchemaMappingFieldSpec[targets.size()];
                for (int fieldIndex = 0; fieldIndex < targets.size(); fieldIndex += 1) {
                    mappingFields[fieldIndex] = new SchemaMappingFieldSpec(
                        asString(targets.get(fieldIndex)),
                        sources.get(fieldIndex)
                    );
                }
                if (selectorExpr) {
                    mapping[index] = new SchemaMappingSpec(asString(targetSpec.get(1)), mappingFields);
                } else {
                    mapping[index] = new SchemaMappingSpec(
                        asString(targetSpec.get(1)),
                        targetSpec.size() > 4 ? asString(targetSpec.get(3)) : "==",
                        Long.valueOf(asLong(targetSpec.get(2))),
                        mappingFields
                    );
                }
            }
            return mapping;
        }
        SchemaMappingFieldSpec[] mappingFields =
            new SchemaMappingFieldSpec[mappingTargetValues.size()];
        for (int index = 0; index < mappingTargetValues.size(); index += 1) {
            mappingFields[index] = new SchemaMappingFieldSpec(
                asString(mappingTargetValues.get(index)),
                mappingSourceValues.get(index)
            );
        }
        return mappingFields.length == 0
            ? new SchemaMappingSpec[0]
            : new SchemaMappingSpec[] { new SchemaMappingSpec(mappingFields) };
    }

    private static Object declaredBinarySchemaSpec(
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object schemaValidation,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        return declaredBinarySchemaSpecWithMaxValues(
            schemaName,
            fieldNames,
            widths,
            maxValues,
            littleEndianValues,
            flag8Values,
            repeatCountFields,
            repeatWidths,
            repeatMaxValues,
            repeatLittleEndianValues,
            repeatByteViewLengthFields,
            repeatSchemaSpecs,
            reservedBitWidths,
            reservedValues,
            predicates,
            schemaValidation,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs,
            mappingTargets,
            mappingSources
        );
    }

    private static Object declaredBinarySchemaSpecWithMaxValues(
        Object schemaName,
        Object fieldNames,
        Object widths,
        Object maxValues,
        Object littleEndianValues,
        Object flag8Values,
        Object repeatCountFields,
        Object repeatWidths,
        Object repeatMaxValues,
        Object repeatLittleEndianValues,
        Object repeatByteViewLengthFields,
        Object repeatSchemaSpecs,
        Object reservedBitWidths,
        Object reservedValues,
        Object predicates,
        Object schemaValidation,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs,
        Object mappingTargets,
        Object mappingSources
    ) {
        String name = asString(schemaName);
        java.util.List<Object> fieldNameValues = asList(fieldNames);
        java.util.List<Object> widthValues = asList(widths);
        java.util.List<Object> maxValueValues =
            maxValues == null ? null : asList(maxValues);
        java.util.List<Object> littleEndianValueList =
            littleEndianValues == null ? null : asList(littleEndianValues);
        java.util.List<Object> flag8ValueList =
            flag8Values == null ? null : asList(flag8Values);
        java.util.List<Object> repeatCountFieldValues =
            repeatCountFields == null ? null : asList(repeatCountFields);
        java.util.List<Object> repeatWidthValues =
            repeatWidths == null ? null : asList(repeatWidths);
        java.util.List<Object> repeatMaxValueValues =
            repeatMaxValues == null ? null : asList(repeatMaxValues);
        java.util.List<Object> repeatLittleEndianValueList =
            repeatLittleEndianValues == null ? null : asList(repeatLittleEndianValues);
        java.util.List<Object> repeatByteViewLengthFieldValues =
            repeatByteViewLengthFields == null ? null : asList(repeatByteViewLengthFields);
        java.util.List<Object> repeatSchemaSpecValues =
            repeatSchemaSpecs == null ? null : asList(repeatSchemaSpecs);
        java.util.List<Object> reservedBitWidthValues = asList(reservedBitWidths);
        java.util.List<Object> reservedValueValues = asList(reservedValues);
        java.util.List<Object> predicateValues = asList(predicates);
        String schemaValidationText = asString(schemaValidation);
        java.util.List<Object> dispatchTagFieldValues = asList(dispatchTagFields);
        java.util.List<Object> dispatchLengthFieldValues = asList(dispatchLengthFields);
        java.util.List<Object> dispatchCaseTagValues = asList(dispatchCaseTags);
        java.util.List<Object> dispatchCaseWidthValues = asList(dispatchCaseWidths);
        java.util.List<Object> dispatchCaseLittleEndianValueList =
            dispatchCaseLittleEndianValues == null ? null : asList(dispatchCaseLittleEndianValues);
        java.util.List<Object> dispatchCaseSchemaSpecValues = asList(dispatchCaseSchemaSpecs);
        java.util.List<Object> mappingTargetValues = asList(mappingTargets);
        java.util.List<Object> mappingSourceValues = asList(mappingSources);
        if (fieldNameValues.size() != widthValues.size()
            || fieldNameValues.size() != reservedBitWidthValues.size()
            || fieldNameValues.size() != reservedValueValues.size()
            || fieldNameValues.size() != predicateValues.size()
            || fieldNameValues.size() != dispatchTagFieldValues.size()
            || fieldNameValues.size() != dispatchLengthFieldValues.size()
            || fieldNameValues.size() != dispatchCaseTagValues.size()
            || fieldNameValues.size() != dispatchCaseWidthValues.size()
            || (littleEndianValueList != null
                && fieldNameValues.size() != littleEndianValueList.size())
            || (flag8ValueList != null && fieldNameValues.size() != flag8ValueList.size())
            || (repeatCountFieldValues != null
                && fieldNameValues.size() != repeatCountFieldValues.size())
            || (repeatWidthValues != null && fieldNameValues.size() != repeatWidthValues.size())
            || (repeatMaxValueValues != null
                && fieldNameValues.size() != repeatMaxValueValues.size())
            || (repeatLittleEndianValueList != null
                && fieldNameValues.size() != repeatLittleEndianValueList.size())
            || (repeatByteViewLengthFieldValues != null
                && fieldNameValues.size() != repeatByteViewLengthFieldValues.size())
            || (repeatSchemaSpecValues != null
                && fieldNameValues.size() != repeatSchemaSpecValues.size())
            || (dispatchCaseLittleEndianValueList != null
                && fieldNameValues.size() != dispatchCaseLittleEndianValueList.size())
            || (maxValueValues != null && fieldNameValues.size() != maxValueValues.size())
            || fieldNameValues.size() != dispatchCaseSchemaSpecValues.size()) {
            return err("invalid schema decoder metadata for schema `" + name + "`");
        }
        if (mappingTargetValues.size() != mappingSourceValues.size()) {
            return err("invalid schema decoder mapping metadata for schema `" + name + "`");
        }
        SchemaPredicate validation = null;
        if (!schemaValidationText.isEmpty()) {
            try {
                validation = new SchemaPredicateParser(schemaValidationText).parse();
            } catch (IllegalStateException failure) {
                return err(failure.getMessage());
            }
        }
        SchemaFieldSpec[] fields = new SchemaFieldSpec[fieldNameValues.size()];
        for (int index = 0; index < fieldNameValues.size(); index += 1) {
            String fieldName = asString(fieldNameValues.get(index));
            long width = asLong(widthValues.get(index));
            long reservedBitWidth = asLong(reservedBitWidthValues.get(index));
            long reservedValue = asLong(reservedValueValues.get(index));
            String tagField = asString(dispatchTagFieldValues.get(index));
            String lengthField = asString(dispatchLengthFieldValues.get(index));
            String repeatCountField = repeatCountFieldValues == null
                ? ""
                : asString(repeatCountFieldValues.get(index));
            String repeatByteViewLengthField =
                repeatByteViewLengthFieldValues == null
                    ? ""
                    : asString(repeatByteViewLengthFieldValues.get(index));
            boolean littleEndian =
                littleEndianValueList != null && asBool(littleEndianValueList.get(index));
            String flagType = flag8ValueList == null
                ? ""
                : schemaFlagTypeFromMetadata(flag8ValueList.get(index));
            if (reservedBitWidth > 0L) {
                if (reservedBitWidth > Integer.MAX_VALUE
                    || !supportedReservedBits((int) reservedBitWidth, reservedValue)) {
                    return err(
                        "unsupported reserved bits encode field `"
                            + fieldName
                            + "` in schema `"
                            + name
                            + "`"
                    );
                }
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    (int) reservedBitWidth,
                    reservedValue
                );
                continue;
            }
            if (!repeatCountField.isEmpty()) {
                if (repeatWidthValues == null
                    || repeatMaxValueValues == null
                    || repeatLittleEndianValueList == null) {
                    return err("invalid repeat metadata for field `" + fieldName + "` in schema `" + name + "`");
                }
                long repeatWidth = asLong(repeatWidthValues.get(index));
                if (repeatWidth < 1L || repeatWidth > 8L) {
                    Object repeatPayloadSchema = repeatSchemaSpecValues == null
                        ? ""
                        : repeatSchemaSpecValues.get(index);
                    if (repeatByteViewLengthField.isEmpty()
                        && !(repeatPayloadSchema instanceof java.util.List<?>)) {
                        return err(
                            "unsupported repeat field width "
                                + repeatWidth
                                + " for field `"
                                + fieldName
                                + "` in schema `"
                                + name
                                + "`"
                        );
                    }
                }
                Object repeatPayloadSchema = repeatSchemaSpecValues == null
                    ? ""
                    : repeatSchemaSpecValues.get(index);
                BinarySchemaSpec payloadSchema = null;
                if (repeatPayloadSchema instanceof java.util.List<?>) {
                    Object parsedSchema = declaredBinarySchemaSpecFromMetadata(repeatPayloadSchema);
                    if (isErr(parsedSchema)) {
                        return parsedSchema;
                    }
                    payloadSchema = (BinarySchemaSpec) unwrapOk(parsedSchema);
                }
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    new SchemaRepeatSpec(
                        repeatCountField,
                        (int) repeatWidth,
                        asLong(repeatMaxValueValues.get(index)),
                        asBool(repeatLittleEndianValueList.get(index)),
                        repeatByteViewLengthField,
                        payloadSchema
                    )
                );
                continue;
            }
            if (!tagField.isEmpty()) {
                Object dispatch = declaredSchemaDispatch(
                    name,
                    fieldName,
                    tagField,
                    lengthField,
                    dispatchCaseTagValues.get(index),
                    dispatchCaseWidthValues.get(index),
                    dispatchCaseLittleEndianValueList == null
                        ? null
                        : dispatchCaseLittleEndianValueList.get(index),
                    dispatchCaseSchemaSpecValues.get(index)
                );
                if (isErr(dispatch)) {
                    return dispatch;
                }
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    0,
                    0L,
                    null,
                    (SchemaDispatchSpec) unwrapOk(dispatch)
                );
                continue;
            }
            if (!lengthField.isEmpty()) {
                fields[index] = new SchemaFieldSpec(fieldName, lengthField);
                continue;
            }
            if (width < 1L || width > 8L) {
                return err(
                    "unsupported schema field width "
                        + width
                        + " for field `"
                        + fieldName
                        + "` in schema `"
                        + name
                        + "`"
                );
            }
            String predicateText = asString(predicateValues.get(index));
            SchemaPredicate predicate = null;
            Long fixedValue = schemaFixedFieldValue(
                fieldName,
                predicateText,
                maxValueValues == null
                    ? unsignedMaxValue((int) width)
                    : asLong(maxValueValues.get(index))
            );
            if (!predicateText.isEmpty()) {
                if (fixedValue == null) {
                    try {
                        predicate = new SchemaPredicateParser(predicateText).parse();
                    } catch (IllegalStateException failure) {
                        return err(failure.getMessage());
                    }
                }
            }
            if (maxValueValues == null) {
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    (int) width,
                    unsignedMaxValue((int) width),
                    false,
                    false,
                    predicate,
                    fixedValue
                );
            } else {
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    (int) width,
                    asLong(maxValueValues.get(index)),
                    littleEndian,
                    flagType,
                    predicate,
                    fixedValue
                );
            }
        }
        SchemaMappingSpec[] mapping;
        boolean selectedMappings =
            !mappingTargetValues.isEmpty() && mappingTargetValues.get(0) instanceof java.util.List;
        if (selectedMappings) {
            mapping = new SchemaMappingSpec[mappingTargetValues.size()];
            for (int index = 0; index < mappingTargetValues.size(); index += 1) {
                java.util.List<Object> targetSpec = asList(mappingTargetValues.get(index));
                String selectorKind = asString(targetSpec.get(0));
                boolean selectorExpr = selectorKind.equals("select_expr");
                if (selectorExpr && targetSpec.size() != 3) {
                    return err("invalid schema decoder mapping selection metadata for schema `" + name + "`");
                }
                if (!selectorExpr
                    && ((targetSpec.size() != 4 && targetSpec.size() != 5)
                        || !selectorKind.equals("select"))) {
                    return err("invalid schema decoder mapping selection metadata for schema `" + name + "`");
                }
                String selectorOperator = targetSpec.size() > 4 ? asString(targetSpec.get(3)) : "==";
                java.util.List<Object> targets = asList(targetSpec.get(selectorExpr ? 2 : (targetSpec.size() > 4 ? 4 : 3)));
                java.util.List<Object> sources = asList(mappingSourceValues.get(index));
                if (targets.size() != sources.size()) {
                    return err("invalid schema decoder mapping metadata for schema `" + name + "`");
                }
                SchemaMappingFieldSpec[] mappingFields =
                    new SchemaMappingFieldSpec[targets.size()];
                for (int fieldIndex = 0; fieldIndex < targets.size(); fieldIndex += 1) {
                    mappingFields[fieldIndex] = new SchemaMappingFieldSpec(
                        asString(targets.get(fieldIndex)),
                        sources.get(fieldIndex)
                    );
                }
                if (selectorExpr) {
                    mapping[index] = new SchemaMappingSpec(asString(targetSpec.get(1)), mappingFields);
                } else {
                    mapping[index] = new SchemaMappingSpec(
                        asString(targetSpec.get(1)),
                        selectorOperator,
                        Long.valueOf(asLong(targetSpec.get(2))),
                        mappingFields
                    );
                }
            }
        } else {
            SchemaMappingFieldSpec[] mappingFields =
                new SchemaMappingFieldSpec[mappingTargetValues.size()];
            for (int index = 0; index < mappingTargetValues.size(); index += 1) {
                mappingFields[index] = new SchemaMappingFieldSpec(
                    asString(mappingTargetValues.get(index)),
                    mappingSourceValues.get(index)
                );
            }
            mapping = mappingFields.length == 0
                ? new SchemaMappingSpec[0]
                : new SchemaMappingSpec[] { new SchemaMappingSpec(mappingFields) };
        }
        return ok(new BinarySchemaSpec(name, fields, validation, mapping));
    }

    private static Object schemaMappingExprValue(
        java.util.LinkedHashMap<String, Object> decoded,
        Object expr
    ) {
        if (expr instanceof String) {
            return decoded.get(asString(expr));
        }
        java.util.List<Object> spec = asList(expr);
        String tag = asString(spec.get(0));
        if (tag.equals("field")) {
            return decoded.get(asString(spec.get(1)));
        }
        if (tag.equals("literal")) {
            return Long.valueOf(asLong(spec.get(1)));
        }
        if (tag.equals("field_access")) {
            return recordField(schemaMappingExprValue(decoded, spec.get(1)), asString(spec.get(2)));
        }
        if (tag.equals("record")) {
            java.util.List<Object> names = asList(spec.get(1));
            java.util.List<Object> values = asList(spec.get(2));
            java.util.ArrayList<Object> entries = new java.util.ArrayList<Object>(names.size() * 2);
            for (int index = 0; index < names.size(); index += 1) {
                entries.add(asString(names.get(index)));
                entries.add(schemaMappingExprValue(decoded, values.get(index)));
            }
            return record(entries.toArray(new Object[entries.size()]));
        }
        if (tag.equals("constructor")) {
            String name = asString(spec.get(1));
            java.util.List<Object> args = asList(spec.get(2));
            Object[] payloads = new Object[args.size()];
            for (int index = 0; index < args.size(); index += 1) {
                payloads[index] = schemaMappingExprValue(decoded, args.get(index));
            }
            return adt(name, payloads);
        }
        if (tag.equals("converter")) {
            if (spec.size() == 4) {
                return call(spec.get(1), schemaMappingExprValue(decoded, spec.get(3)));
            }
            java.util.List<Object> args = asList(spec.get(2));
            Object[] values = new Object[args.size()];
            for (int index = 0; index < args.size(); index += 1) {
                values[index] = schemaMappingExprValue(decoded, args.get(index));
            }
            return call(spec.get(1), values);
        }
        if (tag.equals("binary")) {
            Object left = schemaMappingExprValue(decoded, spec.get(2));
            Object right = schemaMappingExprValue(decoded, spec.get(3));
            String op = asString(spec.get(1));
            if (op.equals("+")) {
                return add(left, right);
            }
            if (op.equals("-")) {
                return subtract(left, right);
            }
            if (op.equals("*")) {
                return multiply(left, right);
            }
            if (op.equals("/")) {
                return divide(left, right);
            }
            if (op.equals("==")) {
                return equal(left, right);
            }
            if (op.equals("!=")) {
                return notEqual(left, right);
            }
            throw new IllegalStateException("unsupported schema mapping binary operator");
        }
        throw new IllegalStateException("unsupported schema mapping expression metadata");
    }

    private static Object declaredSchemaDispatch(
        String schemaName,
        String fieldName,
        String tagField,
        String lengthField,
        Object caseTags,
        Object caseWidths,
        Object caseLittleEndianValues,
        Object caseSchemaSpecs
    ) {
        java.util.List<Object> tagValues = asList(caseTags);
        java.util.List<Object> widthValues = asList(caseWidths);
        java.util.List<Object> littleEndianValues =
            caseLittleEndianValues == null ? null : asList(caseLittleEndianValues);
        java.util.List<Object> schemaSpecValues = asList(caseSchemaSpecs);
        boolean preservesUnknown = !lengthField.isEmpty();
        if (lengthField.startsWith("closed:")) {
            preservesUnknown = false;
            lengthField = lengthField.substring("closed:".length());
        }
        if (tagValues.isEmpty()
            || tagValues.size() != widthValues.size()
            || (littleEndianValues != null && tagValues.size() != littleEndianValues.size())
            || tagValues.size() != schemaSpecValues.size()) {
            return err(
                "invalid dispatch metadata for field `"
                    + fieldName
                    + "` in schema `"
                    + schemaName
                    + "`"
            );
        }
        SchemaDispatchCaseSpec[] cases = new SchemaDispatchCaseSpec[tagValues.size()];
        for (int index = 0; index < tagValues.size(); index += 1) {
            long width = asLong(widthValues.get(index));
            Object schemaSpec = schemaSpecValues.get(index);
            if (schemaSpec instanceof java.util.List<?>) {
                Object payloadSchema = declaredBinarySchemaSpecFromMetadata(schemaSpec);
                if (isErr(payloadSchema)) {
                    return payloadSchema;
                }
                cases[index] = new SchemaDispatchCaseSpec(
                    asLong(tagValues.get(index)),
                    (BinarySchemaSpec) unwrapOk(payloadSchema)
                );
                continue;
            }
            if (schemaSpec instanceof String && asString(schemaSpec).equals(schemaName)) {
                cases[index] = new SchemaDispatchCaseSpec(
                    asLong(tagValues.get(index)),
                    schemaName
                );
                continue;
            }
            if (width < 1L || width > 8L) {
                return err(
                    "unsupported dispatch field width "
                        + width
                        + " for field `"
                        + fieldName
                        + "` in schema `"
                        + schemaName
                        + "`"
                );
            }
            boolean littleEndian =
                littleEndianValues != null && asBool(littleEndianValues.get(index));
            cases[index] = new SchemaDispatchCaseSpec(
                asLong(tagValues.get(index)),
                (int) width,
                littleEndian
            );
        }
        return ok(new SchemaDispatchSpec(tagField, lengthField, preservesUnknown, cases));
    }

    private static Object declaredBinarySchemaSpecFromMetadata(Object metadata) {
        java.util.List<Object> values = asList(metadata);
        if (values.size() != 24) {
            return err("invalid nested schema decoder metadata");
        }
        return declaredBinarySchemaSpecWithMaxValues(
            values.get(0),
            values.get(1),
            values.get(2),
            values.get(3),
            values.get(4),
            values.get(5),
            values.get(6),
            values.get(7),
            values.get(8),
            values.get(9),
            values.get(10),
            values.get(11),
            values.get(12),
            values.get(13),
            values.get(14),
            values.get(15),
            values.get(16),
            values.get(17),
            values.get(18),
            values.get(19),
            values.get(20),
            values.get(21),
            values.get(22),
            values.get(23)
        );
    }

    private static Object unsupportedSchemaPredicate(BinarySchemaSpec schema) {
        java.util.LinkedHashMap<String, Integer> fieldIndexes =
            new java.util.LinkedHashMap<String, Integer>();
        for (int index = 0; index < schema.fields.length; index += 1) {
            fieldIndexes.put(schema.fields[index].name, Integer.valueOf(index));
        }
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
            if (field.predicate == null) {
                continue;
            }
            for (String ref : field.predicate.refs()) {
                Integer refIndex = fieldIndexes.get(ref);
                if (refIndex == null) {
                    return err(
                        "unsupported schema predicate reference `"
                            + ref
                            + "` in schema `"
                            + schema.name
                            + "`: not a schema field"
                    );
                }
                if (refIndex.intValue() > index) {
                    return err(
                        "unsupported schema predicate reference `"
                            + ref
                            + "` in schema `"
                            + schema.name
                            + "`: field is decoded later"
                    );
                }
            }
        }
        if (schema.validation != null) {
            for (String ref : schema.validation.refs()) {
                if (!fieldIndexes.containsKey(ref)) {
                    return err(
                        "unsupported schema validation reference `"
                            + ref
                            + "` in schema `"
                            + schema.name
                            + "`: not a schema field"
                    );
                }
            }
        }
        return null;
    }

    private static String byteViewPreviewTrace(Object view, int maxPreviewBytes) {
        java.util.List<Object> bytes = byteViewBytes(view);
        long offset = byteViewStorageOffset(view);
        long count = byteViewCountLong(view);
        Object boundsError = byteViewBoundsError(bytes, offset, count);
        if (boundsError != null) {
            return ":0:0:false";
        }
        long previewCount = Math.min(count, Math.max(0, maxPreviewBytes));
        long end = offset + previewCount;
        StringBuilder out = new StringBuilder((int) (previewCount * 2));
        char[] digits = "0123456789abcdef".toCharArray();
        for (long index = offset; index < end; index += 1) {
            int value = (int) byteValueLong(bytes.get((int) index));
            out.append(digits[value >>> 4]);
            out.append(digits[value & 0x0f]);
        }
        return hex(out.toString())
            + ":"
            + previewCount
            + ":"
            + count
            + ":"
            + Boolean.toString(count > previewCount);
    }

    private static String byteRangePreviewTrace(
        java.util.List<Object> bytes,
        long offset,
        long count,
        int maxPreviewBytes
    ) {
        if (offset < 0 || count < 0 || offset > bytes.size()) {
            return ":0:0:false";
        }
        long boundedCount = Math.min(count, bytes.size() - offset);
        long previewCount = Math.min(boundedCount, Math.max(0, maxPreviewBytes));
        long end = offset + previewCount;
        StringBuilder out = new StringBuilder((int) (previewCount * 2));
        char[] digits = "0123456789abcdef".toCharArray();
        for (long index = offset; index < end; index += 1) {
            int value = (int) byteValueLong(bytes.get((int) index));
            out.append(digits[value >>> 4]);
            out.append(digits[value & 0x0f]);
        }
        return hex(out.toString())
            + ":"
            + previewCount
            + ":"
            + boundedCount
            + ":"
            + Boolean.toString(boundedCount > previewCount);
    }

    private static Object byteViewBoundsError(Object view) {
        return byteViewBoundsError(
            byteViewBytes(view),
            byteViewStorageOffset(view),
            byteViewCountLong(view)
        );
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
        return byteWriteUnsigned(value, 2, 0xffffL, "byte_write_u16_be", false);
    }

    public static Object byteWriteU24Be(Object value) {
        return byteWriteUnsigned(value, 3, 0xffffffL, "byte_write_u24_be", false);
    }

    public static Object byteWriteU31Be(Object value) {
        return byteWriteUnsigned(value, 4, 0x7fffffffL, "byte_write_u31_be", false);
    }

    public static Object byteWriteU32Be(Object value) {
        return byteWriteUnsigned(value, 4, 0xffffffffL, "byte_write_u32_be", false);
    }

    public static Object byteWriteU40Be(Object value) {
        return byteWriteUnsigned(value, 5, 0xffffffffffL, "byte_write_u40_be", false);
    }

    public static Object byteWriteU48Be(Object value) {
        return byteWriteUnsigned(value, 6, 0xffffffffffffL, "byte_write_u48_be", false);
    }

    public static Object byteWriteU64Be(Object value) {
        return byteWriteUnsigned(value, 8, Long.MAX_VALUE, "byte_write_u64_be", false);
    }

    public static Object byteWriteU16Le(Object value) {
        return byteWriteUnsigned(value, 2, 0xffffL, "byte_write_u16_le", true);
    }

    public static Object byteWriteU24Le(Object value) {
        return byteWriteUnsigned(value, 3, 0xffffffL, "byte_write_u24_le", true);
    }

    public static Object byteWriteU31Le(Object value) {
        return byteWriteUnsigned(value, 4, 0x7fffffffL, "byte_write_u31_le", true);
    }

    public static Object byteWriteU32Le(Object value) {
        return byteWriteUnsigned(value, 4, 0xffffffffL, "byte_write_u32_le", true);
    }

    public static Object byteWriteU40Le(Object value) {
        return byteWriteUnsigned(value, 5, 0xffffffffffL, "byte_write_u40_le", true);
    }

    public static Object byteWriteU48Le(Object value) {
        return byteWriteUnsigned(value, 6, 0xffffffffffffL, "byte_write_u48_le", true);
    }

    public static Object byteWriteU64Le(Object value) {
        return byteWriteUnsigned(value, 8, Long.MAX_VALUE, "byte_write_u64_le", true);
    }

    private static Object byteWriteUnsigned(
        Object value,
        int width,
        long maxValue,
        String name,
        boolean littleEndian
    ) {
        long unsigned = asLong(value);
        if (unsigned < 0 || unsigned > maxValue) {
            String message = name + " value must be between 0 and " + maxValue;
            registerByteWriteValueDiagnostic(
                message,
                name,
                unsigned,
                0,
                maxValue,
                width,
                littleEndian
            );
            return err(message);
        }
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>(width);
        for (int index = width - 1; index >= 0; index -= 1) {
            int shiftIndex = littleEndian ? width - 1 - index : index;
            bytes.add(byteObject((unsigned >> (shiftIndex * 8)) & 0xffL));
        }
        return ok(byteChunkObject(bytes));
    }

    private static Object byteWriteUnsignedBe(Object value, int width, long maxValue, String name) {
        return byteWriteUnsigned(value, width, maxValue, name, false);
    }

    private static Object schemaWriteDispatchPayload(
        java.util.Map<Object, Object> fields,
        BinarySchemaSpec schema,
        SchemaDispatchSpec dispatch,
        String fieldName
    ) {
        if (dispatch.preservesUnknown) {
            return schemaWriteExtensionDispatchPayload(fields, dispatch, schema, fieldName);
        }
        long tagValue = asLong(fields.get(dispatch.tagField));
        String fieldPath = schema.name + "." + fieldName;
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            if (dispatchCase.tag != tagValue) {
                continue;
            }
            BinarySchemaSpec payloadSchema = dispatchCase.payloadSchema(schema);
            Object encoded;
            if (payloadSchema != null) {
                encoded = byteEncodeBinarySchema(fields.get(fieldName), payloadSchema);
            } else {
                encoded = schemaWriteUnsigned(
                    fields.get(fieldName),
                    dispatchCase.width,
                    dispatchCase.maxValue,
                    dispatchCase.littleEndian,
                    schema.name,
                    fieldName
                );
            }
            if (isErr(encoded)) {
                return encoded;
            }
            if (!dispatch.lengthField.isEmpty()) {
                long suppliedLength = asLong(fields.get(dispatch.lengthField));
                return schemaDispatchLengthCheckedChunk(
                    unwrapOk(encoded),
                    suppliedLength,
                    dispatch.lengthField,
                    fieldPath
                );
            }
            return encoded;
        }
        return err(encodeError(
            "codec.dispatch_unknown_tag",
            fieldPath,
            "tag field `"
                + dispatch.tagField
                + "` value "
                + tagValue
                + " must be one of "
                + schemaDispatchExpectedTags(dispatch)
        ));
    }

    private static Object schemaWriteExtensionDispatchPayload(
        java.util.Map<Object, Object> fields,
        SchemaDispatchSpec dispatch,
        BinarySchemaSpec schema,
        String fieldName
    ) {
        String fieldPath = schema.name + "." + fieldName;
        long tagValue = asLong(fields.get(dispatch.tagField));
        long suppliedLength = asLong(fields.get(dispatch.lengthField));
        Object payload = fields.get(fieldName);
        SchemaDispatchCaseSpec selectedCase = null;
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            if (dispatchCase.tag == tagValue) {
                selectedCase = dispatchCase;
                break;
            }
        }
        if (selectedCase != null) {
            if (!isAdtNamed(payload, "SchemaDispatchPayload::Known")) {
                return err(encodeError(
                    "codec.dispatch_mismatch",
                    fieldPath,
                    "tag field `" + dispatch.tagField + "` value " + tagValue
                        + " selects a known payload case"
                ));
            }
            BinarySchemaSpec payloadSchema = selectedCase.payloadSchema(schema);
            if (payloadSchema != null) {
                Object encoded = byteEncodeBinarySchema(
                    asAdt(payload, "SchemaDispatchPayload::Known").payloads[0],
                    payloadSchema
                );
                if (isErr(encoded)) {
                    return encoded;
                }
                return schemaDispatchLengthCheckedChunk(
                    unwrapOk(encoded),
                    suppliedLength,
                    dispatch.lengthField,
                    fieldPath
                );
            }
            Object encoded = schemaWriteUnsigned(
                asAdt(payload, "SchemaDispatchPayload::Known").payloads[0],
                selectedCase.width,
                selectedCase.maxValue,
                selectedCase.littleEndian,
                schema.name,
                fieldName
            );
            if (isErr(encoded)) {
                return encoded;
            }
            return schemaDispatchLengthCheckedChunk(
                unwrapOk(encoded),
                suppliedLength,
                dispatch.lengthField,
                fieldPath
            );
        }
        if (!isAdtNamed(payload, "SchemaDispatchPayload::Unknown")) {
            return err(encodeError(
                "codec.dispatch_mismatch",
                fieldPath,
                "tag field `" + dispatch.tagField + "` value " + tagValue
                    + " requires an unknown payload variant"
            ));
        }
        Adt unknown = asAdt(payload, "SchemaDispatchPayload::Unknown");
        long payloadTag = asLong(unknown.payloads[0]);
        if (payloadTag != tagValue) {
            return err(encodeError(
                "codec.dispatch_mismatch",
                fieldPath,
                "tag field `" + dispatch.tagField + "` value " + tagValue
                    + " does not match unknown payload tag " + payloadTag
            ));
        }
        Object chunk = byteViewToChunk(unknown.payloads[1]);
        return schemaDispatchLengthCheckedChunk(
            chunk,
            suppliedLength,
            dispatch.lengthField,
            fieldPath
        );
    }

    private static Object schemaWriteRepeatedField(
        java.util.Map<Object, Object> fields,
        SchemaFieldSpec field,
        String schemaName
    ) {
        Object expectedCountValue = schemaRepeatCountExpressionValue(
            fields,
            field.repeat.countField,
            schemaName
        );
        if (isErr(expectedCountValue)) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                asString(resultValue(expectedCountValue))
            ));
        }
        long expectedCount = asLong(unwrapOk(expectedCountValue));
        if (field.repeat.isByteViewPayload()) {
            return schemaWriteRepeatedByteViews(fields, field, schemaName, expectedCount);
        }
        Object items = fields.get(field.name);
        ListValue current = asListValue(items);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        long actualCount = 0L;
        while (current.cons) {
            if (actualCount >= expectedCount) {
                return err(encodeError(
                    "codec.encode_value_unrepresentable",
                    schemaName + "." + field.name,
                    "list length exceeds "
                        + schemaCountDescription(field.repeat.countField)
                        + " value "
                        + expectedCount
                ));
            }
            Object encoded = field.repeat.payloadSchema == null
                ? schemaWriteUnsigned(
                    current.head,
                    field.repeat.width,
                    field.repeat.maxValue,
                    field.repeat.littleEndian,
                    schemaName,
                    field.name
                )
                : byteEncodeBinarySchema(current.head, field.repeat.payloadSchema);
            if (isErr(encoded)) {
                if (field.repeat.payloadSchema != null) {
                    return rebaseEncodeErrorPath(
                        encoded,
                        schemaName + "." + field.name + "." + actualCount
                    );
                }
                return encoded;
            }
            bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
            actualCount += 1L;
            current = asListValue(current.tail);
        }
        if (actualCount != expectedCount) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                "list length "
                    + actualCount
                    + " does not match "
                    + schemaCountDescription(field.repeat.countField)
                    + " value "
                    + expectedCount
            ));
        }
        return ok(byteChunkObject(bytes));
    }

    private static Object schemaWriteRepeatedByteViews(
        java.util.Map<Object, Object> fields,
        SchemaFieldSpec field,
        String schemaName,
        long expectedCount
    ) {
        Object lengthValue = schemaLengthExpressionValue(
            fields,
            field.repeat.byteViewLengthField,
            "repeat payload",
            schemaName
        );
        if (isErr(lengthValue)) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                asString(resultValue(lengthValue))
            ));
        }
        long suppliedLength = asLong(unwrapOk(lengthValue));
        Object items = fields.get(field.name);
        ListValue current = asListValue(items);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        long actualCount = 0L;
        while (current.cons) {
            if (actualCount >= expectedCount) {
                return err(encodeError(
                    "codec.encode_value_unrepresentable",
                    schemaName + "." + field.name,
                    "list length exceeds "
                        + schemaCountDescription(field.repeat.countField)
                        + " value "
                        + expectedCount
                ));
            }
            long actualLength = byteViewCountLong(current.head);
            if (actualLength != suppliedLength) {
                Object error = encodeError(
                    "codec.encode_value_unrepresentable",
                    schemaName + "." + field.name + "." + actualCount,
                    "byte view count " + actualLength + " does not match "
                        + schemaLengthDescription(field.repeat.byteViewLengthField)
                        + " value " + suppliedLength
                );
                registerSchemaByteViewEncodeLengthMismatchDiagnostic(
                    error,
                    current.head,
                    suppliedLength,
                    actualLength,
                    field.repeat.byteViewLengthField
                );
                return err(error);
            }
            bytes.addAll(byteChunkBytes(byteViewToChunk(current.head)));
            actualCount += 1L;
            current = asListValue(current.tail);
        }
        if (actualCount != expectedCount) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                "list length "
                    + actualCount
                    + " does not match "
                    + schemaCountDescription(field.repeat.countField)
                    + " value "
                    + expectedCount
            ));
        }
        return ok(byteChunkObject(bytes));
    }

    private static String schemaCountDescription(String expression) {
        SchemaFieldExpression parsed = schemaFieldExpression(expression, false);
        if (parsed != null && parsed.isField()) {
            return "count field `" + expression + "`";
        }
        return "count expression `" + expression + "`";
    }

    private static Object rebaseEncodeErrorPath(Object encoded, String prefix) {
        Adt error = asAdt(resultValue(encoded), "EncodeError::EncodeError");
        return err(encodeError(
            asString(error.payloads[0]),
            prefix + "." + asString(error.payloads[1]),
            asString(error.payloads[2])
        ));
    }

    private static boolean isAdtNamed(Object value, String name) {
        return value instanceof Adt && ((Adt) value).name.equals(name);
    }

    private static Object schemaDispatchLengthCheckedChunk(
        Object chunk,
        long suppliedLength,
        String lengthField,
        String fieldPath
    ) {
        long actualLength = byteChunkBytes(chunk).size();
        if (actualLength != suppliedLength) {
            return err(encodeError(
                "codec.dispatch_length_mismatch",
                fieldPath,
                "length field `" + lengthField + "` value " + suppliedLength
                    + " does not match encoded payload byte count " + actualLength
            ));
        }
        return ok(chunk);
    }

    private static Object schemaLengthExpressionValue(
        java.util.Map<?, ?> fields,
        String expression,
        String role,
        String schemaName
    ) {
        return schemaLengthExpressionValue(
            fields,
            expression,
            role,
            schemaName,
            null,
            null,
            null
        );
    }

    private static Object schemaLengthExpressionValue(
        java.util.Map<?, ?> fields,
        String expression,
        String role,
        String schemaName,
        Long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        String nearbyContext
    ) {
        return schemaFieldExpressionValue(
            fields,
            expression,
            role + " length expression",
            role + " length field",
            role + " length operand",
            schemaName,
            true,
            byteOffset,
            fieldPath,
            nearbyContext
        );
    }

    private static Object schemaRepeatCountExpressionValue(
        java.util.Map<?, ?> fields,
        String expression,
        String schemaName
    ) {
        return schemaRepeatCountExpressionValue(
            fields,
            expression,
            schemaName,
            null,
            null,
            null
        );
    }

    private static Object schemaRepeatCountExpressionValue(
        java.util.Map<?, ?> fields,
        String expression,
        String schemaName,
        Long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        String nearbyContext
    ) {
        return schemaFieldExpressionValue(
            fields,
            expression,
            "repeat count expression",
            "repeat count field",
            "repeat count operand",
            schemaName,
            true,
            byteOffset,
            fieldPath,
            nearbyContext
        );
    }

    private static Object schemaFieldExpressionValue(
        java.util.Map<?, ?> fields,
        String expression,
        String expressionRole,
        String fieldRole,
        String operandRole,
        String schemaName,
        boolean allowProduct,
        Long byteOffset,
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
        String nearbyContext
    ) {
        SchemaFieldExpression parsed = schemaFieldExpression(expression, allowProduct);
        if (parsed == null) {
            return err(
                "unsupported "
                    + expressionRole
                    + " `"
                    + expression
                    + "` in schema `"
                    + schemaName
                    + "`"
            );
        }
        Object left = fields.get(parsed.left);
        if (left == null) {
            return err(
                "missing "
                    + fieldRole
                    + " `"
                    + parsed.left
                    + "` in schema `"
                    + schemaName
                    + "`"
            );
        }
        if (parsed.isField()) {
            return ok(Long.valueOf(asLong(left)));
        }
        Object right = fields.get(parsed.right);
        if (right == null) {
            return err(
                "missing "
                    + operandRole
                    + " `"
                    + parsed.right
                    + "` in expression `"
                    + expression
                    + "` in schema `"
                    + schemaName
                    + "`"
            );
        }
        long leftValue = asLong(left);
        long rightValue = asLong(right);
        if (parsed.operator == '/' && rightValue == 0L) {
            String message = "schema length division by zero";
            if (byteOffset != null) {
                message = message + " at byte offset " + byteOffset.longValue();
            }
            if (byteOffset != null && fieldPath != null && nearbyContext != null) {
                registerSchemaLengthDivisionByZeroDiagnostic(
                    message,
                    byteOffset.longValue(),
                    fieldPath,
                    expression,
                    parsed.right,
                    nearbyContext
                );
            }
            return err(message);
        }
        long result = parsed.operator == '+'
            ? leftValue + rightValue
            : parsed.operator == '-'
                ? leftValue - rightValue
                : parsed.operator == '*'
                    ? leftValue * rightValue
                    : leftValue / rightValue;
        return ok(Long.valueOf(result));
    }

    private static SchemaFieldExpression schemaFieldExpression(
        String expression,
        boolean allowProduct
    ) {
        String trimmed = expression.trim();
        if (isSchemaLengthExpressionIdentifier(trimmed)) {
            return new SchemaFieldExpression(trimmed);
        }
        int plus = trimmed.indexOf('+');
        int minus = trimmed.indexOf('-');
        int product = trimmed.indexOf('*');
        int quotient = trimmed.indexOf('/');
        if (product >= 0 && !allowProduct) {
            return null;
        }
        int operatorCount = (plus >= 0 ? 1 : 0)
            + (minus >= 0 ? 1 : 0)
            + (product >= 0 ? 1 : 0)
            + (quotient >= 0 ? 1 : 0);
        if (operatorCount > 1) {
            return null;
        }
        if (plus >= 0) {
            return schemaBinaryFieldExpression(trimmed, plus, '+');
        }
        if (minus >= 0) {
            return schemaBinaryFieldExpression(trimmed, minus, '-');
        }
        if (product >= 0) {
            return schemaBinaryFieldExpression(trimmed, product, '*');
        }
        if (quotient >= 0) {
            return schemaBinaryFieldExpression(trimmed, quotient, '/');
        }
        return null;
    }

    private static SchemaFieldExpression schemaBinaryFieldExpression(
        String expression,
        int operatorIndex,
        char operator
    ) {
        if (expression.indexOf(operator, operatorIndex + 1) >= 0) {
            return null;
        }
        String left = expression.substring(0, operatorIndex).trim();
        String right = expression.substring(operatorIndex + 1).trim();
        if (!isSchemaLengthExpressionIdentifier(left)
            || !isSchemaLengthExpressionIdentifier(right)) {
            return null;
        }
        return new SchemaFieldExpression(left, operator, right);
    }

    private static final class SchemaFieldExpression {
        final String left;
        final char operator;
        final String right;

        SchemaFieldExpression(String field) {
            this.left = field;
            this.operator = '\0';
            this.right = "";
        }

        SchemaFieldExpression(String left, char operator, String right) {
            this.left = left;
            this.operator = operator;
            this.right = right;
        }

        boolean isField() {
            return right.isEmpty();
        }
    }

    private static boolean isSchemaLengthExpressionIdentifier(String text) {
        if (text.isEmpty()) {
            return false;
        }
        char first = text.charAt(0);
        if (!((first >= 'A' && first <= 'Z') || (first >= 'a' && first <= 'z') || first == '_')) {
            return false;
        }
        for (int index = 1; index < text.length(); index += 1) {
            char ch = text.charAt(index);
            if (!((ch >= 'A' && ch <= 'Z')
                || (ch >= 'a' && ch <= 'z')
                || (ch >= '0' && ch <= '9')
                || ch == '_')) {
                return false;
            }
        }
        return true;
    }

    private static Object schemaWriteLengthBoundedView(
        java.util.Map<Object, Object> fields,
        SchemaFieldSpec field,
        String schemaName
    ) {
        Object lengthValue = schemaLengthExpressionValue(
            fields,
            field.lengthField,
            "payload",
            schemaName
        );
        if (isErr(lengthValue)) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                asString(resultValue(lengthValue))
            ));
        }
        long suppliedLength = asLong(unwrapOk(lengthValue));
        Object view = fields.get(field.name);
        long actualLength = byteViewCountLong(view);
        if (actualLength != suppliedLength) {
            Object error = encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + field.name,
                "byte view count " + actualLength + " does not match "
                    + schemaLengthDescription(field.lengthField)
                    + " value " + suppliedLength
            );
            registerSchemaByteViewEncodeLengthMismatchDiagnostic(
                error,
                view,
                suppliedLength,
                actualLength,
                field.lengthField
            );
            return err(error);
        }
        return ok(byteViewToChunk(view));
    }

    private static String schemaLengthDescription(String expression) {
        SchemaFieldExpression parsed = schemaFieldExpression(expression, true);
        if (parsed != null && parsed.isField()) {
            return "length field `" + expression + "`";
        }
        return "length expression `" + expression + "`";
    }

    private static Object schemaWriteUnsigned(
        Object value,
        int width,
        long maxValue,
        boolean littleEndian,
        String schemaName,
        String fieldName
    ) {
        long unsigned = asLong(value);
        if (unsigned < 0 || unsigned > maxValue) {
            return err(encodeError(
                "codec.encode_value_unrepresentable",
                schemaName + "." + fieldName,
                "value must be between 0 and " + schemaUnsignedMaxValueText(width, maxValue)
            ));
        }
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>(width);
        for (int index = width - 1; index >= 0; index -= 1) {
            int shiftIndex = littleEndian ? width - 1 - index : index;
            bytes.add(byteObject((unsigned >> (shiftIndex * 8)) & 0xffL));
        }
        return ok(byteChunkObject(bytes));
    }

    private static String schemaUnsignedMaxValueText(int width, long maxValue) {
        if (width == 8 && maxValue == Long.MAX_VALUE) {
            return "18446744073709551615";
        }
        return String.valueOf(maxValue);
    }

    private static Object schemaFieldEncodeValue(Object value, SchemaFieldSpec field) {
        return field.isFlagBitset() ? Long.valueOf(schemaFlagBits(value, field.flagType)) : value;
    }

    private static Long schemaEncodedIntValue(Object value, SchemaFieldSpec field) {
        if (field.isFlagBitset()) {
            return null;
        }
        return value instanceof Long ? (Long) value : null;
    }

    private static String schemaFlagTypeFromMetadata(Object value) {
        if (value instanceof java.lang.Boolean) {
            return asBool(value) ? "Flag8" : "";
        }
        return asString(value);
    }

    private static Object schemaFlagObject(SchemaFieldSpec field, long bits) {
        return flagObject(field.flagType, bits);
    }

    private static Object flag8Object(long bits) {
        return flagObject("Flag8", bits);
    }

    private static long flag8Bits(Object value) {
        return schemaFlagBits(value, "Flag8");
    }

    private static Object flag16beObject(long bits) {
        return flagObject("Flag16be", bits);
    }

    private static long flag16beBits(Object value) {
        return schemaFlagBits(value, "Flag16be");
    }

    private static Object flag16leObject(long bits) {
        return flagObject("Flag16le", bits);
    }

    private static long flag16leBits(Object value) {
        return schemaFlagBits(value, "Flag16le");
    }

    private static Object flag24beObject(long bits) {
        return flagObject("Flag24be", bits);
    }

    private static long flag24beBits(Object value) {
        return schemaFlagBits(value, "Flag24be");
    }

    private static Object flag24leObject(long bits) {
        return flagObject("Flag24le", bits);
    }

    private static long flag24leBits(Object value) {
        return schemaFlagBits(value, "Flag24le");
    }

    private static Object flag32beObject(long bits) {
        return flagObject("Flag32be", bits);
    }

    private static long flag32beBits(Object value) {
        return schemaFlagBits(value, "Flag32be");
    }

    private static Object flag32leObject(long bits) {
        return flagObject("Flag32le", bits);
    }

    private static long flag32leBits(Object value) {
        return schemaFlagBits(value, "Flag32le");
    }

    private static Object flag40beObject(long bits) {
        return flagObject("Flag40be", bits);
    }

    private static long flag40beBits(Object value) {
        return schemaFlagBits(value, "Flag40be");
    }

    private static Object flag40leObject(long bits) {
        return flagObject("Flag40le", bits);
    }

    private static long flag40leBits(Object value) {
        return schemaFlagBits(value, "Flag40le");
    }

    private static Object flag48beObject(long bits) {
        return flagObject("Flag48be", bits);
    }

    private static long flag48beBits(Object value) {
        return schemaFlagBits(value, "Flag48be");
    }

    private static Object flag48leObject(long bits) {
        return flagObject("Flag48le", bits);
    }

    private static long flag48leBits(Object value) {
        return schemaFlagBits(value, "Flag48le");
    }

    private static Object flag56beObject(long bits) {
        return flagObject("Flag56be", bits);
    }

    private static long flag56beBits(Object value) {
        return schemaFlagBits(value, "Flag56be");
    }

    private static Object flag56leObject(long bits) {
        return flagObject("Flag56le", bits);
    }

    private static long flag56leBits(Object value) {
        return schemaFlagBits(value, "Flag56le");
    }

    private static Object flag64beObject(long bits) {
        return flagObject("Flag64be", bits);
    }

    private static long flag64beBits(Object value) {
        return schemaFlagBits(value, "Flag64be");
    }

    private static Object flag64leObject(long bits) {
        return flagObject("Flag64le", bits);
    }

    private static long flag64leBits(Object value) {
        return schemaFlagBits(value, "Flag64le");
    }

    private static Object flagObject(String flagType, long bits) {
        return adt(flagType + "::" + flagType, new Object[] { Long.valueOf(bits) });
    }

    private static long schemaFlagBits(Object value, String flagType) {
        if (isAdtNamed(value, flagType)) {
            return asLong(asAdt(value, flagType).payloads[0]);
        }
        return asLong(asAdt(value, flagType + "::" + flagType).payloads[0]);
    }

    private static Object encodeError(String id, String fieldPath, String reason) {
        return adt("EncodeError::EncodeError", new Object[] { id, fieldPath, reason });
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
