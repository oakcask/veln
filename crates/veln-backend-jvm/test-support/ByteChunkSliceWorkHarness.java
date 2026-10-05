public final class ByteChunkSliceWorkHarness {
    private static final long MAXIMUM_RETAINED_CAPACITY_RATIO = 4L;

    private static Object chunk(String text) {
        return VelnRuntime.unwrapOk(VelnRuntime.byteChunkFromVisibleAsciiString(text));
    }

    private static String repeated(char value, int size) {
        StringBuilder source = new StringBuilder(size);
        for (int index = 0; index < size; index += 1) {
            source.append(value);
        }
        return source.toString();
    }

    private static void assertChunk(Object chunk, String expected) {
        if (!VelnRuntime.isAdt(chunk, "ByteChunk")) {
            throw new AssertionError("slice lost its ByteChunk identity");
        }
        Object payload = VelnRuntime.adtPayload(chunk, 0);
        if (!(payload instanceof java.util.List)
            || ((java.util.List<?>) payload).size() != expected.length()) {
            throw new AssertionError("slice exposed the wrong logical range");
        }
        String actual = (String) VelnRuntime.unwrapOk(
            VelnRuntime.byteChunkToVisibleAsciiString(chunk)
        );
        if (!expected.equals(actual)) {
            throw new AssertionError("slice contents changed: " + actual);
        }
    }

    private static long measureOneByteSuffixWork(int size) {
        Object current = chunk(repeated('a', size));
        long copiedElements = 0L;
        long sliceCreations = 0L;
        for (int remaining = size; remaining > 0; remaining -= 1) {
            Object next = VelnRuntime.unwrapOk(
                VelnRuntime.byteDrop(
                    current,
                    VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(1)))
                )
            );
            sliceCreations += 1L;
            if (!VelnRuntime.byteChunksShareStorageForTest(current, next)) {
                copiedElements += VelnRuntime.byteChunkStorageCapacityForTest(next);
            }
            assertChunk(next, repeated('a', remaining - 1));
            current = next;
        }
        return sliceCreations + copiedElements;
    }

    private static void assertLinearSuffixWork() {
        int smallerSize = 256;
        int largerSize = smallerSize + 1;
        long smallerWork = measureOneByteSuffixWork(smallerSize);
        long largerWork = measureOneByteSuffixWork(largerSize);
        if (smallerWork > 2L * smallerSize || largerWork > 2L * largerSize) {
            throw new AssertionError(
                "byte suffix work exceeded its linear bound: "
                    + smallerWork
                    + " -> "
                    + largerWork
            );
        }
    }

    private static void assertSharedAndCompactedRanges() {
        String source = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!!";
        Object original = chunk(source);

        Object sharedTake = VelnRuntime.unwrapOk(
            VelnRuntime.byteTake(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(32)))
            )
        );
        if (!VelnRuntime.byteChunksShareStorageForTest(original, sharedTake)
            || VelnRuntime.byteChunkStorageOffsetForTest(sharedTake) != 0L) {
            throw new AssertionError("a bounded take should share at offset zero");
        }
        assertChunk(sharedTake, source.substring(0, 32));

        Object sharedDrop = VelnRuntime.unwrapOk(
            VelnRuntime.byteDrop(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(16)))
            )
        );
        if (!VelnRuntime.byteChunksShareStorageForTest(original, sharedDrop)
            || VelnRuntime.byteChunkStorageOffsetForTest(sharedDrop) != 16L) {
            throw new AssertionError("a bounded suffix should share at its source offset");
        }
        assertChunk(sharedDrop, source.substring(16));

        Object compactTake = VelnRuntime.unwrapOk(
            VelnRuntime.byteTake(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(8)))
            )
        );
        if (VelnRuntime.byteChunksShareStorageForTest(original, compactTake)
            || VelnRuntime.byteChunkStorageOffsetForTest(compactTake) != 0L
            || VelnRuntime.byteChunkStorageCapacityForTest(compactTake) != 8L) {
            throw new AssertionError("a small take should compact its storage");
        }
        assertChunk(compactTake, source.substring(0, 8));

        Object compactDrop = VelnRuntime.unwrapOk(
            VelnRuntime.byteDrop(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(56)))
            )
        );
        if (VelnRuntime.byteChunksShareStorageForTest(original, compactDrop)
            || VelnRuntime.byteChunkStorageOffsetForTest(compactDrop) != 0L
            || VelnRuntime.byteChunkStorageCapacityForTest(compactDrop) != 8L) {
            throw new AssertionError("a small suffix should compact its storage");
        }
        assertChunk(compactDrop, source.substring(56));

        Object sharedViewValue = VelnRuntime.unwrapOk(
            VelnRuntime.byteView(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteOffset(Long.valueOf(16))),
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(16)))
            )
        );
        Object sharedView = VelnRuntime.byteViewToChunk(sharedViewValue);
        if (!VelnRuntime.byteChunksShareStorageForTest(original, sharedView)
            || VelnRuntime.byteChunkStorageOffsetForTest(sharedView) != 16L
            || VelnRuntime.byteChunkStorageCapacityForTest(sharedView) != 64L) {
            throw new AssertionError("a view at the retention boundary should share its storage");
        }
        assertChunk(sharedView, source.substring(16, 32));

        Object compactViewValue = VelnRuntime.unwrapOk(
            VelnRuntime.byteView(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteOffset(Long.valueOf(20))),
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(15)))
            )
        );
        Object compactView = VelnRuntime.byteViewToChunk(compactViewValue);
        if (VelnRuntime.byteChunksShareStorageForTest(original, compactView)
            || VelnRuntime.byteChunkStorageOffsetForTest(compactView) != 0L
            || VelnRuntime.byteChunkStorageCapacityForTest(compactView) != 15L) {
            throw new AssertionError("a small view conversion should compact its storage");
        }
        assertChunk(compactView, source.substring(20, 35));
    }

    private static Object retainedTake(int size, int count) {
        Object original = chunk(repeated('t', size));
        return VelnRuntime.unwrapOk(
            VelnRuntime.byteTake(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(count)))
            )
        );
    }

    private static Object retainedDrop(int size, int count) {
        Object original = chunk(repeated('d', size));
        return VelnRuntime.unwrapOk(
            VelnRuntime.byteDrop(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(size - count)))
            )
        );
    }

    private static Object retainedView(int size, int offset, int count) {
        Object original = chunk(repeated('v', size));
        Object view = VelnRuntime.unwrapOk(
            VelnRuntime.byteView(
                original,
                VelnRuntime.unwrapOk(VelnRuntime.byteOffset(Long.valueOf(offset))),
                VelnRuntime.unwrapOk(VelnRuntime.byteCount(Long.valueOf(count)))
            )
        );
        return VelnRuntime.byteViewToChunk(view);
    }

    private static void assertBoundedRetention() {
        Object[] retained = new Object[] {
            retainedTake(1024, 8),
            retainedDrop(2048, 7),
            retainedView(4096, 2000, 6),
            retainedTake(1024, 0),
            retainedDrop(2048, 0),
            retainedView(4096, 2000, 0),
        };
        long logicalBytes = 0L;
        long backingCapacity = 0L;
        for (Object value : retained) {
            logicalBytes += ((java.util.List<?>) VelnRuntime.adtPayload(value, 0)).size();
            backingCapacity += VelnRuntime.byteChunkStorageCapacityForTest(value);
        }
        if (backingCapacity > MAXIMUM_RETAINED_CAPACITY_RATIO * logicalBytes) {
            throw new AssertionError(
                "retained backing capacity exceeded its constant bound: "
                    + backingCapacity
                    + " for "
                    + logicalBytes
                    + " logical bytes"
            );
        }
        for (int index = 3; index < retained.length; index += 1) {
            if (VelnRuntime.byteChunkStorageCapacityForTest(retained[index]) != 0L) {
                throw new AssertionError("an empty result retained backing storage");
            }
        }
    }

    public static void main(String[] args) {
        assertLinearSuffixWork();
        assertSharedAndCompactedRanges();
        assertBoundedRetention();
        System.out.println("byte chunk slicing remained linear and retention stayed bounded");
    }
}
