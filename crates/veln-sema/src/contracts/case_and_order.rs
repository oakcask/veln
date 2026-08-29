use super::*;

pub(super) fn has_exhaustive_case_split_top_level_or_between(
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

pub(super) fn exhaustive_case_split_is_complete(disjuncts: &[&str], bases: &[&str]) -> bool {
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

pub(super) fn predicate_polarity_against(predicate: &str, base: &str) -> Option<bool> {
    if same_predicate(predicate, base) {
        Some(true)
    } else if complementary_predicates(predicate, base) {
        Some(false)
    } else {
        None
    }
}

pub(super) fn non_static_conjuncts(predicate: &str) -> Vec<&str> {
    flattened_keyword_clauses(predicate, "and")
        .into_iter()
        .filter(|conjunct| static_boolean_value(conjunct) != StaticBooleanValue::True)
        .collect()
}

pub(super) fn non_static_disjuncts(predicate: &str) -> Vec<&str> {
    flattened_keyword_clauses(predicate, "or")
        .into_iter()
        .filter(|disjunct| static_boolean_value(disjunct) != StaticBooleanValue::False)
        .collect()
}

pub(super) fn has_case_split_top_level_or(predicate: &str) -> bool {
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

pub(super) fn disjunct_case_splits_to_true(left: &str, right: &str) -> bool {
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

pub(super) fn static_true_conjunction_variant(predicate: &str) -> Option<&str> {
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

pub(super) fn has_total_order_top_level_or(predicate: &str) -> bool {
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

pub(super) fn has_inclusive_total_order_top_level_or(predicate: &str) -> bool {
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

pub(super) fn has_disequality_strict_order_split_top_level_or(predicate: &str) -> bool {
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

pub(super) fn has_disequality_inclusive_order_split_top_level_or(predicate: &str) -> bool {
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

pub(super) fn has_strict_order_bound(
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

pub(super) fn has_inclusive_order_bound(
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

pub(super) fn has_exclusive_order_top_level_and(predicate: &str) -> bool {
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

pub(super) fn has_exclusive_inclusive_order_top_level_and(predicate: &str) -> bool {
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
