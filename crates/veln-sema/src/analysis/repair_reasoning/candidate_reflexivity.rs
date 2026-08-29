use super::*;
pub(in crate::analysis) fn contract_callee_segments(callee: &str) -> Vec<String> {
    callee.split("::").map(ToString::to_string).collect()
}

pub(in crate::analysis) struct SatisfyRepairConstraint {
    pub(in crate::analysis) allowed_bindings: Option<Vec<SatisfyAllowedBinding>>,
    pub(in crate::analysis) reason: &'static str,
}

pub(in crate::analysis) struct SatisfyAllowedBinding {
    pub(in crate::analysis) name: String,
    pub(in crate::analysis) reason: &'static str,
}

impl SatisfyRepairConstraint {
    pub(in crate::analysis) fn from_satisfy(
        satisfy: &SatisfyClause,
        allow_static_truth: bool,
    ) -> Option<Self> {
        let candidate = satisfy.candidate.as_ref()?;
        if allow_static_truth
            && predicate_is_statically_true_with_literal_bounds(&satisfy.predicate)
        {
            return Some(Self {
                allowed_bindings: None,
                reason: "satisfy_tautology",
            });
        }
        if let Some(tautology) = tautological_candidate_predicate(&satisfy.predicate, candidate) {
            return Some(Self {
                allowed_bindings: None,
                reason: tautology.reason,
            });
        }
        if let Some(bindings) = reflexive_candidate_disjunct_bindings(&satisfy.predicate, candidate)
        {
            return Some(Self {
                allowed_bindings: Some(bindings),
                reason: "satisfy_reflexive_match",
            });
        }
        reflexive_candidate_binding(&satisfy.predicate, candidate).map(|allowed| Self {
            allowed_bindings: Some(vec![SatisfyAllowedBinding {
                name: allowed.binding,
                reason: allowed.reason,
            }]),
            reason: allowed.reason,
        })
    }

    pub(in crate::analysis) fn reason_for(&self, binding: &str) -> Option<&'static str> {
        match &self.allowed_bindings {
            Some(allowed) => allowed
                .iter()
                .find(|allowed_binding| allowed_binding.name == binding)
                .map(|allowed_binding| allowed_binding.reason),
            None => Some(self.reason),
        }
    }

    pub(in crate::analysis) fn allows_any_binding(&self) -> bool {
        self.allowed_bindings.is_none()
    }

    pub(in crate::analysis) fn extend_allowed_bindings(
        &mut self,
        bindings: Vec<SatisfyAllowedBinding>,
    ) {
        let Some(allowed) = &mut self.allowed_bindings else {
            return;
        };
        for binding in bindings {
            if !allowed.iter().any(|existing| existing.name == binding.name) {
                allowed.push(binding);
            }
        }
    }
}

pub(in crate::analysis) fn reflexive_candidate_disjunct_bindings(
    predicate: &str,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    if repair_relevant_negated_and_clauses(predicate).is_some() {
        return None;
    }
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.len() <= 1 {
        return reflexive_candidate_conjunction_bindings(
            repair_relevant_and_clauses(predicate),
            candidate,
        );
    }
    let mut bindings = Vec::new();
    for disjunct in disjuncts {
        let Some(direct) = reflexive_candidate_conjunction_bindings(
            repair_relevant_and_clauses(disjunct),
            candidate,
        ) else {
            continue;
        };
        for direct_allowed in direct {
            if !bindings
                .iter()
                .any(|binding: &SatisfyAllowedBinding| binding.name == direct_allowed.name)
            {
                bindings.push(direct_allowed);
            }
        }
    }
    (!bindings.is_empty()).then_some(bindings)
}

pub(in crate::analysis) struct ReflexiveCandidateBinding {
    pub(in crate::analysis) binding: String,
    pub(in crate::analysis) reason: &'static str,
}

pub(in crate::analysis) fn reflexive_candidate_binding(
    predicate: &str,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let expanded_disjuncts = repair_relevant_negated_and_clauses(predicate);
    let disjuncts = expanded_disjuncts.as_deref().map_or_else(
        || repair_relevant_or_clauses(predicate),
        |clauses| clauses.iter().map(String::as_str).collect(),
    );
    if disjuncts.is_empty() {
        return None;
    }
    if disjuncts.len() > 1 {
        return reflexive_candidate_disjunction(disjuncts, candidate);
    }
    reflexive_candidate_conjunction(repair_relevant_and_clauses(disjuncts[0]), candidate)
}

pub(in crate::analysis) fn reflexive_candidate_disjunction(
    disjuncts: Vec<&str>,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let mut allowed_binding = None::<String>;
    let mut reason = "satisfy_equality_match";
    for disjunct in disjuncts {
        let direct =
            reflexive_candidate_conjunction(repair_relevant_and_clauses(disjunct), candidate)?;
        if let Some(existing) = &allowed_binding {
            if existing != &direct.binding {
                return None;
            }
        } else {
            allowed_binding = Some(direct.binding);
        }
        if direct.reason != "satisfy_equality_match" {
            reason = direct.reason;
        }
    }
    allowed_binding.map(|binding| ReflexiveCandidateBinding { binding, reason })
}

pub(in crate::analysis) fn reflexive_candidate_conjunction(
    clauses: Vec<String>,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let allowed_bindings = reflexive_candidate_conjunction_bindings(clauses, candidate)?;
    let [allowed] = allowed_bindings.as_slice() else {
        return None;
    };
    Some(ReflexiveCandidateBinding {
        binding: allowed.name.clone(),
        reason: allowed.reason,
    })
}

pub(in crate::analysis) fn reflexive_candidate_conjunction_bindings(
    clauses: Vec<String>,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    let mut allowed_bindings = None::<Vec<SatisfyAllowedBinding>>;
    for clause in clauses {
        if is_surplus_tautology_clause(&clause, candidate) {
            continue;
        }
        let direct = reflexive_candidate_clause_bindings(&clause, candidate)?;
        if let Some(existing) = &mut allowed_bindings {
            existing.retain(|allowed| {
                direct
                    .iter()
                    .any(|direct_allowed| direct_allowed.name == allowed.name)
            });
            for allowed in existing.iter_mut() {
                if allowed.reason == "satisfy_equality_match"
                    && let Some(direct_allowed) = direct
                        .iter()
                        .find(|direct_allowed| direct_allowed.name == allowed.name)
                {
                    allowed.reason = direct_allowed.reason;
                }
            }
            if existing.is_empty() {
                return None;
            }
        } else {
            allowed_bindings = Some(direct);
        }
    }
    allowed_bindings
}

pub(in crate::analysis) fn reflexive_candidate_clause_bindings(
    clause: &str,
    candidate: &str,
) -> Option<Vec<SatisfyAllowedBinding>> {
    let disjuncts = repair_relevant_or_clauses(clause);
    if disjuncts.len() > 1 {
        let bindings = disjuncts
            .into_iter()
            .filter_map(|disjunct| direct_reflexive_clause(disjunct, candidate))
            .fold(
                Vec::<SatisfyAllowedBinding>::new(),
                |mut bindings, direct| {
                    if !bindings
                        .iter()
                        .any(|binding| binding.name == direct.binding)
                    {
                        bindings.push(SatisfyAllowedBinding {
                            name: direct.binding,
                            reason: direct.reason,
                        });
                    }
                    bindings
                },
            );
        return (!bindings.is_empty()).then_some(bindings);
    }
    direct_reflexive_clause(clause, candidate).map(|direct| {
        vec![SatisfyAllowedBinding {
            name: direct.binding,
            reason: direct.reason,
        }]
    })
}

pub(in crate::analysis) struct TautologicalCandidatePredicate {
    reason: &'static str,
}

pub(in crate::analysis) fn tautological_candidate_predicate(
    predicate: &str,
    candidate: &str,
) -> Option<TautologicalCandidatePredicate> {
    if has_true_disjunct(predicate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if repair_relevant_negated_and_clauses(predicate)
        .as_deref()
        .is_some_and(|clauses| clauses.iter().any(|clause| clause == "true"))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if repair_relevant_negated_and_clauses(predicate)
        .as_deref()
        .is_some_and(|clauses| {
            let disjuncts = clauses.iter().map(String::as_str).collect::<Vec<_>>();
            has_complementary_candidate_disjuncts(&disjuncts, candidate)
        })
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if negated_and_clauses(predicate)
        .is_some_and(|clauses| has_exclusive_order_candidate_conjuncts(&clauses, candidate))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if negated_and_clauses(predicate).is_some_and(|clauses| {
        has_exclusive_inclusive_order_candidate_conjuncts(&clauses, candidate)
    }) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    let disjuncts = repair_relevant_or_clauses(predicate);
    if disjuncts.is_empty() {
        return None;
    }
    if has_complementary_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if has_inclusive_total_order_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if has_total_order_candidate_disjuncts(&disjuncts, candidate) {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    if disjuncts
        .into_iter()
        .any(|disjunct| is_candidate_tautology_disjunct(disjunct, candidate))
    {
        return Some(TautologicalCandidatePredicate {
            reason: "satisfy_tautology",
        });
    }
    None
}

pub(in crate::analysis) fn has_complementary_candidate_disjuncts(
    disjuncts: &[&str],
    candidate: &str,
) -> bool {
    if has_complementary_candidate_comparison_disjuncts(disjuncts, candidate) {
        return true;
    }
    let mut positive = Vec::<String>::new();
    let mut negative = Vec::<String>::new();
    for disjunct in disjuncts {
        let Some((negated, clause)) = complementary_disjunct_key(disjunct, candidate) else {
            continue;
        };
        let complements = if negated { &positive } else { &negative };
        if complements.iter().any(|existing| existing == &clause) {
            return true;
        }
        if negated {
            negative.push(clause);
        } else {
            positive.push(clause);
        }
    }
    false
}

pub(in crate::analysis) fn has_complementary_candidate_comparison_disjuncts(
    disjuncts: &[&str],
    candidate: &str,
) -> bool {
    disjuncts.iter().enumerate().any(|(index, left)| {
        disjuncts
            .iter()
            .skip(index + 1)
            .any(|right| complementary_candidate_comparisons(left, right, candidate))
    })
}

pub(in crate::analysis) fn complementary_candidate_comparisons(
    left: &str,
    right: &str,
    candidate: &str,
) -> bool {
    if !expression_references_identifier(left, candidate)
        && !expression_references_identifier(right, candidate)
    {
        return false;
    }
    let Some(left) = NormalizedRepairComparison::parse(left) else {
        return false;
    };
    let Some(right) = NormalizedRepairComparison::parse(right) else {
        return false;
    };
    match (left.operator, right.operator) {
        ("==", "!=") | ("!=", "==") => left.same_operands_unordered(&right),
        ("<", "<=") | ("<=", "<") => {
            left.same_operands_reversed(&right) || left.same_operands_unordered(&right)
        }
        _ => false,
    }
}

pub(in crate::analysis) fn has_inclusive_total_order_candidate_disjuncts(
    disjuncts: &[&str],
    candidate: &str,
) -> bool {
    disjuncts.iter().enumerate().any(|(index, left)| {
        let Some(left) = inclusive_total_order_candidate_clause(left, candidate) else {
            return false;
        };
        disjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|right| inclusive_total_order_candidate_clause(right, candidate))
            .any(|right| left.left == right.right && left.right == right.left)
    })
}

pub(in crate::analysis) struct InclusiveTotalOrderCandidateClause {
    left: String,
    right: String,
}

pub(in crate::analysis) fn inclusive_total_order_candidate_clause(
    disjunct: &str,
    candidate: &str,
) -> Option<InclusiveTotalOrderCandidateClause> {
    if !expression_references_identifier(disjunct, candidate) {
        return None;
    }
    let parsed = NormalizedRepairComparison::parse(disjunct)?;
    if parsed.operator != "<=" {
        return None;
    }
    let left = compact_predicate_text(parsed.left);
    let right = compact_predicate_text(parsed.right);
    (left != right).then_some(InclusiveTotalOrderCandidateClause { left, right })
}

pub(in crate::analysis) fn has_total_order_candidate_disjuncts(
    disjuncts: &[&str],
    candidate: &str,
) -> bool {
    if disjuncts.len() < 3 {
        return false;
    }
    disjuncts.iter().enumerate().any(|(index, disjunct)| {
        let Some(first) = total_order_candidate_clause(disjunct, candidate) else {
            return false;
        };
        disjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| total_order_candidate_clause(other, candidate))
            .filter(|other| other.left == first.left && other.right == first.right)
            .fold(first.relation.bit(), |mask, other| {
                mask | other.relation.bit()
            })
            == TotalOrderRelation::ALL_BITS
    })
}

pub(in crate::analysis) fn has_exclusive_order_candidate_conjuncts(
    conjuncts: &[&str],
    candidate: &str,
) -> bool {
    if conjuncts.len() < 2 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(index, conjunct)| {
        let Some(first) = total_order_candidate_clause(conjunct, candidate) else {
            return false;
        };
        conjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| total_order_candidate_clause(other, candidate))
            .any(|other| {
                other.left == first.left
                    && other.right == first.right
                    && other.relation != first.relation
            })
    })
}

pub(in crate::analysis) fn has_exclusive_inclusive_order_candidate_conjuncts(
    conjuncts: &[&str],
    candidate: &str,
) -> bool {
    if conjuncts.len() < 2 {
        return false;
    }
    conjuncts.iter().enumerate().any(|(index, conjunct)| {
        let Some(first) = order_bound_candidate_clause(conjunct, candidate) else {
            return false;
        };
        conjuncts
            .iter()
            .skip(index + 1)
            .filter_map(|other| order_bound_candidate_clause(other, candidate))
            .any(|other| {
                first.left == other.right
                    && first.right == other.left
                    && (first.strict || other.strict)
            })
    })
}

pub(in crate::analysis) struct OrderBoundCandidateClause {
    left: String,
    right: String,
    strict: bool,
}

pub(in crate::analysis) fn order_bound_candidate_clause(
    conjunct: &str,
    candidate: &str,
) -> Option<OrderBoundCandidateClause> {
    let conjunct = strip_balanced_outer_parens(conjunct);
    if !expression_references_identifier(conjunct, candidate) {
        return None;
    }
    let parsed = ParsedRepairComparison::parse(conjunct)?;
    let mut left = compact_predicate_text(parsed.left);
    let mut right = compact_predicate_text(parsed.right);
    if left == right {
        return None;
    }
    match parsed.operator {
        ">" | ">=" => std::mem::swap(&mut left, &mut right),
        "<" | "<=" => {}
        _ => return None,
    }
    Some(OrderBoundCandidateClause {
        left,
        right,
        strict: matches!(parsed.operator, "<" | ">"),
    })
}
