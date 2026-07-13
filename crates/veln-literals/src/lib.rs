//! Shared parsing for source integer literals.

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
    use super::{IntegerLiteralError, IntegerRadix, parse_integer_literal};

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
