use super::*;
use crate::types::TypeEnvironment;

#[test]
fn compiler_owned_covariant_arguments_join_recursively_and_in_any_order() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "pub type FirstOption = Option\n",
            "pub type SecondOption = Option\n",
            "pub type FirstResult = Result\n",
            "pub type SecondResult = Result\n",
            "pub type FirstList = List\n",
            "pub type SecondList = List\n",
            "pub type FirstDict = Dict\n",
            "pub type SecondDict = Dict\n",
            "type Positive<T>\n",
            "  Present(T)\n",
            "end\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn option(first: Option<First::Ready>, second: Option<Second::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn option_reverse(first: Option<First::Ready>, second: Option<Second::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn option_base(narrow: Option<State::Ready>, wide: Option<State>)\n",
            "  [wide, narrow]\n",
            "end\n",
            "fn result(first: Result<Option<State::Ready>, List<State::Closed>>, second: Result<Option<State::Closed>, List<State::Failed>>)\n",
            "  [first, second]\n",
            "end\n",
            "fn result_reverse(first: Result<Option<State::Ready>, List<State::Closed>>, second: Result<Option<State::Closed>, List<State::Failed>>)\n",
            "  [second, first]\n",
            "end\n",
            "fn list(first: List<State::Ready>, second: List<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn vector(first: Vec<State::Ready>, second: Vec<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn constructed_vector()\n",
            "  [[Ready], [Closed]]\n",
            "end\n",
            "fn selected_vector(flag: Bool, first: Vec<State::Ready>, second: Vec<State::Closed>)\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn selected_constructed_vector(flag: Bool)\n",
            "  if flag\n",
            "    [Ready]\n",
            "  else\n",
            "    [Closed]\n",
            "  end\n",
            "end\n",
            "fn matched_option(flag: Bool, first: Option<State::Ready>, second: Option<State::Closed>)\n",
            "  match flag\n",
            "    true => first\n",
            "    false => second\n",
            "  end\n",
            "end\n",
            "fn paired_option(first: Option<State::Ready>, second: Option<State::Closed>)\n",
            "  Paired(first, second)\n",
            "end\n",
            "fn dictionary(first: Dict<Option<State::Ready>, Positive<State::Closed>>, second: Dict<Option<State::Closed>, Positive<State::Failed>>)\n",
            "  [second, first]\n",
            "end\n",
            "fn dictionary_forward(first: Dict<Option<State::Ready>, Positive<State::Closed>>, second: Dict<Option<State::Closed>, Positive<State::Failed>>)\n",
            "  [first, second]\n",
            "end\n",
            "fn option_aliases(first: FirstOption<State::Ready>, second: SecondOption<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn option_aliases_reverse(first: FirstOption<State::Ready>, second: SecondOption<State::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn result_aliases(first: FirstResult<State::Ready, State::Closed>, second: SecondResult<State::Closed, State::Failed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn result_aliases_reverse(first: FirstResult<State::Ready, State::Closed>, second: SecondResult<State::Closed, State::Failed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn list_aliases(first: FirstList<State::Ready>, second: SecondList<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn list_aliases_reverse(first: FirstList<State::Ready>, second: SecondList<State::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn dict_aliases(first: FirstDict<String, State::Ready>, second: SecondDict<String, State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn dict_aliases_reverse(first: FirstDict<String, State::Ready>, second: SecondDict<String, State::Closed>)\n",
            "  [second, first]\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for (function, expected) in [
        ("option", "Vec<Option<State::Ready | State::Closed>>"),
        (
            "option_reverse",
            "Vec<Option<State::Ready | State::Closed>>",
        ),
        ("option_base", "Vec<Option<State>>"),
        (
            "result",
            "Vec<Result<Option<State::Ready | State::Closed>, List<State::Closed | State::Failed>>>",
        ),
        (
            "result_reverse",
            "Vec<Result<Option<State::Ready | State::Closed>, List<State::Closed | State::Failed>>>",
        ),
        ("list", "Vec<List<State::Ready | State::Closed>>"),
        ("vector", "Vec<Vec<State::Ready | State::Closed>>"),
        (
            "constructed_vector",
            "Vec<Vec<State::Ready | State::Closed>>",
        ),
        ("selected_vector", "Vec<State::Ready | State::Closed>"),
        (
            "selected_constructed_vector",
            "Vec<State::Ready | State::Closed>",
        ),
        ("matched_option", "Option<State::Ready | State::Closed>"),
        (
            "paired_option",
            "Pair<Option<State::Ready | State::Closed>>::Paired",
        ),
        (
            "dictionary",
            "Vec<Dict<Option<State::Ready | State::Closed>, Positive<State::Closed | State::Failed>>>",
        ),
        (
            "dictionary_forward",
            "Vec<Dict<Option<State::Ready | State::Closed>, Positive<State::Closed | State::Failed>>>",
        ),
        (
            "option_aliases",
            "Vec<Option<State::Ready | State::Closed>>",
        ),
        (
            "option_aliases_reverse",
            "Vec<Option<State::Ready | State::Closed>>",
        ),
        (
            "result_aliases",
            "Vec<Result<State::Ready | State::Closed, State::Closed | State::Failed>>",
        ),
        (
            "result_aliases_reverse",
            "Vec<Result<State::Ready | State::Closed, State::Closed | State::Failed>>",
        ),
        ("list_aliases", "Vec<List<State::Ready | State::Closed>>"),
        (
            "list_aliases_reverse",
            "Vec<List<State::Ready | State::Closed>>",
        ),
        (
            "dict_aliases",
            "Vec<Dict<String, State::Ready | State::Closed>>",
        ),
        (
            "dict_aliases_reverse",
            "Vec<Dict<String, State::Ready | State::Closed>>",
        ),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            expected,
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn compiler_owned_outer_alias_conflicts_use_canonical_carrier_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "pub type FirstTable = Dict\n",
            "pub type SecondTable = Dict\n",
            "fn forward(first: FirstTable<String, State::Ready>, second: SecondTable<String, State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn reverse(first: FirstTable<String, State::Ready>, second: SecondTable<String, State::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn canonical_and_alias(first: Dict<String, State::Ready>, second: FirstTable<String, State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn sticky(first: FirstTable<String, State::Ready>, second: SecondTable<String, State::Closed>, later: FirstTable<String, State::Ready>)\n",
            "  [first, second, later]\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for function in ["forward", "reverse", "sticky"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Vec<Dict<String, State>>",
            "unexpected inferred result for {function}",
        );
    }
    assert_eq!(
        environment
            .function("canonical_and_alias")
            .expect("canonical_and_alias function")
            .return_type
            .render(),
        "Vec<FirstTable<String, State>>",
    );
}

#[test]
fn failed_compiler_owned_nested_join_preserves_admitted_state() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "fn retained(first: Dict<Option<Int>, Result<First::Ready, First::Ready>>, rejected: Dict<Option<String>, Result<Second::Failed, Second::Closed>>, later: Dict<Option<Int>, Result<First::Closed, First::Ready>>)\n",
        "  [first, rejected, later]\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "rejected");

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("retained")
            .expect("retained function")
            .return_type
            .render(),
        "Vec<Dict<Option<Int>, Result<First::Ready | First::Closed, First::Ready>>>",
    );
}

#[test]
fn failed_compiler_owned_control_flow_join_preserves_admitted_state() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "fn retained(first: Bool, second: Bool, accepted: Option<State::Ready>, rejected: Option<String>, later: Option<State::Closed>)\n",
        "  if first\n",
        "    accepted\n",
        "  else if second\n",
        "    rejected\n",
        "  else\n",
        "    later\n",
        "  end\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "rejected");

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("retained")
            .expect("retained function")
            .return_type
            .render(),
        "Option<State::Ready | State::Closed>",
    );
}
