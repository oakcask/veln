#[test]
fn dictionary_refinement_sort_caches_each_key_encoding_before_comparison() {
    let runtime = include_str!("../runtime/collections.java.inc");
    let comparator_start = runtime
        .find("private static int compareUtf8Bytes(")
        .expect("dictionary UTF-8 comparator");
    let entry_start = runtime
        .find("private static final class SchemaRefinementDictionaryKey")
        .expect("cached dictionary key entry");
    let entry_end = runtime[entry_start..]
        .find("private static String adtLeafName(")
        .map(|offset| entry_start + offset)
        .expect("method following cached dictionary key entry");
    let comparator = &runtime[comparator_start..entry_start];
    let entry = &runtime[entry_start..entry_end];

    assert!(runtime.contains("keys.add(new SchemaRefinementDictionaryKey(asString(key)));"));
    assert!(runtime.contains("this.utf8Bytes = key.getBytes("));
    assert!(comparator.contains("left.utf8Bytes"));
    assert!(comparator.contains("right.utf8Bytes"));
    assert!(!comparator.contains("getBytes("));
    assert_eq!(
        entry
            .matches("getBytes(java.nio.charset.StandardCharsets.UTF_8)")
            .count(),
        1
    );
}
