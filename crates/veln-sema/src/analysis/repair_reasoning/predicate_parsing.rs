use super::*;

pub(in crate::analysis) struct ParsedRepairComparison<'a> {
    pub(in crate::analysis) clause: &'a str,
    pub(in crate::analysis) left: &'a str,
    pub(in crate::analysis) operator: &'static str,
    pub(in crate::analysis) right: &'a str,
}

impl<'a> ParsedRepairComparison<'a> {
    pub(in crate::analysis) fn parse(clause: &'a str) -> Option<Self> {
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

pub(in crate::analysis) struct NormalizedRepairComparison<'a> {
    pub(in crate::analysis) left: &'a str,
    pub(in crate::analysis) operator: &'static str,
    pub(in crate::analysis) right: &'a str,
}

impl<'a> NormalizedRepairComparison<'a> {
    pub(in crate::analysis) fn parse(clause: &'a str) -> Option<Self> {
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

    pub(in crate::analysis) fn same_operands_unordered(&self, other: &Self) -> bool {
        (compact_predicate_text(self.left) == compact_predicate_text(other.left)
            && compact_predicate_text(self.right) == compact_predicate_text(other.right))
            || self.same_operands_reversed(other)
    }

    pub(in crate::analysis) fn same_operands_reversed(&self, other: &Self) -> bool {
        compact_predicate_text(self.left) == compact_predicate_text(other.right)
            && compact_predicate_text(self.right) == compact_predicate_text(other.left)
    }
}

pub(in crate::analysis) fn split_top_level_keyword<'a>(
    predicate: &'a str,
    keyword: &str,
) -> Vec<&'a str> {
    crate::predicate_text::split_top_level_keyword_raw(predicate, keyword)
}

pub(in crate::analysis) fn normalized_predicate_clause(predicate: &str) -> String {
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

pub(in crate::analysis) fn stripped_not_operand(predicate: &str) -> Option<&str> {
    if let Some(negated) = predicate.strip_prefix("not ") {
        return Some(negated);
    }
    predicate
        .strip_prefix("not(")
        .map(|negated| negated.strip_suffix(')').unwrap_or(negated).trim())
}

pub(in crate::analysis) fn strip_balanced_outer_parens(mut predicate: &str) -> &str {
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

pub(in crate::analysis) fn canonical_repair_clause(clause: impl AsRef<str>) -> String {
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

pub(in crate::analysis) fn canonical_negated_repair_clause(clause: &str) -> Option<String> {
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

pub(in crate::analysis) fn canonical_negated_repair_or_atom_clause(clause: &str) -> Option<String> {
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

pub(in crate::analysis) fn replace_identifier(
    predicate: &str,
    target: &str,
    replacement: &str,
) -> String {
    crate::predicate_text::rewrite_identifiers(predicate, true, |identifier, is_value, output| {
        if identifier == target && is_value {
            output.push_str(replacement);
        } else {
            output.push_str(identifier);
        }
    })
}

pub(in crate::analysis) fn is_value_identifier_position(
    predicate: &str,
    start: usize,
    end: usize,
) -> bool {
    crate::predicate_text::is_value_identifier_position(predicate, start, end)
}

pub(in crate::analysis) fn is_ident_start(ch: char) -> bool {
    crate::predicate_text::is_ident_start(ch)
}

pub(in crate::analysis) fn is_ident_continue(ch: char) -> bool {
    crate::predicate_text::is_ident_continue(ch)
}

pub(in crate::analysis) fn function_returns_result(ty: &Type) -> Option<(&Type, &Type)> {
    let (_, return_type) = ty.function_parts()?;
    adt::result_parts(return_type)
}

pub(in crate::analysis) fn is_ordering_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}

pub(in crate::analysis) fn contract_call_result_is_compared(
    predicate: &str,
    start: usize,
    end: usize,
) -> bool {
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

pub(in crate::analysis) fn contract_call_result_feeds_boolean_predicate(
    predicate: &str,
    start: usize,
    end: usize,
) -> bool {
    let Some(call_depth) = paren_depth_before(predicate, start) else {
        return false;
    };
    let mut found = false;
    crate::predicate_text::visit_unquoted_characters(predicate, |index, _, depth| {
        found = (index < start || index >= end)
            && depth <= call_depth
            && predicate[index..].starts_with_comparison_operator();
        found
    });
    found
}

pub(in crate::analysis) fn contract_call_result_has_field_access(
    predicate: &str,
    end: usize,
) -> bool {
    predicate[end..].trim_start().starts_with('.')
}

pub(in crate::analysis) fn paren_depth_before(text: &str, offset: usize) -> Option<usize> {
    let mut depth_at_offset = None;
    let final_depth = crate::predicate_text::visit_unquoted_characters(text, |index, _, depth| {
        if index >= offset {
            depth_at_offset = Some(depth);
            true
        } else {
            false
        }
    });
    Some(depth_at_offset.unwrap_or(final_depth))
}

pub(in crate::analysis) trait StartsWithComparisonOperator {
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

pub(in crate::analysis) fn contract_call_is_argument(
    calls: &[ContractCall],
    call_index: usize,
) -> bool {
    let call = &calls[call_index];
    calls.iter().enumerate().any(|(index, outer)| {
        index != call_index && outer.start < call.start && call.end < outer.end
    })
}
