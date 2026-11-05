use std::collections::BTreeMap;

use crate::adt::registry::AdtRegistry;
use crate::adt::variance::Variance;

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
         type Builtins<T>\n  Value(Option<Result<List<Vec<Dict<T, T>>>, T>>)\nend\n",
    );

    for (name, expected) in [
        ("NestedProducer", P),
        ("NestedConsumer", N),
        ("NestedFixed", I),
        ("NestedIgnored", X),
        ("ReversedProducer", N),
        ("Builtins", P),
    ] {
        assert_eq!(variances(&registry, name), vec![expected], "{name}");
    }

    for name in ["Option", "Result", "List"] {
        let descriptor = registry
            .descriptors()
            .iter()
            .find(|descriptor| descriptor.type_name == name)
            .expect("compiler-owned descriptor");
        assert_eq!(
            descriptor.parameter_variances,
            vec![P; descriptor.type_parameters.len()],
            "{name}",
        );
    }
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
    use Variance::Covariant;
    let dependency = registry_from_source(
        "mod dependency\n\
         pub type Hidden<T>\n  HiddenValue(T)\nend\n",
    );
    let parsed = veln_syntax::parse(&veln_source::SourceFile::new(
        "consumer.veln",
        "mod consumer\n\
         use dependency\n\
         type Wrapper<T>\n  Value(dependency::Hidden<T>)\nend\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let registry = AdtRegistry::from_module_with_base(&module, &dependency);

    assert_eq!(variances(&registry, "Wrapper"), vec![Covariant]);
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
}
