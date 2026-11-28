//! Shared parsing for source numeric literals.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParsedFloatLiteral {
    pub value: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatLiteralError {
    InvalidSyntax,
    Overflow,
    Underflow,
}

/// Converts an unsigned dotted-decimal source literal to its binary64 value.
///
/// Rust's decimal conversion performs one correctly rounded conversion to
/// binary64. The source-shape check keeps this boundary narrower than a host
/// floating-point parser, and the exact decimal digits distinguish a written
/// zero from a nonzero value that rounded to zero.
pub fn parse_float_literal(text: &str) -> Result<ParsedFloatLiteral, FloatLiteralError> {
    let Some((whole, fraction)) = text.split_once('.') else {
        return Err(FloatLiteralError::InvalidSyntax);
    };
    if whole.is_empty()
        || fraction.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(FloatLiteralError::InvalidSyntax);
    }

    let source_is_zero = whole
        .bytes()
        .chain(fraction.bytes())
        .all(|byte| byte == b'0');
    let value = text
        .parse::<f64>()
        .map_err(|_| FloatLiteralError::InvalidSyntax)?;
    if value.is_infinite() {
        return Err(FloatLiteralError::Overflow);
    }
    if value == 0.0 && !source_is_zero {
        return Err(FloatLiteralError::Underflow);
    }
    Ok(ParsedFloatLiteral { value })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerRadix {
    Decimal,
    Binary,
    Hexadecimal,
}

impl IntegerRadix {
    pub fn base(self) -> u32 {
        match self {
            Self::Decimal => 10,
            Self::Binary => 2,
            Self::Hexadecimal => 16,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Binary => "binary",
            Self::Hexadecimal => "hexadecimal",
        }
    }

    pub fn accepted_digits(self) -> &'static str {
        match self {
            Self::Decimal => "0 through 9",
            Self::Binary => "0 or 1",
            Self::Hexadecimal => "0 through 9, a through f, or A through F",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedIntegerLiteral {
    pub value: i64,
    pub radix: IntegerRadix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegerLiteralError {
    MissingDigits {
        radix: IntegerRadix,
    },
    UnsupportedUppercasePrefix {
        radix: IntegerRadix,
    },
    InvalidDigit {
        radix: IntegerRadix,
        byte_offset: usize,
        character: char,
    },
    Separator {
        radix: IntegerRadix,
        byte_offset: usize,
    },
    PrefixedFloat {
        radix: IntegerRadix,
        byte_offset: usize,
    },
    OutOfRange {
        radix: IntegerRadix,
    },
}

pub fn parse_integer_literal(text: &str) -> Result<ParsedIntegerLiteral, IntegerLiteralError> {
    let (radix, digits) = if let Some(digits) = text.strip_prefix("0b") {
        (IntegerRadix::Binary, digits)
    } else if let Some(digits) = text.strip_prefix("0x") {
        (IntegerRadix::Hexadecimal, digits)
    } else if text.starts_with("0B") {
        return Err(IntegerLiteralError::UnsupportedUppercasePrefix {
            radix: IntegerRadix::Binary,
        });
    } else if text.starts_with("0X") {
        return Err(IntegerLiteralError::UnsupportedUppercasePrefix {
            radix: IntegerRadix::Hexadecimal,
        });
    } else {
        (IntegerRadix::Decimal, text)
    };

    if digits.is_empty() {
        return Err(IntegerLiteralError::MissingDigits { radix });
    }

    let prefix_len = text.len() - digits.len();
    for (offset, character) in digits.char_indices() {
        let byte_offset = prefix_len + offset;
        if character == '_' {
            return Err(IntegerLiteralError::Separator { radix, byte_offset });
        }
        if character == '.' && radix != IntegerRadix::Decimal {
            return Err(IntegerLiteralError::PrefixedFloat { radix, byte_offset });
        }
        if character.to_digit(radix.base()).is_none() {
            return Err(IntegerLiteralError::InvalidDigit {
                radix,
                byte_offset,
                character,
            });
        }
    }

    i64::from_str_radix(digits, radix.base())
        .map(|value| ParsedIntegerLiteral { value, radix })
        .map_err(|_| IntegerLiteralError::OutOfRange { radix })
}

#[cfg(test)]
mod tests {
    use super::{
        FloatLiteralError, IntegerLiteralError, IntegerRadix, parse_float_literal,
        parse_integer_literal,
    };

    fn decimal_fraction(zeros: usize, digits: &str) -> String {
        format!("0.{}{digits}", "0".repeat(zeros))
    }

    #[test]
    fn rounds_decimal_values_once_to_binary64_ties_to_even() {
        let cases = [
            ("9007199254740993.0", 0x4340_0000_0000_0000),
            ("9007199254740995.0", 0x4340_0000_0000_0002),
            ("1.5", 0x3ff8_0000_0000_0000),
        ];
        for (text, expected_bits) in cases {
            assert_eq!(
                parse_float_literal(text).unwrap().value.to_bits(),
                expected_bits
            );
        }
    }

    #[test]
    fn applies_subnormal_and_zero_range_boundaries() {
        let rounds_to_min_subnormal = decimal_fraction(323, "4");
        let rounds_to_zero = decimal_fraction(323, "2");
        assert_eq!(
            parse_float_literal(&rounds_to_min_subnormal)
                .unwrap()
                .value
                .to_bits(),
            1
        );
        assert_eq!(
            parse_float_literal(&rounds_to_zero),
            Err(FloatLiteralError::Underflow)
        );
        assert_eq!(parse_float_literal("0.0").unwrap().value.to_bits(), 0);
    }

    #[test]
    fn rejects_overflow_and_non_literal_host_spellings() {
        let overflow = format!("1{}.0", "0".repeat(309));
        assert_eq!(
            parse_float_literal(&overflow),
            Err(FloatLiteralError::Overflow)
        );
        for text in ["1", "1.", ".5", "1e2", "-0.0", "NaN", "inf"] {
            assert_eq!(
                parse_float_literal(text),
                Err(FloatLiteralError::InvalidSyntax)
            );
        }
    }

    #[test]
    fn parses_equivalent_radix_spellings() {
        for text in ["10", "0b1010", "0x0a", "0x0A"] {
            assert_eq!(parse_integer_literal(text).unwrap().value, 10);
        }
    }

    #[test]
    fn rejects_the_first_invalid_digit() {
        assert_eq!(
            parse_integer_literal("0b102"),
            Err(IntegerLiteralError::InvalidDigit {
                radix: IntegerRadix::Binary,
                byte_offset: 4,
                character: '2',
            })
        );
    }

    #[test]
    fn applies_the_signed_int_literal_limit_to_every_radix() {
        for text in [
            "9223372036854775807",
            "0b111111111111111111111111111111111111111111111111111111111111111",
            "0x7FFFFFFFFFFFFFFF",
        ] {
            assert_eq!(parse_integer_literal(text).unwrap().value, i64::MAX);
        }
        for text in [
            "9223372036854775808",
            "0b1000000000000000000000000000000000000000000000000000000000000000",
            "0x8000000000000000",
        ] {
            assert!(matches!(
                parse_integer_literal(text),
                Err(IntegerLiteralError::OutOfRange { .. })
            ));
        }
    }
}
