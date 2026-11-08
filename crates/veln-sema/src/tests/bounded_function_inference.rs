use super::*;
use crate::semantic_model::Type;
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
fn function_input_meets_keep_irrelevant_arguments_exact() {
    let source = SourceFile::new(
        "main.veln",
        "type State\n  Ready\n  Closed\nend\ntype Ignored<T>\n  Empty\nend\n",
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let environment = TypeEnvironment::from_module(&lower_surface_ast(&parsed.tree));
    let identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Ignored")
        .expect("Ignored descriptor")
        .identity();
    let state_identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "State")
        .expect("State descriptor")
        .identity();
    let refinement = |variant: &str| {
        Type::resolved_variant_refinement(
            "State",
            &state_identity,
            Vec::new(),
            vec![variant.to_string()],
        )
    };
    let function = |argument| Type::Function {
        params: vec![Type::resolved_named("Ignored", &identity, vec![argument])],
        variadic: None,
        return_type: Box::new(Type::int()),
        effects: Vec::new(),
    };
    let initial = function(refinement("Ready"));
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("bounded function join");
    assert!(!joined.try_join(&environment.adts, &function(refinement("Closed"))));
    assert_eq!(joined.result_type(), initial);
}

#[test]
fn delayed_function_input_meets_resolve_in_every_order() {
    let permutations = [
        ["id_only", "label_only", "candidate"],
        ["id_only", "candidate", "label_only"],
        ["label_only", "id_only", "candidate"],
        ["label_only", "candidate", "id_only"],
        ["candidate", "id_only", "label_only"],
        ["candidate", "label_only", "id_only"],
    ];
    let mut source = String::new();
    for (index, order) in permutations.iter().enumerate() {
        source.push_str(&format!(
            "fn meet_{index}(id_only: fn({{id: Int}}) -> Int, label_only: fn({{label: String}}) -> Int, candidate: fn({{id: Int, label: String}}) -> Int)\n  [{}, {}, {}]\nend\n",
            order[0], order[1], order[2]
        ));
    }
    let (module, diagnostics) = analyze(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    for index in 0..permutations.len() {
        assert_eq!(
            result(&environment, &format!("meet_{index}")),
            "Vec<fn({id: Int, label: String}) -> Int>"
        );
    }
}

#[test]
fn structural_function_effect_union_does_not_synthesize_effect_rows() {
    let source = concat!(
        "fn mixed<effect E>(row: fn() -> Int effects [...E], concrete: fn() -> Int effects [stdio])\n  [row, concrete]\nend\n",
        "fn identical<effect E>(first: fn() -> Int effects [...E], second: fn() -> Int effects [...E])\n  [first, second]\nend\n",
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
    assert_eq!(result(&environment, "mixed"), "unknown");
    assert_eq!(
        result(&environment, "identical"),
        "Vec<fn() -> Int effects [...E]>"
    );
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
fn fallback_reconciles_aliases_from_non_candidates_in_every_order() {
    let permutations = [
        ["first", "second", "candidate"],
        ["first", "candidate", "second"],
        ["second", "first", "candidate"],
        ["second", "candidate", "first"],
        ["candidate", "first", "second"],
        ["candidate", "second", "first"],
    ];
    let mut source = String::from(
        "type State\n  Ready\n  Closed\nend\npub type First = State\npub type Second = State\n",
    );
    for (index, order) in permutations.iter().enumerate() {
        source.push_str(&format!(
            "fn aliases_{index}(first: {{state: First::Ready, left: String}}, second: {{state: Second::Ready, right: Bool}}, candidate: {{state: First::Ready}})\n  [{}, {}, {}]\nend\n",
            order[0], order[1], order[2]
        ));
    }
    let (module, diagnostics) = analyze(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    let state_identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "State")
        .expect("State descriptor")
        .identity();
    for index in 0..permutations.len() {
        let inferred = &environment
            .function(&format!("aliases_{index}"))
            .expect("permutation function")
            .return_type;
        assert_eq!(inferred.render(), "Vec<{state: State::Ready}>");
        let Type::Named { args, .. } = inferred else {
            panic!("expected Vec result");
        };
        let Type::Record(fields) = &args[0] else {
            panic!("expected record element");
        };
        let Type::VariantRefinement { name, identity, .. } = &fields[0].1 else {
            panic!("expected refined state field");
        };
        assert_eq!(name, "State");
        assert_eq!(identity, &state_identity);
    }
}

#[test]
fn covariant_wrappers_retain_nested_fallback_candidates_in_every_order() {
    let permutations = [
        ["first", "second", "fallback"],
        ["first", "fallback", "second"],
        ["second", "first", "fallback"],
        ["second", "fallback", "first"],
        ["fallback", "first", "second"],
        ["fallback", "second", "first"],
    ];
    let mut source = String::from("type Positive<A>\n  Present(A)\nend\n");
    for (index, order) in permutations.iter().enumerate() {
        source.push_str(&format!(
            "fn option_{index}(first: Option<{{id: Int, left: String}}>, second: Option<{{id: Int, right: Bool}}>, fallback: Option<{{id: Int}}>)\n  [{}, {}, {}]\nend\n",
            order[0], order[1], order[2]
        ));
        source.push_str(&format!(
            "fn source_{index}(first: Positive<{{id: Int, left: String}}>, second: Positive<{{id: Int, right: Bool}}>, fallback: Positive<{{id: Int}}>)\n  [{}, {}, {}]\nend\n",
            order[0], order[1], order[2]
        ));
    }
    let (module, diagnostics) = analyze(&source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    for index in 0..permutations.len() {
        assert_eq!(
            result(&environment, &format!("option_{index}")),
            "Vec<Option<{id: Int}>>"
        );
        assert_eq!(
            result(&environment, &format!("source_{index}")),
            "Vec<Positive<{id: Int}>>"
        );
    }
}

#[test]
fn rejected_optimized_transition_preserves_the_aggregate_for_later_fallback() {
    let environment = TypeEnvironment::from_module(&lower_surface_ast(
        &parse(&SourceFile::new("main.veln", "")).tree,
    ));
    let initial = Type::Record(vec![
        ("id".to_string(), Type::int()),
        ("extra".to_string(), Type::string()),
    ]);
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("record aggregate");
    let before_rejection = joined.result_type();
    let rejected = Type::Record(vec![
        ("id".to_string(), Type::string()),
        ("extra".to_string(), Type::string()),
    ]);
    assert!(!joined.try_join(&environment.adts, &rejected));
    assert_eq!(joined.result_type(), before_rejection);

    let fallback = Type::Record(vec![("id".to_string(), Type::int())]);
    assert!(joined.try_join(&environment.adts, &fallback));
    assert_eq!(joined.result_type(), fallback);
}

#[test]
fn record_width_fallback_with_function_fields_is_order_independent() {
    let source = concat!(
        "fn forward(narrow: {callback: fn(Int) -> Int}, wide: {callback: fn(Int) -> Int, label: String})\n  [narrow, wide]\nend\n",
        "fn reverse(narrow: {callback: fn(Int) -> Int}, wide: {callback: fn(Int) -> Int, label: String})\n  [wide, narrow]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    for name in ["forward", "reverse"] {
        assert_eq!(
            result(&environment, name),
            "Vec<{callback: fn(Int) -> Int}>"
        );
    }
}

#[test]
fn fallback_uses_contributions_instead_of_an_optimized_prefix_result() {
    let source = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "fn unresolved(first: {state: State::Ready}, second: {state: State::Closed}, wider: {state: State, extra: Int})\n",
        "  [first, second, wider]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.private_inference_incomplete");
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(result(&environment, "unresolved"), "unknown");
}

#[test]
fn recursive_shape_rejection_does_not_change_later_successful_join() {
    let source_text = concat!(
        "fn retained(first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int effects [stdio])\n",
        "  [first, rejected, later]\nend\n",
    );
    let (module, diagnostics) = analyze(source_text);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "rejected");
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        result(&environment, "retained"),
        "Vec<fn(fn(Int) -> Int) -> Int effects [stdio]>"
    );
}

#[test]
fn control_flow_rejection_preserves_bounded_state_for_later_effect_union() {
    let source_text = concat!(
        "type Choice\n  First\n  Rejected\n  Later\nend\n",
        "fn needs_int(value: Int) -> ()\n  ()\nend\n",
        "fn private_if(choice: Choice, first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int effects [stdio])\n",
        "  if choice == Choice::First\n    first\n  else if choice == Choice::Rejected\n    rejected\n  else\n    later\n  end\nend\n",
        "fn private_match(choice: Choice, first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int effects [stdio])\n",
        "  match choice\n    First => first\n    Rejected => rejected\n    Later => later\n  end\nend\n",
        "fn ordinary_if(choice: Choice, first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int effects [stdio]) -> ()\n",
        "  let selected = if choice == Choice::First\n    first\n  else if choice == Choice::Rejected\n    rejected\n  else\n    later\n  end\n  needs_int(selected)\nend\n",
        "fn ordinary_match(choice: Choice, first: fn(fn(Int) -> Int) -> Int, rejected: fn(fn(Int, Int) -> Int) -> Int, later: fn(fn(Int) -> Int) -> Int effects [stdio]) -> ()\n",
        "  let selected = match choice\n    First => first\n    Rejected => rejected\n    Later => later\n  end\n  needs_int(selected)\nend\n",
    );
    let (module, diagnostics) = analyze(source_text);
    let rejected = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_some_and(|span| &source_text[span.start.offset..span.end.offset] == "rejected")
        })
        .collect::<Vec<_>>();
    assert_eq!(rejected.len(), 4, "{diagnostics:#?}");
    assert!(
        rejected
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
    let final_type = "fn(fn(Int) -> Int) -> Int effects [stdio]";
    let ordinary_results = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.message.contains("expected `Int`") && diagnostic.message.contains(final_type)
        })
        .count();
    assert_eq!(ordinary_results, 2, "{diagnostics:#?}");
    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for function in ["private_if", "private_match"] {
        assert_eq!(result(&environment, function), final_type, "{function}");
    }
}

fn fallback_record(index: usize) -> Type {
    Type::Record(vec![
        ("common".to_string(), Type::int()),
        (format!("field_{index:03}"), Type::int()),
    ])
}

fn fallback_measurement(size: usize, resolve_late: bool) -> (usize, usize) {
    let environment = TypeEnvironment::from_module(&lower_surface_ast(
        &parse(&SourceFile::new("main.veln", "")).tree,
    ));
    let initial = fallback_record(0);
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("record fallback join");
    crate::aggregate_type_join::reset_fallback_comparisons();
    for index in 1..size {
        assert!(joined.try_join(&environment.adts, &fallback_record(index)));
    }
    if resolve_late {
        assert!(joined.try_join(
            &environment.adts,
            &Type::Record(vec![("common".to_string(), Type::int())]),
        ));
        assert_eq!(joined.result_type().render(), "{common: Int}");
    } else {
        assert!(!joined.has_resolved_result());
    }
    (
        joined.bounded_retained_state(),
        crate::aggregate_type_join::take_fallback_comparisons(),
    )
}

#[test]
fn fallback_candidate_retained_state_grows_at_most_linearly() {
    let sizes = [8, 16, 32];
    let measurements = sizes.map(|size| fallback_measurement(size, false).0);
    eprintln!("fallback retained state: {measurements:?}");
    for adjacent in measurements.windows(2) {
        assert!(
            adjacent[1] <= adjacent[0] * 2 + 8,
            "retained fallback state must grow at most linearly: {measurements:?}"
        );
    }
}

#[test]
fn fallback_candidate_comparisons_grow_at_most_quadratically() {
    for resolve_late in [false, true] {
        let sizes = [8, 16, 32];
        let measurements = sizes.map(|size| {
            let comparisons = fallback_measurement(size, resolve_late).1;
            let contributions = size + usize::from(resolve_late);
            assert!(
                comparisons <= contributions * contributions,
                "size={size} resolve_late={resolve_late} comparisons={comparisons}"
            );
            comparisons
        });
        eprintln!("fallback comparisons resolve_late={resolve_late}: {measurements:?}");
        for (index, adjacent) in measurements.windows(2).enumerate() {
            let scale = sizes[index + 1] / sizes[index];
            assert!(
                adjacent[1] <= adjacent[0] * scale * scale + sizes[index + 1],
                "{measurements:?}"
            );
        }
    }
}

fn function_input_width_meet_measurement(size: usize) -> (usize, std::time::Duration) {
    let environment = TypeEnvironment::from_module(&lower_surface_ast(
        &parse(&SourceFile::new("main.veln", "")).tree,
    ));
    let function = |width: usize| Type::Function {
        params: vec![Type::Record(
            (0..width)
                .map(|index| (format!("field_{index:03}"), Type::int()))
                .collect(),
        )],
        variadic: None,
        return_type: Box::new(Type::int()),
        effects: Vec::new(),
    };
    let initial = function(1);
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("bounded function join");
    crate::aggregate_type_join::reset_fallback_comparisons();
    let started = std::time::Instant::now();
    for width in 2..=size {
        assert!(joined.try_join(&environment.adts, &function(width)));
    }
    let elapsed = started.elapsed();
    let resolved = joined.result_type();
    let Type::Function { params, .. } = &resolved else {
        panic!("expected function result");
    };
    let Type::Record(fields) = &params[0] else {
        panic!("expected record input");
    };
    assert_eq!(fields.len(), size);
    crate::semantic_model::drop_type_iteratively(resolved);
    (
        crate::aggregate_type_join::take_fallback_comparisons(),
        elapsed,
    )
}

#[test]
fn structural_function_input_meets_grow_at_most_quadratically() {
    let sizes = [8, 16, 32];
    let measurements = sizes.map(function_input_width_meet_measurement);
    eprintln!("function input meet comparisons and elapsed: {measurements:?}");
    for (size, (comparisons, _)) in sizes.into_iter().zip(measurements) {
        assert!(
            comparisons <= size * size,
            "size={size} comparisons={comparisons}"
        );
    }
    for (index, adjacent) in measurements.windows(2).enumerate() {
        assert!(
            adjacent[1].0 <= adjacent[0].0 * 4 + sizes[index + 1],
            "{measurements:?}"
        );
    }
}

fn deep_fallback_component(depth: usize, identity: &str) -> Type {
    (0..depth).fold(Type::int(), |nested, _| Type::Function {
        params: vec![Type::Record(vec![(
            "value".to_string(),
            Type::resolved_named("Positive", identity, vec![nested]),
        )])],
        variadic: None,
        return_type: Box::new(Type::int()),
        effects: Vec::new(),
    })
}

#[test]
fn deeply_nested_contributed_candidate_relation_is_stack_safe() {
    let environment = positive_registry();
    let identity = positive_identity(&environment);
    let depth = 1_024;
    let record = |distinct: &str| {
        Type::Record(vec![
            (
                "common".to_string(),
                deep_fallback_component(depth, &identity),
            ),
            (distinct.to_string(), Type::int()),
        ])
    };
    let initial = record("left");
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("record fallback join");
    crate::aggregate_type_join::reset_fallback_comparisons();
    assert!(joined.try_join(&environment.adts, &record("right")));
    let candidate = Type::Record(vec![(
        "common".to_string(),
        deep_fallback_component(depth, &identity),
    )]);
    assert!(joined.try_join(&environment.adts, &candidate));
    assert_eq!(joined.result_type(), candidate);
    assert!(crate::aggregate_type_join::take_fallback_comparisons() >= 4);
}

#[test]
fn deep_owned_candidate_order_replacement_transition_and_drop_are_stack_safe() {
    let environment = positive_registry();
    let identity = positive_identity(&environment);
    let depth = 12_000;
    let record = |field_order: &[&str]| {
        Type::Record(
            field_order
                .iter()
                .map(|name| {
                    let ty = if *name == "common" {
                        deep_fallback_component(depth, &identity)
                    } else {
                        Type::int()
                    };
                    ((*name).to_string(), ty)
                })
                .collect(),
        )
    };

    let initial = record(&["common", "id", "left"]);
    let mut joined =
        crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
            .expect("deep record aggregate");
    crate::semantic_model::drop_type_iteratively(initial);

    for fields in [
        &["common", "id", "left"][..],
        &["id", "common"][..],
        &["common", "id"][..],
    ] {
        let contribution = record(fields);
        assert!(joined.try_join(&environment.adts, &contribution));
        crate::semantic_model::drop_type_iteratively(contribution);
    }

    let resolved = joined.result_type();
    let Type::Record(fields) = &resolved else {
        panic!("expected record result");
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["common", "id"]
    );
    let mut current = fields
        .iter()
        .find(|(name, _)| name == "common")
        .map(|(_, ty)| ty)
        .expect("common field");
    for _ in 0..depth {
        let Type::Function { params, .. } = current else {
            panic!("expected nested function");
        };
        let Type::Record(fields) = &params[0] else {
            panic!("expected nested record");
        };
        let Type::Named {
            identity: actual,
            args,
            ..
        } = &fields[0].1
        else {
            panic!("expected Positive wrapper");
        };
        assert_eq!(actual, &identity);
        current = &args[0];
    }
    assert!(matches!(current, Type::Named { name, .. } if name == "Int"));
    crate::semantic_model::drop_type_iteratively(resolved);
    drop(joined);
}

#[test]
fn unavailable_disjoint_refinement_meet_has_no_partial_function_type() {
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

#[test]
fn unavailable_disjoint_meet_fails_in_local_and_incompatible_annotated_contexts() {
    let source = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "fn local(first: fn(State::Ready) -> Int, second: fn(State::Closed) -> Int) -> ()\n",
        "  let unavailable = [first, second]\n  ()\nend\n",
        "fn annotated(first: fn(State::Ready) -> Int, second: fn(State::Closed) -> Int) -> ()\n",
        "  let unavailable: Vec<fn(State) -> Int> = [first, second]\n  ()\nend\n",
    );
    let (_, diagnostics) = analyze(source);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    let local = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.local_inference_incomplete")
        .expect("local incomplete-inference diagnostic");
    let inferred = local
        .details
        .as_object()
        .and_then(|details| details.iter().find(|(key, _)| key == "inferred_type"))
        .map(|(_, value)| value);
    assert_eq!(
        inferred,
        Some(&veln_diagnostics::JsonValue::string("unknown"))
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "expected `fn(State) -> Int`, but found `fn(State::Ready) -> Int`",
            "expected `fn(State) -> Int`, but found `fn(State::Closed) -> Int`",
        ]
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("fn(unknown"))
    );
}

#[test]
fn constructible_record_width_meet_uses_explicit_context() {
    let source = concat!(
        "fn contextual(id_only: fn({id: Int}) -> Int, label_only: fn({label: String}) -> Int) -> Vec<fn({id: Int, label: String}) -> Int>\n",
        "  [id_only, label_only]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        result(&environment, "contextual"),
        "Vec<fn({id: Int, label: String}) -> Int>"
    );
}

#[test]
fn diagnosed_contribution_does_not_change_retained_alias_state() {
    let source = concat!(
        "type State\n  Ready\n  Closed\nend\n",
        "pub type First = State\npub type Second = State\n",
        "fn recover(value: Int, callback: fn({id: Int, label: String}) -> Second::Ready) -> fn({id: Int, label: String}) -> Second::Ready\n",
        "  callback\nend\n",
        "fn baseline(id_only: fn({id: Int}) -> First::Ready, label_only: fn({label: String}) -> First::Ready, candidate: fn({id: Int, label: String}) -> First::Ready)\n",
        "  [id_only, label_only, candidate]\nend\n",
        "fn crossed(id_only: fn({id: Int}) -> First::Ready, label_only: fn({label: String}) -> First::Ready, conflicting: fn({id: Int, label: String}) -> Second::Ready, candidate: fn({id: Int, label: String}) -> First::Ready)\n",
        "  [id_only, label_only, recover(\"bad\", conflicting), candidate]\nend\n",
    );
    let (module, diagnostics) = analyze(source);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch");
    let environment = TypeEnvironment::from_module(&module);
    let baseline = &environment.function("baseline").unwrap().return_type;
    let crossed = &environment.function("crossed").unwrap().return_type;
    assert_eq!(crossed, baseline);
    assert_eq!(
        crossed.render(),
        "Vec<fn({id: Int, label: String}) -> First::Ready>"
    );
    let Type::Named { args, .. } = crossed else {
        panic!("expected Vec result");
    };
    let Type::Function { return_type, .. } = &args[0] else {
        panic!("expected callback element");
    };
    let Type::VariantRefinement { name, identity, .. } = return_type.as_ref() else {
        panic!("expected refined callback result");
    };
    let state_identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "State")
        .expect("State descriptor")
        .identity();
    assert_eq!(identity, &state_identity);
    assert_eq!(name, "First");
}

fn positive_registry() -> TypeEnvironment {
    let source = SourceFile::new("main.veln", "type Positive<T>\n  Present(T)\nend\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    TypeEnvironment::from_module(&lower_surface_ast(&parsed.tree))
}

fn positive_identity(environment: &TypeEnvironment) -> String {
    environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Positive")
        .expect("Positive descriptor")
        .identity()
}

fn deep_function_result(depth: usize, identity: &str, effects: Vec<String>) -> Type {
    let leaf = Type::Function {
        params: vec![Type::int()],
        variadic: None,
        return_type: Box::new(Type::int()),
        effects,
    };
    let nested = (0..depth).fold(leaf, |argument, _| {
        Type::resolved_named("Positive", identity, vec![argument])
    });
    Type::Function {
        params: Vec::new(),
        variadic: None,
        return_type: Box::new(nested),
        effects: Vec::new(),
    }
}

#[test]
fn deep_bounded_function_join_uses_linear_frames_and_function_scans() {
    let environment = positive_registry();
    let identity = positive_identity(&environment);
    let depths = [512, 1024, 2048];
    let measurements = depths.map(|depth| {
        let initial = deep_function_result(depth, &identity, Vec::new());
        let right = deep_function_result(depth, &identity, vec!["stdio".to_string()]);
        let mut joined =
            crate::aggregate_type_join::AggregateTypeJoin::new(&environment.adts, &initial)
                .expect("bounded function join");
        crate::aggregate_type_join::reset_bounded_frames();
        crate::aggregate_type_join::reset_function_scan_nodes();
        assert!(joined.try_join(&environment.adts, &right));
        let frames = crate::aggregate_type_join::take_bounded_frames();
        let function_scan_nodes = crate::aggregate_type_join::take_function_scan_nodes();
        let resolved = joined.result_type();
        let Type::Function { return_type, .. } = resolved else {
            panic!("expected outer function");
        };
        let mut leaf = return_type.as_ref();
        for _ in 0..depth {
            let Type::Named { args, .. } = leaf else {
                panic!("expected Positive wrapper");
            };
            leaf = &args[0];
        }
        let Type::Function { effects, .. } = leaf else {
            panic!("expected function leaf");
        };
        assert_eq!(effects, &["stdio"]);
        assert_eq!(frames, depth + 4);
        assert_eq!(function_scan_nodes, depth + 3);
        (frames, function_scan_nodes)
    });
    eprintln!("deep bounded frames and function scans: {measurements:?}");
    for adjacent in measurements.windows(2) {
        assert!(adjacent[1].0 <= adjacent[0].0 * 2 + 1);
        assert!(adjacent[1].1 <= adjacent[0].1 * 2);
    }
}
