use crate::semantic_model::Type;

pub(crate) type EffectRowSubstitutions = Vec<(String, Vec<String>)>;

pub(crate) fn collect_effect_row_substitution(
    expected: &Type,
    actual: &Type,
    row_substitutions: &mut EffectRowSubstitutions,
) {
    let (
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
    ) = (expected, actual)
    else {
        return;
    };

    for effect in expected_effects {
        let Some(row) = effect.strip_prefix("...") else {
            continue;
        };
        let concrete = actual_effects.iter().filter(|actual_effect| {
            !expected_effects
                .iter()
                .any(|expected_effect| expected_effect == *actual_effect)
        });
        merge_effect_row_substitution(row_substitutions, row, concrete);
    }

    for (expected_param, actual_param) in expected_params.iter().zip(actual_params) {
        collect_effect_row_substitution(expected_param, actual_param, row_substitutions);
    }
    if let (Some(expected), Some(actual)) =
        (expected_variadic.as_deref(), actual_variadic.as_deref())
    {
        collect_effect_row_substitution(expected, actual, row_substitutions);
    }
    collect_effect_row_substitution(expected_return, actual_return, row_substitutions);
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

fn merge_effect_row_substitution<'a>(
    row_substitutions: &mut EffectRowSubstitutions,
    row: &str,
    effects: impl IntoIterator<Item = &'a String>,
) {
    if let Some((_, existing)) = row_substitutions
        .iter_mut()
        .find(|(existing_row, _)| existing_row == row)
    {
        for effect in effects {
            push_unique_effect(existing, effect);
        }
        return;
    }
    let mut unique = Vec::new();
    for effect in effects {
        push_unique_effect(&mut unique, effect);
    }
    row_substitutions.push((row.to_string(), unique));
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
    fn nested_function_positions_contribute_to_their_effect_rows() {
        let expected = Type::variadic_function(
            vec![Type::function(
                Vec::new(),
                Type::unit(),
                vec!["...Param".to_string()],
            )],
            Type::function(Vec::new(), Type::unit(), vec!["...Variadic".to_string()]),
            Type::function(Vec::new(), Type::unit(), vec!["...Return".to_string()]),
            Vec::new(),
        );
        let actual = Type::variadic_function(
            vec![Type::function(
                Vec::new(),
                Type::unit(),
                vec!["database".to_string()],
            )],
            Type::function(Vec::new(), Type::unit(), vec!["network".to_string()]),
            Type::function(Vec::new(), Type::unit(), vec!["stdio".to_string()]),
            Vec::new(),
        );
        let mut substitutions = EffectRowSubstitutions::new();

        collect_effect_row_substitution(&expected, &actual, &mut substitutions);

        assert_eq!(
            substitutions,
            vec![
                ("Param".to_string(), vec!["database".to_string()]),
                ("Variadic".to_string(), vec!["network".to_string()]),
                ("Return".to_string(), vec!["stdio".to_string()]),
            ]
        );
    }

    #[test]
    fn large_effect_row_preserves_order_and_removes_duplicates() {
        let expected = Type::function(Vec::new(), Type::unit(), vec!["...E".to_string()]);
        let mut actual_effects = (0..256)
            .map(|index| format!("effect_{index}"))
            .collect::<Vec<_>>();
        actual_effects.push("effect_128".to_string());
        let actual = Type::function(Vec::new(), Type::unit(), actual_effects);
        let mut substitutions = EffectRowSubstitutions::new();

        collect_effect_row_substitution(&expected, &actual, &mut substitutions);
        let instantiated =
            instantiate_effect_rows(&["base".to_string(), "...E".to_string()], &substitutions);

        assert_eq!(instantiated.len(), 257);
        assert_eq!(instantiated.first().map(String::as_str), Some("base"));
        assert_eq!(
            instantiated.get(129).map(String::as_str),
            Some("effect_128")
        );
        assert_eq!(instantiated.last().map(String::as_str), Some("effect_255"));
    }
}
