use std::collections::HashSet;

use crate::adt::registry::AdtRegistry;
use crate::adt::unification;
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
    if trivially_assignable(expected, actual) {
        return true;
    }
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
            same_type_identity(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            ) && invariant_args_match(expected_args, actual_args)
                && actual_variants.iter().all(|variant| {
                    record_variant_set_lookup();
                    expected_variants.contains(variant.as_str())
                })
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
            same_type_identity(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            ) && invariant_args_match(expected_args, actual_args)
        }
        _ => structurally_assignable(expected, actual),
    }
}

pub(crate) fn is_assignable_nested(expected: &Type, actual: &Type) -> bool {
    trivially_assignable(expected, actual) || structurally_assignable(expected, actual)
}

pub(crate) fn join_same_adt_types(adts: &AdtRegistry, left: &Type, right: &Type) -> Option<Type> {
    let left = adt_type_parts(left)?;
    let right = adt_type_parts(right)?;
    if !same_type_identity(left.name, left.identity, right.name, right.identity)
        || !invariant_args_match(left.args, right.args)
    {
        return None;
    }
    let mut joined_args = left.args.to_vec();
    for (joined, right) in joined_args.iter_mut().zip(right.args) {
        unification::merge_type_slot(joined, right);
    }

    if left.variants.is_none() || right.variants.is_none() {
        return Some(Type::resolved_named(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
        ));
    }

    let descriptor = adts.descriptor_for_type(left.ty)?;
    let left_variants = left.variants.expect("checked above");
    let right_variants = right.variants.expect("checked above");
    let variants = descriptor
        .variants
        .iter()
        .filter(|variant| {
            left_variants.contains(&variant.name) || right_variants.contains(&variant.name)
        })
        .map(|variant| variant.name.clone())
        .collect::<Vec<_>>();
    if variants.len() == descriptor.variants.len() {
        Some(Type::resolved_named(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
        ))
    } else {
        Some(Type::resolved_variant_refinement(
            left.name.to_string(),
            left.identity.to_string(),
            joined_args,
            variants,
        ))
    }
}

struct AdtTypeParts<'a> {
    ty: &'a Type,
    name: &'a str,
    identity: &'a str,
    args: &'a [Type],
    variants: Option<&'a [String]>,
}

fn adt_type_parts(ty: &Type) -> Option<AdtTypeParts<'_>> {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        } => Some(AdtTypeParts {
            ty,
            name,
            identity,
            args,
            variants: None,
        }),
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            ..
        } => Some(AdtTypeParts {
            ty,
            name,
            identity,
            args,
            variants: Some(variants),
        }),
        _ => None,
    }
}

fn trivially_assignable(expected: &Type, actual: &Type) -> bool {
    expected == &Type::Unknown || actual == &Type::Unknown || expected == actual
}

fn structurally_assignable(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            records_are_assignable(expected_fields, actual_fields)
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
        ) => named_types_are_assignable(
            expected_name,
            expected_identity,
            expected_args,
            actual_name,
            actual_identity,
            actual_args,
        ),
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
            parameters_are_assignable(expected_params, actual_params)
                && variadics_are_assignable(expected_variadic, actual_variadic)
                && is_assignable_nested(expected_return, actual_return)
                && effects_are_assignable(expected_effects, actual_effects)
        }
        _ => false,
    }
}

fn records_are_assignable(expected: &[(String, Type)], actual: &[(String, Type)]) -> bool {
    expected.iter().all(|(expected_name, expected_ty)| {
        actual
            .iter()
            .find(|(actual_name, _)| actual_name == expected_name)
            .is_some_and(|(_, actual_ty)| is_assignable_nested(expected_ty, actual_ty))
    })
}

fn named_types_are_assignable(
    expected_name: &str,
    expected_identity: &str,
    expected_args: &[Type],
    actual_name: &str,
    actual_identity: &str,
    actual_args: &[Type],
) -> bool {
    same_type_identity(
        expected_name,
        expected_identity,
        actual_name,
        actual_identity,
    ) && expected_args.len() == actual_args.len()
        && expected_args
            .iter()
            .zip(actual_args)
            .all(|(expected, actual)| is_assignable_nested(expected, actual))
}

fn parameters_are_assignable(expected: &[Type], actual: &[Type]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected, actual)| is_assignable_nested(expected, actual))
}

fn variadics_are_assignable(expected: &Option<Box<Type>>, actual: &Option<Box<Type>>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => is_assignable_nested(expected, actual),
        (None, None) => true,
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
            invariant_records_match(expected_fields, actual_fields)
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
            invariant_parameters_match(expected_params, actual_params)
                && invariant_variadics_match(expected_variadic, actual_variadic)
                && invariant_types_match(expected_return, actual_return)
                && expected_effects.len() == actual_effects.len()
                && expected_effects
                    .iter()
                    .all(|expected| actual_effects.contains(expected))
        }
        _ => false,
    }
}

fn invariant_records_match(expected: &[(String, Type)], actual: &[(String, Type)]) -> bool {
    expected.len() == actual.len()
        && expected.iter().all(|(expected_name, expected_ty)| {
            actual
                .iter()
                .find(|(actual_name, _)| actual_name == expected_name)
                .is_some_and(|(_, actual_ty)| invariant_types_match(expected_ty, actual_ty))
        })
}

fn invariant_parameters_match(expected: &[Type], actual: &[Type]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected, actual)| invariant_types_match(expected, actual))
}

fn invariant_variadics_match(expected: &Option<Box<Type>>, actual: &Option<Box<Type>>) -> bool {
    match (expected, actual) {
        (Some(expected), Some(actual)) => invariant_types_match(expected, actual),
        (None, None) => true,
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
