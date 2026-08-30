use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::analysis) enum TotalOrderRelation {
    Less,
    Equal,
    Greater,
}

impl TotalOrderRelation {
    pub(in crate::analysis) const ALL_BITS: u8 =
        Self::Less.bit() | Self::Equal.bit() | Self::Greater.bit();

    pub(in crate::analysis) const fn bit(self) -> u8 {
        match self {
            Self::Less => 0b001,
            Self::Equal => 0b010,
            Self::Greater => 0b100,
        }
    }

    pub(in crate::analysis) fn invert(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::Equal => Self::Equal,
            Self::Greater => Self::Less,
        }
    }
}

pub(in crate::analysis) struct TotalOrderCandidateClause {
    pub(in crate::analysis) left: String,
    pub(in crate::analysis) right: String,
    pub(in crate::analysis) relation: TotalOrderRelation,
}

pub(in crate::analysis) fn total_order_candidate_clause(
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

pub(in crate::analysis) fn complementary_disjunct_key(
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

pub(in crate::analysis) fn has_true_disjunct(predicate: &str) -> bool {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "or")
        .into_iter()
        .any(|clause| normalized_predicate_clause(clause) == "true")
}

pub(in crate::analysis) fn is_candidate_tautology_disjunct(
    predicate: &str,
    candidate: &str,
) -> bool {
    let clauses = repair_relevant_and_clauses(predicate);
    !clauses.is_empty()
        && clauses.iter().all(|clause| {
            is_surplus_tautology_clause(clause, candidate)
                || is_candidate_tautology_clause(clause, candidate)
        })
}

pub(in crate::analysis) fn is_surplus_tautology_clause(clause: &str, candidate: &str) -> bool {
    has_true_disjunct(clause)
        || predicate_is_statically_true(clause)
        || has_complementary_candidate_disjuncts(&repair_relevant_or_clauses(clause), candidate)
}

pub(in crate::analysis) fn is_candidate_tautology_clause(predicate: &str, candidate: &str) -> bool {
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

pub(in crate::analysis) fn tautological_candidate_expression(
    left: &str,
    right: &str,
    candidate: &str,
) -> bool {
    compact_direct_repair_expression_text(left) == compact_direct_repair_expression_text(right)
        && expression_references_identifier(left, candidate)
}

pub(in crate::analysis) fn direct_reflexive_clause(
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

pub(in crate::analysis) fn reflexive_operand(
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

pub(in crate::analysis) fn is_plain_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(in crate::analysis) fn operand_path(value: &str) -> Option<Vec<&str>> {
    value
        .trim()
        .split('.')
        .map(str::trim)
        .map(|segment| is_plain_identifier(segment).then_some(segment))
        .collect()
}

pub(in crate::analysis) fn reflexive_path_binding(
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

pub(in crate::analysis) fn reflexive_expression_operand(
    predicate: &str,
    candidate: &str,
    operator: &str,
) -> Option<String> {
    let (left, right) = predicate.split_once(operator)?;
    reflexive_expression_binding(left, right, candidate)
        .or_else(|| reflexive_expression_binding(right, left, candidate))
}

pub(in crate::analysis) fn reflexive_expression_binding(
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

pub(in crate::analysis) fn expression_references_identifier(expression: &str, name: &str) -> bool {
    expression_identifiers(expression)
        .into_iter()
        .any(|identifier| identifier == name)
}

pub(in crate::analysis) fn expression_identifiers(expression: &str) -> Vec<&str> {
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

pub(in crate::analysis) fn compact_predicate_text(predicate: &str) -> String {
    crate::predicate_text::compact_predicate_text(predicate)
}

pub(in crate::analysis) fn compact_direct_repair_expression_text(predicate: &str) -> String {
    let mut current = compact_predicate_text(predicate);
    loop {
        let stripped = strip_redundant_repair_atom_parens(&current);
        if stripped == current {
            return current;
        }
        current = stripped;
    }
}

pub(in crate::analysis) fn strip_redundant_repair_atom_parens(predicate: &str) -> String {
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

pub(in crate::analysis) fn is_repair_atom_text(text: &str) -> bool {
    operand_path(text).is_some() || repair_numeric_order_literal(text).is_some()
}

pub(in crate::analysis) fn normalized_and_clauses(predicate: &str) -> Vec<String> {
    split_top_level_keyword(strip_balanced_outer_parens(predicate), "and")
        .into_iter()
        .map(normalized_predicate_clause)
        .filter(|clause| !clause.is_empty())
        .collect()
}

pub(in crate::analysis) fn repair_relevant_and_clauses(predicate: &str) -> Vec<String> {
    normalized_and_clauses(predicate)
        .into_iter()
        .flat_map(|clause| {
            canonical_negated_disjunction_repair_clauses(&clause).unwrap_or_else(|| vec![clause])
        })
        .filter(|clause| clause != "true" && !contract_predicate_is_statically_true(clause))
        .collect()
}

pub(in crate::analysis) fn repair_relevant_or_clauses(predicate: &str) -> Vec<&str> {
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

pub(in crate::analysis) fn single_repair_relevant_clause(predicate: &str) -> Option<&str> {
    let clauses = repair_relevant_or_clauses(predicate);
    match clauses.as_slice() {
        [clause] => Some(*clause),
        _ => None,
    }
}

pub(in crate::analysis) fn repair_relevant_negated_and_clauses(
    predicate: &str,
) -> Option<Vec<String>> {
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

pub(in crate::analysis) fn negated_and_clauses(predicate: &str) -> Option<Vec<&str>> {
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
