use super::*;

pub(super) fn contract_callee_segments(callee: &str) -> Vec<String> {
    callee.split("::").map(ToString::to_string).collect()
}

pub(super) struct SatisfyRepairConstraint {
    pub(super) allowed_bindings: Option<Vec<SatisfyAllowedBinding>>,
    pub(super) reason: &'static str,
}

pub(super) struct SatisfyAllowedBinding {
    pub(super) name: String,
    pub(super) reason: &'static str,
}

impl SatisfyRepairConstraint {
    pub(super) fn from_satisfy(satisfy: &SatisfyClause, allow_static_truth: bool) -> Option<Self> {
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

    pub(super) fn reason_for(&self, binding: &str) -> Option<&'static str> {
        match &self.allowed_bindings {
            Some(allowed) => allowed
                .iter()
                .find(|allowed_binding| allowed_binding.name == binding)
                .map(|allowed_binding| allowed_binding.reason),
            None => Some(self.reason),
        }
    }

    pub(super) fn allows_any_binding(&self) -> bool {
        self.allowed_bindings.is_none()
    }

    pub(super) fn extend_allowed_bindings(&mut self, bindings: Vec<SatisfyAllowedBinding>) {
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

pub(super) fn reflexive_candidate_disjunct_bindings(
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

pub(super) struct ReflexiveCandidateBinding {
    binding: String,
    reason: &'static str,
}

pub(super) fn reflexive_candidate_binding(
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

pub(super) fn reflexive_candidate_disjunction(
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

pub(super) fn reflexive_candidate_conjunction(
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

pub(super) fn reflexive_candidate_conjunction_bindings(
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

pub(super) fn reflexive_candidate_clause_bindings(
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

pub(super) struct TautologicalCandidatePredicate {
    reason: &'static str,
}

pub(super) fn tautological_candidate_predicate(
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

pub(super) fn has_complementary_candidate_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
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

pub(super) fn has_complementary_candidate_comparison_disjuncts(
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

pub(super) fn complementary_candidate_comparisons(
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

pub(super) fn has_inclusive_total_order_candidate_disjuncts(
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

pub(super) struct InclusiveTotalOrderCandidateClause {
    left: String,
    right: String,
}

pub(super) fn inclusive_total_order_candidate_clause(
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

pub(super) fn has_total_order_candidate_disjuncts(disjuncts: &[&str], candidate: &str) -> bool {
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

pub(super) fn has_exclusive_order_candidate_conjuncts(conjuncts: &[&str], candidate: &str) -> bool {
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

pub(super) fn has_exclusive_inclusive_order_candidate_conjuncts(
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

pub(super) struct OrderBoundCandidateClause {
    left: String,
    right: String,
    strict: bool,
}

pub(super) fn order_bound_candidate_clause(
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TotalOrderRelation {
    Less,
    Equal,
    Greater,
}

impl TotalOrderRelation {
    const ALL_BITS: u8 = Self::Less.bit() | Self::Equal.bit() | Self::Greater.bit();

    const fn bit(self) -> u8 {
        match self {
            Self::Less => 0b001,
            Self::Equal => 0b010,
            Self::Greater => 0b100,
        }
    }

    pub(super) fn invert(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::Equal => Self::Equal,
            Self::Greater => Self::Less,
        }
    }
}

pub(super) struct TotalOrderCandidateClause {
    left: String,
    right: String,
    relation: TotalOrderRelation,
}

pub(super) fn total_order_candidate_clause(
    disjunct: &str,
    candidate: &str,
) -> Option<TotalOrderCandidateClause> {
    let disjunct = strip_balanced_outer_parens(disjunct);
    if !expression_references_identifier(disjunct, candidate) {
        return None;
    }
    let parsed = ParsedRepairComparison::parse(disjunct)?;
    let mut left = compact_predicate_text(parsed.left);
    let mut right = compact_predicate_text(parsed.right);
    if left == right {
        return None;
    }
    let mut relation = match parsed.operator {
        "==" => TotalOrderRelation::Equal,
        "<" => TotalOrderRelation::Less,
        ">" => TotalOrderRelation::Greater,
        _ => return None,
    };
    if right < left {
        std::mem::swap(&mut left, &mut right);
        relation = relation.invert();
    }
    Some(TotalOrderCandidateClause {
        left,
        right,
        relation,
    })
}

pub(super) fn complementary_disjunct_key(
    disjunct: &str,
    candidate: &str,
) -> Option<(bool, String)> {
    let disjunct = strip_balanced_outer_parens(disjunct);
    let (negated, clause) = stripped_not_operand(disjunct)
        .map(|inner| (true, strip_balanced_outer_parens(inner)))
        .unwrap_or((false, disjunct));
    expression_references_identifier(clause, candidate)
        .then(|| (negated, canonical_repair_clause(clause)))
}

pub(super) fn has_true_disjunct(predicate: &str) -> bool {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "or")
        .into_iter()
        .any(|clause| normalized_predicate_clause(clause) == "true")
}

pub(super) fn is_candidate_tautology_disjunct(predicate: &str, candidate: &str) -> bool {
    let clauses = repair_relevant_and_clauses(predicate);
    !clauses.is_empty()
        && clauses.iter().all(|clause| {
            is_surplus_tautology_clause(clause, candidate)
                || is_candidate_tautology_clause(clause, candidate)
        })
}

pub(super) fn is_surplus_tautology_clause(clause: &str, candidate: &str) -> bool {
    has_true_disjunct(clause)
        || predicate_is_statically_true(clause)
        || has_complementary_candidate_disjuncts(&repair_relevant_or_clauses(clause), candidate)
}

pub(super) fn is_candidate_tautology_clause(predicate: &str, candidate: &str) -> bool {
    let predicate = single_repair_relevant_clause(predicate).unwrap_or(predicate);
    let predicate = canonical_repair_clause(predicate);
    ["==", "<="].iter().any(|operator| {
        let Some((left, right)) = predicate.split_once(operator) else {
            return false;
        };
        if tautological_candidate_expression(left, right, candidate) {
            return true;
        }
        let Some(left) = operand_path(left) else {
            return false;
        };
        let Some(right) = operand_path(right) else {
            return false;
        };
        left.first().is_some_and(|base| *base == candidate) && left == right
    })
}

pub(super) fn tautological_candidate_expression(left: &str, right: &str, candidate: &str) -> bool {
    compact_direct_repair_expression_text(left) == compact_direct_repair_expression_text(right)
        && expression_references_identifier(left, candidate)
}

pub(super) fn direct_reflexive_clause(
    predicate: &str,
    candidate: &str,
) -> Option<ReflexiveCandidateBinding> {
    let predicate = single_repair_relevant_clause(predicate).unwrap_or(predicate);
    let predicate = canonical_repair_clause(predicate);
    if let Some(binding) = reflexive_operand(&predicate, candidate, "==") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_equality_match",
        });
    }
    if let Some(binding) = reflexive_expression_operand(&predicate, candidate, "==") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_equality_match",
        });
    }
    if let Some(binding) = reflexive_operand(&predicate, candidate, "<=") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_reflexive_match",
        });
    }
    if let Some(binding) = reflexive_expression_operand(&predicate, candidate, "<=") {
        return Some(ReflexiveCandidateBinding {
            binding,
            reason: "satisfy_reflexive_match",
        });
    }
    None
}

pub(super) fn reflexive_operand(
    predicate: &str,
    candidate: &str,
    operator: &str,
) -> Option<String> {
    let (left, right) = predicate.split_once(operator)?;
    let left = operand_path(left)?;
    let right = operand_path(right)?;
    reflexive_path_binding(&left, &right, candidate)
        .or_else(|| reflexive_path_binding(&right, &left, candidate))
}

pub(super) fn is_plain_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(super) fn operand_path(value: &str) -> Option<Vec<&str>> {
    value
        .trim()
        .split('.')
        .map(str::trim)
        .map(|segment| is_plain_identifier(segment).then_some(segment))
        .collect()
}

pub(super) fn reflexive_path_binding(
    left: &[&str],
    right: &[&str],
    candidate: &str,
) -> Option<String> {
    let (Some(left_base), Some(right_base)) = (left.first(), right.first()) else {
        return None;
    };
    if *left_base != candidate || *right_base == candidate || !is_plain_identifier(right_base) {
        return None;
    }
    (left[1..] == right[1..]).then(|| (*right_base).to_string())
}

pub(super) fn reflexive_expression_operand(
    predicate: &str,
    candidate: &str,
    operator: &str,
) -> Option<String> {
    let (left, right) = predicate.split_once(operator)?;
    reflexive_expression_binding(left, right, candidate)
        .or_else(|| reflexive_expression_binding(right, left, candidate))
}

pub(super) fn reflexive_expression_binding(
    candidate_expr: &str,
    binding_expr: &str,
    candidate: &str,
) -> Option<String> {
    if !expression_references_identifier(candidate_expr, candidate)
        || expression_references_identifier(binding_expr, candidate)
    {
        return None;
    }
    let matching_bindings = expression_identifiers(binding_expr)
        .into_iter()
        .filter(|binding| *binding != candidate)
        .filter(|binding| {
            is_plain_identifier(binding)
                && compact_direct_repair_expression_text(&replace_identifier(
                    candidate_expr,
                    candidate,
                    binding,
                )) == compact_direct_repair_expression_text(binding_expr)
        })
        .collect::<Vec<_>>();
    match matching_bindings.as_slice() {
        [binding] => Some((*binding).to_string()),
        _ => None,
    }
}

pub(super) fn expression_references_identifier(expression: &str, name: &str) -> bool {
    expression_identifiers(expression)
        .into_iter()
        .any(|identifier| identifier == name)
}

pub(super) fn expression_identifiers(expression: &str) -> Vec<&str> {
    let mut identifiers = Vec::new();
    let mut chars = expression.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '"' {
            let mut escaped = false;
            for (_, string_ch) in chars.by_ref() {
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if is_ident_start(ch) {
            let mut end = start + ch.len_utf8();
            while let Some((next, next_ch)) = chars.peek().copied() {
                if !is_ident_continue(next_ch) {
                    break;
                }
                chars.next();
                end = next + next_ch.len_utf8();
            }
            let ident = &expression[start..end];
            if is_value_identifier_position(expression, start, end)
                && !identifiers.iter().any(|existing| existing == &ident)
            {
                identifiers.push(ident);
            }
        }
    }
    identifiers
}

pub(super) fn compact_predicate_text(predicate: &str) -> String {
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

pub(super) fn compact_direct_repair_expression_text(predicate: &str) -> String {
    let mut current = compact_predicate_text(predicate);
    loop {
        let stripped = strip_redundant_repair_atom_parens(&current);
        if stripped == current {
            return current;
        }
        current = stripped;
    }
}

pub(super) fn strip_redundant_repair_atom_parens(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut cursor = 0;
    while cursor < predicate.len() {
        let rest = &predicate[cursor..];
        if let Some(inner_start) = rest.strip_prefix('(')
            && let Some(end) = inner_start.find(')')
        {
            let inner = &inner_start[..end];
            if is_repair_atom_text(inner) {
                output.push_str(inner);
                cursor += end + 2;
                continue;
            }
        }
        let ch = rest
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        output.push(ch);
        cursor += ch.len_utf8();
    }
    output
}

pub(super) fn is_repair_atom_text(text: &str) -> bool {
    operand_path(text).is_some() || repair_numeric_order_literal(text).is_some()
}

pub(super) fn normalized_and_clauses(predicate: &str) -> Vec<String> {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "and")
        .into_iter()
        .map(normalized_predicate_clause)
        .filter(|clause| !clause.is_empty())
        .collect()
}

pub(super) fn repair_relevant_and_clauses(predicate: &str) -> Vec<String> {
    normalized_and_clauses(predicate)
        .into_iter()
        .flat_map(|clause| {
            canonical_negated_disjunction_repair_clauses(&clause).unwrap_or_else(|| vec![clause])
        })
        .filter(|clause| clause != "true" && !contract_predicate_is_statically_true(clause))
        .collect()
}

pub(super) fn repair_relevant_or_clauses(predicate: &str) -> Vec<&str> {
    let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), "or");
    let has_disjunction = clauses.len() > 1;
    clauses
        .into_iter()
        .filter(|clause| {
            normalized_predicate_clause(clause) != "false"
                && (!has_disjunction || !predicate_is_statically_false(clause))
        })
        .collect()
}

pub(super) fn single_repair_relevant_clause(predicate: &str) -> Option<&str> {
    let clauses = repair_relevant_or_clauses(predicate);
    match clauses.as_slice() {
        [clause] => Some(*clause),
        _ => None,
    }
}

pub(super) fn repair_relevant_negated_and_clauses(predicate: &str) -> Option<Vec<String>> {
    let conjuncts = negated_and_clauses(predicate)?;
    if conjuncts.len() <= 1 {
        return None;
    }
    let clauses = conjuncts
        .into_iter()
        .map(|conjunct| canonical_negated_repair_or_atom_clause(&format!("not ({conjunct})")))
        .collect::<Option<Vec<_>>>()?;
    if clauses.iter().any(|clause| clause == "true") {
        return Some(vec!["true".to_string()]);
    }
    let clauses = clauses
        .into_iter()
        .filter(|clause| clause != "false")
        .collect::<Vec<_>>();
    Some(if clauses.is_empty() {
        vec!["false".to_string()]
    } else {
        clauses
    })
}

pub(super) fn negated_and_clauses(predicate: &str) -> Option<Vec<&str>> {
    let trimmed = predicate.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    Some(flattened_repair_keyword_clauses(negated, "and"))
}

pub(super) fn flattened_repair_keyword_clauses<'a>(
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

pub(super) fn predicate_guaranteed_by_required_predicates(
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

pub(super) fn int_successor_predicate_guaranteed_by_required_predicates(
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

pub(super) fn int_successor_clause_guaranteed_by_required_predicates(
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

pub(super) fn repair_clause_set_int_successor_implies_clause(
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

pub(super) fn required_predicate_int_successor_implies_clause(
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

pub(super) fn int_successor_repair_clause_implies(required: &str, wanted: &str) -> bool {
    let Some(required) = NormalizedRepairComparison::parse(required) else {
        return false;
    };
    let Some(wanted) = NormalizedRepairComparison::parse(wanted) else {
        return false;
    };
    int_successor_repair_comparison_implies(&required, &wanted, &RepairEquivalences::default())
}

pub(super) fn int_successor_repair_comparison_implies(
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

pub(super) fn strict_int_bound_implies_adjacent_inclusive(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    strict_int_lower_bound_implies_adjacent_inclusive(required, wanted, equivalences)
        || strict_int_upper_bound_implies_adjacent_inclusive(required, wanted, equivalences)
}

pub(super) fn inclusive_int_bound_implies_adjacent_strict(
    required: &NormalizedRepairComparison<'_>,
    wanted: &NormalizedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    inclusive_int_lower_bound_implies_adjacent_strict(required, wanted, equivalences)
        || inclusive_int_upper_bound_implies_adjacent_strict(required, wanted, equivalences)
}

pub(super) fn strict_int_lower_bound_implies_adjacent_inclusive(
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

pub(super) fn strict_int_upper_bound_implies_adjacent_inclusive(
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

pub(super) fn inclusive_int_lower_bound_implies_adjacent_strict(
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

pub(super) fn inclusive_int_upper_bound_implies_adjacent_strict(
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

pub(super) fn required_predicate_set_statically_implies_predicate(
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

pub(super) fn required_predicate_implies_disjunctive_predicate(
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

pub(super) fn required_predicate_set_implies_disjunctive_predicate(
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

pub(super) fn disjunctive_branch_set_implies_disjunctive_predicate(
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

pub(super) fn repair_relevant_or_clause_strings(predicate: &str) -> Vec<String> {
    repair_relevant_negated_and_clauses(predicate).unwrap_or_else(|| {
        repair_relevant_or_clauses(predicate)
            .into_iter()
            .map(ToString::to_string)
            .collect()
    })
}

pub(super) fn repair_clause_guaranteed_by_required_predicates(
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

pub(super) fn required_predicate_implies_clause(predicate: &str, wanted: &str) -> bool {
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

pub(super) fn repair_clause_implies(required: &str, wanted: &str) -> bool {
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

pub(super) fn required_predicate_set_implies_clause(
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

pub(super) fn repair_clause_set_implies_clause(required_clauses: &[String], wanted: &str) -> bool {
    let equivalences = repair_equivalences(required_clauses);
    let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
        return required_clauses.iter().any(|required| {
            repair_atoms_equivalent(required, wanted, &equivalences)
                || boolean_literal_comparison_implies_atom(required, wanted, &equivalences)
        });
    };
    repair_clause_set_implies_comparison(required_clauses, &wanted)
}

pub(super) fn repair_clause_set_implies_comparison(
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

pub(super) fn disjunctive_branch_set_implies_clause(
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

pub(super) fn branch_required_clauses(
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

pub(super) fn non_disjunctive_repair_clauses(predicate: &str) -> Vec<String> {
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

pub(super) fn repair_set_clauses(predicate: &str) -> Vec<String> {
    let predicate = strip_balanced_outer_parens(predicate);
    let mut clauses = non_disjunctive_repair_clauses(predicate);
    clauses.extend(disjunctive_common_repair_clauses(predicate));
    clauses
}

pub(super) fn disjunctive_common_repair_clauses(predicate: &str) -> Vec<String> {
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

pub(super) fn implied_clause_candidates(predicate: &str) -> Vec<String> {
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

pub(super) fn canonical_non_disjunctive_repair_clauses(clause: &str) -> Vec<String> {
    if let Some(clauses) = repair_relevant_negated_and_clauses(clause) {
        return clauses;
    }
    canonical_negated_disjunction_repair_clauses(clause)
        .unwrap_or_else(|| vec![canonical_repair_clause(clause)])
}

pub(super) fn canonical_negated_disjunction_repair_clauses(clause: &str) -> Option<Vec<String>> {
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

pub(super) fn inclusive_bounds_imply_equality(
    left: &str,
    right: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(left) = ParsedRepairComparison::parse(left) else {
        return false;
    };
    let Some(right) = ParsedRepairComparison::parse(right) else {
        return false;
    };
    left.operator == "<="
        && right.operator == "<="
        && repair_operands_equivalent_ordered(
            left.left,
            left.right,
            wanted.left,
            wanted.right,
            equivalences,
        )
        && repair_operands_equivalent_ordered(
            right.left,
            right.right,
            wanted.right,
            wanted.left,
            equivalences,
        )
}

pub(super) fn repair_clause_implies_with_equivalences(
    required: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return boolean_atom_implies_literal_comparison(required, wanted, equivalences);
    };
    if required.operator == wanted.operator
        && repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        )
    {
        return true;
    }
    match (required.operator, wanted.operator) {
        ("<", "<=") => repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ),
        ("<", "!=") | ("==", "<=") => repair_operands_equivalent_unordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ),
        ("==", "!=") => equality_with_distinct_literal_implies_disequality(
            required.left,
            required.operator,
            required.right,
            wanted.left,
            wanted.operator,
            wanted.right,
            equivalences,
        ),
        _ => {
            literal_order_comparison_implies(&required, wanted, equivalences)
                || literal_equality_implies_order_comparison(&required, wanted, equivalences)
                || literal_bound_implies_disequality(&required, wanted, equivalences)
                || boolean_literal_comparison_implies_comparison(&required, wanted, equivalences)
        }
    }
}

pub(super) fn literal_order_comparison_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if !matches!(required.operator, "<" | "<=") || !matches!(wanted.operator, "<" | "<=") {
        return false;
    }
    literal_lower_bound_implies(required, wanted, equivalences)
        || literal_upper_bound_implies(required, wanted, equivalences)
}

pub(super) fn literal_lower_bound_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.left) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.left) else {
        return false;
    };
    repair_operands_equivalent(required.right, wanted.right, equivalences)
        && literal_order_strength_implies(
            required_literal,
            required.operator,
            wanted_literal,
            wanted.operator,
            true,
        )
}

pub(super) fn literal_upper_bound_implies(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.right) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.right) else {
        return false;
    };
    repair_operands_equivalent(required.left, wanted.left, equivalences)
        && literal_order_strength_implies(
            required_literal,
            required.operator,
            wanted_literal,
            wanted.operator,
            false,
        )
}

pub(super) fn literal_equality_implies_order_comparison(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if required.operator != "==" || !matches!(wanted.operator, "<" | "<=") {
        return false;
    }
    literal_equality_implies_lower_bound(required, wanted, equivalences)
        || literal_equality_implies_upper_bound(required, wanted, equivalences)
}

pub(super) fn literal_equality_implies_lower_bound(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_subject, required_literal)) = literal_equality_subject(required) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.left) else {
        return false;
    };
    repair_operands_equivalent(required_subject, wanted.right, equivalences)
        && literal_order_strength_implies(
            required_literal,
            "<=",
            wanted_literal,
            wanted.operator,
            true,
        )
}

pub(super) fn literal_equality_implies_upper_bound(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_subject, required_literal)) = literal_equality_subject(required) else {
        return false;
    };
    let Some(wanted_literal) = repair_numeric_order_literal(wanted.right) else {
        return false;
    };
    repair_operands_equivalent(required_subject, wanted.left, equivalences)
        && literal_order_strength_implies(
            required_literal,
            "<=",
            wanted_literal,
            wanted.operator,
            false,
        )
}

pub(super) fn literal_equality_subject<'a>(
    required: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(required.left)
        .map(|literal| (required.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(required.right).map(|literal| (required.left, literal))
        })
}

pub(super) fn literal_order_strength_implies<T: Ord>(
    required_literal: T,
    required_operator: &str,
    wanted_literal: T,
    wanted_operator: &str,
    lower_bound: bool,
) -> bool {
    match required_literal.cmp(&wanted_literal) {
        std::cmp::Ordering::Greater if lower_bound => true,
        std::cmp::Ordering::Less if !lower_bound => true,
        std::cmp::Ordering::Equal => required_operator == "<" || wanted_operator == "<=",
        _ => false,
    }
}

pub(super) fn literal_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    if !matches!(required.operator, "<" | "<=") || wanted.operator != "!=" {
        return false;
    }
    literal_lower_bound_implies_disequality(required, wanted, equivalences)
        || literal_upper_bound_implies_disequality(required, wanted, equivalences)
}

pub(super) fn literal_lower_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.left) else {
        return false;
    };
    let Some(wanted_literal) =
        repair_disequality_literal_for_operand(wanted, required.right, equivalences)
    else {
        return false;
    };
    match wanted_literal.cmp(&required_literal) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Equal => required.operator == "<",
        std::cmp::Ordering::Greater => false,
    }
}

pub(super) fn literal_upper_bound_implies_disequality(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required_literal) = repair_numeric_order_literal(required.right) else {
        return false;
    };
    let Some(wanted_literal) =
        repair_disequality_literal_for_operand(wanted, required.left, equivalences)
    else {
        return false;
    };
    match wanted_literal.cmp(&required_literal) {
        std::cmp::Ordering::Greater => true,
        std::cmp::Ordering::Equal => required.operator == "<",
        std::cmp::Ordering::Less => false,
    }
}

pub(super) fn repair_disequality_literal_for_operand(
    wanted: &ParsedRepairComparison<'_>,
    operand: &str,
    equivalences: &RepairEquivalences,
) -> Option<RepairRational> {
    if repair_operands_equivalent(wanted.left, operand, equivalences) {
        return repair_numeric_order_literal(wanted.right);
    }
    if repair_operands_equivalent(wanted.right, operand, equivalences) {
        return repair_numeric_order_literal(wanted.left);
    }
    None
}

pub(super) fn disequality_implies_numeric_ordering_disjunction(
    required: &str,
    wanted_disjuncts: &[String],
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    if required.operator != "!=" {
        return false;
    }
    let Some((subject, excluded)) = numeric_literal_comparison_side(&required) else {
        return false;
    };
    let mut has_lower_side = false;
    let mut has_upper_side = false;
    for wanted in wanted_disjuncts {
        let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
            continue;
        };
        if wanted.operator != "<" {
            continue;
        }
        if repair_operands_equivalent(wanted.left, subject, equivalences)
            && repair_numeric_order_literal(wanted.right) == Some(excluded)
        {
            has_lower_side = true;
        }
        if repair_numeric_order_literal(wanted.left) == Some(excluded)
            && repair_operands_equivalent(wanted.right, subject, equivalences)
        {
            has_upper_side = true;
        }
    }
    has_lower_side && has_upper_side
}

pub(super) fn inclusive_bound_implies_order_or_equality_disjunction(
    required: &str,
    wanted_disjuncts: &[String],
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    if required.operator != "<=" {
        return false;
    }
    let mut has_strict_side = false;
    let mut has_equality_side = false;
    for wanted in wanted_disjuncts {
        let Some(wanted) = ParsedRepairComparison::parse(wanted) else {
            continue;
        };
        if repair_operands_equivalent_ordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ) && wanted.operator == "<"
        {
            has_strict_side = true;
        }
        if repair_operands_equivalent_unordered(
            required.left,
            required.right,
            wanted.left,
            wanted.right,
            equivalences,
        ) && wanted.operator == "=="
        {
            has_equality_side = true;
        }
    }
    has_strict_side && has_equality_side
}

pub(super) fn numeric_literal_comparison_side<'a>(
    comparison: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(comparison.left)
        .map(|literal| (comparison.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(comparison.right).map(|literal| (comparison.left, literal))
        })
}

pub(super) fn boolean_literal_comparison_implies_atom(
    required: &str,
    wanted_atom: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    let Some((required_atom, required_truth)) = boolean_literal_comparison_truth(&required) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_atom_truth(wanted_atom) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

pub(super) fn boolean_atom_implies_literal_comparison(
    required_atom: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_atom, required_truth)) = boolean_atom_truth(required_atom) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

pub(super) fn boolean_literal_comparison_implies_comparison(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((required_atom, required_truth)) = boolean_literal_comparison_truth(required) else {
        return false;
    };
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

pub(super) fn boolean_disequality_alias_implies_comparison(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some((wanted_atom, wanted_truth)) = boolean_literal_comparison_truth(wanted) else {
        return false;
    };
    required_clauses.iter().any(|required| {
        let Some(required) = ParsedRepairComparison::parse(required) else {
            return false;
        };
        if required.operator != "!=" {
            return false;
        }
        if repair_operands_equivalent(required.left, wanted_atom, equivalences) {
            return boolean_literal_value_for_operand(
                required_clauses,
                required.right,
                equivalences,
            ) == Some(!wanted_truth);
        }
        if repair_operands_equivalent(required.right, wanted_atom, equivalences) {
            return boolean_literal_value_for_operand(
                required_clauses,
                required.left,
                equivalences,
            ) == Some(!wanted_truth);
        }
        false
    })
}

pub(super) fn boolean_literal_value_for_operand(
    required_clauses: &[String],
    operand: &str,
    equivalences: &RepairEquivalences,
) -> Option<bool> {
    required_clauses.iter().find_map(|required| {
        let required = ParsedRepairComparison::parse(required)?;
        let (atom, truth) = boolean_literal_comparison_truth(&required)?;
        repair_operands_equivalent(atom, operand, equivalences).then_some(truth)
    })
}

pub(super) fn boolean_literal_comparison_truth<'a>(
    comparison: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, bool)> {
    let left_literal = RepairLiteral::parse(comparison.left);
    let right_literal = RepairLiteral::parse(comparison.right);
    let (atom, literal) = match (left_literal, right_literal) {
        (None, Some(RepairLiteral::Bool(value))) => (comparison.left, value),
        (Some(RepairLiteral::Bool(value)), None) => (comparison.right, value),
        _ => return None,
    };
    let atom_truth = match comparison.operator {
        "==" => literal,
        "!=" => !literal,
        _ => return None,
    };
    Some((atom, atom_truth))
}

pub(super) fn boolean_atom_truth(atom: &str) -> Option<(&str, bool)> {
    let atom = strip_balanced_outer_parens(atom);
    if atom.is_empty()
        || atom == "true"
        || atom == "false"
        || split_top_level_keyword(atom, "and").len() > 1
        || split_top_level_keyword(atom, "or").len() > 1
        || ParsedRepairComparison::parse(atom).is_some()
    {
        return None;
    }
    if let Some(negated) = stripped_not_operand(atom) {
        let negated = strip_balanced_outer_parens(negated);
        if negated.is_empty()
            || split_top_level_keyword(negated, "and").len() > 1
            || split_top_level_keyword(negated, "or").len() > 1
            || ParsedRepairComparison::parse(negated).is_some()
        {
            return None;
        }
        return Some((negated, false));
    }
    Some((atom, true))
}

pub(super) fn equality_with_distinct_literal_implies_disequality(
    required_left: &str,
    required_operator: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_operator: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    if required_operator != "==" || wanted_operator != "!=" {
        return false;
    }
    equality_side_excludes_wanted_literal(
        required_left,
        required_right,
        wanted_left,
        wanted_right,
        equivalences,
    ) || equality_side_excludes_wanted_literal(
        required_right,
        required_left,
        wanted_left,
        wanted_right,
        equivalences,
    )
}

pub(super) fn equality_side_excludes_wanted_literal(
    required_subject: &str,
    required_value: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    if repair_operands_equivalent(required_subject, wanted_left, equivalences) {
        return repair_literals_are_distinct(required_value, wanted_right);
    }
    if repair_operands_equivalent(required_subject, wanted_right, equivalences) {
        return repair_literals_are_distinct(required_value, wanted_left);
    }
    false
}

pub(super) fn repair_literals_are_distinct(left: &str, right: &str) -> bool {
    if let (Some(left), Some(right)) = (repair_numeric_literal(left), repair_numeric_literal(right))
    {
        return left != right;
    }
    let Some(left) = RepairLiteral::parse(left.trim()) else {
        return false;
    };
    let Some(right) = RepairLiteral::parse(right.trim()) else {
        return false;
    };
    left != right
}

pub(super) fn repair_numeric_literal(text: &str) -> Option<RepairNumber> {
    if let Some(value) = repair_numeric_expression(text) {
        return Some(value);
    }
    match RepairLiteral::parse(text.trim())? {
        RepairLiteral::Number(value) => Some(value),
        RepairLiteral::Bool(_) | RepairLiteral::String(_) => None,
    }
}

pub(super) fn repair_numeric_order_literal(text: &str) -> Option<RepairRational> {
    if let Some(value) = repair_numeric_rational_expression(text) {
        return Some(value);
    }
    repair_numeric_literal(text).and_then(RepairRational::from_number)
}

pub(super) fn repair_numeric_expression(predicate: &str) -> Option<RepairNumber> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = parse_repair_number_literal(predicate) {
        return Some(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_expression(left)?;
            let right = repair_numeric_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_expression(left)?;
            let right = repair_numeric_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return repair_numeric_expression(rest)?.negate();
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct RepairRational {
    numerator: i128,
    denominator: i128,
}

impl Ord for RepairRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("repair rational comparison overflow")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("repair rational comparison overflow"),
            )
    }
}

impl PartialOrd for RepairRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl RepairRational {
    pub(super) fn from_number(number: RepairNumber) -> Option<Self> {
        Self::from_raw(number.mantissa, 10_i128.checked_pow(number.scale)?)
    }

    pub(super) fn from_raw(mut numerator: i128, mut denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }
        let divisor = repair_gcd_i128(numerator, denominator)?;
        Some(Self {
            numerator: numerator.checked_div(divisor)?,
            denominator: denominator.checked_div(divisor)?,
        })
    }

    pub(super) fn negate(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    pub(super) fn add(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_add(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(super) fn add_int(self, integer: i128) -> Option<Self> {
        self.add(Self::from_raw(integer, 1)?)
    }

    pub(super) fn is_integer(&self) -> bool {
        self.denominator == 1
    }

    pub(super) fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    pub(super) fn mul(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(super) fn div(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

pub(super) fn repair_numeric_rational_expression(predicate: &str) -> Option<RepairRational> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = parse_repair_number_literal(predicate) {
        return RepairRational::from_number(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_rational_expression(left)?;
            let right = repair_numeric_rational_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_repair_numeric_operator(predicate, operator) {
            let left = repair_numeric_rational_expression(left)?;
            let right = repair_numeric_rational_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return repair_numeric_rational_expression(rest)?.negate();
    }
    None
}

pub(super) fn split_repair_numeric_operator<'a>(
    predicate: &'a str,
    operator: &str,
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
            _ if depth == 0 && predicate[index..].starts_with(operator) => {
                let left = predicate[..index].trim();
                let right = predicate[index + operator.len()..].trim();
                if !left.is_empty() && !right.is_empty() && operator_is_binary(left, operator) {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn operator_is_binary(left: &str, operator: &str) -> bool {
    operator != "-"
        || left
            .chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == ')' || ch == '"')
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct RepairNumber {
    mantissa: i128,
    scale: u32,
}

impl Ord for RepairNumber {
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

impl PartialOrd for RepairNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl RepairNumber {
    pub(super) fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
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

    pub(super) fn abs_parts(&self) -> (String, String) {
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

    pub(super) fn from_raw(mut mantissa: i128, mut scale: u32) -> Self {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self { mantissa, scale }
    }

    pub(super) fn negate(self) -> Option<Self> {
        Some(Self {
            mantissa: self.mantissa.checked_neg()?,
            scale: self.scale,
        })
    }

    pub(super) fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.scaled_mantissa(scale)?;
        let right = other.scaled_mantissa(scale)?;
        Some(Self::from_raw(left.checked_add(right)?, scale))
    }

    pub(super) fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    pub(super) fn mul(self, other: Self) -> Option<Self> {
        Some(Self::from_raw(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    pub(super) fn div(self, other: Self) -> Option<Self> {
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

        let divisor = repair_gcd_i128(numerator, denominator)?;
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
            .checked_div(repair_divisor_scale(twos, fives)?)?;
        Some(Self::from_raw(mantissa, scale))
    }

    pub(super) fn scaled_mantissa(self, scale: u32) -> Option<i128> {
        let extra_scale = scale.checked_sub(self.scale)?;
        self.mantissa.checked_mul(10_i128.checked_pow(extra_scale)?)
    }
}

pub(super) fn repair_divisor_scale(twos: u32, fives: u32) -> Option<i128> {
    let twos = 2_i128.checked_pow(twos)?;
    let fives = 5_i128.checked_pow(fives)?;
    twos.checked_mul(fives)
}

pub(super) fn repair_gcd_i128(left: i128, right: i128) -> Option<i128> {
    let mut left = left.checked_abs()?;
    let mut right = right.checked_abs()?;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    (left != 0).then_some(left)
}

#[derive(PartialEq, Eq)]
pub(super) enum RepairLiteral {
    Bool(bool),
    Number(RepairNumber),
    String(String),
}

impl RepairLiteral {
    pub(super) fn parse(text: &str) -> Option<Self> {
        match text {
            "true" => return Some(Self::Bool(true)),
            "false" => return Some(Self::Bool(false)),
            _ => {}
        }
        if let Some(number) = parse_repair_number_literal(text) {
            return Some(Self::Number(number));
        }
        parse_repair_string_literal(text).map(Self::String)
    }
}

pub(super) fn parse_repair_number_literal(text: &str) -> Option<RepairNumber> {
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
    Some(RepairNumber { mantissa, scale })
}

pub(super) fn parse_repair_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let escaped = chars.next()?;
            value.push(escaped);
        } else if ch == '"' {
            return None;
        } else {
            value.push(ch);
        }
    }
    Some(value)
}

pub(super) fn ordering_path_implies_clause(
    required_clauses: &[String],
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    match wanted.operator {
        "==" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                false,
                equivalences,
            ) && ordering_path_exists(
                required_clauses,
                wanted.right,
                wanted.left,
                false,
                equivalences,
            )
        }
        "<" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                true,
                equivalences,
            ) || (ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                false,
                equivalences,
            ) && (disequality_clause_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                equivalences,
            ) || ordering_path_contains_disequality(
                required_clauses,
                wanted.left,
                wanted.right,
                equivalences,
            )))
        }
        "<=" => ordering_path_exists(
            required_clauses,
            wanted.left,
            wanted.right,
            false,
            equivalences,
        ),
        "!=" => {
            ordering_path_exists(
                required_clauses,
                wanted.left,
                wanted.right,
                true,
                equivalences,
            ) || ordering_path_exists(
                required_clauses,
                wanted.right,
                wanted.left,
                true,
                equivalences,
            )
        }
        _ => false,
    }
}

pub(super) fn ordering_path_contains_disequality(
    required_clauses: &[String],
    from: &str,
    to: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    required_clauses.iter().any(|clause| {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            return false;
        };
        if parsed.operator != "!=" {
            return false;
        }
        disequality_lies_on_ordering_path(
            required_clauses,
            from,
            to,
            parsed.left,
            parsed.right,
            equivalences,
        ) || disequality_lies_on_ordering_path(
            required_clauses,
            from,
            to,
            parsed.right,
            parsed.left,
            equivalences,
        )
    })
}

pub(super) fn disequality_lies_on_ordering_path(
    required_clauses: &[String],
    from: &str,
    to: &str,
    disequal_left: &str,
    disequal_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    ordering_path_exists(required_clauses, from, disequal_left, false, equivalences)
        && ordering_path_exists(
            required_clauses,
            disequal_left,
            disequal_right,
            false,
            equivalences,
        )
        && ordering_path_exists(required_clauses, disequal_right, to, false, equivalences)
}

pub(super) fn disequality_clause_exists(
    required_clauses: &[String],
    left: &str,
    right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    required_clauses.iter().any(|clause| {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            return false;
        };
        parsed.operator == "!="
            && repair_operands_equivalent_unordered(
                parsed.left,
                parsed.right,
                left,
                right,
                equivalences,
            )
    })
}

pub(super) fn ordering_path_exists(
    required_clauses: &[String],
    from: &str,
    to: &str,
    needs_strict: bool,
    equivalences: &RepairEquivalences,
) -> bool {
    let edges = required_clauses
        .iter()
        .filter_map(|clause| {
            let parsed = ParsedRepairComparison::parse(clause)?;
            matches!(parsed.operator, "<" | "<=").then_some((
                parsed.left,
                parsed.right,
                parsed.operator == "<",
            ))
        })
        .collect::<Vec<_>>();
    let mut pending = vec![(from, false)];
    let mut visited = Vec::<(String, bool)>::new();
    while let Some((current, has_strict)) = pending.pop() {
        if repair_operands_equivalent(current, to, equivalences) && (!needs_strict || has_strict) {
            return true;
        }
        if visited
            .iter()
            .any(|(operand, strict)| operand == current && *strict == has_strict)
        {
            continue;
        }
        visited.push((current.to_string(), has_strict));
        for (left, right, edge_strict) in &edges {
            if repair_operands_equivalent(current, left, equivalences) {
                pending.push((right, has_strict || *edge_strict));
            }
        }
    }
    false
}

pub(super) fn repair_equivalences(clauses: &[String]) -> RepairEquivalences {
    let mut equivalences = RepairEquivalences::default();
    for clause in clauses {
        let Some(parsed) = ParsedRepairComparison::parse(clause) else {
            continue;
        };
        if parsed.operator == "==" {
            equivalences.union(parsed.left, parsed.right);
        }
    }
    equivalences
}

#[derive(Default)]
pub(super) struct RepairEquivalences {
    groups: Vec<Vec<String>>,
}

impl RepairEquivalences {
    pub(super) fn union(&mut self, left: &str, right: &str) {
        let left = normalized_repair_operand_text(left);
        let right = normalized_repair_operand_text(right);
        if left == right {
            return;
        }
        let left_index = self.group_index(&left);
        let right_index = self.group_index(&right);
        match (left_index, right_index) {
            (Some(left_index), Some(right_index)) if left_index != right_index => {
                let right_group = self.groups.remove(right_index);
                let destination = if right_index < left_index {
                    left_index - 1
                } else {
                    left_index
                };
                self.groups[destination].extend(right_group);
            }
            (Some(index), None) => self.groups[index].push(right),
            (None, Some(index)) => self.groups[index].push(left),
            (None, None) => self.groups.push(vec![left, right]),
            _ => {}
        }
    }

    pub(super) fn equivalent(&self, left: &str, right: &str) -> bool {
        let left = normalized_repair_operand_text(left);
        let right = normalized_repair_operand_text(right);
        left == right
            || self.groups.iter().any(|group| {
                group.iter().any(|item| item == &left) && group.iter().any(|item| item == &right)
            })
    }

    pub(super) fn canonical_expression(&self, expression: &str) -> String {
        let mut output = String::with_capacity(expression.len());
        let mut chars = expression.char_indices().peekable();
        while let Some((start, ch)) = chars.next() {
            if ch == '"' {
                output.push(ch);
                let mut escaped = false;
                for (_, string_ch) in chars.by_ref() {
                    output.push(string_ch);
                    if escaped {
                        escaped = false;
                    } else if string_ch == '\\' {
                        escaped = true;
                    } else if string_ch == '"' {
                        break;
                    }
                }
            } else if is_ident_start(ch) {
                let mut end = start + ch.len_utf8();
                while let Some((next, next_ch)) = chars.peek().copied() {
                    if !is_ident_continue(next_ch) {
                        break;
                    }
                    chars.next();
                    end = next + next_ch.len_utf8();
                }
                let ident = &expression[start..end];
                if is_value_identifier_position(expression, start, end) {
                    output.push_str(self.canonical_operand(ident));
                } else {
                    output.push_str(ident);
                }
            } else if !ch.is_whitespace() {
                output.push(ch);
            }
        }
        output
    }

    pub(super) fn canonical_operand<'a>(&'a self, operand: &'a str) -> &'a str {
        self.groups
            .iter()
            .find(|group| group.iter().any(|item| item == operand))
            .and_then(|group| group.iter().min().map(String::as_str))
            .unwrap_or(operand)
    }

    pub(super) fn group_index(&self, operand: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.iter().any(|item| item == operand))
    }
}

pub(super) fn normalized_repair_operand_text(operand: &str) -> String {
    compact_direct_repair_expression_text(strip_balanced_outer_parens(operand))
}

pub(super) fn same_repair_operands_unordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
) -> bool {
    (required_left == wanted_left && required_right == wanted_right)
        || (required_left == wanted_right && required_right == wanted_left)
}

pub(super) fn repair_operands_equivalent_ordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required_left, wanted_left, equivalences)
        && repair_operands_equivalent(required_right, wanted_right, equivalences)
}

pub(super) fn repair_operands_equivalent_unordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent_ordered(
        required_left,
        required_right,
        wanted_left,
        wanted_right,
        equivalences,
    ) || repair_operands_equivalent_ordered(
        required_left,
        required_right,
        wanted_right,
        wanted_left,
        equivalences,
    )
}

pub(super) fn repair_operands_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    equivalences.equivalent(required, wanted)
        || compact_predicate_text(required) == compact_predicate_text(wanted)
        || equivalences.canonical_expression(required) == equivalences.canonical_expression(wanted)
}

pub(super) fn repair_atoms_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    ParsedRepairComparison::parse(required).is_none()
        && (compact_predicate_text(required) == compact_predicate_text(wanted)
            || equivalences.canonical_expression(required)
                == equivalences.canonical_expression(wanted))
}

pub(super) struct ParsedRepairComparison<'a> {
    clause: &'a str,
    left: &'a str,
    operator: &'static str,
    right: &'a str,
}

impl<'a> ParsedRepairComparison<'a> {
    pub(super) fn parse(clause: &'a str) -> Option<Self> {
        for operator in ["==", "!=", "<=", ">=", "<", ">"] {
            let Some((left, right)) = clause.split_once(operator) else {
                continue;
            };
            let left = left.trim();
            let right = right.trim();
            if left.is_empty() || right.is_empty() {
                return None;
            }
            return Some(Self {
                clause,
                left,
                operator,
                right,
            });
        }
        None
    }
}

pub(super) struct NormalizedRepairComparison<'a> {
    left: &'a str,
    operator: &'static str,
    right: &'a str,
}

impl<'a> NormalizedRepairComparison<'a> {
    pub(super) fn parse(clause: &'a str) -> Option<Self> {
        let parsed = ParsedRepairComparison::parse(strip_balanced_outer_parens(clause))?;
        Some(match parsed.operator {
            ">" => Self {
                left: parsed.right,
                operator: "<",
                right: parsed.left,
            },
            ">=" => Self {
                left: parsed.right,
                operator: "<=",
                right: parsed.left,
            },
            _ => Self {
                left: parsed.left,
                operator: parsed.operator,
                right: parsed.right,
            },
        })
    }

    pub(super) fn same_operands_unordered(&self, other: &Self) -> bool {
        (compact_predicate_text(self.left) == compact_predicate_text(other.left)
            && compact_predicate_text(self.right) == compact_predicate_text(other.right))
            || self.same_operands_reversed(other)
    }

    pub(super) fn same_operands_reversed(&self, other: &Self) -> bool {
        compact_predicate_text(self.left) == compact_predicate_text(other.right)
            && compact_predicate_text(self.right) == compact_predicate_text(other.left)
    }
}

pub(super) fn split_top_level_keyword<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
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
                if is_word_boundary(predicate, cursor, keyword_end) {
                    clauses.push(&predicate[start..cursor]);
                    start = keyword_end;
                    cursor = keyword_end;
                    continue;
                }
            }
            _ => {}
        }
        cursor = end;
    }

    clauses.push(&predicate[start..]);
    clauses
}

pub(super) fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_ident_continue(ch)) && after.is_none_or(|ch| !is_ident_continue(ch))
}

pub(super) fn normalized_predicate_clause(predicate: &str) -> String {
    let predicate = strip_balanced_outer_parens(predicate);
    if let Some(negated) = stripped_not_operand(predicate) {
        return match normalized_predicate_clause(negated).as_str() {
            "true" => "false".to_string(),
            "false" => "true".to_string(),
            _ => predicate.split_whitespace().collect::<Vec<_>>().join(" "),
        };
    }
    predicate.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn stripped_not_operand(predicate: &str) -> Option<&str> {
    if let Some(negated) = predicate.strip_prefix("not ") {
        return Some(negated);
    }
    predicate
        .strip_prefix("not(")
        .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())
}

pub(super) fn strip_balanced_outer_parens(mut predicate: &str) -> &str {
    loop {
        let trimmed = predicate.trim();
        if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
            return trimmed;
        }
        let mut depth = 0;
        let mut wraps_whole_clause = true;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && index + ch.len_utf8() != trimmed.len() {
                        wraps_whole_clause = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if wraps_whole_clause && depth == 0 {
            predicate = &trimmed[1..trimmed.len() - 1];
        } else {
            return trimmed;
        }
    }
}

pub(super) fn canonical_repair_clause(clause: impl AsRef<str>) -> String {
    let clause = strip_balanced_outer_parens(clause.as_ref());
    if let Some(negated) = canonical_negated_repair_clause(clause) {
        return negated;
    }
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        let Some((left, right)) = clause.split_once(operator) else {
            continue;
        };
        let left = left.trim();
        let right = right.trim();
        if left.is_empty() || right.is_empty() {
            return clause.to_string();
        }
        return match operator {
            "==" | "!=" if right < left => format!("{right} {operator} {left}"),
            ">" => format!("{right} < {left}"),
            ">=" => format!("{right} <= {left}"),
            _ => format!("{left} {operator} {right}"),
        };
    }
    clause.to_string()
}

pub(super) fn canonical_negated_repair_clause(clause: &str) -> Option<String> {
    let trimmed = clause.trim();
    let negated = if let Some(negated) = trimmed.strip_prefix("not ") {
        negated
    } else {
        trimmed
            .strip_prefix("not(")
            .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())?
    };
    let negated = strip_balanced_outer_parens(negated);
    if let Some(double_negated) = stripped_not_operand(negated) {
        return Some(canonical_repair_clause(double_negated));
    }
    match normalized_predicate_clause(negated).as_str() {
        "true" => return Some("false".to_string()),
        "false" => return Some("true".to_string()),
        _ => {}
    }
    for (operator, inverse) in [
        ("==", "!="),
        ("!=", "=="),
        ("<=", ">"),
        ("<", ">="),
        (">=", "<"),
        (">", "<="),
    ] {
        let Some((left, right)) = negated.split_once(operator) else {
            continue;
        };
        let left = left.trim();
        let right = right.trim();
        if left.is_empty() || right.is_empty() {
            return None;
        }
        return Some(canonical_repair_clause(format!("{left} {inverse} {right}")));
    }
    None
}

pub(super) fn canonical_negated_repair_or_atom_clause(clause: &str) -> Option<String> {
    canonical_negated_repair_clause(clause).or_else(|| {
        let negated = stripped_not_operand(clause.trim())?;
        let negated = strip_balanced_outer_parens(negated);
        if negated.is_empty()
            || split_top_level_keyword(negated, "and").len() > 1
            || split_top_level_keyword(negated, "or").len() > 1
            || ParsedRepairComparison::parse(negated).is_some()
        {
            return None;
        }
        Some(format!("not {negated}"))
    })
}

pub(super) fn replace_identifier(predicate: &str, target: &str, replacement: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if ch == '"' {
            output.push(ch);
            let mut escaped = false;
            for (_, string_ch) in chars.by_ref() {
                output.push(string_ch);
                if escaped {
                    escaped = false;
                } else if string_ch == '\\' {
                    escaped = true;
                } else if string_ch == '"' {
                    break;
                }
            }
        } else if is_ident_start(ch) {
            let mut end = start + ch.len_utf8();
            while let Some((next, next_ch)) = chars.peek().copied() {
                if !is_ident_continue(next_ch) {
                    break;
                }
                chars.next();
                end = next + next_ch.len_utf8();
            }
            let ident = &predicate[start..end];
            if ident == target && is_value_identifier_position(predicate, start, end) {
                output.push_str(replacement);
            } else {
                output.push_str(ident);
            }
        } else {
            output.push(ch);
        }
    }
    output
}

pub(super) fn is_value_identifier_position(predicate: &str, start: usize, end: usize) -> bool {
    !predicate[..start].ends_with('.')
        && !predicate[..start].ends_with("::")
        && !predicate[end..].starts_with("::")
}

pub(super) fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

pub(super) fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub(super) fn callee_name_path_and_type_args(
    callee: &Expr,
) -> Option<(&[String], Option<&[String]>)> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some((segments, None)),
        ExprKind::TypeApply { callee, type_args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return None;
            };
            Some((segments, Some(type_args.as_slice())))
        }
        _ => None,
    }
}

pub(super) fn function_returns_result(ty: &Type) -> Option<(&Type, &Type)> {
    let (_, return_type) = ty.function_parts()?;
    adt::result_parts(return_type)
}

pub(super) fn is_ordering_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}

pub(super) fn contract_call_result_is_compared(predicate: &str, start: usize, end: usize) -> bool {
    let before = predicate[..start].trim_end();
    let after = predicate[end..].trim_start();
    before.ends_with("==")
        || before.ends_with("!=")
        || before.ends_with("<=")
        || before.ends_with(">=")
        || before.ends_with('<')
        || before.ends_with('>')
        || after.starts_with("==")
        || after.starts_with("!=")
        || after.starts_with("<=")
        || after.starts_with(">=")
        || after.starts_with('<')
        || after.starts_with('>')
}

pub(super) fn contract_call_result_feeds_boolean_predicate(
    predicate: &str,
    start: usize,
    end: usize,
) -> bool {
    let Some(call_depth) = paren_depth_before(predicate, start) else {
        return false;
    };
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in predicate.char_indices() {
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
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if (index < start || index >= end)
                && depth <= call_depth
                && predicate[index..].starts_with_comparison_operator() =>
            {
                return true;
            }
            _ => {}
        }
    }
    false
}

pub(super) fn contract_call_result_has_field_access(predicate: &str, end: usize) -> bool {
    predicate[end..].trim_start().starts_with('.')
}

pub(super) fn paren_depth_before(text: &str, offset: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in text.char_indices() {
        if index >= offset {
            return Some(depth);
        }
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
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Some(depth)
}

pub(super) trait StartsWithComparisonOperator {
    fn starts_with_comparison_operator(&self) -> bool;
}

impl StartsWithComparisonOperator for str {
    fn starts_with_comparison_operator(&self) -> bool {
        self.starts_with("==")
            || self.starts_with("!=")
            || self.starts_with("<=")
            || self.starts_with(">=")
            || self.starts_with('<')
            || self.starts_with('>')
    }
}

pub(super) fn contract_call_is_argument(calls: &[ContractCall], call_index: usize) -> bool {
    let call = &calls[call_index];
    calls.iter().enumerate().any(|(index, outer)| {
        index != call_index && outer.start < call.start && call.end < outer.end
    })
}
