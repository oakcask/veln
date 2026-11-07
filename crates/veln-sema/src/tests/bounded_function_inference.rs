use super::*;
use crate::types::TypeEnvironment;

fn analyze(source_text: &str) -> (SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

fn result(environment: &TypeEnvironment, name: &str) -> String {
    environment
        .function(name)
        .unwrap_or_else(|| panic!("missing function {name}"))
        .return_type
        .render()
}

#[test]
fn structural_function_joins_are_order_independent_across_bounded_components() {
    let source = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "fn refinement_forward(first: fn(State) -> Int, second: fn(State::Ready) -> Int effects [stdio])\n  [first, second]\nend\n",
        "fn refinement_reverse(first: fn(State) -> Int, second: fn(State::Ready) -> Int effects [stdio])\n  [second, first]\nend\n",
        "fn width_forward(first: fn({id: Int}) -> Int, second: fn({id: Int, label: String}) -> Int)\n  [first, second]\nend\n",
        "fn width_reverse(first: fn({id: Int}) -> Int, second: fn({id: Int, label: String}) -> Int)\n  [second, first]\nend\n",
        "fn nested_effect_forward(first: fn(fn() -> Int) -> Int, second: fn(fn() -> Int effects [stdio]) -> Int)\n  [first, second]\nend\n",
        "fn nested_effect_reverse(first: fn(fn() -> Int) -> Int, second: fn(fn() -> Int effects [stdio]) -> Int)\n  [second, first]\nend\n",
        "fn variadic_forward(first: fn(Int, ...State) -> Int, second: fn(Int, ...State::Ready) -> Int)\n  [first, second]\nend\n",
        "fn variadic_reverse(first: fn(Int, ...State) -> Int, second: fn(Int, ...State::Ready) -> Int)\n  [second, first]\nend\n",
        "fn alias_forward(first: fn(First::Ready) -> Int, second: fn(Second::Ready) -> Int)\n  [first, second]\nend\n",
        "fn alias_reverse(first: fn(First::Ready) -> Int, second: fn(Second::Ready) -> Int)\n  [second, first]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    for name in ["refinement_forward", "refinement_reverse"] {
        assert_eq!(
            result(&environment, name),
            "Vec<fn(State::Ready) -> Int effects [stdio]>"
        );
    }
    for name in ["width_forward", "width_reverse"] {
        assert_eq!(
            result(&environment, name),
            "Vec<fn({id: Int, label: String}) -> Int>"
        );
    }
    for name in ["nested_effect_forward", "nested_effect_reverse"] {
        assert_eq!(result(&environment, name), "Vec<fn(fn() -> Int) -> Int>");
    }
    for name in ["variadic_forward", "variadic_reverse"] {
        assert_eq!(
            result(&environment, name),
            "Vec<fn(Int, ...State::Ready) -> Int>"
        );
    }
    for name in ["alias_forward", "alias_reverse"] {
        assert_eq!(result(&environment, name), "Vec<fn(State::Ready) -> Int>");
    }
}

#[test]
fn fallback_retains_every_contribution_until_a_unique_candidate_exists() {
    let permutations = [
        ["first", "second", "fallback"],
        ["first", "fallback", "second"],
        ["second", "first", "fallback"],
        ["second", "fallback", "first"],
        ["fallback", "first", "second"],
        ["fallback", "second", "first"],
    ];
    let mut source = String::new();
    for (index, order) in permutations.iter().enumerate() {
        source.push_str(&format!(
            "fn permutation_{index}(first: fn() -> {{id: Int, left: String}}, second: fn() -> {{id: Int, right: Bool}}, fallback: fn() -> {{id: Int}})\n  [{}, {}, {}]\nend\n",
            order[0], order[1], order[2]
        ));
    }
    let (module, diagnostics) = analyze(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    for index in 0..permutations.len() {
        assert_eq!(
            result(&environment, &format!("permutation_{index}")),
            "Vec<fn() -> {id: Int}>"
        );
    }
}

#[test]
fn recursive_shape_rejection_does_not_change_later_successful_join() {
    let source_text = concat!(
        "fn retained(first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int) -> ()\n",
        "  let values = [first, rejected, later]\n  ()\nend\n",
    );
    let (module, diagnostics) = analyze(source_text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "rejected");
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(result(&environment, "retained"), "()");
}

#[test]
fn disjoint_refinement_meet_requires_context_without_partial_function_type() {
    let source = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "fn unavailable(first: fn(State::Ready) -> Int, second: fn(State::Closed) -> Int)\n  [first, second]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.private_inference_incomplete");
    let inferred = diagnostics[0]
        .details
        .as_object()
        .and_then(|details| details.iter().find(|(key, _)| key == "inferred_type"))
        .map(|(_, value)| value);
    assert_eq!(
        inferred,
        Some(&veln_diagnostics::JsonValue::string("unknown"))
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(result(&environment, "unavailable"), "unknown");
}
