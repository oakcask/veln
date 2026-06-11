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
        FixtureHexDiagnostic fixtureHex = fixtureHexDiagnostics.get().remove(value);
        ByteDiagnostic byteDiagnostic = byteDiagnostics.get().remove(value);
        String line = "result"
            + "\t" + hex(value);
        if (fixtureHex != null) {
            line = line
                + "\tfixture_hex"
                + "\t" + fixtureHex.id
                + "\t" + Integer.toString(fixtureHex.textStart)
                + "\t" + Integer.toString(fixtureHex.textEnd)
                + "\t" + Integer.toString(fixtureHex.byteOffset)
                + "\t" + fixtureHex.nibble
                + "\t" + Integer.toString(fixtureHex.contextStart)
                + "\t" + Integer.toString(fixtureHex.contextEnd)
                + "\t" + hex(fixtureHex.context);
        }
        if (byteDiagnostic != null) {
            line = line
                + "\tbyte_diagnostic"
                + "\t" + byteDiagnostic.id
                + "\t" + Long.toString(byteDiagnostic.byteOffset)
                + "\t" + Integer.toString(byteDiagnostic.fieldPath.size())
                + formatByteDiagnosticFieldPath(byteDiagnostic.fieldPath)
                + "\t" + Long.toString(byteDiagnostic.expectedCount)
                + "\t" + Long.toString(byteDiagnostic.availableCount)
                + "\t" + byteDiagnostic.readiness;
        }
        line = line + System.lineSeparator();
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

    private static final ThreadLocal<java.util.Map<String, FixtureHexDiagnostic>> fixtureHexDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static final ThreadLocal<java.util.Map<String, ByteDiagnostic>> byteDiagnostics =
        ThreadLocal.withInitial(java.util.HashMap::new);

    private static void registerCodecIncompleteInputDiagnostic(
        String message,
        long byteOffset,
        long expectedCount,
        long availableCount
    ) {
        byteDiagnostics.get().put(
            message,
            new ByteDiagnostic(
                "codec.incomplete_input",
                byteOffset,
                java.util.Collections.emptyList(),
                expectedCount,
                availableCount,
                "need_bytes"
            )
        );
    }

    private static String formatByteDiagnosticFieldPath(
        java.util.List<ByteDiagnosticFieldPathSegment> fieldPath
    ) {
        String out = "";
        for (ByteDiagnosticFieldPathSegment segment : fieldPath) {
            out = out + "\t" + segment.kind + "\t" + hex(segment.name);
        }
        return out;
    }

    private static void registerFixtureHexDiagnostic(
        String message,
        String id,
        String input,
        int textStart,
        int textEnd,
        int byteOffset,
        String nibble
    ) {
        int contextStart = Math.max(0, textStart - 8);
        int contextEnd = Math.min(input.length(), textEnd + 8);
        fixtureHexDiagnostics.get().put(
            message,
            new FixtureHexDiagnostic(
                id,
                textStart,
                textEnd,
                byteOffset,
                nibble,
                contextStart,
                contextEnd,
                input.substring(contextStart, contextEnd)
            )
        );
    }

    private static final class FixtureHexDiagnostic {
        final String id;
        final int textStart;
        final int textEnd;
        final int byteOffset;
        final String nibble;
        final int contextStart;
        final int contextEnd;
        final String context;

        FixtureHexDiagnostic(
            String id,
            int textStart,
            int textEnd,
            int byteOffset,
            String nibble,
            int contextStart,
            int contextEnd,
            String context
        ) {
            this.id = id;
            this.textStart = textStart;
            this.textEnd = textEnd;
            this.byteOffset = byteOffset;
            this.nibble = nibble;
            this.contextStart = contextStart;
            this.contextEnd = contextEnd;
            this.context = context;
        }
    }

    private static final class ByteDiagnostic {
        final String id;
        final long byteOffset;
        final java.util.List<ByteDiagnosticFieldPathSegment> fieldPath;
        final long expectedCount;
        final long availableCount;
        final String readiness;

        ByteDiagnostic(
            String id,
            long byteOffset,
            java.util.List<ByteDiagnosticFieldPathSegment> fieldPath,
            long expectedCount,
            long availableCount,
            String readiness
        ) {
            this.id = id;
            this.byteOffset = byteOffset;
            this.fieldPath = fieldPath;
            this.expectedCount = expectedCount;
            this.availableCount = availableCount;
            this.readiness = readiness;
        }
    }

    private static final class ByteDiagnosticFieldPathSegment {
        final String kind;
        final String name;

        ByteDiagnosticFieldPathSegment(String kind, String name) {
            this.kind = kind;
            this.name = name;
        }
    }
