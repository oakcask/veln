use std::collections::HashSet;

use crate::semantic_model::Type;

#[cfg(test)]
thread_local! {
    static VARIANT_SET_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_variant_set_lookups() {
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.set(0));
}

#[cfg(test)]
pub(crate) fn take_variant_set_lookups() -> usize {
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.replace(0))
}

#[inline(always)]
pub(crate) fn record_variant_set_lookup() {
    #[cfg(test)]
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.set(lookups.get() + 1));
}

pub(crate) fn is_assignable(expected: &Type, actual: &Type) -> bool {
    is_assignable_at_boundary(expected, actual, true)
}

pub(crate) fn is_assignable_nested(expected: &Type, actual: &Type) -> bool {
    is_assignable_at_boundary(expected, actual, false)
}

fn is_assignable_at_boundary(expected: &Type, actual: &Type, direct: bool) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown || expected == actual {
        return true;
    }
    if direct {
        match (expected, actual) {
            (
                Type::VariantRefinement {
                    name: expected_name,
                    identity: expected_identity,
                    args: expected_args,
                    variants: expected_variants,
                    ..
                },
                Type::VariantRefinement {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                    variants: actual_variants,
                    ..
                },
            ) => {
                let expected_variants = expected_variants
                    .iter()
                    .map(String::as_str)
                    .collect::<HashSet<_>>();
                return same_type_identity(
                    expected_name,
                    expected_identity,
                    actual_name,
                    actual_identity,
                ) && invariant_args_match(expected_args, actual_args)
                    && actual_variants.iter().all(|variant| {
                        record_variant_set_lookup();
                        expected_variants.contains(variant.as_str())
                    });
            }
            (
                Type::Named {
                    name: expected_name,
                    identity: expected_identity,
                    args: expected_args,
                    ..
                },
                Type::VariantRefinement {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                    ..
                },
            ) => {
                return same_type_identity(
                    expected_name,
                    expected_identity,
                    actual_name,
                    actual_identity,
                ) && invariant_args_match(expected_args, actual_args);
            }
            _ => {}
        }
    }
    match (expected, actual) {
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            expected_fields.iter().all(|(expected_name, expected_ty)| {
                actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == expected_name)
                    .is_some_and(|(_, actual_ty)| {
                        is_assignable_at_boundary(expected_ty, actual_ty, false)
                    })
            })
        }
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
        ) => {
            same_type_identity(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            ) && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| is_assignable_at_boundary(expected, actual, false))
        }
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| is_assignable_at_boundary(expected, actual, false))
                && match (expected_variadic, actual_variadic) {
                    (Some(expected), Some(actual)) => {
                        is_assignable_at_boundary(expected, actual, false)
                    }
                    (None, None) => true,
                    _ => false,
                }
                && is_assignable_at_boundary(expected_return, actual_return, false)
                && effects_are_assignable(expected_effects, actual_effects)
        }
        _ => false,
    }
}

fn same_type_identity(
    expected_name: &str,
    expected_identity: &str,
    actual_name: &str,
    actual_identity: &str,
) -> bool {
    expected_identity == actual_identity
        || (expected_name == actual_name
            && (expected_identity == expected_name || actual_identity == actual_name))
}

fn invariant_args_match(expected: &[Type], actual: &[Type]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected, actual)| invariant_types_match(expected, actual))
}

fn invariant_types_match(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
        ) => {
            same_type_identity(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            ) && invariant_args_match(expected_args, actual_args)
        }
        (
            Type::VariantRefinement {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
                variants: expected_variants,
                ..
            },
            Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                variants: actual_variants,
                ..
            },
        ) => {
            same_type_identity(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            ) && invariant_args_match(expected_args, actual_args)
                && expected_variants == actual_variants
        }
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            expected_fields.len() == actual_fields.len()
                && expected_fields.iter().all(|(expected_name, expected_ty)| {
                    actual_fields
                        .iter()
                        .find(|(actual_name, _)| actual_name == expected_name)
                        .is_some_and(|(_, actual_ty)| invariant_types_match(expected_ty, actual_ty))
                })
        }
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| invariant_types_match(expected, actual))
                && match (expected_variadic, actual_variadic) {
                    (Some(expected), Some(actual)) => invariant_types_match(expected, actual),
                    (None, None) => true,
                    _ => false,
                }
                && invariant_types_match(expected_return, actual_return)
                && expected_effects.len() == actual_effects.len()
                && expected_effects
                    .iter()
                    .all(|expected| actual_effects.contains(expected))
        }
        _ => false,
    }
}

fn effects_are_assignable(expected: &[String], actual: &[String]) -> bool {
    if expected.iter().any(|effect| effect.starts_with("...")) {
        return true;
    }
    actual
        .iter()
        .all(|effect| expected.iter().any(|expected| expected == effect))
}
