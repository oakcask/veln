use super::*;
use crate::types::TypeEnvironment;

fn assert_same_inferred_result(
    environment: &TypeEnvironment,
    first_name: &str,
    second_name: &str,
    expected_render: &str,
) {
    let first = &environment
        .function(first_name)
        .unwrap_or_else(|| panic!("missing function {first_name}"))
        .return_type;
    let second = &environment
        .function(second_name)
        .unwrap_or_else(|| panic!("missing function {second_name}"))
        .return_type;
    assert!(
        crate::type_relations::invariant_args_match(
            std::slice::from_ref(first),
            std::slice::from_ref(second),
        ),
        "permutations resolved to different complete identities: {first:?} != {second:?}",
    );
    assert_eq!(
        first.render(),
        second.render(),
        "presentation changed by order"
    );
    assert_eq!(
        first.render(),
        expected_render,
        "unexpected result for {first_name}: {first:?}"
    );
    assert_eq!(
        second.render(),
        expected_render,
        "unexpected result for {second_name}: {second:?}"
    );
}

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
            "type Covariant<T>\n",
            "  CovariantValue(T)\n",
            "end\n",
            "type Contravariant<T>\n",
            "  ContravariantValue(fn(T) -> ())\n",
            "end\n",
            "type NamedPositive<T>\n",
            "  NamedPositiveValue(Covariant<T>, Covariant<T>)\n",
            "end\n",
            "type NamedNegative<T>\n",
            "  NamedNegativeValue(Contravariant<T>, Contravariant<T>)\n",
            "end\n",
            "type VariadicNegative<T>\n",
            "  VariadicNegativeValue(fn(...T) -> (), fn(...T) -> ())\n",
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
            "fn named_positive_private()\n",
            "  NamedPositiveValue(CovariantValue(Ready), CovariantValue(Closed))\n",
            "end\n",
            "fn named_negative_private(first: Contravariant<State>, second: Contravariant<State::Ready>)\n",
            "  NamedNegativeValue(first, second)\n",
            "end\n",
            "fn variadic_negative_private(first: fn(...State) -> (), second: fn(...State::Ready) -> ())\n",
            "  VariadicNegativeValue(first, second)\n",
            "end\n",
            "fn needs_positive(value: Positive<State::Ready | State::Closed>) -> ()\n  ()\nend\n",
            "fn needs_negative(value: Negative<State::Ready>) -> ()\n  ()\nend\n",
            "fn needs_named_positive(value: NamedPositive<State::Ready | State::Closed>) -> ()\n  ()\nend\n",
            "fn needs_named_negative(value: NamedNegative<State::Ready>) -> ()\n  ()\nend\n",
            "fn needs_variadic_negative(value: VariadicNegative<State::Ready>) -> ()\n  ()\nend\n",
            "fn ordinary(first: Contravariant<State>, second: Contravariant<State::Ready>, variadic_state: fn(...State) -> (), variadic_ready: fn(...State::Ready) -> ()) -> ()\n",
            "  needs_positive(PositiveValue({left: Ready, right: Closed}))\n",
            "  needs_negative(NegativeValue(accepts_state, accepts_ready))\n",
            "  needs_named_positive(NamedPositiveValue(CovariantValue(Ready), CovariantValue(Closed)))\n",
            "  needs_named_negative(NamedNegativeValue(first, second))\n",
            "  needs_variadic_negative(VariadicNegativeValue(variadic_state, variadic_ready))\n",
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
    assert_eq!(
        environment
            .function("named_positive_private")
            .unwrap()
            .return_type
            .render(),
        "NamedPositive<State::Ready | State::Closed>::NamedPositiveValue"
    );
    assert_eq!(
        environment
            .function("named_negative_private")
            .unwrap()
            .return_type
            .render(),
        "NamedNegative<State::Ready>::NamedNegativeValue"
    );
    assert_eq!(
        environment
            .function("variadic_negative_private")
            .unwrap()
            .return_type
            .render(),
        "VariadicNegative<State::Ready>::VariadicNegativeValue"
    );
}

#[test]
fn ordinary_context_free_constraints_are_observed_after_construction() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\n  Failed\nend\n",
            "type Positive<T>\n  PositiveValue({left: T, right: T})\nend\n",
            "type Negative<T>\n  NegativeValue(fn(T) -> (), fn(T) -> ())\nend\n",
            "type Exact<T>\n  ExactValue(fn(T) -> T, T)\nend\n",
            "type Phantom<T>\n  Empty\nend\n",
            "type Irrelevant<T>\n  IrrelevantValue(Phantom<T>)\nend\n",
            "type Mixed<T>\n  MixedValue(T, fn(T) -> (), fn(T) -> T)\nend\n",
            "fn accepts_state(value: State) -> ()\n  ()\nend\n",
            "fn accepts_ready(value: State::Ready) -> ()\n  ()\nend\n",
            "fn ready_identity(value: State::Ready) -> State::Ready\n  value\nend\n",
            "fn ordinary(value: Phantom<State::Ready>) -> ()\n",
            "  let positive = PositiveValue({left: Ready, right: Closed})\n",
            "  let PositiveValue(payload) = positive\n",
            "  match payload.left\n",
            "    Ready => ()\n",
            "    Closed => ()\n",
            "  end\n",
            "  let negative = NegativeValue(accepts_state, accepts_ready)\n",
            "  let NegativeValue(first, second) = negative\n",
            "  let first_ready: fn(State::Ready) -> () = first\n",
            "  let second_ready: fn(State::Ready) -> () = second\n",
            "  first_ready(Ready)\n",
            "  second_ready(Ready)\n",
            "  let exact = ExactValue(ready_identity, Ready)\n",
            "  let ExactValue(_, exact_value) = exact\n",
            "  match exact_value\n",
            "    Ready => ()\n",
            "  end\n",
            "  let mixed = MixedValue(Ready, accepts_state, ready_identity)\n",
            "  let MixedValue(mixed_value, _, _) = mixed\n",
            "  match mixed_value\n",
            "    Ready => ()\n",
            "  end\n",
            "  let missing = IrrelevantValue(value)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.inference_ambiguous");
}

#[test]
fn successful_constraint_permutations_preserve_identity_and_presentation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\nend\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "type Lower<T>\n  LowerValue(T, T)\nend\n",
            "type Upper<T>\n  UpperValue(fn(T) -> (), fn(T) -> ())\nend\n",
            "type Exact<T>\n  ExactValue(fn(T) -> T, fn(T) -> T)\nend\n",
            "type Mixed<T>\n",
            "  MixedValue(T, T, fn(T) -> (), fn(T) -> (), fn(T) -> T, fn(T) -> T)\n",
            "end\n",
            "fn accepts_state(value: State) -> ()\n  ()\nend\n",
            "fn accepts_ready(value: State::Ready) -> ()\n  ()\nend\n",
            "fn first_identity(value: First::Ready) -> First::Ready\n  value\nend\n",
            "fn second_identity(value: Second::Ready) -> Second::Ready\n  value\nend\n",
            "fn lower_forward()\n  LowerValue(First::Ready, Second::Closed)\nend\n",
            "fn lower_reverse()\n  LowerValue(Second::Closed, First::Ready)\nend\n",
            "fn upper_forward(first: fn(State) -> (), second: fn(State::Ready) -> ())\n",
            "  UpperValue(first, second)\n",
            "end\n",
            "fn upper_reverse(first: fn(State) -> (), second: fn(State::Ready) -> ())\n",
            "  UpperValue(second, first)\n",
            "end\n",
            "fn exact_forward(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready)\n",
            "  ExactValue(first, second)\n",
            "end\n",
            "fn exact_reverse(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready)\n",
            "  ExactValue(second, first)\n",
            "end\n",
            "fn mixed_forward(upper: fn(State) -> (), narrow: fn(State::Ready) -> (), first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready)\n",
            "  MixedValue(First::Ready, Second::Ready, upper, narrow, first, second)\n",
            "end\n",
            "fn mixed_reverse(upper: fn(State) -> (), narrow: fn(State::Ready) -> (), first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready)\n",
            "  MixedValue(Second::Ready, First::Ready, narrow, upper, second, first)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    assert_same_inferred_result(
        &environment,
        "lower_forward",
        "lower_reverse",
        "Lower<State>::LowerValue",
    );
    assert_same_inferred_result(
        &environment,
        "upper_forward",
        "upper_reverse",
        "Upper<State::Ready>::UpperValue",
    );
    assert_same_inferred_result(
        &environment,
        "exact_forward",
        "exact_reverse",
        "Exact<State::Ready>::ExactValue",
    );
    assert_same_inferred_result(
        &environment,
        "mixed_forward",
        "mixed_reverse",
        "Mixed<State::Ready>::MixedValue",
    );
}

#[test]
fn unresolved_structural_lower_bounds_resolve_in_every_permutation() {
    let mut source_text = concat!(
        "type State\n  Ready\n  Closed\n  Failed\nend\n",
        "type Box<T>\n  Boxed(T)\nend\n",
        "type Mixed<T>\n  Made({upper: fn(T) -> (), first: Box<T>, second: Box<T>, third: Box<T>})\nend\n",
        "fn f(value: {a: Int}) -> State::Ready\n  Ready\nend\n",
        "fn g(value: {b: Int}) -> State::Closed\n  Closed\nend\n",
        "fn h(value: {a: Int, b: Int}) -> State::Ready\n  Ready\nend\n",
        "fn accepts_complete(value: fn({a: Int, b: Int}) -> State) -> ()\n  ()\nend\n",
        "fn needs_complete(value: Mixed<fn({a: Int, b: Int}) -> State::Ready | State::Closed>) -> ()\n  ()\nend\n",
    )
    .to_owned();
    let permutations = [
        ["f", "g", "h"],
        ["f", "h", "g"],
        ["g", "f", "h"],
        ["g", "h", "f"],
        ["h", "f", "g"],
        ["h", "g", "f"],
    ];
    for (index, order) in permutations.iter().enumerate() {
        let order = order.map(|name| match name {
            "f" => "fn({a: Int}) -> State::Ready",
            "g" => "fn({b: Int}) -> State::Closed",
            "h" => "fn({a: Int, b: Int}) -> State::Ready",
            _ => unreachable!(),
        });
        source_text.push_str(&format!(
            "fn private_{index}(payload: {{upper: fn(fn({{a: Int, b: Int}}) -> State) -> (), first: Box<{}>, second: Box<{}>, third: Box<{}>}})\n  Made(payload)\nend\n",
            order[0], order[1], order[2],
        ));
    }
    source_text.push_str("fn ordinary(");
    for (index, order) in permutations.iter().enumerate() {
        let order = order.map(|name| match name {
            "f" => "fn({a: Int}) -> State::Ready",
            "g" => "fn({b: Int}) -> State::Closed",
            "h" => "fn({a: Int, b: Int}) -> State::Ready",
            _ => unreachable!(),
        });
        source_text.push_str(&format!(
            "payload_{index}: {{upper: fn(fn({{a: Int, b: Int}}) -> State) -> (), first: Box<{}>, second: Box<{}>, third: Box<{}>}}, ",
            order[0], order[1], order[2],
        ));
    }
    source_text.push_str(") -> ()\n");
    for index in 0..permutations.len() {
        source_text.push_str(&format!(
            "  let value_{index} = Made(payload_{index})\n  needs_complete(value_{index})\n",
        ));
    }
    source_text.push_str("end\n");

    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    let expected = "Mixed<fn({a: Int, b: Int}) -> State::Ready | State::Closed>::Made";
    for index in 0..permutations.len() {
        let function = environment
            .function(&format!("private_{index}"))
            .expect("private permutation function");
        assert_eq!(function.return_type.render(), expected);
        if index > 0 {
            assert_same_inferred_result(
                &environment,
                "private_0",
                &format!("private_{index}"),
                expected,
            );
        }
    }
}

#[test]
fn structural_bound_transition_keeps_explicit_and_ambiguity_controls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\n  Failed\nend\n",
            "type Box<T>\n  Boxed(T)\nend\n",
            "type Mixed<T>\n  Made({upper: fn(T) -> (), first: Box<T>, second: Box<T>, third: Box<T>})\nend\n",
            "fn explicit(payload: {upper: fn(fn({a: Int, b: Int}) -> State) -> (), first: Box<fn({a: Int}) -> State::Ready>, second: Box<fn({b: Int}) -> State::Closed>, third: Box<fn({a: Int}) -> State::Ready>}) -> Mixed<fn({a: Int, b: Int}) -> State::Ready | State::Closed>\n",
            "  Made(payload)\n",
            "end\n",
            "fn unavailable(payload: {upper: fn(fn({a: Int, b: Int}) -> State) -> (), first: Box<fn({a: Int}) -> State::Ready>, second: Box<fn({b: Int}) -> State::Closed>, third: Box<fn({a: Int}) -> State::Ready>})\n",
            "  Made(payload)\n",
            "end\n",
            "fn ordinary(payload: {upper: fn(fn({a: Int, b: Int}) -> State) -> (), first: Box<fn({a: Int}) -> State::Ready>, second: Box<fn({b: Int}) -> State::Closed>, third: Box<fn({a: Int}) -> State::Ready>}) -> ()\n",
            "  let exact: Mixed<fn({a: Int, b: Int}) -> State::Ready | State::Closed> = Made(payload)\n",
            "  let missing = Made(payload)\n",
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
        "{diagnostics:#?}",
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.private_inference_incomplete")
            .count(),
        1,
        "{diagnostics:#?}",
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("explicit")
            .unwrap()
            .return_type
            .render(),
        "Mixed<fn({a: Int, b: Int}) -> State::Ready | State::Closed>",
    );
}

#[test]
fn post_conflict_alias_permutations_are_canonical_in_both_consumers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\nend\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "type Other\n  OtherValue\nend\n",
            "type Invariant<T>\n  InvariantValue(fn(T) -> T)\nend\n",
            "type Lower<T>\n  LowerValue(T, T, T)\nend\n",
            "type Upper<T>\n  UpperValue(fn(T) -> (), fn(T) -> (), fn(T) -> ())\nend\n",
            "type Exact<T>\n  ExactValue({a: Invariant<T>, b: Invariant<T>, c: Invariant<T>})\nend\n",
            "type Mixed<T>\n  MixedValue(T, T, T, fn(T) -> (), fn(T) -> (), fn(T) -> ())\nend\n",
            "fn accepts_first(value: First::Ready) -> ()\n  ()\nend\n",
            "fn accepts_second(value: Second::Ready) -> ()\n  ()\nend\n",
            "fn first_identity(value: First::Ready) -> First::Ready\n  value\nend\n",
            "fn second_identity(value: Second::Ready) -> Second::Ready\n  value\nend\n",
            "fn private_lower_after_conflict()\n  LowerValue(First::Ready, Second::Ready, First::Ready)\nend\n",
            "fn private_lower_late_conflict()\n  LowerValue(First::Ready, First::Ready, Second::Ready)\nend\n",
            "fn private_upper_after_conflict(first: fn(First::Ready) -> (), second: fn(Second::Ready) -> ())\n  UpperValue(first, second, first)\nend\n",
            "fn private_upper_late_conflict(first: fn(First::Ready) -> (), second: fn(Second::Ready) -> ())\n  UpperValue(first, first, second)\nend\n",
            "fn private_exact_after_conflict(values: {a: Invariant<First::Ready>, b: Invariant<Second::Ready>, c: Invariant<First::Ready>})\n  ExactValue(values)\nend\n",
            "fn private_exact_late_conflict(values: {a: Invariant<First::Ready>, b: Invariant<First::Ready>, c: Invariant<Second::Ready>})\n  ExactValue(values)\nend\n",
            "fn private_mixed_after_conflict(first: fn(First::Ready) -> (), second: fn(Second::Ready) -> ())\n  MixedValue(First::Ready, Second::Ready, First::Ready, first, second, first)\nend\n",
            "fn private_mixed_late_conflict(first: fn(First::Ready) -> (), second: fn(Second::Ready) -> ())\n  MixedValue(First::Ready, First::Ready, Second::Ready, first, first, second)\nend\n",
            "fn needs_lower_other(value: Lower<Other>) -> ()\n  ()\nend\n",
            "fn needs_upper_other(value: Upper<Other>) -> ()\n  ()\nend\n",
            "fn needs_exact_other(value: Exact<Other>) -> ()\n  ()\nend\n",
            "fn needs_mixed_other(value: Mixed<Other>) -> ()\n  ()\nend\n",
            "fn ordinary(exact_after_values: {a: Invariant<First::Ready>, b: Invariant<Second::Ready>, c: Invariant<First::Ready>}, exact_late_values: {a: Invariant<First::Ready>, b: Invariant<First::Ready>, c: Invariant<Second::Ready>}) -> ()\n",
            "  let lower_after = LowerValue(First::Ready, Second::Ready, First::Ready)\n",
            "  let lower_late = LowerValue(First::Ready, First::Ready, Second::Ready)\n",
            "  let upper_after = UpperValue(accepts_first, accepts_second, accepts_first)\n",
            "  let upper_late = UpperValue(accepts_first, accepts_first, accepts_second)\n",
            "  let exact_after = ExactValue(exact_after_values)\n",
            "  let exact_late = ExactValue(exact_late_values)\n",
            "  let mixed_after = MixedValue(First::Ready, Second::Ready, First::Ready, accepts_first, accepts_second, accepts_first)\n",
            "  let mixed_late = MixedValue(First::Ready, First::Ready, Second::Ready, accepts_first, accepts_first, accepts_second)\n",
            "  needs_lower_other(lower_after)\n",
            "  needs_lower_other(lower_late)\n",
            "  needs_upper_other(upper_after)\n",
            "  needs_upper_other(upper_late)\n",
            "  needs_exact_other(exact_after)\n",
            "  needs_exact_other(exact_late)\n",
            "  needs_mixed_other(mixed_after)\n",
            "  needs_mixed_other(mixed_late)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 8, "{diagnostics:#?}");
    let actual_types = diagnostics
        .iter()
        .map(|diagnostic| {
            assert_eq!(diagnostic.id, "type.mismatch", "{diagnostics:#?}");
            diagnostic
                .details
                .as_object()
                .and_then(|details| details.iter().find(|(key, _)| key == "actual_type"))
                .map(|(_, value)| value.clone())
                .expect("ordinary mismatch actual type")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual_types,
        vec![
            veln_diagnostics::JsonValue::string("Lower<State::Ready>::LowerValue"),
            veln_diagnostics::JsonValue::string("Lower<State::Ready>::LowerValue"),
            veln_diagnostics::JsonValue::string("Upper<State::Ready>::UpperValue"),
            veln_diagnostics::JsonValue::string("Upper<State::Ready>::UpperValue"),
            veln_diagnostics::JsonValue::string("Exact<State::Ready>::ExactValue"),
            veln_diagnostics::JsonValue::string("Exact<State::Ready>::ExactValue"),
            veln_diagnostics::JsonValue::string("Mixed<State::Ready>::MixedValue"),
            veln_diagnostics::JsonValue::string("Mixed<State::Ready>::MixedValue"),
        ]
    );

    let environment = TypeEnvironment::from_module(&module);
    for (first, second, expected) in [
        (
            "private_lower_after_conflict",
            "private_lower_late_conflict",
            "Lower<State::Ready>::LowerValue",
        ),
        (
            "private_upper_after_conflict",
            "private_upper_late_conflict",
            "Upper<State::Ready>::UpperValue",
        ),
        (
            "private_exact_after_conflict",
            "private_exact_late_conflict",
            "Exact<State::Ready>::ExactValue",
        ),
        (
            "private_mixed_after_conflict",
            "private_mixed_late_conflict",
            "Mixed<State::Ready>::MixedValue",
        ),
    ] {
        assert_same_inferred_result(&environment, first, second, expected);
    }
}

#[test]
fn explicit_context_resolves_otherwise_unavailable_bounds_in_both_consumers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\nend\n",
            "type Phantom<T>\n  Empty\nend\n",
            "type Pair<T>\n  Paired(T, T)\nend\n",
            "type Upper<T>\n  UpperValue(fn(T) -> (), fn(T) -> ())\nend\n",
            "fn accepts_int(value: Phantom<Int>) -> ()\n  ()\nend\n",
            "fn accepts_string(value: Phantom<String>) -> ()\n  ()\nend\n",
            "fn private_join(first: Phantom<Int>, second: Phantom<String>)\n",
            "  Paired(first, second)\n",
            "end\n",
            "fn private_meet()\n",
            "  UpperValue(accepts_int, accepts_string)\n",
            "end\n",
            "fn needs_pair(value: Pair<Phantom<State::Ready>>) -> ()\n  ()\nend\n",
            "fn needs_upper(value: Upper<Phantom<State::Ready>>) -> ()\n  ()\nend\n",
            "pub fn contextual(first: Phantom<Int>, second: Phantom<String>) -> ()\n",
            "  let joined: Pair<Phantom<State::Ready>> = Paired(first, second)\n",
            "  let met: Upper<Phantom<State::Ready>> = UpperValue(accepts_int, accepts_string)\n",
            "  needs_pair(joined)\n",
            "  needs_upper(met)\n",
            "  needs_pair(private_join(first, second))\n",
            "  needs_upper(private_meet())\n",
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
fn rejected_payloads_preserve_constraints_for_later_accepted_payloads() {
    let source_text = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "type Exact<T>\n  ExactValue(fn(T) -> T, fn(T) -> T, fn(T) -> T, fn(T) -> T)\nend\n",
        "fn first_identity(value: First::Ready) -> First::Ready\n  value\nend\n",
        "fn closed_identity(value: State::Closed) -> State::Closed\n  value\nend\n",
        "fn second_identity(value: Second::Ready) -> Second::Ready\n  value\nend\n",
        "type Other\n  OtherValue\nend\n",
        "fn needs_other(value: Exact<Other>) -> ()\n  ()\nend\n",
        "fn private_transaction(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready, rejected: fn(State::Closed) -> State::Closed)\n",
        "  ExactValue(first, second, rejected, first)\n",
        "end\n",
        "fn private_transaction_late(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready, rejected: fn(State::Closed) -> State::Closed)\n",
        "  ExactValue(first, first, rejected, second)\n",
        "end\n",
        "fn ordinary_transaction(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready, rejected: fn(State::Closed) -> State::Closed) -> ()\n",
        "  let retained = ExactValue(first, second, rejected, first)\n",
        "  needs_other(retained)\n",
        "end\n",
        "fn ordinary_transaction_late(first: fn(First::Ready) -> First::Ready, second: fn(Second::Ready) -> Second::Ready, rejected: fn(State::Closed) -> State::Closed) -> ()\n",
        "  let retained = ExactValue(first, first, rejected, second)\n",
        "  needs_other(retained)\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.span.as_ref().is_some_and(|span| {
                &source_text[span.start.offset..span.end.offset] == "rejected"
            }))
            .count(),
        4,
        "{diagnostics:#?}",
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter_map(|diagnostic| {
                diagnostic.details.as_object().and_then(|details| {
                    details
                        .iter()
                        .find(|(key, _)| key == "actual_type")
                        .map(|(_, value)| value)
                })
            })
            .filter(|actual| {
                **actual == veln_diagnostics::JsonValue::string("Exact<State::Ready>::ExactValue")
            })
            .count(),
        2,
        "{diagnostics:#?}",
    );

    let environment = TypeEnvironment::from_module(&module);
    assert_same_inferred_result(
        &environment,
        "private_transaction",
        "private_transaction_late",
        "Exact<State::Ready>::ExactValue",
    );
}

#[test]
fn rejected_concrete_payload_shapes_preserve_constraints_in_both_consumers() {
    let source_text = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "type Other\n  OtherValue\nend\n",
        "type Box<T>\n  Boxed(T)\nend\n",
        "type Outer<T>\n  Out(T, Box<T>, T)\nend\n",
        "type Structured<T>\n  Shaped(T, {guard: Option<Int>, value: T}, T)\nend\n",
        "fn private_named(rejected: Other)\n",
        "  Out(Ready, rejected, Closed)\n",
        "end\n",
        "fn private_structural(rejected: {wrong: String})\n",
        "  Shaped(Ready, rejected, Closed)\n",
        "end\n",
        "fn ordinary_named(rejected: Other) -> Outer<State::Ready | State::Closed>\n",
        "  Out(Ready, rejected, Closed)\n",
        "end\n",
        "fn ordinary_structural(rejected: {wrong: String}) -> Structured<State::Ready | State::Closed>\n",
        "  Shaped(Ready, rejected, Closed)\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.span.as_ref().is_some_and(|span| {
                    &source_text[span.start.offset..span.end.offset] == "rejected"
                })
        }),
        "{diagnostics:#?}",
    );

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("private_named")
            .unwrap()
            .return_type
            .render(),
        "Outer<State>::Out",
    );
    assert_eq!(
        environment
            .function("private_structural")
            .unwrap()
            .return_type
            .render(),
        "Structured<State>::Shaped",
    );
}

#[test]
fn equality_constraints_keep_irrelevant_named_arguments_in_complete_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Phantom<T>\n  Empty\nend\n",
            "type Duo<A, B>\n  Paired(fn(A) -> A, fn(B) -> B)\nend\n",
            "type Outer<T>\n  Out(T, Duo<T, T>, T)\nend\n",
            "fn ordinary(first: Phantom<Int>, rejected: Duo<Phantom<Int>, Phantom<String>>, later: Phantom<Int>) -> Outer<Phantom<Int>>\n",
            "  Out(first, rejected, later)\n",
            "end\n",
            "fn private(first: Phantom<Int>, rejected: Duo<Phantom<Int>, Phantom<String>>, later: Phantom<Int>)\n",
            "  Out(first, rejected, later)\n",
            "end\n",
        ),
    );
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

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("private")
            .unwrap()
            .return_type
            .render(),
        "Outer<Phantom<Int>>::Out",
        "the rejected nested payload must preserve the earlier constraint and allow the later payload to continue",
    );
}

#[test]
fn ordinary_explicit_context_rejects_one_payload_and_retains_later_inference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n  Ready\n  Closed\n  Last\nend\n",
            "type Triple<T>\n  Grouped(T, T, T)\nend\n",
            "fn ordinary(first: {guard: Option<Int>, state: State::Ready}, rejected: {guard: Option<String>, state: State::Last}, later: {guard: Option<Int>, state: State::Closed}) -> Triple<{guard: Option<Int>, state: State::Ready | State::Closed}>\n",
            "  Grouped(first, rejected, later)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(
        &source.text()[span.start.offset..span.end.offset],
        "rejected",
        "the mismatch must remain attached to the rejected payload",
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
            "type Invariant<T>\n  InvariantValue(fn(T) -> T)\nend\n",
            "type Mixed<T>\n  MixedValue(T, fn(T) -> (), Invariant<T>)\nend\n",
            "type Pair<T>\n  Paired(T, T)\nend\n",
            "type Upper<T>\n  UpperValue(fn(T) -> (), fn(T) -> ())\nend\n",
            "fn accepts_state(value: State) -> ()\n  ()\nend\n",
            "fn accepts_int(value: Int) -> ()\n  ()\nend\n",
            "fn accepts_string(value: String) -> ()\n  ()\nend\n",
            "fn ready_identity(value: State::Ready) -> State::Ready\n  value\nend\n",
            "fn needs_mixed(value: Mixed<State::Ready>) -> ()\n  ()\nend\n",
            "fn mixed_private()\n",
            "  MixedValue(Ready, accepts_state, InvariantValue(ready_identity))\n",
            "end\n",
            "fn mixed_ordinary() -> ()\n",
            "  let mixed = MixedValue(Ready, accepts_state, InvariantValue(ready_identity))\n",
            "  needs_mixed(mixed)\n",
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
