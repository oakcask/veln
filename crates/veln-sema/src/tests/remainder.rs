use super::*;

fn diagnostics_for(text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn remainder_is_int_only_even_in_float_contexts() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> Float\n",
        "  let a: Int = 7 % 3\n",
        "  let b = 7.0 % 3\n",
        "  let c = 7 % 3.0\n",
        "  let d = true % 2\n",
        "  let e = 7 % \"3\"\n",
        "  7 % 3\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        5,
        "{diagnostics:#?}"
    );
}

#[test]
fn literal_zero_remainder_reports_divisor_and_operation() {
    for divisor in ["0", "(-0)", "0x0", "0b0"] {
        let diagnostics = diagnostics_for(&format!("fn main() -> Int\n  7 % {divisor}\nend\n"));
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "type.remainder_by_zero")
            .expect("literal zero should be rejected");
        assert_eq!(diagnostic.message.as_str(), "remainder divisor is zero");
        assert_eq!(diagnostic.related.len(), 1);
    }
}

#[test]
fn computed_zero_remainder_remains_runtime_checked() {
    let diagnostics = diagnostics_for("fn main() -> Int\n  7 % (1 - 1)\nend\n");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.remainder_by_zero"),
        "{diagnostics:#?}"
    );
}
