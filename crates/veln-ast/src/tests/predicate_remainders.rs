use super::*;

#[test]
fn remainder_chain_index_distinguishes_repeated_text_and_quoted_percent() {
    let module = lower_source(concat!(
        "fn identity(value: String) -> String\n",
        "  value\n",
        "end\n",
        "fn checked(value: Int) -> Int\n",
        "require identity(\"value % 2\") == \"value % 2\" and value % 2 == 0 and value % 2 == 0\n",
        "  value\n",
        "end\n",
        "fn nested(value: Int) -> Int\n",
        "require ((value % 6) * 2) % 4 == 0\n",
        "  value\n",
        "end\n",
    ));
    let contract = &module.functions[1].contracts[0];
    let chains = index_predicate_remainder_chains(&contract.text, &contract.remainder_spans)
        .expect("remainder chains must index");
    assert_eq!(chains.len(), 2);
    assert_eq!(
        &contract.text[chains[0].predicate_range.clone()],
        "value % 2"
    );
    assert_eq!(
        &contract.text[chains[1].predicate_range.clone()],
        "value % 2"
    );
    assert!(chains[0].predicate_range.end < chains[1].predicate_range.start);

    let nested = &module.functions[2].contracts[0];
    let nested_chains = index_predicate_remainder_chains(&nested.text, &nested.remainder_spans)
        .expect("nested remainder chains must index");
    assert_eq!(nested_chains.len(), 2);
    assert_eq!(
        &nested.text[nested_chains[0].predicate_range.clone()],
        "value % 6"
    );
    assert_eq!(
        &nested.text[nested_chains[1].predicate_range.clone()],
        "((value % 6) * 2) % 4"
    );
}
