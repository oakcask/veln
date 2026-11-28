use super::*;

#[test]
fn diagnosed_overflow_literals_cross_public_lowering_without_panicking_or_reaching_ir() {
    let overflow = format!("1{}.0", "0".repeat(309));
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn expression() -> Float\n  {overflow}\nend\n\
             fn pattern(value: Float) -> Bool\n  match value\n    {overflow} => true\n    _ => false\n  end\nend\n"
        ),
    );

    let parsed = parse(&source);
    let range_diagnostics = parsed
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "parse.float_literal")
        .collect::<Vec<_>>();
    assert_eq!(range_diagnostics.len(), 2, "{:#?}", parsed.diagnostics);
    assert!(range_diagnostics.iter().all(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .is_some_and(|span| span.end.offset - span.start.offset == overflow.len())
    }));

    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    let lowering_diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "core.invalid_float_literal")
        .count();
    assert_eq!(lowering_diagnostics, 2, "{:#?}", lowered.diagnostics);
    assert!(lowered.ir.is_none());
}
