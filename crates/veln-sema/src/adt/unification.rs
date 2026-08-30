use super::type_operations::adt_args;
use super::{AdtDescriptor, AdtPayloadType, CoreType, Type};

pub(super) fn payload_type_from_args(
    ty: &Type,
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
) -> Option<Type> {
    match payload {
        AdtPayloadType::TypeParameter(index) => adt_args(ty, descriptor)?.get(*index).cloned(),
        AdtPayloadType::SelfType => Some(ty.clone()),
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

pub(super) fn merge_type_slot(slot: &mut Type, actual: &Type) {
    if actual == &Type::Unknown {
        return;
    }
    match (slot, actual) {
        (slot @ Type::Unknown, _) => *slot = actual.clone(),
        (
            Type::Named {
                name: slot_name,
                args: slot_args,
            },
            Type::Named {
                name: actual_name,
                args: actual_args,
            },
        ) if slot_name == actual_name && slot_args.len() == actual_args.len() => {
            for (slot_arg, actual_arg) in slot_args.iter_mut().zip(actual_args) {
                merge_type_slot(slot_arg, actual_arg);
            }
        }
        (Type::Record(slot_fields), Type::Record(actual_fields)) => {
            for (slot_name, slot_ty) in slot_fields {
                if let Some((_, actual_ty)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == slot_name)
                {
                    merge_type_slot(slot_ty, actual_ty);
                }
            }
        }
        (
            Type::Function {
                params: slot_params,
                variadic: slot_variadic,
                return_type: slot_return,
                effects: _,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: _,
            },
        ) if slot_params.len() == actual_params.len()
            && slot_variadic.is_some() == actual_variadic.is_some() =>
        {
            for (slot_param, actual_param) in slot_params.iter_mut().zip(actual_params) {
                merge_type_slot(slot_param, actual_param);
            }
            if let (Some(slot_variadic), Some(actual_variadic)) = (slot_variadic, actual_variadic) {
                merge_type_slot(slot_variadic, actual_variadic);
            }
            merge_type_slot(slot_return, actual_return);
        }
        _ => {}
    }
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
            Type::Named { name, args: nested },
            Type::Named {
                name: _actual_name,
                args: _actual_args,
            },
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>() {
                assign_type_arg(args, index, actual);
            }
        }
        (
            Type::Named { name, args: nested },
            Type::Named {
                name: actual_name,
                args: actual_args,
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
    match template {
        Type::Named { name, args: nested } if name.starts_with("$param") && nested.is_empty() => {
            name.trim_start_matches("$param")
                .parse::<usize>()
                .ok()
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(Type::Unknown)
        }
        Type::Named { name, args: nested } => Type::Named {
            name: name.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_type_parameters(arg, args))
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), substitute_type_parameters(ty, args)))
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
                .map(|ty| substitute_type_parameters(ty, args))
                .collect(),
            variadic: variadic
                .as_deref()
                .map(|ty| Box::new(substitute_type_parameters(ty, args))),
            return_type: Box::new(substitute_type_parameters(return_type, args)),
            effects: effects.clone(),
        },
        Type::Unknown => Type::Unknown,
    }
}

pub(super) fn substitute_core_type_parameters(template: &CoreType, args: &[CoreType]) -> CoreType {
    match template {
        CoreType::Named { name, args: nested }
            if name.starts_with("$param") && nested.is_empty() =>
        {
            name.trim_start_matches("$param")
                .parse::<usize>()
                .ok()
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(CoreType::Unknown)
        }
        CoreType::Named { name, args: nested } => CoreType::Named {
            name: name.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_core_type_parameters(arg, args))
                .collect(),
        },
        CoreType::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), substitute_core_type_parameters(ty, args)))
                .collect(),
        ),
        CoreType::Function {
            params,
            variadic,
            return_type,
            effects,
        } => CoreType::Function {
            params: params
                .iter()
                .map(|ty| substitute_core_type_parameters(ty, args))
                .collect(),
            variadic: variadic
                .as_deref()
                .map(|ty| Box::new(substitute_core_type_parameters(ty, args))),
            return_type: Box::new(substitute_core_type_parameters(return_type, args)),
            effects: effects.clone(),
        },
        CoreType::Unknown => CoreType::Unknown,
    }
}

pub(super) fn core_type_template(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args } => CoreType::Named {
            name: name.clone(),
            args: args.iter().map(core_type_template).collect(),
        },
        Type::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), core_type_template(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type_template).collect(),
            variadic: variadic.as_deref().map(core_type_template).map(Box::new),
            return_type: Box::new(core_type_template(return_type)),
            effects: effects.clone(),
        },
    }
}

pub(crate) trait NamedTypeArguments: Sized {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])>;
}

impl NamedTypeArguments for Type {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])> {
        let Self::Named { name, args } = self else {
            return None;
        };
        Some((name, args))
    }
}

impl NamedTypeArguments for CoreType {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])> {
        let Self::Named { name, args } = self else {
            return None;
        };
        Some((name, args))
    }
}

pub(super) fn named_part<'a, T: NamedTypeArguments>(
    ty: &'a T,
    name: &str,
    arity: usize,
) -> Option<&'a T> {
    let (ty_name, args) = ty.named_type_arguments()?;
    (ty_name == name && args.len() == arity)
        .then(|| args.first())
        .flatten()
}

pub(super) fn named_parts2<'a, T: NamedTypeArguments>(
    ty: &'a T,
    name: &str,
) -> Option<(&'a T, &'a T)> {
    let (ty_name, args) = ty.named_type_arguments()?;
    (ty_name == name && args.len() == 2).then(|| (&args[0], &args[1]))
}
