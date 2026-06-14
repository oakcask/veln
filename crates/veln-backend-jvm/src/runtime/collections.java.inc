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
        final boolean flag8;
        final SchemaPredicate predicate;
        final SchemaDispatchSpec dispatch;
        final SchemaRepeatSpec repeat;
        final String lengthField;
        final int reservedBitWidth;
        final long reservedValue;
        final Long fixedValue;

        SchemaFieldSpec(String name, int width, long maxValue, SchemaPredicate predicate) {
            this(name, width, maxValue, false, false, predicate, null, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            SchemaPredicate predicate
        ) {
            this(name, width, maxValue, littleEndian, false, predicate, null, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            SchemaPredicate predicate,
            Long fixedValue
        ) {
            this(name, width, maxValue, littleEndian, false, predicate, null, null, "", 0, 0L, fixedValue);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            boolean flag8,
            SchemaPredicate predicate
        ) {
            this(name, width, maxValue, littleEndian, flag8, predicate, null, null, "", 0, 0L, null);
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
            this(name, width, maxValue, littleEndian, flag8, predicate, null, null, "", 0, 0L, fixedValue);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            SchemaPredicate predicate,
            SchemaDispatchSpec dispatch
        ) {
            this(name, width, maxValue, false, false, predicate, dispatch, null, "", 0, 0L, null);
        }

        SchemaFieldSpec(String name, String lengthField) {
            this(name, 0, 0L, false, false, null, null, null, lengthField, 0, 0L, null);
        }

        SchemaFieldSpec(String name, SchemaRepeatSpec repeat) {
            this(name, 0, 0L, false, false, null, null, repeat, "", 0, 0L, null);
        }

        SchemaFieldSpec(String name, int reservedBitWidth, long reservedValue) {
            this(name, 0, 0L, false, false, null, null, null, "", reservedBitWidth, reservedValue, null);
        }

        SchemaFieldSpec(
            String name,
            int width,
            long maxValue,
            boolean littleEndian,
            boolean flag8,
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
            this.flag8 = flag8;
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

        boolean isFlag8Bitset() {
            return dispatch == null
                && repeat == null
                && lengthField.isEmpty()
                && reservedBitWidth == 0
                && width == 1
                && maxValue == 0xffL
                && flag8;
        }
    }

    private static final class SchemaRepeatSpec {
        final String countField;
        final int width;
        final long maxValue;
        final boolean littleEndian;

        SchemaRepeatSpec(String countField, int width, long maxValue, boolean littleEndian) {
            this.countField = countField;
            this.width = width;
            this.maxValue = maxValue;
            this.littleEndian = littleEndian;
        }
    }

    private static int reservedByteWidth(int bitWidth) {
        return bitWidth > 0 && bitWidth % 8 == 0 ? bitWidth / 8 : 0;
    }

    private static long reservedMaxValue(int bitWidth) {
        return bitWidth == 32 ? 0xffffffffL : (1L << bitWidth) - 1L;
    }

    private static boolean supportedReservedBits(int bitWidth, long reservedValue) {
        if (bitWidth <= 0 || bitWidth > 32) {
            return false;
        }
        if (bitWidth < 8) {
            return reservedValue >= 0L && reservedValue <= reservedMaxValue(bitWidth);
        }
        if (bitWidth % 8 != 0) {
            return false;
        }
        return reservedValue >= 0L && reservedValue <= reservedMaxValue(bitWidth);
    }

    private static int packedVisibleBitWidth(SchemaFieldSpec reserved, SchemaFieldSpec visible) {
        if (!reserved.isReserved()
            || reserved.reservedBitWidth < 1
            || reserved.reservedBitWidth > 7
            || visible.isReserved()
            || visible.dispatch != null
            || visible.repeat != null
            || visible.isLengthBoundedView()
            || visible.width != 1
            || visible.littleEndian) {
            return 0;
        }
        int visibleBitWidth = 8 - reserved.reservedBitWidth;
        return visible.maxValue == reservedMaxValue(visibleBitWidth) ? visibleBitWidth : 0;
    }

    private static final class SchemaDispatchSpec {
        final String tagField;
        final String lengthField;
        final SchemaDispatchCaseSpec[] cases;

        SchemaDispatchSpec(
            String tagField,
            String lengthField,
            SchemaDispatchCaseSpec[] cases
        ) {
            this.tagField = tagField;
            this.lengthField = lengthField;
            this.cases = cases;
        }
    }

    private static final class SchemaDispatchCaseSpec {
        final long tag;
        final int width;
        final long maxValue;
        final boolean littleEndian;
        final BinarySchemaSpec payloadSchema;

        SchemaDispatchCaseSpec(long tag, int width) {
            this(tag, width, false);
        }

        SchemaDispatchCaseSpec(long tag, int width, boolean littleEndian) {
            this.tag = tag;
            this.width = width;
            this.littleEndian = littleEndian;
            this.payloadSchema = null;
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
        }

        long width() {
            if (payloadSchema != null) {
                return schemaTotalWidth(payloadSchema);
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
        final Long selectorValue;
        final SchemaMappingFieldSpec[] fields;

        SchemaMappingSpec(SchemaMappingFieldSpec[] fields) {
            this("", null, fields);
        }

        SchemaMappingSpec(
            String selectorField,
            Long selectorValue,
            SchemaMappingFieldSpec[] fields
        ) {
            this.selectorField = selectorField;
            this.selectorValue = selectorValue;
            this.fields = fields;
        }

        boolean matches(java.util.Map<String, Object> decoded) {
            if (selectorValue == null) {
                return true;
            }
            Object value = decoded.get(selectorField);
            return value instanceof Long
                && ((Long) value).longValue() == selectorValue.longValue();
        }
    }

    private static final class BinarySchemaSpec {
        final String name;
        final SchemaFieldSpec[] fields;
        final SchemaMappingSpec[] mapping;

        BinarySchemaSpec(String name, SchemaFieldSpec[] fields) {
            this(name, fields, new SchemaMappingSpec[0]);
        }

        BinarySchemaSpec(
            String name,
            SchemaFieldSpec[] fields,
            SchemaMappingSpec[] mapping
        ) {
            this.name = name;
            this.fields = fields;
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
            return err("byte view range exceeds view length");
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
        Object reservedBitWidths,
        Object reservedValues,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs
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
        java.util.List<Object> reservedBitWidthValues = asList(reservedBitWidths);
        java.util.List<Object> reservedValueValues = asList(reservedValues);
        java.util.List<Object> dispatchTagFieldValues = asList(dispatchTagFields);
        java.util.List<Object> dispatchLengthFieldValues = asList(dispatchLengthFields);
        java.util.List<Object> dispatchCaseTagValues = asList(dispatchCaseTags);
        java.util.List<Object> dispatchCaseWidthValues = asList(dispatchCaseWidths);
        java.util.List<Object> dispatchCaseLittleEndianValueList =
            asList(dispatchCaseLittleEndianValues);
        java.util.List<Object> dispatchCaseSchemaSpecValues = asList(dispatchCaseSchemaSpecs);
        if (fieldNameValues.size() != widthValues.size()
            || fieldNameValues.size() != maxValueValues.size()
            || fieldNameValues.size() != littleEndianValueList.size()
            || fieldNameValues.size() != flag8ValueList.size()
            || fieldNameValues.size() != repeatCountFieldValues.size()
            || fieldNameValues.size() != repeatWidthValues.size()
            || fieldNameValues.size() != repeatMaxValueValues.size()
            || fieldNameValues.size() != repeatLittleEndianValueList.size()
            || fieldNameValues.size() != reservedBitWidthValues.size()
            || fieldNameValues.size() != reservedValueValues.size()
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

        java.util.Map<Object, Object> fields = asMap(value);
        SchemaFieldSpec[] schemaFields = new SchemaFieldSpec[fieldNameValues.size()];
        for (int index = 0; index < fieldNameValues.size(); index += 1) {
            String fieldName = asString(fieldNameValues.get(index));
            int width = (int) asLong(widthValues.get(index));
            long maxValue = asLong(maxValueValues.get(index));
            int reservedBitWidth = (int) asLong(reservedBitWidthValues.get(index));
            long reservedValue = asLong(reservedValueValues.get(index));
            String tagField = asString(dispatchTagFieldValues.get(index));
            String lengthField = asString(dispatchLengthFieldValues.get(index));
            boolean littleEndian = asBool(littleEndianValueList.get(index));
            boolean flag8 = asBool(flag8ValueList.get(index));
            String repeatCountField = asString(repeatCountFieldValues.get(index));
            if (reservedBitWidth > 0) {
                schemaFields[index] = new SchemaFieldSpec(
                    fieldName,
                    reservedBitWidth,
                    reservedValue
                );
                continue;
            }
            if (!repeatCountField.isEmpty()) {
                schemaFields[index] = new SchemaFieldSpec(
                    fieldName,
                    new SchemaRepeatSpec(
                        repeatCountField,
                        (int) asLong(repeatWidthValues.get(index)),
                        asLong(repeatMaxValueValues.get(index)),
                        asBool(repeatLittleEndianValueList.get(index))
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
                flag8,
                null
            );
        }
        return byteEncodeBinarySchema(value, new BinarySchemaSpec(name, schemaFields));
    }

    private static Object byteEncodeBinarySchema(Object value, BinarySchemaSpec schema) {
        java.util.Map<Object, Object> fields = asMap(value);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        for (int index = 0; index < schema.fields.length; index += 1) {
            SchemaFieldSpec field = schema.fields[index];
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
                        bytes.add(byteObject(combined & 0xffL));
                        index += 1;
                        continue;
                    }
                }
                continue;
            }
            if (field.dispatch != null) {
                Object encoded = schemaWriteDispatchPayload(
                    fields,
                    field.dispatch,
                    schema.name,
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
            if (field.width < 1 || field.width > 4) {
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
            bytes.addAll(byteChunkBytes(unwrapOk(encoded)));
        }
        return ok(byteChunkObject(bytes));
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
        Object reservedBitWidths,
        Object reservedValues,
        Object dispatchTagFields,
        Object dispatchLengthFields,
        Object dispatchCaseTags,
        Object dispatchCaseWidths,
        Object dispatchCaseLittleEndianValues,
        Object dispatchCaseSchemaSpecs
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
            reservedBitWidths,
            reservedValues,
            dispatchTagFields,
            dispatchLengthFields,
            dispatchCaseTags,
            dispatchCaseWidths,
            dispatchCaseLittleEndianValues,
            dispatchCaseSchemaSpecs
        );
        if (isErr(encoded)) {
            return adt("EncodeStep::Invalid", new Object[] { resultValue(encoded) });
        }
        Object chunks = listCons(unwrapOk(encoded), listNil());
        return adt("EncodeStep::Encoded", new Object[] { chunks });
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
        Object ruleProvenance
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
            provenance
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
        Object ruleProvenance
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
            provenance
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

    public static Object http2ProtocolUnexpectedSettingsAck(
        Object offset,
        Object activeState,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        String state = asString(activeState);
        String provenance = asString(ruleProvenance);
        String message = "HTTP/2 unexpected SETTINGS ACK at byte offset " + byteOffset;
        registerHttp2ProtocolUnexpectedSettingsAckDiagnostic(
            message,
            byteOffset,
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
        Object ruleProvenance
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
            provenance
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
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long observed = asLong(observedSize);
        long allowed = asLong(allowedSize);
        long kind = asLong(frameKind);
        long stream = asLong(streamId);
        String limitProvenance = asString(receiveLimitProvenance);
        String rule = asString(ruleProvenance);
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
            rule
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
        Object activeState,
        Object receiveLimitProvenance,
        Object ruleProvenance
    ) {
        long byteOffset = asLong(offset);
        long stream = asLong(streamId);
        long attempted = asLong(attemptedCount);
        long allowed = asLong(allowedCount);
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
        Object peerLimitProvenance
    ) {
        long byteOffset = asLong(offset);
        long identifier = asLong(settingIdentifier);
        String name = asString(settingName);
        long observed = asLong(observedValue);
        long acceptedMin = asLong(acceptedMinValue);
        long acceptedMax = asLong(acceptedMaxValue);
        String provenance = asString(peerLimitProvenance);
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
            provenance
        );
        return err(message);
    }

    public static Object hpackFixtureUnsupportedHeaderBlock(
        Object offset,
        Object observedSize,
        Object observedFirstByte,
        Object expectedFixture,
        Object codecModule
    ) {
        long byteOffset = asLong(offset);
        long size = asLong(observedSize);
        long firstByte = asLong(observedFirstByte);
        String expected = asString(expectedFixture);
        String module = asString(codecModule);
        String message = "HPACK fixture unsupported header block at byte offset "
            + byteOffset;
        registerHpackFixtureUnsupportedHeaderBlockDiagnostic(
            message,
            byteOffset,
            size,
            firstByte,
            expected,
            module
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
        if (value > maxValue) {
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
        if (value > maxValue) {
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
        int requiredBytes = byteWidth > 0 ? byteWidth : 1;
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
            long mask = reservedMaxValue(bitWidth);
            actualValue =
                (byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset))
                    >> (8 - bitWidth))
                    & mask;
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
        if (available < 1L) {
            long byteOffset = diagnosticBaseOffset + count;
            String message = "truncated schema field `" + fieldName + "` at byte offset " + byteOffset;
            registerSchemaTruncatedFieldDiagnostic(
                message,
                byteOffset,
                schemaName,
                fieldName,
                1L,
                available,
                byteViewPreviewTrace(view, 8)
            );
            return err(message);
        }
        long value =
            byteValueLong(bytes.get((int) storageBaseOffset + relativeOffset))
                & reservedMaxValue(bitWidth);
        return ok(Long.valueOf(value));
    }

    private static Object byteDecodeBinarySchema(Object view, BinarySchemaSpec schema) {
        return byteDecodeBinarySchemaWithBase(view, schema, byteViewOffset(view));
    }

    private static Object byteDecodeBinarySchemaWithBase(
        Object view,
        BinarySchemaSpec schema,
        long diagnosticBaseOffset
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
        for (SchemaFieldSpec field : schema.fields) {
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
                Long payloadLength = decoded.get(field.lengthField);
                if (payloadLength == null) {
                    return err(
                        "missing payload length field `"
                            + field.lengthField
                            + "` in schema `"
                            + schema.name
                            + "`"
                    );
                }
                long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                if (payloadLength.longValue() > availablePayload) {
                    long byteOffset = diagnosticBaseOffset + relativeOffset + availablePayload;
                    String message = "payload length out of bounds at byte offset " + byteOffset;
                    registerSchemaLengthOutOfBoundsDiagnostic(
                        message,
                        byteOffset,
                        schema.name,
                        field.name,
                        payloadLength.longValue(),
                        availablePayload,
                        byteViewPreviewTrace(view, 8)
                    );
                    return err(message);
                }
                Object payloadView = byteViewDerivedObject(view, relativeOffset, payloadLength.longValue());
                values.put(field.name, payloadView);
                relativeOffset += payloadLength.longValue();
                continue;
            }
            if (field.isRepeat()) {
                pendingPackedReserved = null;
                Long repeatCount = decoded.get(field.repeat.countField);
                if (repeatCount == null) {
                    return err(
                        "missing repeat count field `"
                            + field.repeat.countField
                            + "` in schema `"
                            + schema.name
                            + "`"
                    );
                }
                java.util.ArrayList<Object> elements = new java.util.ArrayList<Object>();
                for (long elementIndex = 0L; elementIndex < repeatCount.longValue(); elementIndex += 1L) {
                    long elementOffset = relativeOffset + elementIndex * field.repeat.width;
                    Object read = schemaReadUnsignedWithBasePath(
                        view,
                        diagnosticBaseOffset,
                        (int) elementOffset,
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
                }
                Object repeated = listNil();
                for (int elementIndex = elements.size() - 1; elementIndex >= 0; elementIndex -= 1) {
                    repeated = listCons(elements.get(elementIndex), repeated);
                }
                decoded.put(field.name, Long.valueOf(0L));
                values.put(field.name, repeated);
                relativeOffset += repeatCount.longValue() * field.repeat.width;
                continue;
            }
            if (field.dispatch != null) {
                pendingPackedReserved = null;
                SchemaDispatchCaseSpec dispatchCase = schemaDispatchCase(field, decoded);
                if (dispatchCase == null) {
                    Long tagValue = decoded.get(field.dispatch.tagField);
                    if (!field.dispatch.lengthField.isEmpty() && tagValue != null) {
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
                            registerSchemaLengthOutOfBoundsDiagnostic(
                                message,
                                byteOffset,
                                schema.name,
                                field.name,
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
                long payloadWidth = dispatchCase.width();
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
                        registerSchemaLengthOutOfBoundsDiagnostic(
                            message,
                            byteOffset,
                            schema.name,
                            field.name,
                            payloadLength.longValue(),
                            availablePayload,
                            byteViewPreviewTrace(view, 8)
                        );
                        return err(message);
                    }
                    payloadWidth = payloadLength.longValue();
                }
                Object dispatchValue;
                if (dispatchCase.payloadSchema != null) {
                    long availablePayload = Math.max(0L, byteViewCountLong(view) - relativeOffset);
                    long payloadViewCount = Math.min(payloadWidth, availablePayload);
                    Object payloadView = byteViewDerivedObject(view, relativeOffset, payloadViewCount);
                    Object nestedRead = byteDecodeBinarySchemaWithBase(
                        payloadView,
                        dispatchCase.payloadSchema,
                        diagnosticBaseOffset + relativeOffset
                    );
                    if (isErr(nestedRead)) {
                        return nestedRead;
                    }
                    dispatchValue = unwrapOk(nestedRead);
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
                if (field.dispatch.lengthField.isEmpty()) {
                    values.put(field.name, dispatchValue);
                } else {
                    values.put(
                        field.name,
                        adt(
                            "SchemaDispatchPayload::Known",
                            new Object[] { dispatchValue }
                        )
                    );
                }
                relativeOffset += payloadWidth;
                continue;
            }

            int packedVisibleBitWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedVisibleBitWidth(pendingPackedReserved, field);
            Object read =
                packedVisibleBitWidth > 0
                    ? schemaReadPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        packedVisibleBitWidth,
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
                return read;
            }
            long fieldValue = asLong(unwrapOk(read));
            if (field.fixedValue != null && fieldValue != field.fixedValue.longValue()) {
                long byteOffset = diagnosticBaseOffset + relativeOffset;
                String message = "fixed field mismatch at byte offset " + byteOffset;
                registerSchemaFixedFieldMismatchDiagnostic(
                    message,
                    byteOffset,
                    schema.name,
                    field.name,
                    field.fixedValue.longValue(),
                    fieldValue,
                    byteViewPreviewTrace(view, 8)
                );
                return err(message);
            }
            decoded.put(field.name, Long.valueOf(fieldValue));
            values.put(
                field.name,
                field.isFlag8Bitset()
                    ? flag8Object(fieldValue)
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

            relativeOffset += packedVisibleBitWidth > 0 ? 1 : field.width;
            pendingPackedReserved = null;
        }

        Object decodedRecord = schemaDecodedRecord(schema, values);
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
            return invalidDecodeStep("schema.decode_failed", baseOffset, schema.name);
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
        for (SchemaFieldSpec field : schema.fields) {
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
                Long payloadLength = decoded.get(field.lengthField);
                if (payloadLength == null) {
                    return relativeOffset;
                }
                relativeOffset += payloadLength.longValue();
                continue;
            }
            if (field.isRepeat()) {
                pendingPackedReserved = null;
                Long repeatCount = decoded.get(field.repeat.countField);
                if (repeatCount == null) {
                    return relativeOffset;
                }
                relativeOffset += repeatCount.longValue() * field.repeat.width;
                decoded.put(field.name, Long.valueOf(0L));
                continue;
            }
            if (field.dispatch != null) {
                pendingPackedReserved = null;
                SchemaDispatchCaseSpec dispatchCase = schemaDispatchCase(field, decoded);
                if (dispatchCase == null) {
                    Long payloadLength = decoded.get(field.dispatch.lengthField);
                    if (payloadLength == null) {
                        return relativeOffset;
                    }
                    relativeOffset += payloadLength.longValue();
                    continue;
                }
                long payloadWidth = dispatchCase.width();
                if (!field.dispatch.lengthField.isEmpty()) {
                    Long payloadLength = decoded.get(field.dispatch.lengthField);
                    if (payloadLength == null) {
                        return relativeOffset;
                    }
                    payloadWidth = payloadLength.longValue();
                }
                decoded.put(field.name, Long.valueOf(0L));
                relativeOffset += payloadWidth;
                continue;
            }
            int packedVisibleBitWidth =
                pendingPackedReserved == null
                    ? 0
                    : packedVisibleBitWidth(pendingPackedReserved, field);
            Object read =
                packedVisibleBitWidth > 0
                    ? schemaReadPackedUnsignedWithBase(
                        view,
                        diagnosticBaseOffset,
                        (int) relativeOffset,
                        packedVisibleBitWidth,
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
            relativeOffset += packedVisibleBitWidth > 0 ? 1 : field.width;
            pendingPackedReserved = null;
        }
        return relativeOffset;
    }

    private static long schemaTotalWidth(BinarySchemaSpec schema) {
        long total = 0L;
        for (SchemaFieldSpec field : schema.fields) {
            if (field.isReserved()) {
                total += reservedByteWidth(field.reservedBitWidth);
                continue;
            } else if (field.isLengthBoundedView()) {
                continue;
            } else if (field.isRepeat()) {
                continue;
            } else if (field.dispatch == null) {
                total += field.width;
            } else {
                total += schemaDispatchMaxWidth(field.dispatch);
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

    private static long schemaDispatchMaxWidth(SchemaDispatchSpec dispatch) {
        long width = 0L;
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            width = Math.max(width, dispatchCase.width());
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

    private static Object schemaDecodedRecord(
        BinarySchemaSpec schema,
        java.util.LinkedHashMap<String, Object> decoded
    ) {
        if (schema.mapping.length > 0) {
            SchemaMappingSpec mapping = selectedSchemaMapping(schema, decoded);
            if (mapping == null) {
                return err(
                    "schema mapping selection did not match any mapping in schema `"
                        + schema.name
                        + "`"
                );
            }
            java.util.ArrayList<Object> entries =
                new java.util.ArrayList<Object>(mapping.fields.length * 2);
            for (SchemaMappingFieldSpec field : mapping.fields) {
                entries.add(field.target);
                entries.add(schemaMappingExprValue(decoded, field.expr));
            }
            return record(entries.toArray(new Object[entries.size()]));
        }

        java.util.ArrayList<Object> entries = new java.util.ArrayList<Object>(decoded.size() * 2);
        for (java.util.Map.Entry<String, Object> entry : decoded.entrySet()) {
            entries.add(entry.getKey());
            entries.add(entry.getValue());
        }
        return record(entries.toArray(new Object[entries.size()]));
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
        java.util.List<Object> reservedBitWidthValues = asList(reservedBitWidths);
        java.util.List<Object> reservedValueValues = asList(reservedValues);
        java.util.List<Object> predicateValues = asList(predicates);
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
            || (dispatchCaseLittleEndianValueList != null
                && fieldNameValues.size() != dispatchCaseLittleEndianValueList.size())
            || (maxValueValues != null && fieldNameValues.size() != maxValueValues.size())
            || fieldNameValues.size() != dispatchCaseSchemaSpecValues.size()) {
            return err("invalid schema decoder metadata for schema `" + name + "`");
        }
        if (mappingTargetValues.size() != mappingSourceValues.size()) {
            return err("invalid schema decoder mapping metadata for schema `" + name + "`");
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
            boolean littleEndian =
                littleEndianValueList != null && asBool(littleEndianValueList.get(index));
            boolean flag8 = flag8ValueList != null && asBool(flag8ValueList.get(index));
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
                if (repeatWidth < 1L || repeatWidth > 4L) {
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
                fields[index] = new SchemaFieldSpec(
                    fieldName,
                    new SchemaRepeatSpec(
                        repeatCountField,
                        (int) repeatWidth,
                        asLong(repeatMaxValueValues.get(index)),
                        asBool(repeatLittleEndianValueList.get(index))
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
            if (width < 1L || width > 4L) {
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
                    flag8,
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
                if (targetSpec.size() != 4 || !asString(targetSpec.get(0)).equals("select")) {
                    return err("invalid schema decoder mapping selection metadata for schema `" + name + "`");
                }
                java.util.List<Object> targets = asList(targetSpec.get(3));
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
                mapping[index] = new SchemaMappingSpec(
                    asString(targetSpec.get(1)),
                    Long.valueOf(asLong(targetSpec.get(2))),
                    mappingFields
                );
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
        return ok(new BinarySchemaSpec(name, fields, mapping));
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
            return call(spec.get(1), schemaMappingExprValue(decoded, spec.get(2)));
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
            if (width < 1L || width > 4L) {
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
        return ok(new SchemaDispatchSpec(tagField, lengthField, cases));
    }

    private static Object declaredBinarySchemaSpecFromMetadata(Object metadata) {
        java.util.List<Object> values = asList(metadata);
        if (values.size() != 21) {
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
            values.get(20)
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

    private static Object byteWriteUnsigned(
        Object value,
        int width,
        long maxValue,
        String name,
        boolean littleEndian
    ) {
        long unsigned = asLong(value);
        if (unsigned < 0 || unsigned > maxValue) {
            return err(name + " value must be between 0 and " + maxValue);
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
        SchemaDispatchSpec dispatch,
        String schemaName,
        String fieldName
    ) {
        if (!dispatch.lengthField.isEmpty()) {
            return schemaWriteExtensionDispatchPayload(fields, dispatch, schemaName, fieldName);
        }
        long tagValue = asLong(fields.get(dispatch.tagField));
        for (SchemaDispatchCaseSpec dispatchCase : dispatch.cases) {
            if (dispatchCase.tag != tagValue) {
                continue;
            }
            if (dispatchCase.payloadSchema != null) {
                return byteEncodeBinarySchema(fields.get(fieldName), dispatchCase.payloadSchema);
            }
            return schemaWriteUnsigned(
                fields.get(fieldName),
                dispatchCase.width,
                dispatchCase.maxValue,
                dispatchCase.littleEndian,
                schemaName,
                fieldName
            );
        }
        return err(encodeError(
            "codec.dispatch_unknown_tag",
            schemaName + "." + fieldName,
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
        String schemaName,
        String fieldName
    ) {
        String fieldPath = schemaName + "." + fieldName;
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
            if (selectedCase.payloadSchema != null) {
                Object encoded = byteEncodeBinarySchema(
                    asAdt(payload, "SchemaDispatchPayload::Known").payloads[0],
                    selectedCase.payloadSchema
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
                schemaName,
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
        long expectedCount = asLong(fields.get(field.repeat.countField));
        Object items = fields.get(field.name);
        ListValue current = asListValue(items);
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>();
        long actualCount = 0L;
        while (current.cons) {
            if (actualCount >= expectedCount) {
                return err(encodeError(
                    "codec.encode_value_unrepresentable",
                    schemaName + "." + field.name,
                    "list length exceeds count field `" + field.repeat.countField + "` value " + expectedCount
                ));
            }
            Object encoded = schemaWriteUnsigned(
                current.head,
                field.repeat.width,
                field.repeat.maxValue,
                field.repeat.littleEndian,
                schemaName,
                field.name
            );
            if (isErr(encoded)) {
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
                "list length " + actualCount + " does not match count field `"
                    + field.repeat.countField + "` value " + expectedCount
            ));
        }
        return ok(byteChunkObject(bytes));
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
                "value must be between 0 and " + maxValue
            ));
        }
        java.util.ArrayList<Object> bytes = new java.util.ArrayList<Object>(width);
        for (int index = width - 1; index >= 0; index -= 1) {
            int shiftIndex = littleEndian ? width - 1 - index : index;
            bytes.add(byteObject((unsigned >> (shiftIndex * 8)) & 0xffL));
        }
        return ok(byteChunkObject(bytes));
    }

    private static Object schemaFieldEncodeValue(Object value, SchemaFieldSpec field) {
        return field.isFlag8Bitset() ? Long.valueOf(flag8Bits(value)) : value;
    }

    private static Object flag8Object(long bits) {
        return adt("Flag8::Flag8", new Object[] { Long.valueOf(bits) });
    }

    private static long flag8Bits(Object value) {
        return asLong(asAdt(value, "Flag8::Flag8").payloads[0]);
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
