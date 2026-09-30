use veln_core::CoreType;

use crate::semantic_model::Type;

pub(crate) fn core_type(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args } if name == "WallTime" && args.is_empty() => {
            CoreType::Record(vec![
                ("unix_seconds".to_string(), CoreType::int()),
                ("nanosecond".to_string(), CoreType::int()),
            ])
        }
        Type::Named { name, args } => {
            CoreType::named(name.clone(), args.iter().map(core_type).collect())
        }
        Type::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), core_type(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type).collect(),
            variadic: variadic.as_deref().map(core_type).map(Box::new),
            return_type: Box::new(core_type(return_type)),
            effects: effects.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_time_lowers_to_its_runtime_record_shape() {
        assert_eq!(
            core_type(&Type::named("WallTime", Vec::new())),
            CoreType::Record(vec![
                ("unix_seconds".to_string(), CoreType::int()),
                ("nanosecond".to_string(), CoreType::int()),
            ])
        );
    }
}
