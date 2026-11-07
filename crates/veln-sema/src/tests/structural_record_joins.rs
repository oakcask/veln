use super::*;
use crate::types::TypeEnvironment;

#[test]
fn structural_record_joins_cover_shared_inference_consumers_and_order() {
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
            "fn collection(first: {label: String, state: State::Ready}, second: {state: State::Closed, label: String})\n",
            "  [first, second]\n",
            "end\n",
            "fn collection_reverse(first: {label: String, state: State::Ready}, second: {state: State::Closed, label: String})\n",
            "  [second, first]\n",
            "end\n",
            "fn nested(first: {outer: {state: State::Ready}, label: String}, second: {label: String, outer: {state: State::Closed}})\n",
            "  [first, second]\n",
            "end\n",
            "fn payload(first: {label: String, state: State::Ready}, second: {state: State::Closed, label: String})\n",
            "  Paired(first, second)\n",
            "end\n",
            "fn exact_leaf(first: {callback: fn(Int) -> String, state: State::Ready}, second: {state: State::Closed, callback: fn(Int) -> String})\n",
            "  [second, first]\n",
            "end\n",
            "fn selected(flag: Bool, first: {label: String, state: State::Ready}, second: {state: State::Closed, label: String})\n",
            "  if flag\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end\n",
            "end\n",
            "fn matched(flag: Bool, first: {label: String, state: State::Ready}, second: {state: State::Closed, label: String})\n",
            "  match flag\n",
            "    true => first\n",
            "    false => second\n",
            "  end\n",
            "end\n",
            "fn private_result(flag: Bool)\n",
            "  if flag\n",
            "    {state: Ready, label: \"ready\"}\n",
            "  else\n",
            "    {label: \"closed\", state: Closed}\n",
            "  end\n",
            "end\n",
            "fn consume_private(flag: Bool) -> {label: String, state: State::Ready | State::Closed}\n",
            "  private_result(flag)\n",
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
        ("collection", "Vec<{label: String, state: State}>"),
        ("collection_reverse", "Vec<{label: String, state: State}>"),
        ("nested", "Vec<{label: String, outer: {state: State}}>"),
        ("payload", "Pair<{label: String, state: State}>::Paired"),
        (
            "exact_leaf",
            "Vec<{callback: fn(Int) -> String, state: State}>",
        ),
        ("selected", "{label: String, state: State}"),
        ("matched", "{label: String, state: State}"),
        ("private_result", "{label: String, state: State}"),
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
fn failed_record_join_preserves_fields_refinements_and_alias_presentation() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "fn retained(first: {guard: Option<Int>, state: First::Ready}, rejected: {state: Second::Failed, guard: Option<String>}, later: {state: First::Closed, guard: Option<Int>})\n",
        "  [first, rejected, later]\n",
        "end\n",
        "fn different_shape(first: {id: Int, label: String}, rejected: {id: Int}, later: {label: String, id: Int})\n",
        "  [first, rejected, later]\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );
    let spans = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("mismatch span");
            &source_text[span.start.offset..span.end.offset]
        })
        .collect::<Vec<_>>();
    assert_eq!(spans, ["rejected", "rejected"]);

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("retained")
            .expect("retained function")
            .return_type
            .render(),
        "Vec<{guard: Option<Int>, state: First::Ready | First::Closed}>",
    );
    assert_eq!(
        environment
            .function("different_shape")
            .expect("different_shape function")
            .return_type
            .render(),
        "Vec<{id: Int, label: String}>",
    );
}

#[test]
fn failed_record_control_flow_contributions_preserve_the_admitted_join() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Last\n",
        "  Other\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "fn needs_int(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn ordinary_if(first: Bool, second: Bool, third: Bool, ready: {guard: Option<Int>, state: First::Ready}, closed: {guard: Option<Int>, state: First::Closed}, rejected: {guard: Option<String>, state: Second::Last}, later: {guard: Option<Int>, state: First::Other}) -> ()\n",
        "  let selected = if first\n",
        "    ready\n",
        "  else if second\n",
        "    closed\n",
        "  else if third\n",
        "    rejected\n",
        "  else\n",
        "    later\n",
        "  end\n",
        "  needs_int(selected)\n",
        "end\n",
        "fn ordinary_match(state: State, ready: {guard: Option<Int>, state: First::Ready}, closed: {guard: Option<Int>, state: First::Closed}, rejected: {guard: Option<String>, state: Second::Last}, later: {guard: Option<Int>, state: First::Other}) -> ()\n",
        "  let selected = match state\n",
        "    Ready => ready\n",
        "    Closed => closed\n",
        "    Last => rejected\n",
        "    Other => later\n",
        "  end\n",
        "  needs_int(selected)\n",
        "end\n",
        "fn private_result(first: Bool, second: Bool, third: Bool, ready: {guard: Option<Int>, state: First::Ready}, closed: {guard: Option<Int>, state: First::Closed}, rejected: {guard: Option<String>, state: Second::Last}, later: {guard: Option<Int>, state: First::Other})\n",
        "  if first\n",
        "    ready\n",
        "  else if second\n",
        "    closed\n",
        "  else if third\n",
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
    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );
    let rejected_spans = diagnostics
        .iter()
        .filter(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("mismatch span");
            &source_text[span.start.offset..span.end.offset] == "rejected"
        })
        .count();
    assert_eq!(rejected_spans, 3, "{diagnostics:#?}");
    let final_type = "{guard: Option<Int>, state: First::Ready | First::Closed | First::Other}";
    let observed_final_types = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic
                    .details
                    .to_json()
                    .contains(&format!("\"actual_type\":\"{final_type}\""))
        })
        .count();
    assert_eq!(observed_final_types, 2, "{diagnostics:#?}");

    assert_eq!(
        TypeEnvironment::from_module(&module)
            .function("private_result")
            .expect("private_result function")
            .return_type
            .render(),
        final_type,
    );
}
