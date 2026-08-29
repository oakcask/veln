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

pub(in crate::analysis) fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_ident_continue(ch)) && after.is_none_or(|ch| !is_ident_continue(ch))
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

pub(in crate::analysis) fn is_value_identifier_position(
    predicate: &str,
    start: usize,
    end: usize,
) -> bool {
    !predicate[..start].ends_with('.')
        && !predicate[..start].ends_with("::")
        && !predicate[end..].starts_with("::")
}

pub(in crate::analysis) fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

pub(in crate::analysis) fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

pub(in crate::analysis) fn callee_name_path_and_type_args(
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

pub(in crate::analysis) fn contract_call_result_has_field_access(
    predicate: &str,
    end: usize,
) -> bool {
    predicate[end..].trim_start().starts_with('.')
}

pub(in crate::analysis) fn paren_depth_before(text: &str, offset: usize) -> Option<usize> {
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
