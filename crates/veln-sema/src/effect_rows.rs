use std::collections::BTreeSet;

use crate::semantic_model::Type;

pub(crate) type EffectRowSubstitutions = Vec<(String, Vec<String>)>;

pub(crate) fn call_bound_effect_row(
    params: &[Type],
    variadic: Option<&Type>,
    effects: &[String],
) -> Option<String> {
    effects
        .iter()
        .find_map(|effect| effect.strip_prefix("...").map(str::to_string))
        .or_else(|| params.iter().find_map(effect_row_in_type))
        .or_else(|| variadic.and_then(effect_row_in_type))
}

pub(crate) fn solve_call_bound_effect_row<'a>(
    row: Option<&str>,
    comparisons: impl IntoIterator<Item = (&'a Type, &'a Type)>,
) -> EffectRowSubstitutions {
    let Some(row) = row else {
        return Vec::new();
    };
    let mut lower_bound = BTreeSet::new();
    for (expected, actual) in comparisons {
        collect_lower_bounds(expected, actual, row, &mut lower_bound);
    }
    vec![(row.to_string(), lower_bound.into_iter().collect())]
}

pub(crate) fn instantiate_effect_rows(
    effects: &[String],
    row_substitutions: &EffectRowSubstitutions,
) -> Vec<String> {
    let mut instantiated = Vec::new();
    for effect in effects {
        if let Some(row) = effect.strip_prefix("...") {
            if let Some((_, substitution)) = row_substitutions
                .iter()
                .find(|(candidate, _)| candidate == row)
            {
                for substituted in substitution {
                    push_unique_effect(&mut instantiated, substituted);
                }
            } else {
                push_unique_effect(&mut instantiated, effect);
            }
        } else {
            push_unique_effect(&mut instantiated, effect);
        }
    }
    instantiated
}

pub(crate) fn instantiate_effect_rows_in_type(
    ty: &Type,
    row_substitutions: &EffectRowSubstitutions,
) -> Type {
    match ty {
        Type::Unknown => Type::Unknown,
        Type::Named {
            name,
            identity,
            args,
        } => Type::Named {
            name: name.clone(),
            identity: identity.clone(),
            args: args
                .iter()
                .map(|arg| instantiate_effect_rows_in_type(arg, row_substitutions))
                .collect(),
        },
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        } => Type::VariantRefinement {
            name: name.clone(),
            identity: identity.clone(),
            args: args
                .iter()
                .map(|arg| instantiate_effect_rows_in_type(arg, row_substitutions))
                .collect(),
            variants: variants.clone(),
            unresolved_alternatives: unresolved_alternatives.clone(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|(name, field)| {
                    (
                        name.clone(),
                        instantiate_effect_rows_in_type(field, row_substitutions),
                    )
                })
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
                .map(|param| instantiate_effect_rows_in_type(param, row_substitutions))
                .collect(),
            variadic: variadic.as_ref().map(|variadic| {
                Box::new(instantiate_effect_rows_in_type(variadic, row_substitutions))
            }),
            return_type: Box::new(instantiate_effect_rows_in_type(
                return_type,
                row_substitutions,
            )),
            effects: instantiate_effect_rows(effects, row_substitutions),
        },
    }
}

fn effect_row_in_type(ty: &Type) -> Option<String> {
    match ty {
        Type::Unknown => None,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            args.iter().find_map(effect_row_in_type)
        }
        Type::Record(fields) => fields
            .iter()
            .find_map(|(_, field)| effect_row_in_type(field)),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => effects
            .iter()
            .find_map(|effect| effect.strip_prefix("...").map(str::to_string))
            .or_else(|| params.iter().find_map(effect_row_in_type))
            .or_else(|| variadic.as_deref().and_then(effect_row_in_type))
            .or_else(|| effect_row_in_type(return_type)),
    }
}

fn collect_lower_bounds(
    expected: &Type,
    actual: &Type,
    row: &str,
    lower_bound: &mut BTreeSet<String>,
) {
    match (expected, actual) {
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
            collect_effect_inclusion_lower_bound(
                actual_effects,
                expected_effects,
                row,
                lower_bound,
            );
            for (expected_param, actual_param) in expected_params.iter().zip(actual_params) {
                collect_lower_bounds(actual_param, expected_param, row, lower_bound);
            }
            if let (Some(expected), Some(actual)) =
                (expected_variadic.as_deref(), actual_variadic.as_deref())
            {
                collect_lower_bounds(actual, expected, row, lower_bound);
            }
            collect_lower_bounds(expected_return, actual_return, row, lower_bound);
        }
        (Type::Record(expected), Type::Record(actual)) => {
            for (name, expected_field) in expected {
                if let Some((_, actual_field)) = actual.iter().find(|(actual, _)| actual == name) {
                    collect_lower_bounds(expected_field, actual_field, row, lower_bound);
                }
            }
        }
        (
            Type::Named {
                args: expected_args,
                ..
            }
            | Type::VariantRefinement {
                args: expected_args,
                ..
            },
            Type::Named {
                args: actual_args, ..
            }
            | Type::VariantRefinement {
                args: actual_args, ..
            },
        ) => {
            for (expected_arg, actual_arg) in expected_args.iter().zip(actual_args) {
                collect_lower_bounds(expected_arg, actual_arg, row, lower_bound);
                collect_lower_bounds(actual_arg, expected_arg, row, lower_bound);
            }
        }
        _ => {}
    }
}

fn collect_effect_inclusion_lower_bound(
    subset: &[String],
    superset: &[String],
    row: &str,
    lower_bound: &mut BTreeSet<String>,
) {
    let row_tail = format!("...{row}");
    if !superset.iter().any(|effect| effect == &row_tail) {
        return;
    }
    for effect in subset {
        if !effect.starts_with("...") && !superset.iter().any(|candidate| candidate == effect) {
            lower_bound.insert(effect.clone());
        }
    }
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_polarity_collects_shared_lower_bounds_independently_of_order() {
        let lower_expected = Type::function(Vec::new(), Type::unit(), vec!["...E".to_string()]);
        let lower_actual = Type::function(Vec::new(), Type::unit(), vec!["stdio".to_string()]);
        let upper_expected = Type::function(
            vec![Type::function(
                Vec::new(),
                Type::unit(),
                vec!["...E".to_string()],
            )],
            Type::unit(),
            Vec::new(),
        );
        let upper_actual = Type::function(
            vec![Type::function(
                Vec::new(),
                Type::unit(),
                vec!["stdio".to_string(), "db".to_string()],
            )],
            Type::unit(),
            Vec::new(),
        );

        let forward = solve_call_bound_effect_row(
            Some("E"),
            [
                (&lower_expected, &lower_actual),
                (&upper_expected, &upper_actual),
            ],
        );
        let reverse = solve_call_bound_effect_row(
            Some("E"),
            [
                (&upper_expected, &upper_actual),
                (&lower_expected, &lower_actual),
            ],
        );

        assert_eq!(forward, vec![("E".to_string(), vec!["stdio".to_string()])]);
        assert_eq!(reverse, forward);
    }

    #[test]
    fn instantiation_preserves_concrete_effects_and_empty_rows() {
        let ty = Type::function(
            Vec::new(),
            Type::unit(),
            vec!["fs".to_string(), "...E".to_string()],
        );

        let instantiated =
            instantiate_effect_rows_in_type(&ty, &vec![("E".to_string(), Vec::new())]);

        assert_eq!(
            instantiated,
            Type::function(Vec::new(), Type::unit(), vec!["fs".to_string()])
        );
    }
}
