use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StaticBooleanValue {
    True,
    False,
    Unknown,
}

pub(super) enum BooleanFormula {
    Constant(bool),
    Atom { index: usize, polarity: bool },
    Not(Box<Self>),
    Or(Box<Self>, Box<Self>),
    And(Box<Self>, Box<Self>),
}

impl BooleanFormula {
    pub(super) fn evaluate(&self, mask: usize) -> bool {
        match self {
            Self::Constant(value) => *value,
            Self::Atom { index, polarity } => ((mask & (1usize << index)) != 0) == *polarity,
            Self::Not(inner) => !inner.evaluate(mask),
            Self::Or(left, right) => left.evaluate(mask) || right.evaluate(mask),
            Self::And(left, right) => left.evaluate(mask) && right.evaluate(mask),
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct StaticBooleanOptions {
    classify_contract_contradictions: bool,
    classify_covering_numeric_bounds: bool,
}

pub(super) const MAX_STATIC_BOOLEAN_ATOMS: usize = 13;
pub(super) const MAX_PARTIAL_CASE_SPLIT_ATOMS: usize = 10;

pub(super) fn static_boolean_value(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, false, false)
}

pub(super) fn static_boolean_value_for_contract(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, true, true)
}

pub(super) fn static_boolean_value_with_literal_bounds(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, true, true)
}

pub(super) fn static_boolean_value_inner(
    predicate: &str,
    classify_contract_contradictions: bool,
    classify_covering_numeric_bounds: bool,
) -> StaticBooleanValue {
    let options = StaticBooleanOptions {
        classify_contract_contradictions,
        classify_covering_numeric_bounds,
    };
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(value) = static_boolean_literal_value(predicate) {
        return value;
    }
    if let Some(value) = static_boolean_top_level_shortcut(predicate) {
        return value;
    }
    if let Some(value) = static_boolean_negation_value(predicate, options) {
        return value;
    }
    if let Some(value) = static_boolean_top_level_tautology_value(predicate, options) {
        return value;
    }
    if let Some(value) = static_boolean_or_value(predicate, options) {
        return value;
    }
    if let Some(value) = static_boolean_top_level_contradiction_value(predicate, options) {
        return value;
    }
    if let Some(value) = static_boolean_and_value(predicate, options) {
        return value;
    }
    static_boolean_comparison_value(predicate).unwrap_or(StaticBooleanValue::Unknown)
}

pub(super) fn static_boolean_literal_value(predicate: &str) -> Option<StaticBooleanValue> {
    match predicate {
        "" => Some(StaticBooleanValue::Unknown),
        "true" => Some(StaticBooleanValue::True),
        "false" => Some(StaticBooleanValue::False),
        _ => None,
    }
}

pub(super) fn static_boolean_top_level_shortcut(predicate: &str) -> Option<StaticBooleanValue> {
    let top_level_or_count = split_top_level_keyword(predicate, "or").len();
    if top_level_or_count >= 128 && has_exhaustive_case_split_top_level_or_between(predicate, 7, 11)
    {
        return Some(StaticBooleanValue::True);
    }
    if top_level_or_count < 512
        && let Some(value @ (StaticBooleanValue::True | StaticBooleanValue::False)) =
            static_boolean_truth_table_value(predicate)
    {
        return Some(value);
    }
    None
}

pub(super) fn static_boolean_negation_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    negated_predicate_inner(predicate)
        .map(|inner| static_boolean_value_with_options(inner, options).negate())
}

pub(super) fn static_boolean_top_level_tautology_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    if has_complementary_top_level_clauses(predicate, "or")
        || has_negated_conjunction_top_level_or(predicate)
        || has_negated_disjunction_covered_by_disjuncts(predicate)
        || has_order_bound_transitive_implication_top_level_or(predicate)
        || (options.classify_covering_numeric_bounds
            && has_covering_numeric_literal_bounds_top_level_or(predicate))
        || has_conjunction_covered_by_complement_disjuncts(predicate)
        || has_factored_case_split_covered_by_complements(predicate)
        || has_partial_case_split_top_level_or(predicate)
        || has_case_split_top_level_or(predicate)
        || has_inclusive_total_order_top_level_or(predicate)
        || has_disequality_inclusive_order_split_top_level_or(predicate)
        || has_total_order_top_level_or(predicate)
        || has_disequality_strict_order_split_top_level_or(predicate)
    {
        Some(StaticBooleanValue::True)
    } else {
        None
    }
}

pub(super) fn static_boolean_or_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    split_top_level_keyword_operator(predicate, "or").map(|(left, right)| {
        if complementary_predicates(left, right) {
            StaticBooleanValue::True
        } else {
            static_boolean_value_with_options(left, options)
                .or(static_boolean_value_with_options(right, options))
        }
    })
}

pub(super) fn static_boolean_top_level_contradiction_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    if has_complementary_top_level_clauses(predicate, "and")
        || has_negated_disjunction_top_level_and(predicate)
        || has_disjunction_covered_by_complement_conjuncts(predicate)
        || has_partial_case_split_top_level_and(predicate)
        || has_resolved_complementary_disjunctions_top_level_and(predicate)
        || has_transitive_strict_order_cycle_top_level_and(predicate)
        || has_transitive_order_contradiction_top_level_and(predicate)
        || (options.classify_contract_contradictions
            && has_exclusive_numeric_literal_bounds_top_level_and(predicate))
        || (options.classify_contract_contradictions
            && has_exclusive_literal_equalities_top_level_and(predicate))
        || has_exclusive_inclusive_order_top_level_and(predicate)
        || has_exclusive_order_top_level_and(predicate)
    {
        Some(StaticBooleanValue::False)
    } else {
        None
    }
}

pub(super) fn static_boolean_and_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    split_top_level_keyword_operator(predicate, "and").map(|(left, right)| {
        if complementary_predicates(left, right) {
            StaticBooleanValue::False
        } else {
            static_boolean_value_with_options(left, options)
                .and(static_boolean_value_with_options(right, options))
        }
    })
}

pub(super) fn static_boolean_comparison_value(predicate: &str) -> Option<StaticBooleanValue> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            return static_literal_comparison(left, operator, right)
                .or_else(|| static_complementary_predicate_comparison(left, operator, right))
                .or_else(|| static_boolean_formula_comparison(left, operator, right))
                .or_else(|| static_same_shape_comparison(left, operator, right))
                .map(StaticBooleanValue::from);
        }
    }
    None
}

pub(super) fn static_boolean_value_with_options(
    predicate: &str,
    options: StaticBooleanOptions,
) -> StaticBooleanValue {
    static_boolean_value_inner(
        predicate,
        options.classify_contract_contradictions,
        options.classify_covering_numeric_bounds,
    )
}

pub(super) fn static_boolean_truth_table_value(predicate: &str) -> Option<StaticBooleanValue> {
    let mut atoms = Vec::new();
    collect_boolean_formula_atoms(predicate, &mut atoms)?;
    if atoms.is_empty() || atoms.len() > MAX_STATIC_BOOLEAN_ATOMS {
        return None;
    }
    let formula = compile_boolean_formula(predicate, &atoms)?;

    let mut saw_true = false;
    let mut saw_false = false;
    for mask in 0..(1usize << atoms.len()) {
        match formula.evaluate(mask) {
            true => saw_true = true,
            false => saw_false = true,
        }
        if saw_true && saw_false {
            return Some(StaticBooleanValue::Unknown);
        }
    }

    Some(if saw_true {
        StaticBooleanValue::True
    } else {
        StaticBooleanValue::False
    })
}

pub(super) fn collect_boolean_formula_atoms(
    predicate: &str,
    atoms: &mut Vec<String>,
) -> Option<()> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() || matches!(predicate, "true" | "false") {
        return Some(());
    }
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return collect_boolean_formula_atoms(inner, atoms);
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "or") {
        collect_boolean_formula_atoms(left, atoms)?;
        collect_boolean_formula_atoms(right, atoms)?;
        return Some(());
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "and") {
        collect_boolean_formula_atoms(left, atoms)?;
        collect_boolean_formula_atoms(right, atoms)?;
        return Some(());
    }
    if static_comparison_value(predicate).is_some() {
        return Some(());
    }

    let (shape, _) = normalized_predicate_polarity(predicate);
    if !atoms.iter().any(|atom| atom == &shape) {
        atoms.push(shape);
    }
    Some(())
}

#[cfg(test)]
pub(super) fn eval_boolean_formula(predicate: &str, atoms: &[String], mask: usize) -> Option<bool> {
    compile_boolean_formula(predicate, atoms).map(|formula| formula.evaluate(mask))
}

pub(super) fn compile_boolean_formula(predicate: &str, atoms: &[String]) -> Option<BooleanFormula> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate == "true" {
        return Some(BooleanFormula::Constant(true));
    }
    if predicate == "false" {
        return Some(BooleanFormula::Constant(false));
    }
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return Some(BooleanFormula::Not(Box::new(compile_boolean_formula(
            inner, atoms,
        )?)));
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "or") {
        return Some(BooleanFormula::Or(
            Box::new(compile_boolean_formula(left, atoms)?),
            Box::new(compile_boolean_formula(right, atoms)?),
        ));
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "and") {
        return Some(BooleanFormula::And(
            Box::new(compile_boolean_formula(left, atoms)?),
            Box::new(compile_boolean_formula(right, atoms)?),
        ));
    }
    if let Some(value) = static_comparison_value(predicate) {
        return Some(BooleanFormula::Constant(value));
    }

    let (shape, polarity) = normalized_predicate_polarity(predicate);
    let index = atoms.iter().position(|atom| atom == &shape)?;
    Some(BooleanFormula::Atom { index, polarity })
}
