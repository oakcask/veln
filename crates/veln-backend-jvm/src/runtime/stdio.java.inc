    private static final Object stdioLock = new Object();
    private static int stdioSequence = 0;

    public static Object stdioPrint(Object value) {
        return stdioPrint(value, null, null);
    }

    public static Object stdioPrint(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.out.print(text);
            recordStdioEvent("stdout", "print", "none", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioPrintln(Object value) {
        return stdioPrintln(value, null, null);
    }

    public static Object stdioPrintln(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.out.print(text);
            System.out.print(System.lineSeparator());
            recordStdioEvent("stdout", "println", "newline", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioEprint(Object value) {
        return stdioEprint(value, null, null);
    }

    public static Object stdioEprint(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.err.print(text);
            recordStdioEvent("stderr", "eprint", "none", text, nodeId, sourceFile);
        }
        return UNIT;
    }

    public static Object stdioEprintln(Object value) {
        return stdioEprintln(value, null, null);
    }

    public static Object stdioEprintln(Object value, String nodeId, String sourceFile) {
        String text = format(value);
        synchronized (stdioLock) {
            System.err.print(text);
            System.err.print(System.lineSeparator());
            recordStdioEvent("stderr", "eprintln", "newline", text, nodeId, sourceFile);
        }
        return UNIT;
    }
    private static void recordStdioEvent(
        String stream,
        String operation,
        String terminator,
        String text,
        String nodeId,
        String sourceFile
    ) {
        String path = System.getenv("VELN_STDIO_EVENTS");
        if (path == null || path.isEmpty()) {
            return;
        }
        stdioSequence += 1;
        String line = Integer.toString(stdioSequence)
            + "\t" + stream
            + "\t" + operation
            + "\t" + terminator
            + "\t" + (nodeId == null ? "" : nodeId)
            + "\t" + (sourceFile == null ? "" : sourceFile)
            + "\t" + hex(text)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException error) {
            throw new RuntimeException("failed to record stdio event", error);
        }
    }
