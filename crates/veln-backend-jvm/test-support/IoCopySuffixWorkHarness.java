public final class IoCopySuffixWorkHarness {
    public static void main(String[] args) {
        VelnRuntime.resetByteChunkCopiedElementsForTest();
        VelnEntry.main(new String[0]);
        long copiedElements = VelnRuntime.byteChunkCopiedElementsForTest();
        if (copiedElements == 0L || copiedElements > 64L) {
            throw new AssertionError(
                "copy suffix materialization exceeded its linear bound: " + copiedElements
            );
        }
        System.out.println("copy suffix materialization remained within its linear bound");
    }
}
