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
            "fn paired_constructed_vector()\n",
            "  Paired([Ready], [Closed])\n",
            "end\n",
            "fn paired_constructed_vector_reverse()\n",
            "  Paired([Closed], [Ready])\n",
            "end\n",
            "fn paired_constructed_option()\n",
            "  Paired(Some(Ready), None)\n",
            "end\n",
            "fn paired_constructed_option_reverse()\n",
            "  Paired(None, Some(Ready))\n",
            "end\n",
            "fn constructed_option()\n",
            "  [Some(Ready), None]\n",
            "end\n",
            "fn constructed_option_reverse()\n",
            "  [None, Some(Ready)]\n",
            "end\n",
            "fn selected_constructed_option(flag: Bool)\n",
            "  if flag\n",
            "    Some(Ready)\n",
            "  else\n",
            "    None\n",
            "  end\n",
            "end\n",
            "fn selected_constructed_option_reverse(flag: Bool)\n",
            "  if flag\n",
            "    None\n",
            "  else\n",
            "    Some(Ready)\n",
            "  end\n",
            "end\n",
            "fn matched_constructed_option(flag: Bool)\n",
            "  match flag\n",
            "    true => Some(Ready)\n",
            "    false => None\n",
            "  end\n",
            "end\n",
            "fn matched_constructed_option_reverse(flag: Bool)\n",
            "  match flag\n",
            "    true => None\n",
            "    false => Some(Ready)\n",
            "  end\n",
            "end\n",
            "fn constructed_option_dict()\n",
            "  {1: Some(Ready), 2: None}\n",
            "end\n",
            "fn constructed_option_dict_reverse()\n",
            "  {1: None, 2: Some(Ready)}\n",
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
            "paired_constructed_vector",
            "Pair<Vec<State::Ready | State::Closed>>::Paired",
        ),
        (
            "paired_constructed_vector_reverse",
            "Pair<Vec<State::Ready | State::Closed>>::Paired",
        ),
        (
            "paired_constructed_option",
            "Pair<Option<State::Ready>>::Paired",
        ),
        (
            "paired_constructed_option_reverse",
            "Pair<Option<State::Ready>>::Paired",
        ),
        ("constructed_option", "Vec<Option<State::Ready>>"),
        ("constructed_option_reverse", "Vec<Option<State::Ready>>"),
        ("selected_constructed_option", "Option<State::Ready>"),
        (
            "selected_constructed_option_reverse",
            "Option<State::Ready>",
        ),
        ("matched_constructed_option", "Option<State::Ready>"),
        ("matched_constructed_option_reverse", "Option<State::Ready>"),
        ("constructed_option_dict", "Dict<Int, Option<State::Ready>>"),
        (
            "constructed_option_dict_reverse",
            "Dict<Int, Option<State::Ready>>",
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
fn body_inference_separates_join_contributions_from_completion_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn verify(flag: Bool, complete_list: List<Int>, complete_dictionary: Dict<Int, String>) -> ()\n",
            "  let paired = Paired([Ready], [Closed])\n",
            "  let paired_reverse = Paired([Closed], [Ready])\n",
            "  let option_pair = Paired(Some(Ready), None)\n",
            "  let option_pair_reverse = Paired(None, Some(Ready))\n",
            "  let options = [Some(Ready), None]\n",
            "  let options_reverse = [None, Some(Ready)]\n",
            "  let selected = if flag Some(Ready) else None end\n",
            "  let selected_reverse = if flag None else Some(Ready) end\n",
            "  let matched = match flag true => Some(Ready) false => None end\n",
            "  let matched_reverse = match flag true => None false => Some(Ready) end\n",
            "  let dictionary = {1: Some(Ready), 2: None}\n",
            "  let dictionary_reverse = {1: None, 2: Some(Ready)}\n",
            "  let lists = [complete_list, Nil]\n",
            "  let lists_reverse = [Nil, complete_list]\n",
            "  let vectors = [[1], []]\n",
            "  let vectors_reverse = [[], [1]]\n",
            "  let dictionaries = [complete_dictionary, {}]\n",
            "  let dictionaries_reverse = [{}, complete_dictionary]\n",
            "  let _: Pair<Vec<State::Ready | State::Closed>>::Paired = paired\n",
            "  let _: Pair<Vec<State::Ready | State::Closed>>::Paired = paired_reverse\n",
            "  let _: Pair<Option<State::Ready>>::Paired = option_pair\n",
            "  let _: Pair<Option<State::Ready>>::Paired = option_pair_reverse\n",
            "  let _: Vec<Option<State::Ready>> = options\n",
            "  let _: Vec<Option<State::Ready>> = options_reverse\n",
            "  let _: Option<State::Ready> = selected\n",
            "  let _: Option<State::Ready> = selected_reverse\n",
            "  let _: Option<State::Ready> = matched\n",
            "  let _: Option<State::Ready> = matched_reverse\n",
            "  let _: Dict<Int, Option<State::Ready>> = dictionary\n",
            "  let _: Dict<Int, Option<State::Ready>> = dictionary_reverse\n",
            "  let _: Vec<List<Int>> = lists\n",
            "  let _: Vec<List<Int>> = lists_reverse\n",
            "  let _: Vec<Vec<Int>> = vectors\n",
            "  let _: Vec<Vec<Int>> = vectors_reverse\n",
            "  let _: Vec<Dict<Int, String>> = dictionaries\n",
            "  let _: Vec<Dict<Int, String>> = dictionaries_reverse\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nested_compiler_owned_completion_context_is_order_independent() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn forward()\n",
            "  [Some(None), Some(Some(1))]\n",
            "end\n",
            "fn reverse()\n",
            "  [Some(Some(1)), Some(None)]\n",
            "end\n",
            "fn verify() -> ()\n",
            "  let forward = [Some(None), Some(Some(1))]\n",
            "  let reverse = [Some(Some(1)), Some(None)]\n",
            "  let _: Vec<Option<Option<Int>>> = forward\n",
            "  let _: Vec<Option<Option<Int>>> = reverse\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for function in ["forward", "reverse"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Vec<Option<Option<Int>>>",
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn nested_completion_projects_through_source_adt_payloads_in_any_order() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<T>\n",
            "  Boxed(T)\n",
            "end\n",
            "fn forward()\n",
            "  [Some(Boxed(None)), Some(Boxed(Some(1)))]\n",
            "end\n",
            "fn reverse()\n",
            "  [Some(Boxed(Some(1))), Some(Boxed(None))]\n",
            "end\n",
            "fn verify() -> ()\n",
            "  let forward = [Some(Boxed(None)), Some(Boxed(Some(1)))]\n",
            "  let reverse = [Some(Boxed(Some(1))), Some(Boxed(None))]\n",
            "  let _: Vec<Option<Box<Option<Int>>>> = forward\n",
            "  let _: Vec<Option<Box<Option<Int>>>> = reverse\n",
            "  let _: Vec<Option<Box<Option<Int>>>> = [Some(Boxed(None)), Some(Boxed(Some(1)))]\n",
            "  let _: Vec<Option<Box<Option<Int>>>> = [Some(Boxed(Some(1))), Some(Boxed(None))]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for function in ["forward", "reverse"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Vec<Option<Box<Option<Int>>>>",
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn nested_completion_requires_checked_covariant_source_payloads() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "type Fixed<T>\n",
        "  Output(T)\n",
        "  Input(fn(T) -> ())\n",
        "end\n",
        "type Negative<T>\n",
        "  Consume(fn(T) -> ())\n",
        "  NegativeEmpty\n",
        "end\n",
        "type Ignored<T>\n",
        "  IgnoredEmpty\n",
        "end\n",
        "fn forward(known: Fixed<Option<State::Ready>>)\n",
        "  [known, Output(None)]\n",
        "end\n",
        "fn reverse(known: Fixed<Option<State::Ready>>)\n",
        "  [Output(None), known]\n",
        "end\n",
        "fn negative_forward(first: Negative<Option<State::Ready>>, second: Negative<Option<State::Closed>>)\n",
        "  [first, second]\n",
        "end\n",
        "fn negative_reverse(first: Negative<Option<State::Ready>>, second: Negative<Option<State::Closed>>)\n",
        "  [second, first]\n",
        "end\n",
        "fn ignored_ready(value: Ignored<State::Ready>) -> Ignored<State::Ready>\n",
        "  value\n",
        "end\n",
        "fn ignored_closed(value: Ignored<State::Closed>) -> Ignored<State::Closed>\n",
        "  value\n",
        "end\n",
        "fn ignored_forward(first: Ignored<State::Ready>, second: Ignored<State::Closed>) -> ()\n",
        "  let values = [ignored_ready(first), ignored_closed(second)]\n",
        "  ()\n",
        "end\n",
        "fn ignored_reverse(first: Ignored<State::Ready>, second: Ignored<State::Closed>) -> ()\n",
        "  let values = [ignored_closed(second), ignored_ready(first)]\n",
        "  ()\n",
        "end\n",
        "fn compiler_owned_inferred_context_remains_deferred(flag: Bool) -> Result<(), String>\n",
        "  let _ = if flag\n",
        "    Ok(())\n",
        "  else\n",
        "    Err(\"failed\")\n",
        "  end?\n",
        "  Ok(())\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "type.inference_ambiguous",
            "type.inference_ambiguous",
            "type.mismatch",
            "type.mismatch",
            "type.mismatch",
            "type.mismatch",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn unavailable_source_variance_metadata_disables_nested_completion() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Box<T>\n",
        "  Boxed(T)\n",
        "end\n",
        "fn forward(known: Box<Option<State::Ready>>)\n",
        "  [known, Boxed(None)]\n",
        "end\n",
        "fn reverse(known: Box<Option<State::Ready>>)\n",
        "  [Boxed(None), known]\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let mut environment = TypeEnvironment::from_module(&module);
    let identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Box")
        .expect("box descriptor")
        .identity();
    environment
        .adts
        .clear_parameter_variances_for_identity(&identity);

    let diagnostics =
        crate::pipeline::analyze_surface_module_with_environment_for_test(&module, &environment);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.inference_ambiguous"),
        "{diagnostics:#?}"
    );
}

#[test]
fn non_empty_literals_propagate_nested_completion_context_in_any_order() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn vector_forward()\n",
            "  [[None], [Some(1)]]\n",
            "end\n",
            "fn vector_reverse()\n",
            "  [[Some(1)], [None]]\n",
            "end\n",
            "fn mixed_vector_forward()\n",
            "  [[None, Some(Ready)], [Some(Closed)]]\n",
            "end\n",
            "fn mixed_vector_reverse()\n",
            "  [[Some(Closed)], [None, Some(Ready)]]\n",
            "end\n",
            "fn dictionary_forward()\n",
            "  [{1: None}, {1: Some(1)}]\n",
            "end\n",
            "fn dictionary_reverse()\n",
            "  [{1: Some(1)}, {1: None}]\n",
            "end\n",
            "fn mixed_dictionary_forward()\n",
            "  [{1: None, 2: Some(Ready)}, {1: Some(Closed)}]\n",
            "end\n",
            "fn mixed_dictionary_reverse()\n",
            "  [{1: Some(Closed)}, {1: None, 2: Some(Ready)}]\n",
            "end\n",
            "fn dictionary_entries_forward()\n",
            "  {1: [None], 2: [Some(1)]}\n",
            "end\n",
            "fn dictionary_entries_reverse()\n",
            "  {1: [Some(1)], 2: [None]}\n",
            "end\n",
            "fn repeated_payload_forward()\n",
            "  Paired([None], [Some(1)])\n",
            "end\n",
            "fn repeated_payload_reverse()\n",
            "  Paired([Some(1)], [None])\n",
            "end\n",
            "fn repeated_nested_payload_forward()\n",
            "  [Some(Paired(None, Some(Ready))), Some(Paired(Some(Closed), Some(Closed)))]\n",
            "end\n",
            "fn repeated_nested_payload_reverse()\n",
            "  [Some(Paired(Some(Closed), Some(Closed))), Some(Paired(None, Some(Ready)))]\n",
            "end\n",
            "fn selected_forward(flag: Bool)\n",
            "  if flag [None] else [Some(1)] end\n",
            "end\n",
            "fn selected_reverse(flag: Bool)\n",
            "  if flag [Some(1)] else [None] end\n",
            "end\n",
            "fn matched_forward(flag: Bool)\n",
            "  match flag true => [None] false => [Some(1)] end\n",
            "end\n",
            "fn matched_reverse(flag: Bool)\n",
            "  match flag true => [Some(1)] false => [None] end\n",
            "end\n",
            "fn verify(flag: Bool) -> ()\n",
            "  let vector_forward = [[None], [Some(1)]]\n",
            "  let vector_reverse = [[Some(1)], [None]]\n",
            "  let mixed_vector_forward = [[None, Some(Ready)], [Some(Closed)]]\n",
            "  let mixed_vector_reverse = [[Some(Closed)], [None, Some(Ready)]]\n",
            "  let dictionary_forward = [{1: None}, {1: Some(1)}]\n",
            "  let dictionary_reverse = [{1: Some(1)}, {1: None}]\n",
            "  let mixed_dictionary_forward = [{1: None, 2: Some(Ready)}, {1: Some(Closed)}]\n",
            "  let mixed_dictionary_reverse = [{1: Some(Closed)}, {1: None, 2: Some(Ready)}]\n",
            "  let dictionary_entries_forward = {1: [None], 2: [Some(1)]}\n",
            "  let dictionary_entries_reverse = {1: [Some(1)], 2: [None]}\n",
            "  let repeated_payload_forward = Paired([None], [Some(1)])\n",
            "  let repeated_payload_reverse = Paired([Some(1)], [None])\n",
            "  let repeated_nested_payload_forward = [Some(Paired(None, Some(Ready))), Some(Paired(Some(Closed), Some(Closed)))]\n",
            "  let repeated_nested_payload_reverse = [Some(Paired(Some(Closed), Some(Closed))), Some(Paired(None, Some(Ready)))]\n",
            "  let selected_forward = if flag [None] else [Some(1)] end\n",
            "  let selected_reverse = if flag [Some(1)] else [None] end\n",
            "  let matched_forward = match flag true => [None] false => [Some(1)] end\n",
            "  let matched_reverse = match flag true => [Some(1)] false => [None] end\n",
            "  let _: Vec<Vec<Option<Int>>> = vector_forward\n",
            "  let _: Vec<Vec<Option<Int>>> = vector_reverse\n",
            "  let _: Vec<Vec<Option<State>>> = mixed_vector_forward\n",
            "  let _: Vec<Vec<Option<State>>> = mixed_vector_reverse\n",
            "  let _: Vec<Dict<Int, Option<Int>>> = dictionary_forward\n",
            "  let _: Vec<Dict<Int, Option<Int>>> = dictionary_reverse\n",
            "  let _: Vec<Dict<Int, Option<State>>> = mixed_dictionary_forward\n",
            "  let _: Vec<Dict<Int, Option<State>>> = mixed_dictionary_reverse\n",
            "  let _: Dict<Int, Vec<Option<Int>>> = dictionary_entries_forward\n",
            "  let _: Dict<Int, Vec<Option<Int>>> = dictionary_entries_reverse\n",
            "  let _: Pair<Vec<Option<Int>>>::Paired = repeated_payload_forward\n",
            "  let _: Pair<Vec<Option<Int>>>::Paired = repeated_payload_reverse\n",
            "  let _: Vec<Option<Pair<Option<State>>>> = repeated_nested_payload_forward\n",
            "  let _: Vec<Option<Pair<Option<State>>>> = repeated_nested_payload_reverse\n",
            "  let _: Vec<Option<Int>> = selected_forward\n",
            "  let _: Vec<Option<Int>> = selected_reverse\n",
            "  let _: Vec<Option<Int>> = matched_forward\n",
            "  let _: Vec<Option<Int>> = matched_reverse\n",
            "  ()\n",
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
        ("vector_forward", "Vec<Vec<Option<Int>>>"),
        ("vector_reverse", "Vec<Vec<Option<Int>>>"),
        ("mixed_vector_forward", "Vec<Vec<Option<State>>>"),
        ("mixed_vector_reverse", "Vec<Vec<Option<State>>>"),
        ("dictionary_forward", "Vec<Dict<Int, Option<Int>>>"),
        ("dictionary_reverse", "Vec<Dict<Int, Option<Int>>>"),
        ("mixed_dictionary_forward", "Vec<Dict<Int, Option<State>>>"),
        ("mixed_dictionary_reverse", "Vec<Dict<Int, Option<State>>>"),
        ("dictionary_entries_forward", "Dict<Int, Vec<Option<Int>>>"),
        ("dictionary_entries_reverse", "Dict<Int, Vec<Option<Int>>>"),
        ("repeated_payload_forward", "Pair<Vec<Option<Int>>>::Paired"),
        ("repeated_payload_reverse", "Pair<Vec<Option<Int>>>::Paired"),
        (
            "repeated_nested_payload_forward",
            "Vec<Option<Pair<Option<State>>>::Some>",
        ),
        (
            "repeated_nested_payload_reverse",
            "Vec<Option<Pair<Option<State>>>::Some>",
        ),
        ("selected_forward", "Vec<Option<Int>>"),
        ("selected_reverse", "Vec<Option<Int>>"),
        ("matched_forward", "Vec<Option<Int>>"),
        ("matched_reverse", "Vec<Option<Int>>"),
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
fn source_none_and_nil_heads_keep_refinements_during_list_tail_completion() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  None\n",
            "  Nil\n",
            "  Ready\n",
            "end\n",
            "fn none_forward(ready: List<State::Ready>)\n",
            "  [ready, Cons(State::None, Nil)]\n",
            "end\n",
            "fn none_reverse(ready: List<State::Ready>)\n",
            "  [Cons(State::None, Nil), ready]\n",
            "end\n",
            "fn nil_forward(ready: List<State::Ready>)\n",
            "  [ready, Cons(State::Nil, Nil)]\n",
            "end\n",
            "fn nil_reverse(ready: List<State::Ready>)\n",
            "  [Cons(State::Nil, Nil), ready]\n",
            "end\n",
            "fn verify(ready: List<State::Ready>) -> ()\n",
            "  let none_forward = [ready, Cons(State::None, Nil)]\n",
            "  let none_reverse = [Cons(State::None, Nil), ready]\n",
            "  let nil_forward = [ready, Cons(State::Nil, Nil)]\n",
            "  let nil_reverse = [Cons(State::Nil, Nil), ready]\n",
            "  let _: Vec<List<State::None | State::Ready>> = none_forward\n",
            "  let _: Vec<List<State::None | State::Ready>> = none_reverse\n",
            "  let _: Vec<List<State::Nil | State::Ready>> = nil_forward\n",
            "  let _: Vec<List<State::Nil | State::Ready>> = nil_reverse\n",
            "  ()\n",
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
        ("none_forward", "Vec<List<State::None | State::Ready>>"),
        ("none_reverse", "Vec<List<State::None | State::Ready>>"),
        ("nil_forward", "Vec<List<State::Nil | State::Ready>>"),
        ("nil_reverse", "Vec<List<State::Nil | State::Ready>>"),
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
fn source_variants_named_none_or_nil_remain_join_contributions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  None\n",
            "  Nil\n",
            "  Ready\n",
            "end\n",
            "fn none_forward()\n",
            "  [State::None, State::Ready]\n",
            "end\n",
            "fn none_reverse()\n",
            "  [State::Ready, State::None]\n",
            "end\n",
            "fn nil_forward()\n",
            "  [State::Nil, State::Ready]\n",
            "end\n",
            "fn nil_reverse()\n",
            "  [State::Ready, State::Nil]\n",
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
        ("none_forward", "Vec<State::None | State::Ready>"),
        ("none_reverse", "Vec<State::None | State::Ready>"),
        ("nil_forward", "Vec<State::Nil | State::Ready>"),
        ("nil_reverse", "Vec<State::Nil | State::Ready>"),
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
