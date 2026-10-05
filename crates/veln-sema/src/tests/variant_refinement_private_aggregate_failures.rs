use super::*;
use crate::types::TypeEnvironment;

#[test]
fn aggregate_created_failures_do_not_escape_nested_private_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type Foreign\n",
            "  Other\n",
            "end\n",
            "fn other() -> Foreign::Other\n",
            "  Other\n",
            "end\n",
            "fn ready() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn closed() -> State::Closed\n",
            "  Closed\n",
            "end\n",
            "fn nested_vector()\n",
            "  [[Ready, other()], [Closed]]\n",
            "end\n",
            "fn nested_dict_key()\n",
            "  [{ready(): 1, other(): 2}, {closed(): 3}]\n",
            "end\n",
            "fn nested_dict_value()\n",
            "  [{1: Ready, 2: other()}, {3: Closed}]\n",
            "end\n",
            "fn accept_vector(value: Vec<Vec<State::Closed>>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_dict_key(value: Vec<Dict<State::Closed, Int>>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_dict_value(value: Vec<Dict<Int, State::Closed>>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept_vector(nested_vector())\n",
            "  accept_dict_key(nested_dict_key())\n",
            "  accept_dict_value(nested_dict_value())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
    for diagnostic in &diagnostics {
        assert_eq!(diagnostic.id, "type.mismatch", "{diagnostics:#?}");
    }
    let rejected_spans = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("mismatch span");
            &source.text()[span.start.offset..span.end.offset]
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rejected_spans
            .iter()
            .filter(|span| **span == "other()")
            .count(),
        3,
        "{diagnostics:#?}"
    );
    assert_eq!(
        rejected_spans
            .iter()
            .filter(|span| matches!(**span, "Ready" | "ready()"))
            .count(),
        3,
        "{diagnostics:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    for (function, expected) in [
        ("nested_vector", "Vec<Vec<State::Closed>>"),
        ("nested_dict_key", "Vec<Dict<State::Closed, Int>>"),
        ("nested_dict_value", "Vec<Dict<Int, State::Closed>>"),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected,
            "{function}"
        );
    }
}
