use super::*;
use veln_literals::parse_integer_literal;

pub(in crate::analysis) type RepairRational = crate::contracts::ExactRational;
pub(in crate::analysis) type RepairNumber = crate::contracts::ExactNumber;

pub(in crate::analysis) fn repair_numeric_rational_expression(
    predicate: &str,
) -> Option<RepairRational> {
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

pub(in crate::analysis) fn split_repair_numeric_operator<'a>(
    predicate: &'a str,
    operator: &str,
) -> Option<(&'a str, &'a str)> {
    crate::predicate_text::split_top_level_operator_where(
        predicate,
        operator,
        |text, index, operator| text[index..].starts_with(operator),
        |left, _| operator_is_binary(left, operator),
    )
}

pub(in crate::analysis) fn operator_is_binary(left: &str, operator: &str) -> bool {
    if operator != "-" {
        return true;
    }
    let left = left.trim_end();
    if left
        .chars()
        .next_back()
        .is_some_and(|ch| ch.is_ascii_digit() || ch == ')' || ch == '"')
    {
        return true;
    }
    let literal_start = left
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_alphanumeric())
        .last()
        .map_or(left.len(), |(index, _)| index);
    parse_integer_literal(&left[literal_start..]).is_ok()
}
