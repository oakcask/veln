use crate::semantic_model::Type;

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
                    identity: expected_identity,
                    args: expected_args,
                    variants: expected_variants,
                    ..
                },
                Type::VariantRefinement {
                    identity: actual_identity,
                    args: actual_args,
                    variants: actual_variants,
                    ..
                },
            ) => {
                return expected_identity == actual_identity
                    && expected_args == actual_args
                    && actual_variants
                        .iter()
                        .all(|variant| expected_variants.contains(variant));
            }
            (
                Type::Named {
                    identity: expected_identity,
                    args: expected_args,
                    ..
                },
                Type::VariantRefinement {
                    identity: actual_identity,
                    args: actual_args,
                    ..
                },
            ) => return expected_identity == actual_identity && expected_args == actual_args,
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
                identity: expected_identity,
                args: expected_args,
                ..
            },
            Type::Named {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) => {
            expected_identity == actual_identity
                && expected_args.len() == actual_args.len()
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

fn effects_are_assignable(expected: &[String], actual: &[String]) -> bool {
    if expected.iter().any(|effect| effect.starts_with("...")) {
        return true;
    }
    actual
        .iter()
        .all(|effect| expected.iter().any(|expected| expected == effect))
}
