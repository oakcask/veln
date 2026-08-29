use super::*;

pub(in crate::analysis) fn flattened_repair_keyword_clauses<'a>(
    predicate: &'a str,
    keyword: &str,
) -> Vec<&'a str> {
    let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), keyword);
    if clauses.len() <= 1 {
        return clauses;
    }
    clauses
        .into_iter()
        .flat_map(|clause| flattened_repair_keyword_clauses(clause, keyword))
        .collect()
}

pub(in crate::analysis) fn predicate_guaranteed_by_required_predicates(
    predicate: &str,
    required_predicates: &[String],
) -> bool {
    if required_predicate_set_statically_implies_predicate(required_predicates, predicate) {
        return true;
    }
    if required_predicates
        .iter()
        .any(|required| required_predicate_implies_disjunctive_predicate(required, predicate))
    {
        return true;
    }
    if required_predicate_set_implies_disjunctive_predicate(required_predicates, predicate) {
        return true;
    }
    repair_relevant_or_clause_strings(predicate)
        .into_iter()
        .map(|disjunct| repair_relevant_and_clauses(&disjunct))
        .any(|disjunct_clauses| {
            !disjunct_clauses.is_empty()
                && disjunct_clauses.iter().all(|clause| {
                    repair_clause_guaranteed_by_required_predicates(clause, required_predicates)
                })
        })
}

pub(in crate::analysis) fn int_successor_predicate_guaranteed_by_required_predicates(
    predicate: &str,
    required_predicates: &[String],
) -> bool {
    repair_relevant_or_clause_strings(predicate)
        .into_iter()
        .map(|disjunct| repair_relevant_and_clauses(&disjunct))
        .any(|disjunct_clauses| {
            !disjunct_clauses.is_empty()
                && disjunct_clauses.iter().all(|clause| {
                    repair_clause_guaranteed_by_required_predicates(clause, required_predicates)
                        || int_successor_clause_guaranteed_by_required_predicates(
                            clause,
                            required_predicates,
                        )
                })
        })
}

pub(in crate::analysis) fn int_successor_clause_guaranteed_by_required_predicates(
    clause: &str,
    required_predicates: &[String],
) -> bool {
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    if repair_clause_set_int_successor_implies_clause(&required_clauses, clause) {
        return true;
    }
    required_predicates
        .iter()
        .any(|required| required_predicate_int_successor_implies_clause(required, clause))
}

pub(in crate::analysis) fn repair_clause_set_int_successor_implies_clause(
    required_clauses: &[String],
    wanted: &str,
) -> bool {
    let Some(wanted) = NormalizedRepairComparison::parse(wanted) else {
        return false;
    };
    let equivalences = repair_equivalences(required_clauses);
    required_clauses.iter().any(|required| {
        let Some(required) = NormalizedRepairComparison::parse(required) else {
            return false;
        };
        int_successor_repair_comparison_implies(&required, &wanted, &equivalences)
    })
}

pub(in crate::analysis) fn required_predicate_int_successor_implies_clause(
    predicate: &str,
    wanted: &str,
) -> bool {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return disjuncts
            .into_iter()
            .all(|disjunct| required_predicate_int_successor_implies_clause(disjunct, wanted));
    }
    if disjuncts.is_empty() {
        return false;
    }
    let conjuncts = split_top_level_keyword(disjuncts[0], "and");
    if conjuncts.len() > 1 {
        return conjuncts
            .into_iter()
            .any(|conjunct| required_predicate_int_successor_implies_clause(conjunct, wanted));
    }
    let canonical = canonical_repair_clause(disjuncts[0]);
    int_successor_repair_clause_implies(&canonical, wanted)
}

pub(in crate::analysis) fn int_successor_repair_clause_implies(
    required: &str,
    wanted: &str,
) -> bool {
    let Some(required) = NormalizedRepairComparison::parse(required) else {
        return false;
    };
    let Some(wanted) = NormalizedRepairComparison::parse(wanted) else {
        return false;
    };
    int_successor_repair_comparison_implies(&required, &wanted, &RepairEquivalences::default())
}

pub(in crate::analysis) fn int_successor_repair_comparison_implies(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    match (required.operator, wanted.operator) {
        ("<", "<=") => strict_int_bound_implies_adjacent_inclusive(required, wanted, equivalences),
        ("<=", "<") => inclusive_int_bound_implies_adjacent_strict(required, wanted, equivalences),
        _ => false,
    }
}

pub(in crate::analysis) fn strict_int_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    strict_int_lower_bound_implies_adjacent_inclusive(required, wanted, equivalences)
        || strict_int_upper_bound_implies_adjacent_inclusive(required, wanted, equivalences)
}

pub(in crate::analysis) fn inclusive_int_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    inclusive_int_lower_bound_implies_adjacent_strict(required, wanted, equivalences)
        || inclusive_int_upper_bound_implies_adjacent_strict(required, wanted, equivalences)
}

pub(in crate::analysis) fn strict_int_lower_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && repair_numeric_order_literal(required.left).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.left).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(1)
                })
        })
}

pub(in crate::analysis) fn strict_int_upper_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && repair_numeric_order_literal(required.right).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.right).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(-1)
                })
        })
}

pub(in crate::analysis) fn inclusive_int_lower_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && repair_numeric_order_literal(required.left).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.left).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(-1)
                })
        })
}

pub(in crate::analysis) fn inclusive_int_upper_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && repair_numeric_order_literal(required.right).is_some_and(|required_literal| {
            required_literal.is_integer()
                && repair_numeric_order_literal(wanted.right).is_some_and(|wanted_literal| {
                    wanted_literal.is_integer()
                        && Some(wanted_literal) == required_literal.add_int(1)
                })
        })
}

pub(in crate::analysis) fn required_predicate_set_statically_implies_predicate(
    required_predicates: &[String],
    predicate: &str,
) -> bool {
    if required_predicates.is_empty() {
        return false;
    }
    let antecedent = required_predicates
        .iter()
        .map(|required| format!("({required})"))
        .collect::<Vec<_>>()
        .join(" and ");
    contract_predicate_is_statically_true(&format!("not ({antecedent}) or ({predicate})"))
}

pub(in crate::analysis) fn required_predicate_implies_disjunctive_predicate(
    required: &str,
    wanted: &str,
) -> bool {
    let wanted_disjuncts = repair_relevant_or_clauses(wanted)
        .into_iter()
        .map(canonical_repair_clause)
        .collect::<Vec<_>>();
    if wanted_disjuncts.len() <= 1 {
        return false;
    }
    let required_disjuncts = repair_relevant_negated_and_clauses(required).unwrap_or_else(|| {
        repair_relevant_or_clauses(required)
            .into_iter()
            .map(canonical_repair_clause)
            .collect()
    });
    if required_disjuncts.len() <= 1 {
        return false;
    }
    required_disjuncts.iter().all(|required_disjunct| {
        wanted_disjuncts
            .iter()
            .any(|wanted_disjunct| repair_clause_implies(required_disjunct, wanted_disjunct))
    })
}

pub(in crate::analysis) fn required_predicate_set_implies_disjunctive_predicate(
    required_predicates: &[String],
    wanted: &str,
) -> bool {
    let wanted_disjuncts = repair_relevant_or_clauses(wanted)
        .into_iter()
        .map(canonical_repair_clause)
        .collect::<Vec<_>>();
    if wanted_disjuncts.len() <= 1 {
        return false;
    }
    if disjunctive_branch_set_implies_disjunctive_predicate(required_predicates, &wanted_disjuncts)
    {
        return true;
    }
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    let equivalences = repair_equivalences(&required_clauses);
    required_clauses.iter().any(|required| {
        disequality_implies_numeric_ordering_disjunction(required, &wanted_disjuncts, &equivalences)
            || inclusive_bound_implies_order_or_equality_disjunction(
                required,
                &wanted_disjuncts,
                &equivalences,
            )
    })
}

pub(in crate::analysis) fn disjunctive_branch_set_implies_disjunctive_predicate(
    required_predicates: &[String],
    wanted_disjuncts: &[String],
) -> bool {
    required_predicates
        .iter()
        .enumerate()
        .any(|(disjunctive_index, predicate)| {
            let disjuncts = repair_relevant_or_clauses(predicate);
            disjuncts.len() > 1
                && disjuncts.into_iter().all(|disjunct| {
                    let branch_clauses =
                        branch_required_clauses(required_predicates, disjunctive_index, disjunct);
                    wanted_disjuncts
                        .iter()
                        .any(|wanted| repair_clause_set_implies_clause(&branch_clauses, wanted))
                })
        })
}

pub(in crate::analysis) fn repair_relevant_or_clause_strings(predicate: &str) -> Vec<String> {
    repair_relevant_negated_and_clauses(predicate).unwrap_or_else(|| {
        repair_relevant_or_clauses(predicate)
            .into_iter()
            .map(ToString::to_string)
            .collect()
    })
}

pub(in crate::analysis) fn repair_clause_guaranteed_by_required_predicates(
    clause: &str,
    required_predicates: &[String],
) -> bool {
    if has_true_disjunct(clause) {
        return true;
    }
    let disjuncts = repair_relevant_or_clauses(clause);
    if disjuncts.len() > 1 {
        return disjuncts.into_iter().any(|disjunct| {
            repair_clause_guaranteed_by_required_predicates(disjunct, required_predicates)
        });
    }
    if disjuncts.is_empty() {
        return false;
    }
    let canonical = canonical_repair_clause(disjuncts[0]);
    required_predicates
        .iter()
        .any(|required| required_predicate_implies_clause(required, &canonical))
        || required_predicate_set_implies_clause(required_predicates, &canonical)
}

pub(in crate::analysis) fn required_predicate_implies_clause(
    predicate: &str,
    wanted: &str,
) -> bool {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return disjuncts
            .into_iter()
            .all(|disjunct| required_predicate_implies_clause(disjunct, wanted));
    }
    if disjuncts.is_empty() {
        return false;
    }
    let predicate = disjuncts[0];
    let conjuncts = split_top_level_keyword(predicate, "and");
    if conjuncts.len() > 1 {
        return conjuncts
            .into_iter()
            .any(|conjunct| required_predicate_implies_clause(conjunct, wanted));
    }
    let canonical = canonical_repair_clause(predicate);
    repair_clause_implies(&canonical, wanted)
        || repair_atoms_equivalent(&canonical, wanted, &RepairEquivalences::default())
}

pub(in crate::analysis) fn repair_clause_implies(required: &str, wanted: &str) -> bool {
    if required == wanted {
        return true;
    }
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return false;
    };
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return boolean_atom_implies_literal_comparison(
            required,
            &wanted,
            &RepairEquivalences::default(),
        );
    };
    if required.left == wanted.left
        && required.right == wanted.right
        && matches!((required.operator, wanted.operator), ("<", "<="))
    {
        return true;
    }
    if required.operator == "<"
        && wanted.operator == "!="
        && same_repair_operands_unordered(required.left, required.right, wanted.left, wanted.right)
    {
        return true;
    }
    if equality_with_distinct_literal_implies_disequality(
        required.left,
        required.operator,
        required.right,
        wanted.left,
        wanted.operator,
        wanted.right,
        &RepairEquivalences::default(),
    ) {
        return true;
    }
    if literal_order_comparison_implies(&required, &wanted, &RepairEquivalences::default()) {
        return true;
    }
    if literal_equality_implies_order_comparison(&required, &wanted, &RepairEquivalences::default())
    {
        return true;
    }
    if literal_bound_implies_disequality(&required, &wanted, &RepairEquivalences::default()) {
        return true;
    }
    if boolean_literal_comparison_implies_comparison(
        &required,
        &wanted,
        &RepairEquivalences::default(),
    ) {
        return true;
    }
    required.operator == "=="
        && wanted.operator == "<="
        && same_repair_operands_unordered(required.left, required.right, wanted.left, wanted.right)
}

pub(in crate::analysis) fn required_predicate_set_implies_clause(
    required_predicates: &[String],
    wanted: &str,
) -> bool {
    let required_clauses = required_predicates
        .iter()
        .flat_map(|predicate| repair_set_clauses(predicate))
        .collect::<Vec<_>>();
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return repair_clause_set_implies_clause(&required_clauses, wanted)
            || disjunctive_branch_set_implies_clause(required_predicates, wanted);
    };
    if repair_clause_set_implies_comparison(&required_clauses, &wanted)
        || disjunctive_branch_set_implies_clause(required_predicates, wanted.clause)
    {
        return true;
    }
    false
}

pub(in crate::analysis) fn repair_clause_set_implies_clause(
    required_clauses: &[String],
    wanted: &str,
) -> bool {
    let equivalences = repair_equivalences(required_clauses);
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return required_clauses.iter().any(|required| {
            repair_atoms_equivalent(required, wanted, &equivalences)
                || boolean_literal_comparison_implies_atom(required, wanted, &equivalences)
        });
    };
    repair_clause_set_implies_comparison(required_clauses, &wanted)
}

pub(in crate::analysis) fn repair_clause_set_implies_comparison(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
) -> bool {
    let equivalences = repair_equivalences(required_clauses);
    if required_clauses
        .iter()
        .any(|required| repair_clause_implies_with_equivalences(required, wanted, &equivalences))
    {
        return true;
    }
    if boolean_disequality_alias_implies_comparison(required_clauses, wanted, &equivalences) {
        return true;
    }
    if ordering_path_implies_clause(required_clauses, wanted, &equivalences) {
        return true;
    }
    if wanted.operator != "==" {
        return false;
    }
    required_clauses.iter().any(|left| {
        required_clauses
            .iter()
            .any(|right| inclusive_bounds_imply_equality(left, right, wanted, &equivalences))
    })
}

pub(in crate::analysis) fn disjunctive_branch_set_implies_clause(
    required_predicates: &[String],
    wanted: &str,
) -> bool {
    required_predicates
        .iter()
        .enumerate()
        .any(|(disjunctive_index, predicate)| {
            let disjuncts = repair_relevant_or_clauses(predicate);
            disjuncts.len() > 1
                && disjuncts.into_iter().all(|disjunct| {
                    let branch_clauses =
                        branch_required_clauses(required_predicates, disjunctive_index, disjunct);
                    repair_clause_set_implies_clause(&branch_clauses, wanted)
                })
        })
}

pub(in crate::analysis) fn branch_required_clauses(
    required_predicates: &[String],
    disjunctive_index: usize,
    disjunct: &str,
) -> Vec<String> {
    required_predicates
        .iter()
        .enumerate()
        .flat_map(|(index, predicate)| {
            if index == disjunctive_index {
                repair_set_clauses(disjunct)
            } else {
                repair_set_clauses(predicate)
            }
        })
        .collect()
}

pub(in crate::analysis) fn non_disjunctive_repair_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() > 1 {
        return Vec::new();
    }
    let Some(predicate) = disjuncts.first().copied() else {
        return Vec::new();
    };
    split_top_level_keyword(predicate, "and")
        .into_iter()
        .flat_map(|clause| {
            let clause = strip_balanced_outer_parens(clause);
            if repair_relevant_or_clauses(clause).len() > 1 {
                Vec::new()
            } else {
                canonical_non_disjunctive_repair_clauses(clause)
            }
        })
        .collect()
}

pub(in crate::analysis) fn repair_set_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let mut clauses = non_disjunctive_repair_clauses(predicate);
    clauses.extend(disjunctive_common_repair_clauses(predicate));
    clauses
}

pub(in crate::analysis) fn disjunctive_common_repair_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let mut derived = Vec::new();
    for clause in split_top_level_keyword(predicate, "and") {
        let clause = strip_balanced_outer_parens(clause);
        let disjuncts = repair_relevant_or_clauses(clause);
        if disjuncts.len() <= 1 {
            continue;
        }
        let Some(first) = disjuncts.first().copied() else {
            continue;
        };
        for candidate in implied_clause_candidates(first) {
            if disjuncts
                .iter()
                .all(|disjunct| required_predicate_implies_clause(disjunct, &candidate))
                && !derived.iter().any(|existing| existing == &candidate)
            {
                derived.push(candidate);
            }
        }
    }
    derived
}

pub(in crate::analysis) fn implied_clause_candidates(predicate: &str) -> Vec<String> {
    non_disjunctive_repair_clauses(predicate)
        .into_iter()
        .flat_map(|clause| {
            let Some(parsed) = ParsedRepairComparison::parse(&clause) else {
                return vec![clause];
            };
            vec![
                format!("{} == {}", parsed.left, parsed.right),
                format!("{} != {}", parsed.left, parsed.right),
                format!("{} < {}", parsed.left, parsed.right),
                format!("{} < {}", parsed.right, parsed.left),
                format!("{} <= {}", parsed.left, parsed.right),
                format!("{} <= {}", parsed.right, parsed.left),
            ]
        })
        .map(canonical_repair_clause)
        .fold(Vec::<String>::new(), |mut candidates, candidate| {
            if !candidates.iter().any(|existing| existing == &candidate) {
                candidates.push(candidate);
            }
            candidates
        })
}

pub(in crate::analysis) fn canonical_non_disjunctive_repair_clauses(clause: &str) -> Vec<String> {
    if let Some(clauses) = repair_relevant_negated_and_clauses(clause) {
        return clauses;
    }
    canonical_negated_disjunction_repair_clauses(clause)
        .unwrap_or_else(|| vec![canonical_repair_clause(clause)])
}

pub(in crate::analysis) fn canonical_negated_disjunction_repair_clauses(
    clause: &str,
) -> Option<Vec<String>> {
    let trimmed = clause.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    let negated = strip_balanced_outer_parens(negated);
    let disjuncts = split_top_level_keyword(negated, "or")
        .into_iter()
        .filter(|disjunct| !disjunct.trim().is_empty())
        .collect::<Vec<_>>();
    if disjuncts.len() <= 1 {
        return None;
    }
    let clauses = disjuncts
        .into_iter()
        .map(|disjunct| canonical_negated_repair_or_atom_clause(&format!("not ({disjunct})")))
        .collect::<Option<Vec<_>>>()?;
    if clauses.iter().any(|clause| clause == "false") {
        return Some(vec!["false".to_string()]);
    }
    let clauses = clauses
        .into_iter()
        .filter(|clause| clause != "true")
        .collect::<Vec<_>>();
    Some(if clauses.is_empty() {
        vec!["true".to_string()]
    } else {
        clauses
    })
}
