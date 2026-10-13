use super::*;

#[test]
fn folded_remainder_float_arithmetic_does_not_create_false_repairs() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int) -> Int\n",
            "require fallback >= 0\n",
            "  let false_division: Int = _division satisfy candidate => (5 % 2) / 2.0 == 0\n",
            "  let false_addition: Int = _addition satisfy candidate => (5 % 2) + (1.0 / 2) == 1\n",
            "  let true_float: Int = _true satisfy candidate => (5 % 2) + 0.5 == 1.5\n",
            "  _independent satisfy candidate => candidate >= 0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
    let holes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "hole.unfilled")
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 4, "{diagnostics:#?}");

    for hole in &holes[..2] {
        let false_proof = hole.details.to_json();
        assert!(
            !false_proof.contains("safe_repair_candidate"),
            "{false_proof}"
        );
        assert!(
            !false_proof.contains("statically_satisfied"),
            "{false_proof}"
        );
    }
    let true_float = holes[2].details.to_json();
    assert!(true_float.contains("safe_repair_candidate"), "{true_float}");
    assert!(true_float.contains("statically_satisfied"), "{true_float}");
    let independent = holes[3].details.to_json();
    assert!(
        independent.contains("safe_repair_candidate"),
        "{independent}"
    );
}
