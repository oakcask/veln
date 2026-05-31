    public static void recordContractFailure(ContractFailure error) {
        String path = System.getenv("VELN_CONTRACT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String line = "contract"
            + "\t" + error.clause
            + "\t" + hex(error.predicate)
            + "\t" + hex(error.function)
            + "\t" + error.blame
            + "\t" + hex(error.nodeId)
            + "\t" + hex(error.sourceFile)
            + "\t" + Integer.toString(error.startLine)
            + "\t" + Integer.toString(error.startColumn)
            + "\t" + Integer.toString(error.endLine)
            + "\t" + Integer.toString(error.endColumn)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record contract error", ioError);
        }
    }

    public static void recordResultFailure(Object result) {
        String path = System.getenv("VELN_RESULT_ERRORS");
        if (path == null || path.isEmpty()) {
            return;
        }
        String value = format(asResult(result).value());
        String line = "result"
            + "\t" + hex(value)
            + System.lineSeparator();
        try {
            java.nio.file.Files.write(
                java.nio.file.Paths.get(path),
                line.getBytes(java.nio.charset.StandardCharsets.UTF_8),
                java.nio.file.StandardOpenOption.CREATE,
                java.nio.file.StandardOpenOption.APPEND
            );
        } catch (java.io.IOException ioError) {
            throw new RuntimeException("failed to record result error", ioError);
        }
    }
