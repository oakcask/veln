use super::*;

pub(super) type StaticNumber = ExactNumber;
pub(super) type StaticRational = ExactRational;

#[derive(Clone, PartialEq, Eq)]
pub(super) enum StaticLiteral {
    Bool(bool),
    Number(StaticNumber),
    String(String),
}

pub(super) fn static_literal_comparison(left: &str, operator: &str, right: &str) -> Option<bool> {
    let left_literal = StaticLiteral::parse(left.trim());
    let right_literal = StaticLiteral::parse(right.trim());
    if let (Some(StaticLiteral::Number(left)), Some(StaticLiteral::Number(right))) =
        (&left_literal, &right_literal)
    {
        return static_number_comparison(*left, operator, *right);
    }
    if let (Some(left), Some(right)) = (
        static_numeric_expression(left),
        static_numeric_expression(right),
    ) {
        return static_number_comparison(left, operator, right);
    }
    if let (Some(left), Some(right)) = (
        static_rational_expression(left),
        static_rational_expression(right),
    ) {
        return static_rational_comparison(left, operator, right);
    }
    if matches!(operator, "==" | "!=") {
        let left = static_boolean_value(left);
        let right = static_boolean_value(right);
        if left != StaticBooleanValue::Unknown && right != StaticBooleanValue::Unknown {
            return Some(match operator {
                "==" => left == right,
                "!=" => left != right,
                _ => unreachable!("operator was already checked"),
            });
        }
    }
    match (left_literal?, right_literal?) {
        (StaticLiteral::Bool(left), StaticLiteral::Bool(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        (StaticLiteral::Number(left), StaticLiteral::Number(right)) => {
            static_number_comparison(left, operator, right)
        }
        (StaticLiteral::String(left), StaticLiteral::String(right)) => match operator {
            "==" => Some(left == right),
            "!=" => Some(left != right),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn static_numeric_expression(predicate: &str) -> Option<StaticNumber> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = StaticNumber::parse(predicate) {
        return Some(number);
    }
    if contains_binary_bitwise_operator(predicate) {
        if let Some(value) =
            static_binary_numeric_expression(predicate, &["|", "^", "&"], static_bitwise_operation)
        {
            return value;
        }
        if let Some(value) = static_binary_numeric_expression(
            predicate,
            &[">>>", ">>", "<<"],
            static_shift_operation,
        ) {
            return value;
        }
    }
    if let Some(value) =
        static_binary_numeric_expression(predicate, &["+", "-"], static_additive_operation)
    {
        return value;
    }
    if let Some(value) =
        static_binary_numeric_expression(predicate, &["*", "/"], static_multiplicative_operation)
    {
        return value;
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return static_numeric_expression(rest)?.negate();
    }
    if let Some(rest) = predicate.strip_prefix('~') {
        return Some(StaticNumber::integer(
            !static_numeric_expression(rest)?.as_i64()?,
        ));
    }
    None
}

pub(super) fn static_binary_numeric_expression(
    predicate: &str,
    operators: &[&str],
    operation: fn(StaticNumber, &str, StaticNumber) -> Option<StaticNumber>,
) -> Option<Option<StaticNumber>> {
    operators.iter().find_map(|operator| {
        split_top_level_operator(predicate, operator).map(|(left, right)| {
            let left = static_numeric_expression(left)?;
            let right = static_numeric_expression(right)?;
            operation(left, operator, right)
        })
    })
}

pub(super) fn static_bitwise_operation(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<StaticNumber> {
    let left = left.as_i64()?;
    let right = right.as_i64()?;
    let value = match operator {
        "|" => left | right,
        "^" => left ^ right,
        "&" => left & right,
        _ => return None,
    };
    Some(StaticNumber::integer(value))
}

pub(super) fn static_shift_operation(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<StaticNumber> {
    let left = left.as_i64()?;
    let right = right.as_i64()?;
    let count = u32::try_from(right).ok().filter(|count| *count <= 63)?;
    let value = match operator {
        ">>>" => ((left as u64) >> count) as i64,
        ">>" => left >> count,
        "<<" => left.wrapping_shl(count),
        _ => return None,
    };
    Some(StaticNumber::integer(value))
}

pub(super) fn static_additive_operation(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<StaticNumber> {
    match operator {
        "+" => left.add(right),
        "-" => left.sub(right),
        _ => None,
    }
}

pub(super) fn static_multiplicative_operation(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<StaticNumber> {
    match operator {
        "*" => left.mul(right),
        "/" => left.div(right),
        _ => None,
    }
}

pub(super) fn static_rational_expression(predicate: &str) -> Option<StaticRational> {
    let predicate = strip_balanced_outer_parens(predicate.trim());
    if predicate.is_empty() {
        return None;
    }
    if let Some(number) = StaticNumber::parse(predicate) {
        return StaticRational::from_number(number);
    }
    for operator in ["+", "-"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_rational_expression(left)?;
            let right = static_rational_expression(right)?;
            return match operator {
                "+" => left.add(right),
                "-" => left.sub(right),
                _ => None,
            };
        }
    }
    for operator in ["*", "/"] {
        if let Some((left, right)) = split_top_level_operator(predicate, operator) {
            let left = static_rational_expression(left)?;
            let right = static_rational_expression(right)?;
            return match operator {
                "*" => left.mul(right),
                "/" => left.div(right),
                _ => None,
            };
        }
    }
    if let Some(rest) = predicate.strip_prefix('-') {
        return static_rational_expression(rest)?.negate();
    }
    None
}

pub(super) fn static_rational_comparison(
    left: StaticRational,
    operator: &str,
    right: StaticRational,
) -> Option<bool> {
    static_comparison(left, operator, right)
}

pub(super) fn static_number_comparison(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<bool> {
    static_comparison(left, operator, right)
}

fn static_comparison<T: PartialEq + PartialOrd>(left: T, operator: &str, right: T) -> Option<bool> {
    Some(match operator {
        "==" => left == right,
        "!=" => left != right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => return None,
    })
}

impl StaticLiteral {
    pub(super) fn parse(text: &str) -> Option<Self> {
        let text = strip_balanced_outer_parens(text.trim());
        match text {
            "true" => return Some(Self::Bool(true)),
            "false" => return Some(Self::Bool(false)),
            _ => {}
        }
        if let Some(number) = StaticNumber::parse(text) {
            return Some(Self::Number(number));
        }
        parse_static_string_literal(text).map(Self::String)
    }
}

pub(super) fn parse_static_string_literal(text: &str) -> Option<String> {
    parse_quoted_string_literal(text)
}
