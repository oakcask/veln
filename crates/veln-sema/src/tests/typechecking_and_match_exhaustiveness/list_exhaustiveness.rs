use super::*;

#[test]
fn minimal_list_adt_match_reports_missing_cons_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    Nil => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing list case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Cons(_)");
    assert_diagnostic_span(diagnostic, 6, 3, 8, 6);
}

#[test]
fn minimal_list_adt_match_reports_missing_qualified_nil_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Cons(head, _) => head\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing list case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Nil");
    assert_diagnostic_span(diagnostic, 6, 3, 8, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `List<Int>`."));
    assert!(related.contains("\"start\":{\"line\":6,\"column\":9,"));
    assert!(related.contains("This arm covers Cons(_)."));
    assert!(related.contains("\"start\":{\"line\":7,\"column\":5,"));
}
