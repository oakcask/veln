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

fn fallback_record(index: usize) -> Type {
    Type::Record(vec![
        ("common".to_string(), Type::int()),
        (format!("field_{index}"), Type::int()),
    ])
}

fn fallback_comparison_measurement(size: usize, resolve_late: bool) -> usize {
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
    crate::aggregate_type_join::take_fallback_comparisons()
}

#[test]
fn fallback_candidate_comparisons_grow_at_most_quadratically() {
    for resolve_late in [false, true] {
        let sizes = [8, 16, 32];
        let measurements = sizes.map(|size| {
            let comparisons = fallback_comparison_measurement(size, resolve_late);
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
