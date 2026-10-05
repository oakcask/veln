use super::*;
use crate::types::TypeEnvironment;

#[test]
fn aggregate_refinement_inference_covers_joins_context_and_projection() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "fn ready() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn closed() -> State::Closed\n",
            "  Closed\n",
            "end\n",
            "type Pair<A>\n",
            "  Paired(A, A)\n",
            "end\n",
            "type Container<A>\n",
            "  Built(A, Vec<A>)\n",
            "end\n",
            "type First<A>\n",
            "  Some(A)\n",
            "  Empty\n",
            "end\n",
            "fn forward()\n",
            "  [Ready, Closed]\n",
            "end\n",
            "fn reverse()\n",
            "  [Closed, Ready]\n",
            "end\n",
            "fn complete()\n",
            "  [Failed, Ready, Closed]\n",
            "end\n",
            "fn with_base(value: State)\n",
            "  [Ready, value]\n",
            "end\n",
            "fn optional()\n",
            "  [Option::Some(1), None]\n",
            "end\n",
            "fn table()\n",
            "  {ready(): Closed, closed(): Ready}\n",
            "end\n",
            "fn table_reverse()\n",
            "  {closed(): Ready, ready(): Closed}\n",
            "end\n",
            "fn paired()\n",
            "  Paired(Closed, Ready)\n",
            "end\n",
            "fn paired_reverse()\n",
            "  Paired(Ready, Closed)\n",
            "end\n",
            "fn retained_pair()\n",
            "  Paired(Ready, Ready)\n",
            "end\n",
            "fn mixed_occurrence()\n",
            "  Built(Ready, [Ready])\n",
            "end\n",
            "fn mixed_payload() -> Vec<State::Ready>\n",
            "  match mixed_occurrence()\n",
            "    Built(_, items) => items\n",
            "  end\n",
            "end\n",
            "fn ambiguous_private_result()\n",
            "  Paired(First::Some(1), Empty)\n",
            "end\n",
            "fn payload() -> State::Ready\n",
            "  match retained_pair()\n",
            "    Paired(item, _) => item\n",
            "  end\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> Bool\n",
            "  true\n",
            "end\n",
            "fn projected() -> Vec<State::Ready>\n",
            "  vec_filter([Ready], accept_ready)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let first: Vec<State::Ready | State::Closed> = forward()\n",
            "  let second: Vec<State::Ready | State::Closed> = reverse()\n",
            "  let all: Vec<State> = complete()\n",
            "  let base_join: Vec<State> = with_base(Closed)\n",
            "  let optional_values: Vec<Option<Int>> = optional()\n",
            "  let entries: Dict<State::Ready | State::Closed, State::Ready | State::Closed> = table()\n",
            "  let reverse_entries: Dict<State::Ready | State::Closed, State::Ready | State::Closed> = table_reverse()\n",
            "  let pair: Pair<State::Ready | State::Closed>::Paired = paired()\n",
            "  let reverse_pair: Pair<State::Ready | State::Closed>::Paired = paired_reverse()\n",
            "  let contextual_vec: Vec<State> = [Ready, Closed]\n",
            "  let contextual_dict: Dict<String, State> = {\"ready\": Ready}\n",
            "  let contextual_pair: Pair<State> = Paired(Ready, Closed)\n",
            "  let item: State::Ready = payload()\n",
            "  let items: Vec<State::Ready> = projected()\n",
            "  let nested_items: Vec<State::Ready> = mixed_payload()\n",
            "  let inferred_private: Pair<First<Int>>::Paired = ambiguous_private_result()\n",
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
        ("forward", "Vec<State::Ready | State::Closed>"),
        ("reverse", "Vec<State::Ready | State::Closed>"),
        ("complete", "Vec<State>"),
        ("with_base", "Vec<State>"),
        ("optional", "Vec<Option<Int>>"),
        (
            "table",
            "Dict<State::Ready | State::Closed, State::Ready | State::Closed>",
        ),
        (
            "table_reverse",
            "Dict<State::Ready | State::Closed, State::Ready | State::Closed>",
        ),
        ("paired", "Pair<State::Ready | State::Closed>::Paired"),
        (
            "paired_reverse",
            "Pair<State::Ready | State::Closed>::Paired",
        ),
        ("retained_pair", "Pair<State::Ready>::Paired"),
        ("mixed_occurrence", "Container<State::Ready>::Built"),
        ("ambiguous_private_result", "Pair<First<Int>>::Paired"),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected,
        );
    }
}

#[test]
fn retained_aggregate_refinement_rejections_are_table_driven() {
    for (name, source, rejected_expression, expected, actual, constraint) in [
        (
            "later nested widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = [Ready]\n",
                "  let widened: Vec<State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Vec<State>",
            "Vec<State::Ready>",
            "assignable",
        ),
        (
            "later dictionary value widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = {\"ready\": Ready}\n",
                "  let widened: Dict<String, State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Dict<String, State>",
            "Dict<String, State::Ready>",
            "assignable",
        ),
        (
            "later generic payload widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Pair<A>\n",
                "  Paired(A, A)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = Paired(Ready, Ready)\n",
                "  let widened: Pair<State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Pair<State>",
            "Pair<State::Ready>::Paired",
            "assignable",
        ),
        (
            "mixed direct and nested type parameter",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Container<A>\n",
                "  Built(A, Vec<A>)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Built(Ready, [Closed])\n",
                "end\n",
            ),
            "Closed",
            "State::Ready",
            "State::Closed",
            "list_element",
        ),
        (
            "mixed nested and direct type parameter",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Container<A>\n",
                "  Built(Vec<A>, A)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Built([Ready], Closed)\n",
                "end\n",
            ),
            "Closed",
            "State::Ready",
            "State::Closed",
            "call_argument",
        ),
        (
            "generic argument mismatch",
            concat!(
                "fn int_some() -> Option<Int>::Some\n",
                "  Some(1)\n",
                "end\n",
                "fn string_some() -> Option<String>::Some\n",
                "  Some(\"wrong\")\n",
                "end\n",
                "fn main() -> ()\n",
                "  let values = [int_some(), string_some()]\n",
                "end\n",
            ),
            "string_some()",
            "Option<Int>::Some",
            "Option<String>::Some",
            "list_element",
        ),
        (
            "different resolved ADTs",
            concat!(
                "type First\n",
                "  Value\n",
                "  Other\n",
                "end\n",
                "type Second\n",
                "  Value\n",
                "  Other\n",
                "end\n",
                "fn first() -> First::Value\n",
                "  First::Value\n",
                "end\n",
                "fn second() -> Second::Value\n",
                "  Second::Value\n",
                "end\n",
                "fn main() -> ()\n",
                "  let values = [first(), second()]\n",
                "end\n",
            ),
            "second()",
            "First::Value",
            "Second::Value",
            "list_element",
        ),
        (
            "repeated generic payload mismatch",
            concat!(
                "type Pair<A>\n",
                "  Paired(A, A)\n",
                "end\n",
                "fn int_some() -> Option<Int>::Some\n",
                "  Some(1)\n",
                "end\n",
                "fn string_some() -> Option<String>::Some\n",
                "  Some(\"wrong\")\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Paired(int_some(), string_some())\n",
                "end\n",
            ),
            "string_some()",
            "Option<Int>::Some",
            "Option<String>::Some",
            "call_argument",
        ),
    ] {
        let source_file = SourceFile::new("main.veln", source);
        let parsed = parse(&source_file);
        assert!(
            parsed.diagnostics.is_empty(),
            "{name}: {:#?}",
            parsed.diagnostics
        );
        let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
        assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:#?}");
        let mismatch = &diagnostics[0];
        assert_eq!(mismatch.id, "type.mismatch", "{name}: {diagnostics:#?}");
        assert_eq!(
            mismatch.message.to_string(),
            format!("expected `{expected}`, but found `{actual}`"),
            "{name}: {diagnostics:#?}"
        );
        let span = mismatch.span.as_ref().expect("mismatch span");
        assert_eq!(
            &source[span.start.offset..span.end.offset],
            rejected_expression,
            "{name}: {span:#?}"
        );
        let details = mismatch.details.to_json();
        assert!(
            details.contains(&format!("\"expected_type\":\"{expected}\"")),
            "{name}: {details}"
        );
        assert!(
            details.contains(&format!("\"actual_type\":\"{actual}\"")),
            "{name}: {details}"
        );
        assert!(
            details.contains(&format!("\"constraint\":\"{constraint}\"")),
            "{name}: {details}"
        );
    }
}
