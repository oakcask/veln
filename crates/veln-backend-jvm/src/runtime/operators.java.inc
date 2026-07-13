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
    public static Object not(Object value) {
        return Boolean.valueOf(!asBool(value));
    }

    public static Object bitwiseNot(Object value) {
        return Long.valueOf(~asLong(value));
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
        return Boolean.valueOf(java.util.Objects.equals(
            diagnosticResultValue(left),
            diagnosticResultValue(right)
        ));
    }

    public static Object notEqual(Object left, Object right) {
        return Boolean.valueOf(!java.util.Objects.equals(
            diagnosticResultValue(left),
            diagnosticResultValue(right)
        ));
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

    public static Object bitwiseAnd(Object left, Object right) {
        return Long.valueOf(asLong(left) & asLong(right));
    }

    public static Object bitwiseOr(Object left, Object right) {
        return Long.valueOf(asLong(left) | asLong(right));
    }

    public static Object bitwiseXor(Object left, Object right) {
        return Long.valueOf(asLong(left) ^ asLong(right));
    }

    private static int checkedShiftCount(Object value, String operator) {
        long count = asLong(value);
        if (count < 0 || count > 63) {
            throw new RuntimeFailure(
                "invalid shift count " + count + " for operator `" + operator
                    + "`; expected a value between 0 and 63"
            );
        }
        return (int) count;
    }

    public static Object shiftLeft(Object left, Object right) {
        return Long.valueOf(asLong(left) << checkedShiftCount(right, "<<"));
    }

    public static Object shiftRight(Object left, Object right) {
        return Long.valueOf(asLong(left) >> checkedShiftCount(right, ">>"));
    }

    public static Object shiftRightLogical(Object left, Object right) {
        return Long.valueOf(asLong(left) >>> checkedShiftCount(right, ">>>"));
    }

    public static Object pipe(Object left, Object right) {
        return right;
    }
