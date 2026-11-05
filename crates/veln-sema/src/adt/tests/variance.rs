use std::collections::BTreeMap;

use crate::adt::registry::AdtRegistry;
use crate::adt::variance::{Variance, variance_solver_counters};

use super::{lookup_descriptor, registry};

fn registry_from_source(source: &str) -> AdtRegistry {
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new("variance.veln", source));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    AdtRegistry::from_module_with_base(&module, &registry())
}

fn variances(registry: &AdtRegistry, name: &str) -> Vec<Variance> {
    registry
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == name && descriptor.nominal_identity.is_none())
        .unwrap_or_else(|| panic!("missing descriptor {name}"))
        .parameter_variances
        .clone()
}

#[test]
fn variance_composition_covers_all_cells() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};

    let cases = [
        (P, P, P),
        (P, N, N),
        (P, I, I),
        (P, X, X),
        (N, P, N),
        (N, N, P),
        (N, I, I),
        (N, X, X),
        (I, P, I),
        (I, N, I),
        (I, I, I),
        (I, X, I),
        (X, P, X),
        (X, N, X),
        (X, I, X),
        (X, X, X),
    ];

    for (outer, nested, expected) in cases {
        assert_eq!(
            outer.compose(nested),
            expected,
            "{outer:?} composed with {nested:?}"
        );
    }
}

#[test]
fn nested_source_constructor_composition_covers_all_cells() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};

    let cases = [
        ("PP", "Positive", "Positive", P),
        ("PN", "Positive", "Negative", N),
        ("PI", "Positive", "Invariant", I),
        ("PX", "Positive", "Irrelevant", X),
        ("NP", "Negative", "Positive", N),
        ("NN", "Negative", "Negative", P),
        ("NI", "Negative", "Invariant", I),
        ("NX", "Negative", "Irrelevant", X),
        ("IP", "Invariant", "Positive", I),
        ("IN", "Invariant", "Negative", I),
        ("II", "Invariant", "Invariant", I),
        ("IX", "Invariant", "Irrelevant", I),
        ("XP", "Irrelevant", "Positive", X),
        ("XN", "Irrelevant", "Negative", X),
        ("XI", "Irrelevant", "Invariant", X),
        ("XX", "Irrelevant", "Irrelevant", X),
    ];
    let mut source = cases
        .iter()
        .map(|(case, outer, nested, _)| {
            format!("type Case{case}<T>\n  Value{case}({outer}<{nested}<T>>)\nend\n")
        })
        .collect::<String>();
    source.push_str(
        "type Positive<T>\n  PositiveValue(T)\nend\n\
         type Negative<T>\n  NegativeValue(fn(T) -> ())\nend\n\
         type Invariant<T>\n  InvariantOutput(T)\n  InvariantInput(fn(T) -> ())\nend\n\
         type Irrelevant<T>\n  IrrelevantValue\nend\n",
    );

    let registry = registry_from_source(&source);

    for (case, _, _, expected) in cases {
        assert_eq!(variances(&registry, &format!("Case{case}")), vec![expected]);
    }
}

#[test]
fn occurrence_join_covers_positive_negative_invariant_and_unused() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};

    for (left, right, expected) in [
        (X, X, X),
        (X, P, P),
        (P, X, P),
        (X, N, N),
        (N, X, N),
        (P, P, P),
        (N, N, N),
        (P, N, I),
        (N, P, I),
        (I, X, I),
        (X, I, I),
        (I, P, I),
        (N, I, I),
    ] {
        assert_eq!(left.join(right), expected, "{left:?} joined with {right:?}");
    }
}

#[test]
fn source_payload_shapes_and_private_payloads_produce_checked_metadata() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};
    let registry = registry_from_source(
        "type Direct<T>\n  DirectValue(T)\nend\n\
         type Record<T>\n  RecordValue({value: T})\nend\n\
         type Input<T>\n  InputValue(fn(T) -> ())\nend\n\
         type ResultValue<T>\n  ResultCase(fn() -> T)\nend\n\
         type Variadic<T>\n  VariadicValue(fn(...T) -> ())\nend\n\
         type Mixed<T>\n  Positive(T)\n  Negative(fn(T) -> ())\nend\n\
         type Unused<T>\n  Empty\nend\n\
         type Ordered<A, B, C, D>\n  Value(A, fn(B) -> (), C, fn(C) -> (), Unused<D>)\nend\n\
         pub type PrivatePayload<T>\n  Hidden(T)\nend\n",
    );

    for (name, expected) in [
        ("Direct", P),
        ("Record", P),
        ("Input", N),
        ("ResultValue", P),
        ("Variadic", N),
        ("Mixed", I),
        ("Unused", X),
        ("PrivatePayload", P),
    ] {
        assert_eq!(variances(&registry, name), vec![expected], "{name}");
    }
    assert_eq!(variances(&registry, "Ordered"), vec![P, N, I, X]);
    for descriptor in registry.descriptors() {
        assert_eq!(
            descriptor.parameter_variances.len(),
            descriptor.type_parameters.len(),
            "{}",
            descriptor.type_name,
        );
    }
}

#[test]
fn assignment_consumes_all_checked_source_parameter_variances() {
    let mut registry = registry_from_source(
        "type Positive<T>\n  PositiveValue(T)\nend\n\
         type Negative<T>\n  NegativeValue(fn(T) -> ())\nend\n\
         type Fixed<T>\n  PositiveUse(T)\n  NegativeUse(fn(T) -> ())\nend\n\
         type Ignored<T>\n  IgnoredValue\nend\n",
    );
    let narrow = crate::semantic_model::Type::variant_refinement(
        "State",
        Vec::new(),
        vec!["Ready".to_string()],
    );
    let wide = crate::semantic_model::Type::named("State", Vec::new());
    let named = |name: &str, argument: crate::semantic_model::Type| {
        let identity = registry
            .descriptors()
            .iter()
            .find(|descriptor| {
                descriptor.type_name == name && descriptor.nominal_identity.is_none()
            })
            .expect("source descriptor")
            .identity();
        crate::semantic_model::Type::resolved_named(name, identity, vec![argument])
    };

    let positive_narrow = named("Positive", narrow.clone());
    let positive_wide = named("Positive", wide.clone());
    let negative_narrow = named("Negative", narrow.clone());
    let negative_wide = named("Negative", wide.clone());
    let fixed_narrow = named("Fixed", narrow.clone());
    let fixed_wide = named("Fixed", wide.clone());
    let ignored_narrow = named("Ignored", narrow.clone());
    let ignored_wide = named("Ignored", wide.clone());

    assert!(crate::type_relations::is_assignable_with_registry(
        &positive_wide,
        &positive_narrow,
        &registry,
    ));
    assert!(!crate::type_relations::is_assignable_with_registry(
        &positive_narrow,
        &positive_wide,
        &registry,
    ));
    assert!(crate::type_relations::is_assignable_with_registry(
        &negative_narrow,
        &negative_wide,
        &registry,
    ));
    assert!(!crate::type_relations::is_assignable_with_registry(
        &negative_wide,
        &negative_narrow,
        &registry,
    ));
    assert!(!crate::type_relations::is_assignable_with_registry(
        &fixed_wide,
        &fixed_narrow,
        &registry,
    ));
    assert!(!crate::type_relations::is_assignable_with_registry(
        &fixed_narrow,
        &fixed_wide,
        &registry,
    ));
    assert!(crate::type_relations::is_assignable_with_registry(
        &ignored_wide,
        &ignored_narrow,
        &registry,
    ));
    assert!(crate::type_relations::is_assignable_with_registry(
        &ignored_narrow,
        &ignored_wide,
        &registry,
    ));

    let nested_actual = crate::semantic_model::Type::function(
        vec![positive_wide.clone()],
        crate::semantic_model::Type::Record(vec![("value".to_string(), positive_narrow.clone())]),
        Vec::new(),
    );
    let nested_expected = crate::semantic_model::Type::function(
        vec![positive_narrow.clone()],
        crate::semantic_model::Type::Record(vec![("value".to_string(), positive_wide.clone())]),
        Vec::new(),
    );
    assert!(crate::type_relations::is_assignable_with_registry(
        &nested_expected,
        &nested_actual,
        &registry,
    ));

    let positive_identity = match &positive_wide {
        crate::semantic_model::Type::Named { identity, .. } => identity.clone(),
        _ => unreachable!(),
    };
    registry.clear_parameter_variances_for_identity(&positive_identity);
    assert!(!crate::type_relations::is_assignable_with_registry(
        &positive_wide,
        &positive_narrow,
        &registry,
    ));
}

#[test]
fn nested_source_and_compiler_owned_constructors_compose_checked_metadata() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};
    let registry = registry_from_source(
        "type Producer<T>\n  Produced(T)\nend\n\
         type Consumer<T>\n  Consumed(fn(T) -> ())\nend\n\
         type Fixed<T>\n  One(T)\n  Other(fn(T) -> ())\nend\n\
         type Ignored<T>\n  Nothing\nend\n\
         type NestedProducer<T>\n  Value(Producer<T>)\nend\n\
         type NestedConsumer<T>\n  Value(Consumer<T>)\nend\n\
         type NestedFixed<T>\n  Value(Fixed<T>)\nend\n\
         type NestedIgnored<T>\n  Value(Ignored<T>)\nend\n\
         type ReversedProducer<T>\n  Value(fn(Producer<T>) -> ())\nend\n\
         type Builtins<T>\n  Value(Option<Result<List<Vec<Dict<T, T>>>, T>>)\nend\n\
         type DictKey<T>\n  Value(Dict<T, Int>)\nend\n\
         type DictValue<T>\n  Value(Dict<Int, T>)\nend\n",
    );

    for (name, expected) in [
        ("NestedProducer", P),
        ("NestedConsumer", N),
        ("NestedFixed", I),
        ("NestedIgnored", X),
        ("ReversedProducer", N),
        ("Builtins", P),
        ("DictKey", P),
        ("DictValue", P),
    ] {
        assert_eq!(variances(&registry, name), vec![expected], "{name}");
    }

    for (name, arity) in [("Option", 1), ("Result", 2), ("List", 1)] {
        let descriptor = registry
            .descriptors()
            .iter()
            .find(|descriptor| descriptor.type_name == name)
            .expect("compiler-owned descriptor");
        assert_eq!(descriptor.parameter_variances, vec![P; arity], "{name}",);
        assert_eq!(descriptor.type_parameters.len(), arity, "{name}");
    }
    for name in ["Vec", "Dict"] {
        assert!(
            registry
                .descriptors()
                .iter()
                .all(|descriptor| descriptor.type_name != name),
            "{name} variance metadata must not publish an ADT descriptor",
        );
    }
}

fn wide_shift_left_source(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("T{index}"))
        .collect::<Vec<_>>();
    let mut shifted = parameters.iter().skip(1).cloned().collect::<Vec<_>>();
    shifted.push(parameters.last().expect("positive width").clone());
    format!(
        "type Wide<{}>\n  Direct(T0)\n  Recur(Wide<{}>)\nend\n",
        parameters.join(", "),
        shifted.join(", "),
    )
}

#[test]
fn wide_self_recursive_shift_has_linear_repeated_work() {
    use Variance::Covariant;

    let mut measurements = Vec::new();
    for width in [256usize, 512] {
        variance_solver_counters::reset();
        let start = std::time::Instant::now();
        let registry = registry_from_source(&wide_shift_left_source(width));
        let elapsed = start.elapsed();
        let factor_evaluations = variance_solver_counters::factor_evaluations();
        let constraint_joins = variance_solver_counters::constraint_joins();
        assert_eq!(variances(&registry, "Wide"), vec![Covariant; width]);
        eprintln!(
            "wide variance width={width} factor_evaluations={factor_evaluations} constraint_joins={constraint_joins} elapsed={elapsed:?}"
        );
        measurements.push((factor_evaluations, constraint_joins));
    }
    assert!(
        measurements[1].0 <= measurements[0].0 * 2 + 4,
        "factor evaluations must grow linearly: {measurements:?}",
    );
    assert!(
        measurements[1].1 <= measurements[0].1 * 2 + 4,
        "constraint joins must grow linearly: {measurements:?}",
    );
}

#[test]
fn recursive_and_mutually_recursive_results_ignore_declaration_order() {
    use Variance::{Contravariant as N, Covariant as P, Invariant as I, Irrelevant as X};
    let declarations = [
        "type Positive<T>\n  Next(Positive<T>)\n  Value(T)\nend\n",
        "type Negative<T>\n  Next(Negative<T>)\n  Value(fn(T) -> ())\nend\n",
        "type Mixed<T>\n  Next(Mixed<T>)\n  Value(T)\n  Input(fn(T) -> ())\nend\n",
        "type Irrelevant<T>\n  Next(Irrelevant<T>)\nend\n",
        "type Left<T>\n  ToRight(Right<T>)\nend\n",
        "type Right<T>\n  ToLeft(Left<T>)\n  Value(T)\nend\n",
        "type LeftNegative<T>\n  ToRight(RightNegative<T>)\nend\n",
        "type RightNegative<T>\n  ToLeft(LeftNegative<T>)\n  Value(fn(T) -> ())\nend\n",
        "type LeftMixed<T>\n  ToRight(RightMixed<T>)\n  Value(T)\nend\n",
        "type RightMixed<T>\n  ToLeft(LeftMixed<T>)\n  Value(fn(T) -> ())\nend\n",
        "type LeftIrrelevant<T>\n  ToRight(RightIrrelevant<T>)\nend\n",
        "type RightIrrelevant<T>\n  ToLeft(LeftIrrelevant<T>)\nend\n",
    ];
    let forward = registry_from_source(&declarations.concat());
    let reverse = registry_from_source(&declarations.into_iter().rev().collect::<String>());

    for (name, expected) in [
        ("Positive", P),
        ("Negative", N),
        ("Mixed", I),
        ("Irrelevant", X),
        ("Left", P),
        ("Right", P),
        ("LeftNegative", N),
        ("RightNegative", N),
        ("LeftMixed", I),
        ("RightMixed", I),
        ("LeftIrrelevant", X),
        ("RightIrrelevant", X),
    ] {
        assert_eq!(variances(&forward, name), vec![expected], "forward {name}");
        assert_eq!(variances(&reverse, name), vec![expected], "reverse {name}");
    }
}

#[test]
fn nested_source_analysis_consumes_checked_base_metadata() {
    use Variance::{Covariant, Invariant};
    let dependency = registry_from_source(
        "mod dependency\n\
         pub type Visible<T>\n  VisibleValue(T)\nend\n\
         pub type VisibleAlias = Visible\n\
         type Hidden<T>\n  HiddenValue(T)\nend\n\
         type OwnerWrapper<T>\n  Value(Hidden<T>)\nend\n",
    );
    assert_eq!(variances(&dependency, "Visible"), vec![Covariant]);
    assert_eq!(variances(&dependency, "Hidden"), vec![Covariant]);
    assert_eq!(variances(&dependency, "OwnerWrapper"), vec![Covariant]);
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new(
        "consumer.veln",
        "mod consumer\n\
         use dependency\n\
         type PublicWrapper<T>\n  Value(dependency::Visible<T>)\nend\n\
         type AliasWrapper<T>\n  Value(dependency::VisibleAlias<T>)\nend\n\
         type PrivateWrapper<T>\n  Value(dependency::Hidden<T>)\nend\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let registry = AdtRegistry::from_module_with_base(&module, &dependency);

    assert_eq!(variances(&registry, "PublicWrapper"), vec![Covariant]);
    assert_eq!(variances(&registry, "AliasWrapper"), vec![Covariant]);
    assert_eq!(variances(&registry, "PrivateWrapper"), vec![Invariant]);

    let parsed = veln_syntax::parse(&veln_source::SourceFile::new(
        "aliased_consumer.veln",
        "mod aliased_consumer\n\
         use dependency as dep\n\
         type ImportAliasWrapper<T>\n  Value(dep::Visible<T>)\nend\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let registry = AdtRegistry::from_module_with_base(&module, &dependency);
    assert_eq!(variances(&registry, "ImportAliasWrapper"), vec![Covariant]);
}

#[test]
fn missing_or_malformed_constructor_metadata_is_invariant_without_name_inference() {
    use Variance::Invariant;
    let mut opaque = lookup_descriptor("dependency", 1);
    opaque.type_name = "CovariantLooking".to_string();
    opaque.parameter_variances.clear();
    opaque.refinement_availability = crate::adt::descriptors::VariantRefinementAvailability::Opaque;
    let base = AdtRegistry::from_parts(vec![opaque], BTreeMap::new());
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new(
        "variance.veln",
        "use dependency\n\
         type Wrapper<T>\n  Value(dependency::CovariantLooking<T>)\nend\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let registry = AdtRegistry::from_module_with_base(&module, &base);

    assert_eq!(variances(&registry, "Wrapper"), vec![Invariant]);
    for descriptor in registry.descriptors() {
        assert_eq!(
            descriptor.parameter_variances.len(),
            descriptor.type_parameters.len(),
            "{}",
            descriptor.type_name,
        );
    }
}

#[test]
fn compiler_owned_constructor_metadata_failures_are_invariant() {
    use Variance::Invariant;

    let canonical_wrapper = registry_from_source("type Wrapper<T>\n  Value(Option<T>)\nend\n")
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Wrapper")
        .expect("wrapper descriptor")
        .clone();
    let builtin_descriptors = registry().descriptors().to_vec();

    for (case, mutate) in [
        ("missing", 0usize),
        ("ambiguous", 1),
        ("declared arity", 2),
        ("classification arity", 3),
    ] {
        let mut descriptors = builtin_descriptors.clone();
        let option_index = descriptors
            .iter()
            .position(|descriptor| {
                descriptor.nominal_identity.as_deref()
                    == Some(crate::semantic_model::Type::COMPILER_OPTION_IDENTITY)
            })
            .expect("Option descriptor");
        match mutate {
            0 => {
                descriptors.remove(option_index);
            }
            1 => descriptors.push(descriptors[option_index].clone()),
            2 => descriptors[option_index]
                .type_parameters
                .push("Unexpected".to_string()),
            3 => descriptors[option_index].parameter_variances.clear(),
            _ => unreachable!(),
        }
        let source_start = descriptors.len();
        descriptors.push(canonical_wrapper.clone());
        crate::adt::variance::infer_source_parameter_variances(
            &mut descriptors,
            source_start,
            &Default::default(),
            &Default::default(),
        );
        assert_eq!(
            descriptors[source_start].parameter_variances,
            vec![Invariant],
            "{case}",
        );
    }
}

fn nested_vec_payload_descriptor(depth: usize) -> crate::adt::descriptors::AdtDescriptor {
    let mut descriptor = lookup_descriptor("variance", 1);
    descriptor.type_name = "Deep".to_string();
    descriptor.module_name = None;
    let mut payload = crate::semantic_model::Type::named("$param0", Vec::new());
    for _ in 0..depth {
        payload = crate::semantic_model::Type::vec(payload);
    }
    descriptor.variants[0].payload_fields = vec![crate::adt::descriptors::AdtPayloadField {
        name: "value".to_string(),
        ty: crate::adt::descriptors::AdtPayloadType::Concrete(payload),
    }];
    descriptor
}

#[test]
fn producer_local_constructor_metadata_rejects_wrong_arity() {
    use Variance::Invariant;

    let mut descriptor = nested_vec_payload_descriptor(0);
    let parameter = crate::semantic_model::Type::named("$param0", Vec::new());
    descriptor.variants[0].payload_fields[0].ty = crate::adt::descriptors::AdtPayloadType::Concrete(
        crate::semantic_model::Type::resolved_named(
            "Vec",
            crate::semantic_model::Type::COMPILER_VEC_IDENTITY,
            vec![parameter.clone(), parameter],
        ),
    );
    let mut descriptors = registry().descriptors().to_vec();
    let source_start = descriptors.len();
    descriptors.push(descriptor);
    crate::adt::variance::infer_source_parameter_variances(
        &mut descriptors,
        source_start,
        &Default::default(),
        &Default::default(),
    );

    assert_eq!(
        descriptors[source_start].parameter_variances,
        vec![Invariant]
    );
}

#[test]
fn nested_payload_collection_has_linear_work_and_retained_state() {
    use Variance::Covariant;

    let mut adjacent = Vec::new();
    for depth in [256usize, 512] {
        let mut descriptors = registry().descriptors().to_vec();
        assert!(
            descriptors
                .iter()
                .all(|descriptor| { !matches!(descriptor.type_name.as_str(), "Vec" | "Dict") })
        );
        let source_start = descriptors.len();
        descriptors.push(nested_vec_payload_descriptor(depth));

        variance_solver_counters::reset();
        let start = std::time::Instant::now();
        crate::adt::variance::infer_source_parameter_variances(
            &mut descriptors,
            source_start,
            &Default::default(),
            &Default::default(),
        );
        let elapsed = start.elapsed();
        let visits = variance_solver_counters::collection_visits();
        let retained = variance_solver_counters::retained_factors();

        assert_eq!(
            descriptors[source_start].parameter_variances,
            vec![Covariant]
        );
        assert_eq!(visits, depth + 1, "depth={depth}");
        assert_eq!(retained, depth, "depth={depth}");
        eprintln!(
            "variance payload depth={depth} visits={visits} retained={retained} elapsed={elapsed:?}"
        );
        adjacent.push((visits, retained));
    }
    assert_eq!(adjacent[1].0, adjacent[0].0 * 2 - 1);
    assert_eq!(adjacent[1].1, adjacent[0].1 * 2);
}

fn covariant_pair_comb_source(depth: usize) -> String {
    let mut nested = "T".to_string();
    for _ in 0..depth {
        nested = format!("Pair<T, {nested}>");
    }
    format!(
        "type Pair<A, B>\n  PairValue(A, B)\nend\n\
         type Comb<T>\n  CombValue({nested})\nend\n"
    )
}

#[test]
fn nested_pair_comb_has_linear_factor_and_dependency_work() {
    use Variance::Covariant;

    let mut measurements = Vec::new();
    for depth in [128usize, 256] {
        variance_solver_counters::reset();
        let start = std::time::Instant::now();
        let registry = registry_from_source(&covariant_pair_comb_source(depth));
        let elapsed = start.elapsed();
        let retained = variance_solver_counters::retained_factors();
        let factor_evaluations = variance_solver_counters::factor_evaluations();
        let dependency_edges = variance_solver_counters::dependency_edges();

        assert_eq!(variances(&registry, "Pair"), vec![Covariant; 2]);
        assert_eq!(variances(&registry, "Comb"), vec![Covariant]);
        assert_eq!(retained, depth * 2, "depth={depth}");
        eprintln!(
            "variance comb depth={depth} retained={retained} factor_evaluations={factor_evaluations} dependency_edges={dependency_edges} elapsed={elapsed:?}"
        );
        measurements.push((retained, factor_evaluations, dependency_edges));
    }

    assert!(
        measurements[1].1 <= measurements[0].1 * 2 + 8,
        "factor evaluations must grow linearly: {measurements:?}",
    );
    assert!(
        measurements[1].2 <= measurements[0].2 * 2 + 8,
        "dependency construction must grow linearly: {measurements:?}",
    );
}

fn variance_chain_source(count: usize, reverse: bool) -> String {
    let declarations = (0..count)
        .map(|index| {
            if index == 0 {
                "type A0<T>\n  Value(T)\nend\n".to_string()
            } else {
                format!("type A{index}<T>\n  Value(A{}<T>)\nend\n", index - 1)
            }
        })
        .collect::<Vec<_>>();
    if reverse {
        declarations.into_iter().rev().collect()
    } else {
        declarations.into_iter().collect()
    }
}

#[test]
fn long_single_dependency_chain_has_linear_order_independent_work() {
    use Variance::Covariant;

    for count in [100usize, 200, 400] {
        let mut results = Vec::new();
        for reverse in [false, true] {
            variance_solver_counters::reset();
            let start = std::time::Instant::now();
            let registry = registry_from_source(&variance_chain_source(count, reverse));
            let elapsed = start.elapsed();
            let evaluations = variance_solver_counters::evaluations();
            let chain_variances = (0..count)
                .map(|index| variances(&registry, &format!("A{index}")))
                .collect::<Vec<_>>();
            assert!(
                chain_variances
                    .iter()
                    .all(|variance| variance == &vec![Covariant]),
                "count={count} reverse={reverse}",
            );
            assert!(
                evaluations <= count * 2,
                "count={count} reverse={reverse} evaluations={evaluations}",
            );
            eprintln!(
                "variance chain count={count} reverse={reverse} evaluations={evaluations} elapsed={elapsed:?}"
            );
            results.push(chain_variances);
        }
        assert_eq!(results[0], results[1], "count={count}");
    }
}
