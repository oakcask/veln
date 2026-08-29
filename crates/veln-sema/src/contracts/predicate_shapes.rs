use super::*;

pub(super) fn flattened_keyword_clauses<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut flattened = Vec::new();
    let mut stack = vec![strip_balanced_outer_parens(predicate)];

    while let Some(predicate) = stack.pop() {
        let clauses = split_top_level_keyword(strip_balanced_outer_parens(predicate), keyword);
        if clauses.len() <= 1 {
            flattened.extend(clauses);
            continue;
        }
        stack.extend(clauses.into_iter().rev());
    }

    flattened
}

pub(super) fn negated_predicate_inner(predicate: &str) -> Option<&str> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if let Some(rest) = predicate.strip_prefix("not ") {
        let rest = rest.trim();
        if rest.starts_with('(') {
            return (strip_balanced_outer_parens(rest) != rest).then_some(rest);
        }
        if is_single_negation_operand(rest) {
            return Some(rest);
        }
        return None;
    }
    if let Some(rest) = predicate.strip_prefix("not(") {
        return rest.strip_suffix(')');
    }
    None
}

pub(super) fn whole_negated_predicate_inner(predicate: &str) -> Option<&str> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    let rest = predicate.strip_prefix("not")?;
    if !(rest.starts_with(char::is_whitespace) || rest.starts_with('(')) {
        return None;
    }
    let rest = rest.trim();
    if rest.starts_with('(') {
        return (strip_balanced_outer_parens(rest) != rest).then_some(rest);
    }
    is_single_negation_operand(rest).then_some(rest)
}

pub(super) fn is_single_negation_operand(predicate: &str) -> bool {
    split_top_level_keyword(predicate, "and").len() == 1
        && split_top_level_keyword(predicate, "or").len() == 1
}

pub(super) fn same_predicate(left: &str, right: &str) -> bool {
    predicate_shape(left) == predicate_shape(right)
}

pub(super) fn predicate_shape(predicate: &str) -> String {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    let mut shape = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut pending_space = false;
    for ch in predicate.chars() {
        if in_string {
            shape.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            if pending_space && !shape.is_empty() {
                shape.push(' ');
            }
            pending_space = false;
            in_string = true;
            shape.push(ch);
            continue;
        }
        if ch.is_whitespace() {
            pending_space = !shape.is_empty();
            continue;
        }
        if pending_space {
            shape.push(' ');
            pending_space = false;
        }
        shape.push(ch);
    }
    shape
}

pub(super) fn boolean_literal_alias_shape(predicate: &str) -> Option<(String, bool)> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["==", "!="] {
        let Some((left, right)) = split_top_level_operator(predicate, operator) else {
            continue;
        };
        if let Some(value) = boolean_literal(right) {
            return Some((predicate_shape(left), value == (operator == "==")));
        }
        if let Some(value) = boolean_literal(left) {
            return Some((predicate_shape(right), value == (operator == "==")));
        }
    }
    None
}

pub(super) fn boolean_literal(predicate: &str) -> Option<bool> {
    match strip_balanced_outer_parens(predicate.trim()) {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[derive(PartialEq, Eq)]
pub(super) struct ComparisonShape {
    pub(super) left: String,
    pub(super) operator: &'static str,
    pub(super) right: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum OrderRelation {
    Less,
    Equal,
    Greater,
}

impl OrderRelation {
    pub(super) const ALL_BITS: u8 = Self::Less.bit() | Self::Equal.bit() | Self::Greater.bit();

    pub(super) const fn bit(self) -> u8 {
        match self {
            Self::Less => 0b001,
            Self::Equal => 0b010,
            Self::Greater => 0b100,
        }
    }
}

pub(super) struct OrderTrichotomyShape {
    pub(super) left: String,
    pub(super) right: String,
    pub(super) relation: OrderRelation,
}

pub(super) fn complementary_comparisons(left: &str, right: &str) -> bool {
    let Some(left) = comparison_shape(left) else {
        return false;
    };
    let Some(right) = comparison_shape(right) else {
        return false;
    };
    left.left == right.left
        && left.right == right.right
        && matches!(
            (left.operator, right.operator),
            ("==", "!=") | ("!=", "==") | ("<", ">=") | (">=", "<")
        )
        || complementary_literal_comparison_operands(&left, &right)
}

pub(super) fn complementary_literal_comparison_operands(
    left: &ComparisonShape,
    right: &ComparisonShape,
) -> bool {
    if !matches!((left.operator, right.operator), ("==", "!=") | ("!=", "==")) {
        return false;
    }
    (left.left == right.left
        && static_literal_comparison(&left.right, "==", &right.right) == Some(true))
        || (left.right == right.right
            && static_literal_comparison(&left.left, "==", &right.left) == Some(true))
        || (left.left == right.right
            && static_literal_comparison(&left.right, "==", &right.left) == Some(true))
        || (left.right == right.left
            && static_literal_comparison(&left.left, "==", &right.right) == Some(true))
}

pub(super) fn order_trichotomy_shape(predicate: &str) -> Option<OrderTrichotomyShape> {
    let comparison = comparison_shape(predicate)?;
    let mut left = comparison.left;
    let mut right = comparison.right;
    if left == right {
        return None;
    }
    let mut relation = match comparison.operator {
        "==" => OrderRelation::Equal,
        "<" => OrderRelation::Less,
        _ => return None,
    };
    if right < left {
        std::mem::swap(&mut left, &mut right);
        relation = match relation {
            OrderRelation::Less => OrderRelation::Greater,
            OrderRelation::Equal => OrderRelation::Equal,
            OrderRelation::Greater => OrderRelation::Less,
        };
    }
    Some(OrderTrichotomyShape {
        left,
        right,
        relation,
    })
}

pub(super) fn comparison_shape(predicate: &str) -> Option<ComparisonShape> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    for operator in ["==", "!=", "<=", ">=", "<", ">"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = compact_predicate_text(left);
            let right = compact_predicate_text(right);
            return match operator {
                "==" | "!=" if right < left => Some(ComparisonShape {
                    left: right,
                    operator,
                    right: left,
                }),
                "<=" => Some(ComparisonShape {
                    left: right,
                    operator: ">=",
                    right: left,
                }),
                ">" => Some(ComparisonShape {
                    left: right,
                    operator: "<",
                    right: left,
                }),
                _ => Some(ComparisonShape {
                    left,
                    operator,
                    right,
                }),
            };
        }
    }
    None
}

pub(super) fn static_same_shape_comparison(
    left: &str,
    operator: &str,
    right: &str,
) -> Option<bool> {
    if compact_predicate_text(left) != compact_predicate_text(right) {
        return None;
    }
    match operator {
        "==" | "<=" | ">=" => Some(true),
        "!=" | "<" | ">" => Some(false),
        _ => None,
    }
}

pub(super) fn static_complementary_predicate_comparison(
    left: &str,
    operator: &str,
    right: &str,
) -> Option<bool> {
    if !matches!(operator, "==" | "!=") || !complementary_predicates(left, right) {
        return None;
    }
    Some(operator == "!=")
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

impl StaticBooleanValue {
    pub(super) fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    pub(super) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            (Self::False, value) | (value, Self::False) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }

    pub(super) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            (Self::True, value) | (value, Self::True) => value,
            (Self::Unknown, Self::Unknown) => Self::Unknown,
        }
    }
}

impl From<bool> for StaticBooleanValue {
    fn from(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}
