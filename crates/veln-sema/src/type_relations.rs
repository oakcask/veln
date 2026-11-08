use std::collections::{HashMap, HashSet};

use crate::semantic_model::Type;
use crate::semantic_model::TypeArgs;

#[cfg(test)]
thread_local! {
    static VARIANT_SET_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static RECORD_FIELD_MATCH_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_variant_set_lookups() {
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.set(0));
}

#[cfg(test)]
pub(crate) fn take_variant_set_lookups() -> usize {
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.replace(0))
}

#[cfg(test)]
pub(crate) fn reset_record_field_match_work() {
    RECORD_FIELD_MATCH_WORK.with(|work| work.set(0));
}

#[cfg(test)]
pub(crate) fn take_record_field_match_work() -> usize {
    RECORD_FIELD_MATCH_WORK.with(|work| work.replace(0))
}

#[inline(always)]
fn record_field_match_work() {
    #[cfg(test)]
    RECORD_FIELD_MATCH_WORK.with(|work| work.set(work.get() + 1));
}

#[inline(always)]
pub(crate) fn record_variant_set_lookup() {
    #[cfg(test)]
    VARIANT_SET_LOOKUPS.with(|lookups| lookups.set(lookups.get() + 1));
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TypePresentationWork {
    pub(crate) conflict_lookups: usize,
    pub(crate) conflict_insertions: usize,
    pub(crate) child_lookups: usize,
    pub(crate) child_insertions: usize,
}

#[cfg(test)]
thread_local! {
    static TYPE_PRESENTATION_WORK: std::cell::Cell<TypePresentationWork> =
        const { std::cell::Cell::new(TypePresentationWork {
            conflict_lookups: 0,
            conflict_insertions: 0,
            child_lookups: 0,
            child_insertions: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_type_presentation_work() {
    TYPE_PRESENTATION_WORK.with(|work| work.set(TypePresentationWork::default()));
}

#[cfg(test)]
pub(crate) fn take_type_presentation_work() -> TypePresentationWork {
    TYPE_PRESENTATION_WORK.with(|work| work.replace(TypePresentationWork::default()))
}

#[cfg(test)]
fn update_type_presentation_work(update: impl FnOnce(&mut TypePresentationWork)) {
    TYPE_PRESENTATION_WORK.with(|work| {
        let mut current = work.get();
        update(&mut current);
        work.set(current);
    });
}

#[inline(always)]
fn record_type_presentation_conflict_lookup() {
    #[cfg(test)]
    update_type_presentation_work(|work| work.conflict_lookups += 1);
}

#[inline(always)]
fn record_type_presentation_conflict_insertion() {
    #[cfg(test)]
    update_type_presentation_work(|work| work.conflict_insertions += 1);
}

#[inline(always)]
fn record_type_presentation_child_lookup() {
    #[cfg(test)]
    update_type_presentation_work(|work| work.child_lookups += 1);
}

#[inline(always)]
fn record_type_presentation_child_insertions(inserted: usize) {
    #[cfg(test)]
    update_type_presentation_work(|work| work.child_insertions += inserted);
    #[cfg(not(test))]
    let _ = inserted;
}

use crate::adt::descriptors::{ParameterVarianceLookup, Variance};

pub(crate) fn is_assignable(expected: &Type, actual: &Type) -> bool {
    Assignability::without_registry().is_assignable(expected, actual)
}

pub(crate) fn is_assignable_with_registry(
    expected: &Type,
    actual: &Type,
    adts: &dyn ParameterVarianceLookup,
) -> bool {
    Assignability::with_registry(adts).is_assignable(expected, actual)
}

pub(crate) fn is_assignable_nested_with_registry(
    expected: &Type,
    actual: &Type,
    adts: &dyn ParameterVarianceLookup,
) -> bool {
    let mut pending = vec![(expected, actual)];
    while let Some((expected, actual)) = pending.pop() {
        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) {
            continue;
        }
        if !push_assignable_nested_pair(expected, actual, adts, &mut pending) {
            return false;
        }
    }
    true
}

fn push_assignable_nested_pair<'a>(
    expected: &'a Type,
    actual: &'a Type,
    adts: &dyn ParameterVarianceLookup,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
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
        ) => push_assignable_variant_refinement(
            VariantAssignableParts {
                named: NamedAssignableParts {
                    name: expected_name,
                    identity: expected_identity,
                    args: expected_args,
                },
                variants: expected_variants,
            },
            VariantAssignableParts {
                named: NamedAssignableParts {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                },
                variants: actual_variants,
            },
            adts,
            pending,
        ),
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                ..
            },
        )
        | (
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
        ) => push_assignable_named_type(
            NamedAssignableParts {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            NamedAssignableParts {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
            adts,
            pending,
        ),
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            push_assignable_record_fields(expected_fields, actual_fields, pending)
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
        ) => push_assignable_function_parts(
            FunctionAssignableParts {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            FunctionAssignableParts {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
            pending,
        ),
        _ => false,
    }
}

#[derive(Clone, Copy)]
struct NamedAssignableParts<'a> {
    name: &'a str,
    identity: &'a str,
    args: &'a TypeArgs,
}

#[derive(Clone, Copy)]
struct VariantAssignableParts<'a> {
    named: NamedAssignableParts<'a>,
    variants: &'a [String],
}

#[derive(Clone, Copy)]
struct FunctionAssignableParts<'a> {
    params: &'a [Type],
    variadic: &'a Option<Box<Type>>,
    return_type: &'a Type,
    effects: &'a [String],
}

fn push_assignable_variant_refinement<'a>(
    expected: VariantAssignableParts<'a>,
    actual: VariantAssignableParts<'a>,
    adts: &dyn ParameterVarianceLookup,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    let expected_variants = expected
        .variants
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    same_type_identity(
        expected.named.name,
        expected.named.identity,
        actual.named.name,
        actual.named.identity,
    ) && actual.variants.iter().all(|variant| {
        record_variant_set_lookup();
        expected_variants.contains(variant.as_str())
    }) && push_assignable_named_args(
        expected.named.identity,
        expected.named.args,
        actual.named.identity,
        actual.named.args,
        adts,
        pending,
    )
}

fn push_assignable_named_type<'a>(
    expected: NamedAssignableParts<'a>,
    actual: NamedAssignableParts<'a>,
    adts: &dyn ParameterVarianceLookup,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    same_type_identity(
        expected.name,
        expected.identity,
        actual.name,
        actual.identity,
    ) && push_assignable_named_args(
        expected.identity,
        expected.args,
        actual.identity,
        actual.args,
        adts,
        pending,
    )
}

fn push_assignable_record_fields<'a>(
    expected_fields: &'a [(String, Type)],
    actual_fields: &'a [(String, Type)],
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    for (expected_name, expected_ty) in expected_fields {
        let Some((_, actual_ty)) = actual_fields
            .iter()
            .find(|(actual_name, _)| actual_name == expected_name)
        else {
            return false;
        };
        record_field_match_work();
        pending.push((expected_ty, actual_ty));
    }
    true
}

fn push_assignable_function_parts<'a>(
    expected: FunctionAssignableParts<'a>,
    actual: FunctionAssignableParts<'a>,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    if expected.params.len() != actual.params.len()
        || expected.variadic.is_some() != actual.variadic.is_some()
        || !effects_are_assignable(expected.effects, actual.effects)
    {
        return false;
    }
    pending.push((expected.return_type, actual.return_type));
    if let (Some(expected), Some(actual)) =
        (expected.variadic.as_deref(), actual.variadic.as_deref())
    {
        pending.push((actual, expected));
    }
    pending.extend(
        expected
            .params
            .iter()
            .zip(actual.params)
            .map(|(expected, actual)| (actual, expected)),
    );
    true
}

fn push_assignable_named_args<'a>(
    expected_identity: &str,
    expected_args: &'a TypeArgs,
    actual_identity: &str,
    actual_args: &'a TypeArgs,
    adts: &dyn ParameterVarianceLookup,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    let compiler_variance = match (
        Type::is_compiler_covariant_identity(expected_identity),
        Type::is_compiler_covariant_identity(actual_identity),
    ) {
        (true, true) => Some(Variance::Covariant),
        (true, false) | (false, true) => return false,
        (false, false) => None,
    };
    if expected_args.len() != actual_args.len() {
        return false;
    }
    if expected_args.ptr_eq(actual_args) {
        return true;
    }
    if expected_identity != actual_identity {
        return invariant_args_match(expected_args, actual_args);
    }
    let registry_variances =
        adts.parameter_variances_for_identity(expected_identity, expected_args.len());
    for (index, (expected, actual)) in expected_args.iter().zip(actual_args).enumerate() {
        let variance = compiler_variance
            .or_else(|| registry_variances.and_then(|variances| variances.get(index).copied()))
            .unwrap_or(Variance::Invariant);
        match variance {
            Variance::Covariant => pending.push((expected, actual)),
            Variance::Contravariant => pending.push((actual, expected)),
            Variance::Invariant if !invariant_types_match(expected, actual) => return false,
            Variance::Invariant | Variance::Irrelevant => {}
        }
    }
    true
}

#[derive(Clone, Copy)]
struct Assignability<'a> {
    adts: Option<&'a dyn ParameterVarianceLookup>,
}

impl<'a> Assignability<'a> {
    fn without_registry() -> Self {
        Self { adts: None }
    }

    fn with_registry(adts: &'a dyn ParameterVarianceLookup) -> Self {
        Self { adts: Some(adts) }
    }

    fn is_assignable(self, expected: &Type, actual: &Type) -> bool {
        if Self::trivially_assignable(expected, actual) {
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
                ) && self.named_args_are_assignable(
                    expected_identity,
                    expected_args,
                    actual_identity,
                    actual_args,
                ) && actual_variants.iter().all(|variant| {
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
                ) && self.named_args_are_assignable(
                    expected_identity,
                    expected_args,
                    actual_identity,
                    actual_args,
                )
            }
            _ => self.structurally_assignable(expected, actual),
        }
    }

    fn trivially_assignable(expected: &Type, actual: &Type) -> bool {
        expected == &Type::Unknown || actual == &Type::Unknown || expected == actual
    }

    fn structurally_assignable(self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Record(expected_fields), Type::Record(actual_fields)) => {
                self.records_are_assignable(expected_fields, actual_fields)
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
            ) => self.named_types_are_assignable(
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
                self.parameters_are_assignable(expected_params, actual_params)
                    && self.variadics_are_assignable(expected_variadic, actual_variadic)
                    && self.is_assignable(expected_return, actual_return)
                    && effects_are_assignable(expected_effects, actual_effects)
            }
            _ => false,
        }
    }

    fn records_are_assignable(
        self,
        expected: &[(String, Type)],
        actual: &[(String, Type)],
    ) -> bool {
        expected.iter().all(|(expected_name, expected_ty)| {
            actual
                .iter()
                .find(|(actual_name, _)| actual_name == expected_name)
                .is_some_and(|(_, actual_ty)| self.is_assignable(expected_ty, actual_ty))
        })
    }

    fn named_types_are_assignable(
        self,
        expected_name: &str,
        expected_identity: &str,
        expected_args: &TypeArgs,
        actual_name: &str,
        actual_identity: &str,
        actual_args: &TypeArgs,
    ) -> bool {
        if !same_type_identity(
            expected_name,
            expected_identity,
            actual_name,
            actual_identity,
        ) || expected_args.len() != actual_args.len()
        {
            return false;
        }

        self.named_args_are_assignable(
            expected_identity,
            expected_args,
            actual_identity,
            actual_args,
        )
    }

    fn named_args_are_assignable(
        self,
        expected_identity: &str,
        expected_args: &TypeArgs,
        actual_identity: &str,
        actual_args: &TypeArgs,
    ) -> bool {
        let compiler_variance = match (
            Type::is_compiler_covariant_identity(expected_identity),
            Type::is_compiler_covariant_identity(actual_identity),
        ) {
            (true, true) => Some(Variance::Covariant),
            (true, false) | (false, true) => return false,
            (false, false) => None,
        };
        if expected_args.len() != actual_args.len() {
            return false;
        }
        if expected_args.ptr_eq(actual_args) {
            return true;
        }
        if expected_identity != actual_identity {
            return invariant_args_match(expected_args, actual_args);
        }
        let registry_variances = self.adts.and_then(|adts| {
            adts.parameter_variances_for_identity(expected_identity, expected_args.len())
        });
        expected_args
            .iter()
            .zip(actual_args)
            .enumerate()
            .all(|(index, (expected, actual))| {
                let variance = compiler_variance
                    .or_else(|| {
                        registry_variances.and_then(|variances| variances.get(index).copied())
                    })
                    .unwrap_or(Variance::Invariant);
                match variance {
                    Variance::Covariant => self.is_assignable(expected, actual),
                    Variance::Contravariant => self.is_assignable(actual, expected),
                    Variance::Invariant => invariant_types_match(expected, actual),
                    Variance::Irrelevant => true,
                }
            })
    }

    fn parameters_are_assignable(self, expected: &[Type], actual: &[Type]) -> bool {
        expected.len() == actual.len()
            && expected
                .iter()
                .zip(actual)
                .all(|(expected, actual)| self.is_assignable(actual, expected))
    }

    fn variadics_are_assignable(
        self,
        expected: &Option<Box<Type>>,
        actual: &Option<Box<Type>>,
    ) -> bool {
        match (expected, actual) {
            (Some(expected), Some(actual)) => self.is_assignable(actual, expected),
            (None, None) => true,
            _ => false,
        }
    }
}

pub(crate) fn named_args_are_assignable_with_registry(
    expected_identity: &str,
    expected_args: &TypeArgs,
    actual_identity: &str,
    actual_args: &TypeArgs,
    adts: &dyn ParameterVarianceLookup,
) -> bool {
    Assignability::with_registry(adts).named_args_are_assignable(
        expected_identity,
        expected_args,
        actual_identity,
        actual_args,
    )
}

pub(crate) fn same_type_identity(
    expected_name: &str,
    expected_identity: &str,
    actual_name: &str,
    actual_identity: &str,
) -> bool {
    if Type::is_compiler_covariant_identity(expected_identity)
        != Type::is_compiler_covariant_identity(actual_identity)
    {
        return false;
    }

    expected_identity == actual_identity
        || (expected_name == actual_name
            && (expected_identity == expected_name || actual_identity == actual_name))
}

pub(crate) fn invariant_args_match(expected: &[Type], actual: &[Type]) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    let mut pending = expected.iter().zip(actual).collect::<Vec<_>>();
    while let Some((expected, actual)) = pending.pop() {
        if !invariant_pair_matches(expected, actual, &mut pending) {
            return false;
        }
    }
    true
}

fn invariant_pair_matches<'a>(
    expected: &'a Type,
    actual: &'a Type,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
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
        ) => invariant_named_args_match(
            NamedInvariantParts {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            NamedInvariantParts {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
            pending,
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
        ) => invariant_variant_args_match(
            VariantInvariantParts {
                named: NamedInvariantParts {
                    name: expected_name,
                    identity: expected_identity,
                    args: expected_args,
                },
                variants: expected_variants,
            },
            VariantInvariantParts {
                named: NamedInvariantParts {
                    name: actual_name,
                    identity: actual_identity,
                    args: actual_args,
                },
                variants: actual_variants,
            },
            pending,
        ),
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            invariant_record_fields_match(expected_fields, actual_fields, pending)
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
        ) => invariant_function_parts_match(
            FunctionInvariantParts {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            FunctionInvariantParts {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
            pending,
        ),
        _ => false,
    }
}

#[derive(Clone, Copy)]
struct NamedInvariantParts<'a> {
    name: &'a str,
    identity: &'a str,
    args: &'a [Type],
}

#[derive(Clone, Copy)]
struct VariantInvariantParts<'a> {
    named: NamedInvariantParts<'a>,
    variants: &'a [String],
}

#[derive(Clone, Copy)]
struct FunctionInvariantParts<'a> {
    params: &'a [Type],
    variadic: &'a Option<Box<Type>>,
    return_type: &'a Type,
    effects: &'a [String],
}

fn invariant_named_args_match<'a>(
    expected: NamedInvariantParts<'a>,
    actual: NamedInvariantParts<'a>,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    if !invariant_named_identity_and_arity_match(expected, actual) {
        return false;
    }
    pending.extend(expected.args.iter().zip(actual.args));
    true
}

fn invariant_variant_args_match<'a>(
    expected: VariantInvariantParts<'a>,
    actual: VariantInvariantParts<'a>,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    if !invariant_named_identity_and_arity_match(expected.named, actual.named)
        || expected.variants != actual.variants
    {
        return false;
    }
    pending.extend(expected.named.args.iter().zip(actual.named.args));
    true
}

fn invariant_named_identity_and_arity_match(
    expected: NamedInvariantParts<'_>,
    actual: NamedInvariantParts<'_>,
) -> bool {
    same_type_identity(
        expected.name,
        expected.identity,
        actual.name,
        actual.identity,
    ) && expected.args.len() == actual.args.len()
}

fn invariant_record_fields_match<'a>(
    expected_fields: &'a [(String, Type)],
    actual_fields: &'a [(String, Type)],
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    if expected_fields.len() != actual_fields.len() {
        return false;
    }
    let actual_by_name = index_record_fields(actual_fields);
    for (name, expected_ty) in expected_fields {
        let Some(actual_ty) = indexed_record_field(&actual_by_name, name) else {
            return false;
        };
        pending.push((expected_ty, actual_ty));
    }
    true
}

fn invariant_function_parts_match<'a>(
    expected: FunctionInvariantParts<'a>,
    actual: FunctionInvariantParts<'a>,
    pending: &mut Vec<(&'a Type, &'a Type)>,
) -> bool {
    if expected.params.len() != actual.params.len()
        || expected.variadic.is_some() != actual.variadic.is_some()
        || expected.effects.len() != actual.effects.len()
        || !expected
            .effects
            .iter()
            .all(|expected| actual.effects.contains(expected))
    {
        return false;
    }
    pending.extend(expected.params.iter().zip(actual.params));
    if let (Some(expected), Some(actual)) =
        (expected.variadic.as_deref(), actual.variadic.as_deref())
    {
        pending.push((expected, actual));
    }
    pending.push((expected.return_type, actual.return_type));
    true
}

fn invariant_types_match(expected: &Type, actual: &Type) -> bool {
    invariant_args_match(std::slice::from_ref(expected), std::slice::from_ref(actual))
}

#[derive(Clone, Default)]
struct TypePresentationState {
    conflicted: bool,
    children: Vec<TypePresentationState>,
}

impl Drop for TypePresentationState {
    fn drop(&mut self) {
        let mut pending = std::mem::take(&mut self.children);
        while let Some(mut state) = pending.pop() {
            pending.append(&mut state.children);
        }
    }
}

impl TypePresentationState {
    #[cfg(test)]
    fn retained_nodes(&self) -> usize {
        let mut count = 0;
        let mut pending = vec![self];
        while let Some(state) = pending.pop() {
            count += 1;
            pending.extend(&state.children);
        }
        count
    }
}

#[derive(Clone, Default)]
pub(crate) struct TypePresentationJoin {
    state: TypePresentationState,
}

impl TypePresentationJoin {
    pub(crate) fn merge(&mut self, joined: &mut Type, actual: &Type) -> bool {
        let mut changed = false;
        let mut pending = vec![(joined, actual, &mut self.state)];
        while let Some((joined, actual, state)) = pending.pop() {
            changed |= merge_type_presentation(joined, actual, state, &mut pending);
        }
        changed
    }

    #[cfg(test)]
    pub(crate) fn retained_nodes(&self) -> usize {
        self.state.retained_nodes()
    }
}

fn merge_type_presentation<'a>(
    joined: &'a mut Type,
    actual: &'a Type,
    state: &'a mut TypePresentationState,
    pending: &mut Vec<(&'a mut Type, &'a Type, &'a mut TypePresentationState)>,
) -> bool {
    match (joined, actual) {
        (
            Type::Named {
                name: joined_name,
                identity: joined_identity,
                args: joined_args,
            }
            | Type::VariantRefinement {
                name: joined_name,
                identity: joined_identity,
                args: joined_args,
                ..
            },
            actual,
        ) => merge_named_type_presentation(
            joined_name,
            joined_identity,
            joined_args,
            actual,
            state,
            pending,
        ),
        (Type::Record(joined_fields), Type::Record(actual_fields)) => {
            merge_record_type_presentation(joined_fields, actual_fields, state, pending);
            false
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
        ) if joined_params.len() == actual_params.len() => {
            merge_function_type_presentation(
                MutableFunctionPresentationParts {
                    params: joined_params,
                    variadic: joined_variadic,
                    return_type: joined_return,
                },
                FunctionPresentationParts {
                    params: actual_params,
                    variadic: actual_variadic,
                    return_type: actual_return,
                },
                state,
                pending,
            );
            false
        }
        _ => false,
    }
}

fn merge_named_type_presentation<'a>(
    joined_name: &'a mut String,
    joined_identity: &str,
    joined_args: &'a mut [Type],
    actual: &'a Type,
    state: &'a mut TypePresentationState,
    pending: &mut Vec<(&'a mut Type, &'a Type, &'a mut TypePresentationState)>,
) -> bool {
    let Some((actual_name, actual_identity, actual_args)) = presentation_named_parts(actual) else {
        return false;
    };
    if !same_type_identity(joined_name, joined_identity, actual_name, actual_identity)
        || joined_args.len() != actual_args.len()
    {
        return false;
    }
    let canonical_name = joined_identity
        .rsplit("::")
        .next()
        .unwrap_or(joined_name)
        .to_string();
    let changed = reconcile_presentation_name(
        joined_name,
        actual_name,
        &canonical_name,
        &mut state.conflicted,
    );
    prepare_presentation_children(state, joined_args.len(), joined_args.len());
    pending.extend(
        joined_args
            .iter_mut()
            .zip(actual_args)
            .zip(state.children.iter_mut())
            .map(|((joined, actual), state)| (joined, actual, state)),
    );
    changed
}

fn merge_record_type_presentation<'a>(
    joined_fields: &'a mut [(String, Type)],
    actual_fields: &'a [(String, Type)],
    state: &'a mut TypePresentationState,
    pending: &mut Vec<(&'a mut Type, &'a Type, &'a mut TypePresentationState)>,
) {
    let actual = joined_fields
        .iter()
        .map(|(name, _)| {
            actual_fields
                .iter()
                .find(|(actual_name, _)| actual_name == name)
                .map(|(_, ty)| ty)
        })
        .collect::<Vec<_>>();
    let child_count = actual
        .iter()
        .rposition(Option::is_some)
        .map_or(0, |index| index + 1);
    prepare_presentation_children(
        state,
        child_count,
        actual.iter().filter(|actual| actual.is_some()).count(),
    );
    pending.extend(
        joined_fields
            .iter_mut()
            .zip(actual)
            .zip(state.children.iter_mut())
            .filter_map(|(((.., joined), actual), state)| {
                actual.map(|actual| (joined, actual, state))
            }),
    );
}

struct MutableFunctionPresentationParts<'a> {
    params: &'a mut [Type],
    variadic: &'a mut Option<Box<Type>>,
    return_type: &'a mut Type,
}

#[derive(Clone, Copy)]
struct FunctionPresentationParts<'a> {
    params: &'a [Type],
    variadic: &'a Option<Box<Type>>,
    return_type: &'a Type,
}

fn merge_function_type_presentation<'a>(
    joined: MutableFunctionPresentationParts<'a>,
    actual: FunctionPresentationParts<'a>,
    state: &'a mut TypePresentationState,
    pending: &mut Vec<(&'a mut Type, &'a Type, &'a mut TypePresentationState)>,
) {
    let variadic_count = usize::from(joined.variadic.is_some() && actual.variadic.is_some());
    let return_index = joined.params.len() + 1;
    prepare_presentation_children(
        state,
        return_index + 1,
        joined.params.len() + variadic_count + 1,
    );
    let (param_states, remaining_states) = state.children.split_at_mut(joined.params.len());
    let (variadic_states, return_states) = remaining_states.split_at_mut(1);
    pending.extend(
        joined
            .params
            .iter_mut()
            .zip(actual.params)
            .zip(param_states)
            .map(|((joined, actual), state)| (joined, actual, state)),
    );
    if let (Some(joined), Some(actual)) =
        (joined.variadic.as_deref_mut(), actual.variadic.as_deref())
    {
        pending.push((joined, actual, &mut variadic_states[0]));
    }
    pending.push((
        joined.return_type,
        actual.return_type,
        &mut return_states[0],
    ));
}

fn presentation_named_parts(ty: &Type) -> Option<(&str, &str, &[Type])> {
    match ty {
        Type::Named {
            name,
            identity,
            args,
        }
        | Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Some((name, identity, args)),
        _ => None,
    }
}

fn prepare_presentation_children(
    state: &mut TypePresentationState,
    required_len: usize,
    lookups: usize,
) {
    for _ in 0..lookups {
        record_type_presentation_child_lookup();
    }
    if state.children.len() < required_len {
        let inserted = required_len - state.children.len();
        record_type_presentation_child_insertions(inserted);
        state
            .children
            .resize_with(required_len, TypePresentationState::default);
    }
}

fn reconcile_presentation_name(
    joined: &mut String,
    actual: &str,
    canonical: &str,
    conflicted: &mut bool,
) -> bool {
    record_type_presentation_conflict_lookup();
    if *conflicted {
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
    *conflicted = true;
    record_type_presentation_conflict_insertion();
    true
}

pub(crate) fn record_fields_match_by_name(
    expected: &[(String, Type)],
    actual: &[(String, Type)],
    mut types_match: impl FnMut(&Type, &Type) -> bool,
) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    let actual_by_name = index_record_fields(actual);
    expected.iter().all(|(name, expected_ty)| {
        indexed_record_field(&actual_by_name, name)
            .is_some_and(|actual_ty| types_match(expected_ty, actual_ty))
    })
}

fn index_record_fields(fields: &[(String, Type)]) -> HashMap<&str, &Type> {
    let mut fields_by_name = HashMap::with_capacity(fields.len());
    for (name, ty) in fields {
        record_field_match_work();
        fields_by_name.entry(name.as_str()).or_insert(ty);
    }
    fields_by_name
}

fn indexed_record_field<'a>(
    fields_by_name: &HashMap<&str, &'a Type>,
    name: &str,
) -> Option<&'a Type> {
    record_field_match_work();
    fields_by_name.get(name).copied()
}

fn effects_are_assignable(expected: &[String], actual: &[String]) -> bool {
    actual
        .iter()
        .all(|effect| expected.iter().any(|expected| expected == effect))
}
