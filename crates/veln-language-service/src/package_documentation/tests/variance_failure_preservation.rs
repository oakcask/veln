use super::*;

#[test]
fn package_catalog_rejects_the_complete_variance_failed_signature_set() {
    let result = generate(
        concat!(
            "[package]\n",
            "name = \"demo\"\n",
            "version = \"0.1.0\"\n",
            "\n[lib]\n",
            "exports = [\"main.veln\"]\n",
        ),
        &[(
            "main.veln",
            concat!(
                "pub type Producer<T>\n",
                "  pub Produced(value: T)\n",
                "end\n",
                "pub fn accepts_string(value: String) -> Int\n",
                "  1\n",
                "end\n",
                "pub fn rejected(value: Producer<String>) -> Producer<Int>\n",
                "  let callback: fn(Int) -> Int = accepts_string\n",
                "  value\n",
                "end\n",
                "pub fn healthy() -> Int\n",
                "  1\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.gate == "semantic_variance" && diagnostic.code == "type.mismatch"
    }));
}
