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

pub(crate) fn same_type_identity(
    expected_name: &str,
    expected_identity: &str,
    actual_name: &str,
    actual_identity: &str,
) -> bool {
    expected_identity == actual_identity
        || (expected_name == actual_name
            && (expected_identity == expected_name || actual_identity == actual_name))
}

pub(crate) fn invariant_args_match(expected: &[Type], actual: &[Type]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected, actual)| invariant_types_match(expected, actual))
}

#[derive(Default)]
pub(crate) struct TypePresentationJoin {
    conflicted_paths: HashSet<Vec<usize>>,
}

impl TypePresentationJoin {
    pub(crate) fn merge(&mut self, joined: &mut Type, actual: &Type) -> bool {
        self.merge_at(joined, actual, &mut Vec::new())
    }

    fn merge_at(&mut self, joined: &mut Type, actual: &Type, path: &mut Vec<usize>) -> bool {
        match (joined, actual) {
            (
                Type::Named {
                    name: joined_name,
                    identity: joined_identity,
                    args: joined_args,
                },
                Type::Named {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                },
            )
            | (
                Type::VariantRefinement {
                    name: joined_name,
                    identity: joined_identity,
                    args: joined_args,
                    ..
                },
                Type::VariantRefinement {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                    ..
                },
            ) if same_type_identity(joined_name, joined_identity, actual_name, actual_identity)
                && joined_args.len() == actual_args.len() =>
            {
                self.merge_named(
                    joined_name,
                    joined_identity,
                    joined_args,
                    actual_name,
                    actual_args,
                    path,
                )
            }
            (Type::Record(joined_fields), Type::Record(actual_fields)) => {
                self.merge_record(joined_fields, actual_fields, path)
            }
            (
                Type::Function {
                    params: joined_params,
                    variadic: joined_variadic,
                    return_type: joined_return,
                    ..
                },
                Type::Function {
                    params: actual_params,
                    variadic: actual_variadic,
                    return_type: actual_return,
                    ..
                },
            ) if joined_params.len() == actual_params.len() => self.merge_function(
                (joined_params, joined_variadic, joined_return),
                (actual_params, actual_variadic, actual_return),
                path,
            ),
            _ => false,
        }
    }

    fn merge_named(
        &mut self,
        joined_name: &mut String,
        joined_identity: &str,
        joined_args: &mut [Type],
        actual_name: &str,
        actual_args: &[Type],
        path: &mut Vec<usize>,
    ) -> bool {
        let canonical_name = joined_identity
            .rsplit("::")
            .next()
            .unwrap_or(joined_name)
            .to_string();
        let mut changed = reconcile_presentation_name(
            joined_name,
            actual_name,
            &canonical_name,
            path,
            &mut self.conflicted_paths,
        );
        for (index, (joined_arg, actual_arg)) in joined_args.iter_mut().zip(actual_args).enumerate()
        {
            changed |= self.merge_child(joined_arg, actual_arg, path, index);
        }
        changed
    }

    fn merge_record(
        &mut self,
        joined_fields: &mut [(String, Type)],
        actual_fields: &[(String, Type)],
        path: &mut Vec<usize>,
    ) -> bool {
        let mut changed = false;
        for (index, (name, joined_field)) in joined_fields.iter_mut().enumerate() {
            let Some((_, actual_field)) = actual_fields
                .iter()
                .find(|(actual_name, _)| actual_name == name)
            else {
                continue;
            };
            changed |= self.merge_child(joined_field, actual_field, path, index);
        }
        changed
    }

    fn merge_function(
        &mut self,
        joined: (&mut [Type], &mut Option<Box<Type>>, &mut Type),
        actual: (&[Type], &Option<Box<Type>>, &Type),
        path: &mut Vec<usize>,
    ) -> bool {
        let (joined_params, joined_variadic, joined_return) = joined;
        let (actual_params, actual_variadic, actual_return) = actual;
        let mut changed = false;
        for (index, (joined_param, actual_param)) in
            joined_params.iter_mut().zip(actual_params).enumerate()
        {
            changed |= self.merge_child(joined_param, actual_param, path, index);
        }
        if let (Some(joined_variadic), Some(actual_variadic)) =
            (joined_variadic.as_deref_mut(), actual_variadic.as_deref())
        {
            changed |=
                self.merge_child(joined_variadic, actual_variadic, path, joined_params.len());
        }
        changed | self.merge_child(joined_return, actual_return, path, joined_params.len() + 1)
    }

    fn merge_child(
        &mut self,
        joined: &mut Type,
        actual: &Type,
        path: &mut Vec<usize>,
        index: usize,
    ) -> bool {
        path.push(index);
        let changed = self.merge_at(joined, actual, path);
        path.pop();
        changed
    }
}

fn reconcile_presentation_name(
    joined: &mut String,
    actual: &str,
    canonical: &str,
    path: &[usize],
    conflicted_paths: &mut HashSet<Vec<usize>>,
) -> bool {
    if conflicted_paths.contains(path) {
        if joined == canonical {
            return false;
        }
        joined.clear();
        joined.push_str(canonical);
        return true;
    }
    if actual == canonical {
        return false;
    }
    if joined == canonical {
        joined.clear();
        joined.push_str(actual);
        return true;
    }
    if joined == actual {
        return false;
    }
    joined.clear();
    joined.push_str(canonical);
    conflicted_paths.insert(path.to_vec());
    true
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
