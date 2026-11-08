use super::*;
use crate::types::TypeEnvironment;

#[test]
fn nested_constructor_constraints_use_composed_polarity_in_both_consumers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type Positive<T>\n",
            "  PositiveValue({left: T, right: T})\n",
            "end\n",
            "type Negative<T>\n",
            "  NegativeValue(fn(T) -> (), fn(T) -> ())\n",
            "end\n",
            "type Exact<T>\n",
            "  ExactValue(fn(T) -> T)\n",
            "end\n",
            "fn accepts_state(value: State) -> ()\n  ()\nend\n",
            "fn accepts_ready(value: State::Ready) -> ()\n  ()\nend\n",
            "fn ready_identity(value: State::Ready) -> State::Ready\n  value\nend\n",
            "fn positive_private()\n",
            "  PositiveValue({left: Ready, right: Closed})\n",
            "end\n",
            "fn positive_private_reverse()\n",
            "  PositiveValue({left: Closed, right: Ready})\n",
            "end\n",
            "fn negative_private(first: fn(State) -> (), second: fn(State::Ready) -> ())\n",
            "  NegativeValue(first, second)\n",
            "end\n",
            "fn exact_private(callback: fn(State::Ready) -> State::Ready)\n",
            "  ExactValue(callback)\n",
            "end\n",
            "fn needs_positive(value: Positive<State::Ready | State::Closed>) -> ()\n  ()\nend\n",
            "fn needs_negative(value: Negative<State::Ready>) -> ()\n  ()\nend\n",
            "fn ordinary() -> ()\n",
            "  needs_positive(PositiveValue({left: Ready, right: Closed}))\n",
            "  needs_negative(NegativeValue(accepts_state, accepts_ready))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    for function in ["positive_private", "positive_private_reverse"] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            "Positive<State::Ready | State::Closed>::PositiveValue"
        );
    }
    assert_eq!(
        environment
            .function("negative_private")
            .unwrap()
            .return_type
            .render(),
        "Negative<State::Ready>::NegativeValue"
    );
    assert_eq!(
        environment
            .function("exact_private")
            .unwrap()
            .return_type
            .render(),
        "Exact<State::Ready>::ExactValue"
    );
}

#[test]
fn irrelevant_occurrences_need_context_and_invariant_occurrences_roll_back() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\nend\n",
            "type Phantom<T>\n  Empty\nend\n",
            "type Irrelevant<T>\n  IrrelevantValue(Phantom<T>)\nend\n",
            "type Exact<T>\n  ExactValue(fn(T) -> T, T)\nend\n",
            "fn ready_identity(value: State::Ready) -> State::Ready\n  value\nend\n",
            "fn unconstrained(value: Phantom<State::Ready>) -> ()\n",
            "  let missing = IrrelevantValue(value)\n",
            "end\n",
            "fn contextual(value: Phantom<State::Ready>) -> Irrelevant<State::Closed>\n",
            "  IrrelevantValue(value)\n",
            "end\n",
            "fn rollback(callback: fn(State::Ready) -> State::Ready, rejected: State::Closed)\n",
            "  ExactValue(callback, rejected)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.inference_ambiguous")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("contextual")
            .unwrap()
            .return_type
            .render(),
        "Irrelevant<State::Closed>"
    );
    assert_eq!(
        environment
            .function("rollback")
            .unwrap()
            .return_type
            .render(),
        "Exact<State::Ready>::ExactValue"
    );
}

#[test]
fn mixed_bounds_select_one_identity_and_missing_bounds_remain_ambiguous() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\n  Failed\nend\n",
            "type Mixed<T>\n  MixedValue(T, fn(T) -> (), fn(T) -> T)\nend\n",
            "type Pair<T>\n  Paired(T, T)\nend\n",
            "type Upper<T>\n  UpperValue(fn(T) -> (), fn(T) -> ())\nend\n",
            "fn accepts_state(value: State) -> ()\n  ()\nend\n",
            "fn accepts_int(value: Int) -> ()\n  ()\nend\n",
            "fn accepts_string(value: String) -> ()\n  ()\nend\n",
            "fn ready_identity(value: State::Ready) -> State::Ready\n  value\nend\n",
            "fn mixed_private()\n",
            "  MixedValue(Ready, accepts_state, ready_identity)\n",
            "end\n",
            "fn unavailable_join() -> ()\n",
            "  let value = Paired(1, \"text\")\n",
            "end\n",
            "fn unavailable_meet() -> ()\n",
            "  let value = UpperValue(accepts_int, accepts_string)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.inference_ambiguous")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("mixed_private")
            .unwrap()
            .return_type
            .render(),
        "Mixed<State::Ready>::MixedValue"
    );
}

#[test]
fn missing_nested_variance_metadata_falls_back_to_equality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\nend\n",
            "type Carrier<T>\n  Carried(T)\nend\n",
            "type Outer<T>\n  OuterValue(Carrier<T>, Carrier<T>)\nend\n",
            "fn rejected(value: Carrier<State::Ready>, later: Carrier<State::Closed>) -> ()\n",
            "  let result = OuterValue(value, later)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let mut environment = TypeEnvironment::from_module(&module);
    let identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Carrier")
        .expect("carrier descriptor")
        .identity();
    environment
        .adts
        .clear_parameter_variances_for_identity(&identity);
    let diagnostics =
        crate::pipeline::analyze_surface_module_with_environment_for_test(&module, &environment);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
}
