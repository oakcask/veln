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
