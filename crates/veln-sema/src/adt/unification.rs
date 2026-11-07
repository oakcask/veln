use std::collections::HashMap;

use veln_core::CoreType;

use crate::semantic_model::Type;
use crate::type_relations::TypePresentationJoin;

use super::descriptors::{AdtDescriptor, AdtPayloadType};

pub(super) use core_templates::{core_type_template, substitute_core_type_parameters};
pub(crate) use named_arguments::adt_args;
pub(super) use named_arguments::{named_part, named_parts2};

mod core_templates;
mod named_arguments;

pub(super) fn payload_type_from_args(
    ty: &Type,
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
) -> Option<Type> {
    match payload {
        AdtPayloadType::TypeParameter(index) => adt_args(ty, descriptor)?.get(*index).cloned(),
        AdtPayloadType::SelfType => Some(match ty {
            Type::VariantRefinement {
                name,
                identity,
                args,
                ..
            } => Type::resolved_named(name.clone(), identity.clone(), args.to_vec()),
            _ => ty.clone(),
        }),
        AdtPayloadType::Concrete(template) => {
            let args = adt_args(ty, descriptor)?;
            Some(substitute_type_parameters(template, args))
        }
    }
}

pub(super) fn core_payload_type_from_args(
    ty: &CoreType,
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
) -> Option<CoreType> {
    match payload {
        AdtPayloadType::TypeParameter(index) => adt_args(ty, descriptor)?.get(*index).cloned(),
        AdtPayloadType::SelfType => Some(ty.clone()),
        AdtPayloadType::Concrete(template) => {
            let args = adt_args(ty, descriptor)?;
            Some(substitute_core_type_parameters(
                &core_type_template(template),
                args,
            ))
        }
    }
}

pub(super) fn fill_type_parameters(
    args: &mut [Type],
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
    actual: &Type,
) {
    match payload {
        AdtPayloadType::TypeParameter(index) => assign_type_arg(args, *index, actual),
        AdtPayloadType::Concrete(template) => unify_template(args, template, actual),
        AdtPayloadType::SelfType => unify_self_type(args, descriptor, actual),
    }
}

pub(super) fn fill_core_type_parameters(
    args: &mut [CoreType],
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
    actual: &CoreType,
) {
    match payload {
        AdtPayloadType::TypeParameter(index) => assign_core_type_arg(args, *index, actual),
        AdtPayloadType::Concrete(template) => {
            unify_core_template(args, &core_type_template(template), actual);
        }
        AdtPayloadType::SelfType => unify_core_self_type(args, descriptor, actual),
    }
}

pub(super) fn assign_type_arg(args: &mut [Type], index: usize, actual: &Type) {
    let Some(slot) = args.get_mut(index) else {
        return;
    };
    merge_type_slot(slot, actual);
}

pub(super) fn assign_core_type_arg(args: &mut [CoreType], index: usize, actual: &CoreType) {
    let Some(slot) = args.get_mut(index) else {
        return;
    };
    merge_core_type_slot(slot, actual);
}

pub(crate) fn merge_type_slot(slot: &mut Type, actual: &Type) -> bool {
    let mut changed = false;
    let mut pending = vec![(slot, actual)];
    while let Some((slot, actual)) = pending.pop() {
        if matches!(actual, Type::Unknown) {
            continue;
        }
        match slot {
            slot @ Type::Unknown => {
                *slot = actual.clone();
                changed = true;
            }
            Type::Named {
                name: slot_name,
                identity: slot_identity,
                args: slot_args,
            } => queue_named_type_merge(slot_name, slot_identity, slot_args, actual, &mut pending),
            Type::VariantRefinement {
                name: slot_name,
                identity: slot_identity,
                args: slot_args,
                ..
            } => queue_refinement_type_merge(
                slot_name,
                slot_identity,
                slot_args,
                actual,
                &mut pending,
            ),
            Type::Record(slot_fields) => queue_record_type_merge(slot_fields, actual, &mut pending),
            Type::Function {
                params: slot_params,
                variadic: slot_variadic,
                return_type: slot_return,
                effects: _,
            } => queue_function_type_merge(
                slot_params,
                slot_variadic,
                slot_return,
                actual,
                &mut pending,
            ),
        }
    }
    changed
}

fn queue_named_type_merge<'slot, 'actual>(
    slot_name: &str,
    slot_identity: &str,
    slot_args: &'slot mut [Type],
    actual: &'actual Type,
    pending: &mut Vec<(&'slot mut Type, &'actual Type)>,
) {
    let Type::Named {
        name: actual_name,
        identity: actual_identity,
        args: actual_args,
    } = actual
    else {
        return;
    };
    queue_matching_type_args(
        (slot_name, slot_identity, slot_args),
        (actual_name, actual_identity, actual_args),
        pending,
    );
}

fn queue_refinement_type_merge<'slot, 'actual>(
    slot_name: &str,
    slot_identity: &str,
    slot_args: &'slot mut [Type],
    actual: &'actual Type,
    pending: &mut Vec<(&'slot mut Type, &'actual Type)>,
) {
    let (actual_name, actual_identity, actual_args) = match actual {
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
        } => (name, identity, args),
        _ => return,
    };
    queue_matching_type_args(
        (slot_name, slot_identity, slot_args),
        (actual_name, actual_identity, actual_args),
        pending,
    );
}

fn queue_matching_type_args<'slot, 'actual>(
    (slot_name, slot_identity, slot_args): (&str, &str, &'slot mut [Type]),
    (actual_name, actual_identity, actual_args): (&str, &str, &'actual [Type]),
    pending: &mut Vec<(&'slot mut Type, &'actual Type)>,
) {
    if slot_args.len() == actual_args.len()
        && crate::type_relations::same_type_identity(
            slot_name,
            slot_identity,
            actual_name,
            actual_identity,
        )
    {
        pending.extend(slot_args.iter_mut().zip(actual_args));
    }
}

fn queue_record_type_merge<'slot, 'actual>(
    slot_fields: &'slot mut [(String, Type)],
    actual: &'actual Type,
    pending: &mut Vec<(&'slot mut Type, &'actual Type)>,
) {
    let Type::Record(actual_fields) = actual else {
        return;
    };
    for (name, slot) in slot_fields {
        if let Some((_, actual)) = actual_fields
            .iter()
            .find(|(actual_name, _)| actual_name == name)
        {
            pending.push((slot, actual));
        }
    }
}

fn queue_function_type_merge<'slot, 'actual>(
    slot_params: &'slot mut [Type],
    slot_variadic: &'slot mut Option<Box<Type>>,
    slot_return: &'slot mut Type,
    actual: &'actual Type,
    pending: &mut Vec<(&'slot mut Type, &'actual Type)>,
) {
    let Type::Function {
        params: actual_params,
        variadic: actual_variadic,
        return_type: actual_return,
        effects: _,
    } = actual
    else {
        return;
    };
    if slot_params.len() != actual_params.len()
        || slot_variadic.is_some() != actual_variadic.is_some()
    {
        return;
    }
    pending.extend(slot_params.iter_mut().zip(actual_params));
    if let (Some(slot), Some(actual)) = (slot_variadic.as_deref_mut(), actual_variadic.as_deref()) {
        pending.push((slot, actual));
    }
    pending.push((slot_return, actual_return));
}

#[derive(Debug)]
pub(crate) struct TypeParameterContributionConflict {
    pub(crate) expected: Box<Type>,
    pub(crate) actual: Box<Type>,
}

pub(crate) fn merge_type_parameter_contributions_transactionally(
    contributions: &[(usize, Type)],
    mut current: impl FnMut(usize) -> Option<(Type, TypePresentationJoin)>,
) -> Result<Vec<(usize, Type, TypePresentationJoin)>, TypeParameterContributionConflict> {
    let mut trial = Vec::<(usize, Type, TypePresentationJoin)>::new();
    let mut trial_positions = HashMap::<usize, usize>::new();
    for (index, actual) in contributions {
        crate::inference_work::record(1);
        let trial_index = if let Some(trial_index) = trial_positions.get(index) {
            *trial_index
        } else {
            let Some((expected, presentation)) = current(*index) else {
                continue;
            };
            crate::inference_work::record(1);
            let trial_index = trial.len();
            trial.push((*index, expected, presentation));
            trial_positions.insert(*index, trial_index);
            trial_index
        };
        let rollback_work = trial.len();
        let (_, expected, presentation) = &mut trial[trial_index];
        crate::inference_work::record(1);
        if !type_parameter_contributions_compatible(expected, actual) {
            crate::inference_work::record(rollback_work);
            return Err(TypeParameterContributionConflict {
                expected: Box::new(expected.clone()),
                actual: Box::new(actual.clone()),
            });
        }
        crate::inference_work::record(1);
        merge_type_slot(expected, actual);
        presentation.merge(expected, actual);
    }
    Ok(trial)
}

fn type_parameter_contributions_compatible(expected: &Type, actual: &Type) -> bool {
    crate::type_relations::is_assignable(expected, actual)
        && crate::type_relations::is_assignable(actual, expected)
}

pub(crate) fn is_direct_variant_carrier(expected: &Type, actual: &Type) -> bool {
    let (
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
    ) = (expected, actual)
    else {
        return false;
    };
    crate::type_relations::same_type_identity(
        expected_name,
        expected_identity,
        actual_name,
        actual_identity,
    ) && crate::type_relations::invariant_args_match(expected_args, actual_args)
}

pub(super) fn merge_core_type_slot(slot: &mut CoreType, actual: &CoreType) {
    if actual == &CoreType::Unknown {
        return;
    }
    match (slot, actual) {
        (slot @ CoreType::Unknown, _) => *slot = actual.clone(),
        (
            CoreType::Named {
                name: slot_name,
                args: slot_args,
            },
            CoreType::Named {
                name: actual_name,
                args: actual_args,
            },
        ) if slot_name == actual_name && slot_args.len() == actual_args.len() => {
            for (slot_arg, actual_arg) in slot_args.iter_mut().zip(actual_args) {
                merge_core_type_slot(slot_arg, actual_arg);
            }
        }
        (CoreType::Record(slot_fields), CoreType::Record(actual_fields)) => {
            for (slot_name, slot_ty) in slot_fields {
                if let Some((_, actual_ty)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == slot_name)
                {
                    merge_core_type_slot(slot_ty, actual_ty);
                }
            }
        }
        (
            CoreType::Function {
                params: slot_params,
                variadic: slot_variadic,
                return_type: slot_return,
                effects: _,
            },
            CoreType::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: _,
            },
        ) if slot_params.len() == actual_params.len()
            && slot_variadic.is_some() == actual_variadic.is_some() =>
        {
            for (slot_param, actual_param) in slot_params.iter_mut().zip(actual_params) {
                merge_core_type_slot(slot_param, actual_param);
            }
            if let (Some(slot_variadic), Some(actual_variadic)) = (slot_variadic, actual_variadic) {
                merge_core_type_slot(slot_variadic, actual_variadic);
            }
            merge_core_type_slot(slot_return, actual_return);
        }
        _ => {}
    }
}

pub(super) fn unify_self_type(args: &mut [Type], descriptor: &AdtDescriptor, actual: &Type) {
    let Some(actual_args) = adt_args(actual, descriptor) else {
        return;
    };
    for (index, actual_arg) in actual_args.iter().enumerate() {
        assign_type_arg(args, index, actual_arg);
    }
}

pub(super) fn unify_core_self_type(
    args: &mut [CoreType],
    descriptor: &AdtDescriptor,
    actual: &CoreType,
) {
    let Some(actual_args) = adt_args(actual, descriptor) else {
        return;
    };
    for (index, actual_arg) in actual_args.iter().enumerate() {
        assign_core_type_arg(args, index, actual_arg);
    }
}

pub(super) fn unify_template(args: &mut [Type], template: &Type, actual: &Type) {
    match (template, actual) {
        (
            Type::Named {
                name, args: nested, ..
            },
            actual,
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>() {
                assign_type_arg(args, index, actual);
            }
        }
        (
            Type::Named {
                name, args: nested, ..
            },
            Type::Named {
                name: actual_name,
                args: actual_args,
                ..
            },
        ) if name == actual_name && nested.len() == actual_args.len() => {
            for (nested, actual) in nested.iter().zip(actual_args) {
                unify_template(args, nested, actual);
            }
        }
        (Type::Record(fields), Type::Record(actual_fields)) => {
            for (name, field) in fields {
                if let Some((_, actual_field)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    unify_template(args, field, actual_field);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn unify_core_template(args: &mut [CoreType], template: &CoreType, actual: &CoreType) {
    match (template, actual) {
        (
            CoreType::Named { name, args: nested },
            CoreType::Named {
                name: _actual_name,
                args: _actual_args,
            },
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>() {
                assign_core_type_arg(args, index, actual);
            }
        }
        (
            CoreType::Named { name, args: nested },
            CoreType::Named {
                name: actual_name,
                args: actual_args,
            },
        ) if name == actual_name && nested.len() == actual_args.len() => {
            for (nested, actual) in nested.iter().zip(actual_args) {
                unify_core_template(args, nested, actual);
            }
        }
        (CoreType::Record(fields), CoreType::Record(actual_fields)) => {
            for (name, field) in fields {
                if let Some((_, actual_field)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    unify_core_template(args, field, actual_field);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn substitute_type_parameters(template: &Type, args: &[Type]) -> Type {
    substitute_type_parameters_with(template, &mut |index| args.get(index).cloned())
}

pub(super) fn substitute_type_parameters_with(
    template: &Type,
    resolve: &mut impl FnMut(usize) -> Option<Type>,
) -> Type {
    match template {
        Type::Named {
            name, args: nested, ..
        } if name.starts_with("$param") && nested.is_empty() => name
            .trim_start_matches("$param")
            .parse::<usize>()
            .ok()
            .and_then(resolve)
            .unwrap_or(Type::Unknown),
        Type::Named {
            name,
            identity,
            args: nested,
        } => Type::Named {
            name: name.clone(),
            identity: identity.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_type_parameters_with(arg, resolve))
                .collect(),
        },
        Type::VariantRefinement {
            name,
            identity,
            args: nested,
            variants,
            unresolved_alternatives,
        } => Type::VariantRefinement {
            name: name.clone(),
            identity: identity.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_type_parameters_with(arg, resolve))
                .collect(),
            variants: variants.clone(),
            unresolved_alternatives: unresolved_alternatives.clone(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), substitute_type_parameters_with(ty, resolve)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: params
                .iter()
                .map(|ty| substitute_type_parameters_with(ty, resolve))
                .collect(),
            variadic: variadic
                .as_deref()
                .map(|ty| Box::new(substitute_type_parameters_with(ty, resolve))),
            return_type: Box::new(substitute_type_parameters_with(return_type, resolve)),
            effects: effects.clone(),
        },
        Type::Unknown => Type::Unknown,
    }
}

pub(super) fn visit_type_parameter_contributions(
    template: &Type,
    actual: &Type,
    visit: &mut impl FnMut(usize, &Type),
) {
    match (template, actual) {
        (
            Type::Named {
                name, args: nested, ..
            },
            actual,
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>() {
                visit(index, actual);
            }
        }
        (
            Type::Named {
                name,
                identity,
                args: nested,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            }
            | Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if crate::type_relations::same_type_identity(
            name,
            identity,
            actual_name,
            actual_identity,
        ) && nested.len() == actual_args.len() =>
        {
            for (nested, actual) in nested.iter().zip(actual_args) {
                visit_type_parameter_contributions(nested, actual, visit);
            }
        }
        (Type::Record(fields), Type::Record(actual_fields)) => {
            for (name, field) in fields {
                if let Some((_, actual_field)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    visit_type_parameter_contributions(field, actual_field, visit);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn visit_type_parameter_indices(template: &Type, visit: &mut impl FnMut(usize)) {
    let mut pending = vec![template];
    while let Some(ty) = pending.pop() {
        match ty {
            Type::Named { name, args, .. } if name.starts_with("$param") && args.is_empty() => {
                if let Ok(index) = name.trim_start_matches("$param").parse::<usize>() {
                    visit(index);
                }
            }
            Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
                pending.extend(args);
            }
            Type::Record(fields) => pending.extend(fields.iter().map(|(_, field)| field)),
            Type::Function {
                params,
                variadic,
                return_type,
                ..
            } => {
                pending.extend(params);
                pending.extend(variadic.as_deref());
                pending.push(return_type);
            }
            Type::Unknown => {}
        }
    }
}
