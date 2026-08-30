use super::*;

pub(super) fn static_comparison_value(predicate: &str) -> Option<bool> {
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

pub(super) fn static_boolean_formula_comparison(
    left: &str,
    operator: &str,
    right: &str,
) -> Option<bool> {
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

pub(super) fn complementary_predicates(left: &str, right: &str) -> bool {
    let (left_shape, left_polarity) = normalized_predicate_polarity(left);
    let (right_shape, right_polarity) = normalized_predicate_polarity(right);
    (left_shape == right_shape && left_polarity != right_polarity)
        || complementary_comparisons(left, right)
}

pub(super) fn normalized_predicate_polarity(predicate: &str) -> (String, bool) {
    if let Some(inner) = negated_predicate_inner(predicate) {
        let (shape, polarity) = normalized_predicate_polarity(inner);
        return (shape, !polarity);
    }
    if let Some(alias) = boolean_literal_alias_shape(predicate) {
        return alias;
    }
    (predicate_shape(predicate), true)
}

pub(super) fn has_complementary_top_level_clauses(predicate: &str, keyword: &str) -> bool {
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

pub(super) fn has_negated_conjunction_top_level_or(predicate: &str) -> bool {
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

pub(super) fn negated_conjunction_contains(negated: &str, predicate: &str) -> bool {
    let Some(inner) = negated_predicate_inner(negated) else {
        return false;
    };
    let clauses = flattened_keyword_clauses(inner, "and");
    clauses.len() > 1
        && clauses
            .iter()
            .any(|clause| same_predicate(clause, predicate))
}

pub(super) fn has_negated_disjunction_top_level_and(predicate: &str) -> bool {
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

pub(super) fn has_negated_disjunction_covered_by_disjuncts(predicate: &str) -> bool {
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

pub(super) fn negated_disjunction_contains(negated: &str, predicate: &str) -> bool {
    let Some(inner) = negated_predicate_inner(negated) else {
        return false;
    };
    let clauses = flattened_keyword_clauses(inner, "or");
    clauses.len() > 1
        && clauses
            .iter()
            .any(|clause| same_predicate(clause, predicate))
}

pub(super) fn has_conjunction_covered_by_complement_disjuncts(predicate: &str) -> bool {
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

pub(super) fn has_disjunction_covered_by_complement_conjuncts(predicate: &str) -> bool {
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

pub(super) fn has_resolved_complementary_disjunctions_top_level_and(predicate: &str) -> bool {
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

pub(super) fn resolvable_disjunction_pair_is_contradicted(
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

pub(super) fn has_transitive_order_contradiction_top_level_and(predicate: &str) -> bool {
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

pub(super) fn has_transitive_strict_order_cycle_top_level_and(predicate: &str) -> bool {
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

pub(super) fn has_factored_case_split_covered_by_complements(predicate: &str) -> bool {
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

pub(super) fn has_partial_case_split_top_level_or(predicate: &str) -> bool {
    has_partial_case_split(
        predicate,
        "or",
        non_static_conjuncts,
        AssignmentPolarity::Matching,
    )
}

#[derive(Clone, Copy)]
enum AssignmentPolarity {
    Matching,
    Opposite,
}

fn has_partial_case_split(
    predicate: &str,
    outer_keyword: &str,
    inner_clauses: for<'a> fn(&'a str) -> Vec<&'a str>,
    assignment_polarity: AssignmentPolarity,
) -> bool {
    let outer_clauses = flattened_keyword_clauses(predicate, outer_keyword);
    if outer_clauses.len() < 3 {
        return false;
    }
    let mut bases: Vec<&str> = Vec::new();
    for outer_clause in &outer_clauses {
        for inner_clause in inner_clauses(outer_clause) {
            if bases.iter().all(|base| {
                !same_predicate(base, inner_clause) && !complementary_predicates(base, inner_clause)
            }) {
                bases.push(inner_clause);
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
    let mut accounted_for = vec![false; assignment_count];
    let mut accounted_for_count = 0;
    for outer_clause in outer_clauses {
        let Some(assignments) = partial_case_split_assignments(
            outer_clause,
            &bases,
            inner_clauses,
            assignment_polarity,
        ) else {
            continue;
        };
        for assignment in assignments {
            if !accounted_for[assignment] {
                accounted_for[assignment] = true;
                accounted_for_count += 1;
                if accounted_for_count == assignment_count {
                    return true;
                }
            }
        }
    }
    false
}

fn partial_case_split_assignments(
    outer_clause: &str,
    bases: &[&str],
    inner_clauses: for<'a> fn(&'a str) -> Vec<&'a str>,
    assignment_polarity: AssignmentPolarity,
) -> Option<Vec<usize>> {
    let mut polarities = vec![None; bases.len()];
    for inner_clause in inner_clauses(outer_clause) {
        let mut matched = false;
        for (index, base) in bases.iter().enumerate() {
            if let Some(polarity) = predicate_polarity_against(inner_clause, base) {
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
            let polarity_matches = (assignment & bit != 0) == *expected;
            if polarity_matches != matches!(assignment_polarity, AssignmentPolarity::Matching) {
                continue 'assignments;
            }
        }
        covered.push(assignment);
    }
    Some(covered)
}

pub(super) fn has_partial_case_split_top_level_and(predicate: &str) -> bool {
    has_partial_case_split(
        predicate,
        "and",
        non_static_disjuncts,
        AssignmentPolarity::Opposite,
    )
}
