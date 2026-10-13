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
    Or(Vec<Self>),
    And(Vec<Self>),
}

impl BooleanFormula {
    pub(super) fn evaluate(&self, mask: usize) -> bool {
        match self {
            Self::Constant(value) => *value,
            Self::Atom { index, polarity } => ((mask & (1usize << index)) != 0) == *polarity,
            Self::Not(inner) => !inner.evaluate(mask),
            Self::Or(clauses) => clauses.iter().any(|clause| clause.evaluate(mask)),
            Self::And(clauses) => clauses.iter().all(|clause| clause.evaluate(mask)),
        }
    }
}

#[cfg(test)]
thread_local! {
    static STATIC_BOOLEAN_LEAF_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(crate) fn reset_static_boolean_work() {
    STATIC_BOOLEAN_LEAF_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn take_static_boolean_work() -> usize {
    STATIC_BOOLEAN_LEAF_VISITS.replace(0)
}

#[cfg(test)]
fn record_static_boolean_leaf_visit() {
    STATIC_BOOLEAN_LEAF_VISITS.set(STATIC_BOOLEAN_LEAF_VISITS.get() + 1);
}

#[cfg(not(test))]
fn record_static_boolean_leaf_visit() {}

#[derive(Clone, Copy)]
pub(super) struct StaticBooleanOptions {
    classify_contract_contradictions: bool,
    classify_covering_numeric_bounds: bool,
}

pub(super) const MAX_STATIC_BOOLEAN_ATOMS: usize = 13;
pub(super) const MAX_PARTIAL_CASE_SPLIT_ATOMS: usize = 10;
const MAX_RECURSIVE_REMAINDER_BOOLEAN_CLAUSES: usize = 512;

pub(super) fn static_boolean_value(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, false, false)
}

pub(super) fn static_boolean_value_for_contract(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, true, true)
}

pub(super) fn static_boolean_value_for_remainder_contract(predicate: &str) -> StaticBooleanValue {
    if !has_wide_boolean_composition(predicate) {
        return static_boolean_value_for_contract(predicate);
    }
    static_boolean_value_for_wide_remainder_contract(predicate)
}

fn has_wide_boolean_composition(predicate: &str) -> bool {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return has_wide_boolean_composition(inner);
    }

    let disjuncts = split_top_level_keyword(predicate, "or");
    if disjuncts.len() > 1 {
        return disjuncts.len() > MAX_RECURSIVE_REMAINDER_BOOLEAN_CLAUSES
            || disjuncts.into_iter().any(has_wide_boolean_composition);
    }

    let conjuncts = split_top_level_keyword(predicate, "and");
    conjuncts.len() > MAX_RECURSIVE_REMAINDER_BOOLEAN_CLAUSES
        || (conjuncts.len() > 1 && conjuncts.into_iter().any(has_wide_boolean_composition))
}

fn static_boolean_value_for_wide_remainder_contract(predicate: &str) -> StaticBooleanValue {
    let options = StaticBooleanOptions {
        classify_contract_contradictions: true,
        classify_covering_numeric_bounds: true,
    };
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(value) = static_boolean_literal_value(predicate) {
        return value;
    }
    let literal_composition = static_literal_boolean_composition_value(predicate);
    if literal_composition != StaticBooleanValue::Unknown {
        return literal_composition;
    }
    if let Some(value) = static_boolean_top_level_shortcut(predicate) {
        return value;
    }
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return static_boolean_value_for_remainder_contract(inner).negate();
    }
    if let Some(value) = static_boolean_top_level_tautology_value(predicate, options) {
        return value;
    }
    if is_oversized_partial_case_split_ladder(predicate) {
        return StaticBooleanValue::Unknown;
    }

    let disjuncts = split_top_level_keyword(predicate, "or");
    if disjuncts.len() > 1 {
        return static_boolean_value_for_wide_clauses(disjuncts, true);
    }
    if let Some(value) = static_boolean_top_level_contradiction_value(predicate, options) {
        return value;
    }

    let conjuncts = split_top_level_keyword(predicate, "and");
    if conjuncts.len() > 1 {
        return static_boolean_value_for_wide_clauses(conjuncts, false);
    }

    static_boolean_value_for_contract(predicate)
}

fn static_boolean_value_for_wide_clauses(
    clauses: Vec<&str>,
    disjunction: bool,
) -> StaticBooleanValue {
    let mut unknown = Vec::new();
    for clause in clauses {
        let value = static_boolean_value_for_remainder_contract(clause);
        if (disjunction && value == StaticBooleanValue::True)
            || (!disjunction && value == StaticBooleanValue::False)
        {
            return value;
        }
        if value == StaticBooleanValue::Unknown {
            unknown.push(clause);
        }
    }

    if unknown.is_empty() {
        return if disjunction {
            StaticBooleanValue::False
        } else {
            StaticBooleanValue::True
        };
    }
    if unknown.len() <= MAX_RECURSIVE_REMAINDER_BOOLEAN_CLAUSES
        && unknown
            .iter()
            .all(|clause| !has_wide_boolean_composition(clause))
    {
        let operator = if disjunction { " or " } else { " and " };
        return static_boolean_value_for_contract(&unknown.join(operator));
    }
    if let Some(value) = static_boolean_prefix_truth_table_value(&unknown, disjunction) {
        return value;
    }
    StaticBooleanValue::Unknown
}

fn static_boolean_prefix_truth_table_value(
    clauses: &[&str],
    disjunction: bool,
) -> Option<StaticBooleanValue> {
    let mut atoms = Vec::new();
    let mut values = vec![!disjunction];
    let mut atom_count = 0usize;

    for clause in clauses {
        collect_boolean_formula_atoms(clause, &mut atoms)?;
        while atom_count < atoms.len() {
            let previous = values.clone();
            values.extend_from_slice(&previous);
            atom_count += 1;
        }
        let formula = compile_boolean_formula(clause, &atoms)?;
        for (mask, value) in values.iter_mut().enumerate() {
            let clause_value = formula.evaluate(mask);
            *value = if disjunction {
                *value || clause_value
            } else {
                *value && clause_value
            };
        }
        if disjunction && values.iter().all(|value| *value) {
            return Some(StaticBooleanValue::True);
        }
        if !disjunction && values.iter().all(|value| !*value) {
            return Some(StaticBooleanValue::False);
        }
    }

    None
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
    let literal_composition = static_literal_boolean_composition_value(predicate);
    if literal_composition != StaticBooleanValue::Unknown {
        return literal_composition;
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
    if is_oversized_partial_case_split_ladder(predicate) {
        return StaticBooleanValue::Unknown;
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

fn static_literal_boolean_composition_value(predicate: &str) -> StaticBooleanValue {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(value) = static_boolean_literal_value(predicate) {
        record_static_boolean_leaf_visit();
        return value;
    }
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return static_literal_boolean_composition_value(inner).negate();
    }

    let disjuncts = split_top_level_keyword(predicate, "or");
    if disjuncts.len() > 1 {
        let mut value = StaticBooleanValue::False;
        for disjunct in disjuncts {
            value = value.or(static_literal_boolean_composition_value(disjunct));
            if value == StaticBooleanValue::True {
                break;
            }
        }
        return value;
    }

    let conjuncts = split_top_level_keyword(predicate, "and");
    if conjuncts.len() > 1 {
        let mut value = StaticBooleanValue::True;
        for conjunct in conjuncts {
            value = value.and(static_literal_boolean_composition_value(conjunct));
            if value == StaticBooleanValue::False {
                break;
            }
        }
        return value;
    }

    record_static_boolean_leaf_visit();
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            return static_literal_comparison(left, operator, right)
                .map(StaticBooleanValue::from)
                .unwrap_or(StaticBooleanValue::Unknown);
        }
    }
    StaticBooleanValue::Unknown
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
        && let Some(value) = static_boolean_truth_table_value(predicate)
        && (matches!(value, StaticBooleanValue::True | StaticBooleanValue::False)
            || is_opaque_call_conjunction(predicate))
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
    let mut pending = vec![predicate];
    while let Some(predicate) = pending.pop() {
        let predicate = strip_balanced_outer_parens(predicate.trim());
        if predicate.is_empty() || matches!(predicate, "true" | "false") {
            continue;
        }
        if let Some(inner) = whole_negated_predicate_inner(predicate) {
            pending.push(inner);
            continue;
        }
        let disjuncts = split_top_level_keyword(predicate, "or");
        if disjuncts.len() > 1 {
            pending.extend(disjuncts.into_iter().rev());
            continue;
        }
        let conjuncts = split_top_level_keyword(predicate, "and");
        if conjuncts.len() > 1 {
            pending.extend(conjuncts.into_iter().rev());
            continue;
        }
        if static_comparison_value(predicate).is_some() {
            continue;
        }

        let (shape, _) = normalized_predicate_polarity(predicate);
        if !atoms.iter().any(|atom| atom == &shape) {
            atoms.push(shape);
            if atoms.len() > MAX_STATIC_BOOLEAN_ATOMS {
                return None;
            }
        }
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
    let disjuncts = split_top_level_keyword(predicate, "or");
    if disjuncts.len() > 1 {
        return disjuncts
            .into_iter()
            .map(|clause| compile_boolean_formula(clause, atoms))
            .collect::<Option<Vec<_>>>()
            .map(BooleanFormula::Or);
    }
    let conjuncts = split_top_level_keyword(predicate, "and");
    if conjuncts.len() > 1 {
        return conjuncts
            .into_iter()
            .map(|clause| compile_boolean_formula(clause, atoms))
            .collect::<Option<Vec<_>>>()
            .map(BooleanFormula::And);
    }
    if let Some(value) = static_comparison_value(predicate) {
        return Some(BooleanFormula::Constant(value));
    }

    let (shape, polarity) = normalized_predicate_polarity(predicate);
    let index = atoms.iter().position(|atom| atom == &shape)?;
    Some(BooleanFormula::Atom { index, polarity })
}
