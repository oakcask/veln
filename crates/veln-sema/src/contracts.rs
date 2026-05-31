use veln_ast::ContractKind;

use crate::types::{Binding, Type};

pub(crate) enum ContractValidation {
    Valid,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
    MissingField { base_type: String, field: String },
}

pub(crate) struct ContractCall {
    pub(crate) callee: String,
    pub(crate) args: Vec<String>,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
        ContractKind::Invariant => "invariant",
    }
}

pub(crate) fn predicate_is_statically_true(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::True
}

pub(crate) fn predicate_is_statically_true_with_literal_bounds(predicate: &str) -> bool {
    static_boolean_value_with_literal_bounds(predicate) == StaticBooleanValue::True
}

pub(crate) fn contract_predicate_is_statically_true(predicate: &str) -> bool {
    static_boolean_value_for_contract(predicate) == StaticBooleanValue::True
}

pub(crate) fn predicate_is_statically_false(predicate: &str) -> bool {
    static_boolean_value(predicate) == StaticBooleanValue::False
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StaticBooleanValue {
    True,
    False,
    Unknown,
}

#[derive(Clone, Copy)]
struct StaticBooleanOptions {
    classify_contract_contradictions: bool,
    classify_covering_numeric_bounds: bool,
}

const MAX_STATIC_BOOLEAN_ATOMS: usize = 13;
const MAX_PARTIAL_CASE_SPLIT_ATOMS: usize = 10;

fn static_boolean_value(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, false, false)
}

fn static_boolean_value_for_contract(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, true, true)
}

fn static_boolean_value_with_literal_bounds(predicate: &str) -> StaticBooleanValue {
    static_boolean_value_inner(predicate, true, true)
}

fn static_boolean_value_inner(
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

fn static_boolean_literal_value(predicate: &str) -> Option<StaticBooleanValue> {
    match predicate {
        "" => Some(StaticBooleanValue::Unknown),
        "true" => Some(StaticBooleanValue::True),
        "false" => Some(StaticBooleanValue::False),
        _ => None,
    }
}

fn static_boolean_top_level_shortcut(predicate: &str) -> Option<StaticBooleanValue> {
    let top_level_or_count = split_top_level_keyword(predicate, "or").len();
    if top_level_or_count >= 512 && has_exhaustive_case_split_top_level_or_between(predicate, 2, 11)
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

fn static_boolean_negation_value(
    predicate: &str,
    options: StaticBooleanOptions,
) -> Option<StaticBooleanValue> {
    negated_predicate_inner(predicate)
        .map(|inner| static_boolean_value_with_options(inner, options).negate())
}

fn static_boolean_top_level_tautology_value(
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

fn static_boolean_or_value(
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

fn static_boolean_top_level_contradiction_value(
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

fn static_boolean_and_value(
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

fn static_boolean_comparison_value(predicate: &str) -> Option<StaticBooleanValue> {
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

fn static_boolean_value_with_options(
    predicate: &str,
    options: StaticBooleanOptions,
) -> StaticBooleanValue {
    static_boolean_value_inner(
        predicate,
        options.classify_contract_contradictions,
        options.classify_covering_numeric_bounds,
    )
}

fn static_boolean_truth_table_value(predicate: &str) -> Option<StaticBooleanValue> {
    let mut atoms = Vec::new();
    collect_boolean_formula_atoms(predicate, &mut atoms)?;
    if atoms.is_empty() || atoms.len() > MAX_STATIC_BOOLEAN_ATOMS {
        return None;
    }

    let mut saw_true = false;
    let mut saw_false = false;
    for mask in 0..(1usize << atoms.len()) {
        match eval_boolean_formula(predicate, &atoms, mask)? {
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

fn collect_boolean_formula_atoms(predicate: &str, atoms: &mut Vec<String>) -> Option<()> {
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

fn eval_boolean_formula(predicate: &str, atoms: &[String], mask: usize) -> Option<bool> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate == "true" {
        return Some(true);
    }
    if predicate == "false" {
        return Some(false);
    }
    if let Some(inner) = whole_negated_predicate_inner(predicate) {
        return eval_boolean_formula(inner, atoms, mask).map(|value| !value);
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "or") {
        return Some(
            eval_boolean_formula(left, atoms, mask)? || eval_boolean_formula(right, atoms, mask)?,
        );
    }
    if let Some((left, right)) = split_top_level_keyword_operator(predicate, "and") {
        return Some(
            eval_boolean_formula(left, atoms, mask)? && eval_boolean_formula(right, atoms, mask)?,
        );
    }
    if let Some(value) = static_comparison_value(predicate) {
        return Some(value);
    }

    let (shape, polarity) = normalized_predicate_polarity(predicate);
    let index = atoms.iter().position(|atom| atom == &shape)?;
    Some(((mask & (1usize << index)) != 0) == polarity)
}

fn static_comparison_value(predicate: &str) -> Option<bool> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            return static_literal_comparison(left, operator, right)
                .or_else(|| static_complementary_predicate_comparison(left, operator, right))
                .or_else(|| static_boolean_formula_comparison(left, operator, right))
                .or_else(|| static_same_shape_comparison(left, operator, right));
        }
    }
    None
}

fn static_boolean_formula_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    if !matches!(operator, "==" | "!=") {
        return None;
    }

    let mut atoms = Vec::new();
    collect_boolean_formula_atoms(left, &mut atoms)?;
    collect_boolean_formula_atoms(right, &mut atoms)?;
    if atoms.is_empty() || atoms.len() > MAX_STATIC_BOOLEAN_ATOMS {
        return None;
    }

    let mut saw_true = false;
    let mut saw_false = false;
    for mask in 0..(1usize << atoms.len()) {
        let left_value = eval_boolean_formula(left, &atoms, mask)?;
        let right_value = eval_boolean_formula(right, &atoms, mask)?;
        let comparison = match operator {
            "==" => left_value == right_value,
            "!=" => left_value != right_value,
            _ => return None,
        };
        saw_true |= comparison;
        saw_false |= !comparison;
        if saw_true && saw_false {
            return None;
        }
    }

    Some(saw_true)
}

fn complementary_predicates(left: &str, right: &str) -> bool {
    let (left_shape, left_polarity) = normalized_predicate_polarity(left);
    let (right_shape, right_polarity) = normalized_predicate_polarity(right);
    (left_shape == right_shape && left_polarity != right_polarity)
        || complementary_comparisons(left, right)
}

fn normalized_predicate_polarity(predicate: &str) -> (String, bool) {
    if let Some(inner) = negated_predicate_inner(predicate) {
        let (shape, polarity) = normalized_predicate_polarity(inner);
        return (shape, !polarity);
    }
    if let Some(alias) = boolean_literal_alias_shape(predicate) {
        return alias;
    }
    (predicate_shape(predicate), true)
}

fn has_complementary_top_level_clauses(predicate: &str, keyword: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, keyword);
    if clauses.len() <= 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, left)| {
        clauses
            .iter()
            .skip(index + 1)
            .any(|right| complementary_predicates(left, right))
    })
}

fn has_negated_conjunction_top_level_or(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "or");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, left)| {
        clauses.iter().skip(index + 1).any(|right| {
            negated_conjunction_contains(left, right) || negated_conjunction_contains(right, left)
        })
    })
}

fn negated_conjunction_contains(negated: &str, predicate: &str) -> bool {
    let Some(inner) = negated_predicate_inner(negated) else {
        return false;
    };
    let clauses = flattened_keyword_clauses(inner, "and");
    clauses.len() > 1
        && clauses
            .iter()
            .any(|clause| same_predicate(clause, predicate))
}

fn has_negated_disjunction_top_level_and(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, left)| {
        clauses.iter().skip(index + 1).any(|right| {
            negated_disjunction_contains(left, right) || negated_disjunction_contains(right, left)
        })
    })
}

fn has_negated_disjunction_covered_by_disjuncts(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 3 {
        return false;
    }
    disjuncts.iter().any(|disjunct| {
        let Some(inner) = negated_predicate_inner(disjunct) else {
            return false;
        };
        let inner_disjuncts = non_static_disjuncts(inner);
        inner_disjuncts.len() > 1
            && inner_disjuncts.iter().all(|inner_disjunct| {
                disjuncts
                    .iter()
                    .any(|outer_disjunct| same_predicate(inner_disjunct, outer_disjunct))
            })
    })
}

fn negated_disjunction_contains(negated: &str, predicate: &str) -> bool {
    let Some(inner) = negated_predicate_inner(negated) else {
        return false;
    };
    let clauses = flattened_keyword_clauses(inner, "or");
    clauses.len() > 1
        && clauses
            .iter()
            .any(|clause| same_predicate(clause, predicate))
}

fn has_conjunction_covered_by_complement_disjuncts(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 3 {
        return false;
    }
    disjuncts.iter().enumerate().any(|(index, disjunct)| {
        let conjuncts = flattened_keyword_clauses(disjunct, "and");
        let non_static_conjuncts: Vec<_> = conjuncts
            .into_iter()
            .filter(|conjunct| static_boolean_value(conjunct) != StaticBooleanValue::True)
            .collect();
        non_static_conjuncts.len() > 1
            && non_static_conjuncts.iter().all(|conjunct| {
                disjuncts.iter().enumerate().any(|(other_index, other)| {
                    index != other_index && complementary_predicates(conjunct, other)
                })
            })
    })
}

fn has_disjunction_covered_by_complement_conjuncts(predicate: &str) -> bool {
    let conjuncts = flattened_keyword_clauses(predicate, "and");
    if conjuncts.len() < 3 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(index, conjunct)| {
        let disjuncts = flattened_keyword_clauses(conjunct, "or");
        let non_static_disjuncts: Vec<_> = disjuncts
            .into_iter()
            .filter(|disjunct| static_boolean_value(disjunct) != StaticBooleanValue::False)
            .collect();
        non_static_disjuncts.len() > 1
            && non_static_disjuncts.iter().all(|disjunct| {
                conjuncts.iter().enumerate().any(|(other_index, other)| {
                    index != other_index && complementary_predicates(disjunct, other)
                })
            })
    })
}

fn has_resolved_complementary_disjunctions_top_level_and(predicate: &str) -> bool {
    let conjuncts = flattened_keyword_clauses(predicate, "and");
    if conjuncts.len() < 3 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(left_index, left)| {
        let left_disjuncts = flattened_keyword_clauses(left, "or");
        left_disjuncts.len() == 2
            && conjuncts
                .iter()
                .enumerate()
                .skip(left_index + 1)
                .any(|(_, right)| {
                    let right_disjuncts = flattened_keyword_clauses(right, "or");
                    right_disjuncts.len() == 2
                        && resolvable_disjunction_pair_is_contradicted(
                            &conjuncts,
                            &left_disjuncts,
                            &right_disjuncts,
                        )
                })
    })
}

fn resolvable_disjunction_pair_is_contradicted(
    conjuncts: &[&str],
    left: &[&str],
    right: &[&str],
) -> bool {
    left.iter().enumerate().any(|(left_index, left_disjunct)| {
        right
            .iter()
            .enumerate()
            .any(|(right_index, right_disjunct)| {
                same_predicate(left_disjunct, right_disjunct)
                    && complementary_predicates(left[1 - left_index], right[1 - right_index])
                    && conjuncts
                        .iter()
                        .any(|conjunct| complementary_predicates(left_disjunct, conjunct))
            })
    })
}

fn has_transitive_order_contradiction_top_level_and(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() < 2 {
        return false;
    }
    let edges: Vec<_> = clauses
        .iter()
        .flat_map(|clause| order_bound_transitive_edges(clause))
        .collect();
    if edges.is_empty() {
        return false;
    }

    clauses.iter().any(|clause| {
        equality_shape(clause).is_some_and(|(left, right)| {
            order_bound_edges_imply(&edges, &left, &right, true)
                || order_bound_edges_imply(&edges, &right, &left, true)
        }) || disequality_shape(clause).is_some_and(|(left, right)| {
            order_bound_edges_imply_non_strict(&edges, &left, &right)
                && order_bound_edges_imply_non_strict(&edges, &right, &left)
        })
    })
}

fn has_transitive_strict_order_cycle_top_level_and(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() < 2 {
        return false;
    }
    let edges: Vec<_> = clauses
        .iter()
        .flat_map(|clause| order_bound_transitive_edges(clause))
        .collect();
    if edges.len() < 2 {
        return false;
    }

    edges
        .iter()
        .any(|edge| order_bound_edges_imply(&edges, &edge.right, &edge.left, !edge.strict))
}

fn has_factored_case_split_covered_by_complements(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 3 {
        return false;
    }
    disjuncts.iter().enumerate().any(|(left_index, left)| {
        let left_conjuncts = non_static_conjuncts(left);
        if left_conjuncts.len() < 2 {
            return false;
        }
        disjuncts
            .iter()
            .enumerate()
            .skip(left_index + 1)
            .any(|(_, right)| {
                let right_conjuncts = non_static_conjuncts(right);
                if right_conjuncts.len() != left_conjuncts.len() {
                    return false;
                }
                let mut common = Vec::new();
                let mut complementary_pair_found = false;
                let mut unmatched_right = right_conjuncts.clone();
                for left_conjunct in &left_conjuncts {
                    if let Some(position) = unmatched_right
                        .iter()
                        .position(|right_conjunct| same_predicate(left_conjunct, right_conjunct))
                    {
                        common.push(*left_conjunct);
                        unmatched_right.remove(position);
                        continue;
                    }
                    if let Some(position) = unmatched_right.iter().position(|right_conjunct| {
                        complementary_predicates(left_conjunct, right_conjunct)
                    }) {
                        if complementary_pair_found {
                            return false;
                        }
                        complementary_pair_found = true;
                        unmatched_right.remove(position);
                        continue;
                    }
                    return false;
                }
                complementary_pair_found
                    && !common.is_empty()
                    && common.iter().all(|conjunct| {
                        disjuncts.iter().any(|disjunct| {
                            !same_predicate(disjunct, left)
                                && !same_predicate(disjunct, right)
                                && complementary_predicates(conjunct, disjunct)
                        })
                    })
            })
    })
}

fn has_partial_case_split_top_level_or(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 3 {
        return false;
    }
    let mut bases: Vec<&str> = Vec::new();
    for disjunct in &disjuncts {
        for conjunct in non_static_conjuncts(disjunct) {
            if bases.iter().all(|base| {
                !same_predicate(base, conjunct) && !complementary_predicates(base, conjunct)
            }) {
                bases.push(conjunct);
                if bases.len() > MAX_PARTIAL_CASE_SPLIT_ATOMS {
                    return false;
                }
            }
        }
    }
    if bases.len() < 2 {
        return false;
    }

    let Some(assignment_count) = 1usize.checked_shl(bases.len() as u32) else {
        return false;
    };
    let mut covered = vec![false; assignment_count];
    let mut covered_count = 0;
    for disjunct in disjuncts {
        let Some(assignments) = partial_case_split_covered_assignments(disjunct, &bases) else {
            continue;
        };
        for assignment in assignments {
            if !covered[assignment] {
                covered[assignment] = true;
                covered_count += 1;
                if covered_count == assignment_count {
                    return true;
                }
            }
        }
    }
    false
}

fn partial_case_split_covered_assignments(disjunct: &str, bases: &[&str]) -> Option<Vec<usize>> {
    let mut polarities = vec![None; bases.len()];
    for conjunct in non_static_conjuncts(disjunct) {
        let mut matched = false;
        for (index, base) in bases.iter().enumerate() {
            if let Some(polarity) = predicate_polarity_against(conjunct, base) {
                if polarities[index].is_some() {
                    return None;
                }
                polarities[index] = Some(polarity);
                matched = true;
                break;
            }
        }
        if !matched {
            return None;
        }
    }
    if polarities.iter().all(Option::is_none) {
        return None;
    }

    let assignment_count = 1usize << bases.len();
    let mut covered = Vec::new();
    'assignments: for assignment in 0..assignment_count {
        for (index, expected) in polarities.iter().enumerate() {
            let Some(expected) = expected else {
                continue;
            };
            let bit = 1usize << index;
            if (assignment & bit != 0) != *expected {
                continue 'assignments;
            }
        }
        covered.push(assignment);
    }
    Some(covered)
}

fn has_partial_case_split_top_level_and(predicate: &str) -> bool {
    let conjuncts = flattened_keyword_clauses(predicate, "and");
    if conjuncts.len() < 3 {
        return false;
    }
    let mut bases: Vec<&str> = Vec::new();
    for conjunct in &conjuncts {
        for disjunct in non_static_disjuncts(conjunct) {
            if bases.iter().all(|base| {
                !same_predicate(base, disjunct) && !complementary_predicates(base, disjunct)
            }) {
                bases.push(disjunct);
                if bases.len() > MAX_PARTIAL_CASE_SPLIT_ATOMS {
                    return false;
                }
            }
        }
    }
    if bases.len() < 2 {
        return false;
    }

    let Some(assignment_count) = 1usize.checked_shl(bases.len() as u32) else {
        return false;
    };
    let mut rejected = vec![false; assignment_count];
    let mut rejected_count = 0;
    for conjunct in conjuncts {
        let Some(assignments) = partial_case_split_rejected_assignments(conjunct, &bases) else {
            continue;
        };
        for assignment in assignments {
            if !rejected[assignment] {
                rejected[assignment] = true;
                rejected_count += 1;
                if rejected_count == assignment_count {
                    return true;
                }
            }
        }
    }
    false
}

fn partial_case_split_rejected_assignments(conjunct: &str, bases: &[&str]) -> Option<Vec<usize>> {
    let mut polarities = vec![None; bases.len()];
    for disjunct in non_static_disjuncts(conjunct) {
        let mut matched = false;
        for (index, base) in bases.iter().enumerate() {
            if let Some(polarity) = predicate_polarity_against(disjunct, base) {
                if polarities[index].is_some() {
                    return None;
                }
                polarities[index] = Some(polarity);
                matched = true;
                break;
            }
        }
        if !matched {
            return None;
        }
    }
    if polarities.iter().all(Option::is_none) {
        return None;
    }

    let assignment_count = 1usize << bases.len();
    let mut rejected = Vec::new();
    'assignments: for assignment in 0..assignment_count {
        for (index, rejected_polarity) in polarities.iter().enumerate() {
            let Some(rejected_polarity) = rejected_polarity else {
                continue;
            };
            let bit = 1usize << index;
            if (assignment & bit != 0) == *rejected_polarity {
                continue 'assignments;
            }
        }
        rejected.push(assignment);
    }
    Some(rejected)
}

fn has_exhaustive_case_split_top_level_or_between(
    predicate: &str,
    min_arity: usize,
    max_arity: usize,
) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if min_arity == 0 || min_arity > max_arity {
        return false;
    }
    let Some(min_clause_count) = 1usize.checked_shl(min_arity as u32) else {
        return false;
    };
    if disjuncts.len() < min_clause_count {
        return false;
    }
    disjuncts.iter().any(|candidate| {
        let bases = non_static_conjuncts(candidate);
        let arity = bases.len();
        let Some(expected_clause_count) = 1usize.checked_shl(arity as u32) else {
            return false;
        };
        (min_arity..=max_arity).contains(&arity)
            && disjuncts.len() >= expected_clause_count
            && bases.iter().enumerate().all(|(index, base)| {
                bases.iter().skip(index + 1).all(|other| {
                    !same_predicate(base, other) && !complementary_predicates(base, other)
                })
            })
            && exhaustive_case_split_is_complete(&disjuncts, &bases)
    })
}

fn exhaustive_case_split_is_complete(disjuncts: &[&str], bases: &[&str]) -> bool {
    let Some(expected_clause_count) = 1usize.checked_shl(bases.len() as u32) else {
        return false;
    };
    let mut covered = vec![false; expected_clause_count];
    let mut covered_count = 0;

    'disjuncts: for disjunct in disjuncts {
        let conjuncts = non_static_conjuncts(disjunct);
        if conjuncts.len() != bases.len() {
            continue;
        }
        let mut polarities = vec![None; bases.len()];
        for conjunct in conjuncts {
            let mut matched = false;
            for (index, base) in bases.iter().enumerate() {
                if polarities[index].is_none()
                    && let Some(polarity) = predicate_polarity_against(conjunct, base)
                {
                    polarities[index] = Some(polarity);
                    matched = true;
                    break;
                }
            }
            if !matched {
                continue 'disjuncts;
            }
        }
        let mut mask = 0usize;
        for polarity in polarities {
            match polarity {
                Some(polarity) => {
                    mask = (mask << 1) | usize::from(polarity);
                    continue;
                }
                None => continue 'disjuncts,
            }
        }
        if !covered[mask] {
            covered[mask] = true;
            covered_count += 1;
            if covered_count == expected_clause_count {
                return true;
            }
        }
    }

    false
}

fn predicate_polarity_against(predicate: &str, base: &str) -> Option<bool> {
    if same_predicate(predicate, base) {
        Some(true)
    } else if complementary_predicates(predicate, base) {
        Some(false)
    } else {
        None
    }
}

fn non_static_conjuncts(predicate: &str) -> Vec<&str> {
    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter(|conjunct| static_boolean_value(conjunct) != StaticBooleanValue::True)
        .collect()
}

fn non_static_disjuncts(predicate: &str) -> Vec<&str> {
    flattened_keyword_clauses(predicate, "or")
        .into_iter()
        .filter(|disjunct| static_boolean_value(disjunct) != StaticBooleanValue::False)
        .collect()
}

fn has_case_split_top_level_or(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 2 {
        return false;
    }
    disjuncts.iter().enumerate().any(|(index, left)| {
        disjuncts.iter().enumerate().any(|(other_index, right)| {
            index != other_index && disjunct_case_splits_to_true(left, right)
        })
    })
}

fn disjunct_case_splits_to_true(left: &str, right: &str) -> bool {
    if let (Some(left), Some(right)) = (
        static_true_conjunction_variant(left),
        static_true_conjunction_variant(right),
    ) {
        return complementary_predicates(left, right);
    }

    let right_clauses = flattened_keyword_clauses(right, "and");
    right_clauses.len() > 1
        && right_clauses
            .iter()
            .any(|clause| complementary_predicates(left, clause))
        && right_clauses.iter().all(|clause| {
            complementary_predicates(left, clause)
                || static_boolean_value(clause) == StaticBooleanValue::True
        })
}

fn static_true_conjunction_variant(predicate: &str) -> Option<&str> {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() <= 1 {
        return Some(strip_balanced_outer_parens(predicate.trim()));
    }

    let mut variant = None;
    for clause in clauses {
        if static_boolean_value(clause) == StaticBooleanValue::True {
            continue;
        }
        if variant.replace(clause).is_some() {
            return None;
        }
    }
    variant
}

fn has_total_order_top_level_or(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "or");
    if clauses.len() < 3 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(first) = order_trichotomy_shape(clause) else {
            return false;
        };
        clauses
            .iter()
            .skip(index + 1)
            .filter_map(|other| order_trichotomy_shape(other))
            .filter(|other| other.left == first.left && other.right == first.right)
            .fold(first.relation.bit(), |mask, other| {
                mask | other.relation.bit()
            })
            == OrderRelation::ALL_BITS
    })
}

fn has_inclusive_total_order_top_level_or(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "or");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(first) = comparison_shape(clause) else {
            return false;
        };
        first.operator == ">="
            && clauses.iter().skip(index + 1).any(|other| {
                comparison_shape(other).is_some_and(|other| {
                    other.operator == ">=" && other.left == first.right && other.right == first.left
                })
            })
    })
}

fn has_disequality_strict_order_split_top_level_or(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "or");
    if clauses.len() < 3 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(inner) = negated_predicate_inner(clause) else {
            return false;
        };
        let Some((left, right)) = disequality_shape(inner) else {
            return false;
        };
        has_strict_order_bound(&clauses, index, &left, &right)
            && has_strict_order_bound(&clauses, index, &right, &left)
    })
}

fn has_disequality_inclusive_order_split_top_level_or(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "or");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(inner) = negated_predicate_inner(clause) else {
            return false;
        };
        let Some((left, right)) = disequality_shape(inner) else {
            return false;
        };
        has_inclusive_order_bound(&clauses, index, &left, &right)
            || has_inclusive_order_bound(&clauses, index, &right, &left)
    })
}

fn has_strict_order_bound(
    clauses: &[&str],
    excluded_index: usize,
    left: &str,
    right: &str,
) -> bool {
    clauses.iter().enumerate().any(|(index, clause)| {
        index != excluded_index
            && order_bound_shape(clause)
                .is_some_and(|bound| bound.strict && bound.left == left && bound.right == right)
    })
}

fn has_inclusive_order_bound(
    clauses: &[&str],
    excluded_index: usize,
    left: &str,
    right: &str,
) -> bool {
    clauses.iter().enumerate().any(|(index, clause)| {
        index != excluded_index
            && order_bound_shape(clause)
                .is_some_and(|bound| !bound.strict && bound.left == left && bound.right == right)
    })
}

fn has_exclusive_order_top_level_and(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(first) = order_trichotomy_shape(clause) else {
            return false;
        };
        clauses
            .iter()
            .skip(index + 1)
            .filter_map(|other| order_trichotomy_shape(other))
            .any(|other| {
                other.left == first.left
                    && other.right == first.right
                    && other.relation != first.relation
            })
    })
}

fn has_exclusive_inclusive_order_top_level_and(predicate: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    if clauses.len() < 2 {
        return false;
    }
    clauses.iter().enumerate().any(|(index, clause)| {
        let Some(first) = order_bound_shape(clause) else {
            return false;
        };
        clauses
            .iter()
            .skip(index + 1)
            .filter_map(|other| order_bound_shape(other))
            .any(|other| {
                first.left == other.right
                    && first.right == other.left
                    && (first.strict || other.strict)
            })
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NumericLiteralBoundKind {
    Lower,
    Upper,
}

struct NumericLiteralBound {
    subject: String,
    value: StaticRational,
    inclusive: bool,
    kind: NumericLiteralBoundKind,
}

fn has_exclusive_numeric_literal_bounds_top_level_and(predicate: &str) -> bool {
    let bounds = flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .collect::<Vec<_>>();
    if bounds.len() < 2 {
        return false;
    }

    bounds.iter().enumerate().any(|(index, left)| {
        bounds.iter().skip(index + 1).any(|right| {
            left.subject == right.subject
                && matches!(
                    (left.kind, right.kind),
                    (
                        NumericLiteralBoundKind::Lower,
                        NumericLiteralBoundKind::Upper
                    ) | (
                        NumericLiteralBoundKind::Upper,
                        NumericLiteralBoundKind::Lower
                    )
                )
                && literal_bounds_do_not_overlap(left, right)
        })
    })
}

fn has_covering_numeric_literal_bounds_top_level_or(predicate: &str) -> bool {
    let bounds = flattened_keyword_clauses(predicate, "or")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .collect::<Vec<_>>();
    if bounds.len() < 2 {
        return false;
    }

    bounds.iter().enumerate().any(|(index, left)| {
        bounds.iter().skip(index + 1).any(|right| {
            left.subject == right.subject
                && matches!(
                    (left.kind, right.kind),
                    (
                        NumericLiteralBoundKind::Lower,
                        NumericLiteralBoundKind::Upper
                    ) | (
                        NumericLiteralBoundKind::Upper,
                        NumericLiteralBoundKind::Lower
                    )
                )
                && literal_bounds_cover_all_values(left, right)
        })
    })
}

fn numeric_literal_bound_shape(predicate: &str) -> Option<NumericLiteralBound> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["<=", ">=", "<", ">"] {
        let Some((left, right)) = split_top_level_operator(predicate, operator) else {
            continue;
        };
        let left_value = static_rational_expression(left);
        let right_value = static_rational_expression(right);
        match (left_value, right_value) {
            (Some(value), None) => {
                let kind = match operator {
                    "<" | "<=" => NumericLiteralBoundKind::Lower,
                    ">" | ">=" => NumericLiteralBoundKind::Upper,
                    _ => return None,
                };
                return Some(NumericLiteralBound {
                    subject: compact_predicate_text(right),
                    value,
                    inclusive: matches!(operator, "<=" | ">="),
                    kind,
                });
            }
            (None, Some(value)) => {
                let kind = match operator {
                    "<" | "<=" => NumericLiteralBoundKind::Upper,
                    ">" | ">=" => NumericLiteralBoundKind::Lower,
                    _ => return None,
                };
                return Some(NumericLiteralBound {
                    subject: compact_predicate_text(left),
                    value,
                    inclusive: matches!(operator, "<=" | ">="),
                    kind,
                });
            }
            _ => {}
        }
    }
    None
}

fn literal_bounds_do_not_overlap(left: &NumericLiteralBound, right: &NumericLiteralBound) -> bool {
    let (lower, upper) = match (left.kind, right.kind) {
        (NumericLiteralBoundKind::Lower, NumericLiteralBoundKind::Upper) => (left, right),
        (NumericLiteralBoundKind::Upper, NumericLiteralBoundKind::Lower) => (right, left),
        _ => return false,
    };
    lower.value > upper.value
        || (lower.value == upper.value && (!lower.inclusive || !upper.inclusive))
}

fn literal_bounds_cover_all_values(
    left: &NumericLiteralBound,
    right: &NumericLiteralBound,
) -> bool {
    let (lower, upper) = match (left.kind, right.kind) {
        (NumericLiteralBoundKind::Lower, NumericLiteralBoundKind::Upper) => (left, right),
        (NumericLiteralBoundKind::Upper, NumericLiteralBoundKind::Lower) => (right, left),
        _ => return false,
    };
    lower.value < upper.value
        || (lower.value == upper.value && (lower.inclusive || upper.inclusive))
}

struct LiteralEqualityShape {
    subject: String,
    value: StaticLiteral,
}

fn has_exclusive_literal_equalities_top_level_and(predicate: &str) -> bool {
    let equalities: Vec<_> = flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(literal_equality_shape)
        .collect();

    equalities.iter().enumerate().any(|(index, left)| {
        equalities[index + 1..]
            .iter()
            .any(|right| left.subject == right.subject && left.value != right.value)
    })
}

fn literal_equality_shape(predicate: &str) -> Option<LiteralEqualityShape> {
    let (left, right) = split_top_level_operator(predicate, "==")?;
    literal_equality_shape_from_parts(left, right)
        .or_else(|| literal_equality_shape_from_parts(right, left))
}

fn literal_equality_shape_from_parts(subject: &str, value: &str) -> Option<LiteralEqualityShape> {
    if StaticLiteral::parse(subject.trim()).is_some() {
        return None;
    }
    Some(LiteralEqualityShape {
        subject: compact_predicate_text(subject),
        value: StaticLiteral::parse(value.trim())?,
    })
}

struct OrderBoundShape {
    left: String,
    right: String,
    strict: bool,
}

fn order_bound_shape(predicate: &str) -> Option<OrderBoundShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let mut left = compact_predicate_text(left);
            let mut right = compact_predicate_text(right);
            if left == right {
                return None;
            }
            if matches!(operator, ">" | ">=") {
                std::mem::swap(&mut left, &mut right);
            }
            return Some(OrderBoundShape {
                left,
                right,
                strict: matches!(operator, "<" | ">"),
            });
        }
    }
    None
}

fn has_order_bound_transitive_implication_top_level_or(predicate: &str) -> bool {
    let disjuncts = flattened_keyword_clauses(predicate, "or");
    if disjuncts.len() < 2 {
        return false;
    }

    disjuncts.iter().any(|antecedent| {
        let Some(inner) = negated_predicate_inner(antecedent) else {
            return false;
        };
        order_bound_antecedent_implies_any_consequent(inner, antecedent, &disjuncts)
    })
}

fn order_bound_antecedent_implies_any_consequent(
    antecedent_inner: &str,
    antecedent: &str,
    disjuncts: &[&str],
) -> bool {
    let branches = flattened_keyword_clauses(antecedent_inner, "or");
    if branches.len() > 1 {
        return disjuncts.iter().any(|consequent| {
            !same_predicate(antecedent, consequent)
                && branches.iter().all(|branch| {
                    order_bound_branch_implies_consequent(branch, consequent, antecedent, disjuncts)
                })
        });
    }

    disjuncts.iter().any(|consequent| {
        !same_predicate(antecedent, consequent)
            && order_bound_branch_implies_consequent(
                antecedent_inner,
                consequent,
                antecedent,
                disjuncts,
            )
    })
}

fn order_bound_branch_implies_consequent(
    branch: &str,
    consequent: &str,
    antecedent: &str,
    disjuncts: &[&str],
) -> bool {
    let edges: Vec<_> = flattened_keyword_clauses(branch, "and")
        .into_iter()
        .flat_map(order_bound_transitive_edges)
        .collect();
    if edges.is_empty() {
        return false;
    }
    order_bound_shape(consequent).is_some_and(|wanted| {
        order_bound_edges_imply(&edges, &wanted.left, &wanted.right, wanted.strict)
            || numeric_literal_bounds_imply(branch, &wanted)
    }) || equality_shape(consequent).is_some_and(|(left, right)| {
        order_bound_edges_imply_non_strict(&edges, &left, &right)
            && order_bound_edges_imply_non_strict(&edges, &right, &left)
    }) || disequality_shape(consequent).is_some_and(|(left, right)| {
        order_bound_edges_imply(&edges, &left, &right, true)
            || order_bound_edges_imply(&edges, &right, &left, true)
            || equality_disequality_edges_imply_disequality(branch, &left, &right)
            || numeric_literal_bounds_imply_disequality(branch, &left, &right)
    }) || order_bound_edges_imply_strict_or_equality_disjunction(disjuncts, antecedent, &edges)
}

fn order_bound_edges_imply_strict_or_equality_disjunction(
    disjuncts: &[&str],
    antecedent: &str,
    edges: &[OrderBoundShape],
) -> bool {
    disjuncts
        .iter()
        .filter(|disjunct| !same_predicate(antecedent, disjunct))
        .filter_map(|disjunct| order_bound_shape(disjunct).filter(|bound| bound.strict))
        .any(|strict_bound| {
            order_bound_edges_imply(edges, &strict_bound.left, &strict_bound.right, false)
                && disjuncts
                    .iter()
                    .filter(|disjunct| !same_predicate(antecedent, disjunct))
                    .filter_map(|disjunct| equality_shape(disjunct))
                    .any(|(left, right)| {
                        (left == strict_bound.left && right == strict_bound.right)
                            || (left == strict_bound.right && right == strict_bound.left)
                    })
        })
}

fn numeric_literal_bounds_imply(predicate: &str, wanted: &OrderBoundShape) -> bool {
    let Some(wanted) = numeric_literal_bound_shape_from_order_bound(wanted) else {
        return false;
    };
    let equality_edges = flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(equality_shape)
        .flat_map(|(left, right)| [(left.clone(), right.clone()), (right, left)])
        .collect::<Vec<_>>();
    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .any(|required| {
            numeric_literal_bound_implies(&required, &wanted)
                || (equality_edges_imply(&equality_edges, &required.subject, &wanted.subject)
                    && numeric_literal_bound_strength_implies(&required, &wanted))
        })
}

fn numeric_literal_bound_shape_from_order_bound(
    bound: &OrderBoundShape,
) -> Option<NumericLiteralBound> {
    numeric_literal_bound_shape(&format!(
        "{} {} {}",
        bound.left,
        if bound.strict { "<" } else { "<=" },
        bound.right
    ))
}

fn numeric_literal_bound_implies(
    required: &NumericLiteralBound,
    wanted: &NumericLiteralBound,
) -> bool {
    required.subject == wanted.subject && numeric_literal_bound_strength_implies(required, wanted)
}

fn numeric_literal_bounds_imply_disequality(predicate: &str, left: &str, right: &str) -> bool {
    let equality_edges = flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(equality_shape)
        .flat_map(|(left, right)| [(left.clone(), right.clone()), (right, left)])
        .collect::<Vec<_>>();
    let Some((wanted_subject, wanted_value)) =
        numeric_literal_disequality_subject_value(left, right)
    else {
        return false;
    };

    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter_map(numeric_literal_bound_shape)
        .any(|required| {
            equality_edges_imply(&equality_edges, &required.subject, &wanted_subject)
                && numeric_literal_bound_excludes_value(&required, wanted_value)
        })
}

fn numeric_literal_disequality_subject_value(
    left: &str,
    right: &str,
) -> Option<(String, StaticRational)> {
    static_rational_expression(left)
        .map(|value| (compact_predicate_text(right), value))
        .or_else(|| {
            static_rational_expression(right).map(|value| (compact_predicate_text(left), value))
        })
}

fn numeric_literal_bound_excludes_value(
    required: &NumericLiteralBound,
    wanted_value: StaticRational,
) -> bool {
    match required.kind {
        NumericLiteralBoundKind::Lower => {
            wanted_value < required.value || (wanted_value == required.value && !required.inclusive)
        }
        NumericLiteralBoundKind::Upper => {
            wanted_value > required.value || (wanted_value == required.value && !required.inclusive)
        }
    }
}

fn numeric_literal_bound_strength_implies(
    required: &NumericLiteralBound,
    wanted: &NumericLiteralBound,
) -> bool {
    required.kind == wanted.kind
        && match required.kind {
            NumericLiteralBoundKind::Lower => {
                required.value > wanted.value
                    || (required.value == wanted.value && (wanted.inclusive || !required.inclusive))
            }
            NumericLiteralBoundKind::Upper => {
                required.value < wanted.value
                    || (required.value == wanted.value && (wanted.inclusive || !required.inclusive))
            }
        }
}

fn equality_shape(predicate: &str) -> Option<(String, String)> {
    let (left, right) = split_top_level_operator(predicate, "==")?;
    let left = compact_predicate_text(left);
    let right = compact_predicate_text(right);
    (left != right).then_some((left, right))
}

fn disequality_shape(predicate: &str) -> Option<(String, String)> {
    let (left, right) = split_top_level_operator(predicate, "!=")?;
    let left = compact_predicate_text(left);
    let right = compact_predicate_text(right);
    (left != right).then_some((left, right))
}

fn equality_disequality_edges_imply_disequality(predicate: &str, left: &str, right: &str) -> bool {
    let clauses = flattened_keyword_clauses(predicate, "and");
    let equality_edges: Vec<_> = clauses
        .iter()
        .filter_map(|clause| equality_shape(clause))
        .flat_map(|(left, right)| [(left.clone(), right.clone()), (right, left)])
        .collect();
    let disequalities: Vec<_> = clauses
        .iter()
        .filter_map(|clause| disequality_shape(clause))
        .collect();

    disequalities.iter().any(|(disequal_left, disequal_right)| {
        (equality_edges_imply(&equality_edges, left, disequal_left)
            && equality_edges_imply(&equality_edges, right, disequal_right))
            || (equality_edges_imply(&equality_edges, left, disequal_right)
                && equality_edges_imply(&equality_edges, right, disequal_left))
    })
}

fn equality_edges_imply(edges: &[(String, String)], left: &str, right: &str) -> bool {
    if left == right {
        return true;
    }

    let mut stack = vec![left.to_string()];
    let mut visited = Vec::new();

    while let Some(current) = stack.pop() {
        if visited.iter().any(|node| node == &current) {
            continue;
        }
        visited.push(current.clone());

        for (_, edge_right) in edges.iter().filter(|(edge_left, _)| edge_left == &current) {
            if edge_right == right {
                return true;
            }
            stack.push(edge_right.clone());
        }
    }

    false
}

fn order_bound_transitive_edges(predicate: &str) -> Vec<OrderBoundShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some((left, right)) = split_top_level_operator(predicate, "==") {
        let left = compact_predicate_text(left);
        let right = compact_predicate_text(right);
        if left == right {
            return Vec::new();
        }
        return vec![
            OrderBoundShape {
                left: left.clone(),
                right: right.clone(),
                strict: false,
            },
            OrderBoundShape {
                left: right,
                right: left,
                strict: false,
            },
        ];
    }
    order_bound_shape(predicate).into_iter().collect()
}

fn order_bound_edges_imply(
    edges: &[OrderBoundShape],
    left: &str,
    right: &str,
    strict: bool,
) -> bool {
    let mut stack = vec![(left.to_string(), false)];
    let mut visited = Vec::new();

    while let Some((current, path_strict)) = stack.pop() {
        if visited
            .iter()
            .any(|(node, strictness)| node == &current && *strictness == path_strict)
        {
            continue;
        }
        visited.push((current.clone(), path_strict));

        for edge in edges.iter().filter(|edge| edge.left == current) {
            let next_strict = path_strict || edge.strict;
            if edge.right == right && (!strict || next_strict) {
                return true;
            }
            stack.push((edge.right.clone(), next_strict));
        }
    }

    false
}

fn order_bound_edges_imply_non_strict(edges: &[OrderBoundShape], left: &str, right: &str) -> bool {
    let mut stack = vec![left.to_string()];
    let mut visited = Vec::new();

    while let Some(current) = stack.pop() {
        if visited.iter().any(|node| node == &current) {
            continue;
        }
        visited.push(current.clone());

        for edge in edges
            .iter()
            .filter(|edge| !edge.strict && edge.left == current)
        {
            if edge.right == right {
                return true;
            }
            stack.push(edge.right.clone());
        }
    }

    false
}

fn flattened_keyword_clauses<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut flattened = Vec::new();
    let mut stack = vec![strip_balanced_outer_parens(predicate)];

    while let Some(predicate) = stack.pop() {
        let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), keyword);
        if clauses.len() <= 1 {
            flattened.extend(clauses);
            continue;
        }
        stack.extend(clauses.into_iter().rev());
    }

    flattened
}

fn negated_predicate_inner(predicate: &str) -> Option<&str> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(rest) = predicate.strip_prefix("not ") {
        let rest = rest.trim();
        if rest.starts_with('(') {
            return (strip_balanced_outer_parens(rest) != rest).then_some(rest);
        }
        if is_single_negation_operand(rest) {
            return Some(rest);
        }
        return None;
    }
    if let Some(rest) = predicate.strip_prefix("not(") {
        return rest.strip_suffix(')');
    }
    None
}

fn whole_negated_predicate_inner(predicate: &str) -> Option<&str> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    let rest = predicate.strip_prefix("not")?;
    if !(rest.starts_with(char::is_whitespace) || rest.starts_with('(')) {
        return None;
    }
    let rest = rest.trim();
    if rest.starts_with('(') {
        return (strip_balanced_outer_parens(rest) != rest).then_some(rest);
    }
    is_single_negation_operand(rest).then_some(rest)
}

fn is_single_negation_operand(predicate: &str) -> bool {
    split_top_level_keyword(predicate, "and").len() == 1
        && split_top_level_keyword(predicate, "or").len() == 1
}

fn same_predicate(left: &str, right: &str) -> bool {
    predicate_shape(left) == predicate_shape(right)
}

fn predicate_shape(predicate: &str) -> String {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    let mut shape = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut pending_space = false;
    for ch in predicate.chars() {
        if in_string {
            shape.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            if pending_space && !shape.is_empty() {
                shape.push(' ');
            }
            pending_space = false;
            in_string = true;
            shape.push(ch);
            continue;
        }
        if ch.is_whitespace() {
            pending_space = !shape.is_empty();
            continue;
        }
        if pending_space {
            shape.push(' ');
            pending_space = false;
        }
        shape.push(ch);
    }
    shape
}

fn boolean_literal_alias_shape(predicate: &str) -> Option<(String, bool)> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["==", "!="] {
        let Some((left, right)) = split_top_level_operator(predicate, operator) else {
            continue;
        };
        if let Some(value) = boolean_literal(right) {
            return Some((predicate_shape(left), value == (operator == "==")));
        }
        if let Some(value) = boolean_literal(left) {
            return Some((predicate_shape(right), value == (operator == "==")));
        }
    }
    None
}

fn boolean_literal(predicate: &str) -> Option<bool> {
    match strip_balanced_outer_parens(predicate.trim()) {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[derive(PartialEq, Eq)]
struct ComparisonShape {
    left: String,
    operator: &'static str,
    right: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OrderRelation {
    Less,
    Equal,
    Greater,
}

impl OrderRelation {
    const ALL_BITS: u8 = Self::Less.bit() | Self::Equal.bit() | Self::Greater.bit();

    const fn bit(self) -> u8 {
        match self {
            Self::Less => 0b001,
            Self::Equal => 0b010,
            Self::Greater => 0b100,
        }
    }
}

struct OrderTrichotomyShape {
    left: String,
    right: String,
    relation: OrderRelation,
}

fn complementary_comparisons(left: &str, right: &str) -> bool {
    let Some(left) = comparison_shape(left) else {
        return false;
    };
    let Some(right) = comparison_shape(right) else {
        return false;
    };
    left.left == right.left
        && left.right == right.right
        && matches!(
            (left.operator, right.operator),
            ("==", "!=") | ("!=", "==") | ("<", ">=") | (">=", "<")
        )
        || complementary_literal_comparison_operands(&left, &right)
}

fn complementary_literal_comparison_operands(
    left: &ComparisonShape,
    right: &ComparisonShape,
) -> bool {
    if !matches!((left.operator, right.operator), ("==", "!=") | ("!=", "==")) {
        return false;
    }
    (left.left == right.left
        && static_literal_comparison(&left.right, "==", &right.right) == Some(true))
        || (left.right == right.right
            && static_literal_comparison(&left.left, "==", &right.left) == Some(true))
        || (left.left == right.right
            && static_literal_comparison(&left.right, "==", &right.left) == Some(true))
        || (left.right == right.left
            && static_literal_comparison(&left.left, "==", &right.right) == Some(true))
}

fn order_trichotomy_shape(predicate: &str) -> Option<OrderTrichotomyShape> {
    let comparison = comparison_shape(predicate)?;
    let mut left = comparison.left;
    let mut right = comparison.right;
    if left == right {
        return None;
    }
    let mut relation = match comparison.operator {
        "==" => OrderRelation::Equal,
        "<" => OrderRelation::Less,
        _ => return None,
    };
    if right < left {
        std::mem::swap(&mut left, &mut right);
        relation = match relation {
            OrderRelation::Less => OrderRelation::Greater,
            OrderRelation::Equal => OrderRelation::Equal,
            OrderRelation::Greater => OrderRelation::Less,
        };
    }
    Some(OrderTrichotomyShape {
        left,
        right,
        relation,
    })
}

fn comparison_shape(predicate: &str) -> Option<ComparisonShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = compact_predicate_text(left);
            let right = compact_predicate_text(right);
            return match operator {
                "==" | "!=" if right < left => Some(ComparisonShape {
                    left: right,
                    operator,
                    right: left,
                }),
                "<=" => Some(ComparisonShape {
                    left: right,
                    operator: ">=",
                    right: left,
                }),
                ">" => Some(ComparisonShape {
                    left: right,
                    operator: "<",
                    right: left,
                }),
                _ => Some(ComparisonShape {
                    left,
                    operator,
                    right,
                }),
            };
        }
    }
    None
}

fn static_same_shape_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    if compact_predicate_text(left) != compact_predicate_text(right) {
        return None;
    }
    match operator {
        "==" | "<=" | ">=" => Some(true),
        "!=" | "<" | ">" => Some(false),
        _ => None,
    }
}

fn static_complementary_predicate_comparison(
    left: &str,
    operator: &str,
    right: &str,
) -> Option<bool> {
    if !matches!(operator, "==" | "!=") || !complementary_predicates(left, right) {
        return None;
    }
    Some(operator == "!=")
}

fn compact_predicate_text(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut chars = predicate.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' {
            output.push(ch);
            let mut escaped = false;
            for string_ch in chars.by_ref() {
                output.push(string_ch);
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if !ch.is_whitespace() {
            output.push(ch);
        }
    }
    output
}

impl StaticBooleanValue {
    fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            (Self::False, value) | (value, Self::False) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            (Self::True, value) | (value, Self::True) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }
}

impl From<bool> for StaticBooleanValue {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exhaustive_case_split_predicate(subject: &str, fields: &[&str]) -> String {
        let assignment_count = 1usize << fields.len();
        (0..assignment_count)
            .map(|assignment| {
                let conjuncts = fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| {
                        let bit = 1usize << (fields.len() - index - 1);
                        if assignment & bit != 0 {
                            format!("{subject}.{field}")
                        } else {
                            format!("not {subject}.{field}")
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" and ");
                format!("({conjuncts})")
            })
            .collect::<Vec<_>>()
            .join(" or ")
    }

    #[test]
    fn negation_prefix_does_not_capture_following_conjunction() {
        let predicate = "(not value.ready and value.paid) or (value.ready and value.paid)";

        assert!(!has_complementary_top_level_clauses(predicate, "or"));
        assert!(!predicate_is_statically_true(predicate));
    }

    #[test]
    fn small_boolean_truth_table_proves_nested_tautology() {
        let predicate = "not (value.ready and not extra) or not (not value.ready and not extra)";
        let mut atoms = Vec::new();
        collect_boolean_formula_atoms(predicate, &mut atoms);

        assert_eq!(atoms, vec!["value.ready".to_string(), "extra".to_string()]);
        assert_eq!(eval_boolean_formula(predicate, &atoms, 0), Some(true));
        assert_eq!(eval_boolean_formula(predicate, &atoms, 1), Some(true));
        assert_eq!(eval_boolean_formula(predicate, &atoms, 2), Some(true));
        assert_eq!(eval_boolean_formula(predicate, &atoms, 3), Some(true));
        assert_eq!(
            static_boolean_truth_table_value(predicate),
            Some(StaticBooleanValue::True)
        );
        assert!(predicate_is_statically_true(predicate));
    }

    #[test]
    fn high_arity_exhaustive_case_splits_are_statically_true() {
        for fields in [
            &["a", "b", "c", "d", "e", "f", "g", "h", "i"][..],
            &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"][..],
            &["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"][..],
        ] {
            let predicate = exhaustive_case_split_predicate("value", fields);

            assert!(predicate_is_statically_true(&predicate));
        }
    }

    #[test]
    fn boolean_formula_comparison_proves_commutative_conjunction() {
        assert_eq!(
            static_boolean_formula_comparison(
                "(value.ready and value.paid)",
                "==",
                "(value.paid and value.ready)",
            ),
            Some(true)
        );
        assert_eq!(
            split_top_level_operator(
                "(value.ready and value.paid) ==(value.paid and value.ready)",
                "==",
            ),
            Some((
                "(value.ready and value.paid)",
                "(value.paid and value.ready)"
            ))
        );
        assert!(predicate_is_statically_true(
            "(value.ready and value.paid) ==(value.paid and value.ready)"
        ));
    }

    #[test]
    fn contract_static_truth_classifies_disjoint_literal_bounds() {
        for predicate in [
            "not (value > 10 and value < 5)",
            "not (10 < value and value < 10)",
            "not (1 + 1 <= value and value < 2)",
            "not (2 > value and value >= 2)",
        ] {
            assert!(
                contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn contract_static_truth_classifies_covering_literal_bounds() {
        for predicate in [
            "value <= 10 or value >= 5",
            "value < 10 or 5 <= value",
            "1 + 1 >= value or value >= 2",
            "value > 2 or value <= 2",
        ] {
            assert!(
                contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn general_static_truth_leaves_literal_bound_shapes_unknown() {
        assert!(!predicate_is_statically_true(
            "not (value > 10 and value < 5)"
        ));
        assert!(!predicate_is_statically_false("value > 10 and value < 5"));
        assert!(!predicate_is_statically_true("value <= 10 or value >= 5"));
    }

    #[test]
    fn contract_static_truth_classifies_exclusive_literal_equalities() {
        for predicate in [
            "not (value == \"ready\" and value == \"done\")",
            "not (1 == value and value == 2)",
            "not ((value.ready) == true and false == value.ready)",
        ] {
            assert!(
                contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn repair_static_truth_classifies_exclusive_literal_equalities() {
        for predicate in [
            "not (value == \"ready\" and value == \"done\")",
            "not (1 == value and value == 2)",
            "not ((value.ready) == true and false == value.ready)",
        ] {
            assert!(
                predicate_is_statically_true_with_literal_bounds(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn general_static_truth_leaves_literal_equality_shapes_unknown() {
        assert!(!predicate_is_statically_true(
            "not (value == \"ready\" and value == \"done\")"
        ));
        assert!(!predicate_is_statically_false(
            "value == \"ready\" and value == \"done\""
        ));
    }

    #[test]
    fn contract_static_truth_keeps_compatible_literal_equalities_runtime_checked() {
        for predicate in [
            "value == \"ready\" and value == \"ready\"",
            "value == \"ready\" and other == \"done\"",
        ] {
            assert!(
                !has_exclusive_literal_equalities_top_level_and(predicate),
                "{predicate}"
            );
            assert!(
                !contract_predicate_is_statically_true(&format!("not ({predicate})")),
                "{predicate}"
            );
        }
        assert!(!has_exclusive_literal_equalities_top_level_and(
            "\"ready\" == \"done\" and value == \"ready\""
        ));
    }

    #[test]
    fn contract_static_truth_keeps_overlapping_literal_bounds_runtime_checked() {
        assert!(!has_exclusive_numeric_literal_bounds_top_level_and(
            "value >= 5 and value <= 5"
        ));
        assert!(!has_exclusive_inclusive_order_top_level_and(
            "value >= 5 and value <= 5"
        ));
        assert_eq!(
            static_boolean_truth_table_value("value >= 5 and value <= 5"),
            Some(StaticBooleanValue::Unknown)
        );
        assert_eq!(static_comparison_value("value >= 5"), None);
        assert_eq!(static_comparison_value("value <= 5"), None);
        assert!(!complementary_predicates("value >= 5", "value <= 5"));
        assert_eq!(
            static_boolean_value_inner("value >= 5 and value <= 5", true, true),
            StaticBooleanValue::Unknown
        );
        for predicate in [
            "not (value > 5 and value < 10)",
            "not (value >= 5 and value <= 5)",
            "not (value > 5 and other < 5)",
            "value < 5 or value > 5",
            "value <= 5 or other >= 5",
        ] {
            assert!(
                !contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn contract_static_truth_classifies_literal_bounds_excluding_disequality_values() {
        for predicate in [
            "not (value > 10) or (value != 10)",
            "not (value <= 1 / 2) or (value != 0.75)",
            "not (value == alias and alias < 20) or (value != 20)",
        ] {
            assert!(
                contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }

    #[test]
    fn contract_static_truth_keeps_possible_bound_endpoint_disequality_runtime_checked() {
        for predicate in [
            "not (value >= 10) or (value != 10)",
            "not (value <= 20) or (value != 20)",
            "not (value == alias and alias > 5) or (value != 6)",
        ] {
            assert!(
                !contract_predicate_is_statically_true(predicate),
                "{predicate}"
            );
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum StaticLiteral {
    Bool(bool),
    Number(StaticNumber),
    String(String),
}

fn static_literal_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    let left_literal = StaticLiteral::parse(left.trim());
    let right_literal = StaticLiteral::parse(right.trim());
    if let (Some(StaticLiteral::Number(left)), Some(StaticLiteral::Number(right))) =
        (&left_literal, &right_literal)
    {
        return static_number_comparison(*left, operator, *right);
    }
    if let (Some(left), Some(right)) = (
        static_numeric_expression(left),
        static_numeric_expression(right),
    ) {
        return static_number_comparison(left, operator, right);
    }
    if let (Some(left), Some(right)) = (
        static_rational_expression(left),
        static_rational_expression(right),
    ) {
        return static_rational_comparison(left, operator, right);
    }
    if matches!(operator, "==" | "!=") {
        let left = static_boolean_value(left);
        let right = static_boolean_value(right);
        if left != StaticBooleanValue::Unknown && right != StaticBooleanValue::Unknown {
            return Some(match operator {
                "==" => left == right,
                "!=" => left != right,
                _ => unreachable!("operator was already checked"),
            });
        }
    }
    match (left_literal?, right_literal?) {
        (StaticLiteral::Bool(left), StaticLiteral::Bool(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        (StaticLiteral::Number(left), StaticLiteral::Number(right)) => {
            static_number_comparison(left, operator, right)
        }
        (StaticLiteral::String(left), StaticLiteral::String(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        _ => None,
    }
}

fn static_numeric_expression(predicate: &str) -> Option<StaticNumber> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = StaticNumber::parse(predicate) {
        return Some(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_numeric_expression(left)?;
            let right = static_numeric_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_numeric_expression(left)?;
            let right = static_numeric_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return static_numeric_expression(rest)?.negate();
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct StaticRational {
    numerator: i128,
    denominator: i128,
}

impl Ord for StaticRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("static rational comparison overflow")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("static rational comparison overflow"),
            )
    }
}

impl PartialOrd for StaticRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl StaticRational {
    fn from_number(number: StaticNumber) -> Option<Self> {
        Self::from_raw(number.mantissa, 10_i128.checked_pow(number.scale)?)
    }

    fn from_raw(mut numerator: i128, mut denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }
        let divisor = gcd_i128(numerator, denominator)?;
        Some(Self {
            numerator: numerator.checked_div(divisor)?,
            denominator: denominator.checked_div(divisor)?,
        })
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_add(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn div(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

fn static_rational_expression(predicate: &str) -> Option<StaticRational> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = StaticNumber::parse(predicate) {
        return StaticRational::from_number(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_rational_expression(left)?;
            let right = static_rational_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_rational_expression(left)?;
            let right = static_rational_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return static_rational_expression(rest)?.negate();
    }
    None
}

fn static_rational_comparison(
    left: StaticRational,
    operator: &str,
    right: StaticRational,
) -> Option<bool> {
    Some(match operator {
        "==" => left == right,
        "!=" => left != right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => return None,
    })
}

fn static_number_comparison(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<bool> {
    Some(match operator {
        "==" => left == right,
        "!=" => left != right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => return None,
    })
}

impl StaticLiteral {
    fn parse(text: &str) -> Option<Self> {
        let text = strip_balanced_outer_parens(text.trim());
        match text {
            "true" => return Some(Self::Bool(true)),
            "false" => return Some(Self::Bool(false)),
            _ => {}
        }
        if let Some(number) = StaticNumber::parse(text) {
            return Some(Self::Number(number));
        }
        parse_static_string_literal(text).map(Self::String)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct StaticNumber {
    mantissa: i128,
    scale: u32,
}

impl Ord for StaticNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.mantissa.is_negative(), other.mantissa.is_negative()) {
            (true, false) => return std::cmp::Ordering::Less,
            (false, true) => return std::cmp::Ordering::Greater,
            _ => {}
        }
        let ordering = self.abs_cmp(other);
        if self.mantissa.is_negative() {
            ordering.reverse()
        } else {
            ordering
        }
    }
}

impl PartialOrd for StaticNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl StaticNumber {
    fn parse(text: &str) -> Option<Self> {
        let (negative, digits) = text
            .strip_prefix('-')
            .map_or((false, text), |digits| (true, digits.trim_start()));
        if digits.is_empty() {
            return None;
        }
        let (integer, fraction) = digits.split_once('.').map_or((digits, ""), |parts| parts);
        if integer.is_empty()
            || !integer.chars().all(|ch| ch.is_ascii_digit())
            || !fraction.chars().all(|ch| ch.is_ascii_digit())
            || (digits.contains('.') && fraction.is_empty())
        {
            return None;
        }
        let mut scale = fraction.len() as u32;
        let signed_digits = if negative {
            format!("-{integer}{fraction}")
        } else {
            format!("{integer}{fraction}")
        };
        let mut mantissa = signed_digits.parse::<i128>().ok()?;
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Some(Self { mantissa, scale })
    }

    fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
        let (left_integer, left_fraction) = self.abs_parts();
        let (right_integer, right_fraction) = other.abs_parts();
        left_integer
            .len()
            .cmp(&right_integer.len())
            .then_with(|| left_integer.cmp(&right_integer))
            .then_with(|| {
                let scale = left_fraction.len().max(right_fraction.len());
                let mut left_fraction = left_fraction;
                let mut right_fraction = right_fraction;
                left_fraction.extend(std::iter::repeat_n('0', scale - left_fraction.len()));
                right_fraction.extend(std::iter::repeat_n('0', scale - right_fraction.len()));
                left_fraction.cmp(&right_fraction)
            })
    }

    fn abs_parts(&self) -> (String, String) {
        let mut digits = self.mantissa.unsigned_abs().to_string();
        if self.scale == 0 {
            return (digits, String::new());
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            let padding = "0".repeat(scale + 1 - digits.len());
            digits = format!("{padding}{digits}");
        }
        let split = digits.len() - scale;
        let integer = digits[..split].trim_start_matches('0');
        let integer = if integer.is_empty() { "0" } else { integer };
        (integer.to_string(), digits[split..].to_string())
    }

    fn from_raw(mut mantissa: i128, mut scale: u32) -> Self {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self { mantissa, scale }
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            mantissa: self.mantissa.checked_neg()?,
            scale: self.scale,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.scaled_mantissa(scale)?;
        let right = other.scaled_mantissa(scale)?;
        Some(Self::from_raw(left.checked_add(right)?, scale))
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Some(Self::from_raw(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    fn div(self, other: Self) -> Option<Self> {
        if other.mantissa == 0 {
            return None;
        }

        let mut numerator = self
            .mantissa
            .checked_mul(10_i128.checked_pow(other.scale)?)?;
        let mut denominator = other
            .mantissa
            .checked_mul(10_i128.checked_pow(self.scale)?)?;
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }

        let divisor = gcd_i128(numerator, denominator)?;
        numerator /= divisor;
        denominator /= divisor;

        let mut twos = 0u32;
        while denominator % 2 == 0 {
            denominator /= 2;
            twos += 1;
        }
        let mut fives = 0u32;
        while denominator % 5 == 0 {
            denominator /= 5;
            fives += 1;
        }
        if denominator != 1 {
            return None;
        }

        let scale = twos.max(fives);
        let scale_up = 10_i128.checked_pow(scale)?;
        let mantissa = numerator
            .checked_mul(scale_up)?
            .checked_div(divisor_scale(twos, fives)?)?;
        Some(Self::from_raw(mantissa, scale))
    }

    fn scaled_mantissa(self, scale: u32) -> Option<i128> {
        let extra_scale = scale.checked_sub(self.scale)?;
        self.mantissa.checked_mul(10_i128.checked_pow(extra_scale)?)
    }
}

fn divisor_scale(twos: u32, fives: u32) -> Option<i128> {
    let twos = 2_i128.checked_pow(twos)?;
    let fives = 5_i128.checked_pow(fives)?;
    twos.checked_mul(fives)
}

fn gcd_i128(left: i128, right: i128) -> Option<i128> {
    let mut left = left.checked_abs()?;
    let mut right = right.checked_abs()?;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    (left != 0).then_some(left)
}

fn parse_static_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            value.push(chars.next()?);
        } else if ch == '"' {
            return None;
        } else {
            value.push(ch);
        }
    }
    Some(value)
}

pub(crate) fn contract_calls(predicate: &str) -> Vec<ContractCall> {
    let bytes = predicate.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        if bytes[index] != b'('
            || index == 0
            || !predicate[..index].trim_end().ends_with_identifier()
        {
            index += 1;
            continue;
        }
        let Some(callee_start) = callee_start(predicate, index) else {
            index += 1;
            continue;
        };
        let Some(close) = matching_close(predicate, index) else {
            index += 1;
            continue;
        };
        calls.push(ContractCall {
            callee: predicate[callee_start..index].trim().to_string(),
            args: split_call_args(&predicate[index + 1..close]),
            start: callee_start,
            end: close + 1,
        });
        index += 1;
    }
    calls
}

trait EndsWithIdentifier {
    fn ends_with_identifier(&self) -> bool;
}

impl EndsWithIdentifier for str {
    fn ends_with_identifier(&self) -> bool {
        self.chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }
}

fn callee_start(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut index = open;
    while index > 0 {
        let ch = bytes[index - 1] as char;
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            index -= 1;
        } else {
            break;
        }
    }
    (index < open).then_some(index)
}

fn matching_close(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_call_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg.to_string());
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg.to_string());
    }
    args
}

pub(crate) fn referenced_names(predicate: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = predicate.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let ch = bytes[index] as char;
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            if start >= 2 && &predicate[start - 2..start] == "::" {
                continue;
            }
            if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
                continue;
            }
            if start >= 1 && &predicate[start - 1..start] == "." {
                continue;
            }
            let name = predicate[start..index].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        } else {
            index += 1;
        }
    }
    names
}

pub(crate) fn is_contract_keyword(name: &str) -> bool {
    matches!(name, "and" | "or" | "not")
}

pub(crate) fn missing_contract_field(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<(String, String)> {
    for access in field_accesses(predicate) {
        let Some(mut current) = predicate_type_with_calls(&access.base, bindings, call_type) else {
            continue;
        };
        for field in access.fields {
            let Some(next) = current.record_field(&field) else {
                return Some((current.render(), field));
            };
            current = next.clone();
        }
    }
    None
}

pub(crate) fn predicate_is_boolean_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> bool {
    let trimmed = predicate.trim();
    predicate_type_with_calls(trimmed, bindings, call_type).is_some_and(
        |ty| matches!(ty, Type::Named { name, args } if name == "Bool" && args.is_empty()),
    )
}

pub(crate) fn predicate_rendered_type_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> String {
    let trimmed = predicate.trim();
    predicate_type_with_calls(trimmed, bindings, call_type)
        .map_or_else(|| "unknown".to_string(), |ty| ty.render())
}

pub(crate) fn predicate_type_with_calls(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    predicate_literal_type(predicate)
        .or_else(|| predicate_unary_type(predicate, bindings, call_type))
        .or_else(|| predicate_boolean_type(predicate, bindings, call_type))
        .or_else(|| predicate_comparison_type(predicate, bindings, call_type))
        .or_else(|| predicate_arithmetic_type(predicate, bindings, call_type))
        .or_else(|| predicate_field_access_type(predicate, bindings, call_type))
        .or_else(|| predicate_contract_call_type(predicate, call_type))
        .or_else(|| predicate_binding_type(predicate, bindings))
}

fn predicate_literal_type(predicate: &str) -> Option<Type> {
    if matches!(predicate, "true" | "false") {
        return Some(Type::bool());
    }
    if predicate == "()" {
        return Some(Type::unit());
    }
    if is_complete_string_literal(predicate) {
        return Some(Type::string());
    }
    if predicate.chars().all(|ch| ch.is_ascii_digit()) {
        return Some(Type::int());
    }
    is_float_literal(predicate).then(Type::float)
}

fn predicate_unary_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    if let Some(rest) = predicate.strip_prefix('-') {
        let ty = predicate_type_with_calls(rest, bindings, call_type)?;
        return matches!(ty, Type::Named { ref name, ref args } if args.is_empty() && (name == "Int" || name == "Float"))
            .then_some(ty);
    }
    if let Some(rest) = predicate.strip_prefix("not ") {
        return boolean_unary_type(rest, bindings, call_type);
    }
    let inner = predicate.strip_prefix("not(")?.strip_suffix(')')?;
    boolean_unary_type(inner, bindings, call_type)
}

fn boolean_unary_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    (predicate_type_with_calls(predicate, bindings, call_type)? == Type::bool()).then(Type::bool)
}

fn predicate_boolean_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["or", "and"] {
        if let Some((left, right)) = split_top_level_keyword_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return (left == Type::bool() && right == Type::bool()).then(Type::bool);
        }
    }
    None
}

fn predicate_comparison_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return comparable_predicate_operands(&left, &right).then(Type::bool);
        }
    }
    None
}

fn predicate_arithmetic_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    for operator in ["+", "-", "*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = predicate_type_with_calls(left, bindings, call_type)?;
            let right = predicate_type_with_calls(right, bindings, call_type)?;
            return numeric_result_type(&left, &right);
        }
    }
    None
}

fn predicate_field_access_type(
    predicate: &str,
    bindings: &[Binding],
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let access = split_field_access(predicate)?;
    let mut current = predicate_type_with_calls(access.base, bindings, call_type)?;
    for field in access.fields {
        current = current.record_field(field)?.clone();
    }
    Some(current)
}

fn predicate_contract_call_type(
    predicate: &str,
    call_type: &impl Fn(&str) -> Option<Type>,
) -> Option<Type> {
    let calls = contract_calls(predicate);
    let [call] = calls.as_slice() else {
        return None;
    };
    (call.start == 0 && call.end == predicate.len())
        .then(|| call_type(&call.callee))
        .flatten()
}

fn predicate_binding_type(predicate: &str, bindings: &[Binding]) -> Option<Type> {
    if let Some(binding) = bindings.iter().find(|binding| binding.name == predicate) {
        return Some(binding.ty.clone());
    }
    let mut parts = predicate.split('.');
    let base = parts.next()?;
    let binding = bindings.iter().find(|binding| binding.name == base)?;
    let mut current = binding.ty.clone();
    for field in parts {
        current = current.record_field(field)?.clone();
    }
    Some(current)
}

fn comparable_predicate_operands(left: &Type, right: &Type) -> bool {
    left == right || (is_numeric_type(left) && is_numeric_type(right))
}

fn numeric_result_type(left: &Type, right: &Type) -> Option<Type> {
    if !is_numeric_type(left) || !is_numeric_type(right) {
        return None;
    }
    if left == &Type::float() || right == &Type::float() {
        Some(Type::float())
    } else {
        Some(Type::int())
    }
}

fn is_numeric_type(ty: &Type) -> bool {
    ty == &Type::int() || ty == &Type::float()
}

fn is_float_literal(text: &str) -> bool {
    let Some((left, right)) = text.split_once('.') else {
        return false;
    };
    !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|ch| ch.is_ascii_digit())
        && right.chars().all(|ch| ch.is_ascii_digit())
}

fn split_top_level_operator<'a>(predicate: &'a str, operator: &str) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[index..].starts_with(operator) => {
                let left = predicate[..index].trim();
                let right = predicate[index + operator.len()..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_keyword_operator<'a>(
    predicate: &'a str,
    keyword: &str,
) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[index..].starts_with(keyword) => {
                let end = index + keyword.len();
                if !is_keyword_boundary(predicate, index, end) {
                    continue;
                }
                let left = predicate[..index].trim();
                let right = predicate[end..].trim();
                if !left.is_empty() && !right.is_empty() {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_keyword<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = 0;

    while cursor < predicate.len() {
        let ch = predicate[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        let end = cursor + ch.len_utf8();
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            cursor = end;
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[cursor..].starts_with(keyword) => {
                let keyword_end = cursor + keyword.len();
                if is_keyword_boundary(predicate, cursor, keyword_end) {
                    clauses.push(predicate[start..cursor].trim());
                    start = keyword_end;
                    cursor = keyword_end;
                    continue;
                }
            }
            _ => {}
        }
        cursor = end;
    }

    clauses.push(predicate[start..].trim());
    clauses
}

fn is_keyword_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_ident_continue(ch)) && after.is_none_or(|ch| !is_ident_continue(ch))
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn strip_balanced_outer_parens(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let mut depth = 0usize;
        let mut balanced_outer = true;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index != trimmed.len() - 1 {
                        balanced_outer = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !balanced_outer || depth != 0 {
            return trimmed;
        }
        trimmed = trimmed[1..trimmed.len() - 1].trim();
    }
}

struct FieldAccess {
    base: String,
    fields: Vec<String>,
}

struct FieldAccessRef<'a> {
    base: &'a str,
    fields: Vec<&'a str>,
}

fn split_field_access(predicate: &str) -> Option<FieldAccessRef<'_>> {
    let mut scanner = FieldAccessScanner::default();
    let mut first_dot = None;
    let mut fields = Vec::new();
    let mut index = 0usize;
    while index < predicate.len() {
        let ch = predicate[index..].chars().next()?;
        if scanner.consume_quoted(ch) {
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => {
                scanner.start_string();
                index += ch.len_utf8();
            }
            '(' => {
                scanner.open_group();
                index += ch.len_utf8();
            }
            ')' => {
                scanner.close_group();
                index += ch.len_utf8();
            }
            '.' if scanner.at_top_level() => {
                let (field, field_end) = parse_field_access_segment(predicate, index)?;
                first_dot.get_or_insert(index);
                fields.push(field);
                index = field_end;
                let rest = predicate[index..].trim_start();
                if rest.is_empty() {
                    break;
                }
                if !rest.starts_with('.') {
                    return None;
                }
            }
            _ => index += ch.len_utf8(),
        }
    }
    let dot = first_dot?;
    let base = predicate[..dot].trim();
    (!base.is_empty() && !fields.is_empty()).then_some(FieldAccessRef { base, fields })
}

#[derive(Default)]
struct FieldAccessScanner {
    depth: usize,
    in_string: bool,
    escaped: bool,
}

impl FieldAccessScanner {
    fn consume_quoted(&mut self, ch: char) -> bool {
        if !self.in_string {
            return false;
        }
        if self.escaped {
            self.escaped = false;
        } else if ch == '\\' {
            self.escaped = true;
        } else if ch == '"' {
            self.in_string = false;
        }
        true
    }

    fn start_string(&mut self) {
        self.in_string = true;
    }

    fn open_group(&mut self) {
        self.depth += 1;
    }

    fn close_group(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn at_top_level(&self) -> bool {
        self.depth == 0
    }
}

fn parse_field_access_segment(predicate: &str, dot_index: usize) -> Option<(&str, usize)> {
    let field_start = dot_index + '.'.len_utf8();
    let field_first = predicate[field_start..].chars().next()?;
    if !(field_first.is_ascii_alphabetic() || field_first == '_') {
        return None;
    }
    let mut field_end = field_start + field_first.len_utf8();
    while field_end < predicate.len() {
        let next = predicate[field_end..].chars().next()?;
        if next.is_ascii_alphanumeric() || next == '_' {
            field_end += next.len_utf8();
        } else {
            break;
        }
    }
    Some((&predicate[field_start..field_end], field_end))
}

fn field_accesses(predicate: &str) -> Vec<FieldAccess> {
    let bytes = predicate.as_bytes();
    let mut accesses = Vec::new();
    for call in contract_calls(predicate) {
        if let Some(fields) = field_suffix(&predicate[call.end..]) {
            accesses.push(FieldAccess {
                base: predicate[call.start..call.end].to_string(),
                fields,
            });
        }
    }
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let ch = bytes[index] as char;
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() {
            let ch = bytes[index] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                index += 1;
            } else {
                break;
            }
        }
        if start >= 1 && &predicate[start - 1..start] == "." {
            continue;
        }
        if start >= 2 && &predicate[start - 2..start] == "::" {
            continue;
        }
        if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
            continue;
        }
        let base = predicate[start..index].to_string();
        let mut fields = Vec::new();
        while index < bytes.len() && &predicate[index..index + 1] == "." {
            let field_start = index + 1;
            if field_start >= bytes.len() {
                break;
            }
            let first = bytes[field_start] as char;
            if !(first.is_ascii_alphabetic() || first == '_') {
                break;
            }
            index = field_start + 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            fields.push(predicate[field_start..index].to_string());
        }
        if !fields.is_empty() {
            accesses.push(FieldAccess { base, fields });
        }
    }
    accesses
}

fn field_suffix(text: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut rest = text.trim_start();
    while let Some(after_dot) = rest.strip_prefix('.') {
        let mut chars = after_dot.char_indices();
        let (_, first) = chars.next()?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return None;
        }
        let mut end = first.len_utf8();
        for (index, ch) in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end = index + ch.len_utf8();
            } else {
                break;
            }
        }
        fields.push(after_dot[..end].to_string());
        rest = after_dot[end..].trim_start();
    }
    (!fields.is_empty()).then_some(fields)
}

fn is_complete_string_literal(text: &str) -> bool {
    if !text.starts_with('"') {
        return false;
    }
    string_literal_end(text, 0).is_some_and(|end| end == text.len())
}

fn string_literal_end(text: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut cursor = start + 1;
    while cursor < text.len() {
        let ch = text[cursor..].chars().next()?;
        cursor += ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(cursor);
        }
    }
    None
}
