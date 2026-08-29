use super::*;

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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct StaticRational {
    numerator: i128,
    denominator: i128,
}

impl Ord for StaticRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("static rational comparison overflow")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("static rational comparison overflow"),
            )
    }
}

impl PartialOrd for StaticRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl StaticRational {
    fn from_number(number: StaticNumber) -> Option<Self> {
        Self::from_raw(number.mantissa, 10_i128.checked_pow(number.scale)?)
    }

    fn from_raw(mut numerator: i128, mut denominator: i128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if denominator < 0 {
            numerator = numerator.checked_neg()?;
            denominator = denominator.checked_neg()?;
        }
        let divisor = gcd_i128(numerator, denominator)?;
        Some(Self {
            numerator: numerator.checked_div(divisor)?,
            denominator: denominator.checked_div(divisor)?,
        })
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_add(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    fn div(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
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

pub(super) fn static_number_comparison(
    left: StaticNumber,
    operator: &str,
    right: StaticNumber,
) -> Option<bool> {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct StaticNumber {
    mantissa: i128,
    scale: u32,
}

impl Ord for StaticNumber {
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

impl PartialOrd for StaticNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl StaticNumber {
    fn integer(value: i64) -> Self {
        Self {
            mantissa: i128::from(value),
            scale: 0,
        }
    }

    fn as_i64(self) -> Option<i64> {
        (self.scale == 0)
            .then(|| i64::try_from(self.mantissa).ok())
            .flatten()
    }

    fn parse(text: &str) -> Option<Self> {
        let (negative, digits) = text
            .strip_prefix('-')
            .map_or((false, text), |digits| (true, digits.trim_start()));
        if digits.is_empty() {
            return None;
        }
        if !digits.contains('.')
            && let Ok(literal) = parse_integer_literal(digits)
        {
            return Some(Self {
                mantissa: if negative {
                    -i128::from(literal.value)
                } else {
                    i128::from(literal.value)
                },
                scale: 0,
            });
        }
        let (integer, fraction) = digits.split_once('.').unwrap_or((digits, ""));
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
        Some(Self { mantissa, scale })
    }

    fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
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

    fn abs_parts(&self) -> (String, String) {
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

    fn from_raw(mut mantissa: i128, mut scale: u32) -> Self {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self { mantissa, scale }
    }

    fn negate(self) -> Option<Self> {
        Some(Self {
            mantissa: self.mantissa.checked_neg()?,
            scale: self.scale,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.scaled_mantissa(scale)?;
        let right = other.scaled_mantissa(scale)?;
        Some(Self::from_raw(left.checked_add(right)?, scale))
    }

    fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    fn mul(self, other: Self) -> Option<Self> {
        Some(Self::from_raw(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    fn div(self, other: Self) -> Option<Self> {
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

        let divisor = gcd_i128(numerator, denominator)?;
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
            .checked_div(divisor_scale(twos, fives)?)?;
        Some(Self::from_raw(mantissa, scale))
    }

    fn scaled_mantissa(self, scale: u32) -> Option<i128> {
        let extra_scale = scale.checked_sub(self.scale)?;
        self.mantissa.checked_mul(10_i128.checked_pow(extra_scale)?)
    }
}

pub(super) fn divisor_scale(twos: u32, fives: u32) -> Option<i128> {
    let twos = 2_i128.checked_pow(twos)?;
    let fives = 5_i128.checked_pow(fives)?;
    twos.checked_mul(fives)
}

pub(super) fn gcd_i128(left: i128, right: i128) -> Option<i128> {
    let mut left = left.checked_abs()?;
    let mut right = right.checked_abs()?;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    (left != 0).then_some(left)
}

pub(super) fn parse_static_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            value.push(chars.next()?);
        } else if ch == '"' {
            return None;
        } else {
            value.push(ch);
        }
    }
    Some(value)
}
