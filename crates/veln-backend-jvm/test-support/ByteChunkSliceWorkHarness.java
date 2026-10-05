public final class ByteChunkSliceWorkHarness {
    private static long[] measure(int size) {
        StringBuilder source = new StringBuilder(size);
        for (int index = 0; index < size; index += 1) {
            source.append('a');
        }

        Object chunk = VelnRuntime.unwrapOk(
            VelnRuntime.byteChunkFromVisibleAsciiString(source.toString())
        );
        Object original = chunk;
        VelnRuntime.resetByteChunkWorkForTest();
        for (int remaining = size; remaining > 1; remaining -= 1) {
            if (!VelnRuntime.isAdt(chunk, "ByteChunk")) {
                throw new AssertionError("slice lost its ByteChunk identity");
            }
            Object payload = VelnRuntime.adtPayload(chunk, 0);
            if (!(payload instanceof java.util.List)
                || ((java.util.List<?>) payload).size() != remaining) {
                throw new AssertionError("slice exposed the wrong remaining range");
            }
            Object wrapped = VelnRuntime.adt("Envelope", new Object[] { chunk });
            chunk = VelnRuntime.unwrapOk(
                VelnRuntime.byteDrop(
                    VelnRuntime.adtPayload(wrapped, 0),
                    VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(1)))
                )
            );
            if (!VelnRuntime.byteChunksShareStorageForTest(original, chunk)
                || VelnRuntime.byteChunkStorageOffsetForTest(chunk) != size - remaining + 1L) {
                throw new AssertionError("derived suffix did not retain its backing storage");
            }
        }
        long materialized = VelnRuntime.byteChunkMaterializedElementWorkForTest();
        long slices = VelnRuntime.byteChunkSliceWorkForTest();
        Object view = VelnRuntime.unwrapOk(
            VelnRuntime.byteView(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteOffset(Long.valueOf(size / 4))),
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(size / 2)))
            )
        );
        Object viewChunk = VelnRuntime.byteViewToChunk(view);
        if (!VelnRuntime.byteChunksShareStorageForTest(original, viewChunk)
            || VelnRuntime.byteChunkStorageOffsetForTest(viewChunk) != size / 4L) {
            throw new AssertionError("ByteView conversion did not retain its backing storage");
        }
        return new long[] { materialized, slices };
    }

    public static void main(String[] args) {
        long[] smaller = measure(256);
        long[] larger = measure(512);
        if (smaller[0] != 0L || larger[0] != 0L) {
            throw new AssertionError(
                "byte suffixes rematerialized storage: "
                    + smaller[0]
                    + " -> "
                    + larger[0]
            );
        }
        if (smaller[1] != 255L || larger[1] != 511L
            || larger[1] != smaller[1] * 2L + 1L) {
            throw new AssertionError(
                "byte slice creation was not linear: "
                    + smaller[1]
                    + " -> "
                    + larger[1]
            );
        }
        System.out.println("byte chunk suffix work remained linear");
    }
}
