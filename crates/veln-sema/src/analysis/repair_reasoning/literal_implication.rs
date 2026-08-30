use super::*;

pub(in crate::analysis) fn inclusive_bounds_imply_equality(
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

pub(in crate::analysis) fn repair_clause_implies_with_equivalences(
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

pub(in crate::analysis) fn literal_order_comparison_implies(
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

pub(in crate::analysis) fn literal_lower_bound_implies(
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

pub(in crate::analysis) fn literal_upper_bound_implies(
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

pub(in crate::analysis) fn literal_equality_implies_order_comparison(
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

pub(in crate::analysis) fn literal_equality_implies_lower_bound(
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

pub(in crate::analysis) fn literal_equality_implies_upper_bound(
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

pub(in crate::analysis) fn literal_equality_subject<'a>(
    required: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(required.left)
        .map(|literal| (required.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(required.right).map(|literal| (required.left, literal))
        })
}

pub(in crate::analysis) fn literal_order_strength_implies<T: Ord>(
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

pub(in crate::analysis) fn literal_bound_implies_disequality(
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

pub(in crate::analysis) fn literal_lower_bound_implies_disequality(
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

pub(in crate::analysis) fn literal_upper_bound_implies_disequality(
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

pub(in crate::analysis) fn repair_disequality_literal_for_operand(
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

pub(in crate::analysis) fn disequality_implies_numeric_ordering_disjunction(
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

pub(in crate::analysis) fn inclusive_bound_implies_order_or_equality_disjunction(
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

pub(in crate::analysis) fn numeric_literal_comparison_side<'a>(
    comparison: &'a ParsedRepairComparison<'a>,
) -> Option<(&'a str, RepairRational)> {
    repair_numeric_order_literal(comparison.left)
        .map(|literal| (comparison.right, literal))
        .or_else(|| {
            repair_numeric_order_literal(comparison.right).map(|literal| (comparison.left, literal))
        })
}

pub(in crate::analysis) fn boolean_literal_comparison_implies_atom(
    required: &str,
    wanted_atom: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    let Some(required) = ParsedRepairComparison::parse(required) else {
        return false;
    };
    boolean_truth_implies(
        boolean_literal_comparison_truth(&required),
        boolean_atom_truth(wanted_atom),
        equivalences,
    )
}

pub(in crate::analysis) fn boolean_atom_implies_literal_comparison(
    required_atom: &str,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    boolean_truth_implies(
        boolean_atom_truth(required_atom),
        boolean_literal_comparison_truth(wanted),
        equivalences,
    )
}

pub(in crate::analysis) fn boolean_literal_comparison_implies_comparison(
    required: &ParsedRepairComparison<'_>,
    wanted: &ParsedRepairComparison<'_>,
    equivalences: &RepairEquivalences,
) -> bool {
    boolean_truth_implies(
        boolean_literal_comparison_truth(required),
        boolean_literal_comparison_truth(wanted),
        equivalences,
    )
}

fn boolean_truth_implies(
    required: Option<(&str, bool)>,
    wanted: Option<(&str, bool)>,
    equivalences: &RepairEquivalences,
) -> bool {
    let (Some((required_atom, required_truth)), Some((wanted_atom, wanted_truth))) =
        (required, wanted)
    else {
        return false;
    };
    required_truth == wanted_truth
        && repair_operands_equivalent(required_atom, wanted_atom, equivalences)
}

pub(in crate::analysis) fn boolean_disequality_alias_implies_comparison(
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

pub(in crate::analysis) fn boolean_literal_value_for_operand(
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

pub(in crate::analysis) fn boolean_literal_comparison_truth<'a>(
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

pub(in crate::analysis) fn boolean_atom_truth(atom: &str) -> Option<(&str, bool)> {
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

pub(in crate::analysis) fn equality_with_distinct_literal_implies_disequality(
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

pub(in crate::analysis) fn equality_side_excludes_wanted_literal(
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

pub(in crate::analysis) fn repair_literals_are_distinct(left: &str, right: &str) -> bool {
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

pub(in crate::analysis) fn repair_numeric_literal(text: &str) -> Option<RepairNumber> {
    if let Some(value) = repair_numeric_expression(text) {
        return Some(value);
    }
    match RepairLiteral::parse(text.trim())? {
        RepairLiteral::Number(value) => Some(value),
        RepairLiteral::Bool(_) | RepairLiteral::String(_) => None,
    }
}

pub(in crate::analysis) fn repair_numeric_order_literal(text: &str) -> Option<RepairRational> {
    if let Some(value) = repair_numeric_rational_expression(text) {
        return Some(value);
    }
    repair_numeric_literal(text).and_then(RepairRational::from_number)
}

pub(in crate::analysis) fn repair_numeric_expression(predicate: &str) -> Option<RepairNumber> {
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
