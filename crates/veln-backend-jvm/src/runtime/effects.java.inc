    private static java.util.List<Object> processArgs =
        java.util.Collections.unmodifiableList(new java.util.ArrayList<Object>());

    public static void setProcessArgs(String[] args) {
        java.util.ArrayList<Object> values = new java.util.ArrayList<Object>();
        for (String arg : args) {
            values.add(arg);
        }
        processArgs = freezeList(values);
    }

    public static Object fsReadToString(Object path) {
        try {
            byte[] bytes = java.nio.file.Files.readAllBytes(asPath(path));
            return ok(new String(bytes, java.nio.charset.StandardCharsets.UTF_8));
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsWriteString(Object path, Object text) {
        try {
            java.nio.file.Files.write(
                asPath(path),
                asString(text).getBytes(java.nio.charset.StandardCharsets.UTF_8)
            );
            return ok(UNIT);
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsExists(Object path) {
        try {
            return ok(Boolean.valueOf(java.nio.file.Files.exists(asPath(path))));
        } catch (RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object fsReadDir(Object path) {
        try (java.util.stream.Stream<java.nio.file.Path> stream =
            java.nio.file.Files.list(asPath(path))) {
            java.util.ArrayList<Object> paths = new java.util.ArrayList<Object>();
            stream.forEach(entry -> paths.add(pathValue(entry)));
            return ok(freezeList(paths));
        } catch (java.io.IOException | RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object netReceiveChunk() {
        String input = System.getenv("VELN_NET_CHUNK_HEX");
        if (input == null) {
            input = "";
        }
        Object chunk = byteChunkFromHex(input);
        if (isErr(chunk)) {
            throw new RuntimeFailure("transport receive failed: malformed VELN_NET_CHUNK_HEX");
        }
        return unwrapOk(chunk);
    }

    public static Object netSendChunk(Object bytes) {
        try {
            writeNetEvent("send " + format(bytes));
        } catch (java.io.IOException | RuntimeException error) {
            throw new RuntimeFailure(
                "transport send failed: could not record VELN_NET_EVENTS",
                error
            );
        }
        return UNIT;
    }

    public static Object processArgs() {
        return processArgs;
    }

    public static Object processEnv(Object name) {
        String value = System.getenv(asString(name));
        return value == null ? none() : some(value);
    }

    public static Object processCwd() {
        try {
            return ok(pathValue(java.nio.file.Paths.get("").toAbsolutePath().normalize()));
        } catch (RuntimeException error) {
            return err(error.getMessage() == null ? error.getClass().getSimpleName() : error.getMessage());
        }
    }

    public static Object processExit(Object status) {
        long code = asLong(status);
        if (code < 0L) {
            code = 0L;
        }
        if (code > 255L) {
            code = 255L;
        }
        System.exit((int) code);
        return UNIT;
    }

    public static Object timeTimeoutMs(Object milliseconds) {
        long value = asLong(milliseconds);
        if (value <= 0L) {
            return UNIT;
        }
        try {
            Thread.sleep(value);
        } catch (InterruptedException error) {
            Thread.currentThread().interrupt();
        }
        return UNIT;
    }

    private static void writeNetEvent(String line) throws java.io.IOException {
        String path = System.getenv("VELN_NET_EVENTS");
        if (path == null || path.isEmpty()) {
            return;
        }
        java.nio.file.Files.write(
            java.nio.file.Paths.get(path),
            java.util.Collections.singletonList(line),
            java.nio.charset.StandardCharsets.UTF_8,
            java.nio.file.StandardOpenOption.CREATE,
            java.nio.file.StandardOpenOption.APPEND
        );
    }
