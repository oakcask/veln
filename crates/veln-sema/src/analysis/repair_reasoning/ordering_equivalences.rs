use super::*;

#[derive(PartialEq, Eq)]
pub(in crate::analysis) enum RepairLiteral {
    Bool(bool),
    Number(RepairNumber),
    String(String),
}

impl RepairLiteral {
    pub(in crate::analysis) fn parse(text: &str) -> Option<Self> {
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

pub(in crate::analysis) fn parse_repair_number_literal(text: &str) -> Option<RepairNumber> {
    RepairNumber::parse(text)
}

pub(in crate::analysis) fn parse_repair_string_literal(text: &str) -> Option<String> {
    crate::contracts::parse_quoted_string_literal(text)
}

pub(in crate::analysis) fn ordering_path_implies_clause(
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

pub(in crate::analysis) fn ordering_path_contains_disequality(
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

pub(in crate::analysis) fn disequality_lies_on_ordering_path(
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

pub(in crate::analysis) fn disequality_clause_exists(
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

pub(in crate::analysis) fn ordering_path_exists(
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

pub(in crate::analysis) fn repair_equivalences(clauses: &[String]) -> RepairEquivalences {
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
pub(in crate::analysis) struct RepairEquivalences {
    groups: Vec<Vec<String>>,
}

impl RepairEquivalences {
    pub(in crate::analysis) fn union(&mut self, left: &str, right: &str) {
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

    pub(in crate::analysis) fn equivalent(&self, left: &str, right: &str) -> bool {
        let left = normalized_repair_operand_text(left);
        let right = normalized_repair_operand_text(right);
        left == right
            || self.groups.iter().any(|group| {
                group.iter().any(|item| item == &left) && group.iter().any(|item| item == &right)
            })
    }

    pub(in crate::analysis) fn canonical_expression(&self, expression: &str) -> String {
        crate::predicate_text::rewrite_identifiers(
            expression,
            false,
            |identifier, is_value, output| {
                if is_value {
                    output.push_str(self.canonical_operand(identifier));
                } else {
                    output.push_str(identifier);
                }
            },
        )
    }

    pub(in crate::analysis) fn canonical_operand<'a>(&'a self, operand: &'a str) -> &'a str {
        self.groups
            .iter()
            .find(|group| group.iter().any(|item| item == operand))
            .and_then(|group| group.iter().min().map(String::as_str))
            .unwrap_or(operand)
    }

    pub(in crate::analysis) fn group_index(&self, operand: &str) -> Option<usize> {
        self.groups
            .iter()
            .position(|group| group.iter().any(|item| item == operand))
    }
}

pub(in crate::analysis) fn normalized_repair_operand_text(operand: &str) -> String {
    compact_direct_repair_expression_text(strip_balanced_outer_parens(operand))
}

pub(in crate::analysis) fn same_repair_operands_unordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
) -> bool {
    (required_left == wanted_left && required_right == wanted_right)
        || (required_left == wanted_right && required_right == wanted_left)
}

pub(in crate::analysis) fn repair_operands_equivalent_ordered(
    required_left: &str,
    required_right: &str,
    wanted_left: &str,
    wanted_right: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    repair_operands_equivalent(required_left, wanted_left, equivalences)
        && repair_operands_equivalent(required_right, wanted_right, equivalences)
}

pub(in crate::analysis) fn repair_operands_equivalent_unordered(
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

pub(in crate::analysis) fn repair_operands_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    equivalences.equivalent(required, wanted)
        || compact_predicate_text(required) == compact_predicate_text(wanted)
        || equivalences.canonical_expression(required) == equivalences.canonical_expression(wanted)
}

pub(in crate::analysis) fn repair_atoms_equivalent(
    required: &str,
    wanted: &str,
    equivalences: &RepairEquivalences,
) -> bool {
    ParsedRepairComparison::parse(required).is_none()
        && (compact_predicate_text(required) == compact_predicate_text(wanted)
            || equivalences.canonical_expression(required)
                == equivalences.canonical_expression(wanted))
}
