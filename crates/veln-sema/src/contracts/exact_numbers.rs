use veln_literals::parse_integer_literal;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactRational {
    pub(crate) numerator: i128,
    pub(crate) denominator: i128,
}

impl Ord for ExactRational {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("exact rational comparison overflow")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("exact rational comparison overflow"),
            )
    }
}

impl PartialOrd for ExactRational {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ExactRational {
    pub(crate) fn from_number(number: ExactNumber) -> Option<Self> {
        Self::from_raw(number.mantissa, 10_i128.checked_pow(number.scale)?)
    }

    pub(crate) fn from_raw(mut numerator: i128, mut denominator: i128) -> Option<Self> {
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

    pub(crate) fn negate(self) -> Option<Self> {
        Some(Self {
            numerator: self.numerator.checked_neg()?,
            denominator: self.denominator,
        })
    }

    pub(crate) fn add(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator
                .checked_mul(other.denominator)?
                .checked_add(other.numerator.checked_mul(self.denominator)?)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(crate) fn add_int(self, integer: i128) -> Option<Self> {
        self.add(Self::from_raw(integer, 1)?)
    }

    pub(crate) fn is_integer(&self) -> bool {
        self.denominator == 1
    }

    pub(crate) fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    pub(crate) fn mul(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.numerator)?,
            self.denominator.checked_mul(other.denominator)?,
        )
    }

    pub(crate) fn div(self, other: Self) -> Option<Self> {
        Self::from_raw(
            self.numerator.checked_mul(other.denominator)?,
            self.denominator.checked_mul(other.numerator)?,
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExactNumber {
    pub(crate) mantissa: i128,
    pub(crate) scale: u32,
}

impl Ord for ExactNumber {
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

impl PartialOrd for ExactNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl ExactNumber {
    pub(crate) fn integer(value: i64) -> Self {
        Self {
            mantissa: i128::from(value),
            scale: 0,
        }
    }

    pub(crate) fn as_i64(self) -> Option<i64> {
        (self.scale == 0)
            .then(|| i64::try_from(self.mantissa).ok())
            .flatten()
    }

    pub(crate) fn parse(text: &str) -> Option<Self> {
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
            || !integer.chars().all(|character| character.is_ascii_digit())
            || !fraction.chars().all(|character| character.is_ascii_digit())
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

    pub(crate) fn abs_cmp(&self, other: &Self) -> std::cmp::Ordering {
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

    pub(crate) fn abs_parts(&self) -> (String, String) {
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

    pub(crate) fn from_raw(mut mantissa: i128, mut scale: u32) -> Self {
        while scale > 0 && mantissa % 10 == 0 {
            mantissa /= 10;
            scale -= 1;
        }
        Self { mantissa, scale }
    }

    pub(crate) fn negate(self) -> Option<Self> {
        Some(Self {
            mantissa: self.mantissa.checked_neg()?,
            scale: self.scale,
        })
    }

    pub(crate) fn add(self, other: Self) -> Option<Self> {
        let scale = self.scale.max(other.scale);
        let left = self.scaled_mantissa(scale)?;
        let right = other.scaled_mantissa(scale)?;
        Some(Self::from_raw(left.checked_add(right)?, scale))
    }

    pub(crate) fn sub(self, other: Self) -> Option<Self> {
        self.add(other.negate()?)
    }

    pub(crate) fn mul(self, other: Self) -> Option<Self> {
        Some(Self::from_raw(
            self.mantissa.checked_mul(other.mantissa)?,
            self.scale.checked_add(other.scale)?,
        ))
    }

    pub(crate) fn div(self, other: Self) -> Option<Self> {
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
        let mantissa = numerator
            .checked_mul(10_i128.checked_pow(scale)?)?
            .checked_div(divisor_scale(twos, fives)?)?;
        Some(Self::from_raw(mantissa, scale))
    }

    fn scaled_mantissa(self, scale: u32) -> Option<i128> {
        let extra_scale = scale.checked_sub(self.scale)?;
        self.mantissa.checked_mul(10_i128.checked_pow(extra_scale)?)
    }
}

pub(crate) fn parse_quoted_string_literal(text: &str) -> Option<String> {
    if !text.starts_with('"') || !text.ends_with('"') {
        return None;
    }
    let mut value = String::new();
    let mut characters = text[1..text.len() - 1].chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            value.push(characters.next()?);
        } else if character == '"' {
            return None;
        } else {
            value.push(character);
        }
    }
    Some(value)
}

fn divisor_scale(twos: u32, fives: u32) -> Option<i128> {
    2_i128
        .checked_pow(twos)?
        .checked_mul(5_i128.checked_pow(fives)?)
}

fn gcd_i128(left: i128, right: i128) -> Option<i128> {
    let mut left = left.checked_abs()?;
    let mut right = right.checked_abs()?;
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    (left != 0).then_some(left)
}
