use veln_core::CoreType;

use crate::semantic_model::Type;

pub(crate) fn substitute_core_type_parameters(template: &CoreType, args: &[CoreType]) -> CoreType {
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

pub(crate) fn core_type_template(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args, .. } => CoreType::Named {
            name: name.clone(),
            args: args.iter().map(core_type_template).collect(),
        },
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => CoreType::Named {
            name: identity.rsplit("::").next().unwrap_or(name).to_string(),
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
